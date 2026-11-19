//! Integer Log-Sine / EXP transforms and a two-operator FM synthesis candidate.
//!
//! The transform follows the mathematical curves and signal arithmetic
//! described in the 2016-07-16 and 2017-01-26 reverse-engineering models.
//! Lookup tables are generated from those formulas in this repository.
//! This module does not own PG, EG, LFO, slots, clocks, or channel registers.

use crate::synthesis_tables::{YM2413_EXP_TABLE, YM2413_LOG_SINE_TABLE};

pub const YM2413_PHASE_INDEX_MASK: u16 = 0x03ff;
pub const YM2413_OPERATOR_PAIR_STATE_VERSION: u16 = 2;

/// Errors produced by the candidate operator synthesis path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorPairError {
    InvalidPhaseIndex(u16),
    InvalidFeedback(u8),
    InvalidTotalLevel(u8),
    InvalidVolume(u8),
    InvalidKsl(u8),
    InvalidEnvelope(u8),
    InvalidAm(u8),
    InvalidState,
}

/// Parameters supplied by the owning channel/register decoder.
///
/// KSL values are already converted to the EG attenuation domain (0..112).
/// The KSL lookup itself remains a separate channel-level responsibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatorPairParameters {
    pub feedback: u8,
    pub modulator_total_level: u8,
    pub modulator_ksl: u8,
    pub modulator_envelope: u8,
    pub modulator_am: u8,
    pub modulator_half_wave: bool,
    pub carrier_volume: u8,
    pub carrier_ksl: u8,
    pub carrier_envelope: u8,
    pub carrier_am: u8,
    pub carrier_half_wave: bool,
}

impl OperatorPairParameters {
    /// Validate the bit widths and attenuation ranges used by the OPLL model.
    pub const fn validate(self) -> Result<(), OperatorPairError> {
        if self.feedback > 7 {
            return Err(OperatorPairError::InvalidFeedback(self.feedback));
        }
        if self.modulator_total_level > 63 {
            return Err(OperatorPairError::InvalidTotalLevel(
                self.modulator_total_level,
            ));
        }
        if self.carrier_volume > 15 {
            return Err(OperatorPairError::InvalidVolume(self.carrier_volume));
        }
        if self.modulator_ksl > 112 {
            return Err(OperatorPairError::InvalidKsl(self.modulator_ksl));
        }
        if self.carrier_ksl > 112 {
            return Err(OperatorPairError::InvalidKsl(self.carrier_ksl));
        }
        if self.modulator_envelope > 127 {
            return Err(OperatorPairError::InvalidEnvelope(self.modulator_envelope));
        }
        if self.carrier_envelope > 127 {
            return Err(OperatorPairError::InvalidEnvelope(self.carrier_envelope));
        }
        if self.modulator_am > 13 {
            return Err(OperatorPairError::InvalidAm(self.modulator_am));
        }
        if self.carrier_am > 13 {
            return Err(OperatorPairError::InvalidAm(self.carrier_am));
        }
        Ok(())
    }

    /// Compute the modulator attenuation: min(127, 2*TL + KSL + EG + AM).
    pub const fn modulator_attenuation(self) -> u8 {
        let value = 2 * self.modulator_total_level as u16
            + self.modulator_ksl as u16
            + self.modulator_envelope as u16
            + self.modulator_am as u16;
        if value > 127 { 127 } else { value as u8 }
    }

    /// Compute the carrier attenuation: min(127, 8*VOL + KSL + EG + AM).
    pub const fn carrier_attenuation(self) -> u8 {
        let value = 8 * self.carrier_volume as u16
            + self.carrier_ksl as u16
            + self.carrier_envelope as u16
            + self.carrier_am as u16;
        if value > 127 { 127 } else { value as u8 }
    }
}

/// Parameters for evaluating one operator through the carrier output path.
///
/// Rhythm Mode's single-operator percussion voices use carrier-style output
/// scaling even when their physical serial slot is normally a modulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SingleOperatorCarrierParameters {
    /// Four-bit percussion volume attenuation field.
    pub volume: u8,
    /// KSL attenuation in EG units (0..112).
    pub ksl: u8,
    /// Current seven-bit EG attenuation.
    pub envelope: u8,
    /// Current AM-LFO attenuation (0..13), or zero when AM is disabled.
    pub am: u8,
    /// Whether the negative half-cycle is suppressed.
    pub half_wave: bool,
}

