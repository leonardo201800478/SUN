//! Serial YM2413 Phase Generator service.
//!
//! This service is called by the core-owned pre/post EG slot hooks. It keeps
//! Nuked's delayed PG array write, phase-next latch, increment latch and PM
//! counter in one cloneable state machine. The core remains the sole owner of
//! input clocks and the serial EG sequencer.
//!
//! This first integration gate covers normal melodic mode. Rhythm-specific PG
//! phase combiners and the special channel-7..9 parameter routing remain FM-5.

use crate::am_lfo::{Ym2413SerialAmLfo, Ym2413SerialAmLfoError, Ym2413SerialAmLfoState};
use crate::channel::{ChannelSynthesisError, OperatorPhaseParameters, decode_channel_synthesis};
use crate::envelope_sequencer::{EnvelopeSequencerState, EnvelopeSequencerTick};
use crate::phase::{PhaseGenerator, PhaseGeneratorError};
use crate::serial_operator::{
    Ym2413SerialOperator, Ym2413SerialOperatorError, Ym2413SerialOperatorPatch1Latches,
    Ym2413SerialOperatorPatch2Latches, Ym2413SerialOperatorState,
};
use crate::slot::{Ym2413Operator, nuked_pg_phase_slot_descriptor};
use crate::slot_service::{Ym2413SlotService, Ym2413SlotServiceContext};
use crate::synthesis::{
    Ym2413SynthesisBank, Ym2413SynthesisBankState, Ym2413SynthesisError,
    Ym2413SynthesisInitialState,
};

pub const YM2413_PG_SERIAL_STATE_VERSION: u16 = 5;
pub const YM2413_PG_SERIAL_PIPELINE_STATE_VERSION: u16 = 3;

/// Maximum increment reachable from the legal FNUM/BLOCK/MUL/PM input domain.
pub const YM2413_PG_INCREMENT_MAX: u32 = 987_840;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413PgSerialError {
    InvalidState,
    InvalidSlotIndex(u8),
    UnsupportedRhythmMode { cycle: u8, channel: u8 },
    Channel(ChannelSynthesisError),
    Phase(PhaseGeneratorError),
    Synthesis(Ym2413SynthesisError),
    Operator(Ym2413SerialOperatorError),
    AmLfo(Ym2413SerialAmLfoError),
}

/// Error returned by the public coarse-grained core + PG advancement API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413PgSerialAdvanceError {
    Core(crate::Ym2413Error),
    Service(Ym2413PgSerialError),
}

impl std::fmt::Display for Ym2413PgSerialAdvanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core(error) => write!(f, "YM2413 core error: {error}"),
            Self::Service(error) => write!(f, "YM2413 PG service error: {error:?}"),
        }
    }
}

impl std::error::Error for Ym2413PgSerialAdvanceError {}

impl From<ChannelSynthesisError> for Ym2413PgSerialError {
    fn from(value: ChannelSynthesisError) -> Self {
        Self::Channel(value)
    }
}
impl From<PhaseGeneratorError> for Ym2413PgSerialError {
    fn from(value: PhaseGeneratorError) -> Self {
        Self::Phase(value)
    }
}
impl From<Ym2413SynthesisError> for Ym2413PgSerialError {
    fn from(value: Ym2413SynthesisError) -> Self {
        Self::Synthesis(value)
    }
}
impl From<Ym2413SerialOperatorError> for Ym2413PgSerialError {
    fn from(value: Ym2413SerialOperatorError) -> Self {
        Self::Operator(value)
    }
}
impl From<Ym2413SerialAmLfoError> for Ym2413PgSerialError {
    fn from(value: Ym2413SerialAmLfoError) -> Self {
        Self::AmLfo(value)
    }
}

/// Parameters retained by Nuked's c_fnum/c_block/c_multi/c_vib latches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413PgParameterLatch {
    pub fnum: u16,
    pub block: u8,
    pub multiplier: u8,
    pub pm_enabled: bool,
}

impl Ym2413PgParameterLatch {
    const RESET: Self = Self {
        fnum: 0,
        block: 0,
        multiplier: 0,
        pm_enabled: false,
    };

    fn from_operator(parameters: OperatorPhaseParameters) -> Self {
        Self {
            fnum: parameters.fnum,
            block: parameters.block,
            multiplier: parameters.multiplier,
            pm_enabled: parameters.pm_enabled,
        }
    }

