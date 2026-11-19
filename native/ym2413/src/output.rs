//! YM2413 digital output-pin routing and per-slot pulse-width contract.
//!
//! The Yamaha manual identifies MO as the melody output and RO as the rhythm
//! output. Hardware measurements report that each selected channel remains
//! active for three input clocks within its four-clock slot, then returns to
//! the neutral output level for the remaining clock.
//!
//! This module deliberately models only the slot-level routing and measured
//! pulse width. It does not choose the pulse's exact sub-clock phase, define
//! the neutral electrical voltage, convert the operator's one's-complement
//! i16 representation into a raw nine-bit DAC word, or model the analog
//! source-follower/load circuitry. Those details remain separate evidence
//! questions. The supplied Ym2413DacCode is the project's reconstructed
//! signed measurement-domain level, not a raw DAC input word.

use crate::rhythm_output::{Ym2413DacCode, Ym2413RhythmOutputEvent, Ym2413RhythmOutputSlotResult};
use crate::slot::{
    YM2413_RHYTHM_OUTPUT_MAP, YM2413_SLOT_COUNT, Ym2413Operator, Ym2413RhythmVoice, slot_descriptor,
};

/// Number of input clocks in one serial operator slot.
pub const YM2413_OUTPUT_SLOT_CLOCKS: u8 = 4;
/// Measured number of clocks for which a selected output level remains active.
pub const YM2413_OUTPUT_ACTIVE_CLOCKS: u8 = 3;
/// Measured neutral interval remaining in each four-clock slot.
pub const YM2413_OUTPUT_NEUTRAL_CLOCKS: u8 =
    YM2413_OUTPUT_SLOT_CLOCKS - YM2413_OUTPUT_ACTIVE_CLOCKS;

/// Physical analog output pin selected by the YM2413 output routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413OutputPin {
    /// Melody output pin.
    Mo,
    /// Rhythm output pin.
    Ro,
}

/// Logical source associated with one output pulse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413OutputSource {
    /// Zero-based melodic channel index (0..=8).
    MelodyChannel(u8),
    /// Rhythm voice routed to RO.
    RhythmVoice(Ym2413RhythmVoice),
}

/// Result of adapting a rhythm-retention slot result to the output-pin layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413OutputSlotResult {
    /// The slot does not emit a rhythm value.
    NoOutput,
    /// A retained or original rhythm value is routed to RO.
    Pulse(Ym2413OutputPulse),
    /// A delayed-repeat slot was reached before its source value was observed.
    RepeatNotInitialized {
        slot_index: u8,
        voice: Ym2413RhythmVoice,
    },
}

/// One slot's measured-domain output pulse.
///
/// The pulse width is fixed by the documented/observed three-active-clock
/// interval. Its exact position among the four clocks is intentionally not
/// represented because the available evidence does not establish that
/// sub-clock alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413OutputPulse {
    pub pin: Ym2413OutputPin,
    pub slot_index: u8,
    pub source: Ym2413OutputSource,
    pub level: Ym2413DacCode,
}

impl Ym2413OutputPulse {
    /// Return the measured active duration in input clocks.
    pub const fn active_input_clocks(self) -> u8 {
        YM2413_OUTPUT_ACTIVE_CLOCKS
    }

    /// Return the complete slot duration in input clocks.
    pub const fn slot_input_clocks(self) -> u8 {
        YM2413_OUTPUT_SLOT_CLOCKS
    }

    /// Return the neutral interval in input clocks after the active window.
    pub const fn neutral_input_clocks(self) -> u8 {
        YM2413_OUTPUT_NEUTRAL_CLOCKS
    }
}

/// Errors from the slot-level MO/RO routing contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413OutputRoutingError {
    /// The slot index is outside the 18-slot rotation.
    InvalidSlotIndex(u8),
    /// A melodic carrier slot was serviced without a supplied output level.
    MissingMelodyLevel { slot_index: u8, channel: u8 },
    /// A level was supplied at a slot that does not emit melodic output.
    UnexpectedMelodyLevel { slot_index: u8 },
    /// A rhythm event does not match the published slot/voice/repeat map.
    InvalidRhythmEvent(Ym2413RhythmOutputEvent),
}

impl core::fmt::Display for Ym2413OutputRoutingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSlotIndex(slot) => {
                write!(f, "invalid YM2413 output slot index: {slot}")
            }
            Self::MissingMelodyLevel {
                slot_index,
                channel,
            } => write!(
                f,
                "missing YM2413 MO level at slot {slot_index} for channel {channel}"
            ),
            Self::UnexpectedMelodyLevel { slot_index } => write!(
                f,
                "unexpected YM2413 MO level at non-melodic-output slot {slot_index}"
            ),
            Self::InvalidRhythmEvent(event) => write!(
                f,
                "invalid YM2413 RO event at slot {} for {:?}",
                event.slot_index, event.voice
            ),
        }
    }
}

