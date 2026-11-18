//! Single-channel YM2413 operator signal path.
//!
//! This is the measured arithmetic model from the published 2017-01-26
//! reverse-engineering notes: one modulator with two-sample feedback followed
//! by one carrier. It does not own PG, EG evolution, slot timing or registers.

use crate::operator::{lookup_exp, lookup_log_sine};

/// Static and dynamic inputs consumed by one operator calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operator {
    /// Key-scale attenuation in the YM2413 envelope domain (0..=112).
    pub key_scale_level: u8,
    /// Current envelope attenuation (0..=127).
    pub envelope_level: u8,
    /// Apply the shared AM LFO attenuation to this operator.
    pub amplitude_modulation: bool,
    /// Select the positive half-wave instead of a full sine wave.
    pub half_wave: bool,
}

impl Operator {
    /// Construct an operator with validated attenuation state.
    pub const fn new(
        key_scale_level: u8,
        envelope_level: u8,
        amplitude_modulation: bool,
        half_wave: bool,
    ) -> Option<Self> {
        if key_scale_level > 112 || envelope_level > 127 {
            return None;
        }
        Some(Self {
            key_scale_level,
            envelope_level,
            amplitude_modulation,
            half_wave,
        })
    }

    /// Calculate one operator output using the YM2413 log-sine/EXP path.
    ///
    /// extra_level is expressed in envelope attenuation units. The AM input is
    /// already decoded to the 0..=13 attenuation level by the LFO owner.
    #[inline]
    pub fn calculate(&self, phase: u16, extra_level: u16, am_level: u8) -> i16 {
        // The measured stop condition ignores the two least significant EG bits.
        if (self.envelope_level & 0x7c) == 0x7c {
            return 0;
        }
        let sine = lookup_log_sine(phase, self.half_wave);
        let am = if self.amplitude_modulation {
            am_level
        } else {
            0
        };
        let attenuation = (extra_level
            + u16::from(self.key_scale_level)
            + u16::from(self.envelope_level)
            + u16::from(am))
        .min(127);
        lookup_exp(sine.wrapping_add(16 * attenuation))
    }
}

/// Modulator operator with the two previous outputs used by feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modulator {
    pub operator: Operator,
    /// Six-bit total-level field from the custom instrument patch.
    pub total_level: u8,
    /// Three-bit feedback field from the custom instrument patch.
    pub feedback: u8,
    previous_output: i16,
    previous_previous_output: i16,
}

impl Modulator {
    /// Construct a modulator after validating TL and FB field widths.
    pub const fn new(operator: Operator, total_level: u8, feedback: u8) -> Option<Self> {
        if total_level > 63 || feedback > 7 {
            return None;
        }
        Some(Self {
            operator,
            total_level,
            feedback,
            previous_output: 0,
            previous_previous_output: 0,
        })
    }

    /// Return the current feedback history as a pair of signed samples.
    pub const fn feedback_history(&self) -> (i16, i16) {
        (self.previous_output, self.previous_previous_output)
    }

    /// Clear both feedback delay elements.
    pub fn reset_feedback(&mut self) {
        self.previous_output = 0;
        self.previous_previous_output = 0;
    }

    /// Calculate the modulator sample and update its two-sample feedback line.
    #[inline]
    pub fn calculate(&mut self, phase: u16, am_level: u8) -> i16 {
        let feedback_phase = if self.feedback == 0 {
            0
        } else {
            (i32::from(self.previous_output) + i32::from(self.previous_previous_output))
                >> (8 - self.feedback)
        };
        let phase = wrap_phase(i32::from(phase) + feedback_phase);
        let output = self
            .operator
            .calculate(phase, u16::from(self.total_level) * 2, am_level)
            >> 1;
        self.previous_previous_output = self.previous_output;
        self.previous_output = output;
        output
    }
}

/// Carrier operator and channel output attenuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Carrier {
    pub operator: Operator,
    /// Four-bit channel volume field from register $30..$38.
    pub volume: u8,
}

impl Carrier {
    /// Construct a carrier after validating its four-bit volume field.
    pub const fn new(operator: Operator, volume: u8) -> Option<Self> {
        if volume > 15 {
            return None;
        }
        Some(Self { operator, volume })
    }

