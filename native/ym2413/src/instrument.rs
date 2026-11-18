//! YM2413 user-patch, channel-register and candidate preset decoding.
//!
//! The 15 melody preset byte patterns are adapted from the YM2413 bank in
//! digital-sound-antiques/emu2413 (MIT), commit
//! cef17c32f67a88df381e08168e5820984de7e936. They are candidate patch values,
//! not a verified physical ROM dump; andete reported that reverse-engineered
//! settings were close but not always identical to real YM2413 instruments.

pub const YM2413_PRESET_PATCH_SOURCE: &str =
    "https://github.com/digital-sound-antiques/emu2413/blob/cef17c32f67a88df381e08168e5820984de7e936/emu2413.c";
pub const YM2413_PRESET_PATCH_SOURCE_COMMIT: &str =
    "cef17c32f67a88df381e08168e5820984de7e936";
pub const YM2413_PRESET_PATCH_LICENSE: &str = "MIT";
pub const YM2413_PRESET_PATCHES_HARDWARE_VERIFIED: bool = false;

pub const YM2413_PRESET_NAMES: [&str; 15] = [
    "Violin",
    "Guitar",
    "Piano",
    "Flute",
    "Clarinet",
    "Oboe",
    "Trumpet",
    "Organ",
    "Horn",
    "Synthesizer",
    "Harpsichord",
    "Vibraphone",
    "Synthesizer Bass",
    "Acoustic Bass",
    "Electric Guitar",
];