    const fn is_valid(self) -> bool {
        self.fnum <= 0x01ff && self.block <= 7 && self.multiplier <= 0x0f
    }
}

/// Latches owned by the serial PG pipeline, separate from the 18 accumulators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413PgSerialPipelineState {
    pub format_version: u16,
    pub pg_increment: u32,
    pub pg_phase_next: u32,
    pub pg_output: u16,
    /// Nuked's three-bit lfo_vib_counter, which can diverge from lfo_counter
    /// while Test Mode bit 3 is active.
    pub lfo_vib_counter: u8,
    pub parameters: Ym2413PgParameterLatch,
}

/// Complete serial PG service snapshot, including all eighteen PG accumulators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413PgSerialServiceState {
    pub format_version: u16,
    pub pipeline: Ym2413PgSerialPipelineState,
    pub synthesis: Ym2413SynthesisBankState,
    pub operator: Ym2413SerialOperatorState,
    pub serial_am_lfo: Ym2413SerialAmLfoState,
}

/// Cloneable serial PG state machine. Its Clone is deep: all mutable state is
/// owned values or the synthesis bank's independently cloned value state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ym2413PgSerialService {
    synthesis: Ym2413SynthesisBank,
    pipeline: Ym2413PgSerialPipelineState,
    operator: Ym2413SerialOperator,
    serial_am_lfo: Ym2413SerialAmLfo,
}

impl Ym2413PgSerialService {
    /// Create a serial PG service around an explicitly initialized synthesis bank.
    pub fn new(synthesis: Ym2413SynthesisBank) -> Self {
        Self {
            synthesis,
            pipeline: Ym2413PgSerialPipelineState {
                format_version: YM2413_PG_SERIAL_PIPELINE_STATE_VERSION,
                pg_increment: 0,
                pg_phase_next: 0,
                pg_output: 0,
                lfo_vib_counter: 0,
                parameters: Ym2413PgParameterLatch::RESET,
            },
            operator: Ym2413SerialOperator::new(),
            serial_am_lfo: Ym2413SerialAmLfo::new(),
        }
    }

    /// Build a reset candidate bank for tests and explicit initial-state users.
    pub fn with_initial_state(
        initial: Ym2413SynthesisInitialState,
    ) -> Result<Self, Ym2413SynthesisError> {
        Ok(Self::new(Ym2413SynthesisBank::with_initial_state(initial)?))
    }

    /// Return the current ten-bit PG output latch.
    pub const fn pg_output(&self) -> u16 {
        self.pipeline.pg_output
    }

    /// Return the retained PG increment.
    pub const fn pg_increment(&self) -> u32 {
        self.pipeline.pg_increment
    }

    /// Return the delayed PG next-phase latch.
    pub const fn pg_phase_next(&self) -> u32 {
        self.pipeline.pg_phase_next
    }

    /// Return the current three-bit vibrato LFO counter.
    pub const fn lfo_vib_counter(&self) -> u8 {
        self.pipeline.lfo_vib_counter
    }

    /// Borrow the eighteen-operator synthesis bank.
    pub const fn synthesis(&self) -> &Ym2413SynthesisBank {
        &self.synthesis
    }

    /// Export every accumulator, latch and LFO state needed for exact resumption.
    pub const fn export_state(&self) -> Ym2413PgSerialServiceState {
        Ym2413PgSerialServiceState {
            format_version: YM2413_PG_SERIAL_STATE_VERSION,
            pipeline: self.pipeline,
            synthesis: self.synthesis.export_state(),
            operator: self.operator.export_state(),
            serial_am_lfo: self.serial_am_lfo.export_state(),
        }
    }

    /// Restore the serial PG service after validating its complete state.
    pub fn restore_state(state: Ym2413PgSerialServiceState) -> Result<Self, Ym2413PgSerialError> {
        let pipeline = state.pipeline;
        if state.format_version != YM2413_PG_SERIAL_STATE_VERSION
            || pipeline.format_version != YM2413_PG_SERIAL_PIPELINE_STATE_VERSION
            || pipeline.lfo_vib_counter > 7
            || pipeline.pg_increment > YM2413_PG_INCREMENT_MAX
            || !pipeline.parameters.is_valid()
        {
            return Err(Ym2413PgSerialError::InvalidState);
        }
        Ok(Self {
            synthesis: Ym2413SynthesisBank::restore_state(state.synthesis)?,
            pipeline,
            operator: Ym2413SerialOperator::restore_state(state.operator)?,
            serial_am_lfo: Ym2413SerialAmLfo::restore_state(state.serial_am_lfo)?,
        })
    }

