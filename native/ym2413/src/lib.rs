//! YM2413 register, clock, PG, EG, and operator-synthesis candidate components.
//!
//! These components are independently testable and do not yet form a complete
//! 18-slot FM pipeline or emit integrated YM2413 PCM.

pub mod am_lfo;
pub mod bus_timing;
pub mod channel;
pub mod envelope;
pub mod envelope_operator;
pub mod envelope_timer;
pub mod instrument;
pub mod key_scale;
pub mod operator;
pub mod output;
pub mod phase;
pub mod pm_lfo;
pub mod rhythm;
pub mod rhythm_channel;
pub mod rhythm_output;
pub mod rhythm_phase;
pub mod slot;
pub mod synthesis;
pub mod synthesis_tables;
pub mod timed_bus;
pub use am_lfo::{
    AmLfo, AmLfoError, AmLfoState, YM2413_AM_LFO_LEVELS, YM2413_AM_LFO_PERIOD_SAMPLES,
    YM2413_AM_LFO_SAMPLES_PER_STEP, YM2413_AM_LFO_STATE_VERSION, YM2413_AM_LFO_TABLE_LENGTH,
};
pub use bus_timing::{
    YM2413_ADDRESS_WRITE_WAIT_CLOCKS, YM2413_BUS_TIMING_STATE_VERSION,
    YM2413_DATA_WRITE_WAIT_CLOCKS, Ym2413BusTiming, Ym2413BusTimingError, Ym2413BusTimingState,
    Ym2413BusWriteKind,
};
pub use channel::{
    ChannelSynthesisConfig, ChannelSynthesisError, OperatorPhaseParameters,
    decode_channel_synthesis,
};
pub use envelope_operator::{
    EnvelopeControlPhase, EnvelopeOperator, EnvelopeOperatorError, EnvelopeOperatorStage,
    EnvelopeOperatorState, EnvelopeOperatorTick, YM2413_ENVELOPE_OPERATOR_MAX_LEVEL,
    YM2413_ENVELOPE_OPERATOR_STATE_VERSION,
};
pub use envelope::{
    AttackRate15Transition, DampTransitionConfig, DampTransitionCoordination, EnvelopeGenerator,
    EnvelopeGeneratorError, EnvelopeGeneratorState, EnvelopeParameters, EnvelopeStage,
    EnvelopeTickResult, YM2413_EG_LEVELS, YM2413_EG_MAX_LEVEL, YM2413_EG_STATE_VERSION,
};
pub use instrument::{
    ChannelRegisterState, InstrumentError, InstrumentPatch, OperatorPatch, YM2413_PRESET_NAMES,
    YM2413_PRESET_PATCH_LICENSE, YM2413_PRESET_PATCH_SOURCE, YM2413_PRESET_PATCH_SOURCE_COMMIT,
    YM2413_PRESET_PATCHES_CANDIDATE, YM2413_PRESET_PATCHES_HARDWARE_VERIFIED,
    decode_channel_registers, decode_patch_registers, decode_user_patch, preset_name, preset_patch,
    resolve_instrument, rhythm_mode_enabled,
};
pub use key_scale::{KeyScaleError, YM2413_KSL1_TABLE, key_scale_attenuation};
pub use operator::{
    OperatorPair, OperatorPairError, OperatorPairParameters, OperatorPairSample, OperatorPairState,
    YM2413_OPERATOR_PAIR_STATE_VERSION, exponential_lookup, log_sine_lookup,
};
pub use output::{
    YM2413_OUTPUT_ACTIVE_CLOCKS, YM2413_OUTPUT_NEUTRAL_CLOCKS, YM2413_OUTPUT_SLOT_CLOCKS,
    Ym2413OutputPin, Ym2413OutputPulse, Ym2413OutputRoutingError, Ym2413OutputSlotResult,
    Ym2413OutputSource, melody_output_for_slot, rhythm_output_event, rhythm_slot_result,
};
pub use phase::{
    PhaseGenerator, PhaseGeneratorError, PhaseGeneratorState, YM2413_ML_TABLE,
    YM2413_PG_STATE_VERSION, YM2413_PHASE_BITS, YM2413_PHASE_MASK, YM2413_PM_POSITIONS,
    YM2413_PM_TABLE,
};
pub use pm_lfo::{
    PhaseModulationLfo, PhaseModulationLfoError, PhaseModulationLfoState,
    YM2413_PM_LFO_COUNTER_BITS, YM2413_PM_LFO_COUNTER_MASK, YM2413_PM_LFO_PERIOD_SAMPLES,
    YM2413_PM_LFO_POSITIONS, YM2413_PM_LFO_SAMPLES_PER_POSITION, YM2413_PM_LFO_STATE_VERSION,
};
pub use rhythm::{
    YM2413_RHYTHM_HIHAT_NOISE_SLOT_INDEX, YM2413_RHYTHM_MODE_BIT, YM2413_RHYTHM_NOISE_LFSR_BITS,
    YM2413_RHYTHM_NOISE_LFSR_MASK, YM2413_RHYTHM_NOISE_LFSR_PERIOD, YM2413_RHYTHM_NOISE_LFSR_TAPS,
    YM2413_RHYTHM_NOISE_SLOTS_PER_ROTATION, YM2413_RHYTHM_NOISE_STATE_VERSION,
    YM2413_RHYTHM_SLOT_NOISE_SEQUENCER_STATE_VERSION, YM2413_RHYTHM_SLOT_NOISE_STATE_VERSION,
    YM2413_RHYTHM_SNARE_NOISE_SLOT_INDEX, YM2413_RHYTHM_TRIGGER_MASK, Ym2413RhythmChannelFrequency,
    Ym2413RhythmConfig, Ym2413RhythmNoiseError, Ym2413RhythmNoiseLfsr, Ym2413RhythmNoiseSlotSample,
    Ym2413RhythmNoiseState, Ym2413RhythmSlotNoiseLfsr, Ym2413RhythmSlotNoiseSequencer,
    Ym2413RhythmSlotNoiseSequencerState, Ym2413RhythmSlotNoiseState, Ym2413RhythmVoiceConfig,
    decode_rhythm_registers,
};
pub use rhythm_channel::{
    RhythmChannelSynthesisError, RhythmOperatorSynthesisConfig, RhythmVoiceSynthesisConfig,
    RhythmVoiceSynthesisError, Ym2413RhythmSynthesisConfig, decode_rhythm_synthesis,
};
pub use rhythm_output::{
    YM2413_DAC_CODE_MAX, YM2413_DAC_CODE_MIN, YM2413_RHYTHM_OUTPUT_DELAY_STATE_VERSION,
    Ym2413DacCode, Ym2413RhythmOutputDelay, Ym2413RhythmOutputDelayState, Ym2413RhythmOutputError,
    Ym2413RhythmOutputEvent, Ym2413RhythmOutputSlotResult,
};
pub use rhythm_phase::{
    RhythmNoisePhaseInputs, RhythmPhaseCandidateError, RhythmPhaseSource,
    YM2413_RHYTHM_PHASE_CANDIDATE_SOURCE, YM2413_TOP_CYMBAL_EQUIVALENT_PHASE_VARIANTS,
    hi_hat_phase_candidate, rhythm_noise_phase_candidate, rhythm_phase_mix_bit_candidate,
    snare_drum_phase_candidate, top_cymbal_phase_candidate,
};
pub use slot::{
    YM2413_SLOT_COUNT, YM2413_SLOT_MAP, Ym2413Operator, Ym2413RhythmVoice, Ym2413SlotDescriptor,
    slot_descriptor,
};
pub use synthesis::{
    YM2413_MELODIC_CHANNELS, YM2413_OPERATORS_PER_CHANNEL, YM2413_SYNTHESIS_STATE_VERSION,
    Ym2413ChannelInitialState, Ym2413ChannelSynthesisState, Ym2413RhythmVoicePhases,
    Ym2413RhythmVoiceSample, Ym2413SynthesisBank, Ym2413SynthesisBankState, Ym2413SynthesisError,
    Ym2413SynthesisInitialState,
};
pub use synthesis_tables::{YM2413_EXP_TABLE, YM2413_LOG_SINE_TABLE};
pub use timed_bus::{
    YM2413_TIMED_BUS_STATE_VERSION, Ym2413TimedBus, Ym2413TimedBusError, Ym2413TimedBusState,
};

