//! YM2413 serial operator-slot assignment.
//!
//! The Yamaha application manual documents one shared FM operator cell
//! traversing eighteen slots. Its table assigns slots 1..18 to channels
//! 1,2,3,1,2,3,4,5,6,4,5,6,7,8,9,7,8,9. Within each channel the first
//! occurrence is the modulator and the second is the carrier.
//!
//! Rhythm mode repurposes six slots from melodic channels 7..9. This module
//! describes ownership only; it does not implement percussion synthesis or
//! the internal PG/EG/operator pipeline stage offsets.

/// Number of serial operator slots in one YM2413 synthesis rotation.
pub const YM2413_SLOT_COUNT: usize = 18;

/// Operator position within a two-operator melodic channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413Operator {
    /// First operator cell for a channel; normally the modulator.
    Modulator,
    /// Second operator cell for a channel; normally the carrier.
    Carrier,
}

/// Rhythm voice assignment for a slot when Rhythm Mode is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413RhythmVoice {
    /// Bass drum uses both channel 7 operators (slots 13 and 16).
    BassDrum,
    /// Hi-hat uses slot 14.
    HiHat,
    /// Tom-tom uses slot 15.
    TomTom,
    /// Snare drum uses slot 17.
    SnareDrum,
    /// Top cymbal uses slot 18.
    TopCymbal,
}

/// Measured rhythm-output occurrence for one slot in the 18-slot rotation.
///
/// A repeated occurrence reproduces the same nine-bit value measured at
/// source_slot_index. This map records output positions only; it does not
/// assert whether the physical chip stores or recomputes that value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmOutputOccurrence {
    /// Zero-based slot index in the 18-slot rotation.
    pub slot_index: u8,
    /// Percussion voice routed at this occurrence.
    pub voice: Ym2413RhythmVoice,
    /// True for the delayed second emission of this voice.
    pub is_delayed_repeat: bool,
    /// Zero-based slot index where the original voice value was generated.
    pub source_slot_index: u8,
}

/// Rhythm-output occurrences measured by andete on a real YM2413.
///
/// The original voices are emitted at slots 13..17; their identical delayed
/// copies appear at slots 0, 1, 2, 6 and 7 of the following rotation.
pub const YM2413_RHYTHM_OUTPUT_MAP: [Ym2413RhythmOutputOccurrence; 10] = [
    Ym2413RhythmOutputOccurrence {
        slot_index: 0,
        voice: Ym2413RhythmVoice::HiHat,
        is_delayed_repeat: true,
        source_slot_index: 13,
    },
    Ym2413RhythmOutputOccurrence {
        slot_index: 1,
        voice: Ym2413RhythmVoice::TomTom,
        is_delayed_repeat: true,
        source_slot_index: 14,
    },
    Ym2413RhythmOutputOccurrence {
        slot_index: 2,
        voice: Ym2413RhythmVoice::BassDrum,
        is_delayed_repeat: true,
        source_slot_index: 15,
    },
    Ym2413RhythmOutputOccurrence {
        slot_index: 6,
        voice: Ym2413RhythmVoice::SnareDrum,
        is_delayed_repeat: true,
        source_slot_index: 16,
    },
    Ym2413RhythmOutputOccurrence {
        slot_index: 7,
        voice: Ym2413RhythmVoice::TopCymbal,
        is_delayed_repeat: true,
        source_slot_index: 17,
    },
    Ym2413RhythmOutputOccurrence {
        slot_index: 13,
        voice: Ym2413RhythmVoice::HiHat,
        is_delayed_repeat: false,
        source_slot_index: 13,
    },
    Ym2413RhythmOutputOccurrence {
        slot_index: 14,
        voice: Ym2413RhythmVoice::TomTom,
        is_delayed_repeat: false,
        source_slot_index: 14,
    },
    Ym2413RhythmOutputOccurrence {
        slot_index: 15,
        voice: Ym2413RhythmVoice::BassDrum,
        is_delayed_repeat: false,
        source_slot_index: 15,
    },
    Ym2413RhythmOutputOccurrence {
        slot_index: 16,
        voice: Ym2413RhythmVoice::SnareDrum,
        is_delayed_repeat: false,
        source_slot_index: 16,
    },
    Ym2413RhythmOutputOccurrence {
        slot_index: 17,
        voice: Ym2413RhythmVoice::TopCymbal,
        is_delayed_repeat: false,
        source_slot_index: 17,
    },
];

