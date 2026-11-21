//! Chip-wide YM2413 amplitude-modulation LFO.
//!
//! The phase advances in YM2413 output-sample units. The Scheduler/caller owns
//! the sample cadence; this component does not create an independent clock.

pub const YM2413_AM_LFO_TABLE_LENGTH: u16 = 210;
pub const YM2413_AM_LFO_SAMPLES_PER_STEP: u16 = 64;
pub const YM2413_AM_LFO_PERIOD_SAMPLES: u16 =
    YM2413_AM_LFO_TABLE_LENGTH * YM2413_AM_LFO_SAMPLES_PER_STEP;
pub const YM2413_AM_LFO_LEVELS: u8 = 14;
pub const YM2413_AM_LFO_STATE_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmLfoError {
    InvalidPhase(u16),
    InvalidState,
}

/// Versioned phase for the chip-wide AM LFO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmLfoState {
    pub format_version: u16,
    /// Position in the 13,440-output-sample cycle.
    pub phase: u16,
}

/// YM2413 AM LFO candidate derived from the 2015-11-28 silicon measurements.
///
/// The original 210-step triangle is represented algorithmically. Each
/// position is held for 64 output samples and its amplitude is the original
/// 0..26 value with the low bit discarded, yielding 14 levels (0..13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmLfo {
    phase: u16,
}

impl AmLfo {
    /// Construct with an explicit phase; the hardware reset phase is unknown.
    pub const fn with_phase(phase: u16) -> Result<Self, AmLfoError> {
        if phase >= YM2413_AM_LFO_PERIOD_SAMPLES {
            return Err(AmLfoError::InvalidPhase(phase));
        }
        Ok(Self { phase })
    }

    /// Return the current AM attenuation level (0..13).
    pub const fn current_value(&self) -> u8 {
        let table_index = self.phase / YM2413_AM_LFO_SAMPLES_PER_STEP;
        am_lfo_table_value(table_index)
    }

    /// Return the current sample phase within the AM LFO cycle.
    pub const fn phase(&self) -> u16 {
        self.phase
    }

    /// Reset the candidate AM LFO phase to its deterministic reset origin.
    pub fn reset(&mut self) {
        self.phase = 0;
    }

    /// Advance by one YM2413 output sample.
    pub fn advance_one(&mut self) {
        self.advance_samples(1);
    }

    /// Advance by an arbitrary number of YM2413 output samples.
    ///
    /// This is intentionally independent of input-clock advancement. The
    /// caller must invoke it at the YM2413 sample boundary (one 72-clock
    /// operator rotation), preserving the Scheduler as time authority.
    pub fn advance_samples(&mut self, samples: u32) {
        let delta = samples % u32::from(YM2413_AM_LFO_PERIOD_SAMPLES);
        let next = u32::from(self.phase) + delta;
        self.phase = (next % u32::from(YM2413_AM_LFO_PERIOD_SAMPLES)) as u16;
    }

    /// Export the current LFO phase for an aggregate YM2413 snapshot.
    pub const fn export_state(&self) -> AmLfoState {
        AmLfoState {
            format_version: YM2413_AM_LFO_STATE_VERSION,
            phase: self.phase,
        }
    }

    /// Restore an AM LFO snapshot after validating version and phase.
    pub const fn restore_state(state: AmLfoState) -> Result<Self, AmLfoError> {
        if state.format_version != YM2413_AM_LFO_STATE_VERSION
            || state.phase >= YM2413_AM_LFO_PERIOD_SAMPLES
        {
            return Err(AmLfoError::InvalidState);
        }
        Ok(Self { phase: state.phase })
    }
}

