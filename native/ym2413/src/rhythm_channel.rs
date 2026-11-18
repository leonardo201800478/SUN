//! Static register-to-operator configuration for the five YM2413 rhythm voices.
//!
//! This module resolves the existing rhythm register decoder, measured patch
//! candidates, physical slot ownership, PG/EG inputs and KSL. It does not
//! advance PG/EG/LFSR state, synthesize PCM, infer KEY-ON interaction, or own
//! the YM2413 slot scheduler. Candidate patch data remains non-hardware-verified.

use crate::channel::OperatorPhaseParameters;
use crate::envelope::EnvelopeParameters;
use crate::instrument::{
    InstrumentPatch, OperatorPatch, Ym2413RhythmInstrument, rhythm_patch_candidate,
};
use crate::key_scale::{KeyScaleError, key_scale_attenuation};
use crate::rhythm::{Ym2413RhythmChannelFrequency, Ym2413RhythmConfig, decode_rhythm_registers};
use crate::slot::{Ym2413Operator, Ym2413RhythmVoice};

/// Errors raised while resolving static rhythm-voice synthesis inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhythmChannelSynthesisError {
    /// BLOCK/FNUM/KSL could not be represented by the shared KSL model.
    KeyScale(KeyScaleError),
}

impl From<KeyScaleError> for RhythmChannelSynthesisError {
    fn from(value: KeyScaleError) -> Self {
        Self::KeyScale(value)
    }
}

/// Static inputs for one physical rhythm operator slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RhythmOperatorSynthesisConfig {
    /// Zero-based physical channel index (6, 7 or 8).
    pub physical_channel: u8,
    /// Physical slot number from the Yamaha serial slot table (13..=18).
    pub slot_number: u8,
    /// Physical role of this operator in the melodic channel layout.
    pub physical_role: Ym2413Operator,
    /// Candidate operator parameters selected for this rhythm voice.
    pub patch: OperatorPatch,
    /// PG inputs derived from the channel frequency and candidate multiplier.
    pub phase: OperatorPhaseParameters,
    /// EG inputs derived from the candidate patch and raw channel SUS bit.
    pub envelope: EnvelopeParameters,
    /// KSL attenuation in EG units.
    pub ksl: u8,
}

/// Resolved static configuration for one percussion voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RhythmVoiceSynthesisConfig {
    /// Logical rhythm voice.
    pub voice: Ym2413RhythmVoice,
    /// Raw trigger bit from register $0E (not a complete activity predicate).
    pub trigger: bool,
    /// Raw four-bit attenuation from the corresponding $36-$38 nibble.
    pub volume: u8,
    /// Raw channel 7, 8 or 9 frequency and melodic KEY-ON/SUS state.
    pub channel_frequency: Ym2413RhythmChannelFrequency,
    /// Modulator-side physical operator, when used by this voice.
    pub modulator: Option<RhythmOperatorSynthesisConfig>,
    /// Carrier-side physical operator, when used by this voice.
    pub carrier: Option<RhythmOperatorSynthesisConfig>,
    /// True for single-operator voices, which use carrier amplitude scaling
    /// even when the physical slot is a modulator.
    pub single_operator_uses_carrier_path: bool,
    /// Candidate feedback setting (meaningful for the two-operator BD voice).
    pub feedback: u8,
}

/// Static configuration of all five Rhythm Mode voices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmSynthesisConfig {
    /// Global RHY bit from register $0E.
    pub rhythm_mode_enabled: bool,
    /// Voices in stable order: BD, HH, SD, TOM, TCY.
    pub voices: [RhythmVoiceSynthesisConfig; 5],
}

fn operator_config(
    channel: Ym2413RhythmChannelFrequency,
    slot_number: u8,
    physical_role: Ym2413Operator,
    patch: OperatorPatch,
) -> Result<RhythmOperatorSynthesisConfig, RhythmChannelSynthesisError> {
    let phase = OperatorPhaseParameters {
        fnum: channel.fnum,
        block: channel.block,
        multiplier: patch.multiplier,
        pm_enabled: patch.vibrato_enabled,
    };
    let envelope = EnvelopeParameters {
        attack_rate: patch.attack_rate,
        decay_rate: patch.decay_rate,
        release_rate: patch.release_rate,
        sustain_level: patch.sustain_level,
        key_scale_rate: patch.key_scale_rate,
        fnum: channel.fnum,
        block: channel.block,
        sustain_on: channel.sustain_on,
        sustained_tone: patch.sustained_envelope,
    };
    let ksl = key_scale_attenuation(channel.block, channel.fnum, patch.key_scale_level)?;
    Ok(RhythmOperatorSynthesisConfig {
        physical_channel: channel.channel,
        slot_number,
        physical_role,
        patch,
        phase,
        envelope,
        ksl,
    })
}

