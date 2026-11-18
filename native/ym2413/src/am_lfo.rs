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
        AmLfo, AmLfoError, AmLfoState, YM2413_AM_LFO_LEVELS,
        YM2413_AM_LFO_PERIOD_SAMPLES, YM2413_AM_LFO_SAMPLES_PER_STEP,
        YM2413_AM_LFO_STATE_VERSION, YM2413_AM_LFO_TABLE_LENGTH,
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