impl std::error::Error for Ym2413OutputRoutingError {}

/// Resolve the MO output event for one slot.
///
/// In melodic mode, all nine carrier slots emit to MO. In Rhythm Mode, only
/// channels 1..=6 remain melodic, so carrier slots for channels 7..=9 do not
/// emit a melody event. A zero-valued level must be supplied explicitly for a
/// silent carrier slot; None means the caller did not provide the slot result.
///
/// This function does not evaluate operators or advance clocks.
pub fn melody_output_for_slot(
    slot_index: u8,
    rhythm_mode_enabled: bool,
    level: Option<Ym2413DacCode>,
) -> Result<Option<Ym2413OutputPulse>, Ym2413OutputRoutingError> {
    let descriptor = slot_descriptor(slot_index)
        .ok_or(Ym2413OutputRoutingError::InvalidSlotIndex(slot_index))?;
    let is_melody_output = descriptor.operator == Ym2413Operator::Carrier
        && (!rhythm_mode_enabled || descriptor.channel < 6);

    if !is_melody_output {
        return if level.is_some() {
            Err(Ym2413OutputRoutingError::UnexpectedMelodyLevel { slot_index })
        } else {
            Ok(None)
        };
    }

    let level = level.ok_or(Ym2413OutputRoutingError::MissingMelodyLevel {
        slot_index,
        channel: descriptor.channel,
    })?;

    Ok(Some(Ym2413OutputPulse {
        pin: Ym2413OutputPin::Mo,
        slot_index,
        source: Ym2413OutputSource::MelodyChannel(descriptor.channel),
        level,
    }))
}

/// Route one already-retained rhythm output event to RO.
///
/// The event is checked against the measured original/repeat slot map. This
/// does not synthesize the voice, perform the operator-to-DAC conversion, or
/// advance the rhythm retention state.
pub fn rhythm_output_event(
    event: Ym2413RhythmOutputEvent,
) -> Result<Ym2413OutputPulse, Ym2413OutputRoutingError> {
    let valid = YM2413_RHYTHM_OUTPUT_MAP.iter().any(|entry| {
        entry.slot_index == event.slot_index
            && entry.voice == event.voice
            && entry.is_delayed_repeat == event.is_delayed_repeat
    });
    if !valid || usize::from(event.slot_index) >= YM2413_SLOT_COUNT {
        return Err(Ym2413OutputRoutingError::InvalidRhythmEvent(event));
    }

    Ok(Ym2413OutputPulse {
        pin: Ym2413OutputPin::Ro,
        slot_index: event.slot_index,
        source: Ym2413OutputSource::RhythmVoice(event.voice),
        level: event.code,
    })
}