impl SingleOperatorCarrierParameters {
    /// Validate the carrier-path attenuation fields.
    pub const fn validate(self) -> Result<(), OperatorPairError> {
        if self.volume > 15 {
            return Err(OperatorPairError::InvalidVolume(self.volume));
        }
        if self.ksl > 112 {
            return Err(OperatorPairError::InvalidKsl(self.ksl));
        }
        if self.envelope > 127 {
            return Err(OperatorPairError::InvalidEnvelope(self.envelope));
        }
        if self.am > 13 {
            return Err(OperatorPairError::InvalidAm(self.am));
        }
        Ok(())
    }

    /// Compute carrier attenuation: min(127, 8*VOL + KSL + EG + AM).
    pub const fn attenuation(self) -> u8 {
        let value =
            8 * self.volume as u16 + self.ksl as u16 + self.envelope as u16 + self.am as u16;
        if value > 127 { 127 } else { value as u8 }
    }
}

/// Evaluate one standalone rhythm operator using the carrier output path.
///
/// The returned `i16` carries the candidate operator's one's-complement output
/// representation after carrier scaling; it is not a conventional signed
/// amplitude or a raw nine-bit DAC input word. In this representation, `-1`
/// preserves negative zero and `-256` is the negative-peak encoding after the
/// four-bit carrier shift. No physical DAC conversion is implied.
///
/// The caller supplies the ten-bit phase-table address and current attenuation
/// inputs. This function does not advance PG, EG, LFSR, feedback, slots, or
/// chip time. The carrier output scaling (exponential result divided by 16)
/// is intentional even when the operator occupies a physical modulator slot.
pub fn evaluate_single_operator_as_carrier(
    phase_index: u16,
    parameters: SingleOperatorCarrierParameters,
) -> Result<i16, OperatorPairError> {
    parameters.validate()?;
    if phase_index > YM2413_PHASE_INDEX_MASK {
        return Err(OperatorPairError::InvalidPhaseIndex(phase_index));
    }

    if parameters.envelope & 0x7c == 0x7c {
        return Ok(0);
    }

    let sine = log_sine_lookup(phase_index, parameters.half_wave)?;
    let encoded = sine.wrapping_add(u16::from(parameters.attenuation()) << 4);
    Ok(exponential_lookup(encoded) >> 4)
}

/// Return the 15-bit log-sine magnitude with sign in bit 15.
///
/// The phase index is ten bits: bit 9 is sign, bit 8 selects the mirrored
/// quarter-wave, and bits 0..7 select the generated table entry. When
/// half-wave mode is enabled, the negative half-cycle maps to maximum
/// attenuation (0x0fff) before the sign bit is applied.
pub fn log_sine_lookup(phase_index: u16, half_wave: bool) -> Result<u16, OperatorPairError> {
    if phase_index > YM2413_PHASE_INDEX_MASK {
        return Err(OperatorPairError::InvalidPhaseIndex(phase_index));
    }

    let sign = phase_index & 0x0200 != 0;
    let mirror = phase_index & 0x0100 != 0;
    let index = (phase_index & 0x00ff) as usize;
    let table_index = if mirror { index ^ 0x00ff } else { index };
    let mut result = YM2413_LOG_SINE_TABLE[table_index];

    if sign {
        if half_wave {
            result = 0x0fff;
        }
        result |= 0x8000;
    }
    Ok(result)
}

/// Convert sign/exponent/mantissa log-domain input to a one's-complement
/// encoded linear result, transported in an `i16`.
///
/// A set sign bit complements the positive magnitude, preserving distinct
/// positive-zero (`0`) and negative-zero (`-1`) encodings. Consequently, the
/// returned host integer must not be treated as a conventional signed
/// amplitude or as a raw DAC input code. Inputs outside the normal operator
/// attenuation range remain defined: shifts of 32 or more return zero
/// magnitude rather than panicking.
pub fn exponential_lookup(encoded: u16) -> i16 {
    let sign = encoded & 0x8000 != 0;
    let mantissa_index = ((encoded & 0x00ff) ^ 0x00ff) as usize;
    let exponent = u32::from((encoded & 0x7f00) >> 8);
    let base = (u32::from(YM2413_EXP_TABLE[mantissa_index]) << 1) | 0x0800;
    let magnitude = if exponent >= 32 {
        0
    } else {
        (base >> exponent) as i16
    };

    if sign { !magnitude } else { magnitude }
}

