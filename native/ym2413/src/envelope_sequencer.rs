//! Chip-wide YM2413 serial EG sequencer.
//!
//! This component owns the 18 operator EG states, shared timer, and delayed
//! KEY-ON/OFF/DOKON/SL latches. Register decoding and the caller's master-clock
//! scheduler remain external: one selected channel/operator input is supplied
//! for each serial cycle. No wall-clock or audio callback advances this state.

use crate::envelope_operator::{
    EnvelopeControlPhase, EnvelopeOperator, EnvelopeOperatorError, EnvelopeOperatorStage,
    EnvelopeOperatorState, EnvelopeOperatorTick,
};
use crate::envelope_timer::{
    EnvelopeTimer, EnvelopeTimerError, EnvelopeTimerSnapshot, EnvelopeTimerState,
};

pub const YM2413_ENVELOPE_SEQUENCER_STATE_VERSION: u16 = 1;
pub const YM2413_ENVELOPE_SEQUENCER_SLOTS: usize = 18;

/// Register-derived inputs selected for the current serial cycle.
///
/// `sustain_level` is the previously prepared c_sl latch transferred into
/// eg_sl by this cycle; it is intentionally one call behind the freshly
/// decoded c_sl value visible after the current OPLL clock call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EnvelopeSlotInput {
    pub attack_rate: u8,
    pub decay_rate: u8,
    pub release_rate: u8,
    pub sustain_level: u8,
    pub key_scale_rate: bool,
    /// Four-bit channel key code: (BLOCK << 1) | FNUM[8].
    pub ksr_frequency: u8,
    pub sustain_on: bool,
    pub sustained_tone: bool,
    /// Raw channel KON bit appended to the delayed EG key-on pipeline.
    pub key_on: bool,
}

/// Complete serializable state of the 18-slot EG sequencer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeSequencerState {
    pub format_version: u16,
    pub operators: [EnvelopeOperatorState; YM2413_ENVELOPE_SEQUENCER_SLOTS],
    pub timer: EnvelopeTimerState,
    pub eg_kon: u8,
    pub eg_off: u8,
    pub eg_dokon: u32,
    pub eg_sl: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeSequencerError {
    InvalidSlot(u8),
    InvalidInput,
    InvalidState,
    Timer(EnvelopeTimerError),
    Operator {
        slot: u8,
        source: EnvelopeOperatorError,
    },
}

impl core::fmt::Display for EnvelopeSequencerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSlot(slot) => write!(f, "invalid YM2413 EG cycle {slot}"),
            Self::InvalidInput => f.write_str("invalid YM2413 EG slot input"),
            Self::InvalidState => f.write_str("invalid YM2413 EG sequencer state"),
            Self::Timer(error) => write!(f, "YM2413 EG timer error: {error}"),
            Self::Operator { slot, source } => {
                write!(f, "YM2413 EG operator slot {slot} error: {source:?}")
            }
        }
    }
}

impl std::error::Error for EnvelopeSequencerError {}

impl From<EnvelopeTimerError> for EnvelopeSequencerError {
    fn from(value: EnvelopeTimerError) -> Self {
        Self::Timer(value)
    }
}

/// Result of servicing one serial cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeSequencerTick {
    pub cycle: u8,
    /// EG array entry updated by this serial cycle: (cycle + 16) % 18.
    pub operator_slot: u8,
    pub operator: EnvelopeOperatorTick,
    pub timer: EnvelopeTimerSnapshot,
    /// Newly shifted pipeline values after this cycle.
    pub eg_kon: u8,
    pub eg_off: u8,
    pub eg_dokon: u32,
    pub eg_sl: u8,
    /// Composed six-bit rate prepared for the following serial cycle.
    pub next_rate: u8,
    /// Base-rate-zero latch prepared independently of KSR.
    pub next_zero_rate: bool,
}

/// Deterministic 18-slot EG sequencer with explicit pipeline state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeSequencer {
    operators: [EnvelopeOperator; YM2413_ENVELOPE_SEQUENCER_SLOTS],
    timer: EnvelopeTimer,
    eg_kon: u8,
    eg_off: u8,
    eg_dokon: u32,
    eg_sl: u8,
}

