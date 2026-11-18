//! Isolated YM2413 rhythm phase-generation candidates.
//!
//! The equations here are derived from the physical capture analysis in
//! andete's rhythm investigation and its 2020 LFSR update. They are pure
//! combinational functions: no PG, EG, LFSR, slot, or chip-clock state advances
//! here. The exact initial phase-counter offsets remain unresolved.

use crate::operator::YM2413_PHASE_INDEX_MASK;
use crate::rhythm::Ym2413RhythmNoiseSlotSample;
use crate::slot::Ym2413RhythmVoice;

/// Experimental source for the measured rhythm waveforms and phase equations.
pub const YM2413_RHYTHM_PHASE_CANDIDATE_SOURCE: &str =
    "https://github.com/andete/ym2413/blob/master/results/rhythm/rhythm.md";

/// Number of phase truth-table variants compatible with the published Top
/// Cymbal captures when the two PG initial offsets are left unconstrained.
pub const YM2413_TOP_CYMBAL_EQUIVALENT_PHASE_VARIANTS: usize = 16;

/// Errors returned by isolated rhythm phase candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhythmPhaseCandidateError {
    /// A supplied PG phase index exceeds the ten-bit operator phase domain.
    InvalidPhaseIndex {
        source: RhythmPhaseSource,
        value: u16,
    },
    /// The sampled voice, slot, and supplied phase-input variant do not match.
    UnexpectedNoiseSample {
        voice: Ym2413RhythmVoice,
        slot_index: u8,
    },
}

/// Identifies the operator phase input rejected by a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhythmPhaseSource {
    /// Modulator phase of hardware channel 7 (zero-based channel 6).
    Channel7Modulator,
    /// Carrier phase of hardware channel 8 (zero-based channel 7).
    Channel8Carrier,
}

fn validate_phase_inputs(
    channel7_modulator_phase: u16,
    channel8_carrier_phase: u16,
) -> Result<(), RhythmPhaseCandidateError> {
    if channel7_modulator_phase > YM2413_PHASE_INDEX_MASK {
        return Err(RhythmPhaseCandidateError::InvalidPhaseIndex {
            source: RhythmPhaseSource::Channel7Modulator,
            value: channel7_modulator_phase,
        });
    }
    if channel8_carrier_phase > YM2413_PHASE_INDEX_MASK {
        return Err(RhythmPhaseCandidateError::InvalidPhaseIndex {
            source: RhythmPhaseSource::Channel8Carrier,
            value: channel8_carrier_phase,
        });
    }
    Ok(())
}

/// Calculate the shared candidate phase bit used by Top Cymbal and Hi-Hat.
///
/// This is the OR-form candidate reported after the YM2413 capture analysis:
/// (m72 XOR m77) OR (m73 XOR c85) OR (c83 XOR c85).
/// The report explains that multiple Boolean forms fit the measured captures
/// because the absolute starting offsets of the two phase counters are not
/// known. This function therefore does not establish the physical reset phase.
pub fn rhythm_phase_mix_bit_candidate(
    channel7_modulator_phase: u16,
    channel8_carrier_phase: u16,
) -> Result<bool, RhythmPhaseCandidateError> {
    validate_phase_inputs(channel7_modulator_phase, channel8_carrier_phase)?;

    let m77 = (channel7_modulator_phase >> 7) & 1;
    let m73 = (channel7_modulator_phase >> 3) & 1;
    let m72 = (channel7_modulator_phase >> 2) & 1;
    let c85 = (channel8_carrier_phase >> 5) & 1;
    let c83 = (channel8_carrier_phase >> 3) & 1;

    Ok(((m72 ^ m77) | (m73 ^ c85) | (c83 ^ c85)) != 0)
}

/// Calculate the candidate Top Cymbal phase-table address.
///
/// The capture analysis and subsequent Nuke.YKT correction agree on a phase
/// address of 0x100 or 0x300, corresponding to full positive/negative
/// amplitude. This candidate uses the OR-form phase bit above and an explicit
/// 0x100 phase offset. The alternative equivalent truth tables and the
/// hardware's absolute PG starting offsets remain unresolved.
pub fn top_cymbal_phase_candidate(
    channel7_modulator_phase: u16,
    channel8_carrier_phase: u16,
) -> Result<u16, RhythmPhaseCandidateError> {
    let phase_bit =
        rhythm_phase_mix_bit_candidate(channel7_modulator_phase, channel8_carrier_phase)?;
    Ok((u16::from(phase_bit) << 9) | 0x100)
}

/// Calculate the candidate Hi-Hat phase-table address.
///
/// noise_bit is sampled state supplied by the caller. This function does not
/// advance or schedule the LFSR. The four possible phase addresses are
/// 0x034, 0x0d0, 0x234, and 0x2d0.
pub fn hi_hat_phase_candidate(
    channel7_modulator_phase: u16,
    channel8_carrier_phase: u16,
    noise_bit: bool,
) -> Result<u16, RhythmPhaseCandidateError> {
    let phase_bit =
        rhythm_phase_mix_bit_candidate(channel7_modulator_phase, channel8_carrier_phase)?;
    let low_phase = if phase_bit ^ noise_bit { 0x0d0 } else { 0x034 };
    Ok((u16::from(phase_bit) << 9) | low_phase)
}