/// Versioned feedback history for one modulator/carrier channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatorPairState {
    pub format_version: u16,
    /// [p0, p1], the two previous modulator outputs in the operator's
    /// one's-complement representation, transported as i16 values.
    pub feedback_history: [i16; 2],
    /// Modulator phase-table index from the previous output-sample update.
    /// An absent value means the first evaluation uses its current phase input.
    pub previous_modulator_phase_index: Option<u16>,
}

/// Output of one modulator/carrier synthesis update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatorPairSample {
    /// Modulator result in the candidate one's-complement representation.
    pub modulator_output: i16,
    /// Carrier result in the candidate one's-complement representation.
    pub carrier_output: i16,
    /// Feedback phase offset applied to the modulator, in 10-bit phase units.
    pub feedback_phase_offset: i32,
    /// Modulator phase after feedback and 10-bit wrap.
    pub modulator_phase_used: u16,
    /// Carrier phase after adding twice the modulator output and wrapping.
    pub carrier_phase_used: u16,
}

/// One FM channel's operator-pair synthesis state.
///
/// Both operator phase indices are supplied independently. This deliberately
/// avoids assuming that modulator and carrier PG accumulators are aligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatorPair {
    feedback_history: [i16; 2],
    previous_modulator_phase_index: Option<u16>,
}

impl OperatorPair {
    /// Construct with explicit feedback history and no prior modulator phase.
    ///
    /// On the first evaluation, the current modulator phase is used as the
    /// startup history. Supply an explicit prior phase through
    /// with_initial_state when that condition is known.
    pub const fn with_feedback_history(feedback_history: [i16; 2]) -> Self {
        Self {
            feedback_history,
            previous_modulator_phase_index: None,
        }
    }

    /// Construct with explicit feedback history and optional prior-sample phase.
    pub fn with_initial_state(
        feedback_history: [i16; 2],
        previous_modulator_phase_index: Option<u16>,
    ) -> Result<Self, OperatorPairError> {
        if let Some(phase) = previous_modulator_phase_index
            && phase > YM2413_PHASE_INDEX_MASK
        {
            return Err(OperatorPairError::InvalidPhaseIndex(phase));
        }
        Ok(Self {
            feedback_history,
            previous_modulator_phase_index,
        })
    }

    /// Return the current feedback pipeline state.
    pub const fn feedback_history(&self) -> [i16; 2] {
        self.feedback_history
    }

    /// Return the prior-sample modulator phase, when initialized.
    pub const fn previous_modulator_phase_index(&self) -> Option<u16> {
        self.previous_modulator_phase_index
    }

    /// Evaluate one FM sample using independent modulator and carrier PG phases.
    ///
    /// The modulator phase used by this evaluation is the phase index saved from
    /// the previous call. The current modulator phase is saved for the next
    /// output-sample update. On the first call without initialized phase history,
    /// the current input phase is used; this is a startup fallback, not a claim
    /// about the physical reset state.
    ///
    /// All inputs are validated before feedback and phase histories are mutated.
    pub fn evaluate(
        &mut self,
        modulator_phase_index: u16,
        carrier_phase_index: u16,
        parameters: OperatorPairParameters,
    ) -> Result<OperatorPairSample, OperatorPairError> {
        parameters.validate()?;
        if modulator_phase_index > YM2413_PHASE_INDEX_MASK {
            return Err(OperatorPairError::InvalidPhaseIndex(modulator_phase_index));
        }
        if carrier_phase_index > YM2413_PHASE_INDEX_MASK {
            return Err(OperatorPairError::InvalidPhaseIndex(carrier_phase_index));
        }

        let feedback_phase_offset = if parameters.feedback == 0 {
            0
        } else {
            (i32::from(self.feedback_history[0]) + i32::from(self.feedback_history[1]))
                >> (8 - parameters.feedback)
        };
        // Andete's measured model uses the modulator phase from the previous
        // output-sample update. A fixed -1 table-index offset is not equivalent:
        // at FNUM=0x40/BLOCK=0/MUL=0, one table step spans 16 output samples.
        let delayed_modulator_phase = self
            .previous_modulator_phase_index
            .unwrap_or(modulator_phase_index);
        let modulator_phase_used =
            wrap_phase(i32::from(delayed_modulator_phase) + feedback_phase_offset);

        let modulator_output = if parameters.modulator_envelope & 0x7c == 0x7c {
            0
        } else {
            let sine = log_sine_lookup(modulator_phase_used, parameters.modulator_half_wave)?;
            let attenuation = parameters.modulator_attenuation();
            let encoded = sine.wrapping_add(u16::from(attenuation) << 4);
            exponential_lookup(encoded) >> 1
        };

        self.feedback_history[1] = self.feedback_history[0];
        self.feedback_history[0] = modulator_output;
        self.previous_modulator_phase_index = Some(modulator_phase_index);

        let carrier_phase_offset = 2 * i32::from(modulator_output);
        let carrier_phase_used = wrap_phase(i32::from(carrier_phase_index) + carrier_phase_offset);
        let carrier_output = evaluate_single_operator_as_carrier(
            carrier_phase_used,
            SingleOperatorCarrierParameters {
                volume: parameters.carrier_volume,
                ksl: parameters.carrier_ksl,
                envelope: parameters.carrier_envelope,
                am: parameters.carrier_am,
                half_wave: parameters.carrier_half_wave,
            },
        )?;

        Ok(OperatorPairSample {
            modulator_output,
            carrier_output,
            feedback_phase_offset,
            modulator_phase_used,
            carrier_phase_used,
        })
    }