/// Static assignment of one serial slot.
///
/// 'channel' is zero-based to match the public channel-indexing APIs in this
/// crate. 'slot_number' is the one-based number printed in the Yamaha manual.
/// 'rhythm_voice' is present only for slots reassigned in Rhythm Mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413SlotDescriptor {
    /// One-based physical/logical slot number (1..=18).
    pub slot_number: u8,
    /// Zero-based channel index (0..=8).
    pub channel: u8,
    /// Modulator or carrier position within the channel.
    pub operator: Ym2413Operator,
    /// Rhythm voice taking ownership of this slot in Rhythm Mode.
    pub rhythm_voice: Option<Ym2413RhythmVoice>,
}

/// Yamaha manual's serial slot allocation, in execution order.
///
/// The channel/operator order is transcribed from Table II-3. Rhythm voice
/// ownership follows the documented six-slot allocation for channels 7..9.
pub const YM2413_SLOT_MAP: [Ym2413SlotDescriptor; YM2413_SLOT_COUNT] = [
    Ym2413SlotDescriptor {
        slot_number: 1,
        channel: 0,
        operator: Ym2413Operator::Modulator,
        rhythm_voice: None,
    },
    Ym2413SlotDescriptor {
        slot_number: 2,
        channel: 1,
        operator: Ym2413Operator::Modulator,
        rhythm_voice: None,
    },
    Ym2413SlotDescriptor {
        slot_number: 3,
        channel: 2,
        operator: Ym2413Operator::Modulator,
        rhythm_voice: None,
    },
    Ym2413SlotDescriptor {
        slot_number: 4,
        channel: 0,
        operator: Ym2413Operator::Carrier,
        rhythm_voice: None,
    },
    Ym2413SlotDescriptor {
        slot_number: 5,
        channel: 1,
        operator: Ym2413Operator::Carrier,
        rhythm_voice: None,
    },
    Ym2413SlotDescriptor {
        slot_number: 6,
        channel: 2,
        operator: Ym2413Operator::Carrier,
        rhythm_voice: None,
    },
    Ym2413SlotDescriptor {
        slot_number: 7,
        channel: 3,
        operator: Ym2413Operator::Modulator,
        rhythm_voice: None,
    },
    Ym2413SlotDescriptor {
        slot_number: 8,
        channel: 4,
        operator: Ym2413Operator::Modulator,
        rhythm_voice: None,
    },
    Ym2413SlotDescriptor {
        slot_number: 9,
        channel: 5,
        operator: Ym2413Operator::Modulator,
        rhythm_voice: None,
    },
    Ym2413SlotDescriptor {
        slot_number: 10,
        channel: 3,
        operator: Ym2413Operator::Carrier,
        rhythm_voice: None,
    },
    Ym2413SlotDescriptor {
        slot_number: 11,
        channel: 4,
        operator: Ym2413Operator::Carrier,
        rhythm_voice: None,
    },
    Ym2413SlotDescriptor {
        slot_number: 12,
        channel: 5,
        operator: Ym2413Operator::Carrier,
        rhythm_voice: None,
    },
    Ym2413SlotDescriptor {
        slot_number: 13,
        channel: 6,
        operator: Ym2413Operator::Modulator,
        rhythm_voice: Some(Ym2413RhythmVoice::BassDrum),
    },
    Ym2413SlotDescriptor {
        slot_number: 14,
        channel: 7,
        operator: Ym2413Operator::Modulator,
        rhythm_voice: Some(Ym2413RhythmVoice::HiHat),
    },
    Ym2413SlotDescriptor {
        slot_number: 15,
        channel: 8,
        operator: Ym2413Operator::Modulator,
        rhythm_voice: Some(Ym2413RhythmVoice::TomTom),
    },
    Ym2413SlotDescriptor {
        slot_number: 16,
        channel: 6,
        operator: Ym2413Operator::Carrier,
        rhythm_voice: Some(Ym2413RhythmVoice::BassDrum),
    },
    Ym2413SlotDescriptor {
        slot_number: 17,
        channel: 7,
        operator: Ym2413Operator::Carrier,
        rhythm_voice: Some(Ym2413RhythmVoice::SnareDrum),
    },
    Ym2413SlotDescriptor {
        slot_number: 18,
        channel: 8,
        operator: Ym2413Operator::Carrier,
        rhythm_voice: Some(Ym2413RhythmVoice::TopCymbal),
    },
];

/// Resolve a zero-based serial slot index to its fixed channel/operator assignment.
pub const fn slot_descriptor(slot_index: u8) -> Option<Ym2413SlotDescriptor> {
    if (slot_index as usize) < YM2413_SLOT_COUNT {
        Some(YM2413_SLOT_MAP[slot_index as usize])
    } else {
        None
    }
}