    /// Consume one PG phase-array write and one delayed output/reset decision.
    fn generate_phase(
        &mut self,
        context: &Ym2413SlotServiceContext,
    ) -> Result<(), Ym2413PgSerialError> {
        let write = nuked_pg_phase_slot_descriptor(context.event.pg_phase_write_index).ok_or(
            Ym2413PgSerialError::InvalidSlotIndex(context.event.pg_phase_write_index),
        )?;
        let write_carrier = matches!(write.operator, Ym2413Operator::Carrier);
        let phase_write = self
            .pipeline
            .pg_phase_next
            .wrapping_add(self.pipeline.pg_increment);
        self.synthesis
            .write_operator_phase(write.channel, write_carrier, phase_write)?;

        let output = nuked_pg_phase_slot_descriptor(context.event.pg_phase_output_index).ok_or(
            Ym2413PgSerialError::InvalidSlotIndex(context.event.pg_phase_output_index),
        )?;
        let output_carrier = matches!(output.operator, Ym2413Operator::Carrier);
        let phase = self
            .synthesis
            .phase_accumulator(output.channel, output_carrier)?;

        // Nuked stores the reset decision in pg_phase_next. It does not reset
        // the current output accumulator or the phase array entry just written.
        self.pipeline.pg_phase_next = if context.event.phase_reset_required {
            0
        } else {
            phase
        };
        self.pipeline.pg_output = (phase >> 9) as u16;
        Ok(())
    }

    /// Calculate pg_inc from the previous cycle's retained c_* parameter latches.
    fn calculate_increment(&mut self) -> Result<(), Ym2413PgSerialError> {
        self.pipeline.pg_increment = PhaseGenerator::phase_step(
            self.pipeline.parameters.fnum,
            self.pipeline.parameters.block,
            self.pipeline.parameters.multiplier,
            self.pipeline.parameters.pm_enabled,
            self.pipeline.lfo_vib_counter,
        )?;
        Ok(())
    }

    /// Advance the PM counters at cycle 17, matching the OPLL_DoLFO stage.
    fn clock_pm_lfo(&mut self, testmode: u8) {
        let counter = self.synthesis.pm_lfo_counter();
        let rollover = u8::from((counter & 0x03ff) == 0x03ff);
        let test_step = (testmode >> 3) & 1;
        self.pipeline.lfo_vib_counter = self
            .pipeline
            .lfo_vib_counter
            .wrapping_add(rollover | test_step)
            & 7;
        self.synthesis.advance_pm_lfo_samples(1);
    }

    /// Reproduce PreparePatch2's channel/operator parameter selection for the
    /// next serial cycle. Rhythm-specific patch routing is deliberately gated.
    fn latch_next_parameters(
        &mut self,
        context: &Ym2413SlotServiceContext,
    ) -> Result<(), Ym2413PgSerialError> {
        let descriptor = context.event.selected_operator;
        if context.registers[0x0e] & 0x20 != 0 || context.rm_enable != 0 || context.rm_select < 6 {
            return Err(Ym2413PgSerialError::UnsupportedRhythmMode {
                cycle: context.event.cycle,
                channel: descriptor.channel,
            });
        }
        let config = decode_channel_synthesis(&context.registers, descriptor.channel)?;
        let parameters = match descriptor.operator {
            Ym2413Operator::Modulator => config.modulator_phase,
            Ym2413Operator::Carrier => config.carrier_phase,
        };
        self.pipeline.parameters = Ym2413PgParameterLatch::from_operator(parameters);
        let operator_patch = match descriptor.operator {
            Ym2413Operator::Modulator => config.patch.modulator,
            Ym2413Operator::Carrier => config.patch.carrier,
        };
        self.operator
            .prepare_patch2(Ym2413SerialOperatorPatch2Latches {
                feedback: config.patch.feedback,
                dc_waveform: config.patch.carrier.half_wave,
                dm_waveform: config.patch.modulator.half_wave,
                am_enabled: operator_patch.am_enabled,
                vibrato_enabled: operator_patch.vibrato_enabled,
                multiplier: operator_patch.multiplier,
                sustain_level: operator_patch.sustain_level,
                fnum: config.channel.fnum,
                block: config.channel.block,
            })?;
        Ok(())
    }
}

