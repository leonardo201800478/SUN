//! Composed serial FM service: PG, operator pipeline, core-owned EG boundary.
//!
//! This service preserves the core as the sole EG authority. It runs the
//! pre-EG OPLL stages, lets Ym2413Core perform exactly one EG tick, then runs
//! DoLFO and PreparePatch2 in the post-EG hook.

use core::fmt;

use crate::envelope_sequencer::{EnvelopeSequencerState, EnvelopeSequencerTick};
use crate::operator_serial::{
    Ym2413OperatorSerialError, Ym2413OperatorSerialService, Ym2413OperatorSerialState,
};
use crate::pg_serial::{
    Ym2413PgSerialError, Ym2413PgSerialService, Ym2413PgSerialServiceState,
};
use crate::slot_service::{Ym2413SlotService, Ym2413SlotServiceContext};
use crate::synthesis::Ym2413SynthesisInitialState;

pub const YM2413_FM_SERIAL_STATE_VERSION: u16 = 1;

/// Errors raised by the composed serial FM service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413FmSerialError {
    InvalidState,
    Pg(Ym2413PgSerialError),
    Operator(Ym2413OperatorSerialError),
}

impl From<Ym2413PgSerialError> for Ym2413FmSerialError {
    fn from(value: Ym2413PgSerialError) -> Self {
        Self::Pg(value)
    }
}

impl From<Ym2413OperatorSerialError> for Ym2413FmSerialError {
    fn from(value: Ym2413OperatorSerialError) -> Self {
        Self::Operator(value)
    }
}

impl fmt::Display for Ym2413FmSerialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState => formatter.write_str("invalid YM2413 serial FM state"),
            Self::Pg(error) => write!(formatter, "YM2413 PG serial error: {error}"),
            Self::Operator(error) => write!(formatter, "YM2413 operator serial error: {error:?}"),
        }
    }
}

impl std::error::Error for Ym2413FmSerialError {}

/// Core/service error boundary for coarse-grained serial FM advancement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413FmSerialAdvanceError {
    Core(crate::Ym2413Error),
    Service(Ym2413FmSerialError),
}

impl fmt::Display for Ym2413FmSerialAdvanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => write!(formatter, "YM2413 core error: {error}"),
            Self::Service(error) => write!(formatter, "YM2413 FM service error: {error}"),
        }
    }
}

impl std::error::Error for Ym2413FmSerialAdvanceError {}

/// Complete resumable state for the composed PG + operator serial service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413FmSerialServiceState {
    pub format_version: u16,
    pub pg: Ym2413PgSerialServiceState,
    pub operator: Ym2413OperatorSerialState,
}

/// Composed FM service. PG and operator state are owned independently and
/// cloned transactionally by the core's slot-service block advancement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ym2413FmSerialService {
    pg: Ym2413PgSerialService,
    operator: Ym2413OperatorSerialService,
}

impl Ym2413FmSerialService {
    /// Compose a previously initialized PG service with a reset operator pipeline.
    pub const fn from_pg_service(pg: Ym2413PgSerialService) -> Self {
        Self {
            pg,
            operator: Ym2413OperatorSerialService::new(),
        }
    }

    /// Construct the complete serial FM service from a synthesis-bank state.
    pub fn with_initial_state(
        initial: Ym2413SynthesisInitialState,
    ) -> Result<Self, Ym2413FmSerialError> {
        Ok(Self::from_pg_service(
            Ym2413PgSerialService::with_initial_state(initial)?,
        ))
    }

    /// Return the PG service for phase-state inspection and diagnostics.
    pub const fn pg_service(&self) -> &Ym2413PgSerialService {
        &self.pg
    }

    /// Return the serial operator service for latch inspection and diagnostics.
    pub const fn operator_service(&self) -> &Ym2413OperatorSerialService {
        &self.operator
    }

    /// Export PG and operator latches as one resumable FM state.
    pub const fn export_state(&self) -> Ym2413FmSerialServiceState {
        Ym2413FmSerialServiceState {
            format_version: YM2413_FM_SERIAL_STATE_VERSION,
            pg: self.pg.export_state(),
            operator: self.operator.export_state(),
        }
    }

    /// Restore both serial pipelines atomically after validating their snapshots.
    pub fn restore_state(
        state: Ym2413FmSerialServiceState,
    ) -> Result<Self, Ym2413FmSerialError> {
        if state.format_version != YM2413_FM_SERIAL_STATE_VERSION {
            return Err(Ym2413FmSerialError::InvalidState);
        }
        Ok(Self {
            pg: Ym2413PgSerialService::restore_state(state.pg)?,
            operator: Ym2413OperatorSerialService::restore_state(state.operator)?,
        })
    }
}

