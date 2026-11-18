//! YM2413 rhythm noise generator (candidate digital component).
//!
//! The 2018 capture analysis identified the decimated 23-bit sequence
//! x^23 + x^9 + x^8 + x + 1. Its 2020 update explains that the physical rhythm
//! generator advances every operator slot using x^23 + x^9 + 1: 18 fast steps
//! occur during one 18-slot/72-input-clock rotation. The older recurrence is
//! retained below as a decimated-sequence reference; the slot-rate model is
//! represented by Ym2413RhythmSlotNoiseLfsr. Neither model assumes a reset seed
//! or advances itself from the MachineScheduler.

/// Number of state bits in the measured decimated rhythm-noise sequence.
pub const YM2413_RHYTHM_NOISE_LFSR_BITS: u32 = 23;

/// Mask for the 23-bit decimated-sequence reference register.
pub const YM2413_RHYTHM_NOISE_LFSR_MASK: u32 = 0x7f_ffff;

/// Galois feedback mask for the sequence observed when the slot-rate LFSR is decimated by 18.
pub const YM2413_RHYTHM_NOISE_LFSR_TAPS: u32 = 0x40_0181;

/// Maximal period of the non-zero 23-bit decimated-sequence reference.
pub const YM2413_RHYTHM_NOISE_LFSR_PERIOD: u32 = YM2413_RHYTHM_NOISE_LFSR_MASK;

/// Version of the serializable rhythm-noise state.
pub const YM2413_RHYTHM_NOISE_STATE_VERSION: u16 = 1;

/// Register bit controlling the YM2413's six-channel melody/five-voice rhythm mode.
pub const YM2413_RHYTHM_MODE_BIT: u8 = 0x20;

/// Register bits that trigger bass drum, snare, tom-tom, top cymbal and hi-hat.
pub const YM2413_RHYTHM_TRIGGER_MASK: u8 = 0x1f;

/// Decoded trigger and attenuation fields for one rhythm voice.
///
/// `key_on` is the raw trigger bit from register $0E. It becomes active only
/// while Rhythm Mode is enabled. `volume` is the raw four-bit attenuation
/// field from registers $36-$38; its physical dB transfer curve is not modeled
/// by this register decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmVoiceConfig {
    /// Raw trigger bit from register $0E.
    pub key_on: bool,
    /// Raw four-bit volume/attenuation field from registers $36-$38.
    pub volume: u8,
}

impl Ym2413RhythmVoiceConfig {
    /// Return the register-level trigger gate: RHY enabled and this $0E bit set.
    ///
    /// This is not a complete hardware-activity predicate when channels 7-9
    /// have KEY-ON set. The manual says those bits must be cleared; andete's
    /// notes report a possible OR interaction that remains unresolved.
    pub const fn is_active(self, rhythm_mode_enabled: bool) -> bool {
        rhythm_mode_enabled && self.key_on
    }
}

/// Frequency-related register fields for one of channels 7, 8 or 9.
///
/// Channel numbering in this type is zero-based, matching the crate's channel
/// APIs. This deliberately excludes the $30-$38 instrument/volume field,
/// because those locations become rhythm attenuation registers in Rhythm Mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmChannelFrequency {
    /// Zero-based channel index (6..=8).
    pub channel: u8,
    /// Nine-bit frequency number.
    pub fnum: u16,
    /// Three-bit octave/block field.
    pub block: u8,
    /// Raw melodic KEY-ON bit from the channel register.
    pub melodic_key_on: bool,
    /// Raw SUS bit from the channel register.
    pub sustain_on: bool,
}

/// Static register-derived state for the five YM2413 percussion voices.
///
/// This is a decoder only: it does not synthesize percussion phases, envelopes,
/// PCM, or decide when the rhythm LFSR advances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmConfig {
    /// Global RHY bit from register $0E.
    pub rhythm_mode_enabled: bool,
    /// Bass Drum trigger and volume.
    pub bass_drum: Ym2413RhythmVoiceConfig,
    /// Snare Drum trigger and volume.
    pub snare_drum: Ym2413RhythmVoiceConfig,
    /// Tom-Tom trigger and volume.
    pub tom_tom: Ym2413RhythmVoiceConfig,
    /// Top Cymbal trigger and volume.
    pub top_cymbal: Ym2413RhythmVoiceConfig,
    /// Hi-Hat trigger and volume.
    pub hi_hat: Ym2413RhythmVoiceConfig,
    /// Channel indices 6, 7 and 8 (hardware channels 7, 8 and 9).
    pub channel_frequencies: [Ym2413RhythmChannelFrequency; 3],
}