impl Ym2413SlotService for Ym2413PgSerialService {
    type Error = Ym2413PgSerialError;

    fn before_eg_update(&mut self, context: &Ym2413SlotServiceContext) -> Result<(), Self::Error> {
        if context.registers[0x0e] & 0x20 != 0 || context.rm_enable != 0 || context.rm_select < 6 {
            return Err(Ym2413PgSerialError::UnsupportedRhythmMode {
                cycle: context.event.cycle,
                channel: context.event.selected_operator.channel,
            });
        }
        // Nuked order in this pre-EG hook:
        // PreparePatch1 -> Channel -> PhaseGenerate -> Operator
        // -> PhaseCalcIncrement -> EnvelopeOutput -> EnvelopeKSLTL.
        let descriptor = context.event.selected_operator;
        let config = decode_channel_synthesis(&context.registers, descriptor.channel)?;
        let (operator_patch, ksl_attenuation, total_level, operator_index) =
            match descriptor.operator {
                Ym2413Operator::Modulator => (
                    config.patch.modulator,
                    config.modulator_ksl,
                    config.patch.modulator.total_level,
                    0,
                ),
                Ym2413Operator::Carrier => (
                    config.patch.carrier,
                    config.carrier_ksl,
                    config.channel.volume << 2,
                    1,
                ),
            };
        self.operator
            .prepare_patch1(Ym2413SerialOperatorPatch1Latches {
                total_level,
                ksl_attenuation,
                ksl: operator_patch.key_scale_level,
                adrr: [
                    operator_patch.attack_rate,
                    operator_patch.decay_rate,
                    operator_patch.release_rate,
                ],
                envelope_type: operator_patch.sustained_envelope,
                key_scale_rate: operator_patch.key_scale_rate,
                key_scale_rate_frequency: (config.channel.block << 1)
                    | ((config.channel.fnum >> 8) as u8 & 1),
                key_scale_level_frequency: (config.channel.fnum >> 5) as u8,
                key_scale_level_block: config.channel.block,
            })?;
        self.operator
            .prepare_channel(config.channel.instrument, operator_index)?;
        self.serial_am_lfo.capture_output(context.event.cycle)?;
        self.generate_phase(context)?;
        self.operator
            .tick_operator(context.event.cycle, self.pipeline.pg_output)?;
        self.calculate_increment()?;
        let eg_levels = std::array::from_fn(|index| context.envelope_state.operators[index].level);
        self.operator.advance_envelope_output(
            context.event.cycle,
            &eg_levels,
            self.serial_am_lfo.output(),
            context.registers[0x0f],
        )?;
        Ok(())
    }

    fn after_eg_update(
        &mut self,
        context: &Ym2413SlotServiceContext,
        _tick: EnvelopeSequencerTick,
        _post_state: EnvelopeSequencerState,
    ) -> Result<(), Self::Error> {
        // Nuked order after the core-owned EnvelopeGenerate stage:
        // DoLFO -> DoRhythm (FM-5; rejected here) -> PreparePatch2.
        let testmode = context.registers[0x0f];
        // OPLL_DoLFO advances the serial AM network on every call and
        // derives its cycle-17 step from the PM counter before that counter
        // is advanced by the PM stage.
        let pm_counter = self.synthesis.pm_lfo_counter();
        self.serial_am_lfo
            .clock(context.event.cycle, pm_counter, testmode)?;
        if testmode & 0x02 != 0 {
            self.pipeline.lfo_vib_counter = 0;
            self.synthesis.reset_pm_lfo();
        } else if context.event.cycle == 17 {
            self.clock_pm_lfo(testmode);
        }
        self.latch_next_parameters(context)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        YM2413_PG_SERIAL_PIPELINE_STATE_VERSION, YM2413_PG_SERIAL_STATE_VERSION,
        Ym2413PgParameterLatch, Ym2413PgSerialError, Ym2413PgSerialPipelineState,
        Ym2413PgSerialService,
    };
    use crate::envelope::{
        AttackRate15Transition, DampTransitionConfig, DampTransitionCoordination, EnvelopeStage,
    };
    use crate::envelope_sequencer::{
        EnvelopeSequencer, EnvelopeSequencerState, EnvelopeSequencerTick, EnvelopeSlotInput,
    };
    use crate::phase::PhaseGenerator;
    use crate::slot::{nuked_pg_phase_slot_descriptor, nuked_slot_service_event_with_testmode};
    use crate::slot_service::{
        Ym2413SlotService, Ym2413SlotServiceAdvanceError, Ym2413SlotServiceContext,
    };
    use crate::synthesis::{
        Ym2413ChannelInitialState, Ym2413SynthesisBank, Ym2413SynthesisInitialState,
    };
    use crate::{YM2413_INPUT_CLOCKS_PER_SLOT, Ym2413Core};

