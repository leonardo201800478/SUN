//! Isolated YM2413 rhythm phase-generation candidates.
//!
//! These helpers are pure combinational candidates. They do not advance PG,
//! EG, LFSR, slots, or the chip clock. Callers must provide the exact phase
//! indices at a separately established hardware boundary. The phase equations
//! below are not promoted to a YM2413 specification solely because a software
//! implementation reports a hardware comparison.

use crate::operator::YM2413_PHASE_INDEX_MASK;

/// Source used to formulate the Top Cymbal phase candidate.
///
/// Genesis Plus GX 1.7.4+git20221128-2 labels this phase equation as verified
/// on real YM2413 hardware. This remains a comparative software reference,
/// not an independent measurement supplied with this crate.
pub const YM2413_TOP_CYMBAL_PHASE_CANDIDATE_SOURCE: &str =
    "https://sources.debian.org/src/genesisplusgx/1.7.4+git20221128-2/core/sound/ym2413.c";

/// Errors returned by isolated rhythm phase candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhythmPhaseCandidateError {
    /// A supplied PG phase index exceeds the ten-bit operator phase domain.
    InvalidPhaseIndex { source: RhythmPhaseSource, value: u16 },
}

/// Identifies the operator phase input rejected by a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhythmPhaseSource {
    /// Modulator phase of hardware channel 8 (zero-based channel 7).
    Channel8Modulator,
    /// Carrier phase of hardware channel 9 (zero-based channel 8).
    Channel9Carrier,
}

/// Compute the candidate Top Cymbal waveform phase address.
///
/// Inputs are ten-bit phase-table indices from the channel 8 modulator and
/// channel 9 carrier. The result is a ten-bit phase address (0x100 or 0x300).
/// The equation uses bits 7, 3 and 2 of channel 8's modulator phase to select
/// the base phase, then bits 5 and 3 of channel 9's carrier phase as an enable
/// gate. It intentionally has no noise or clock input: the cited comparative
/// implementation's Top Cymbal path does not use the LFSR bit.
///
/// This function is only a combinational candidate. It does not define when
/// either phase is sampled, the PG/slot offset, envelope, ROM patch, output
/// routing, or whether this candidate matches every YM2413 revision.
pub fn top_cymbal_phase_candidate(
    channel8_modulator_phase: u16,
    channel9_carrier_phase: u16,
) -> Result<u16, RhythmPhaseCandidateError> {
    if channel8_modulator_phase > YM2413_PHASE_INDEX_MASK {
        return Err(RhythmPhaseCandidateError::InvalidPhaseIndex {
            source: RhythmPhaseSource::Channel8Modulator,
            value: channel8_modulator_phase,
        });
    }
    if channel9_carrier_phase > YM2413_PHASE_INDEX_MASK {
        return Err(RhythmPhaseCandidateError::InvalidPhaseIndex {
            source: RhythmPhaseSource::Channel9Carrier,
            value: channel9_carrier_phase,
        });
    }

    let phase_bit_7 = (channel8_modulator_phase >> 7) & 1;
    let phase_bit_3 = (channel8_modulator_phase >> 3) & 1;
    let phase_bit_2 = (channel8_modulator_phase >> 2) & 1;
    let base_select = (phase_bit_2 ^ phase_bit_7) | phase_bit_3;
    let base_phase = if base_select != 0 { 0x300 } else { 0x100 };

    let gate_bit_5 = (channel9_carrier_phase >> 5) & 1;
    let gate_bit_3 = (channel9_carrier_phase >> 3) & 1;
    let enabled = (gate_bit_5 | gate_bit_3) != 0;

    Ok(if enabled { 0x300 } else { base_phase })
}

#[cfg(test)]
mod tests {
    use super::{
        RhythmPhaseCandidateError, RhythmPhaseSource, top_cymbal_phase_candidate,
    };

    #[test]
    fn top_cymbal_candidate_exhaustively_covers_the_five_relevant_phase_bits() {
        for channel8_bits in 0..8_u16 {
            let channel8_phase = ((channel8_bits & 1) << 2)
                | (((channel8_bits >> 1) & 1) << 3)
                | (((channel8_bits >> 2) & 1) << 7);
            let base_select = ((channel8_phase >> 2) & 1)
                ^ ((channel8_phase >> 7) & 1)
                | ((channel8_phase >> 3) & 1);

            for channel9_bits in 0..4_u16 {
                let channel9_phase = ((channel9_bits & 1) << 3)
                    | (((channel9_bits >> 1) & 1) << 5);
                let enabled = ((channel9_phase >> 3) | (channel9_phase >> 5)) & 1 != 0;
                let expected = if enabled || base_select != 0 {
                    0x300
                } else {
                    0x100
                };

                assert_eq!(
                    top_cymbal_phase_candidate(channel8_phase, channel9_phase),
                    Ok(expected)
                );
            }
        }
    }

    #[test]
    fn top_cymbal_candidate_rejects_non_ten_bit_phase_indices() {
        assert_eq!(
            top_cymbal_phase_candidate(0x0400, 0),
            Err(RhythmPhaseCandidateError::InvalidPhaseIndex {
                source: RhythmPhaseSource::Channel8Modulator,
                value: 0x0400,
            })
        );
        assert_eq!(
            top_cymbal_phase_candidate(0, 0x0400),
            Err(RhythmPhaseCandidateError::InvalidPhaseIndex {
                source: RhythmPhaseSource::Channel9Carrier,
                value: 0x0400,
            })
        );
    }

    #[test]
    fn top_cymbal_candidate_ignores_phase_bits_not_in_the_documented_equation() {
        for channel8_phase in [0, 1, 0x20, 0x100, 0x200, 0x3ff] {
            for channel9_phase in [0, 1, 0x10, 0x100, 0x200, 0x3ff] {
                let expected = top_cymbal_phase_candidate(
                    channel8_phase & ((1 << 2) | (1 << 3) | (1 << 7)),
                    channel9_phase & ((1 << 3) | (1 << 5)),
                )
                .unwrap();
                assert_eq!(
                    top_cymbal_phase_candidate(channel8_phase, channel9_phase),
                    Ok(expected)
                );
            }
        }
    }
}
