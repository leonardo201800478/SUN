//! Behavioral retention of measured YM2413 Rhythm Mode output values.
//!
//! This module models the externally observed duplicate emissions. It does
//! not claim a particular physical shift-register implementation, convert
//! operator samples into DAC codes, advance chip clocks, or model MO/RO pulse
//! widths and analog circuitry.

use crate::slot::{YM2413_RHYTHM_OUTPUT_MAP, YM2413_SLOT_COUNT, Ym2413RhythmVoice};

/// Lowest signed digital value observed at the YM2413's nine-bit output.
pub const YM2413_DAC_CODE_MIN: i16 = -255;
/// Highest signed digital value observed at the YM2413's nine-bit output.
pub const YM2413_DAC_CODE_MAX: i16 = 255;
/// Version of the rhythm output-retention snapshot.
pub const YM2413_RHYTHM_OUTPUT_DELAY_STATE_VERSION: u16 = 1;

/// One signed digital code accepted by the YM2413's nine-bit output stage.
///
/// The range reflects the 511 distinct values measured by andete
/// (-255..=+255); the unused -256 code is rejected. This type does not
/// describe the electrical voltage on MO/RO or convert operator i16 values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413DacCode(i16);

impl Ym2413DacCode {
    /// Construct a measured-domain DAC code, rejecting values outside its range.
    pub const fn new(value: i16) -> Result<Self, Ym2413RhythmOutputError> {
        if value < YM2413_DAC_CODE_MIN || value > YM2413_DAC_CODE_MAX {
            return Err(Ym2413RhythmOutputError::InvalidDacCode(value));
        }
        Ok(Self(value))
    }

    /// Return the signed nine-bit digital code.
    pub const fn value(self) -> i16 {
        self.0
    }
}

/// One rhythm value emitted at an original or repeated slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmOutputEvent {
    /// Zero-based slot index in the 18-slot rotation.
    pub slot_index: u8,
    /// Percussion voice whose value is emitted.
    pub voice: Ym2413RhythmVoice,
    /// True when the value is the measured delayed duplicate.
    pub is_delayed_repeat: bool,
    /// Exact signed nine-bit code emitted for this occurrence.
    pub code: Ym2413DacCode,
}

/// Result of processing one serial slot.
///
/// An uninitialized repeat is distinct from an emitted zero: reset contents
/// of the physical retention path are not established by current evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413RhythmOutputSlotResult {
    /// This slot does not carry a rhythm output occurrence.
    NoRhythmOutput,
    /// A rhythm value was emitted.
    Emitted(Ym2413RhythmOutputEvent),
    /// A repeated slot was reached before this model had observed its source.
    RepeatNotInitialized {
        slot_index: u8,
        voice: Ym2413RhythmVoice,
    },
}

/// Errors from the rhythm output-retention model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413RhythmOutputError {
    /// A value is outside the measured signed nine-bit output domain.
    InvalidDacCode(i16),
    /// The supplied slot index is outside 0..18.
    InvalidSlotIndex(u8),
    /// The slot is not the next slot expected by this state.
    UnexpectedSlot {
        expected: u8,
        actual: u8,
    },
    /// An original rhythm-output slot was reached without its computed value.
    MissingOriginalSample {
        slot_index: u8,
        voice: Ym2413RhythmVoice,
    },
    /// A sample was supplied for a slot that does not generate an original value.
    UnexpectedOriginalSample {
        slot_index: u8,
    },
    /// Snapshot version or cursor is invalid.
    InvalidState,
}

/// Snapshot of the five retained rhythm output values and serial cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmOutputDelayState {
    pub format_version: u16,
    /// Slot index expected by the next process_slot call.
    pub next_slot_index: u8,
    /// Most recently generated original code for BD, HH, SD, TOM and TCY.
    /// None means no source value has been observed in this state history.
    pub retained_values: [Option<Ym2413DacCode>; 5],
}

/// Behavioral model of the five measured delayed rhythm-output values.
///
/// Call process_slot exactly once for each completed slot, in serial order.
/// Supply a code only at original source slots 13..17. Repeated occurrences
/// at slots 0, 1, 2, 6 and 7 emit the retained source code without recomputing
/// it. The caller owns the actual operator evaluation and chip-clock timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmOutputDelay {
    state: Ym2413RhythmOutputDelayState,
}

impl Ym2413RhythmOutputDelay {
    /// Restore from caller-supplied initial conditions; no latch reset value is guessed.
    pub const fn with_initial_state(
        state: Ym2413RhythmOutputDelayState,
    ) -> Result<Self, Ym2413RhythmOutputError> {
        if state.format_version != YM2413_RHYTHM_OUTPUT_DELAY_STATE_VERSION
            || state.next_slot_index as usize >= YM2413_SLOT_COUNT
        {
            return Err(Ym2413RhythmOutputError::InvalidState);
        }
        Ok(Self { state })
    }

    /// Export all retained codes and the next expected slot for save states.
    pub const fn export_state(&self) -> Ym2413RhythmOutputDelayState {
        self.state
    }