const fn am_lfo_table_value(index: u16) -> u8 {
    if index < 7 {
        0
    } else if index < 107 {
        let original_level = ((index - 7) / 4) + 1;
        (original_level / 2) as u8
    } else if index < 110 {
        13
    } else {
        let original_level = 25 - ((index - 110) / 4);
        (original_level / 2) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AmLfo, AmLfoError, AmLfoState, YM2413_AM_LFO_LEVELS, YM2413_AM_LFO_PERIOD_SAMPLES,
        YM2413_AM_LFO_SAMPLES_PER_STEP, YM2413_AM_LFO_STATE_VERSION, YM2413_AM_LFO_TABLE_LENGTH,
    };

    #[test]
    fn measured_am_lfo_shape_has_fourteen_levels_and_expected_turning_points() {
        let mut lfo = AmLfo::with_phase(0).unwrap();
        assert_eq!(lfo.current_value(), 0);

        lfo.advance_samples(11 * 64);
        assert_eq!(lfo.current_value(), 1);

        lfo.advance_samples(8 * 64);
        assert_eq!(lfo.current_value(), 2);

        lfo.advance_samples((107 - 19) * 64);
        assert_eq!(lfo.current_value(), 13);

        lfo.advance_samples(3 * 64);
        assert_eq!(lfo.current_value(), 12);
        assert_eq!(YM2413_AM_LFO_LEVELS, 14);
    }

    #[test]
    fn each_table_position_is_held_for_exactly_sixty_four_samples() {
        let mut lfo = AmLfo::with_phase(704).unwrap();
        assert_eq!(lfo.current_value(), 1);
        lfo.advance_samples(63);
        assert_eq!(lfo.phase(), 767);
        assert_eq!(lfo.current_value(), 1);
        lfo.advance_one();
        assert_eq!(lfo.phase(), 768);
        assert_eq!(lfo.current_value(), 1);
    }

    #[test]
    fn peak_level_is_held_for_three_table_positions() {
        let mut lfo = AmLfo::with_phase(107 * 64).unwrap();
        assert_eq!(lfo.current_value(), 13);
        lfo.advance_samples(3 * 64 - 1);
        assert_eq!(lfo.current_value(), 13);
        lfo.advance_one();
        assert_eq!(lfo.current_value(), 12);
    }

    #[test]
    fn period_is_210_positions_of_64_samples() {
        assert_eq!(YM2413_AM_LFO_TABLE_LENGTH, 210);
        assert_eq!(YM2413_AM_LFO_SAMPLES_PER_STEP, 64);
        assert_eq!(YM2413_AM_LFO_PERIOD_SAMPLES, 13_440);
    }

    #[test]
    fn phase_wraps_at_exact_period() {
        let mut lfo = AmLfo::with_phase(YM2413_AM_LFO_PERIOD_SAMPLES - 1).unwrap();
        lfo.advance_one();
        assert_eq!(lfo.phase(), 0);
        assert_eq!(lfo.current_value(), 0);
    }

    #[test]
    fn continuous_and_chunked_advancement_are_equivalent() {
        let mut continuous = AmLfo::with_phase(1234).unwrap();
        let mut chunked = continuous;
        continuous.advance_samples(777_777);
        chunked.advance_samples(123_456);
        chunked.advance_samples(654_321);
        assert_eq!(continuous, chunked);
    }

    #[test]
    fn snapshot_round_trip_preserves_phase() {
        let lfo = AmLfo::with_phase(9876).unwrap();
        assert_eq!(AmLfo::restore_state(lfo.export_state()), Ok(lfo));
    }

    #[test]
    fn invalid_initial_phase_and_snapshot_are_rejected() {
        assert_eq!(
            AmLfo::with_phase(YM2413_AM_LFO_PERIOD_SAMPLES),
            Err(AmLfoError::InvalidPhase(YM2413_AM_LFO_PERIOD_SAMPLES))
        );
        assert_eq!(
            AmLfo::restore_state(AmLfoState {
                format_version: YM2413_AM_LFO_STATE_VERSION,
                phase: YM2413_AM_LFO_PERIOD_SAMPLES,
            }),
            Err(AmLfoError::InvalidState)
        );
        assert_eq!(
            AmLfo::restore_state(AmLfoState {
                format_version: 99,
                phase: 0,
            }),
            Err(AmLfoError::InvalidState)
        );
    }
}


/// Version of the exact serial AM shift-register state.
pub const YM2413_SERIAL_AM_LFO_STATE_VERSION: u16 = 1;

/// Errors returned by the serial AM LFO state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413SerialAmLfoError {
    /// The serial cycle must be in the range 0..18.
    InvalidCycle(u8),
    /// The saved AM shift-register state is invalid.
    InvalidState,
}