/// Candidate bytes for instruments 1..15; instrument 0 is the user patch.
pub const YM2413_PRESET_PATCHES_CANDIDATE: [[u8; 8]; 15] = [
    [0x71, 0x61, 0x1e, 0x17, 0xd0, 0x78, 0x00, 0x17],
    [0x13, 0x41, 0x1a, 0x0d, 0xd8, 0xf7, 0x23, 0x13],
    [0x13, 0x01, 0x99, 0x00, 0xf2, 0xc4, 0x21, 0x23],
    [0x11, 0x61, 0x0e, 0x07, 0x8d, 0x64, 0x70, 0x27],
    [0x32, 0x21, 0x1e, 0x06, 0xe1, 0x76, 0x01, 0x28],
    [0x31, 0x22, 0x16, 0x05, 0xe0, 0x71, 0x00, 0x18],
    [0x21, 0x61, 0x1d, 0x07, 0x82, 0x81, 0x11, 0x07],
    [0x33, 0x21, 0x2d, 0x13, 0xb0, 0x70, 0x00, 0x07],
    [0x61, 0x61, 0x1b, 0x06, 0x64, 0x65, 0x10, 0x17],
    [0x41, 0x61, 0x0b, 0x18, 0x85, 0xf0, 0x81, 0x07],
    [0x33, 0x01, 0x83, 0x11, 0xea, 0xef, 0x10, 0x04],
    [0x17, 0xc1, 0x24, 0x07, 0xf8, 0xf8, 0x22, 0x12],
    [0x61, 0x50, 0x0c, 0x05, 0xd2, 0xf5, 0x40, 0x42],
    [0x01, 0x01, 0x55, 0x03, 0xe9, 0x90, 0x03, 0x02],
    [0x41, 0x41, 0x89, 0x03, 0xf1, 0xe4, 0xc0, 0x13],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatorPatch {
    pub am_enabled: bool,
    pub vibrato_enabled: bool,
    pub sustained_envelope: bool,
    pub key_scale_rate: bool,
    pub multiplier: u8,
    pub key_scale_level: u8,
    /// Modulator TL. The carrier uses the channel volume register instead.
    pub total_level: u8,
    pub half_wave: bool,
    pub attack_rate: u8,
    pub decay_rate: u8,
    pub sustain_level: u8,
    pub release_rate: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstrumentPatch {
    pub modulator: OperatorPatch,
    pub carrier: OperatorPatch,
    pub feedback: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelRegisterState {
    pub channel: u8,
    pub fnum: u16,
    pub block: u8,
    pub key_on: bool,
    pub sustain_on: bool,
    pub instrument: u8,
    pub volume: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentError {
    InvalidPreset(u8),
    InvalidChannel(u8),
}

/// Decode the eight user-tone bytes (or one eight-byte candidate preset).
pub const fn decode_patch_registers(bytes: [u8; 8]) -> InstrumentPatch {
    let mod_flags = bytes[0];
    let car_flags = bytes[1];

    let modulator = OperatorPatch {
        am_enabled: mod_flags & 0x80 != 0,
        vibrato_enabled: mod_flags & 0x40 != 0,
        sustained_envelope: mod_flags & 0x20 != 0,
        key_scale_rate: mod_flags & 0x10 != 0,
        multiplier: mod_flags & 0x0f,
        key_scale_level: bytes[2] >> 6,
        total_level: bytes[2] & 0x3f,
        half_wave: bytes[3] & 0x08 != 0,
        attack_rate: bytes[4] >> 4,
        decay_rate: bytes[4] & 0x0f,
        sustain_level: bytes[6] >> 4,
        release_rate: bytes[6] & 0x0f,
    };

    let carrier = OperatorPatch {
        am_enabled: car_flags & 0x80 != 0,
        vibrato_enabled: car_flags & 0x40 != 0,
        sustained_envelope: car_flags & 0x20 != 0,
        key_scale_rate: car_flags & 0x10 != 0,
        multiplier: car_flags & 0x0f,
        key_scale_level: bytes[3] >> 6,
        total_level: 0,
        half_wave: bytes[3] & 0x10 != 0,
        attack_rate: bytes[5] >> 4,
        decay_rate: bytes[5] & 0x0f,
        sustain_level: bytes[7] >> 4,
        release_rate: bytes[7] & 0x0f,
    };

    InstrumentPatch {
        modulator,
        carrier,
        feedback: bytes[3] & 0x07,
    }
}

/// Decode the live user patch from the YM2413 64-byte register image.
pub const fn decode_user_patch(registers: &[u8; 64]) -> InstrumentPatch {
    decode_patch_registers([
        registers[0],
        registers[1],
        registers[2],
        registers[3],
        registers[4],
        registers[5],
        registers[6],
        registers[7],
    ])
}

/// Resolve instrument code 0 to the live user patch and 1..15 to candidate ROM data.
pub fn resolve_instrument(
    instrument: u8,
    user_patch: InstrumentPatch,
) -> Result<InstrumentPatch, InstrumentError> {
    if instrument == 0 {
        return Ok(user_patch);
    }
    if instrument > 15 {
        return Err(InstrumentError::InvalidPreset(instrument));
    }
    Ok(decode_patch_registers(
        YM2413_PRESET_PATCHES_CANDIDATE[(instrument - 1) as usize],
    ))
}

/// Return the candidate preset patch for instrument code 1..15.
pub fn preset_patch(instrument: u8) -> Result<InstrumentPatch, InstrumentError> {
    resolve_instrument(instrument, decode_patch_registers([0; 8]))
}

/// Return the human-readable name for instrument code 1..15.
pub const fn preset_name(instrument: u8) -> Option<&'static str> {
    if instrument == 0 || instrument > 15 {
        None
    } else {
        Some(YM2413_PRESET_NAMES[(instrument - 1) as usize])
    }
}

/// Decode one of the nine melodic channel register triplets.
pub fn decode_channel_registers(
    registers: &[u8; 64],
    channel: u8,
) -> Result<ChannelRegisterState, InstrumentError> {
    if channel > 8 {
        return Err(InstrumentError::InvalidChannel(channel));
    }

    let index = usize::from(channel);
    let fnum_low = registers[0x10 + index];
    let fnum_high = registers[0x20 + index];
    let instrument_volume = registers[0x30 + index];

    Ok(ChannelRegisterState {
        channel,
        fnum: u16::from(fnum_low) | (u16::from(fnum_high & 0x01) << 8),
        block: (fnum_high >> 1) & 0x07,
        key_on: fnum_high & 0x10 != 0,
        sustain_on: fnum_high & 0x20 != 0,
        instrument: instrument_volume >> 4,
        volume: instrument_volume & 0x0f,
    })
}

/// Return whether register 0x0E selects rhythm mode.
pub const fn rhythm_mode_enabled(registers: &[u8; 64]) -> bool {
    registers[0x0e] & 0x20 != 0
}

#[cfg(test)]
mod tests {
    use super::{
        decode_channel_registers, decode_patch_registers, decode_user_patch, preset_name,
        preset_patch, resolve_instrument, rhythm_mode_enabled, InstrumentError,
        YM2413_PRESET_NAMES, YM2413_PRESET_PATCHES_CANDIDATE,
    };

    #[test]
    fn user_patch_registers_decode_operator_fields() {
        let patch = decode_patch_registers([
            0xf2, 0xb5, 0xaf, 0xdb, 0xa5, 0x3c, 0x6d, 0xf2,
        ]);

        assert!(patch.modulator.am_enabled);
        assert!(patch.modulator.vibrato_enabled);
        assert!(patch.modulator.sustained_envelope);
        assert!(patch.modulator.key_scale_rate);
        assert_eq!(patch.modulator.multiplier, 2);
        assert_eq!(patch.modulator.key_scale_level, 2);
        assert_eq!(patch.modulator.total_level, 47);
        assert!(patch.modulator.half_wave);
        assert_eq!(patch.modulator.attack_rate, 10);
        assert_eq!(patch.modulator.decay_rate, 5);
        assert_eq!(patch.modulator.sustain_level, 6);
        assert_eq!(patch.modulator.release_rate, 13);

        assert!(patch.carrier.am_enabled);
        assert!(!patch.carrier.vibrato_enabled);
        assert!(patch.carrier.sustained_envelope);
        assert!(patch.carrier.key_scale_rate);
        assert_eq!(patch.carrier.multiplier, 5);
        assert_eq!(patch.carrier.key_scale_level, 3);
        assert_eq!(patch.carrier.total_level, 0);
        assert!(patch.carrier.half_wave);
        assert_eq!(patch.carrier.attack_rate, 3);
        assert_eq!(patch.carrier.decay_rate, 12);
        assert_eq!(patch.carrier.sustain_level, 15);
        assert_eq!(patch.carrier.release_rate, 2);
        assert_eq!(patch.feedback, 3);
    }

    #[test]
    fn user_patch_reads_the_first_eight_registers() {
        let mut registers = [0; 64];
        registers[..8].copy_from_slice(&[0x71, 0x61, 0x1e, 0x17, 0xd0, 0x78, 0, 0x17]);
        assert_eq!(decode_user_patch(&registers), decode_patch_registers([
            0x71, 0x61, 0x1e, 0x17, 0xd0, 0x78, 0, 0x17,
        ]));
    }

    #[test]
    fn all_fifteen_candidate_presets_have_names_and_decode() {
        assert_eq!(YM2413_PRESET_PATCHES_CANDIDATE.len(), 15);
        assert_eq!(YM2413_PRESET_NAMES.len(), 15);
        for instrument in 1..=15 {
            assert!(preset_patch(instrument).is_ok());
            assert!(preset_name(instrument).is_some());
        }
    }

    #[test]
    fn instrument_zero_resolves_to_user_patch() {
        let user = decode_patch_registers([0x71, 0x61, 0x1e, 0x17, 0xd0, 0x78, 0, 0x17]);
        assert_eq!(resolve_instrument(0, user), Ok(user));
    }

    #[test]
    fn invalid_instrument_codes_are_rejected() {
        let user = decode_patch_registers([0; 8]);
        assert_eq!(
            preset_patch(0),
            Err(InstrumentError::InvalidPreset(0))
        );
        assert_eq!(
            resolve_instrument(16, user),
            Err(InstrumentError::InvalidPreset(16))
        );
        assert_eq!(preset_name(0), None);
        assert_eq!(preset_name(16), None);
    }

    #[test]
    fn channel_register_triplet_decodes_fnum_block_key_instrument_and_volume() {
        let mut registers = [0; 64];
        registers[0x18] = 0xa5;
        registers[0x28] = 0x35;
        registers[0x38] = 0xaf;

        let channel = decode_channel_registers(&registers, 8).unwrap();
        assert_eq!(channel.channel, 8);
        assert_eq!(channel.fnum, 0x1a5);
        assert_eq!(channel.block, 2);
        assert!(channel.key_on);
        assert!(channel.sustain_on);
        assert_eq!(channel.instrument, 10);
        assert_eq!(channel.volume, 15);
    }

    #[test]
    fn invalid_channel_is_rejected() {
        assert_eq!(
            decode_channel_registers(&[0; 64], 9),
            Err(InstrumentError::InvalidChannel(9))
        );
    }

    #[test]
    fn rhythm_mode_uses_register_zero_e_bit_five() {
        let mut registers = [0; 64];
        assert!(!rhythm_mode_enabled(&registers));
        registers[0x0e] = 0x20;
        assert!(rhythm_mode_enabled(&registers));
    }
}