/// Map Nuked-OPLL's serial `cycles` value to the selected logical operator
/// in the canonical Yamaha slot map.
///
/// The pinned Nuked schedule selects `ch_offset[cycle]` and
/// `mcsel=((cycle+1)/3)&1`. Across all eighteen cycles this is the
/// canonical `YM2413_SLOT_MAP` rotated by one position. This mapping is a
/// software-oracle compatibility contract, not a measured reset-phase claim.
pub const fn nuked_cycle_selected_slot_index(cycle: u8) -> Option<u8> {
    if cycle < YM2413_SLOT_COUNT as u8 {
        Some((cycle + 1) % YM2413_SLOT_COUNT as u8)
    } else {
        None
    }
}

/// Resolve the logical channel/operator selected by one Nuked serial cycle.
pub const fn nuked_cycle_selected_slot_descriptor(
    cycle: u8,
) -> Option<Ym2413SlotDescriptor> {
    match nuked_cycle_selected_slot_index(cycle) {
        Some(index) => slot_descriptor(index),
        None => None,
    }
}

/// PG phase-array entry written by Nuked `OPLL_PhaseGenerate` this cycle.
pub const fn nuked_pg_phase_write_index(cycle: u8) -> Option<u8> {
    if cycle < YM2413_SLOT_COUNT as u8 {
        Some((cycle + 17) % YM2413_SLOT_COUNT as u8)
    } else {
        None
    }
}

/// PG phase-array entry read to form the current `pg_out`.
pub const fn nuked_pg_phase_output_index(cycle: u8) -> Option<u8> {
    if cycle < YM2413_SLOT_COUNT as u8 {
        Some(cycle)
    } else {
        None
    }
}

/// EG operator-array entry updated by Nuked `OPLL_EnvelopeGenerate`.
pub const fn nuked_eg_update_index(cycle: u8) -> Option<u8> {
    if cycle < YM2413_SLOT_COUNT as u8 {
        Some((cycle + 16) % YM2413_SLOT_COUNT as u8)
    } else {
        None
    }
}