    fn initial_bank(pm_lfo_counter: u16) -> Ym2413SynthesisBank {
        let initial = Ym2413SynthesisInitialState {
            channels: std::array::from_fn(|channel| Ym2413ChannelInitialState {
                phase: [0x1000 * channel as u32, 0x1800 * channel as u32],
                envelope_level: [127; 2],
                envelope_stage: [EnvelopeStage::Damp; 2],
                envelope_key_on: [false; 2],
                feedback_history: [0; 2],
                previous_modulator_phase_index: None,
            }),
            pm_lfo_counter,
            am_lfo_phase: 0,
            damp_config: DampTransitionConfig::new(
                124,
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            )
            .unwrap(),
        };
        Ym2413SynthesisBank::with_initial_state(initial).unwrap()
    }

    fn context(cycle: u8, registers: [u8; 64], testmode: u8) -> Ym2413SlotServiceContext {
        let eg = EnvelopeSequencer::new();
        let state = eg.export_state();
        let event =
            nuked_slot_service_event_with_testmode(cycle, 0, state.eg_dokon, testmode).unwrap();
        let mut registers = registers;
        registers[0x0f] = testmode;
        Ym2413SlotServiceContext {
            event,
            registers,
            envelope_input: EnvelopeSlotInput::default(),
            envelope_state: state,
            rm_enable: 0,
            rm_select: 6,
        }
    }

    fn tick(cycle: u8) -> (crate::EnvelopeSequencerTick, crate::EnvelopeSequencerState) {
        let mut eg = EnvelopeSequencer::new();
        for prior_cycle in 0..cycle {
            eg.tick_slot(prior_cycle, EnvelopeSlotInput::default())
                .unwrap();
        }
        let result = eg.tick_slot(cycle, EnvelopeSlotInput::default()).unwrap();
        (result, eg.export_state())
    }

    fn service_with_pipeline(
        bank: Ym2413SynthesisBank,
        pg_increment: u32,
        pg_phase_next: u32,
        parameters: Ym2413PgParameterLatch,
        lfo_vib_counter: u8,
    ) -> Ym2413PgSerialService {
        let mut service = Ym2413PgSerialService::new(bank);
        let mut state = service.export_state();
        state.pipeline = Ym2413PgSerialPipelineState {
            format_version: YM2413_PG_SERIAL_PIPELINE_STATE_VERSION,
            pg_increment,
            pg_phase_next,
            pg_output: 0,
            lfo_vib_counter,
            parameters,
        };
        state.format_version = YM2413_PG_SERIAL_STATE_VERSION;
        service = Ym2413PgSerialService::restore_state(state).unwrap();
        service
    }

    #[test]
    fn pg_output_latch_preserves_bits_above_the_ten_bit_sine_address() {
        let mut bank = initial_bank(0);
        let mut bank_state = bank.export_state();
        bank_state.channels[1].modulator_phase.phase = 0x0008_4000;
        bank = Ym2413SynthesisBank::restore_state(bank_state).unwrap();
        let mut service = service_with_pipeline(bank, 0, 0, Ym2413PgParameterLatch::RESET, 0);

        service.before_eg_update(&context(0, [0; 64], 0)).unwrap();
        assert_eq!(service.pg_output(), 0x0420);

        let snapshot = service.export_state();
        let restored = Ym2413PgSerialService::restore_state(snapshot).unwrap();
        assert_eq!(restored.pg_output(), 0x0420);
        assert_eq!(restored.export_state(), snapshot);
    }

