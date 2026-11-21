//! Deterministic ownership dispatch for the YM2413's eighteen serial slots.
//!
//! This module resolves the existing slot table against the delayed RHY mode
//! register and the core-owned serial rhythm selector. It does not advance
//! clocks, PG, EG, operators, LFSR state, or rhythm output retention.
//!
//! A selector mismatch during a mode transition is represented explicitly;
//! it is never silently routed to a percussion voice.

use crate::envelope_sequencer::EnvelopeRhythmSelect;
use crate::slot::{slot_descriptor, Ym2413RhythmVoice, Ym2413SlotDescriptor};

/// Result of resolving one core-owned serial slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413SerialSlotDispatch {
    /// Ordinary melodic ownership, including channels 7..9 when RHY is clear.
    Melodic { slot: Ym2413SlotDescriptor },
    /// A rhythm selector agrees with the canonical slot and voice assignment.
    Rhythm {
        slot: Ym2413SlotDescriptor,
        voice: Ym2413RhythmVoice,
        selector: EnvelopeRhythmSelect,
    },
    /// RHY is set, but the delayed selector has not reached this slot's
    /// expected voice yet. The caller must preserve transition semantics.
    RhythmTransition {
        slot: Ym2413SlotDescriptor,
        expected: EnvelopeRhythmSelect,
        actual: EnvelopeRhythmSelect,
    },
}

/// Errors returned by serial slot ownership resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413SerialSlotDispatchError {
    /// The serial cycle is outside the eighteen-slot rotation.
    InvalidCycle(u8),
}

/// Resolve the logical owner of a serial cycle from the core's delayed
/// register image and rhythm selector.
///
/// cycle is the zero-based cycle supplied by Ym2413Core. registers must be
/// the post-mode-write register image from Ym2413SlotServiceContext.
/// The function is pure: it does not mutate or infer the core's mode latches.
pub fn dispatch_serial_slot(
    cycle: u8,
    registers: &[u8; 64],
    rhythm_select: EnvelopeRhythmSelect,
) -> Result<Ym2413SerialSlotDispatch, Ym2413SerialSlotDispatchError> {
    let slot = slot_descriptor(cycle)
        .ok_or(Ym2413SerialSlotDispatchError::InvalidCycle(cycle))?;

    if registers[0x0e] & 0x20 == 0 || slot.rhythm_voice.is_none() {
        return Ok(Ym2413SerialSlotDispatch::Melodic { slot });
    }

    let expected = match cycle {
        11 => EnvelopeRhythmSelect::BassDrumFirst,
        12 => EnvelopeRhythmSelect::HiHat,
        13 => EnvelopeRhythmSelect::TomTom,
        14 => EnvelopeRhythmSelect::BassDrumSecond,
        15 => EnvelopeRhythmSelect::SnareDrum,
        16 => EnvelopeRhythmSelect::TopCymbal,
        _ => return Ok(Ym2413SerialSlotDispatch::Melodic { slot }),
    };

    if rhythm_select != expected {
        return Ok(Ym2413SerialSlotDispatch::RhythmTransition {
            slot,
            expected,
            actual: rhythm_select,
        });
    }

    let Some(voice) = slot.rhythm_voice else {
        return Ok(Ym2413SerialSlotDispatch::Melodic { slot });
    };
    Ok(Ym2413SerialSlotDispatch::Rhythm {
        slot,
        voice,
        selector: rhythm_select,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Ym2413SerialSlotDispatch, Ym2413SerialSlotDispatchError, dispatch_serial_slot,
    };
    use crate::envelope_sequencer::EnvelopeRhythmSelect as Select;
    use crate::slot::{YM2413_SLOT_COUNT, YM2413_SLOT_MAP, Ym2413RhythmVoice};

    fn registers(rhythm: bool) -> [u8; 64] {
        let mut registers = [0; 64];
        registers[0x0e] = if rhythm { 0x20 } else { 0 };
        registers
    }

    #[test]
    fn melodic_mode_keeps_all_eighteen_slots_on_the_canonical_channel_map() {
        let registers = registers(false);
        for cycle in 0..YM2413_SLOT_COUNT as u8 {
            let dispatch = dispatch_serial_slot(cycle, &registers, Select::None).unwrap();
            assert_eq!(
                dispatch,
                Ym2413SerialSlotDispatch::Melodic {
                    slot: YM2413_SLOT_MAP[usize::from(cycle)],
                },
                "cycle {cycle}"
            );
        }
    }

    #[test]
    fn rhythm_mode_reassigns_only_the_six_documented_serial_cycles() {
        let registers = registers(true);
        let rhythm_cycles = [
            (11, Select::BassDrumFirst, Ym2413RhythmVoice::BassDrum),
            (12, Select::HiHat, Ym2413RhythmVoice::HiHat),
            (13, Select::TomTom, Ym2413RhythmVoice::TomTom),
            (14, Select::BassDrumSecond, Ym2413RhythmVoice::BassDrum),
            (15, Select::SnareDrum, Ym2413RhythmVoice::SnareDrum),
            (16, Select::TopCymbal, Ym2413RhythmVoice::TopCymbal),
        ];

        for (cycle, selector, voice) in rhythm_cycles {
            assert_eq!(
                dispatch_serial_slot(cycle, &registers, selector).unwrap(),
                Ym2413SerialSlotDispatch::Rhythm {
                    slot: YM2413_SLOT_MAP[usize::from(cycle)],
                    voice,
                    selector,
                },
                "cycle {cycle}"
            );
        }

        for cycle in (0..YM2413_SLOT_COUNT as u8).filter(|cycle| !(11..=16).contains(cycle)) {
            assert!(matches!(
                dispatch_serial_slot(cycle, &registers, Select::None).unwrap(),
                Ym2413SerialSlotDispatch::Melodic { .. }
            ));
        }
    }

    #[test]
    fn rhythm_selector_mismatch_is_explicit_and_never_misroutes_a_voice() {
        let registers = registers(true);
        for cycle in 11..=16 {
            for actual_code in 0..=6 {
                let actual = Select::from_code(actual_code).unwrap();
                let result = dispatch_serial_slot(cycle, &registers, actual).unwrap();
                let expected = match cycle {
                    11 => Select::BassDrumFirst,
                    12 => Select::HiHat,
                    13 => Select::TomTom,
                    14 => Select::BassDrumSecond,
                    15 => Select::SnareDrum,
                    16 => Select::TopCymbal,
                    _ => unreachable!(),
                };
                if actual == expected {
                    assert!(matches!(result, Ym2413SerialSlotDispatch::Rhythm { .. }));
                } else {
                    assert_eq!(
                        result,
                        Ym2413SerialSlotDispatch::RhythmTransition {
                            slot: YM2413_SLOT_MAP[usize::from(cycle)],
                            expected,
                            actual,
                        },
                        "cycle {cycle}, selector {actual_code}"
                    );
                }
            }
        }
    }

    #[test]
    fn invalid_cycle_is_rejected_without_indexing_the_slot_table() {
        assert_eq!(
            dispatch_serial_slot(18, &registers(true), Select::None),
            Err(Ym2413SerialSlotDispatchError::InvalidCycle(18))
        );
        assert_eq!(
            dispatch_serial_slot(u8::MAX, &registers(false), Select::None),
            Err(Ym2413SerialSlotDispatchError::InvalidCycle(u8::MAX))
        );
    }
}