const fn rhythm_channel_frequency(
    registers: &[u8; 64],
    channel: u8,
) -> Ym2413RhythmChannelFrequency {
    let index = channel as usize;
    let fnum_low = registers[0x10 + index];
    let fnum_high = registers[0x20 + index];
    Ym2413RhythmChannelFrequency {
        channel,
        fnum: fnum_low as u16 | (((fnum_high & 0x01) as u16) << 8),
        block: (fnum_high >> 1) & 0x07,
        melodic_key_on: fnum_high & 0x10 != 0,
        sustain_on: fnum_high & 0x20 != 0,
    }
}

/// Decode Rhythm Mode control, per-voice trigger/volume fields and channels 7-9.
///
/// Register mapping follows the Yamaha YM2413 Application Manual:
/// - $0E: RHY (D5), BD (D4), SD (D3), TOM (D2), TOP-CY (D1), HH (D0).
/// - $36: BD volume in the low nibble.
/// - $37: HH volume in the high nibble, SD volume in the low nibble.
/// - $38: TOM volume in the high nibble, TOP-CY volume in the low nibble.
///
/// The frequency inputs for channels 7-9 are retained separately. The manual
/// requires the melodic KEY-ON bits $26-$28 to be cleared when using Rhythm
/// Mode; this decoder reports those raw bits rather than silently rewriting
/// caller-owned register state.
pub const fn decode_rhythm_registers(registers: &[u8; 64]) -> Ym2413RhythmConfig {
    let control = registers[0x0e];
    let triggers = control & YM2413_RHYTHM_TRIGGER_MASK;
    let rhythm_vol_36 = registers[0x36];
    let rhythm_vol_37 = registers[0x37];
    let rhythm_vol_38 = registers[0x38];

    Ym2413RhythmConfig {
        rhythm_mode_enabled: control & YM2413_RHYTHM_MODE_BIT != 0,
        bass_drum: Ym2413RhythmVoiceConfig {
            key_on: triggers & 0x10 != 0,
            volume: rhythm_vol_36 & 0x0f,
        },
        snare_drum: Ym2413RhythmVoiceConfig {
            key_on: triggers & 0x08 != 0,
            volume: rhythm_vol_37 & 0x0f,
        },
        tom_tom: Ym2413RhythmVoiceConfig {
            key_on: triggers & 0x04 != 0,
            volume: rhythm_vol_38 >> 4,
        },
        top_cymbal: Ym2413RhythmVoiceConfig {
            key_on: triggers & 0x02 != 0,
            volume: rhythm_vol_38 & 0x0f,
        },
        hi_hat: Ym2413RhythmVoiceConfig {
            key_on: triggers & 0x01 != 0,
            volume: rhythm_vol_37 >> 4,
        },
        channel_frequencies: [
            rhythm_channel_frequency(registers, 6),
            rhythm_channel_frequency(registers, 7),
            rhythm_channel_frequency(registers, 8),
        ],
    }
}

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

/// Explicitly seeded reference LFSR for the decimated 72-clock noise sequence.
///
/// This is not the physical slot-rate recurrence. No default seed is provided
/// because the reset contents of the physical YM2413 shift register are unknown.
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

/// Version of the serializable slot-rate rhythm-noise state.
pub const YM2413_RHYTHM_SLOT_NOISE_STATE_VERSION: u16 = 1;

/// Snapshot of the fast 23-bit rhythm LFSR that advances once per operator slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmSlotNoiseState {
    pub format_version: u16,
    pub register: u32,
}

/// Slot-rate rhythm LFSR candidate using x^23 + x^9 + 1.
///
/// The reverse-engineering update from 2020 describes one update after every
/// operator slot (one update per four YM2413 input clocks). HH reads the bit at
/// slot 13; SD reads the bit three LFSR steps later at slot 16. The caller owns
/// those sample/advance boundaries. The physical reset seed remains unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmSlotNoiseLfsr {
    register: u32,
}

impl Ym2413RhythmSlotNoiseLfsr {
    /// Construct from an explicit non-zero 23-bit seed.
    pub const fn with_seed(seed: u32) -> Result<Self, Ym2413RhythmNoiseError> {
        if seed == 0 || seed & !YM2413_RHYTHM_NOISE_LFSR_MASK != 0 {
            return Err(Ym2413RhythmNoiseError::InvalidRegister(seed));
        }
        Ok(Self { register: seed })
    }

    /// Return the current 23-bit register.
    pub const fn register(&self) -> u32 {
        self.register
    }

    /// Return the current output bit without advancing the generator.
    pub const fn output_bit(&self) -> u8 {
        (self.register & 1) as u8
    }