use core::fmt;

/// Number of addressable YM2413 register locations.
pub const YM2413_REGISTER_COUNT: usize = 0x40;

/// The YM2413 exposes six address bits on its register interface.
pub const YM2413_ADDRESS_MASK: u8 = 0x3f;

/// Input clocks consumed by one sequential YM2413 operator slot.
pub const YM2413_INPUT_CLOCKS_PER_SLOT: u64 = 4;

/// Number of sequential operator slots in one complete rotation.
pub const YM2413_SLOTS_PER_ROTATION: u64 = 18;

/// Derived input-clock count for one complete 18-slot rotation.
pub const YM2413_INPUT_CLOCKS_PER_ROTATION: u64 =
    YM2413_INPUT_CLOCKS_PER_SLOT * YM2413_SLOTS_PER_ROTATION;

const YM2413_STATE_FORMAT_VERSION: u16 = 1;

/// Errors raised by the YM2413 register and clock foundation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413Error {
    /// The accumulated input-clock or rotation counter would overflow.
    ClockOverflow,
    /// The state version is not supported by this implementation.
    StateFormatVersionMismatch { expected: u16, actual: u16 },
    /// A snapshot contains an invalid latch, slot, phase, or clock relation.
    InvalidState,
}

impl fmt::Display for Ym2413Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClockOverflow => f.write_str("YM2413 input-clock counter overflow"),
            Self::StateFormatVersionMismatch { expected, actual } => write!(
                f,
                "YM2413 state version {actual} is unsupported (expected {expected})"
            ),
            Self::InvalidState => f.write_str("YM2413 register/clock state is inconsistent"),
        }
    }
}