/// Complete snapshot of Nuked-compatible serial AM LFO state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413SerialAmLfoState {
    pub format_version: u16,
    /// Nine-bit serial AM shift register.
    pub counter: u16,
    /// Direction latch used by the AM incrementer.
    pub direction: bool,
    /// Carry latch between adjacent AM shift operations.
    pub carry: bool,
    /// AM step event generated at serial cycle 17.
    pub step: bool,
    /// Four-bit AM output captured at cycle 0 before the shift stage.
    pub output: u8,
}

impl Default for Ym2413SerialAmLfoState {
    fn default() -> Self {
        Self {
            format_version: YM2413_SERIAL_AM_LFO_STATE_VERSION,
            counter: 0,
            direction: false,
            carry: false,
            step: false,
            output: 0,
        }
    }
}

/// YM2413 AM LFO modeled as the serial shift/carry network used by OPLL_DoLFO.
///
/// Unlike the measured sample-domain AM candidate above, this state advances
/// once per OPLL serial cycle and latches its four-bit output at cycle 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ym2413SerialAmLfo {
    state: Ym2413SerialAmLfoState,
}

impl Ym2413SerialAmLfo {
    /// Construct the deterministic zeroed shift-register candidate.
    pub const fn new() -> Self {
        Self {
            state: Ym2413SerialAmLfoState {
                format_version: YM2413_SERIAL_AM_LFO_STATE_VERSION,
                counter: 0,
                direction: false,
                carry: false,
                step: false,
                output: 0,
            },
        }
    }

    /// Return the current four-bit AM output latch.
    pub const fn output(&self) -> u8 {
        self.state.output
    }

    /// Return the current serial AM state.
    pub const fn state(&self) -> &Ym2413SerialAmLfoState {
        &self.state
    }

    /// Capture the shift-register output at the beginning of serial cycle 0.
    ///
    /// Nuked performs this before OPLL_DoLFO shifts the AM register for the
    /// current call. Other cycles retain the value captured for this rotation.
    pub fn capture_output(&mut self, cycle: u8) -> Result<(), Ym2413SerialAmLfoError> {
        if cycle >= 18 {
            return Err(Ym2413SerialAmLfoError::InvalidCycle(cycle));
        }
        if cycle == 0 {
            self.state.output = ((self.state.counter >> 3) & 0x0f) as u8;
        }
        Ok(())
    }

    /// Execute one OPLL_DoLFO AM substage using the pre-increment PM counter.
    ///
    /// The PM counter is supplied by the serial PG service. At cycle 17 this
    /// method derives the next AM step before the caller advances that shared
    /// counter, matching Nuked's order. Test Mode bit 3 forces AM shifts during
    /// the first nine cycles; bit 1 resets the direction and high counter bit
    /// after the current shift, as in the reference implementation.
    pub fn clock(
        &mut self,
        cycle: u8,
        lfo_counter: u16,
        testmode: u8,
    ) -> Result<(), Ym2413SerialAmLfoError> {
        if cycle >= 18 {
            return Err(Ym2413SerialAmLfoError::InvalidCycle(cycle));
        }
        if cycle == 17 {
            self.state.step = ((lfo_counter & 0x003f) + 1) >> 6 != 0;
        }

        let mut am_increment = 0_u8;
        if (self.state.step || testmode & 0x08 != 0) && cycle < 9 {
            am_increment = u8::from(self.state.direction || cycle == 0);
        }

        if cycle >= 9 {
            self.state.carry = false;
        }

        if cycle == 0 {
            if self.state.direction && self.state.counter & 0x007f == 0 {
                self.state.direction = false;
            } else if !self.state.direction && self.state.counter & 0x0069 == 0x0069 {
                self.state.direction = true;
            }
        }

        let am_bit = (self.state.counter & 1) as u8
            + am_increment
            + u8::from(self.state.carry);
        self.state.carry = am_bit >> 1 != 0;
        self.state.counter =
            (u16::from(am_bit & 1) << 8) | (self.state.counter >> 1);

        if testmode & 0x02 != 0 {
            self.state.direction = false;
            self.state.counter &= 0x00ff;
        }
        Ok(())
    }

    /// Export the exact shift, direction, carry, step and output latches.
    pub const fn export_state(&self) -> Ym2413SerialAmLfoState {
        self.state
    }

