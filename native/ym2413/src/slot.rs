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
pub const fn nuked_cycle_selected_slot_descriptor(cycle: u8) -> Option<Ym2413SlotDescriptor> {
    match nuked_cycle_selected_slot_index(cycle) {
        Some(index) => slot_descriptor(index),
        None => None,
    }
}

/// Resolve a Nuked PG phase-array index to its logical operator.
///
/// The PG ring is indexed by Nuked cycle order (`ch_offset`), which is the
/// canonical Yamaha slot map rotated by one position. It is not indexed by
/// `YM2413_SLOT_MAP` directly.
pub const fn nuked_pg_phase_slot_descriptor(index: u8) -> Option<Ym2413SlotDescriptor> {
    nuked_cycle_selected_slot_descriptor(index)
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

/// Return Nuked's phase-reset request for the PG next-phase latch.
///
/// Call with post-shift/post-mode-write `rm_enable`, pre-update EG
/// `eg_dokon`, and the current test-mode register. Test bit 2 requests a
/// reset independently of the selected operator's DOKON bit.
pub const fn nuked_phase_reset_required_with_testmode(
    cycle: u8,
    rm_enable: u8,
    eg_dokon: u32,
    testmode: u8,
) -> Option<bool> {
    if cycle >= YM2413_SLOT_COUNT as u8 {
        return None;
    }
    if (testmode & 0x04) != 0 {
        return Some(true);
    }

    let is_modulator = if (rm_enable & 0x40) != 0 && (cycle == 13 || cycle == 14) {
        false
    } else {
        ((cycle + 3) / 3) & 1 != 0
    };

    Some(if is_modulator {
        (eg_dokon & 0x8000) != 0
    } else {
        (eg_dokon & 0x01) != 0
    })
}

/// Return the normal-mode phase-reset request (test mode disabled).
pub const fn nuked_phase_reset_required(cycle: u8, rm_enable: u8, eg_dokon: u32) -> Option<bool> {
    nuked_phase_reset_required_with_testmode(cycle, rm_enable, eg_dokon, 0)
}

/// Immutable description of the distinct Nuked pipeline indices serviced
/// during one serial cycle. The EG sequencer remains owned by Ym2413Core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413SlotServiceEvent {
    pub cycle: u8,
    /// Current patch/EG operator selected by Nuked's channel and operator latches.
    pub selected_operator: Ym2413SlotDescriptor,
    /// PG accumulator entry updated with the previous call's PG latches.
    pub pg_phase_write_index: u8,
    /// PG phase entry consumed by the current PhaseGenerate/Operator stages.
    pub pg_phase_output_index: u8,
    /// Pre-update EG level consumed by the current Operator and advanced afterward.
    pub eg_update_index: u8,
    /// EG level entry sampled after Operator to produce the next call's eg_out latch.
    pub eg_output_index: u8,
    /// Reset request consumed only by the delayed PG next-phase latch.
    pub phase_reset_required: bool,
}

/// Build a service event from the post-shift mode state, pre-update EG
/// latches, and current test-mode register.
pub const fn nuked_slot_service_event_with_testmode(
    cycle: u8,
    rm_enable: u8,
    eg_dokon: u32,
    testmode: u8,
) -> Option<Ym2413SlotServiceEvent> {
    let selected_operator = match nuked_cycle_selected_slot_descriptor(cycle) {
        Some(descriptor) => descriptor,
        None => return None,
    };
    let pg_phase_write_index = match nuked_pg_phase_write_index(cycle) {
        Some(index) => index,
        None => return None,
    };
    let pg_phase_output_index = match nuked_pg_phase_output_index(cycle) {
        Some(index) => index,
        None => return None,
    };
    let eg_update_index = match nuked_eg_update_index(cycle) {
        Some(index) => index,
        None => return None,
    };
    let eg_output_index = match nuked_eg_output_index(cycle) {
        Some(index) => index,
        None => return None,
    };
    let phase_reset_required =
        match nuked_phase_reset_required_with_testmode(cycle, rm_enable, eg_dokon, testmode) {
            Some(required) => required,
            None => return None,
        };

    Some(Ym2413SlotServiceEvent {
        cycle,
        selected_operator,
        pg_phase_write_index,
        pg_phase_output_index,
        eg_update_index,
        eg_output_index,
        phase_reset_required,
    })
}