impl Default for EnvelopeSequencer {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvelopeSequencer {
    /// Construct the Nuked reset state: silent RELEASE operators and cleared
    /// delayed control pipelines.
    pub fn new() -> Self {
        let initial = EnvelopeOperator::with_initial_state(127, EnvelopeOperatorStage::Release)
            .expect("the YM2413 EG reset state is valid");
        Self {
            operators: [initial; YM2413_ENVELOPE_SEQUENCER_SLOTS],
            timer: EnvelopeTimer::new(),
            eg_kon: 0,
            eg_off: 0,
            eg_dokon: 0,
            eg_sl: 0,
        }
    }

    /// Return one operator's current attenuation and four-state EG phase.
    pub fn operator(&self, slot: u8) -> Option<(u8, EnvelopeOperatorStage)> {
        self.operators
            .get(usize::from(slot))
            .map(|operator| (operator.level(), operator.stage()))
    }

    /// Return the current shared timer snapshot.
    pub const fn timer_state(&self) -> EnvelopeTimerState {
        self.timer.export_state()
    }

    /// Service one serial cycle using the current channel/operator register latches.
    ///
    /// The EG array update occurs at (cycle + 16) % 18. Rate selection uses
    /// the pre-update state at array index cycle and the newly shifted control
    /// pipeline, matching the two-slot relationship in the serial reference.
    pub fn tick_slot(
        &mut self,
        cycle: u8,
        input: EnvelopeSlotInput,
    ) -> Result<EnvelopeSequencerTick, EnvelopeSequencerError> {
        if cycle >= YM2413_ENVELOPE_SEQUENCER_SLOTS as u8 {
            return Err(EnvelopeSequencerError::InvalidSlot(cycle));
        }
        if input.attack_rate > 15
            || input.decay_rate > 15
            || input.release_rate > 15
            || input.sustain_level > 15
            || input.ksr_frequency > 15
            || self.eg_sl > 15
        {
            return Err(EnvelopeSequencerError::InvalidInput);
        }

        let rate_operator = self.operators[usize::from(cycle)];
        let state_rate = rate_operator.stage();
        let off_bit = (rate_operator.level() >> 2) == 0x1f;
        let next_eg_kon = self.eg_kon.wrapping_shl(1) | u8::from(input.key_on);
        let next_eg_off = self.eg_off.wrapping_shl(1) | u8::from(off_bit);
        let mut next_eg_dokon = self.eg_dokon.wrapping_shl(1);

        let key_on_pipeline = next_eg_kon & 1 != 0;
        let off_pipeline = next_eg_off & 1 != 0;
        let mut rate_state = state_rate;
        if state_rate == EnvelopeOperatorStage::Release && key_on_pipeline && off_pipeline {
            rate_state = EnvelopeOperatorStage::Attack;
            next_eg_dokon |= 1;
        }

        let mcsel = ((u16::from(cycle) + 1) / 3) & 1 != 0;
        let mut base_rate = match rate_state {
            EnvelopeOperatorStage::Attack => input.attack_rate,
            EnvelopeOperatorStage::Decay => input.decay_rate,
            EnvelopeOperatorStage::Sustain => {
                if input.sustained_tone {
                    0
                } else {
                    input.release_rate
                }
            }
            EnvelopeOperatorStage::Release => {
                if input.sustain_on {
                    5
                } else {
                    input.release_rate
                }
            }
        };

        if !key_on_pipeline && !mcsel {
            base_rate = 0;
        }
        if key_on_pipeline && state_rate == EnvelopeOperatorStage::Release && !off_pipeline {
            base_rate = 12;
        }
        if !key_on_pipeline && !input.sustain_on && mcsel && !input.sustained_tone {
            base_rate = 7;
        }

        let next_zero_rate = base_rate == 0;
        let ksr = if input.key_scale_rate {
            input.ksr_frequency
        } else {
            input.ksr_frequency >> 2
        };
        let composed_rate = (u16::from(base_rate) << 2) + u16::from(ksr);
        let next_rate = if composed_rate & 0x40 != 0 {
            0x3c | (ksr & 3)
        } else {
            composed_rate as u8
        };

        let timer = self.timer.tick_slot(cycle, next_rate, next_zero_rate)?;
        let operator_slot = (cycle + 16) % YM2413_ENVELOPE_SEQUENCER_SLOTS as u8;
        let operator = self.operators[usize::from(operator_slot)]
            .tick(EnvelopeControlPhase {
                rate: timer.rate_phase,
                key_on: self.eg_kon & 0x02 != 0,
                off: self.eg_off & 0x02 != 0,
                dokon: self.eg_dokon & 0x02 != 0,
                sustain_level: self.eg_sl,
            })
            .map_err(|source| EnvelopeSequencerError::Operator {
                slot: operator_slot,
                source,
            })?;

        self.eg_kon = next_eg_kon;
        self.eg_off = next_eg_off;
        self.eg_dokon = next_eg_dokon;
        self.eg_sl = input.sustain_level;

        Ok(EnvelopeSequencerTick {
            cycle,
            operator_slot,
            operator,
            timer,
            eg_kon: self.eg_kon,
            eg_off: self.eg_off,
            eg_dokon: self.eg_dokon,
            eg_sl: self.eg_sl,
            next_rate,
            next_zero_rate,
        })
    }