impl std::error::Error for Ym2413Error {}

/// Complete snapshot of the current register and clock foundation.
///
/// The register image is internal device state. It is not a claim that the
/// YM2413 exposes readable registers on its external bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ym2413State {
    pub format_version: u16,
    pub registers: [u8; YM2413_REGISTER_COUNT],
    pub address_latch: u8,
    pub input_clock_phase: u8,
    pub slot_index: u8,
    pub elapsed_input_clocks: u64,
    pub completed_rotations: u64,
}

/// Deterministic YM2413 register and sequential-slot clock foundation.
///
/// This type is independent of the SMS bus and scheduler. The machine adapter
/// must supply input-clock advances at the correct scheduler deadline. A full
/// rotation is a timing boundary only; this stage does not emit PCM samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ym2413Core {
    registers: [u8; YM2413_REGISTER_COUNT],
    address_latch: u8,
    input_clock_phase: u8,
    slot_index: u8,
    elapsed_input_clocks: u64,
    completed_rotations: u64,
}

impl Default for Ym2413Core {
    fn default() -> Self {
        Self::new()
    }
}

impl Ym2413Core {
    /// Construct a reset YM2413 register/clock foundation.
    pub const fn new() -> Self {
        Self {
            registers: [0; YM2413_REGISTER_COUNT],
            address_latch: 0,
            input_clock_phase: 0,
            slot_index: 0,
            elapsed_input_clocks: 0,
            completed_rotations: 0,
        }
    }

    /// Reset the logical register image, address latch, and sequential clock phase.
    ///
    /// The machine adapter remains responsible for the documented external reset-pulse timing.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Latch the six-bit register address presented on the address port.
    pub fn write_address(&mut self, address: u8) {
        self.address_latch = address & YM2413_ADDRESS_MASK;
    }

