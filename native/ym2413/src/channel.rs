//! Register-to-synthesis parameter bridge for the nine melodic YM2413 channels.
//!
//! This module connects the existing register, instrument, KSL, PG, EG, AM,
//! and operator-pair components without owning time or advancing any hardware.
//! It deliberately does not define slot ordering, reset alignment, or PCM
//! scheduling; those contracts belong to the later aggregate chip pipeline.

use crate::envelope::EnvelopeParameters;
use crate::envelope_sequencer::EnvelopeSlotInput;
use crate::instrument::{
    ChannelRegisterState, InstrumentError, InstrumentPatch, OperatorPatch,
    decode_channel_registers, decode_user_patch, resolve_instrument, rhythm_mode_enabled,
};
use crate::key_scale::{KeyScaleError, key_scale_attenuation};
use crate::operator::{OperatorPairError, OperatorPairParameters};
use crate::phase::{PhaseGenerator, PhaseGeneratorError};

/// Errors produced while resolving one channel's register-derived parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelSynthesisError {
    /// The channel index or selected instrument code is not valid.
    Instrument(InstrumentError),
    /// The decoded BLOCK/FNUM/KSL tuple could not be represented.
    KeyScale(KeyScaleError),
    /// Rhythm Mode owns channels 7–9; they are not melodic channels in this mode.
    RhythmModeActive { channel: u8 },
    /// The serial EG cycle index must be in 0..18.
    InvalidEnvelopeCycle { cycle: u8 },
}

impl From<InstrumentError> for ChannelSynthesisError {
    fn from(value: InstrumentError) -> Self {
        Self::Instrument(value)
    }
}

impl From<KeyScaleError> for ChannelSynthesisError {
    fn from(value: KeyScaleError) -> Self {
        Self::KeyScale(value)
    }
}

/// Static PG inputs for one operator, derived from its channel and patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatorPhaseParameters {
    /// Nine-bit channel frequency number.
    pub fnum: u16,
    /// Three-bit octave/block field.
    pub block: u8,
    /// Four-bit operator multiplier field.
    pub multiplier: u8,
    /// Whether this operator enables phase modulation (vibrato).
    pub pm_enabled: bool,
}

/// Resolved, time-independent synthesis configuration for one melodic channel.
///
/// Envelope levels, AM level, PG accumulators, and feedback history remain
/// dynamic state and are supplied by their owning components at update time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelSynthesisConfig {
    /// Decoded channel register triplet ($10+n, $20+n, $30+n).
    pub channel: ChannelRegisterState,
    /// Resolved user patch or candidate preset.
    pub patch: InstrumentPatch,
    /// PG inputs for the modulator.
    pub modulator_phase: OperatorPhaseParameters,
    /// PG inputs for the carrier.
    pub carrier_phase: OperatorPhaseParameters,
    /// Register-derived EG inputs for the modulator.
    pub modulator_envelope: EnvelopeParameters,
    /// Register-derived EG inputs for the carrier.
    pub carrier_envelope: EnvelopeParameters,
    /// KSL attenuation for the modulator in EG units.
    pub modulator_ksl: u8,
    /// KSL attenuation for the carrier in EG units.
    pub carrier_ksl: u8,
}

impl ChannelSynthesisConfig {
    /// Build the current-cycle EG input for this channel and operator role.
    ///
    /// sustain_level is the fresh c_sl latch; EnvelopeSequencer owns its
    /// one-cycle transfer into eg_sl. The channel's KON and SUS bits are
    /// copied separately from the selected operator's patch fields.
    pub fn envelope_slot_input(&self, carrier: bool) -> EnvelopeSlotInput {
        let envelope = if carrier {
            self.carrier_envelope
        } else {
            self.modulator_envelope
        };
        EnvelopeSlotInput {
            attack_rate: envelope.attack_rate,
            decay_rate: envelope.decay_rate,
            release_rate: envelope.release_rate,
            sustain_level: envelope.sustain_level,
            key_scale_rate: envelope.key_scale_rate,
            ksr_frequency: ((envelope.block << 1) | (envelope.fnum >> 8) as u8) & 0x0f,
            sustain_on: self.channel.sustain_on,
            sustained_tone: envelope.sustained_tone,
            key_on: self.channel.key_on,
        }
    }