    /// Export all 18 operator states, timer fields, and delayed control latches.
    pub fn export_state(&self) -> EnvelopeSequencerState {
        EnvelopeSequencerState {
            format_version: YM2413_ENVELOPE_SEQUENCER_STATE_VERSION,
            operators: std::array::from_fn(|index| self.operators[index].export_state()),
            timer: self.timer.export_state(),
            eg_kon: self.eg_kon,
            eg_off: self.eg_off,
            eg_dokon: self.eg_dokon,
            eg_sl: self.eg_sl,
        }
    }

    /// Restore a complete sequencer snapshot after validating every component.
    pub fn restore_state(state: EnvelopeSequencerState) -> Result<Self, EnvelopeSequencerError> {
        if state.format_version != YM2413_ENVELOPE_SEQUENCER_STATE_VERSION || state.eg_sl > 15 {
            return Err(EnvelopeSequencerError::InvalidState);
        }

        let timer = EnvelopeTimer::restore_state(state.timer)?;
        let mut operators = [Self::new().operators[0]; YM2413_ENVELOPE_SEQUENCER_SLOTS];
        for (slot, operator_state) in state.operators.into_iter().enumerate() {
            operators[slot] =
                EnvelopeOperator::restore_state(operator_state).map_err(|source| {
                    EnvelopeSequencerError::Operator {
                        slot: slot as u8,
                        source,
                    }
                })?;
        }

        Ok(Self {
            operators,
            timer,
            eg_kon: state.eg_kon,
            eg_off: state.eg_off,
            eg_dokon: state.eg_dokon,
            eg_sl: state.eg_sl,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{EnvelopeSequencer, EnvelopeSlotInput};

    #[test]
    fn reset_state_matches_serial_eg_reset_latches() {
        let sequencer = EnvelopeSequencer::new();
        let state = sequencer.export_state();
        assert_eq!(state.eg_kon, 0);
        assert_eq!(state.eg_off, 0);
        assert_eq!(state.eg_dokon, 0);
        assert_eq!(state.eg_sl, 0);
        assert!(!state.timer.pending_zero_rate);
        for slot in 0..18 {
            assert_eq!(
                sequencer.operator(slot),
                Some((127, super::EnvelopeOperatorStage::Release))
            );
        }
    }

    #[test]
    fn zero_base_rate_remains_distinct_from_ksr_composed_rate() {
        let mut sequencer = EnvelopeSequencer::new();
        let result = sequencer
            .tick_slot(
                0,
                EnvelopeSlotInput {
                    attack_rate: 0,
                    decay_rate: 0,
                    release_rate: 0,
                    sustain_level: 0,
                    key_scale_rate: true,
                    ksr_frequency: 7,
                    sustain_on: false,
                    sustained_tone: false,
                    key_on: false,
                },
            )
            .unwrap();
        assert_eq!(result.next_rate, 7);
        assert!(result.next_zero_rate);
        assert!(!result.timer.rate_phase.inc_lo);
    }

    #[test]
    fn state_round_trip_preserves_pipeline_and_timer_phase() {
        let mut sequencer = EnvelopeSequencer::new();
        for cycle in 0..18 {
            sequencer
                .tick_slot(
                    cycle,
                    EnvelopeSlotInput {
                        attack_rate: 10,
                        decay_rate: 5,
                        release_rate: 7,
                        sustain_level: 4,
                        key_scale_rate: false,
                        ksr_frequency: 9,
                        sustain_on: false,
                        sustained_tone: true,
                        key_on: cycle < 4,
                    },
                )
                .unwrap();
        }
        let state = sequencer.export_state();
        let restored = EnvelopeSequencer::restore_state(state).unwrap();
        assert_eq!(restored.export_state(), state);
    }
}