/// Calculate the candidate Snare Drum phase-table address.
///
/// The phase bit comes from bit 8 of the channel 7 modulator PG output. The
/// caller supplies the LFSR output bit at the required slot boundary.
pub fn snare_drum_phase_candidate(
    channel7_modulator_phase: u16,
    noise_bit: bool,
) -> Result<u16, RhythmPhaseCandidateError> {
    if channel7_modulator_phase > YM2413_PHASE_INDEX_MASK {
        return Err(RhythmPhaseCandidateError::InvalidPhaseIndex {
            source: RhythmPhaseSource::Channel7Modulator,
            value: channel7_modulator_phase,
        });
    }

    let phase_bit = (channel7_modulator_phase >> 8) & 1;
    let noise_phase_bit = phase_bit ^ u16::from(noise_bit);
    Ok((phase_bit << 9) | (noise_phase_bit << 8))
}

/// Phase inputs required by the two rhythm voices that consume the LFSR bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhythmNoisePhaseInputs {
    /// Hi-Hat combines channel 7 modulator, channel 8 carrier and the sampled noise bit.
    HiHat {
        channel7_modulator_phase: u16,
        channel8_carrier_phase: u16,
    },
    /// Snare Drum combines channel 7 modulator phase and the sampled noise bit.
    SnareDrum {
        channel7_modulator_phase: u16,
    },
}

