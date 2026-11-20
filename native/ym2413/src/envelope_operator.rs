//! Per-operator YM2413 EG update using externally sequenced hardware latches.
//!
//! This component owns only one operator's attenuation and four-state EG
//! machine. The chip-wide timer, 18-slot traversal, key-on/off pipelines,
//! register selection, and latch preparation remain caller-owned. This keeps
//! the operator update independently replayable against a cycle-level oracle.

use crate::envelope_timer::EnvelopeRatePhase;

pub const YM2413_ENVELOPE_OPERATOR_STATE_VERSION: u16 = 1;
pub const YM2413_ENVELOPE_OPERATOR_MAX_LEVEL: u8 = 0x7f;

/// The four states stored by the YM2413 serial EG state array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EnvelopeOperatorStage {
    Attack = 0,
    Decay = 1,
    Sustain = 2,
    Release = 3,
}

/// Pipeline controls consumed by one operator EG update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeControlPhase {
    /// Global rate and counter latches consumed by this slot.
    pub rate: EnvelopeRatePhase,
    /// Delayed KEY-ON bit consumed by the current operator update.
    pub key_on: bool,
    /// Delayed OFF bit consumed by the current operator update.
    pub off: bool,
    /// Delayed DAMP/KEY-ON completion bit consumed by the current update.
    pub dokon: bool,
    /// Sustain-level latch consumed by the current update (0..15).
    pub sustain_level: u8,
}

/// Complete serializable state of one hardware-state EG operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeOperatorState {
    pub format_version: u16,
    pub level: u8,
    pub stage: EnvelopeOperatorStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeOperatorError {
    InvalidLevel(u8),
    InvalidSustainLevel(u8),
    InvalidRatePhase,
    InvalidState,
}

/// Result of one operator EG update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeOperatorTick {
    pub level: u8,
    pub stage: EnvelopeOperatorStage,
    /// Signed increment applied to the pre-update level.
    pub step: i16,
}

/// One YM2413 operator's four-state envelope machine.
///
/// This type does not calculate rates or synthesize DAMP/KEY-ON/OFF pipelines.
/// Those are shared serial-chip responsibilities and are supplied through
/// EnvelopeControlPhase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeOperator {
    level: u8,
    stage: EnvelopeOperatorStage,
}

impl EnvelopeOperator {
    /// Construct from explicit reset or replay state.
    pub const fn with_initial_state(
        level: u8,
        stage: EnvelopeOperatorStage,
    ) -> Result<Self, EnvelopeOperatorError> {
        if level > YM2413_ENVELOPE_OPERATOR_MAX_LEVEL {
            return Err(EnvelopeOperatorError::InvalidLevel(level));
        }
        Ok(Self { level, stage })
    }

    /// Current seven-bit attenuation (0 is loudest, 127 is silent).
    pub const fn level(&self) -> u8 {
        self.level
    }

    /// Current serial EG state.
    pub const fn stage(&self) -> EnvelopeOperatorStage {
        self.stage
    }