    /// Export feedback history for an aggregate YM2413 snapshot.
    pub const fn export_state(&self) -> OperatorPairState {
        OperatorPairState {
            format_version: YM2413_OPERATOR_PAIR_STATE_VERSION,
            feedback_history: self.feedback_history,
            previous_modulator_phase_index: self.previous_modulator_phase_index,
        }
    }

    /// Restore feedback history after checking the snapshot version.
    pub const fn restore_state(state: OperatorPairState) -> Result<Self, OperatorPairError> {
        if state.format_version != YM2413_OPERATOR_PAIR_STATE_VERSION {
            return Err(OperatorPairError::InvalidState);
        }
        if let Some(phase) = state.previous_modulator_phase_index
            && phase > YM2413_PHASE_INDEX_MASK
        {
            return Err(OperatorPairError::InvalidState);
        }
        Ok(Self {
            feedback_history: state.feedback_history,
            previous_modulator_phase_index: state.previous_modulator_phase_index,
        })
    }
}

const fn wrap_phase(phase: i32) -> u16 {
    phase.rem_euclid(1024) as u16
}

#[cfg(test)]
mod tests {
    use super::{
        OperatorPair, OperatorPairError, OperatorPairParameters, OperatorPairState,
        SingleOperatorCarrierParameters, evaluate_single_operator_as_carrier, exponential_lookup,
        log_sine_lookup,
    };

    fn parameters() -> OperatorPairParameters {
        OperatorPairParameters {
            feedback: 0,
            modulator_total_level: 0,
            modulator_ksl: 0,
            modulator_envelope: 0,
            modulator_am: 0,
            modulator_half_wave: false,
            carrier_volume: 0,
            carrier_ksl: 0,
            carrier_envelope: 0,
            carrier_am: 0,
            carrier_half_wave: false,
        }
    }

    fn pair() -> OperatorPair {
        OperatorPair::with_feedback_history([0, 0])
    }

    #[test]
    fn log_sine_lookup_handles_quadrant_mirror_sign_and_half_wave() {
        assert_eq!(log_sine_lookup(0, false), Ok(2137));
        assert_eq!(log_sine_lookup(256, false), Ok(0));
        assert_eq!(log_sine_lookup(512, false), Ok(0x8000 | 2137));
        assert_eq!(log_sine_lookup(512, true), Ok(0x8fff));
        assert_eq!(
            log_sine_lookup(1024, false),
            Err(OperatorPairError::InvalidPhaseIndex(1024))
        );
    }

    #[test]
    fn exponential_lookup_uses_ones_complement_for_negative_values() {
        for value in [0x0000, 0x0123, 0x1234, 0x7fff] {
            assert_eq!(
                exponential_lookup(value | 0x8000),
                !exponential_lookup(value)
            );
        }
    }

    #[test]
    fn carrier_scaled_output_preserves_both_zero_encodings_and_negative_peak() {
        // The candidate output uses one's-complement encoding after the
        // carrier's four-bit scale reduction. Both zeros remain distinct.
        assert_eq!(exponential_lookup(0x0000) >> 4, 255);
        assert_eq!(exponential_lookup(0x8000) >> 4, -256);
        assert_eq!(exponential_lookup(0x7f00) >> 4, 0);
        assert_eq!(exponential_lookup(0xff00) >> 4, -1);
    }