    #[test]
    fn phase_write_and_phase_output_use_distinct_physical_slot_indices() {
        let mut service = service_with_pipeline(
            initial_bank(0),
            0x1000,
            0x2000,
            Ym2413PgParameterLatch::RESET,
            0,
        );
        let context = context(0, [0; 64], 0);
        service.before_eg_update(&context).unwrap();

        let written = nuked_pg_phase_slot_descriptor(17).unwrap();
        let output = nuked_pg_phase_slot_descriptor(0).unwrap();
        assert_eq!(
            service
                .synthesis()
                .phase_accumulator(
                    written.channel,
                    matches!(written.operator, crate::Ym2413Operator::Carrier),
                )
                .unwrap(),
            0x3000
        );
        assert_eq!(
            service
                .synthesis()
                .phase_accumulator(
                    output.channel,
                    matches!(output.operator, crate::Ym2413Operator::Carrier),
                )
                .unwrap(),
            0x1000
        );
        assert_eq!(service.pg_phase_next(), 0x1000);
        assert_eq!(service.pg_output(), 8);
    }

    #[test]
    fn key_on_reset_changes_only_the_delayed_next_phase_latch() {
        let mut service = service_with_pipeline(
            initial_bank(0),
            0x0200,
            0x1000,
            Ym2413PgParameterLatch::RESET,
            0,
        );
        let context = context(0, [0; 64], 0x04);
        service.before_eg_update(&context).unwrap();

        let output = nuked_pg_phase_slot_descriptor(0).unwrap();
        let written = nuked_pg_phase_slot_descriptor(17).unwrap();
        assert_eq!(
            service
                .synthesis()
                .phase_accumulator(
                    output.channel,
                    matches!(output.operator, crate::Ym2413Operator::Carrier),
                )
                .unwrap(),
            0x1000
        );
        assert_eq!(
            service
                .synthesis()
                .phase_accumulator(
                    written.channel,
                    matches!(written.operator, crate::Ym2413Operator::Carrier),
                )
                .unwrap(),
            0x1200
        );
        assert_eq!(service.pg_phase_next(), 0);
    }

    #[test]
    fn phase_increment_uses_previous_parameter_latch_before_latching_next_slot() {
        let mut registers = [0; 64];
        registers[..8].copy_from_slice(&[0x41, 0x02, 0, 0, 0, 0, 0, 0]);
        registers[0x11] = 0xc0;
        registers[0x21] = 0x0d;
        registers[0x31] = 0x00;

        let parameters = Ym2413PgParameterLatch {
            fnum: 0x1c0,
            block: 6,
            multiplier: 2,
            pm_enabled: true,
        };
        let mut service = service_with_pipeline(initial_bank(0), 0, 0, parameters, 3);
        let context = context(0, registers, 0);
        service.before_eg_update(&context).unwrap();
        let (tick, post) = tick(0);
        service.after_eg_update(&context, tick, post).unwrap();

        assert_eq!(
            service.pg_increment(),
            PhaseGenerator::phase_step(0x1c0, 6, 2, true, 3).unwrap()
        );
        assert_eq!(
            service.pipeline.parameters,
            Ym2413PgParameterLatch {
                fnum: 0x1c0,
                block: 6,
                multiplier: 1,
                pm_enabled: true,
            }
        );
    }

    #[test]
    fn cycle_seventeen_advances_vibrato_and_pm_latches_after_increment() {
        let mut service =
            service_with_pipeline(initial_bank(0x03ff), 0, 0, Ym2413PgParameterLatch::RESET, 2);
        let context = context(17, [0; 64], 0x08);
        service.before_eg_update(&context).unwrap();
        let (tick, post) = tick(17);
        service.after_eg_update(&context, tick, post).unwrap();
        assert_eq!(service.lfo_vib_counter(), 3);
        assert_eq!(service.synthesis().pm_lfo_counter(), 0x0400);
    }

    #[test]
    fn test_mode_lfo_reset_is_applied_on_each_serial_cycle() {
        let mut service =
            service_with_pipeline(initial_bank(0x0555), 0, 0, Ym2413PgParameterLatch::RESET, 6);
        let context = context(3, [0; 64], 0x02);
        service.before_eg_update(&context).unwrap();
        let (tick, post) = tick(3);
        service.after_eg_update(&context, tick, post).unwrap();
        assert_eq!(service.lfo_vib_counter(), 0);
        assert_eq!(service.synthesis().pm_lfo_counter(), 0);
    }

    #[test]
    fn test_mode_lfo_reset_occurs_after_cycle_seventeen_update() {
        let mut service =
            service_with_pipeline(initial_bank(0x03ff), 0, 0, Ym2413PgParameterLatch::RESET, 6);
        let context = context(17, [0; 64], 0x0a);
        service.before_eg_update(&context).unwrap();
        let (tick, post) = tick(17);
        service.after_eg_update(&context, tick, post).unwrap();
        assert_eq!(service.lfo_vib_counter(), 0);
        assert_eq!(service.synthesis().pm_lfo_counter(), 0);
    }

