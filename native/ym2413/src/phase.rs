//! Candidate YM2413 Phase Generator (PG) model.
//!
//! This module implements the measured 19-bit phase accumulator and PM
//! correction described in the project's M1 research notes. It deliberately
//! does not own the global LFO/sample counter or slot scheduler: callers must
//! provide the PM position at the exact operator update boundary. Reset
//! alignment, register-write sampling and slot pipeline timing remain open.

/// Number of significant bits in the measured YM2413 phase accumulator.
pub const YM2413_PHASE_BITS: u32 = 19;
/// Mask for the 19-bit phase accumulator.
pub const YM2413_PHASE_MASK: u32 = (1 << YM2413_PHASE_BITS) - 1;
/// Number of positions in the measured phase-modulation sequence.
pub const YM2413_PM_POSITIONS: usize = 8;

/// Version of the standalone phase-generator snapshot.
pub const YM2413_PG_STATE_VERSION: u16 = 1;

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

/// One operator's 19-bit phase accumulator.
///
/// This is a digital candidate, not a complete cycle-accurate PG/slot model.
/// The owner supplies current channel parameters and PM position at each
/// operator update. This keeps the unresolved global-LFO alignment outside
/// the PG until it is established by independent vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhaseGenerator {
    phase: u32,
}

impl PhaseGenerator {
    /// Create a phase generator with a zeroed phase accumulator.
    pub const fn new() -> Self {
        Self { phase: 0 }
    }

    /// Return the full 19-bit fixed-point phase accumulator.
    pub const fn phase(&self) -> u32 {
        self.phase
    }

    /// Return the upper ten bits used as the sine-table address.
    pub const fn phase_index(&self) -> u16 {
        (self.phase >> 9) as u16
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

        // Preserve the two integer truncation points used by the primary
        // Nuked-OPLL reference: block scaling is shifted before MUL is applied.
        // Combining the operations algebraically changes some odd PM-adjusted
        // frequencies by 1..7 phase units.
        let block_adjusted_frequency = (corrected_fnum << block) >> 1;
        let step = (block_adjusted_frequency * ml) >> 1;
        u32::try_from(step).map_err(|_| PhaseGeneratorError::InvalidState)
    }

    /// Advance this operator by one output-sample update and return its phase.
    ///
    /// The accumulator wraps modulo 2^19. The caller determines the precise
    /// slot and sample boundary; this method does not advance global time.
    pub fn advance(
        &mut self,
        fnum: u16,
        block: u8,
        mul: u8,
        pm_enabled: bool,
        pm_position: u8,
    ) -> Result<u32, PhaseGeneratorError> {
        let step = Self::phase_step(fnum, block, mul, pm_enabled, pm_position)?;
        self.phase = self.phase.wrapping_add(step) & YM2413_PHASE_MASK;
        Ok(self.phase)
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
        if state.format_version != YM2413_PG_STATE_VERSION || state.phase > YM2413_PHASE_MASK {
            return Err(PhaseGeneratorError::InvalidState);
        }
        Ok(Self { phase: state.phase })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PhaseGenerator, PhaseGeneratorError, PhaseGeneratorState, YM2413_ML_TABLE,
        YM2413_PHASE_MASK,
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
    fn phase_step_matches_nuked_integer_rounding_for_all_legal_inputs() {
        // Nuked-OPLL 1.0.2, pinned at 1269cf5a783b65583b50fa2464d08be75830aaa0:
        // frequency starts at FNUM << 1, PM is applied, BLOCK is shifted and
        // truncated, then the MUL table is applied and truncated again.
        fn nuked_phase_step(fnum: u16, block: u8, mul: u8, pm_position: u8) -> u32 {
            let mut frequency = u32::from(fnum) << 1;
            match pm_position {
                1 | 3 => frequency += frequency >> 8,
                2 => frequency += frequency >> 7,
                5 | 7 => frequency -= frequency >> 8,
                6 => frequency -= frequency >> 7,
                _ => {}
            }

            let block_adjusted_frequency = (frequency << block) >> 1;
            (block_adjusted_frequency * u32::from(YM2413_ML_TABLE[usize::from(mul)])) >> 1
        }

        // This boundary distinguishes staged truncation (128) from the
        // algebraically combined expression (129).
        assert_eq!(PhaseGenerator::phase_step(64, 0, 2, true, 2), Ok(128));

        for fnum in 0..=0x01ff_u16 {
            for block in 0..=7_u8 {
                for mul in 0..=15_u8 {
                    for pm_position in 0..8_u8 {
                        assert_eq!(
                            PhaseGenerator::phase_step(fnum, block, mul, true, pm_position),
                            Ok(nuked_phase_step(fnum, block, mul, pm_position)),
                            "FNUM={fnum:#05x}, BLOCK={block}, MUL={mul}, PM={pm_position}"
                        );
                    }
                }
            }
        }
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
    fn snapshot_round_trip_preserves_phase() {
        let mut pg = PhaseGenerator::new();
        pg.advance(0x1c0, 6, 1, true, 2).unwrap();
        let restored = PhaseGenerator::restore_state(pg.export_state()).unwrap();
        assert_eq!(restored, pg);
    }

    #[test]
    fn restore_rejects_invalid_version_and_phase() {
        assert_eq!(
            PhaseGenerator::restore_state(PhaseGeneratorState {
                format_version: 99,
                phase: 0,
            }),
            Err(PhaseGeneratorError::InvalidState)
        );
        assert_eq!(
            PhaseGenerator::restore_state(PhaseGeneratorState {
                format_version: 1,
                phase: YM2413_PHASE_MASK + 1,
            }),
            Err(PhaseGeneratorError::InvalidState)
        );
    }

    #[test]
    fn zero_fnum_and_zero_base_increment_remain_zero() {
        assert_eq!(PhaseGenerator::phase_step(0, 7, 15, true, 7), Ok(0));
        assert_eq!(PhaseGenerator::phase_step(1, 0, 0, false, 0), Ok(0));
    }
}