    /// Apply one operator update using the latches consumed on this slot.
    ///
    /// The pre-update level/state drive the comparators. The OFF and DOKON
    /// pipelines may override the next level before the stage-specific step;
    /// KEY-ON and DOKON then select the next state in that order.
    pub fn tick(
        &mut self,
        control: EnvelopeControlPhase,
    ) -> Result<EnvelopeOperatorTick, EnvelopeOperatorError> {
        if control.sustain_level > 15 {
            return Err(EnvelopeOperatorError::InvalidSustainLevel(
                control.sustain_level,
            ));
        }
        if control.rate.rate_hi > 15
            || control.rate.max_rate != (control.rate.rate_hi == 15)
        {
            return Err(EnvelopeOperatorError::InvalidRatePhase);
        }

        let level_before = self.level;
        let stage_before = self.stage;
        let zero = level_before == 0;
        let mut next_level = i32::from(level_before);

        if stage_before != EnvelopeOperatorStage::Attack && control.off && !control.dokon {
            next_level = i32::from(YM2413_ENVELOPE_OPERATOR_MAX_LEVEL);
        }
        if control.rate.max_rate && control.dokon {
            next_level = 0;
        }

        let mut step = 0_i32;
        let mut next_stage = match stage_before {
            EnvelopeOperatorStage::Attack => {
                if !control.rate.max_rate && control.key_on && !zero {
                    let shift = if control.rate.rate_hi < 12 {
                        u8::from(control.rate.inc_lo)
                    } else {
                        control.rate.rate_hi - 11 + u8::from(control.rate.inc_hi)
                    };
                    if shift > 0 {
                        step = (!i32::from(level_before)) >> (5 - u32::from(shift));
                    }
                }
                if zero {
                    EnvelopeOperatorStage::Decay
                } else {
                    EnvelopeOperatorStage::Attack
                }
            }
            EnvelopeOperatorStage::Decay => {
                if !control.off
                    && !control.dokon
                    && (level_before >> 3) != control.sustain_level
                {
                    step = i32::from(decay_increment(control.rate));
                }
                if (level_before >> 3) == control.sustain_level {
                    EnvelopeOperatorStage::Sustain
                } else {
                    EnvelopeOperatorStage::Decay
                }
            }
            EnvelopeOperatorStage::Sustain | EnvelopeOperatorStage::Release => {
                if !control.off && !control.dokon {
                    step = i32::from(decay_increment(control.rate));
                }
                stage_before
            }
        };

        if !control.key_on {
            next_stage = EnvelopeOperatorStage::Release;
        }
        if control.dokon {
            next_stage = EnvelopeOperatorStage::Attack;
        }

        // The reference stores the result in an eight-bit EG array. Preserve
        // byte-level behavior rather than silently clamping the arithmetic.
        self.level = (next_level + step) as u8;
        self.stage = next_stage;

        Ok(EnvelopeOperatorTick {
            level: self.level,
            stage: self.stage,
            step: step as i16,
        })
    }

    /// Export the complete operator state needed for deterministic resume.
    pub const fn export_state(&self) -> EnvelopeOperatorState {
        EnvelopeOperatorState {
            format_version: YM2413_ENVELOPE_OPERATOR_STATE_VERSION,
            level: self.level,
            stage: self.stage,
        }
    }

    /// Restore an operator snapshot after validating its version and level.
    pub const fn restore_state(
        state: EnvelopeOperatorState,
    ) -> Result<Self, EnvelopeOperatorError> {
        if state.format_version != YM2413_ENVELOPE_OPERATOR_STATE_VERSION
            || state.level > YM2413_ENVELOPE_OPERATOR_MAX_LEVEL
        {
            return Err(EnvelopeOperatorError::InvalidState);
        }
        Ok(Self {
            level: state.level,
            stage: state.stage,
        })
    }
}

/// Reconstruct the two-bit DECAY/SUSTAIN/RELEASE increment from consumed latches.
fn decay_increment(rate: EnvelopeRatePhase) -> u8 {
    let rate_hi = rate.rate_hi;
    let inc_hi = rate.inc_hi;
    let inc_lo = rate.inc_lo;
    let counter = rate.counter_state_prev;

    let i0 = rate_hi == 15 || (rate_hi == 14 && inc_hi);
    let i1 = (rate_hi == 14 && !inc_hi)
        || (rate_hi == 13 && inc_hi)
        || (rate_hi == 13 && !inc_hi && counter & 1 != 0)
        || (rate_hi == 12 && inc_hi && counter & 1 != 0)
        || (rate_hi == 12 && !inc_hi && counter & 3 == 3)
        || (inc_lo && counter & 3 == 3);
    (u8::from(i0) << 1) | u8::from(i1)
}

#[cfg(test)]
mod tests {
    use super::{
        EnvelopeControlPhase, EnvelopeOperator, EnvelopeOperatorError, EnvelopeOperatorStage,
    };
    use crate::envelope_timer::EnvelopeRatePhase;