    /// Calculate one operator's candidate phase increment for a PM table position.
    ///
    /// The caller still owns the PG accumulator and decides the exact slot
    /// boundary. A disabled PM input intentionally ignores the supplied
    /// position, as the standalone PhaseGenerator contract specifies.
    pub fn phase_step(&self, carrier: bool, pm_position: u8) -> Result<u32, PhaseGeneratorError> {
        let operator = if carrier {
            self.carrier_phase
        } else {
            self.modulator_phase
        };
        PhaseGenerator::phase_step(
            operator.fnum,
            operator.block,
            operator.multiplier,
            operator.pm_enabled,
            pm_position,
        )
    }

    /// Build the dynamic inputs consumed by the two-operator FM evaluator.
    ///
    /// The envelope arguments are current EG attenuation values; am_level is
    /// the current global AM-LFO table value (0..13). AM is applied
    /// independently according to each operator's AM-enable bit.
    pub fn operator_pair_parameters(
        &self,
        modulator_envelope: u8,
        carrier_envelope: u8,
        am_level: u8,
    ) -> Result<OperatorPairParameters, OperatorPairError> {
        let parameters = OperatorPairParameters {
            feedback: self.patch.feedback,
            modulator_total_level: self.patch.modulator.total_level,
            modulator_ksl: self.modulator_ksl,
            modulator_envelope,
            modulator_am: if self.patch.modulator.am_enabled {
                am_level
            } else {
                0
            },
            modulator_half_wave: self.patch.modulator.half_wave,
            carrier_volume: self.channel.volume,
            carrier_ksl: self.carrier_ksl,
            carrier_envelope,
            carrier_am: if self.patch.carrier.am_enabled {
                am_level
            } else {
                0
            },
            carrier_half_wave: self.patch.carrier.half_wave,
        };
        parameters.validate()?;
        Ok(parameters)
    }
}

/// Convert the decoded operator patch into its candidate EG register inputs.
fn envelope_parameters(
    channel: ChannelRegisterState,
    operator: OperatorPatch,
) -> EnvelopeParameters {
    EnvelopeParameters {
        attack_rate: operator.attack_rate,
        decay_rate: operator.decay_rate,
        release_rate: operator.release_rate,
        sustain_level: operator.sustain_level,
        key_scale_rate: operator.key_scale_rate,
        fnum: channel.fnum,
        block: channel.block,
        sustain_on: channel.sustain_on,
        sustained_tone: operator.sustained_envelope,
    }
}

/// Convert the decoded operator patch into its candidate PG register inputs.
fn phase_parameters(
    channel: ChannelRegisterState,
    operator: OperatorPatch,
) -> OperatorPhaseParameters {
    OperatorPhaseParameters {
        fnum: channel.fnum,
        block: channel.block,
        multiplier: operator.multiplier,
        pm_enabled: operator.vibrato_enabled,
    }
}

/// Nuked-OPLL channel selected by each of the 18 serial EG cycles.
pub const YM2413_EG_CYCLE_CHANNELS: [u8; 18] =
    [1, 2, 0, 1, 2, 3, 4, 5, 3, 4, 5, 6, 7, 8, 6, 7, 8, 0];

/// Return whether a serial EG cycle selects the carrier operator.
pub const fn envelope_cycle_is_carrier(cycle: u8) -> Option<bool> {
    if cycle >= 18 {
        None
    } else {
        Some(((cycle + 1) / 3) & 1 != 0)
    }
}