    /// Write data to the currently latched register.
    ///
    /// Bus wait intervals and timestamp ordering are enforced by the future
    /// SMS adapter; this pure core applies the write when called.
    pub fn write_data(&mut self, value: u8) {
        self.registers[usize::from(self.address_latch)] = value;
    }

    /// Return the current address latch for deterministic diagnostics.
    pub const fn address_latch(&self) -> u8 {
        self.address_latch
    }

    /// Return the next sequential operator slot index (0..17).
    pub const fn slot_index(&self) -> u8 {
        self.slot_index
    }

    /// Describe the next serial operator slot without advancing the clock.
    ///
    /// The returned descriptor follows the Yamaha Table II-3 ordering. It
    /// identifies channel/operator ownership only; PG, EG and operator stage
    /// offsets are not inferred by this accessor.
    pub const fn next_slot_descriptor(&self) -> Option<Ym2413SlotDescriptor> {
        slot_descriptor(self.slot_index)
    }

    /// Return input clocks accumulated within the current four-clock slot.
    pub const fn input_clock_phase(&self) -> u8 {
        self.input_clock_phase
    }

    /// Return total input clocks elapsed since reset.
    pub const fn elapsed_input_clocks(&self) -> u64 {
        self.elapsed_input_clocks
    }

    /// Return complete 18-slot rotations elapsed since reset.
    pub const fn completed_rotations(&self) -> u64 {
        self.completed_rotations
    }

    /// Advance the sequential slot clock by an exact number of input clocks.
    ///
    /// Returns the number of complete 18-slot rotations crossed by this call.
    /// The result is independent of how the caller partitions the clock block.
    /// It is a timing boundary, not an audio sample, because synthesis has not
    /// been implemented in this foundation stage.
    pub fn advance_input_clocks(&mut self, clocks: u64) -> Result<u64, Ym2413Error> {
        let next_elapsed = self
            .elapsed_input_clocks
            .checked_add(clocks)
            .ok_or(Ym2413Error::ClockOverflow)?;

        let total_slot_clocks = u128::from(self.input_clock_phase) + u128::from(clocks);
        let completed_slots = total_slot_clocks / u128::from(YM2413_INPUT_CLOCKS_PER_SLOT);
        let next_clock_phase =
            u8::try_from(total_slot_clocks % u128::from(YM2413_INPUT_CLOCKS_PER_SLOT))
                .map_err(|_| Ym2413Error::InvalidState)?;

        let total_slots = u128::from(self.slot_index) + completed_slots;
        let rotations = u64::try_from(total_slots / u128::from(YM2413_SLOTS_PER_ROTATION))
            .map_err(|_| Ym2413Error::ClockOverflow)?;
        let next_completed_rotations = self
            .completed_rotations
            .checked_add(rotations)
            .ok_or(Ym2413Error::ClockOverflow)?;
        let next_slot = u8::try_from(total_slots % u128::from(YM2413_SLOTS_PER_ROTATION))
            .map_err(|_| Ym2413Error::InvalidState)?;

        self.elapsed_input_clocks = next_elapsed;
        self.input_clock_phase = next_clock_phase;
        self.slot_index = next_slot;
        self.completed_rotations = next_completed_rotations;

        Ok(rotations)
    }

    /// Export the complete register and sequential clock state.
    pub fn export_state(&self) -> Ym2413State {
        Ym2413State {
            format_version: YM2413_STATE_FORMAT_VERSION,
            registers: self.registers,
            address_latch: self.address_latch,
            input_clock_phase: self.input_clock_phase,
            slot_index: self.slot_index,
            elapsed_input_clocks: self.elapsed_input_clocks,
            completed_rotations: self.completed_rotations,
        }
    }