    /// Process one slot and return its rhythm-output event, if any.
    ///
    /// original_code must be present only at source slots 13..17. At repeat
    /// slots the retained value is emitted verbatim. No clock or subphase is
    /// advanced here; the caller invokes this at its established slot boundary.
    pub fn process_slot(
        &mut self,
        slot_index: u8,
        original_code: Option<Ym2413DacCode>,
    ) -> Result<Ym2413RhythmOutputSlotResult, Ym2413RhythmOutputError> {
        if slot_index as usize >= YM2413_SLOT_COUNT {
            return Err(Ym2413RhythmOutputError::InvalidSlotIndex(slot_index));
        }
        if slot_index != self.state.next_slot_index {
            return Err(Ym2413RhythmOutputError::UnexpectedSlot {
                expected: self.state.next_slot_index,
                actual: slot_index,
            });
        }

        let occurrence = YM2413_RHYTHM_OUTPUT_MAP
            .iter()
            .find(|entry| entry.slot_index == slot_index)
            .copied();

        let result = match occurrence {
            None => {
                if original_code.is_some() {
                    return Err(Ym2413RhythmOutputError::UnexpectedOriginalSample { slot_index });
                }
                Ym2413RhythmOutputSlotResult::NoRhythmOutput
            }
            Some(entry) if entry.is_delayed_repeat => {
                if original_code.is_some() {
                    return Err(Ym2413RhythmOutputError::UnexpectedOriginalSample { slot_index });
                }
                match self.state.retained_values[voice_index(entry.voice)] {
                    Some(code) => Ym2413RhythmOutputSlotResult::Emitted(
                        Ym2413RhythmOutputEvent {
                            slot_index,
                            voice: entry.voice,
                            is_delayed_repeat: true,
                            code,
                        },
                    ),
                    None => Ym2413RhythmOutputSlotResult::RepeatNotInitialized {
                        slot_index,
                        voice: entry.voice,
                    },
                }
            }
            Some(entry) => {
                let code = original_code.ok_or(
                    Ym2413RhythmOutputError::MissingOriginalSample {
                        slot_index,
                        voice: entry.voice,
                    },
                )?;
                self.state.retained_values[voice_index(entry.voice)] = Some(code);
                Ym2413RhythmOutputSlotResult::Emitted(Ym2413RhythmOutputEvent {
                    slot_index,
                    voice: entry.voice,
                    is_delayed_repeat: false,
                    code,
                })
            }
        };

        self.state.next_slot_index = (slot_index + 1) % YM2413_SLOT_COUNT as u8;
        Ok(result)
    }
}

