//! Candidate YM2413 Phase Generator (PG) model.
//!
//! This module implements the measured 19-bit phase coordinate and PM
//! correction described in the project's M1 research notes. The serial PG
//! additionally preserves Nuked's full-width `uint32_t` phase-ring values. It deliberately
//! does not own the global LFO/sample counter or slot scheduler: callers must
//! provide the PM position at the exact operator update boundary. Reset
//! alignment, register-write sampling and slot pipeline timing remain open.

/// Number of phase-coordinate bits consumed by the measured YM2413 sine path.
pub const YM2413_PHASE_BITS: u32 = 19;
/// Mask for the 19-bit phase coordinate used by the standalone candidate.
pub const YM2413_PHASE_MASK: u32 = (1 << YM2413_PHASE_BITS) - 1;
/// Number of positions in the measured phase-modulation sequence.
pub const YM2413_PM_POSITIONS: usize = 8;

/// Version of the standalone phase-generator snapshot.
pub const YM2413_PG_STATE_VERSION: u16 = 2;

/// Errors returned by the candidate Phase Generator model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseGeneratorError {
    /// FNUM is wider than the YM2413's 9-bit channel frequency field.
    InvalidFnum(u16),
    /// BLOCK is wider than the YM2413's 3-bit octave field.
    InvalidBlock(u8),
    /// MUL is wider than the YM2413's 4-bit multiplier field.
    InvalidMultiplier(u8),
    /// PM position must be in the range 0..8.
    InvalidPmPosition(u8),
    /// Snapshot version or phase value is invalid.
    InvalidState,
}

/// Measured ML multiplier mapping indexed by the four-bit MUL field.
pub const YM2413_ML_TABLE: [u8; 16] = [1, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 20, 24, 24, 30, 30];

/// Measured 8×8 phase-modulation correction table.
///
/// Rows are selected by FNUM[8:6], columns by the global PM LFO position.
pub const YM2413_PM_TABLE: [[i8; YM2413_PM_POSITIONS]; 8] = [
    [0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0, 0, -1, 0],
    [0, 1, 2, 1, 0, -1, -2, -1],
    [0, 1, 3, 1, 0, -1, -3, -1],
    [0, 2, 4, 2, 0, -2, -4, -2],
    [0, 2, 5, 2, 0, -2, -5, -2],
    [0, 3, 6, 3, 0, -3, -6, -3],
    [0, 3, 7, 3, 0, -3, -7, -3],
];

/// Snapshot of one operator's candidate phase-accumulator state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseGeneratorState {
    pub format_version: u16,
    pub phase: u32,
}

/// One operator's phase state, including full-width serial PG intermediates.
///
/// The standalone convenience update remains a 19-bit candidate. The serial
/// hardware path uses `write_serial_phase` and `advance_serial_by_increment`
/// to preserve Nuked's full-width `uint32_t` ring state. The owner supplies
/// channel parameters and PM position at the established operator boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhaseGenerator {
    phase: u32,
}

impl PhaseGenerator {
    /// Create a phase generator with a zeroed phase accumulator.
    pub const fn new() -> Self {
        Self { phase: 0 }
    }

    /// Return the raw phase accumulator retained by the serial PG ring.
    pub const fn phase(&self) -> u32 {
        self.phase
    }

    /// Return the ten phase bits consumed by the YM2413 sine-table address.
    pub const fn phase_index(&self) -> u16 {
        ((self.phase >> 9) & 0x03ff) as u16
    }

    /// Reset the phase accumulator to zero.
    pub fn reset(&mut self) {
        self.phase = 0;
    }