    /// Restore a snapshot after validating its version and clock arithmetic.
    pub fn restore_state(state: Ym2413State) -> Result<Self, Ym2413Error> {
        if state.format_version != YM2413_STATE_FORMAT_VERSION {
            return Err(Ym2413Error::StateFormatVersionMismatch {
                expected: YM2413_STATE_FORMAT_VERSION,
                actual: state.format_version,
            });
        }
        if state.address_latch & !YM2413_ADDRESS_MASK != 0
            || u64::from(state.input_clock_phase) >= YM2413_INPUT_CLOCKS_PER_SLOT
            || u64::from(state.slot_index) >= YM2413_SLOTS_PER_ROTATION
        {
            return Err(Ym2413Error::InvalidState);
        }

        let expected_elapsed = u128::from(state.completed_rotations)
            * u128::from(YM2413_INPUT_CLOCKS_PER_ROTATION)
            + u128::from(state.slot_index) * u128::from(YM2413_INPUT_CLOCKS_PER_SLOT)
            + u128::from(state.input_clock_phase);
        if expected_elapsed != u128::from(state.elapsed_input_clocks) {
            return Err(Ym2413Error::InvalidState);
        }

        Ok(Self {
            registers: state.registers,
            address_latch: state.address_latch,
            input_clock_phase: state.input_clock_phase,
            slot_index: state.slot_index,
            elapsed_input_clocks: state.elapsed_input_clocks,
            completed_rotations: state.completed_rotations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        YM2413_INPUT_CLOCKS_PER_ROTATION, YM2413_INPUT_CLOCKS_PER_SLOT, YM2413_SLOT_COUNT,
        YM2413_SLOT_MAP, Ym2413Core, Ym2413Error, Ym2413State,
    };

    #[test]
    fn address_latch_masks_to_six_bits() {
        let mut core = Ym2413Core::new();

        core.write_address(0xe7);

        assert_eq!(core.address_latch(), 0x27);
    }

    #[test]
    fn data_write_targets_only_the_latched_register() {
        let mut core = Ym2413Core::new();

        core.write_address(0x21);
        core.write_data(0x9a);
        core.write_address(0x32);
        core.write_data(0x47);

        let state = core.export_state();
        assert_eq!(state.registers[0x21], 0x9a);
        assert_eq!(state.registers[0x32], 0x47);
        assert_eq!(state.registers[0x20], 0);
    }

    #[test]
    fn one_complete_rotation_requires_seventy_two_input_clocks() {
        let mut core = Ym2413Core::new();

        assert_eq!(
            core.advance_input_clocks(YM2413_INPUT_CLOCKS_PER_ROTATION - 1),
            Ok(0)
        );
        assert_eq!(core.completed_rotations(), 0);
        assert_eq!(core.advance_input_clocks(1), Ok(1));
        assert_eq!(core.completed_rotations(), 1);
        assert_eq!(core.slot_index(), 0);
        assert_eq!(core.input_clock_phase(), 0);
    }

    #[test]
    fn clock_advance_is_invariant_under_block_partitioning() {
        let mut continuous = Ym2413Core::new();
        let mut chunked = Ym2413Core::new();

        continuous.advance_input_clocks(5_003).unwrap();
        for clocks in [1, 3, 4, 17, 72, 511, 1_024, 3_371] {
            chunked.advance_input_clocks(clocks).unwrap();
        }

        assert_eq!(continuous.export_state(), chunked.export_state());
    }

    #[test]
    fn snapshot_restore_preserves_registers_and_partial_slot_phase() {
        let mut core = Ym2413Core::new();
        core.write_address(0x18);
        core.write_data(0x73);
        core.advance_input_clocks(77).unwrap();

        let restored = Ym2413Core::restore_state(core.export_state()).unwrap();

        assert_eq!(restored.export_state(), core.export_state());
        assert_eq!(restored.slot_index(), 1);
        assert_eq!(restored.input_clock_phase(), 1);
    }

    #[test]
    fn restore_rejects_inconsistent_clock_state() {
        let mut core = Ym2413Core::new();
        core.advance_input_clocks(73).unwrap();
        let mut state: Ym2413State = core.export_state();
        state.elapsed_input_clocks += 1;

        assert_eq!(
            Ym2413Core::restore_state(state),
            Err(Ym2413Error::InvalidState)
        );
    }

    #[test]
    fn reset_clears_registers_latch_and_clock_phase() {
        let mut core = Ym2413Core::new();
        core.write_address(0x10);
        core.write_data(0xff);
        core.advance_input_clocks(91).unwrap();

        core.reset();

        let state = core.export_state();
        assert_eq!(state.registers, [0; 0x40]);
        assert_eq!(state.address_latch, 0);
        assert_eq!(state.input_clock_phase, 0);
        assert_eq!(state.slot_index, 0);
        assert_eq!(state.elapsed_input_clocks, 0);
        assert_eq!(state.completed_rotations, 0);
    }

    #[test]
    fn next_slot_descriptor_tracks_the_serial_clock_and_wraps_after_eighteen_slots() {
        let mut core = Ym2413Core::new();

        for expected_slot in 0..18_u8 {
            let descriptor = core.next_slot_descriptor().unwrap();
            assert_eq!(descriptor.slot_number, expected_slot + 1);
            core.advance_input_clocks(YM2413_INPUT_CLOCKS_PER_SLOT)
                .unwrap();
        }

        assert_eq!(core.slot_index(), 0);
        assert_eq!(core.next_slot_descriptor().unwrap().slot_number, 1);
    }

    #[test]
    fn slot_descriptor_changes_only_at_four_clock_boundaries_and_wraps_after_one_rotation() {
        let mut core = Ym2413Core::new();

        for (slot_index, expected) in YM2413_SLOT_MAP.iter().copied().enumerate() {
            assert_eq!(core.next_slot_descriptor(), Some(expected));

            for clock_in_slot in 1..=YM2413_INPUT_CLOCKS_PER_SLOT {
                let crossed_rotation = slot_index + 1 == YM2413_SLOT_COUNT
                    && clock_in_slot == YM2413_INPUT_CLOCKS_PER_SLOT;
                assert_eq!(
                    core.advance_input_clocks(1),
                    Ok(if crossed_rotation { 1 } else { 0 })
                );

                if clock_in_slot < YM2413_INPUT_CLOCKS_PER_SLOT {
                    assert_eq!(core.next_slot_descriptor(), Some(expected));
                    assert_eq!(core.input_clock_phase(), clock_in_slot as u8);
                } else {
                    let next_slot = (slot_index + 1) % YM2413_SLOT_COUNT;
                    assert_eq!(
                        core.next_slot_descriptor(),
                        Some(YM2413_SLOT_MAP[next_slot])
                    );
                    assert_eq!(core.input_clock_phase(), 0);
                }
            }
        }

        assert_eq!(
            core.elapsed_input_clocks(),
            YM2413_INPUT_CLOCKS_PER_ROTATION
        );
        assert_eq!(core.completed_rotations(), 1);
        assert_eq!(core.slot_index(), 0);
    }

    #[test]
    fn snapshot_restore_preserves_slot_boundary_state_at_every_clock_offset() {
        for offset in 0..YM2413_INPUT_CLOCKS_PER_ROTATION {
            let mut core = Ym2413Core::new();
            core.advance_input_clocks(offset).unwrap();

            let expected_descriptor = core.next_slot_descriptor();
            let expected_phase = core.input_clock_phase();
            let expected_rotation = core.completed_rotations();
            let restored = Ym2413Core::restore_state(core.export_state()).unwrap();

            assert_eq!(restored.next_slot_descriptor(), expected_descriptor);
            assert_eq!(restored.input_clock_phase(), expected_phase);
            assert_eq!(restored.completed_rotations(), expected_rotation);
            assert_eq!(restored.elapsed_input_clocks(), offset);
        }
    }
}