const fn voice_index(voice: Ym2413RhythmVoice) -> usize {
    match voice {
        Ym2413RhythmVoice::BassDrum => 0,
        Ym2413RhythmVoice::HiHat => 1,
        Ym2413RhythmVoice::SnareDrum => 2,
        Ym2413RhythmVoice::TomTom => 3,
        Ym2413RhythmVoice::TopCymbal => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        YM2413_DAC_CODE_MAX, YM2413_DAC_CODE_MIN, YM2413_RHYTHM_OUTPUT_DELAY_STATE_VERSION,
        Ym2413DacCode, Ym2413RhythmOutputDelay, Ym2413RhythmOutputDelayState,
        Ym2413RhythmOutputError, Ym2413RhythmOutputSlotResult,
    };
    use crate::slot::Ym2413RhythmVoice;

    fn initial_state(next_slot_index: u8) -> Ym2413RhythmOutputDelayState {
        Ym2413RhythmOutputDelayState {
            format_version: YM2413_RHYTHM_OUTPUT_DELAY_STATE_VERSION,
            next_slot_index,
            retained_values: [None; 5],
        }
    }

    fn code(value: i16) -> Ym2413DacCode {
        Ym2413DacCode::new(value).unwrap()
    }

    #[test]
    fn dac_code_accepts_measured_endpoints_and_rejects_unused_or_out_of_range_values() {
        assert_eq!(code(YM2413_DAC_CODE_MIN).value(), -255);
        assert_eq!(code(YM2413_DAC_CODE_MAX).value(), 255);
        assert_eq!(
            Ym2413DacCode::new(-256),
            Err(Ym2413RhythmOutputError::InvalidDacCode(-256))
        );
        assert_eq!(
            Ym2413DacCode::new(256),
            Err(Ym2413RhythmOutputError::InvalidDacCode(256))
        );
    }

    #[test]
    fn original_rhythm_codes_are_repeated_verbatim_at_measured_slots() {
        let mut delay =
            Ym2413RhythmOutputDelay::with_initial_state(initial_state(13)).unwrap();
        let originals = [
            (13, Ym2413RhythmVoice::HiHat, 21),
            (14, Ym2413RhythmVoice::TomTom, -34),
            (15, Ym2413RhythmVoice::BassDrum, 55),
            (16, Ym2413RhythmVoice::SnareDrum, -89),
            (17, Ym2413RhythmVoice::TopCymbal, 144),
        ];

        for (slot, voice, value) in originals {
            assert_eq!(
                delay.process_slot(slot, Some(code(value))).unwrap(),
                Ym2413RhythmOutputSlotResult::Emitted(super::Ym2413RhythmOutputEvent {
                    slot_index: slot,
                    voice,
                    is_delayed_repeat: false,
                    code: code(value),
                })
            );
        }

        for slot in 0..13_u8 {
            let expected = match slot {
                0 => Some((Ym2413RhythmVoice::HiHat, 21)),
                1 => Some((Ym2413RhythmVoice::TomTom, -34)),
                2 => Some((Ym2413RhythmVoice::BassDrum, 55)),
                6 => Some((Ym2413RhythmVoice::SnareDrum, -89)),
                7 => Some((Ym2413RhythmVoice::TopCymbal, 144)),
                _ => None,
            };
            let actual = delay.process_slot(slot, None).unwrap();
            if let Some((voice, value)) = expected {
                assert_eq!(
                    actual,
                    Ym2413RhythmOutputSlotResult::Emitted(
                        super::Ym2413RhythmOutputEvent {
                            slot_index: slot,
                            voice,
                            is_delayed_repeat: true,
                            code: code(value),
                        }
                    )
                );
            } else {
                assert_eq!(actual, Ym2413RhythmOutputSlotResult::NoRhythmOutput);
            }
        }
    }

    #[test]
    fn repeat_before_first_source_is_not_silently_converted_to_zero() {
        let mut delay =
            Ym2413RhythmOutputDelay::with_initial_state(initial_state(0)).unwrap();
        assert_eq!(
            delay.process_slot(0, None).unwrap(),
            Ym2413RhythmOutputSlotResult::RepeatNotInitialized {
                slot_index: 0,
                voice: Ym2413RhythmVoice::HiHat,
            }
        );
    }

    #[test]
    fn invalid_slot_call_does_not_advance_or_mutate_state() {
        let mut delay =
            Ym2413RhythmOutputDelay::with_initial_state(initial_state(0)).unwrap();
        let before = delay.export_state();
        assert_eq!(
            delay.process_slot(1, None),
            Err(Ym2413RhythmOutputError::UnexpectedSlot {
                expected: 0,
                actual: 1,
            })
        );
        assert_eq!(delay.export_state(), before);
    }

    #[test]
    fn original_slot_requires_a_value_and_rejects_values_on_non_source_slots() {
        let mut delay =
            Ym2413RhythmOutputDelay::with_initial_state(initial_state(13)).unwrap();
        assert_eq!(
            delay.process_slot(13, None),
            Err(Ym2413RhythmOutputError::MissingOriginalSample {
                slot_index: 13,
                voice: Ym2413RhythmVoice::HiHat,
            })
        );
        assert_eq!(delay.export_state().next_slot_index, 13);
        assert_eq!(
            delay.process_slot(13, Some(code(1))).unwrap(),
            Ym2413RhythmOutputSlotResult::Emitted(super::Ym2413RhythmOutputEvent {
                slot_index: 13,
                voice: Ym2413RhythmVoice::HiHat,
                is_delayed_repeat: false,
                code: code(1),
            })
        );
    }

    #[test]
    fn unexpected_sample_is_rejected_without_changing_retention_or_cursor() {
        let mut delay =
            Ym2413RhythmOutputDelay::with_initial_state(initial_state(0)).unwrap();
        let before = delay.export_state();

        assert_eq!(
            delay.process_slot(0, Some(code(77))),
            Err(Ym2413RhythmOutputError::UnexpectedOriginalSample { slot_index: 0 })
        );
        assert_eq!(delay.export_state(), before);

        assert_eq!(
            delay.process_slot(18, None),
            Err(Ym2413RhythmOutputError::InvalidSlotIndex(18))
        );
        assert_eq!(delay.export_state(), before);
    }

    #[test]
    fn snapshot_restore_preserves_retained_codes_and_slot_cursor() {
        let mut delay =
            Ym2413RhythmOutputDelay::with_initial_state(initial_state(13)).unwrap();
        delay.process_slot(13, Some(code(123))).unwrap();
        let snapshot = delay.export_state();
        let restored = Ym2413RhythmOutputDelay::with_initial_state(snapshot).unwrap();
        assert_eq!(restored.export_state(), snapshot);
        assert_eq!(restored.state.retained_values[1], Some(code(123)));
    }

    #[test]
    fn restore_rejects_invalid_version_or_slot_cursor() {
        let mut state = initial_state(0);
        state.format_version += 1;
        assert_eq!(
            Ym2413RhythmOutputDelay::with_initial_state(state),
            Err(Ym2413RhythmOutputError::InvalidState)
        );
        let state = initial_state(18);
        assert_eq!(
            Ym2413RhythmOutputDelay::with_initial_state(state),
            Err(Ym2413RhythmOutputError::InvalidState)
        );
    }
}