impl Ym2413SlotService for Ym2413FmSerialService {
    type Error = Ym2413FmSerialError;

    fn before_eg_update(
        &mut self,
        context: &Ym2413SlotServiceContext,
    ) -> Result<(), Self::Error> {
        self.pg.ensure_melodic_mode(context)?;

        // PreparePatch1 latches the current slot's TL/KSL contribution.
        let next_eg_ksltl = self.operator.prepare_patch1(context)?;
        self.operator
            .sample_am_lfo_output(context.event.cycle)?;

        // OPLL_Clock order: PhaseGenerate -> Operator -> PhaseCalcIncrement.
        self.pg.generate_phase(context)?;
        let parameters = self.pg.parameter_latch();
        self.operator.clock_operator(
            context.event.cycle,
            self.pg.pg_output(),
            parameters.feedback,
        )?;
        self.pg.calculate_increment()?;

        // EnvelopeOutput -> EnvelopeKSLTL -> DoEG. The EG level is read from
        // the core's pre-update state; the core advances it once after this hook.
        self.operator
            .clock_envelope_output(context, parameters.am_enabled)?;
        self.operator.latch_eg_ksltl(next_eg_ksltl);
        self.operator.clock_eg_silent(context)?;
        Ok(())
    }

    fn after_eg_update(
        &mut self,
        context: &Ym2413SlotServiceContext,
        tick: EnvelopeSequencerTick,
        post_state: EnvelopeSequencerState,
    ) -> Result<(), Self::Error> {
        // DoLFO's AM side samples the PM counter before PG increments it.
        self.operator.clock_am_lfo(
            context.event.cycle,
            context.registers[0x0f],
            self.pg.pm_lfo_counter(),
        )?;

        // PG service performs the PM side of DoLFO and then PreparePatch2.
        <Ym2413PgSerialService as Ym2413SlotService>::after_eg_update(
            &mut self.pg,
            context,
            tick,
            post_state,
        )?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::{
        Ym2413FmSerialError, Ym2413FmSerialService, Ym2413FmSerialServiceState,
    };
    use crate::envelope::{
        AttackRate15Transition, DampTransitionConfig, DampTransitionCoordination, EnvelopeStage,
    };
    use crate::envelope_sequencer::{EnvelopeSequencer, EnvelopeSlotInput};
    use crate::pg_serial::{Ym2413PgParameterLatch, Ym2413PgSerialError};
    use crate::slot::nuked_slot_service_event;
    use crate::slot_service::{Ym2413SlotService, Ym2413SlotServiceContext};
    use crate::synthesis::{Ym2413ChannelInitialState, Ym2413SynthesisInitialState};
    use crate::{Ym2413Core, YM2413_INPUT_CLOCKS_PER_SLOT};

    fn initial_synthesis_state() -> Ym2413SynthesisInitialState {
        Ym2413SynthesisInitialState {
            channels: std::array::from_fn(|channel| Ym2413ChannelInitialState {
                phase: [0x1000 * channel as u32, 0x1800 * channel as u32],
                envelope_level: [0x7f; 2],
                envelope_stage: [EnvelopeStage::Damp; 2],
                envelope_key_on: [false; 2],
                feedback_history: [0; 2],
                previous_modulator_phase_index: None,
            }),
            pm_lfo_counter: 0x003f,
            am_lfo_phase: 0,
            damp_config: DampTransitionConfig::new(
                124,
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            )
            .unwrap(),
        }
    }

    fn seeded_service() -> Ym2413FmSerialService {
        let service =
            Ym2413FmSerialService::with_initial_state(initial_synthesis_state()).unwrap();
        let mut state: Ym2413FmSerialServiceState = service.export_state();
        state.pg.pipeline.pg_phase_next = 0x1234;
        state.pg.pipeline.pg_increment = 0x0045;
        state.pg.pipeline.lfo_vib_counter = 3;
        state.pg.pipeline.parameters = Ym2413PgParameterLatch {
            fnum: 0x01c0,
            block: 6,
            multiplier: 2,
            pm_enabled: true,
            feedback: 4,
            am_enabled: true,
        };
        state.operator.op_logsin = 0x0123;
        state.operator.op_neg = 0x02;
        state.operator.op_exp_m = 0x07fa;
        state.operator.op_exp_s = 1;
        state.operator.op_mod = 0x0120;
        state.operator.op_fbsum = 80;
        state.operator.op_fb1[3] = 120;
        state.operator.op_fb2[3] = -40;
        state.operator.eg_ksltl = 24;
        state.operator.eg_out = 32;
        state.operator.eg_silent = false;
        state.operator.lfo_am_counter = 0x01ab;
        state.operator.lfo_am_dir = true;
        state.operator.lfo_am_car = true;
        state.operator.lfo_am_step = true;
        state.operator.lfo_am_out = 7;
        Ym2413FmSerialService::restore_state(state).unwrap()
    }

    fn rhythm_context() -> Ym2413SlotServiceContext {
        let envelope = EnvelopeSequencer::new();
        let envelope_state = envelope.export_state();
        let registers = [0; crate::YM2413_REGISTER_COUNT];
        let event = nuked_slot_service_event(0, 0, envelope_state.eg_dokon).unwrap();
        Ym2413SlotServiceContext {
            event,
            registers,
            envelope_input: EnvelopeSlotInput::default(),
            envelope_state,
            rm_enable: 0,
            rm_select: 5,
        }
    }

    #[test]
    fn serial_fm_service_snapshot_restores_pg_and_operator_latches() {
        let service = seeded_service();
        let snapshot = service.export_state();
        let restored = Ym2413FmSerialService::restore_state(snapshot).unwrap();
        assert_eq!(restored.export_state(), snapshot);
    }

    #[test]
    fn serial_fm_service_rejects_invalid_snapshot_version() {
        let service = seeded_service();
        let mut snapshot = service.export_state();
        snapshot.format_version = 0;
        assert_eq!(
            Ym2413FmSerialService::restore_state(snapshot),
            Err(Ym2413FmSerialError::InvalidState)
        );
    }

    #[test]
    fn serial_fm_service_is_equivalent_for_continuous_and_partitioned_blocks() {
        let total = YM2413_INPUT_CLOCKS_PER_SLOT * 18 + 7;
        let mut whole_core = Ym2413Core::new();
        let mut whole_service = seeded_service();
        whole_core
            .advance_input_clocks_with_fm_service(total, &mut whole_service)
            .unwrap();

        let mut partitioned_core = Ym2413Core::new();
        let mut partitioned_service = seeded_service();
        let mut remaining = total;
        for block in [1_u64, 2, 3, 5, 8, 13, 21, 27, 34] {
            if remaining == 0 {
                break;
            }
            let clocks = block.min(remaining);
            partitioned_core
                .advance_input_clocks_with_fm_service(clocks, &mut partitioned_service)
                .unwrap();
            remaining -= clocks;
        }
        if remaining != 0 {
            partitioned_core
                .advance_input_clocks_with_fm_service(remaining, &mut partitioned_service)
                .unwrap();
        }

        assert_eq!(partitioned_core.export_state(), whole_core.export_state());
        assert_eq!(
            partitioned_service.export_state(),
            whole_service.export_state()
        );
    }

    #[test]
    fn serial_fm_service_does_not_advance_a_second_envelope_generator() {
        let clocks = YM2413_INPUT_CLOCKS_PER_SLOT * 18;
        let mut fm_core = Ym2413Core::new();
        let mut service = seeded_service();
        fm_core
            .advance_input_clocks_with_fm_service(clocks, &mut service)
            .unwrap();

        let mut core_only = Ym2413Core::new();
        core_only.advance_input_clocks(clocks).unwrap();
        assert_eq!(fm_core.export_state(), core_only.export_state());
    }

    #[test]
    fn serial_fm_service_rejects_rhythm_before_mutating_any_latch() {
        let mut service = seeded_service();
        let before = service.export_state();
        let error = <Ym2413FmSerialService as Ym2413SlotService>::before_eg_update(
            &mut service,
            &rhythm_context(),
        );
        assert_eq!(
            error,
            Err(Ym2413FmSerialError::Pg(
                Ym2413PgSerialError::UnsupportedRhythmMode {
                    cycle: 0,
                    channel: 1,
                }
            ))
        );
        assert_eq!(service.export_state(), before);
    }

    #[test]
    fn serial_fm_service_clone_owns_independent_operator_and_pg_state() {
        let original = seeded_service();
        let mut clone = original.clone();
        let before = original.export_state();
        <Ym2413FmSerialService as Ym2413SlotService>::before_eg_update(
            &mut clone,
            &rhythm_context(),
        )
        .unwrap_err();
        assert_eq!(original.export_state(), before);
        assert_eq!(clone.export_state(), before);
    }
}