    /// Emit the current LSB and advance one slot-rate LFSR step.
    ///
    /// The recurrence is: feedback = bit0 XOR bit14; shift right and insert
    /// feedback at bit22. This method is one explicit LFSR step, not an input
    /// clock; the owner must call it at the researched four-clock slot boundary.
    pub fn clock(&mut self) -> u8 {
        let output = self.output_bit();
        let feedback = (self.register ^ (self.register >> 14)) & 1;
        self.register = (self.register >> 1) | (feedback << 22);
        output
    }

    /// Advance an explicit number of operator-slot LFSR steps.
    pub fn advance(&mut self, steps: u64) {
        for _ in 0..steps {
            let _ = self.clock();
        }
    }

    /// Export the complete slot-rate LFSR state.
    pub const fn export_state(&self) -> Ym2413RhythmSlotNoiseState {
        Ym2413RhythmSlotNoiseState {
            format_version: YM2413_RHYTHM_SLOT_NOISE_STATE_VERSION,
            register: self.register,
        }
    }

    /// Restore the slot-rate LFSR after validating version and register contents.
    pub const fn restore_state(
        state: Ym2413RhythmSlotNoiseState,
    ) -> Result<Self, Ym2413RhythmNoiseError> {
        if state.format_version != YM2413_RHYTHM_SLOT_NOISE_STATE_VERSION {
            return Err(Ym2413RhythmNoiseError::StateVersionMismatch {
                expected: YM2413_RHYTHM_SLOT_NOISE_STATE_VERSION,
                actual: state.format_version,
            });
        }
        Self::with_seed(state.register)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        YM2413_RHYTHM_NOISE_LFSR_MASK, YM2413_RHYTHM_NOISE_LFSR_PERIOD,
        YM2413_RHYTHM_SLOT_NOISE_STATE_VERSION, Ym2413RhythmNoiseError, Ym2413RhythmNoiseLfsr,
        Ym2413RhythmNoiseState, Ym2413RhythmSlotNoiseLfsr, Ym2413RhythmSlotNoiseState,
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

    #[test]
    fn rhythm_register_decoder_maps_all_trigger_bits_and_volume_nibbles() {
        let mut registers = [0; 64];
        registers[0x0e] = 0x3f;
        registers[0x36] = 0xa5;
        registers[0x37] = 0xc4;
        registers[0x38] = 0x7e;

        let config = super::decode_rhythm_registers(&registers);

        assert!(config.rhythm_mode_enabled);
        assert_eq!(
            config.bass_drum,
            super::Ym2413RhythmVoiceConfig {
                key_on: true,
                volume: 5,
            }
        );
        assert_eq!(
            config.snare_drum,
            super::Ym2413RhythmVoiceConfig {
                key_on: true,
                volume: 4,
            }
        );
        assert_eq!(
            config.tom_tom,
            super::Ym2413RhythmVoiceConfig {
                key_on: true,
                volume: 7,
            }
        );
        assert_eq!(
            config.top_cymbal,
            super::Ym2413RhythmVoiceConfig {
                key_on: true,
                volume: 14,
            }
        );
        assert_eq!(
            config.hi_hat,
            super::Ym2413RhythmVoiceConfig {
                key_on: true,
                volume: 12,
            }
        );
    }

    #[test]
    fn rhythm_voice_activation_is_gated_by_rhythm_mode_not_raw_trigger_bits() {
        let mut registers = [0; 64];
        registers[0x0e] = 0x1f;

        let config = super::decode_rhythm_registers(&registers);

        assert!(!config.rhythm_mode_enabled);
        assert!(config.bass_drum.key_on);
        assert!(!config.bass_drum.is_active(config.rhythm_mode_enabled));

        registers[0x0e] |= super::YM2413_RHYTHM_MODE_BIT;
        let enabled = super::decode_rhythm_registers(&registers);
        assert!(enabled.bass_drum.is_active(enabled.rhythm_mode_enabled));
    }

    #[test]
    fn rhythm_decoder_preserves_channels_seven_to_nine_frequency_and_key_fields() {
        let mut registers = [0; 64];
        registers[0x16] = 0xa5;
        registers[0x26] = 0x35;
        registers[0x17] = 0x42;
        registers[0x27] = 0x0b;
        registers[0x18] = 0xfe;
        registers[0x28] = 0x1e;

        let config = super::decode_rhythm_registers(&registers);

        assert_eq!(
            config.channel_frequencies[0],
            super::Ym2413RhythmChannelFrequency {
                channel: 6,
                fnum: 0x1a5,
                block: 2,
                melodic_key_on: true,
                sustain_on: true,
            }
        );
        assert_eq!(
            config.channel_frequencies[1],
            super::Ym2413RhythmChannelFrequency {
                channel: 7,
                fnum: 0x142,
                block: 5,
                melodic_key_on: false,
                sustain_on: false,
            }
        );
        assert_eq!(
            config.channel_frequencies[2],
            super::Ym2413RhythmChannelFrequency {
                channel: 8,
                fnum: 0x0fe,
                block: 7,
                melodic_key_on: true,
                sustain_on: false,
            }
        );
    }

    #[test]
    fn rhythm_volume_registers_are_not_decoded_as_melodic_instruments() {
        let mut registers = [0; 64];
        registers[0x36] = 0xf3;
        registers[0x37] = 0xa6;
        registers[0x38] = 0x5c;

        let config = super::decode_rhythm_registers(&registers);

        assert_eq!(config.bass_drum.volume, 3);
        assert_eq!(config.hi_hat.volume, 10);
        assert_eq!(config.snare_drum.volume, 6);
        assert_eq!(config.tom_tom.volume, 5);
        assert_eq!(config.top_cymbal.volume, 12);
    }

    #[test]
    fn rhythm_trigger_bits_are_decoded_independently_one_hot() {
        let cases = [
            (0x10, 0), // Bass Drum
            (0x08, 1), // Snare Drum
            (0x04, 2), // Tom-Tom
            (0x02, 3), // Top Cymbal
            (0x01, 4), // Hi-Hat
        ];

        for (trigger, voice_index) in cases {
            let mut registers = [0; 64];
            registers[0x0e] = super::YM2413_RHYTHM_MODE_BIT | trigger;
            let config = super::decode_rhythm_registers(&registers);
            let active = [
                config.bass_drum.key_on,
                config.snare_drum.key_on,
                config.tom_tom.key_on,
                config.top_cymbal.key_on,
                config.hi_hat.key_on,
            ];

            for (index, is_triggered) in active.into_iter().enumerate() {
                assert_eq!(
                    is_triggered,
                    index == voice_index,
                    "trigger bit {trigger:#04x} must select only voice index {voice_index}"
                );
            }
        }
    }

    #[test]
    fn slot_rate_lfsr_uses_the_measured_x23_x9_plus_one_recurrence() {
        let mut lfsr = Ym2413RhythmSlotNoiseLfsr::with_seed(1).unwrap();

        assert_eq!(lfsr.output_bit(), 1);
        assert_eq!(lfsr.clock(), 1);
        assert_eq!(lfsr.register(), 0x40_0000);
    }

    #[test]
    fn every_eighteenth_slot_rate_bit_obeys_the_decimated_noise_polynomial() {
        let mut lfsr = Ym2413RhythmSlotNoiseLfsr::with_seed(1).unwrap();
        let mut sampled = Vec::with_capacity(128);

        for _ in 0..128 {
            sampled.push(lfsr.output_bit());
            lfsr.advance(18);
        }

        for index in 23..sampled.len() {
            let expected =
                sampled[index - 1] ^ sampled[index - 8] ^ sampled[index - 9] ^ sampled[index - 23];
            assert_eq!(sampled[index], expected, "decimated bit index {index}");
        }
    }

    #[test]
    fn slot_rate_lfsr_matches_measured_three_slot_hh_to_sd_spacing_vector() {
        let mut lfsr = Ym2413RhythmSlotNoiseLfsr::with_seed(0x12_3457).unwrap();

        // Slot 13: HH samples the current bit.
        assert_eq!(lfsr.output_bit(), 1);
        lfsr.advance(3);

        // Slot 16: SD samples the bit after three operator-slot steps.
        assert_eq!(lfsr.output_bit(), 0);
        lfsr.advance(15);

        // The next rotation returns to HH at slot 13 after 18 total steps.
        assert_eq!(lfsr.output_bit(), 0);
    }

    #[test]
    fn slot_rate_lfsr_snapshot_restore_and_chunked_advance_are_equivalent() {
        let mut continuous = Ym2413RhythmSlotNoiseLfsr::with_seed(0x5a_1234).unwrap();
        let mut chunked = continuous;

        continuous.advance(100_003);
        for steps in [1, 7, 64, 3_001, 17, 65_536, 31_377] {
            chunked.advance(steps);
        }
        assert_eq!(continuous.export_state(), chunked.export_state());

        let snapshot = continuous.export_state();
        assert_eq!(
            Ym2413RhythmSlotNoiseLfsr::restore_state(snapshot),
            Ok(continuous)
        );
        assert_eq!(
            Ym2413RhythmSlotNoiseLfsr::restore_state(Ym2413RhythmSlotNoiseState {
                format_version: YM2413_RHYTHM_SLOT_NOISE_STATE_VERSION + 1,
                register: 1,
            }),
            Err(Ym2413RhythmNoiseError::StateVersionMismatch {
                expected: YM2413_RHYTHM_SLOT_NOISE_STATE_VERSION,
                actual: YM2413_RHYTHM_SLOT_NOISE_STATE_VERSION + 1,
            })
        );
    }
}