    /// Restore a serial AM LFO snapshot after validating its register widths.
    pub const fn restore_state(
        state: Ym2413SerialAmLfoState,
    ) -> Result<Self, Ym2413SerialAmLfoError> {
        if state.format_version != YM2413_SERIAL_AM_LFO_STATE_VERSION
            || state.counter > 0x01ff
            || state.output > 0x0f
        {
            return Err(Ym2413SerialAmLfoError::InvalidState);
        }
        Ok(Self { state })
    }
}

#[cfg(test)]
mod serial_tests {
    use super::{
        Ym2413SerialAmLfo, Ym2413SerialAmLfoError, Ym2413SerialAmLfoState,
        YM2413_SERIAL_AM_LFO_STATE_VERSION,
    };

    #[test]
    fn output_is_captured_before_the_cycle_zero_shift() {
        let state = Ym2413SerialAmLfoState {
            counter: 0x01a8,
            ..Ym2413SerialAmLfoState::default()
        };
        let mut lfo = Ym2413SerialAmLfo::restore_state(state).unwrap();
        lfo.capture_output(0).unwrap();
        assert_eq!(lfo.output(), 5);
        lfo.clock(0, 0, 0).unwrap();
        assert_eq!(lfo.output(), 5);
    }

    #[test]
    fn cycle_seventeen_derives_the_next_step_from_the_preincrement_pm_counter() {
        let mut lfo = Ym2413SerialAmLfo::new();
        lfo.clock(17, 63, 0).unwrap();
        assert!(lfo.state().step);
        lfo.clock(0, 64, 0).unwrap();
        assert_eq!(lfo.state().counter, 0x0100);
    }

    #[test]
    fn test_mode_bit_three_forces_the_first_nine_serial_shifts() {
        let mut lfo = Ym2413SerialAmLfo::new();
        for cycle in 0..9 {
            lfo.clock(cycle, 0, 0x08).unwrap();
        }
        assert_ne!(lfo.state().counter, 0);
        assert!(!lfo.state().carry);
    }

    #[test]
    fn test_mode_bit_one_resets_direction_and_high_counter_bit_after_shift() {
        let state = Ym2413SerialAmLfoState {
            counter: 0x01ff,
            direction: true,
            carry: true,
            ..Ym2413SerialAmLfoState::default()
        };
        let mut lfo = Ym2413SerialAmLfo::restore_state(state).unwrap();
        lfo.clock(0, 0, 0x02).unwrap();
        assert!(!lfo.state().direction);
        assert_eq!(lfo.state().counter & 0x0100, 0);
    }

    #[test]
    fn continuous_and_partitioned_serial_advancement_are_equivalent() {
        let mut continuous = Ym2413SerialAmLfo::new();
        let mut partitioned = continuous;
        let mut pm_counter = 0_u16;
        for cycle_index in 0..360_u16 {
            let cycle = (cycle_index % 18) as u8;
            continuous.clock(cycle, pm_counter, 0).unwrap();
            if cycle == 17 {
                pm_counter = (pm_counter + 1) & 0x1fff;
            }
        }
        pm_counter = 0;
        for cycle_index in 0..117_u16 {
            let cycle = (cycle_index % 18) as u8;
            partitioned.clock(cycle, pm_counter, 0).unwrap();
            if cycle == 17 {
                pm_counter = (pm_counter + 1) & 0x1fff;
            }
        }
        let saved = partitioned.export_state();
        partitioned = Ym2413SerialAmLfo::restore_state(saved).unwrap();
        for cycle_index in 117..360_u16 {
            let cycle = (cycle_index % 18) as u8;
            partitioned.clock(cycle, pm_counter, 0).unwrap();
            if cycle == 17 {
                pm_counter = (pm_counter + 1) & 0x1fff;
            }
        }
        assert_eq!(continuous, partitioned);
    }

    #[test]
    fn snapshot_validation_rejects_invalid_counter_and_version() {
        let mut state = Ym2413SerialAmLfoState::default();
        state.counter = 0x0200;
        assert_eq!(
            Ym2413SerialAmLfo::restore_state(state),
            Err(Ym2413SerialAmLfoError::InvalidState)
        );
        state.counter = 0;
        state.format_version = YM2413_SERIAL_AM_LFO_STATE_VERSION + 1;
        assert_eq!(
            Ym2413SerialAmLfo::restore_state(state),
            Err(Ym2413SerialAmLfoError::InvalidState)
        );
    }
}