/// Convert a rhythm retention result to an RO pulse when a value was emitted.
///
/// Uninitialized repeats remain a distinct result for diagnostics and reset-state
/// investigations; neither case is silently converted to digital zero.
pub fn rhythm_slot_result(
    result: Ym2413RhythmOutputSlotResult,
) -> Result<Ym2413OutputSlotResult, Ym2413OutputRoutingError> {
    match result {
        Ym2413RhythmOutputSlotResult::Emitted(event) => {
            rhythm_output_event(event).map(Ym2413OutputSlotResult::Pulse)
        }
        Ym2413RhythmOutputSlotResult::NoRhythmOutput => Ok(Ym2413OutputSlotResult::NoOutput),
        Ym2413RhythmOutputSlotResult::RepeatNotInitialized { slot_index, voice } => {
            Ok(Ym2413OutputSlotResult::RepeatNotInitialized { slot_index, voice })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        YM2413_OUTPUT_ACTIVE_CLOCKS, YM2413_OUTPUT_NEUTRAL_CLOCKS, YM2413_OUTPUT_SLOT_CLOCKS,
        Ym2413OutputPin, Ym2413OutputRoutingError, Ym2413OutputSlotResult, Ym2413OutputSource,
        melody_output_for_slot,
        rhythm_output_event, rhythm_slot_result,
    };
    use crate::rhythm_output::{
        Ym2413DacCode, Ym2413RhythmOutputEvent, Ym2413RhythmOutputSlotResult,
    };
    use crate::slot::Ym2413RhythmVoice;

    fn level(value: i16) -> Ym2413DacCode {
        Ym2413DacCode::new(value).unwrap()
    }

    #[test]
    fn measured_output_window_is_three_active_clocks_in_each_four_clock_slot() {
        assert_eq!(YM2413_OUTPUT_ACTIVE_CLOCKS, 3);
        assert_eq!(YM2413_OUTPUT_NEUTRAL_CLOCKS, 1);
        assert_eq!(
            YM2413_OUTPUT_ACTIVE_CLOCKS + YM2413_OUTPUT_NEUTRAL_CLOCKS,
            YM2413_OUTPUT_SLOT_CLOCKS
        );
    }

    #[test]
    fn melodic_mode_routes_all_nine_carrier_slots_to_mo() {
        let expected = [
            (3, 0),
            (4, 1),
            (5, 2),
            (9, 3),
            (10, 4),
            (11, 5),
            (15, 6),
            (16, 7),
            (17, 8),
        ];

        for (slot, channel) in expected {
            let pulse = melody_output_for_slot(slot, false, Some(level(channel as i16))).unwrap();
            assert_eq!(
                pulse,
                Some(super::Ym2413OutputPulse {
                    pin: Ym2413OutputPin::Mo,
                    slot_index: slot,
                    source: Ym2413OutputSource::MelodyChannel(channel),
                    level: level(channel as i16),
                })
            );
        }
    }

    #[test]
    fn rhythm_mode_routes_only_the_first_six_melodic_channels_to_mo() {
        for slot in [3, 4, 5, 9, 10, 11] {
            assert!(
                melody_output_for_slot(slot, true, Some(level(12)))
                    .unwrap()
                    .is_some()
            );
        }
        for slot in [15, 16, 17] {
            assert_eq!(melody_output_for_slot(slot, true, None), Ok(None));
        }
    }

    #[test]
    fn non_carrier_slots_do_not_emit_melody_and_reject_supplied_levels() {
        for slot in [0, 1, 2, 6, 7, 8, 12, 13, 14] {
            assert_eq!(melody_output_for_slot(slot, false, None), Ok(None));
            assert_eq!(
                melody_output_for_slot(slot, false, Some(level(1))),
                Err(Ym2413OutputRoutingError::UnexpectedMelodyLevel { slot_index: slot })
            );
        }
    }

    #[test]
    fn melody_carrier_slots_require_explicit_levels_including_silence() {
        assert_eq!(
            melody_output_for_slot(3, false, None),
            Err(Ym2413OutputRoutingError::MissingMelodyLevel {
                slot_index: 3,
                channel: 0,
            })
        );
        assert_eq!(
            melody_output_for_slot(18, false, None),
            Err(Ym2413OutputRoutingError::InvalidSlotIndex(18))
        );
    }

    #[test]
    fn measured_rhythm_original_and_repeat_events_route_to_ro() {
        let events = [
            (0, Ym2413RhythmVoice::HiHat, true),
            (1, Ym2413RhythmVoice::TomTom, true),
            (2, Ym2413RhythmVoice::BassDrum, true),
            (6, Ym2413RhythmVoice::SnareDrum, true),
            (7, Ym2413RhythmVoice::TopCymbal, true),
            (13, Ym2413RhythmVoice::HiHat, false),
            (14, Ym2413RhythmVoice::TomTom, false),
            (15, Ym2413RhythmVoice::BassDrum, false),
            (16, Ym2413RhythmVoice::SnareDrum, false),
            (17, Ym2413RhythmVoice::TopCymbal, false),
        ];

        for (slot, voice, is_delayed_repeat) in events {
            let event = Ym2413RhythmOutputEvent {
                slot_index: slot,
                voice,
                is_delayed_repeat,
                code: level(-37),
            };
            let pulse = rhythm_output_event(event).unwrap();
            assert_eq!(pulse.pin, Ym2413OutputPin::Ro);
            assert_eq!(pulse.slot_index, slot);
            assert_eq!(pulse.source, Ym2413OutputSource::RhythmVoice(voice));
            assert_eq!(pulse.level, level(-37));
            assert_eq!(pulse.active_input_clocks(), 3);
            assert_eq!(pulse.neutral_input_clocks(), 1);
        }
    }

    #[test]
    fn invalid_rhythm_events_are_rejected() {
        let event = Ym2413RhythmOutputEvent {
            slot_index: 12,
            voice: Ym2413RhythmVoice::HiHat,
            is_delayed_repeat: false,
            code: level(0),
        };
        assert_eq!(
            rhythm_output_event(event),
            Err(Ym2413OutputRoutingError::InvalidRhythmEvent(event))
        );
    }

    #[test]
    fn rhythm_slot_result_preserves_uninitialized_repeat_as_no_pulse() {
        assert_eq!(
            rhythm_slot_result(Ym2413RhythmOutputSlotResult::RepeatNotInitialized {
                slot_index: 0,
                voice: Ym2413RhythmVoice::HiHat,
            }),
            Ok(Ym2413OutputSlotResult::RepeatNotInitialized {
                slot_index: 0,
                voice: Ym2413RhythmVoice::HiHat,
            })
        );
        assert_eq!(
            rhythm_slot_result(Ym2413RhythmOutputSlotResult::NoRhythmOutput),
            Ok(Ym2413OutputSlotResult::NoOutput)
        );
    }
}