    #[test]
    fn rhythm_mode_is_rejected_before_any_melodic_pg_state_is_mutated() {
        let mut service = Ym2413PgSerialService::new(initial_bank(0));
        let before = service.export_state();
        let mut registers = [0; 64];
        registers[0x0e] = 0x20;
        let context = context(0, registers, 0);
        assert_eq!(
            service.before_eg_update(&context),
            Err(Ym2413PgSerialError::UnsupportedRhythmMode {
                cycle: 0,
                channel: 1,
            })
        );
        assert_eq!(service.export_state(), before);
    }

    #[test]
    fn delayed_rhythm_enable_latches_are_rejected_until_they_drain() {
        for (rm_enable, rm_select) in [(0x40, 6), (0, 2)] {
            let mut service = Ym2413PgSerialService::new(initial_bank(0));
            let before = service.export_state();
            let mut context = context(0, [0; 64], 0);
            context.rm_enable = rm_enable;
            context.rm_select = rm_select;

            assert!(matches!(
                service.before_eg_update(&context),
                Err(Ym2413PgSerialError::UnsupportedRhythmMode { cycle: 0, .. })
            ));
            assert_eq!(service.export_state(), before);
        }
    }

    #[test]
    fn service_clone_is_independent_and_snapshot_restores_every_pg_latch() {
        let service = service_with_pipeline(
            initial_bank(1234),
            0x12345,
            0x23456,
            Ym2413PgParameterLatch {
                fnum: 0x1c0,
                block: 6,
                multiplier: 2,
                pm_enabled: true,
            },
            5,
        );
        let snapshot = service.export_state();
        let restored = Ym2413PgSerialService::restore_state(snapshot).unwrap();
        assert_eq!(restored.export_state(), snapshot);

        let mut clone = service.clone();
        clone.before_eg_update(&context(0, [0; 64], 0)).unwrap();
        assert_eq!(service.export_state(), snapshot);
        assert_ne!(clone.export_state(), snapshot);
    }

    #[test]
    fn service_rejects_invalid_pipeline_snapshot_fields() {
        let service = Ym2413PgSerialService::new(initial_bank(0));
        let mut state = service.export_state();
        state.pipeline.lfo_vib_counter = 8;
        assert_eq!(
            Ym2413PgSerialService::restore_state(state),
            Err(Ym2413PgSerialError::InvalidState)
        );
        state = service.export_state();
        state.pipeline.parameters.fnum = 0x0200;
        assert_eq!(
            Ym2413PgSerialService::restore_state(state),
            Err(Ym2413PgSerialError::InvalidState)
        );
    }

    #[test]
    fn restore_state_enforces_the_reachable_pg_increment_bound() {
        let service = Ym2413PgSerialService::new(initial_bank(0));
        let mut state = service.export_state();
        state.pipeline.pg_increment = YM2413_PG_INCREMENT_MAX + 1;
        assert_eq!(
            Ym2413PgSerialService::restore_state(state),
            Err(Ym2413PgSerialError::InvalidState)
        );

        let mut state = service.export_state();
        state.pipeline.pg_increment = YM2413_PG_INCREMENT_MAX;
        assert!(Ym2413PgSerialService::restore_state(state).is_ok());
    }

    #[test]
    fn maximum_pg_increment_is_reachable_from_legal_parameters() {
        // FNUM=511 selects PM row 7; position 2 contributes +7.
        // ((511 * 2 + 7) * 2^7 / 2) * 30 / 2 = 987_840.
        assert_eq!(
            PhaseGenerator::phase_step(0x01ff, 7, 14, true, 2),
            Ok(YM2413_PG_INCREMENT_MAX)
        );
        assert_eq!(
            PhaseGenerator::phase_step(0x01ff, 7, 15, true, 2),
            Ok(YM2413_PG_INCREMENT_MAX)
        );
    }

    #[derive(Clone)]
    struct FailingPgService {
        inner: Ym2413PgSerialService,
        fail_cycle: Option<u8>,
    }

    impl Ym2413SlotService for FailingPgService {
        type Error = u8;