fn build_voice(
    voice: Ym2413RhythmVoice,
    trigger: bool,
    volume: u8,
    channel: Ym2413RhythmChannelFrequency,
    instrument: Ym2413RhythmInstrument,
    modulator_slot: Option<u8>,
    carrier_slot: Option<u8>,
    single_operator_uses_carrier_path: bool,
) -> Result<RhythmVoiceSynthesisConfig, RhythmChannelSynthesisError> {
    let patch: InstrumentPatch = rhythm_patch_candidate(instrument);
    let modulator = match modulator_slot {
        Some(slot) => Some(operator_config(
            channel,
            slot,
            Ym2413Operator::Modulator,
            patch.modulator,
        )?),
        None => None,
    };
    let carrier = match carrier_slot {
        Some(slot) => Some(operator_config(
            channel,
            slot,
            Ym2413Operator::Carrier,
            patch.carrier,
        )?),
        None => None,
    };
    Ok(RhythmVoiceSynthesisConfig {
        voice,
        trigger,
        volume,
        channel_frequency: channel,
        modulator,
        carrier,
        single_operator_uses_carrier_path,
        feedback: patch.feedback,
    })
}

/// Decode all five rhythm voices into reusable static PG/EG/operator inputs.
///
/// The existing `decode_rhythm_registers` remains the source of register
/// semantics. This builder preserves raw melodic KEY-ON and SUS bits and does
/// not decide whether KEY-ON is ORed with the $0E rhythm trigger. The manual
/// asks software to clear melodic KEY-ON in Rhythm Mode; the conflicting
/// reverse-engineering observation is intentionally left unresolved.
///
/// Physical slot numbers follow the existing `YM2413_SLOT_MAP`: BD uses
/// channel 7 slots 13/16; HH uses channel 8 slot 14; TOM uses channel 9 slot
/// 15; SD uses channel 8 slot 17; TCY uses channel 9 slot 18.
pub fn decode_rhythm_synthesis(
    registers: &[u8; 64],
) -> Result<Ym2413RhythmSynthesisConfig, RhythmChannelSynthesisError> {
    let decoded: Ym2413RhythmConfig = decode_rhythm_registers(registers);
    let [channel7, channel8, channel9] = decoded.channel_frequencies;

    let voices = [
        build_voice(
            Ym2413RhythmVoice::BassDrum,
            decoded.bass_drum.key_on,
            decoded.bass_drum.volume,
            channel7,
            Ym2413RhythmInstrument::BassDrum,
            Some(13),
            Some(16),
            false,
        )?,
        build_voice(
            Ym2413RhythmVoice::HiHat,
            decoded.hi_hat.key_on,
            decoded.hi_hat.volume,
            channel8,
            Ym2413RhythmInstrument::HiHat,
            Some(14),
            None,
            true,
        )?,
        build_voice(
            Ym2413RhythmVoice::SnareDrum,
            decoded.snare_drum.key_on,
            decoded.snare_drum.volume,
            channel8,
            Ym2413RhythmInstrument::SnareDrum,
            None,
            Some(17),
            true,
        )?,
        build_voice(
            Ym2413RhythmVoice::TomTom,
            decoded.tom_tom.key_on,
            decoded.tom_tom.volume,
            channel9,
            Ym2413RhythmInstrument::TomTom,
            Some(15),
            None,
            true,
        )?,
        build_voice(
            Ym2413RhythmVoice::TopCymbal,
            decoded.top_cymbal.key_on,
            decoded.top_cymbal.volume,
            channel9,
            Ym2413RhythmInstrument::TopCymbal,
            None,
            Some(18),
            true,
        )?,
    ];

    Ok(Ym2413RhythmSynthesisConfig {
        rhythm_mode_enabled: decoded.rhythm_mode_enabled,
        voices,
    })
}

#[cfg(test)]
mod tests {
    use super::{decode_rhythm_synthesis, RhythmVoiceSynthesisConfig};
    use crate::slot::{Ym2413Operator, Ym2413RhythmVoice};

    fn configured_registers() -> [u8; 64] {
        let mut registers = [0; 64];
        registers[0x0e] = 0x3f;
        registers[0x10 + 6] = 0x55;
        registers[0x20 + 6] = 0x35;
        registers[0x10 + 7] = 0xaa;
        registers[0x20 + 7] = 0x26;
        registers[0x10 + 8] = 0x33;
        registers[0x20 + 8] = 0x3b;
        registers[0x36] = 0xa3;
        registers[0x37] = 0x5c;
        registers[0x38] = 0xe7;
        registers
    }

    fn assert_operator(
        operator: super::RhythmOperatorSynthesisConfig,
        channel: u8,
        slot: u8,
        role: Ym2413Operator,
    ) {
        assert_eq!(operator.physical_channel, channel);
        assert_eq!(operator.slot_number, slot);
        assert_eq!(operator.physical_role, role);
        assert_eq!(operator.phase.fnum, operator.envelope.fnum);
        assert_eq!(operator.phase.block, operator.envelope.block);
        assert_eq!(operator.phase.multiplier, operator.patch.multiplier);
    }

