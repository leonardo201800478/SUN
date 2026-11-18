//! Candidate global phase-modulation LFO counter for the YM2413.
//!
//! The counter is advanced explicitly by completed operator rotations.
//! One sample is one complete 18-slot rotation (72 input clocks). The
//! hardware's initial counter alignment is not established, so construction
//! requires an explicit initial counter value.

/// Number of bits in the measured PM LFO counter.
pub const YM2413_PM_LFO_COUNTER_BITS: u32 = 13;
/// Mask for the 13-bit PM LFO counter.
pub const YM2413_PM_LFO_COUNTER_MASK: u16 = (1 << YM2413_PM_LFO_COUNTER_BITS) - 1;
/// Number of completed YM2413 samples in one PM LFO period.
pub const YM2413_PM_LFO_PERIOD_SAMPLES: u64 = 8_192;
/// Number of samples for which each PM table position is held.
pub const YM2413_PM_LFO_SAMPLES_PER_POSITION: u64 = 1_024;
/// Number of positions in the PM table.
pub const YM2413_PM_LFO_POSITIONS: u8 = 8;

/// Version of the standalone PM LFO counter snapshot.
pub const YM2413_PM_LFO_STATE_VERSION: u16 = 1;

/// Errors returned by the candidate PM LFO counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseModulationLfoError {
    /// The initial or restored counter exceeds the 13-bit hardware width.
    InvalidCounter(u16),
    /// Snapshot version or counter state is invalid.
    InvalidState,
}

/// Snapshot of the global PM LFO counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseModulationLfoState {
    pub format_version: u16,
    pub counter: u16,
}

/// Candidate 13-bit PM LFO counter.
///
/// The caller owns clock scheduling and must call `advance_samples` once for
/// each completed 18-slot rotation. The initial phase is explicit because
/// available measurements establish the cadence and table position decode,
/// but do not establish the counter's alignment to chip reset or first output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseModulationLfo {
    counter: u16,
}

impl PhaseModulationLfo {
    /// Construct the PM LFO with an explicit 13-bit initial counter value.
    pub fn new(initial_counter: u16) -> Result<Self, PhaseModulationLfoError> {
        if initial_counter > YM2413_PM_LFO_COUNTER_MASK {
            return Err(PhaseModulationLfoError::InvalidCounter(initial_counter));
        }
        Ok(Self {
            counter: initial_counter,
        })
    }

    /// Return the complete 13-bit PM LFO counter.
    pub const fn counter(&self) -> u16 {
        self.counter
    }

    /// Return the current PM table position selected by counter bits 12..10.
    pub const fn position(&self) -> u8 {
        (self.counter >> 10) as u8
    }

    /// Advance by completed YM2413 output samples and return the new PM position.
    ///
    /// The counter wraps modulo 8192. This method does not advance chip clocks;
    /// the owner must call it at the correct completed-rotation boundary.
    pub fn advance_samples(&mut self, samples: u64) -> u8 {
        let increment = samples % YM2413_PM_LFO_PERIOD_SAMPLES;
        self.counter =
            ((u64::from(self.counter) + increment) & u64::from(YM2413_PM_LFO_COUNTER_MASK)) as u16;
        self.position()
    }

    /// Export the complete counter state.
    pub const fn export_state(&self) -> PhaseModulationLfoState {
        PhaseModulationLfoState {
            format_version: YM2413_PM_LFO_STATE_VERSION,
            counter: self.counter,
        }
    }

    /// Restore a counter snapshot after validating its version and width.
    pub const fn restore_state(
        state: PhaseModulationLfoState,
    ) -> Result<Self, PhaseModulationLfoError> {
        if state.format_version != YM2413_PM_LFO_STATE_VERSION
            || state.counter > YM2413_PM_LFO_COUNTER_MASK
        {
            return Err(PhaseModulationLfoError::InvalidState);
        }
        Ok(Self {
            counter: state.counter,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PhaseModulationLfo, PhaseModulationLfoError, PhaseModulationLfoState,
        YM2413_PM_LFO_COUNTER_MASK,
    };

    #[test]
    fn constructor_requires_a_valid_explicit_initial_phase() {
        assert_eq!(PhaseModulationLfo::new(0).unwrap().position(), 0);
        assert_eq!(PhaseModulationLfo::new(7_168).unwrap().position(), 7);
        assert_eq!(
            PhaseModulationLfo::new(YM2413_PM_LFO_COUNTER_MASK + 1),
            Err(PhaseModulationLfoError::InvalidCounter(8_192))
        );
    }

    #[test]
    fn pm_position_changes_at_each_1024_sample_boundary() {
        let mut lfo = PhaseModulationLfo::new(0).unwrap();
        assert_eq!(lfo.advance_samples(1_023), 0);
        assert_eq!(lfo.counter(), 1_023);
        assert_eq!(lfo.advance_samples(1), 1);
        assert_eq!(lfo.counter(), 1_024);
        assert_eq!(lfo.advance_samples(1_024), 2);
    }

    #[test]
    fn pm_position_sequence_repeats_after_8192_samples() {
        let mut lfo = PhaseModulationLfo::new(0).unwrap();
        let positions: Vec<u8> = (0..8)
            .map(|_| {
                let position = lfo.position();
                lfo.advance_samples(1_024);
                position
            })
            .collect();
        assert_eq!(positions, [0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(lfo.counter(), 0);
        assert_eq!(lfo.advance_samples(8_192), 0);
        assert_eq!(lfo.counter(), 0);
    }

    #[test]
    fn explicit_initial_phase_is_preserved_and_wraps() {
        let mut lfo = PhaseModulationLfo::new(7_000).unwrap();
        assert_eq!(lfo.position(), 6);
        assert_eq!(lfo.advance_samples(1_192), 0);
        assert_eq!(lfo.counter(), 0);
    }

    #[test]
    fn advance_is_invariant_under_sample_block_partitioning() {
        let mut continuous = PhaseModulationLfo::new(6_731).unwrap();
        let mut chunked = continuous;
        continuous.advance_samples(20_003);
        for samples in [1, 3, 4, 17, 72, 511, 1_024, 3_371, 15_000] {
            chunked.advance_samples(samples);
        }
        assert_eq!(continuous, chunked);
    }

    #[test]
    fn snapshot_round_trip_preserves_counter_and_phase() {
        let mut lfo = PhaseModulationLfo::new(3_333).unwrap();
        lfo.advance_samples(4_567);
        let restored = PhaseModulationLfo::restore_state(lfo.export_state()).unwrap();
        assert_eq!(restored, lfo);
    }

    #[test]
    fn restore_rejects_invalid_version_and_counter() {
        assert_eq!(
            PhaseModulationLfo::restore_state(PhaseModulationLfoState {
                format_version: 99,
                counter: 0,
            }),
            Err(PhaseModulationLfoError::InvalidState)
        );
        assert_eq!(
            PhaseModulationLfo::restore_state(PhaseModulationLfoState {
                format_version: 1,
                counter: 8_192,
            }),
            Err(PhaseModulationLfoError::InvalidState)
        );
    }
}