        fn before_eg_update(
            &mut self,
            context: &Ym2413SlotServiceContext,
        ) -> Result<(), Self::Error> {
            self.inner
                .before_eg_update(context)
                .map_err(|_| context.event.cycle)
        }

        fn after_eg_update(
            &mut self,
            context: &Ym2413SlotServiceContext,
            tick: EnvelopeSequencerTick,
            post_state: EnvelopeSequencerState,
        ) -> Result<(), Self::Error> {
            self.inner
                .after_eg_update(context, tick, post_state)
                .map_err(|_| context.event.cycle)?;
            if self.fail_cycle == Some(context.event.cycle) {
                Err(context.event.cycle)
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn core_and_pg_service_snapshot_restore_and_transactional_retry_are_equivalent() {
        let initial = initial_bank(0);
        let mut whole_core = Ym2413Core::new();
        let mut whole_service = Ym2413PgSerialService::new(initial.clone());
        let total_clocks = 18 * YM2413_INPUT_CLOCKS_PER_SLOT;
        whole_core
            .advance_input_clocks_with_slot_service(total_clocks, &mut whole_service)
            .unwrap();

        let mut checkpoint_core = Ym2413Core::new();
        let mut checkpoint_service = Ym2413PgSerialService::new(initial.clone());
        checkpoint_core
            .advance_input_clocks_with_slot_service(37, &mut checkpoint_service)
            .unwrap();
        let mut restored_core = Ym2413Core::restore_state(checkpoint_core.export_state()).unwrap();
        let mut restored_service =
            Ym2413PgSerialService::restore_state(checkpoint_service.export_state()).unwrap();
        restored_core
            .advance_input_clocks_with_slot_service(total_clocks - 37, &mut restored_service)
            .unwrap();
        assert_eq!(whole_core.export_state(), restored_core.export_state());
        assert_eq!(
            whole_service.export_state(),
            restored_service.export_state()
        );

        let mut rollback_core = Ym2413Core::new();
        let mut rollback_service = FailingPgService {
            inner: Ym2413PgSerialService::new(initial.clone()),
            fail_cycle: Some(3),
        };
        let core_before = rollback_core.export_state();
        let service_before = rollback_service.inner.export_state();
        assert_eq!(
            rollback_core.advance_input_clocks_with_slot_service(
                4 * YM2413_INPUT_CLOCKS_PER_SLOT,
                &mut rollback_service,
            ),
            Err(Ym2413SlotServiceAdvanceError::Service(3))
        );
        assert_eq!(rollback_core.export_state(), core_before);
        assert_eq!(rollback_service.inner.export_state(), service_before);

        rollback_service.fail_cycle = None;
        rollback_core
            .advance_input_clocks_with_slot_service(
                4 * YM2413_INPUT_CLOCKS_PER_SLOT,
                &mut rollback_service,
            )
            .unwrap();

        let mut reference_core = Ym2413Core::new();
        let mut reference_service = Ym2413PgSerialService::new(initial);
        reference_core
            .advance_input_clocks_with_slot_service(
                4 * YM2413_INPUT_CLOCKS_PER_SLOT,
                &mut reference_service,
            )
            .unwrap();
        assert_eq!(rollback_core.export_state(), reference_core.export_state());
        assert_eq!(
            rollback_service.inner.export_state(),
            reference_service.export_state()
        );
    }

    #[test]
    fn core_and_serial_pg_service_are_equivalent_across_block_partitions() {
        let initial = initial_bank(0);
        let mut whole_core = Ym2413Core::new();
        let mut split_core = Ym2413Core::new();
        let mut whole = Ym2413PgSerialService::new(initial.clone());
        let mut split = Ym2413PgSerialService::new(initial);

        whole_core
            .advance_input_clocks_with_slot_service(18 * YM2413_INPUT_CLOCKS_PER_SLOT, &mut whole)
            .unwrap();
        for clocks in [1, 3, 4, 17, 29, 7, 11] {
            split_core
                .advance_input_clocks_with_slot_service(clocks, &mut split)
                .unwrap();
        }
        let elapsed = split_core.elapsed_input_clocks();
        let total = 18 * YM2413_INPUT_CLOCKS_PER_SLOT;
        if elapsed < total {
            split_core
                .advance_input_clocks_with_slot_service(total - elapsed, &mut split)
                .unwrap();
        }

        assert_eq!(whole_core.export_state(), split_core.export_state());
        assert_eq!(whole.export_state(), split.export_state());
    }
}
