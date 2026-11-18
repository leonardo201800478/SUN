//! YM2413 rhythm noise generator (candidate digital component).
//!
//! The reverse-engineering measurements by andete identify a 23-bit maximal
//! length LFSR with polynomial x^23 + x^9 + x^8 + x + 1. The published
//! Galois recurrence is implemented here directly:
//!
//! 1. Emit state bit 0.
//! 2. Shift the register right by one.
//! 3. If the emitted bit was 1, XOR the shifted state with 0x400181.
//!
//! The physical reset seed and the exact hardware clock-enable cadence remain
//! unknown. Consequently construction and reset require an explicit non-zero
//! seed, and this module never advances itself from YM2413 clocks or samples.

/// Number of state bits in the measured YM2413 rhythm noise LFSR.
pub const YM2413_RHYTHM_NOISE_LFSR_BITS: u32 = 23;

/// Mask for the 23-bit noise register.
pub const YM2413_RHYTHM_NOISE_LFSR_MASK: u32 = 0x7f_ffff;

/// Galois feedback mask for x^23 + x^9 + x^8 + x + 1.
pub const YM2413_RHYTHM_NOISE_LFSR_TAPS: u32 = 0x40_0181;

/// Maximal period of a non-zero 23-bit LFSR with the documented polynomial.
pub const YM2413_RHYTHM_NOISE_LFSR_PERIOD: u32 = YM2413_RHYTHM_NOISE_LFSR_MASK;

/// Version of the serializable rhythm-noise state.
pub const YM2413_RHYTHM_NOISE_STATE_VERSION: u16 = 1;

/// Errors returned by the rhythm-noise generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413RhythmNoiseError {
    /// A seed or restored register is zero or exceeds the 23-bit register.
    InvalidRegister(u32),
    /// The snapshot format is not supported.
    StateVersionMismatch { expected: u16, actual: u16 },
}

impl core::fmt::Display for Ym2413RhythmNoiseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidRegister(value) => {
                write!(f, "invalid YM2413 rhythm-noise LFSR register: {value:#x}")
            }
            Self::StateVersionMismatch { expected, actual } => write!(
                f,
                "YM2413 rhythm-noise state version {actual} is unsupported (expected {expected})"
            ),
        }
    }
}

impl std::error::Error for Ym2413RhythmNoiseError {}

/// Versioned snapshot of the 23-bit rhythm-noise shift register.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmNoiseState {
    pub format_version: u16,
    pub register: u32,
}

/// Explicitly seeded YM2413 rhythm-noise LFSR.
///
/// No default seed is provided because the reset contents of the physical
/// YM2413 shift register have not been established by the available evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmNoiseLfsr {
    register: u32,
}

impl Ym2413RhythmNoiseLfsr {
    /// Construct the generator from an explicit, non-zero 23-bit seed.
    pub const fn with_seed(seed: u32) -> Result<Self, Ym2413RhythmNoiseError> {
        if seed == 0 || seed & !YM2413_RHYTHM_NOISE_LFSR_MASK != 0 {
            return Err(Ym2413RhythmNoiseError::InvalidRegister(seed));
        }
        Ok(Self { register: seed })
    }

    /// Return the current 23-bit shift-register contents.
    pub const fn register(&self) -> u32 {
        self.register
    }

    /// Return the output bit currently present at the least-significant stage.
    pub const fn output_bit(&self) -> u8 {
        (self.register & 1) as u8
    }

    /// Emit the current output bit and advance the LFSR by one explicit step.
    ///
    /// This method represents one LFSR clock, not one YM2413 input clock,
    /// internal slot, or output sample. The owner must supply the physical
    /// clock-enable cadence once that behavior is established.
    pub fn clock(&mut self) -> u8 {
        let output = self.output_bit();
        self.register >>= 1;
        if output != 0 {
            self.register ^= YM2413_RHYTHM_NOISE_LFSR_TAPS;
        }
        output
    }

    /// Advance by an explicit number of LFSR clocks without consuming output.
    pub fn advance(&mut self, clocks: u64) {
        for _ in 0..clocks {
            let _ = self.clock();
        }
    }

    /// Replace the register with a new explicit non-zero 23-bit seed.
    pub fn reset(&mut self, seed: u32) -> Result<(), Ym2413RhythmNoiseError> {
        let next = Self::with_seed(seed)?;
        self.register = next.register;
        Ok(())
    }

    /// Export the complete deterministic LFSR state.
    pub const fn export_state(&self) -> Ym2413RhythmNoiseState {
        Ym2413RhythmNoiseState {
            format_version: YM2413_RHYTHM_NOISE_STATE_VERSION,
            register: self.register,
        }
    }