    #[test]
    fn operator_candidate_boundary_reproduces_issue_1031_minus_256_at_phase_0x300() {
        // Issue #1031's isolated candidate setup: phase 0x300, zero
        // attenuation inputs, and the full sine wave. This proves the Rust
        // operator candidate can produce -256; it does not identify a
        // physical YM2413 DAC input word or approve a conversion.
        let output = evaluate_single_operator_as_carrier(
            0x300,
            SingleOperatorCarrierParameters {
                volume: 0,
                ksl: 0,
                envelope: 0,
                am: 0,
                half_wave: false,
            },
        )
        .unwrap();

        assert_eq!(output, -256);
    }

    #[test]
    fn exponential_lookup_defines_large_exponents_without_panicking() {
        assert_eq!(exponential_lookup(0x7f00), 0);
        assert_eq!(exponential_lookup(0xff00), -1);
    }

    #[test]
    fn attenuation_formulas_saturate_at_127() {
        let mut p = parameters();
        p.modulator_total_level = 63;
        p.modulator_ksl = 112;
        p.modulator_envelope = 127;
        p.modulator_am = 13;
        p.carrier_volume = 15;
        p.carrier_ksl = 112;
        p.carrier_envelope = 127;
        p.carrier_am = 13;
        assert_eq!(p.modulator_attenuation(), 127);
        assert_eq!(p.carrier_attenuation(), 127);
    }

    #[test]
    fn feedback_zero_disables_phase_feedback() {
        let mut pair = pair();
        let sample = pair.evaluate(0, 512, parameters()).unwrap();
        assert_eq!(sample.feedback_phase_offset, 0);
        assert_eq!(sample.modulator_phase_used, 0);
        assert_eq!(
            sample.carrier_phase_used,
            wrap_phase_for_test(512 + 2 * i32::from(sample.modulator_output))
        );
    }

    #[test]
    fn feedback_uses_two_previous_modulator_outputs() {
        let mut pair = OperatorPair::with_feedback_history([128, 128]);
        let mut p = parameters();
        p.feedback = 1;
        let sample = pair.evaluate(0, 0, p).unwrap();
        assert_eq!(sample.feedback_phase_offset, 2);
        assert_eq!(pair.feedback_history()[1], 128);
        assert_eq!(pair.feedback_history()[0], sample.modulator_output);
    }

    #[test]
    fn modulator_and_carrier_phase_inputs_are_independent() {
        let mut pair = pair();
        let sample = pair.evaluate(100, 700, parameters()).unwrap();
        assert_eq!(sample.modulator_phase_used, 100);
        assert_eq!(
            sample.carrier_phase_used,
            wrap_phase_for_test(700 + 2 * i32::from(sample.modulator_output))
        );
    }

    #[test]
    fn modulator_uses_previous_output_sample_phase_not_one_table_step() {
        let mut pair = OperatorPair::with_initial_state([0, 0], Some(0)).unwrap();
        let mut phase = 0_u32;
        let step = 32_u32; // FNUM=0x40, BLOCK=0, MUL=0: 1/16 table entry/sample.

        for sample in 0..18 {
            let current_index = (phase >> 9) as u16;
            let result = pair
                .evaluate(current_index, current_index, parameters())
                .unwrap();
            let expected_previous_index = if sample <= 16 { 0 } else { 1 };
            assert_eq!(result.modulator_phase_used, expected_previous_index);
            phase = (phase + step) & 0x7ffff;
        }
    }

    #[test]
    fn snapshot_round_trip_preserves_previous_modulator_phase() {
        let mut pair = OperatorPair::with_initial_state([0, 0], Some(0x155)).unwrap();
        pair.evaluate(0x166, 0x200, parameters()).unwrap();
        let restored = OperatorPair::restore_state(pair.export_state()).unwrap();
        assert_eq!(restored, pair);
        assert_eq!(restored.previous_modulator_phase_index(), Some(0x166));
    }

    #[test]
    fn envelope_silence_gate_outputs_zero_for_each_operator() {
        let mut pair = pair();
        let mut p = parameters();
        p.modulator_envelope = 124;
        p.carrier_envelope = 124;
        let sample = pair.evaluate(0, 0, p).unwrap();
        assert_eq!(sample.modulator_output, 0);
        assert_eq!(sample.carrier_output, 0);
    }

