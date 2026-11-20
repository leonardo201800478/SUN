//! Static register-to-operator configuration for the five YM2413 rhythm voices.
//!
//! This module resolves the existing rhythm register decoder, measured patch
//! candidates, physical slot ownership, PG/EG inputs and KSL. It does not
//! advance PG/EG/LFSR state, synthesize PCM, infer KEY-ON interaction, or own
//! the YM2413 slot scheduler. Candidate patch data remains non-hardware-verified.

use crate::channel::OperatorPhaseParameters;
use crate::envelope::EnvelopeParameters;
use crate::envelope_sequencer::{EnvelopeRhythmSelect, EnvelopeSlotInput};
use crate::instrument::{
    InstrumentPatch, OperatorPatch, Ym2413RhythmInstrument, rhythm_patch_candidate,
};
use crate::key_scale::{KeyScaleError, key_scale_attenuation};
use crate::operator::{
    OperatorPairError, OperatorPairParameters, SingleOperatorCarrierParameters,
    evaluate_single_operator_as_carrier,
};
use crate::rhythm::{Ym2413RhythmChannelFrequency, Ym2413RhythmConfig, decode_rhythm_registers};
use crate::slot::{Ym2413Operator, Ym2413RhythmVoice};

/// Errors raised while resolving static rhythm-voice synthesis inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhythmChannelSynthesisError {
    /// BLOCK/FNUM/KSL could not be represented by the shared KSL model.
    KeyScale(KeyScaleError),
    /// The requested serial EG cycle is not one of the six rhythm cycles.
    InvalidEnvelopeCycle { cycle: u8 },
    /// The selected rhythm cycle has no operator in its declared physical role.
    MissingEnvelopeOperator { cycle: u8 },
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

/// Errors raised while evaluating the current operator inputs of one rhythm voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhythmVoiceSynthesisError {
    /// The requested evaluator does not match this voice's physical operator layout.
    InvalidOperatorLayout { voice: Ym2413RhythmVoice },
    /// A single-operator voice requires a phase produced by its phase source.
    MissingPhaseInput { voice: Ym2413RhythmVoice },
    /// Bass Drum derives both phases from its existing FM operator pair.
    UnexpectedPhaseInput { voice: Ym2413RhythmVoice },
    /// The existing digital operator evaluator rejected one of its inputs.
    Operator(OperatorPairError),
}

impl From<OperatorPairError> for RhythmVoiceSynthesisError {
    fn from(value: OperatorPairError) -> Self {
        Self::Operator(value)
    }
}