    /// Restore a snapshot after validating its version and register contents.
    pub const fn restore_state(
        state: Ym2413RhythmNoiseState,
    ) -> Result<Self, Ym2413RhythmNoiseError> {
        if state.format_version != YM2413_RHYTHM_NOISE_STATE_VERSION {
            return Err(Ym2413RhythmNoiseError::StateVersionMismatch {
                expected: YM2413_RHYTHM_NOISE_STATE_VERSION,
                actual: state.format_version,
            });
        }
        Self::with_seed(state.register)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        YM2413_RHYTHM_NOISE_LFSR_MASK, YM2413_RHYTHM_NOISE_LFSR_PERIOD, Ym2413RhythmNoiseError,
        Ym2413RhythmNoiseLfsr, Ym2413RhythmNoiseState,
    };

    #[test]
    fn galois_recurrence_matches_reference_sequence_from_seed_one() {
        let mut lfsr = Ym2413RhythmNoiseLfsr::with_seed(1).unwrap();
        let expected = [
            1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 1, 0,
            1, 0, 1,
        ];

        for bit in expected {
            assert_eq!(lfsr.clock(), bit);
        }
    }

    #[test]
    fn one_clock_emits_lsb_before_advancing_the_register() {
        let mut lfsr = Ym2413RhythmNoiseLfsr::with_seed(1).unwrap();

        assert_eq!(lfsr.output_bit(), 1);
        assert_eq!(lfsr.clock(), 1);
        assert_eq!(lfsr.register(), 0x40_0181);
    }

    #[test]
    fn zero_and_out_of_range_seeds_are_rejected() {
        assert_eq!(
            Ym2413RhythmNoiseLfsr::with_seed(0),
            Err(Ym2413RhythmNoiseError::InvalidRegister(0))
        );
        assert_eq!(
            Ym2413RhythmNoiseLfsr::with_seed(YM2413_RHYTHM_NOISE_LFSR_MASK + 1),
            Err(Ym2413RhythmNoiseError::InvalidRegister(
                YM2413_RHYTHM_NOISE_LFSR_MASK + 1
            ))
        );
    }

    #[test]
    fn continuous_and_chunked_advancement_are_equivalent() {
        let mut continuous = Ym2413RhythmNoiseLfsr::with_seed(0x5a_1234).unwrap();
        let mut chunked = continuous;

        continuous.advance(100_003);
        for clocks in [1, 7, 64, 3_001, 17, 65_536, 31_377] {
            chunked.advance(clocks);
        }

        assert_eq!(continuous.export_state(), chunked.export_state());
    }

    #[test]
    fn snapshot_restore_preserves_the_exact_sequence_position() {
        let mut original = Ym2413RhythmNoiseLfsr::with_seed(0x12_3456).unwrap();
        original.advance(12_345);
        let mut restored = Ym2413RhythmNoiseLfsr::restore_state(original.export_state()).unwrap();

        for _ in 0..256 {
            assert_eq!(original.clock(), restored.clock());
        }
        assert_eq!(original.export_state(), restored.export_state());
    }

    #[test]
    fn invalid_snapshots_are_rejected_without_normalizing_the_register() {
        assert_eq!(
            Ym2413RhythmNoiseLfsr::restore_state(Ym2413RhythmNoiseState {
                format_version: 0,
                register: 1,
            }),
            Err(Ym2413RhythmNoiseError::StateVersionMismatch {
                expected: 1,
                actual: 0,
            })
        );
        assert_eq!(
            Ym2413RhythmNoiseLfsr::restore_state(Ym2413RhythmNoiseState {
                format_version: 1,
                register: 0,
            }),
            Err(Ym2413RhythmNoiseError::InvalidRegister(0))
        );
    }

    #[test]
    fn documented_polynomial_has_the_full_non_zero_23_bit_period() {
        let seed = 1;
        let mut lfsr = Ym2413RhythmNoiseLfsr::with_seed(seed).unwrap();
        let mut after_47 = None;
        let mut after_178_481 = None;

        for step in 1..=YM2413_RHYTHM_NOISE_LFSR_PERIOD {
            lfsr.advance(1);
            if step == 47 {
                after_47 = Some(lfsr.register());
            }
            if step == 178_481 {
                after_178_481 = Some(lfsr.register());
            }
        }

        // 2^23 - 1 = 47 * 178481. These checks exclude both proper
        // prime-factor reductions of the candidate period.
        assert_ne!(after_47, Some(seed));
        assert_ne!(after_178_481, Some(seed));
        assert_eq!(lfsr.register(), seed);
    }
}