/// Build a normal-mode service event (test mode disabled).
pub const fn nuked_slot_service_event(
    cycle: u8,
    rm_enable: u8,
    eg_dokon: u32,
) -> Option<Ym2413SlotServiceEvent> {
    nuked_slot_service_event_with_testmode(cycle, rm_enable, eg_dokon, 0)
}

#[cfg(test)]
mod tests {
    use super::{
        YM2413_SLOT_COUNT, YM2413_SLOT_MAP, Ym2413Operator, Ym2413RhythmVoice,
        nuked_cycle_selected_slot_descriptor, nuked_cycle_selected_slot_index,
        nuked_eg_output_index, nuked_eg_update_index, nuked_pg_phase_output_index,
        nuked_pg_phase_write_index, nuked_phase_reset_required,
        nuked_phase_reset_required_with_testmode, nuked_slot_service_event,
        nuked_slot_service_event_with_testmode, slot_descriptor,
    };

    #[test]
    fn pg_phase_array_uses_nuked_cycle_order_not_canonical_slot_order() {
        let phase_15 = super::nuked_pg_phase_slot_descriptor(15).unwrap();
        assert_eq!(phase_15.channel, 7);
        assert_eq!(phase_15.operator, super::Ym2413Operator::Carrier);
        assert_eq!(super::slot_descriptor(15).unwrap().channel, 6);
        assert_eq!(
            super::slot_descriptor(15).unwrap().operator,
            super::Ym2413Operator::Carrier
        );
    }

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
            assert_eq!(
                descriptor.channel,
                YM2413_EG_CYCLE_CHANNELS[usize::from(cycle)]
            );
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

    #[test]
    fn nuked_phase_reset_uses_serial_dokon_bit_and_rhythm_override() {
        for cycle in 0..YM2413_SLOT_COUNT as u8 {
            let is_modulator = ((cycle + 3) / 3) & 1 != 0;
            assert_eq!(
                nuked_phase_reset_required(cycle, 0, 0x8000),
                Some(is_modulator),
                "modulator DOKON selector at cycle {cycle}"
            );
            assert_eq!(
                nuked_phase_reset_required(cycle, 0, 1),
                Some(!is_modulator),
                "carrier DOKON selector at cycle {cycle}"
            );
        }

        for cycle in [13, 14] {
            assert_eq!(nuked_phase_reset_required(cycle, 0x40, 0x8000), Some(false));
            assert_eq!(nuked_phase_reset_required(cycle, 0x40, 1), Some(true));
        }

        assert_eq!(nuked_phase_reset_required(18, 0, u32::MAX), None);
        assert_eq!(nuked_phase_reset_required(u8::MAX, 0, u32::MAX), None);
    }

    #[test]
    fn testmode_bit_two_forces_phase_reset_for_every_serial_cycle() {
        for cycle in 0..YM2413_SLOT_COUNT as u8 {
            assert_eq!(
                nuked_phase_reset_required_with_testmode(cycle, 0, 0, 0x04),
                Some(true),
                "test-mode phase reset at cycle {cycle}"
            );
            let event = nuked_slot_service_event_with_testmode(cycle, 0, 0, 0x04).unwrap();
            assert!(event.phase_reset_required);
        }
        assert_eq!(
            nuked_phase_reset_required_with_testmode(18, 0, 0, 0x04),
            None
        );
    }

    #[test]
    fn nuked_slot_service_event_keeps_all_pipeline_boundaries_explicit() {
        for cycle in 0..YM2413_SLOT_COUNT as u8 {
            let event = nuked_slot_service_event(cycle, 0, 0x8001).unwrap();
            assert_eq!(
                event.selected_operator,
                nuked_cycle_selected_slot_descriptor(cycle).unwrap()
            );
            assert_eq!(
                event.pg_phase_write_index,
                nuked_pg_phase_write_index(cycle).unwrap()
            );
            assert_eq!(
                event.pg_phase_output_index,
                nuked_pg_phase_output_index(cycle).unwrap()
            );
            assert_eq!(event.eg_update_index, nuked_eg_update_index(cycle).unwrap());
            assert_eq!(event.eg_output_index, nuked_eg_output_index(cycle).unwrap());
            assert_eq!(
                event.phase_reset_required,
                nuked_phase_reset_required(cycle, 0, 0x8001).unwrap()
            );
        }

        assert_eq!(nuked_slot_service_event(18, 0, 0), None);
    }
}