/// Convert a sequencer event into the corresponding candidate phase-table address.
///
/// This is a combinational bridge only: it validates the event's physical slot
/// and voice, then delegates to the existing candidate equation. It does not
/// advance PG, EG, LFSR, slots or chip time, and it does not claim that the
/// candidate's absolute phase offsets are hardware-verified.
pub fn rhythm_noise_phase_candidate(
    sample: Ym2413RhythmNoiseSlotSample,
    inputs: RhythmNoisePhaseInputs,
) -> Result<u16, RhythmPhaseCandidateError> {
    match (sample.voice, sample.slot_index, inputs) {
        (
            Ym2413RhythmVoice::HiHat,
            crate::rhythm::YM2413_RHYTHM_HIHAT_NOISE_SLOT_INDEX,
            RhythmNoisePhaseInputs::HiHat {
                channel7_modulator_phase,
                channel8_carrier_phase,
            },
        ) => hi_hat_phase_candidate(
            channel7_modulator_phase,
            channel8_carrier_phase,
            sample.noise_bit,
        ),
        (
            Ym2413RhythmVoice::SnareDrum,
            crate::rhythm::YM2413_RHYTHM_SNARE_NOISE_SLOT_INDEX,
            RhythmNoisePhaseInputs::SnareDrum {
                channel7_modulator_phase,
            },
        ) => snare_drum_phase_candidate(channel7_modulator_phase, sample.noise_bit),
        (voice, slot_index, _) => Err(RhythmPhaseCandidateError::UnexpectedNoiseSample {
            voice,
            slot_index,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RhythmNoisePhaseInputs, RhythmPhaseCandidateError, RhythmPhaseSource,
        hi_hat_phase_candidate, rhythm_noise_phase_candidate, rhythm_phase_mix_bit_candidate,
        snare_drum_phase_candidate, top_cymbal_phase_candidate,
    };


    #[test]
    fn sequencer_hihat_event_feeds_the_matching_phase_candidate() {
        let mut sequencer =
            crate::rhythm::Ym2413RhythmSlotNoiseSequencer::with_seed_at_slot(0x5a_1234, 13)
                .unwrap();
        let sample = sequencer.advance_slot().unwrap();
        let inputs = RhythmNoisePhaseInputs::HiHat {
            channel7_modulator_phase: 0x008c,
            channel8_carrier_phase: 0x0028,
        };

        assert_eq!(
            rhythm_noise_phase_candidate(sample, inputs),
            hi_hat_phase_candidate(0x008c, 0x0028, sample.noise_bit)
        );
    }

    #[test]
    fn sequencer_snare_event_feeds_the_matching_phase_candidate() {
        let mut sequencer =
            crate::rhythm::Ym2413RhythmSlotNoiseSequencer::with_seed_at_slot(0x5a_1234, 16)
                .unwrap();
        let sample = sequencer.advance_slot().unwrap();
        let inputs = RhythmNoisePhaseInputs::SnareDrum {
            channel7_modulator_phase: 0x0100,
        };

        assert_eq!(
            rhythm_noise_phase_candidate(sample, inputs),
            snare_drum_phase_candidate(0x0100, sample.noise_bit)
        );
    }

    #[test]
    fn noise_phase_bridge_rejects_mismatched_voice_slot_or_input_variant() {
        let hihat = crate::rhythm::Ym2413RhythmNoiseSlotSample {
            slot_index: crate::rhythm::YM2413_RHYTHM_HIHAT_NOISE_SLOT_INDEX,
            voice: Ym2413RhythmVoice::HiHat,
            noise_bit: true,
        };
        assert_eq!(
            rhythm_noise_phase_candidate(
                hihat,
                RhythmNoisePhaseInputs::SnareDrum {
                    channel7_modulator_phase: 0,
                }
            ),
            Err(RhythmPhaseCandidateError::UnexpectedNoiseSample {
                voice: Ym2413RhythmVoice::HiHat,
                slot_index: crate::rhythm::YM2413_RHYTHM_HIHAT_NOISE_SLOT_INDEX,
            })
        );

        let invalid_slot = crate::rhythm::Ym2413RhythmNoiseSlotSample {
            slot_index: 12,
            ..hihat
        };
        assert_eq!(
            rhythm_noise_phase_candidate(
                invalid_slot,
                RhythmNoisePhaseInputs::HiHat {
                    channel7_modulator_phase: 0,
                    channel8_carrier_phase: 0,
                }
            ),
            Err(RhythmPhaseCandidateError::UnexpectedNoiseSample {
                voice: Ym2413RhythmVoice::HiHat,
                slot_index: 12,
            })
        );
    }

    #[test]
    fn rhythm_phase_mix_candidate_exhaustively_covers_the_five_relevant_bits() {
        for mod_bits in 0..8_u16 {
            let mod_phase =
                ((mod_bits & 1) << 2) | (((mod_bits >> 1) & 1) << 3) | (((mod_bits >> 2) & 1) << 7);

            for car_bits in 0..4_u16 {
                let car_phase = ((car_bits & 1) << 3) | (((car_bits >> 1) & 1) << 5);
                let m77 = (mod_phase >> 7) & 1;
                let m73 = (mod_phase >> 3) & 1;
                let m72 = (mod_phase >> 2) & 1;
                let c85 = (car_phase >> 5) & 1;
                let c83 = (car_phase >> 3) & 1;
                let expected = ((m72 ^ m77) | (m73 ^ c85) | (c83 ^ c85)) != 0;

                assert_eq!(
                    rhythm_phase_mix_bit_candidate(mod_phase, car_phase),
                    Ok(expected)
                );
                assert_eq!(
                    top_cymbal_phase_candidate(mod_phase, car_phase),
                    Ok(if expected { 0x300 } else { 0x100 })
                );

                for noise_bit in [false, true] {
                    let low_phase = if expected ^ noise_bit { 0x0d0 } else { 0x034 };
                    assert_eq!(
                        hi_hat_phase_candidate(mod_phase, car_phase, noise_bit),
                        Ok((u16::from(expected) << 9) | low_phase)
                    );
                }
            }
        }
    }

    #[test]
    fn snare_drum_candidate_covers_all_phase_and_noise_combinations() {
        for phase_bit in [0_u16, 1] {
            for noise_bit in [false, true] {
                let phase = phase_bit << 8;
                let expected = (phase_bit << 9) | ((phase_bit ^ u16::from(noise_bit)) << 8);
                assert_eq!(snare_drum_phase_candidate(phase, noise_bit), Ok(expected));
            }
        }
    }

    #[test]
    fn rhythm_phase_candidates_reject_non_ten_bit_phase_indices() {
        assert_eq!(
            rhythm_phase_mix_bit_candidate(0x0400, 0),
            Err(RhythmPhaseCandidateError::InvalidPhaseIndex {
                source: RhythmPhaseSource::Channel7Modulator,
                value: 0x0400,
            })
        );
        assert_eq!(
            rhythm_phase_mix_bit_candidate(0, 0x0400),
            Err(RhythmPhaseCandidateError::InvalidPhaseIndex {
                source: RhythmPhaseSource::Channel8Carrier,
                value: 0x0400,
            })
        );
        assert_eq!(
            snare_drum_phase_candidate(0x0400, false),
            Err(RhythmPhaseCandidateError::InvalidPhaseIndex {
                source: RhythmPhaseSource::Channel7Modulator,
                value: 0x0400,
            })
        );
    }

    #[test]
    fn phase_candidates_ignore_bits_not_used_by_their_equations() {
        for channel7_phase in [0, 1, 0x10, 0x20, 0x100, 0x200, 0x3ff] {
            for channel8_phase in [0, 1, 0x10, 0x20, 0x100, 0x200, 0x3ff] {
                let masked_channel7 = channel7_phase & ((1 << 2) | (1 << 3) | (1 << 7));
                let masked_channel8 = channel8_phase & ((1 << 3) | (1 << 5));

                for noise_bit in [false, true] {
                    assert_eq!(
                        hi_hat_phase_candidate(channel7_phase, channel8_phase, noise_bit),
                        hi_hat_phase_candidate(masked_channel7, masked_channel8, noise_bit)
                    );
                }
                assert_eq!(
                    top_cymbal_phase_candidate(channel7_phase, channel8_phase),
                    top_cymbal_phase_candidate(masked_channel7, masked_channel8)
                );
            }
        }
    }
}