    fn control(
        rate_hi: u8,
        inc_lo: bool,
        counter_state_prev: u8,
        key_on: bool,
        off: bool,
        dokon: bool,
        sustain_level: u8,
    ) -> EnvelopeControlPhase {
        EnvelopeControlPhase {
            rate: EnvelopeRatePhase::from_consumed_latches(
                rate_hi,
                inc_lo,
                false,
                rate_hi == 15,
                counter_state_prev,
            ),
            key_on,
            off,
            dokon,
            sustain_level,
        }
    }

    #[test]
    fn attack_uses_consumed_rate_latches_and_pre_update_zero_test() {
        let mut operator =
            EnvelopeOperator::with_initial_state(1, EnvelopeOperatorStage::Attack).unwrap();
        let result = operator
            .tick(control(11, true, 3, true, false, false, 4))
            .unwrap();
        assert_eq!(result.level, 0);
        assert_eq!(result.stage, EnvelopeOperatorStage::Attack);

        let next = operator
            .tick(control(11, true, false, false, 3, true, false, false, 4))
            .unwrap();
        assert_eq!(next.stage, EnvelopeOperatorStage::Decay);
    }

    #[test]
    fn decay_to_sustain_uses_pre_update_level_and_defers_sustain_step() {
        let mut operator =
            EnvelopeOperator::with_initial_state(31, EnvelopeOperatorStage::Decay).unwrap();
        let result = operator
            .tick(control(15, false, 3, true, false, false, 4))
            .unwrap();
        assert_eq!(result.level, 33);
        assert_eq!(result.stage, EnvelopeOperatorStage::Decay);

        let next = operator
            .tick(control(15, false, false, true, 3, true, false, false, 4))
            .unwrap();
        assert_eq!(next.level, 33);
        assert_eq!(next.stage, EnvelopeOperatorStage::Sustain);
    }

    #[test]
    fn key_off_pipeline_selects_release_and_dokon_has_final_priority() {
        let mut operator =
            EnvelopeOperator::with_initial_state(20, EnvelopeOperatorStage::Decay).unwrap();
        let release = operator
            .tick(control(0, false, 0, false, false, false, 4))
            .unwrap();
        assert_eq!(release.stage, EnvelopeOperatorStage::Release);

        let key_on_completion = operator
            .tick(control(15, false, 0, false, true, true, 4))
            .unwrap();
        assert_eq!(key_on_completion.level, 0);
        assert_eq!(key_on_completion.stage, EnvelopeOperatorStage::Attack);
    }

    #[test]
    fn off_pipeline_forces_silence_without_overriding_attack_stage() {
        let mut operator =
            EnvelopeOperator::with_initial_state(12, EnvelopeOperatorStage::Release).unwrap();
        let result = operator
            .tick(control(0, false, 0, false, true, false, 4))
            .unwrap();
        assert_eq!(result.level, 127);
        assert_eq!(result.stage, EnvelopeOperatorStage::Release);
    }

    #[test]
    fn invalid_rate_and_sustain_latches_are_rejected_without_mutation() {
        let mut operator =
            EnvelopeOperator::with_initial_state(24, EnvelopeOperatorStage::Decay).unwrap();
        assert_eq!(
            operator.tick(control(15, false, 0, true, false, false, 4)),
            Err(EnvelopeOperatorError::InvalidRatePhase)
        );
        assert_eq!(
            operator.tick(control(0, false, 0, true, false, false, 16)),
            Err(EnvelopeOperatorError::InvalidSustainLevel(16))
        );
        assert_eq!(operator.level(), 24);
        assert_eq!(operator.stage(), EnvelopeOperatorStage::Decay);
    }

    #[test]
    fn state_round_trip_preserves_release_and_level() {
        let operator =
            EnvelopeOperator::with_initial_state(87, EnvelopeOperatorStage::Release).unwrap();
        let restored = EnvelopeOperator::restore_state(operator.export_state()).unwrap();
        assert_eq!(restored, operator);
    }
}