    #[test]
    fn single_operator_carrier_path_matches_pair_carrier_output() {
        let mut pair = OperatorPair::with_feedback_history([321, -123]);
        let mut pair_parameters = parameters();
        pair_parameters.modulator_envelope = 127;
        pair_parameters.carrier_volume = 3;
        pair_parameters.carrier_ksl = 12;
        pair_parameters.carrier_envelope = 25;
        pair_parameters.carrier_am = 4;
        pair_parameters.carrier_half_wave = true;

        let phase_index = 0x135;
        let pair_sample = pair.evaluate(0x222, phase_index, pair_parameters).unwrap();
        let single_output = evaluate_single_operator_as_carrier(
            phase_index,
            SingleOperatorCarrierParameters {
                volume: pair_parameters.carrier_volume,
                ksl: pair_parameters.carrier_ksl,
                envelope: pair_parameters.carrier_envelope,
                am: pair_parameters.carrier_am,
                half_wave: pair_parameters.carrier_half_wave,
            },
        )
        .unwrap();

        assert_eq!(pair_sample.modulator_output, 0);
        assert_eq!(pair_sample.carrier_phase_used, phase_index);
        assert_eq!(pair_sample.carrier_output, single_output);
    }

    #[test]
    fn single_operator_carrier_path_validates_inputs_and_silence_gate() {
        let parameters = SingleOperatorCarrierParameters {
            volume: 0,
            ksl: 0,
            envelope: 124,
            am: 0,
            half_wave: false,
        };
        assert_eq!(
            evaluate_single_operator_as_carrier(0x100, parameters),
            Ok(0)
        );
        assert_eq!(
            evaluate_single_operator_as_carrier(0x400, parameters),
            Err(OperatorPairError::InvalidPhaseIndex(0x400))
        );

        for invalid in [
            SingleOperatorCarrierParameters {
                volume: 16,
                ..parameters
            },
            SingleOperatorCarrierParameters {
                ksl: 113,
                ..parameters
            },
            SingleOperatorCarrierParameters {
                envelope: 128,
                ..parameters
            },
            SingleOperatorCarrierParameters {
                am: 14,
                ..parameters
            },
        ] {
            assert!(evaluate_single_operator_as_carrier(0, invalid).is_err());
        }
    }

    #[test]
    fn single_operator_carrier_attenuation_saturates_at_127() {
        let parameters = SingleOperatorCarrierParameters {
            volume: 15,
            ksl: 112,
            envelope: 127,
            am: 13,
            half_wave: false,
        };
        assert_eq!(parameters.attenuation(), 127);
    }

    #[test]
    fn initial_modulator_phase_history_rejects_out_of_range_index() {
        assert_eq!(
            OperatorPair::with_initial_state([0, 0], Some(1024)),
            Err(OperatorPairError::InvalidPhaseIndex(1024))
        );
    }

    #[test]
    fn invalid_parameters_and_phases_preserve_both_histories() {
        let mut pair = OperatorPair::with_initial_state([10, 20], Some(0x123)).unwrap();
        let before = pair;
        let mut invalid = parameters();
        invalid.feedback = 8;
        assert_eq!(
            pair.evaluate(0x124, 0x200, invalid),
            Err(OperatorPairError::InvalidFeedback(8))
        );
        assert_eq!(pair, before);
        assert_eq!(
            pair.evaluate(0x400, 0x200, parameters()),
            Err(OperatorPairError::InvalidPhaseIndex(0x400))
        );
        assert_eq!(pair, before);
    }

    #[test]
    fn snapshot_round_trip_preserves_feedback_history() {
        let pair = OperatorPair::with_feedback_history([123, -456]);
        let restored = OperatorPair::restore_state(pair.export_state()).unwrap();
        assert_eq!(restored, pair);
    }

    #[test]
    fn snapshot_rejects_unknown_version() {
        assert_eq!(
            OperatorPair::restore_state(OperatorPairState {
                format_version: 99,
                feedback_history: [0, 0],
                previous_modulator_phase_index: None,
            }),
            Err(OperatorPairError::InvalidState)
        );
    }

    #[test]
    fn invalid_parameters_and_phases_do_not_mutate_feedback_history() {
        let mut pair = OperatorPair::with_feedback_history([10, 20]);
        let before = pair;
        let mut p = parameters();
        p.feedback = 8;
        assert_eq!(
            pair.evaluate(0, 0, p),
            Err(OperatorPairError::InvalidFeedback(8))
        );
        assert_eq!(pair, before);
        assert_eq!(
            pair.evaluate(1024, 0, parameters()),
            Err(OperatorPairError::InvalidPhaseIndex(1024))
        );
        assert_eq!(pair, before);
        assert_eq!(
            pair.evaluate(0, 1024, parameters()),
            Err(OperatorPairError::InvalidPhaseIndex(1024))
        );
        assert_eq!(pair, before);
    }

    fn wrap_phase_for_test(phase: i32) -> u16 {
        phase.rem_euclid(1024) as u16
    }
}