impl RhythmVoiceSynthesisConfig {
    /// Build the carrier-path inputs for a one-operator rhythm voice.
    ///
    /// The physical operator may occupy either the modulator or carrier slot;
    /// its amplitude is nevertheless evaluated through the carrier path.
    /// Envelope level and AM level are current dynamic values supplied by the
    /// caller. This method does not advance either generator.
    pub fn single_operator_parameters(
        &self,
        envelope_level: u8,
        am_level: u8,
    ) -> Result<SingleOperatorCarrierParameters, RhythmVoiceSynthesisError> {
        if !self.single_operator_uses_carrier_path {
            return Err(RhythmVoiceSynthesisError::InvalidOperatorLayout { voice: self.voice });
        }
        let operator = match (self.modulator, self.carrier) {
            (Some(operator), None) | (None, Some(operator)) => operator,
            _ => {
                return Err(RhythmVoiceSynthesisError::InvalidOperatorLayout { voice: self.voice });
            }
        };
        let parameters = SingleOperatorCarrierParameters {
            volume: self.volume,
            ksl: operator.ksl,
            envelope: envelope_level,
            am: if operator.patch.am_enabled {
                am_level
            } else {
                0
            },
            half_wave: operator.patch.half_wave,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    /// Evaluate one single-operator rhythm voice at a caller-supplied PG phase.
    ///
    /// The phase index and EG/AM values must already represent the correct
    /// hardware slot boundary. No PG, EG, LFSR, feedback or chip time advances.
    pub fn evaluate_single_operator(
        &self,
        phase_index: u16,
        envelope_level: u8,
        am_level: u8,
    ) -> Result<i16, RhythmVoiceSynthesisError> {
        let parameters = self.single_operator_parameters(envelope_level, am_level)?;
        Ok(evaluate_single_operator_as_carrier(
            phase_index,
            parameters,
        )?)
    }

    /// Build the normal two-operator FM inputs for the Bass Drum voice.
    ///
    /// The caller supplies the current EG levels and AM-LFO value. The returned
    /// parameters can be passed to the existing stateful OperatorPair; this
    /// builder itself does not mutate feedback history or advance chip time.
    pub fn bass_drum_operator_pair_parameters(
        &self,
        modulator_envelope: u8,
        carrier_envelope: u8,
        am_level: u8,
    ) -> Result<OperatorPairParameters, RhythmVoiceSynthesisError> {
        if self.voice != Ym2413RhythmVoice::BassDrum {
            return Err(RhythmVoiceSynthesisError::InvalidOperatorLayout { voice: self.voice });
        }
        let (Some(modulator), Some(carrier)) = (self.modulator, self.carrier) else {
            return Err(RhythmVoiceSynthesisError::InvalidOperatorLayout { voice: self.voice });
        };
        let parameters = OperatorPairParameters {
            feedback: self.feedback,
            modulator_total_level: modulator.patch.total_level,
            modulator_ksl: modulator.ksl,
            modulator_envelope,
            modulator_am: if modulator.patch.am_enabled {
                am_level
            } else {
                0
            },
            modulator_half_wave: modulator.patch.half_wave,
            carrier_volume: self.volume,
            carrier_ksl: carrier.ksl,
            carrier_envelope,
            carrier_am: if carrier.patch.am_enabled {
                am_level
            } else {
                0
            },
            carrier_half_wave: carrier.patch.half_wave,
        };
        parameters.validate()?;
        Ok(parameters)
    }
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

struct RhythmVoiceBuild {
    voice: Ym2413RhythmVoice,
    trigger: bool,
    volume: u8,
    channel: Ym2413RhythmChannelFrequency,
    instrument: Ym2413RhythmInstrument,
    modulator_slot: Option<u8>,
    carrier_slot: Option<u8>,
    single_operator_uses_carrier_path: bool,
}

fn build_voice(
    build: RhythmVoiceBuild,
) -> Result<RhythmVoiceSynthesisConfig, RhythmChannelSynthesisError> {
    let patch: InstrumentPatch = rhythm_patch_candidate(build.instrument);
    let modulator = match build.modulator_slot {
        Some(slot) => Some(operator_config(
            build.channel,
            slot,
            Ym2413Operator::Modulator,
            patch.modulator,
        )?),
        None => None,
    };
    let carrier = match build.carrier_slot {
        Some(slot) => Some(operator_config(
            build.channel,
            slot,
            Ym2413Operator::Carrier,
            patch.carrier,
        )?),
        None => None,
    };
    Ok(RhythmVoiceSynthesisConfig {
        voice: build.voice,
        trigger: build.trigger,
        volume: build.volume,
        channel_frequency: build.channel,
        modulator,
        carrier,
        single_operator_uses_carrier_path: build.single_operator_uses_carrier_path,
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
        build_voice(RhythmVoiceBuild {
            voice: Ym2413RhythmVoice::BassDrum,
            trigger: decoded.bass_drum.key_on,
            volume: decoded.bass_drum.volume,
            channel: channel7,
            instrument: Ym2413RhythmInstrument::BassDrum,
            modulator_slot: Some(13),
            carrier_slot: Some(16),
            single_operator_uses_carrier_path: false,
        })?,
        build_voice(RhythmVoiceBuild {
            voice: Ym2413RhythmVoice::HiHat,
            trigger: decoded.hi_hat.key_on,
            volume: decoded.hi_hat.volume,
            channel: channel8,
            instrument: Ym2413RhythmInstrument::HiHat,
            modulator_slot: Some(14),
            carrier_slot: None,
            single_operator_uses_carrier_path: true,
        })?,
        build_voice(RhythmVoiceBuild {
            voice: Ym2413RhythmVoice::SnareDrum,
            trigger: decoded.snare_drum.key_on,
            volume: decoded.snare_drum.volume,
            channel: channel8,
            instrument: Ym2413RhythmInstrument::SnareDrum,
            modulator_slot: None,
            carrier_slot: Some(17),
            single_operator_uses_carrier_path: true,
        })?,
        build_voice(RhythmVoiceBuild {
            voice: Ym2413RhythmVoice::TomTom,
            trigger: decoded.tom_tom.key_on,
            volume: decoded.tom_tom.volume,
            channel: channel9,
            instrument: Ym2413RhythmInstrument::TomTom,
            modulator_slot: Some(15),
            carrier_slot: None,
            single_operator_uses_carrier_path: true,
        })?,
        build_voice(RhythmVoiceBuild {
            voice: Ym2413RhythmVoice::TopCymbal,
            trigger: decoded.top_cymbal.key_on,
            volume: decoded.top_cymbal.volume,
            channel: channel9,
            instrument: Ym2413RhythmInstrument::TopCymbal,
            modulator_slot: None,
            carrier_slot: Some(18),
            single_operator_uses_carrier_path: true,
        })?,
    ];

    Ok(Ym2413RhythmSynthesisConfig {
        rhythm_mode_enabled: decoded.rhythm_mode_enabled,
        voices,
    })
}


/// Resolve the register-derived EG input for one of the six Rhythm Mode cycles.
///
/// The serial EG selector order is BD0, HH, TOM, BD1, SD, TC at cycles
/// 11 through 16. The rate fields come from the corresponding dedicated
/// rhythm patch candidate; raw channel KEY-ON/SUS and $0E trigger bits remain
/// separate inputs so the sequencer owns their delayed interaction.
pub fn decode_rhythm_envelope_slot_input(
    registers: &[u8; 64],
    cycle: u8,
) -> Result<EnvelopeSlotInput, RhythmChannelSynthesisError> {
    let (voice_index, carrier, selector) = match cycle {
        11 => (0, false, EnvelopeRhythmSelect::BassDrumFirst),
        12 => (1, false, EnvelopeRhythmSelect::HiHat),
        13 => (3, false, EnvelopeRhythmSelect::TomTom),
        14 => (0, true, EnvelopeRhythmSelect::BassDrumSecond),
        15 => (2, true, EnvelopeRhythmSelect::SnareDrum),
        16 => (4, true, EnvelopeRhythmSelect::TopCymbal),
        _ => return Err(RhythmChannelSynthesisError::InvalidEnvelopeCycle { cycle }),
    };

    let config = decode_rhythm_synthesis(registers)?;
    let voice = config.voices[voice_index];
    let operator = if carrier { voice.carrier } else { voice.modulator }
        .ok_or(RhythmChannelSynthesisError::MissingEnvelopeOperator { cycle })?;
    let envelope = operator.envelope;
    let channel = voice.channel_frequency;

    Ok(EnvelopeSlotInput {
        attack_rate: envelope.attack_rate,
        decay_rate: envelope.decay_rate,
        release_rate: envelope.release_rate,
        sustain_level: envelope.sustain_level,
        key_scale_rate: envelope.key_scale_rate,
        ksr_frequency: ((envelope.block << 1) | (envelope.fnum >> 8) as u8) & 0x0f,
        sustain_on: channel.sustain_on,
        sustained_tone: envelope.sustained_tone,
        key_on: channel.melodic_key_on,
        rhythm_select: selector,
        rhythm_bits: registers[0x0e] & 0x3f,
    })
}

#[cfg(test)]
mod tests {
    use super::{RhythmVoiceSynthesisConfig, decode_rhythm_synthesis};
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
    fn serial_rhythm_cycles_resolve_the_dedicated_operator_envelopes() {
        use super::decode_rhythm_envelope_slot_input;
        use crate::envelope_sequencer::EnvelopeRhythmSelect as Rhythm;

        let registers = configured_registers();
        let expected = [
            (11, Rhythm::BassDrumFirst, 13, 15, 10, 6),
            (12, Rhythm::HiHat, 12, 8, 7, 10),
            (13, Rhythm::TomTom, 15, 8, 9, 5),
            (14, Rhythm::BassDrumSecond, 15, 8, 13, 6),
            (15, Rhythm::SnareDrum, 13, 8, 8, 6),
            (16, Rhythm::TopCymbal, 10, 10, 5, 5),
        ];

        for (cycle, selector, attack, decay, release, sustain) in expected {
            let input = decode_rhythm_envelope_slot_input(&registers, cycle).unwrap();
            assert_eq!(input.rhythm_select, selector, "cycle {cycle}");
            assert_eq!(input.rhythm_bits, registers[0x0e] & 0x3f, "cycle {cycle}");
            assert_eq!(input.attack_rate, attack, "cycle {cycle}");
            assert_eq!(input.decay_rate, decay, "cycle {cycle}");
            assert_eq!(input.release_rate, release, "cycle {cycle}");
            assert_eq!(input.sustain_level, sustain, "cycle {cycle}");
            assert!(input.key_on, "cycle {cycle}");
            assert!(input.sustain_on, "cycle {cycle}");
        }
    }

    #[test]
    fn rhythm_eg_decoder_rejects_cycles_outside_the_six_rhythm_slots() {
        use super::{RhythmChannelSynthesisError, decode_rhythm_envelope_slot_input};

        assert_eq!(
            decode_rhythm_envelope_slot_input(&configured_registers(), 10),
            Err(RhythmChannelSynthesisError::InvalidEnvelopeCycle { cycle: 10 })
        );
        assert_eq!(
            decode_rhythm_envelope_slot_input(&configured_registers(), 17),
            Err(RhythmChannelSynthesisError::InvalidEnvelopeCycle { cycle: 17 })
        );
    }

    #[test]
    fn maps_trigger_bits_and_volume_nibbles_without_reordering() {
        let config = decode_rhythm_synthesis(&configured_registers()).unwrap();
        let [bd, hh, sd, tom, tcy] = config.voices;
        assert!(config.rhythm_mode_enabled);
        assert_eq!(
            [bd.trigger, hh.trigger, sd.trigger, tom.trigger, tcy.trigger],
            [true; 5]
        );
        assert_eq!(
            [bd.volume, hh.volume, sd.volume, tom.volume, tcy.volume],
            [3, 5, 12, 14, 7]
        );
    }

    #[test]
    fn retains_channel_frequency_and_raw_melodic_key_and_sustain_bits() {
        let config = decode_rhythm_synthesis(&configured_registers()).unwrap();
        let [bd, hh, _, tom, _] = config.voices;
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
    fn single_operator_parameter_builder_uses_voice_volume_and_carrier_scaling() {
        let config = decode_rhythm_synthesis(&configured_registers()).unwrap();
        let hh = config.voices[1];
        let operator = hh.modulator.unwrap();
        let parameters = hh.single_operator_parameters(37, 13).unwrap();
        assert_eq!(parameters.volume, hh.volume);
        assert_eq!(parameters.ksl, operator.ksl);
        assert_eq!(parameters.envelope, 37);
        assert_eq!(parameters.am, 0);
        assert_eq!(parameters.half_wave, operator.patch.half_wave);

        let mut am_enabled = hh;
        let mut physical_operator = operator;
        physical_operator.patch.am_enabled = true;
        am_enabled.modulator = Some(physical_operator);
        assert_eq!(
            am_enabled.single_operator_parameters(37, 11).unwrap().am,
            11
        );
    }

    #[test]
    fn single_operator_evaluator_reuses_the_shared_carrier_path() {
        use crate::operator::evaluate_single_operator_as_carrier;

        let config = decode_rhythm_synthesis(&configured_registers()).unwrap();
        let voice = config.voices[3];
        let parameters = voice.single_operator_parameters(29, 7).unwrap();
        assert_eq!(
            voice.evaluate_single_operator(0x155, 29, 7),
            evaluate_single_operator_as_carrier(0x155, parameters)
                .map_err(super::RhythmVoiceSynthesisError::from)
        );
        assert_eq!(
            voice.single_operator_parameters(128, 0),
            Err(super::RhythmVoiceSynthesisError::Operator(
                crate::operator::OperatorPairError::InvalidEnvelope(128)
            ))
        );
    }

    #[test]
    fn bass_drum_builder_produces_normal_two_operator_fm_parameters() {
        let config = decode_rhythm_synthesis(&configured_registers()).unwrap();
        let bd = config.voices[0];
        let modulator = bd.modulator.unwrap();
        let carrier = bd.carrier.unwrap();
        let parameters = bd.bass_drum_operator_pair_parameters(23, 41, 9).unwrap();
        assert_eq!(parameters.feedback, bd.feedback);
        assert_eq!(
            parameters.modulator_total_level,
            modulator.patch.total_level
        );
        assert_eq!(parameters.modulator_ksl, modulator.ksl);
        assert_eq!(parameters.modulator_envelope, 23);
        assert_eq!(parameters.modulator_am, 0);
        assert_eq!(parameters.carrier_volume, bd.volume);
        assert_eq!(parameters.carrier_ksl, carrier.ksl);
        assert_eq!(parameters.carrier_envelope, 41);
        assert_eq!(parameters.carrier_am, 0);
    }

    #[test]
    fn rhythm_voice_evaluators_reject_incompatible_operator_layouts() {
        let config = decode_rhythm_synthesis(&configured_registers()).unwrap();
        assert_eq!(
            config.voices[0].single_operator_parameters(10, 0),
            Err(super::RhythmVoiceSynthesisError::InvalidOperatorLayout {
                voice: Ym2413RhythmVoice::BassDrum,
            })
        );
        assert_eq!(
            config.voices[1].bass_drum_operator_pair_parameters(10, 10, 0),
            Err(super::RhythmVoiceSynthesisError::InvalidOperatorLayout {
                voice: Ym2413RhythmVoice::HiHat,
            })
        );
    }

    #[test]
    fn operator_sustain_input_is_preserved_per_physical_channel() {
        let mut registers = configured_registers();
        registers[0x27] &= !0x20;
        let config = decode_rhythm_synthesis(&registers).unwrap();
        let RhythmVoiceSynthesisConfig {
            modulator, carrier, ..
        } = config.voices[1];
        assert!(!modulator.unwrap().envelope.sustain_on);
        assert!(!config.voices[2].carrier.unwrap().envelope.sustain_on);
        assert!(config.voices[0].modulator.unwrap().envelope.sustain_on);
        assert!(carrier.is_none());
    }
}