    #[test]
    fn maps_all_five_voices_to_their_measured_channels_slots_and_roles() {
        let config = decode_rhythm_synthesis(&configured_registers()).unwrap();
        assert_eq!(
            config.voices.map(|voice| voice.voice),
            [
                Ym2413RhythmVoice::BassDrum,
                Ym2413RhythmVoice::HiHat,
                Ym2413RhythmVoice::SnareDrum,
                Ym2413RhythmVoice::TomTom,
                Ym2413RhythmVoice::TopCymbal,
            ]
        );
        let [bd, hh, sd, tom, tcy] = config.voices;
        assert_operator(bd.modulator.unwrap(), 6, 13, Ym2413Operator::Modulator);
        assert_operator(bd.carrier.unwrap(), 6, 16, Ym2413Operator::Carrier);
        assert_operator(hh.modulator.unwrap(), 7, 14, Ym2413Operator::Modulator);
        assert_operator(sd.carrier.unwrap(), 7, 17, Ym2413Operator::Carrier);
        assert_operator(tom.modulator.unwrap(), 8, 15, Ym2413Operator::Modulator);
        assert_operator(tcy.carrier.unwrap(), 8, 18, Ym2413Operator::Carrier);
        assert!(hh.single_operator_uses_carrier_path);
        assert!(sd.single_operator_uses_carrier_path);
        assert!(tom.single_operator_uses_carrier_path);
        assert!(tcy.single_operator_uses_carrier_path);
        assert!(!bd.single_operator_uses_carrier_path);
    }

    #[test]
    fn maps_trigger_bits_and_volume_nibbles_without_reordering() {
        let config = decode_rhythm_synthesis(&configured_registers()).unwrap();
        let [bd, hh, sd, tom, tcy] = config.voices;
        assert!(config.rhythm_mode_enabled);
        assert_eq!([bd.trigger, hh.trigger, sd.trigger, tom.trigger, tcy.trigger], [true; 5]);
        assert_eq!([bd.volume, hh.volume, sd.volume, tom.volume, tcy.volume], [3, 5, 12, 14, 7]);
    }

    #[test]
    fn retains_channel_frequency_and_raw_melodic_key_and_sustain_bits() {
        let config = decode_rhythm_synthesis(&configured_registers()).unwrap();
        let [bd, hh, sd, tom, tcy] = config.voices;
        assert_eq!(bd.channel_frequency.fnum, 0x155);
        assert_eq!(bd.channel_frequency.block, 2);
        assert!(bd.channel_frequency.melodic_key_on);
        assert!(bd.channel_frequency.sustain_on);
        assert_eq!(hh.channel_frequency.fnum, 0x0aa);
        assert!(!hh.channel_frequency.melodic_key_on);
        assert!(hh.channel_frequency.sustain_on);
        assert_eq!(tom.channel_frequency.fnum, 0x133);
        assert!(tom.channel_frequency.melodic_key_on);
        assert!(tom.channel_frequency.sustain_on);
    }

    #[test]
    fn rhythm_mode_off_preserves_raw_fields_but_disables_global_mode() {
        let mut registers = configured_registers();
        registers[0x0e] &= !0x20;
        let config = decode_rhythm_synthesis(&registers).unwrap();
        assert!(!config.rhythm_mode_enabled);
        assert!(config.voices.iter().all(|voice| voice.trigger));
    }

    #[test]
    fn candidate_patch_data_is_selected_for_each_voice() {
        let config = decode_rhythm_synthesis(&configured_registers()).unwrap();
        let [bd, hh, sd, tom, tcy] = config.voices;
        assert_eq!(bd.modulator.unwrap().patch.multiplier, 1);
        assert_eq!(hh.modulator.unwrap().patch.multiplier, 1);
        assert_eq!(sd.carrier.unwrap().patch.multiplier, 1);
        assert_eq!(tom.modulator.unwrap().patch.multiplier, 5);
        assert_eq!(tcy.carrier.unwrap().patch.multiplier, 1);
    }

    #[test]
    fn operator_sustain_input_is_preserved_per_physical_channel() {
        let mut registers = configured_registers();
        registers[0x27] &= !0x20;
        let config = decode_rhythm_synthesis(&registers).unwrap();
        let RhythmVoiceSynthesisConfig { modulator, carrier, .. } = config.voices[1];
        assert!(!modulator.unwrap().envelope.sustain_on);
        assert!(!config.voices[2].carrier.unwrap().envelope.sustain_on);
        assert!(config.voices[0].modulator.unwrap().envelope.sustain_on);
        assert!(carrier.is_none());
    }
}