/// EG operator-array entry sampled by Nuked `OPLL_EnvelopeOutput`.
pub const fn nuked_eg_output_index(cycle: u8) -> Option<u8> {
    if cycle < YM2413_SLOT_COUNT as u8 {
        Some((cycle + 17) % YM2413_SLOT_COUNT as u8)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        YM2413_SLOT_COUNT, YM2413_SLOT_MAP, Ym2413Operator, Ym2413RhythmVoice, slot_descriptor,
    };

    #[test]
    fn serial_order_matches_yamaha_channel_and_operator_table() {
        let channels = [0, 1, 2, 0, 1, 2, 3, 4, 5, 3, 4, 5, 6, 7, 8, 6, 7, 8];
        let operators = [
            Ym2413Operator::Modulator,
            Ym2413Operator::Modulator,
            Ym2413Operator::Modulator,
            Ym2413Operator::Carrier,
            Ym2413Operator::Carrier,
            Ym2413Operator::Carrier,
            Ym2413Operator::Modulator,
            Ym2413Operator::Modulator,
            Ym2413Operator::Modulator,
            Ym2413Operator::Carrier,
            Ym2413Operator::Carrier,
            Ym2413Operator::Carrier,
            Ym2413Operator::Modulator,
            Ym2413Operator::Modulator,
            Ym2413Operator::Modulator,
            Ym2413Operator::Carrier,
            Ym2413Operator::Carrier,
            Ym2413Operator::Carrier,
        ];

        assert_eq!(YM2413_SLOT_MAP.len(), YM2413_SLOT_COUNT);
        for index in 0..YM2413_SLOT_COUNT {
            let descriptor = slot_descriptor(index as u8).unwrap();
            assert_eq!(descriptor.slot_number, index as u8 + 1);
            assert_eq!(descriptor.channel, channels[index]);
            assert_eq!(descriptor.operator, operators[index]);
        }
    }

    #[test]
    fn rhythm_mode_reassigns_exactly_the_six_documented_slots() {
        let expected = [
            Some(Ym2413RhythmVoice::BassDrum),
            Some(Ym2413RhythmVoice::HiHat),
            Some(Ym2413RhythmVoice::TomTom),
            Some(Ym2413RhythmVoice::BassDrum),
            Some(Ym2413RhythmVoice::SnareDrum),
            Some(Ym2413RhythmVoice::TopCymbal),
        ];
        for (offset, voice) in expected.into_iter().enumerate() {
            assert_eq!(
                slot_descriptor(12 + offset as u8).unwrap().rhythm_voice,
                voice
            );
        }
        assert!(
            YM2413_SLOT_MAP[..12]
                .iter()
                .all(|descriptor| descriptor.rhythm_voice.is_none())
        );
    }

    #[test]
    fn rhythm_output_map_matches_measured_original_and_repeat_slots() {
        use super::{YM2413_RHYTHM_OUTPUT_MAP, Ym2413RhythmVoice};

        let expected = [
            (0, Ym2413RhythmVoice::HiHat, true, 13),
            (1, Ym2413RhythmVoice::TomTom, true, 14),
            (2, Ym2413RhythmVoice::BassDrum, true, 15),
            (6, Ym2413RhythmVoice::SnareDrum, true, 16),
            (7, Ym2413RhythmVoice::TopCymbal, true, 17),
            (13, Ym2413RhythmVoice::HiHat, false, 13),
            (14, Ym2413RhythmVoice::TomTom, false, 14),
            (15, Ym2413RhythmVoice::BassDrum, false, 15),
            (16, Ym2413RhythmVoice::SnareDrum, false, 16),
            (17, Ym2413RhythmVoice::TopCymbal, false, 17),
        ];

        assert_eq!(YM2413_RHYTHM_OUTPUT_MAP.len(), expected.len());
        for (actual, (slot, voice, repeated, source)) in
            YM2413_RHYTHM_OUTPUT_MAP.into_iter().zip(expected)
        {
            assert_eq!(actual.slot_index, slot);
            assert_eq!(actual.voice, voice);
            assert_eq!(actual.is_delayed_repeat, repeated);
            assert_eq!(actual.source_slot_index, source);
        }
    }

    #[test]
    fn invalid_slot_indices_are_rejected_without_wrapping() {
        assert_eq!(slot_descriptor(18), None);
        assert_eq!(slot_descriptor(u8::MAX), None);
    }
    #[test]
    fn nuked_cycle_selection_maps_bijectively_to_the_yamaha_slot_map() {
        use crate::channel::{YM2413_EG_CYCLE_CHANNELS, envelope_cycle_is_carrier};

        let mut seen = [false; YM2413_SLOT_COUNT];
        for cycle in 0..YM2413_SLOT_COUNT as u8 {
            let slot_index = nuked_cycle_selected_slot_index(cycle).unwrap();
            let descriptor = nuked_cycle_selected_slot_descriptor(cycle).unwrap();
            let carrier = envelope_cycle_is_carrier(cycle).unwrap();

            assert_eq!(slot_index, (cycle + 1) % YM2413_SLOT_COUNT as u8);
            assert_eq!(descriptor, YM2413_SLOT_MAP[usize::from(slot_index)]);
            assert_eq!(descriptor.channel, YM2413_EG_CYCLE_CHANNELS[usize::from(cycle)]);
            assert_eq!(
                matches!(descriptor.operator, Ym2413Operator::Carrier),
                carrier
            );
            assert!(!seen[usize::from(slot_index)]);
            seen[usize::from(slot_index)] = true;
        }
        assert!(seen.into_iter().all(|selected| selected));
    }

    #[test]
    fn nuked_pg_and_eg_array_offsets_are_distinct_and_wrap_exactly() {
        for cycle in 0..YM2413_SLOT_COUNT as u8 {
            assert_eq!(
                nuked_pg_phase_write_index(cycle),
                Some((cycle + 17) % YM2413_SLOT_COUNT as u8)
            );
            assert_eq!(nuked_pg_phase_output_index(cycle), Some(cycle));
            assert_eq!(
                nuked_eg_update_index(cycle),
                Some((cycle + 16) % YM2413_SLOT_COUNT as u8)
            );
            assert_eq!(
                nuked_eg_output_index(cycle),
                Some((cycle + 17) % YM2413_SLOT_COUNT as u8)
            );
        }
        for invalid in [18, u8::MAX] {
            assert_eq!(nuked_cycle_selected_slot_index(invalid), None);
            assert_eq!(nuked_cycle_selected_slot_descriptor(invalid), None);
            assert_eq!(nuked_pg_phase_write_index(invalid), None);
            assert_eq!(nuked_pg_phase_output_index(invalid), None);
            assert_eq!(nuked_eg_update_index(invalid), None);
            assert_eq!(nuked_eg_output_index(invalid), None);
        }
    }

}