    /// Calculate the candidate phase increment for one operator update.
    ///
    /// FNUM is 9-bit, BLOCK is 3-bit, MUL is 4-bit. If PM is disabled, the
    /// supplied position is ignored. The integer arithmetic follows the
    /// measured M1 expression and intentionally performs no floating-point
    /// operations.
    pub fn phase_step(
        fnum: u16,
        block: u8,
        mul: u8,
        pm_enabled: bool,
        pm_position: u8,
    ) -> Result<u32, PhaseGeneratorError> {
        if fnum > 0x01ff {
            return Err(PhaseGeneratorError::InvalidFnum(fnum));
        }
        if block > 7 {
            return Err(PhaseGeneratorError::InvalidBlock(block));
        }
        if mul > 0x0f {
            return Err(PhaseGeneratorError::InvalidMultiplier(mul));
        }
        if pm_enabled && usize::from(pm_position) >= YM2413_PM_POSITIONS {
            return Err(PhaseGeneratorError::InvalidPmPosition(pm_position));
        }

        let ml = u64::from(YM2413_ML_TABLE[usize::from(mul)]);
        let pm = if pm_enabled {
            i64::from(YM2413_PM_TABLE[usize::from(fnum >> 6)][usize::from(pm_position)])
        } else {
            0
        };
        let corrected_fnum = u64::try_from(i64::from(fnum) * 2 + pm)
            .map_err(|_| PhaseGeneratorError::InvalidState)?;
        // Preserve Nuked-OPLL's two integer truncation points: block scaling
        // precedes multiplier application. Combining these shifts is not
        // equivalent for odd PM-adjusted frequencies.
        let block_adjusted_frequency = (corrected_fnum << block) >> 1;
        let step = (block_adjusted_frequency * ml) >> 1;
        u32::try_from(step).map_err(|_| PhaseGeneratorError::InvalidState)
    }

    /// Replace the accumulator with a serial PG phase-array write.
    ///
    /// Nuked's `pg_phase[(cycle + 17) % 18]` stage assigns
    /// `pg_phase_next + pg_inc`; it does not add that value to the previous
    /// contents of the selected phase-array entry.
    pub fn write_phase(&mut self, phase: u32) {
        self.phase = phase & YM2413_PHASE_MASK;
    }

    /// Replace the serial PG accumulator with Nuked's full-width phase value.
    ///
    /// Nuked stores `pg_phase`, `pg_phase_next`, and their sum as `uint32_t`.
    /// The phase bits above bit 18 are retained internally even though the
    /// sine-table address consumes only bits 9..18.
    pub fn write_serial_phase(&mut self, phase: u32) {
        self.phase = phase;
    }

    /// Add one latched PG increment using the serial hardware's u32 arithmetic.
    pub fn advance_serial_by_increment(&mut self, increment: u32) -> u32 {
        self.phase = self.phase.wrapping_add(increment);
        self.phase
    }

    /// Add one already-latched hardware PG increment to this accumulator.
    ///
    /// The serial chip pipeline calculates `pg_inc` in a different stage
    /// from the delayed phase-array write. Keeping the adder separate avoids
    /// recalculating FNUM/BLOCK/MUL at the accumulator write boundary.
    pub fn advance_by_increment(&mut self, increment: u32) -> u32 {
        self.phase = self.phase.wrapping_add(increment) & YM2413_PHASE_MASK;
        self.phase
    }

    /// Advance this operator by one output-sample update and return its phase.
    ///
    /// This convenience path combines increment calculation and accumulation.
    /// The serial chip pipeline should use `advance_by_increment` with its
    /// retained PG increment latch instead.
    pub fn advance(
        &mut self,
        fnum: u16,
        block: u8,
        mul: u8,
        pm_enabled: bool,
        pm_position: u8,
    ) -> Result<u32, PhaseGeneratorError> {
        let step = Self::phase_step(fnum, block, mul, pm_enabled, pm_position)?;
        Ok(self.advance_by_increment(step))
    }

    /// Export the complete state of this phase accumulator.
    pub const fn export_state(&self) -> PhaseGeneratorState {
        PhaseGeneratorState {
            format_version: YM2413_PG_STATE_VERSION,
            phase: self.phase,
        }
    }

    /// Restore a phase accumulator after validating its snapshot.
    pub const fn restore_state(state: PhaseGeneratorState) -> Result<Self, PhaseGeneratorError> {
        if state.format_version != YM2413_PG_STATE_VERSION {
            return Err(PhaseGeneratorError::InvalidState);
        }
        Ok(Self { phase: state.phase })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PhaseGenerator, PhaseGeneratorError, PhaseGeneratorState, YM2413_ML_TABLE,
        YM2413_PG_STATE_VERSION, YM2413_PHASE_MASK,
    };

    #[test]
    fn ml_table_matches_the_documented_mul_mapping() {
        assert_eq!(
            YM2413_ML_TABLE,
            [1, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 20, 24, 24, 30, 30]
        );
    }

    #[test]
    fn base_phase_step_matches_measured_formula() {
        // FNUM=0x1c0, BLOCK=6, MUL=1 => ML=2.
        assert_eq!(
            PhaseGenerator::phase_step(0x1c0, 6, 1, false, 0),
            Ok(28_672)
        );
    }