    /// Calculate the carrier output from the modulator's delayed output.
    #[inline]
    pub fn calculate(&self, phase: u16, modulator_output: i16, am_level: u8) -> i16 {
        let phase = wrap_phase(i32::from(phase) + 2 * i32::from(modulator_output));
        self.operator
            .calculate(phase, u16::from(self.volume) * 8, am_level)
            >> 4
    }
}

/// One melodic two-operator YM2413 channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MelodicChannel {
    pub modulator: Modulator,
    pub carrier: Carrier,
}

impl MelodicChannel {
    /// Construct a melodic channel from its two operator stages.
    pub const fn new(modulator: Modulator, carrier: Carrier) -> Self {
        Self { modulator, carrier }
    }

    /// Calculate one channel sample in the measured modulator/carrier order.
    ///
    /// The modulator phase is one phase-table step behind the carrier phase.
    #[inline]
    pub fn calculate(&mut self, phase: u16, am_level: u8) -> i16 {
        let modulator_phase = wrap_phase(i32::from(phase) - 1);
        let modulator_output = self.modulator.calculate(modulator_phase, am_level);
        self.carrier.calculate(phase, modulator_output, am_level)
    }

    /// Reset the channel's modulator feedback history.
    pub fn reset_feedback(&mut self) {
        self.modulator.reset_feedback();
    }
}

#[inline]
const fn wrap_phase(phase: i32) -> u16 {
    let remainder = phase % 1024;
    if remainder < 0 {
        (remainder + 1024) as u16
    } else {
        remainder as u16
    }
}

#[cfg(test)]
mod tests {
    use super::{Carrier, MelodicChannel, Modulator, Operator};

    fn operator(envelope_level: u8, half_wave: bool) -> Operator {
        Operator::new(0, envelope_level, false, half_wave).unwrap()
    }

    #[test]
    fn operator_rejects_out_of_range_envelope_and_key_scale() {
        assert!(Operator::new(113, 0, false, false).is_none());
        assert!(Operator::new(0, 128, false, false).is_none());
    }

    #[test]
    fn operator_stop_condition_mutes_all_terminal_envelope_levels() {
        for level in 124..=127 {
            assert_eq!(operator(level, false).calculate(0, 0, 0), 0);
        }
    }

    #[test]
    fn amplitude_modulation_is_applied_only_when_enabled() {
        let disabled = Operator::new(0, 0, false, false).unwrap();
        let enabled = Operator::new(0, 0, true, false).unwrap();
        assert_eq!(disabled.calculate(0, 0, 13), disabled.calculate(0, 0, 0));
        assert!(enabled.calculate(0, 0, 13) < enabled.calculate(0, 0, 0));
    }

    #[test]
    fn half_wave_operator_suppresses_the_negative_half_cycle() {
        let full = operator(0, false);
        let half = operator(0, true);
        assert_ne!(full.calculate(512, 0, 0), 0);
        assert!(half.calculate(512, 0, 0).abs() <= 1);
    }

    #[test]
    fn modulator_validates_patch_fields_and_shifts_feedback_history() {
        let op = operator(0, false);
        assert!(Modulator::new(op, 64, 0).is_none());
        assert!(Modulator::new(op, 0, 8).is_none());
        let mut modulator = Modulator::new(op, 0, 7).unwrap();
        assert_eq!(modulator.feedback_history(), (0, 0));
        let _ = modulator.calculate(128, 0);
        let (current, previous) = modulator.feedback_history();
        assert_eq!(previous, 0);
        let _ = modulator.calculate(128, 0);
        assert_eq!(modulator.feedback_history().1, current);
        modulator.reset_feedback();
        assert_eq!(modulator.feedback_history(), (0, 0));
    }

    #[test]
    fn carrier_and_channel_phase_wrap_at_ten_bits() {
        let modulator = Modulator::new(operator(0, false), 0, 0).unwrap();
        let carrier = Carrier::new(operator(0, false), 0).unwrap();
        let mut channel = MelodicChannel::new(modulator, carrier);
        assert_eq!(channel.calculate(0, 0), channel.calculate(1024, 0));
    }

    #[test]
    fn carrier_rejects_volume_outside_four_bits() {
        assert!(Carrier::new(operator(0, false), 16).is_none());
    }
}