/// Resolve the current cycle's channel/operator register inputs.
///
/// The caller supplies all nine decoded channel configurations. The serial
/// channel map and modulator/carrier cadence are kept here so every runtime
/// host uses the same register-to-EG selection contract.
pub fn envelope_slot_input_for_cycle(
    channels: &[ChannelSynthesisConfig; 9],
    cycle: u8,
) -> Result<EnvelopeSlotInput, ChannelSynthesisError> {
    let channel = *YM2413_EG_CYCLE_CHANNELS
        .get(usize::from(cycle))
        .ok_or(ChannelSynthesisError::InvalidEnvelopeCycle { cycle })?;
    let carrier = envelope_cycle_is_carrier(cycle)
        .ok_or(ChannelSynthesisError::InvalidEnvelopeCycle { cycle })?;
    Ok(channels[usize::from(channel)].envelope_slot_input(carrier))
}

/// Resolve one melodic channel's patch and all static synthesis parameters.
///
/// Instrument code 0 selects the live user patch in registers $00-$07.
/// Codes 1..15 select the repository's explicitly candidate preset bank.
/// Channels 7..9 are rejected while Rhythm Mode is active because those
/// channel resources are assigned to rhythm voices rather than melodic FM.
pub fn decode_channel_synthesis(
    registers: &[u8; 64],
    channel: u8,
) -> Result<ChannelSynthesisConfig, ChannelSynthesisError> {
    let channel_registers = decode_channel_registers(registers, channel)?;
    if channel >= 6 && rhythm_mode_enabled(registers) {
        return Err(ChannelSynthesisError::RhythmModeActive { channel });
    }
    let user_patch = decode_user_patch(registers);
    let patch = resolve_instrument(channel_registers.instrument, user_patch)?;

    let modulator_ksl = key_scale_attenuation(
        channel_registers.block,
        channel_registers.fnum,
        patch.modulator.key_scale_level,
    )?;
    let carrier_ksl = key_scale_attenuation(
        channel_registers.block,
        channel_registers.fnum,
        patch.carrier.key_scale_level,
    )?;

    Ok(ChannelSynthesisConfig {
        channel: channel_registers,
        patch,
        modulator_phase: phase_parameters(channel_registers, patch.modulator),
        carrier_phase: phase_parameters(channel_registers, patch.carrier),
        modulator_envelope: envelope_parameters(channel_registers, patch.modulator),
        carrier_envelope: envelope_parameters(channel_registers, patch.carrier),
        modulator_ksl,
        carrier_ksl,
    })
}

#[cfg(test)]
mod tests {
    use super::{ChannelSynthesisError, decode_channel_synthesis};
    use crate::instrument::{InstrumentError, decode_patch_registers, resolve_instrument};
    use crate::key_scale::key_scale_attenuation;
    use crate::operator::OperatorPairError;
    use crate::phase::{PhaseGenerator, PhaseGeneratorError};

    fn registers_for_channel(channel: u8, instrument: u8) -> [u8; 64] {
        let mut registers = [0; 64];
        registers[..8].copy_from_slice(&[0xf2, 0xb5, 0xaf, 0xdb, 0xa5, 0x3c, 0x6d, 0xf2]);
        let index = usize::from(channel);
        registers[0x10 + index] = 0xa5;
        registers[0x20 + index] = 0x35;
        registers[0x30 + index] = (instrument << 4) | 0x0b;
        registers
    }