    #[test]
    fn pm_sequence_matches_published_ym2413_measurement_vector() {
        // Published andete example: FNUM=0x1c0, BLOCK=6, ML=2.
        let actual: Vec<u32> = (0..8)
            .map(|position| PhaseGenerator::phase_step(0x1c0, 6, 1, true, position).unwrap())
            .collect();
        assert_eq!(
            actual,
            [
                28_672, 28_768, 28_896, 28_768, 28_672, 28_576, 28_448, 28_576
            ]
        );
    }

    #[test]
    fn pm_disabled_ignores_lfo_position() {
        assert_eq!(
            PhaseGenerator::phase_step(0x1c0, 6, 1, false, u8::MAX),
            Ok(28_672)
        );
    }

    #[test]
    fn invalid_channel_parameters_are_rejected() {
        assert_eq!(
            PhaseGenerator::phase_step(0x0200, 0, 0, false, 0),
            Err(PhaseGeneratorError::InvalidFnum(0x0200))
        );
        assert_eq!(
            PhaseGenerator::phase_step(0, 8, 0, false, 0),
            Err(PhaseGeneratorError::InvalidBlock(8))
        );
        assert_eq!(
            PhaseGenerator::phase_step(0, 0, 16, false, 0),
            Err(PhaseGeneratorError::InvalidMultiplier(16))
        );
        assert_eq!(
            PhaseGenerator::phase_step(0, 0, 0, true, 8),
            Err(PhaseGeneratorError::InvalidPmPosition(8))
        );
    }

    #[test]
    fn phase_accumulator_wraps_at_nineteen_bits() {
        let mut pg = PhaseGenerator {
            phase: YM2413_PHASE_MASK,
        };
        let phase = pg.advance(0x1c0, 6, 1, false, 0).unwrap();
        assert_eq!(phase, 28_671);
        assert_eq!(pg.phase_index(), (phase >> 9) as u16);
    }

    #[test]
    fn serial_phase_write_preserves_bits_above_the_nineteen_bit_sine_phase() {
        let mut pg = PhaseGenerator::new();
        pg.write_serial_phase(0x0008_4000);
        assert_eq!(pg.phase(), 0x0008_4000);
        assert_eq!(pg.phase_index(), 0x20);
        assert_eq!(pg.advance_serial_by_increment(0x0001_0000), 0x0009_4000);
    }

    #[test]
    fn snapshot_round_trip_preserves_phase() {
        let mut pg = PhaseGenerator::new();
        pg.advance(0x1c0, 6, 1, true, 2).unwrap();
        let restored = PhaseGenerator::restore_state(pg.export_state()).unwrap();
        assert_eq!(restored, pg);
    }

    #[test]
    fn restore_rejects_invalid_version_and_preserves_full_width_serial_phase() {
        assert_eq!(
            PhaseGenerator::restore_state(PhaseGeneratorState {
                format_version: 99,
                phase: 0,
            }),
            Err(PhaseGeneratorError::InvalidState)
        );
        let restored = PhaseGenerator::restore_state(PhaseGeneratorState {
            format_version: YM2413_PG_STATE_VERSION,
            phase: 0x0008_4000,
        })
        .unwrap();
        assert_eq!(restored.phase(), 0x0008_4000);
        assert_eq!(restored.phase_index(), 0x20);
    }

    #[test]
    fn zero_fnum_and_zero_base_increment_remain_zero() {
        assert_eq!(PhaseGenerator::phase_step(0, 7, 15, true, 7), Ok(0));
        assert_eq!(PhaseGenerator::phase_step(1, 0, 0, false, 0), Ok(0));
    }
    #[test]
    fn phase_step_preserves_nuked_rounding_at_positive_and_negative_pm_boundaries() {
        let positive_pm = [
            32, 64, 128, 192, 256, 320, 384, 448, 512, 576, 640, 640, 768, 768, 960, 960,
        ];
        let negative_pm = [
            31, 63, 126, 189, 252, 315, 378, 441, 504, 567, 630, 630, 756, 756, 945, 945,
        ];

        for mul in 0..=15_u8 {
            assert_eq!(
                PhaseGenerator::phase_step(64, 0, mul, true, 2),
                Ok(positive_pm[usize::from(mul)]),
                "positive PM, MUL={mul}"
            );
            assert_eq!(
                PhaseGenerator::phase_step(64, 0, mul, true, 6),
                Ok(negative_pm[usize::from(mul)]),
                "negative PM, MUL={mul}"
            );
        }

        assert_eq!(PhaseGenerator::phase_step(64, 1, 2, true, 2), Ok(258));
    }
}