    #[test]
    fn resolves_user_patch_and_channel_registers_into_synthesis_inputs() {
        let registers = registers_for_channel(2, 0);
        let config = decode_channel_synthesis(&registers, 2).unwrap();

        assert_eq!(config.channel.channel, 2);
        assert_eq!(config.channel.fnum, 0x1a5);
        assert_eq!(config.channel.block, 2);
        assert!(config.channel.key_on);
        assert!(config.channel.sustain_on);
        assert_eq!(config.channel.volume, 0x0b);
        assert_eq!(
            config.patch,
            decode_patch_registers([0xf2, 0xb5, 0xaf, 0xdb, 0xa5, 0x3c, 0x6d, 0xf2])
        );
        assert_eq!(config.modulator_phase.multiplier, 2);
        assert_eq!(config.carrier_phase.multiplier, 5);
        assert!(config.modulator_phase.pm_enabled);
        assert!(!config.carrier_phase.pm_enabled);
        assert_eq!(config.modulator_envelope.attack_rate, 10);
        assert_eq!(config.carrier_envelope.attack_rate, 3);
        assert_eq!(
            config.modulator_ksl,
            key_scale_attenuation(2, 0x1a5, 2).unwrap()
        );
        assert_eq!(
            config.carrier_ksl,
            key_scale_attenuation(2, 0x1a5, 3).unwrap()
        );
    }

    #[test]
    fn resolves_each_candidate_preset_for_all_nine_melodic_channels() {
        for channel in 0..9 {
            for instrument in 1..=15 {
                let registers = registers_for_channel(channel, instrument);
                let config = decode_channel_synthesis(&registers, channel).unwrap();
                assert_eq!(config.channel.channel, channel);
                assert_eq!(config.channel.instrument, instrument);
            }
        }
    }

    #[test]
    fn invalid_channel_and_instrument_codes_are_reported() {
        assert_eq!(
            decode_channel_synthesis(&[0; 64], 9),
            Err(ChannelSynthesisError::Instrument(
                InstrumentError::InvalidChannel(9)
            ))
        );

        let user_patch = decode_patch_registers([0; 8]);
        assert_eq!(
            resolve_instrument(16, user_patch),
            Err(InstrumentError::InvalidPreset(16))
        );
    }

    #[test]
    fn rhythm_mode_rejects_melodic_resolution_for_channels_seven_to_nine() {
        let mut registers = registers_for_channel(6, 1);
        registers[0x0e] = 0x20;
        for channel in 6..=8 {
            assert_eq!(
                decode_channel_synthesis(&registers, channel),
                Err(ChannelSynthesisError::RhythmModeActive { channel })
            );
        }
        assert!(decode_channel_synthesis(&registers, 5).is_ok());
    }

    #[test]
    fn operator_pair_parameters_apply_am_enable_per_operator() {
        let mut registers = registers_for_channel(0, 0);
        registers[0] &= !0x80;
        registers[1] |= 0x80;
        let config = decode_channel_synthesis(&registers, 0).unwrap();

        let parameters = config.operator_pair_parameters(11, 17, 13).unwrap();
        assert_eq!(parameters.modulator_am, 0);
        assert_eq!(parameters.carrier_am, 13);
        assert_eq!(parameters.modulator_envelope, 11);
        assert_eq!(parameters.carrier_envelope, 17);
        assert_eq!(parameters.carrier_volume, 0x0b);
    }

    #[test]
    fn operator_pair_parameter_builder_rejects_invalid_dynamic_levels() {
        let config = decode_channel_synthesis(&registers_for_channel(0, 0), 0).unwrap();
        assert_eq!(
            config.operator_pair_parameters(128, 0, 0),
            Err(OperatorPairError::InvalidEnvelope(128))
        );
        assert_eq!(
            config.operator_pair_parameters(0, 0, 14),
            Err(OperatorPairError::InvalidAm(14))
        );
    }

    #[test]
    fn phase_step_uses_operator_multiplier_and_pm_enable() {
        let config = decode_channel_synthesis(&registers_for_channel(0, 0), 0).unwrap();

        assert_eq!(
            config.phase_step(false, 0),
            Ok(PhaseGenerator::phase_step(0x1a5, 2, 2, true, 0).unwrap())
        );
        assert_eq!(
            config.phase_step(true, u8::MAX),
            Ok(PhaseGenerator::phase_step(0x1a5, 2, 5, false, 0).unwrap())
        );
        assert_eq!(
            config.phase_step(false, 8),
            Err(PhaseGeneratorError::InvalidPmPosition(8))
        );
    }
}
