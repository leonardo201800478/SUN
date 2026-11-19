//! Aggregate digital state for the YM2413's nine melodic FM channels.
//!
//! This bank owns eighteen PGs, eighteen EGs, nine feedback pipelines, and
//! chip-wide AM/PM LFO counters. It does not own input-clock progression,
//! slot ordering, the global EG counter, or PCM cadence. The caller supplies
//! every operator update at its externally established boundary. Reset
//! phases and EG startup states are explicit because absolute hardware
//! alignment has not been established.

use crate::am_lfo::{AmLfo, AmLfoError, AmLfoState, YM2413_AM_LFO_PERIOD_SAMPLES};
use crate::channel::{ChannelSynthesisConfig, OperatorPhaseParameters};
use crate::envelope::{
    DampTransitionConfig, EnvelopeGenerator, EnvelopeGeneratorError, EnvelopeGeneratorState,
    EnvelopeParameters, EnvelopeStage, EnvelopeTickResult,
};
use crate::operator::{OperatorPair, OperatorPairError, OperatorPairSample, OperatorPairState};
use crate::phase::{PhaseGenerator, PhaseGeneratorError, PhaseGeneratorState};
use crate::pm_lfo::{PhaseModulationLfo, PhaseModulationLfoError, PhaseModulationLfoState};
use crate::rhythm_channel::{
    RhythmOperatorSynthesisConfig, RhythmVoiceSynthesisError, Ym2413RhythmSynthesisConfig,
};
use crate::slot::{Ym2413Operator, Ym2413RhythmVoice};

/// Number of melodic channels in the YM2413.
pub const YM2413_MELODIC_CHANNELS: usize = 9;
/// Two independent operators are retained for every melodic channel.
pub const YM2413_OPERATORS_PER_CHANNEL: usize = 2;
/// Version of the aggregate digital synthesis snapshot.
pub const YM2413_SYNTHESIS_STATE_VERSION: u16 = 2;

/// Errors returned by the aggregate synthesis bank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413SynthesisError {
    /// Channel index is outside the nine melodic channels.
    InvalidChannel(u8),
    /// A PG update rejected its register-derived inputs or state.
    Phase(PhaseGeneratorError),
    /// An EG update rejected its parameters or state.
    Envelope(EnvelopeGeneratorError),
    /// The FM operator pair rejected its phase, attenuation, or state.
    Operator(OperatorPairError),
    /// The PM LFO rejected its initial or restored counter.
    PhaseModulationLfo(PhaseModulationLfoError),
    /// The AM LFO rejected its initial or restored phase.
    AmLfo(AmLfoError),
    /// Rhythm Mode is not enabled in the supplied register configuration.
    RhythmModeDisabled,
    /// The rhythm voice configuration is invalid for the selected evaluator.
    RhythmVoice(RhythmVoiceSynthesisError),
    /// Aggregate snapshot version or contents are invalid.
    InvalidState,
}

impl From<PhaseGeneratorError> for Ym2413SynthesisError {
    fn from(value: PhaseGeneratorError) -> Self {
        Self::Phase(value)
    }
}

impl From<EnvelopeGeneratorError> for Ym2413SynthesisError {
    fn from(value: EnvelopeGeneratorError) -> Self {
        Self::Envelope(value)
    }
}

impl From<OperatorPairError> for Ym2413SynthesisError {
    fn from(value: OperatorPairError) -> Self {
        Self::Operator(value)
    }
}

impl From<PhaseModulationLfoError> for Ym2413SynthesisError {
    fn from(value: PhaseModulationLfoError) -> Self {
        Self::PhaseModulationLfo(value)
    }
}

impl From<AmLfoError> for Ym2413SynthesisError {
    fn from(value: AmLfoError) -> Self {
        Self::AmLfo(value)
    }
}

impl From<RhythmVoiceSynthesisError> for Ym2413SynthesisError {
    fn from(value: RhythmVoiceSynthesisError) -> Self {
        Self::RhythmVoice(value)
    }
}

/// One logical Rhythm Mode voice's current digital output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmVoiceSample {
    /// Logical percussion voice that was evaluated.
    pub voice: Ym2413RhythmVoice,
    /// Signed digital output from the shared operator synthesis path.
    pub output: i16,
}

/// Explicit phase-table addresses for one complete Rhythm Mode evaluation.
///
/// Phase generation remains caller-owned: HH, SD and TCY require the
/// candidate phase combiners, while TOM uses its channel-9 modulator PG.
/// Bass Drum derives both operator phases from the shared channel-7 pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413RhythmVoicePhases {
    pub hi_hat: u16,
    pub snare_drum: u16,
    pub tom_tom: u16,
    pub top_cymbal: u16,
}

/// Explicit startup state for one melodic channel's two operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413ChannelInitialState {
    /// Initial 19-bit PG accumulators: [modulator, carrier].
    pub phase: [u32; YM2413_OPERATORS_PER_CHANNEL],
    /// Initial seven-bit EG attenuation: [modulator, carrier].
    pub envelope_level: [u8; YM2413_OPERATORS_PER_CHANNEL],
    /// Initial EG stages: [modulator, carrier].
    pub envelope_stage: [EnvelopeStage; YM2413_OPERATORS_PER_CHANNEL],
    /// Initial EG key state: [modulator, carrier].
    pub envelope_key_on: [bool; YM2413_OPERATORS_PER_CHANNEL],
    /// Initial two-sample modulator feedback history [p0, p1].
    pub feedback_history: [i16; 2],
    /// Explicit phase-table index from the modulator's previous output sample.
    /// None leaves the first-sample condition unspecified and uses current phase.
    pub previous_modulator_phase_index: Option<u16>,
}

/// Explicit initial conditions for the aggregate YM2413 digital model.
///
/// No PG phase, EG level/stage, feedback history, or LFO phase is guessed.
/// The caller must choose these values from a documented reset hypothesis or
/// a measured startup trace. The DAMP policy is likewise explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413SynthesisInitialState {
    /// Initial state for channels 1 through 9.
    pub channels: [Ym2413ChannelInitialState; YM2413_MELODIC_CHANNELS],
    /// Explicit 13-bit initial PM LFO counter.
    pub pm_lfo_counter: u16,
    /// Explicit AM LFO sample phase within its measured cycle.
    pub am_lfo_phase: u16,
    /// Candidate DAMP transition policy used by all eighteen EGs.
    pub damp_config: DampTransitionConfig,
}

/// Snapshot of one channel's two PGs, two EGs, and feedback pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413ChannelSynthesisState {
    pub modulator_phase: PhaseGeneratorState,
    pub carrier_phase: PhaseGeneratorState,
    pub modulator_envelope: EnvelopeGeneratorState,
    pub carrier_envelope: EnvelopeGeneratorState,
    pub operator_pair: OperatorPairState,
}

/// Complete aggregate digital synthesis snapshot.
///
/// The input-clock foundation remains a separate owner. This snapshot
/// captures every state field owned by this synthesis bank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413SynthesisBankState {
    pub format_version: u16,
    pub channels: [Ym2413ChannelSynthesisState; YM2413_MELODIC_CHANNELS],
    pub pm_lfo: PhaseModulationLfoState,
    pub am_lfo: AmLfoState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ym2413ChannelSynthesis {
    phase: [PhaseGenerator; YM2413_OPERATORS_PER_CHANNEL],
    envelope: [EnvelopeGenerator; YM2413_OPERATORS_PER_CHANNEL],
    operator_pair: OperatorPair,
}

impl Ym2413ChannelSynthesis {
    /// Build one channel from caller-supplied initial conditions.
    fn from_initial(
        initial: Ym2413ChannelInitialState,
        damp_config: DampTransitionConfig,
    ) -> Result<Self, Ym2413SynthesisError> {
        let phase = [
            PhaseGenerator::restore_state(PhaseGeneratorState {
                format_version: crate::phase::YM2413_PG_STATE_VERSION,
                phase: initial.phase[0],
            })?,
            PhaseGenerator::restore_state(PhaseGeneratorState {
                format_version: crate::phase::YM2413_PG_STATE_VERSION,
                phase: initial.phase[1],
            })?,
        ];
        let envelope = [
            EnvelopeGenerator::with_initial_state(
                initial.envelope_level[0],
                initial.envelope_stage[0],
                initial.envelope_key_on[0],
                damp_config,
            )?,
            EnvelopeGenerator::with_initial_state(
                initial.envelope_level[1],
                initial.envelope_stage[1],
                initial.envelope_key_on[1],
                damp_config,
            )?,
        ];
        Ok(Self {
            phase,
            envelope,
            operator_pair: OperatorPair::with_initial_state(
                initial.feedback_history,
                initial.previous_modulator_phase_index,
            )?,
        })
    }

    /// Serialize all local operator and feedback state.
    const fn export_state(self) -> Ym2413ChannelSynthesisState {
        Ym2413ChannelSynthesisState {
            modulator_phase: self.phase[0].export_state(),
            carrier_phase: self.phase[1].export_state(),
            modulator_envelope: self.envelope[0].export_state(),
            carrier_envelope: self.envelope[1].export_state(),
            operator_pair: self.operator_pair.export_state(),
        }
    }

    /// Restore one channel after validating every nested component snapshot.
    fn restore_state(state: Ym2413ChannelSynthesisState) -> Result<Self, Ym2413SynthesisError> {
        Ok(Self {
            phase: [
                PhaseGenerator::restore_state(state.modulator_phase)?,
                PhaseGenerator::restore_state(state.carrier_phase)?,
            ],
            envelope: [
                EnvelopeGenerator::restore_state(state.modulator_envelope)?,
                EnvelopeGenerator::restore_state(state.carrier_envelope)?,
            ],
            operator_pair: OperatorPair::restore_state(state.operator_pair)?,
        })
    }
}

/// Aggregate state for all nine melodic channels and both chip-wide LFOs.
///
/// The bank is a digital synthesis-state owner, not a slot scheduler. Its
/// update methods are intentionally per-operator so a future machine adapter
/// can supply the researched slot order without this type inventing one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ym2413SynthesisBank {
    channels: [Ym2413ChannelSynthesis; YM2413_MELODIC_CHANNELS],
    pm_lfo: PhaseModulationLfo,
    am_lfo: AmLfo,
}

impl Ym2413SynthesisBank {
    /// Construct all eighteen operators from explicit, validated initial state.
    pub fn with_initial_state(
        initial: Ym2413SynthesisInitialState,
    ) -> Result<Self, Ym2413SynthesisError> {
        let mut channels = Vec::with_capacity(YM2413_MELODIC_CHANNELS);
        for channel in initial.channels {
            channels.push(Ym2413ChannelSynthesis::from_initial(
                channel,
                initial.damp_config,
            )?);
        }
        let channels: [Ym2413ChannelSynthesis; YM2413_MELODIC_CHANNELS] = channels
            .try_into()
            .map_err(|_| Ym2413SynthesisError::InvalidState)?;

        Ok(Self {
            channels,
            pm_lfo: PhaseModulationLfo::new(initial.pm_lfo_counter)?,
            am_lfo: AmLfo::with_phase(initial.am_lfo_phase)?,
        })
    }

    /// Return the current global PM table position for the next PG update.
    pub const fn pm_position(&self) -> u8 {
        self.pm_lfo.position()
    }

    /// Return the current global AM attenuation level (0..13).
    pub const fn am_level(&self) -> u8 {
        self.am_lfo.current_value()
    }

    /// Advance both LFOs by externally completed YM2413 output samples.
    ///
    /// The caller must invoke this exactly at its established output-sample
    /// boundary. This method does not advance input clocks, slots, or EG time.
    pub fn advance_lfo_samples(&mut self, samples: u64) {
        self.pm_lfo.advance_samples(samples);
        let am_samples = samples % u64::from(YM2413_AM_LFO_PERIOD_SAMPLES);
        self.am_lfo.advance_samples(am_samples as u32);
    }

    /// Advance one selected operator PG and return its ten-bit sine-table index.
    ///
    /// The caller supplies the decoded operator multiplier. The bank reads
    /// the current global PM position but does not infer a slot order.
    pub fn advance_operator_phase(
        &mut self,
        channel: u8,
        carrier: bool,
        parameters: OperatorPhaseParameters,
    ) -> Result<u16, Ym2413SynthesisError> {
        let pm_position = self.pm_lfo.position();
        let state = self.channel_mut(channel)?;
        let operator_index = if carrier { 1 } else { 0 };
        let phase = &mut state.phase[operator_index];
        phase.advance(
            parameters.fnum,
            parameters.block,
            parameters.multiplier,
            parameters.pm_enabled,
            pm_position,
        )?;
        Ok(phase.phase_index())
    }

    /// Tick one selected operator EG using the caller-owned global EG counter.
    ///
    /// The global counter is passed through unchanged; this bank does not
    /// choose its cadence or increment it. The carrier flag selects the
    /// standalone EG candidate's carrier release path.
    pub fn tick_operator_envelope(
        &mut self,
        channel: u8,
        carrier: bool,
        key_on: bool,
        global_counter: u32,
        parameters: EnvelopeParameters,
    ) -> Result<u8, Ym2413SynthesisError> {
        self.tick_operator_envelope_with_events(
            channel,
            carrier,
            key_on,
            global_counter,
            parameters,
        )
        .map(|result| result.level)
    }

    /// Tick one EG and expose candidate DAMP-completion phase-reset events.
    ///
    /// This method does not reset either PG. The caller must consume
    /// `phase_reset_required` at the selected update boundary. If the channel
    /// uses `CarrierDrivesBoth`, synchronize the modulator EG and reset both
    /// PGs when the carrier reports the event. No slot ordering is inferred.
    pub fn tick_operator_envelope_with_events(
        &mut self,
        channel: u8,
        carrier: bool,
        key_on: bool,
        global_counter: u32,
        parameters: EnvelopeParameters,
    ) -> Result<EnvelopeTickResult, Ym2413SynthesisError> {
        let state = self.channel_mut(channel)?;
        let operator_index = if carrier { 1 } else { 0 };
        Ok(state.envelope[operator_index].tick_with_events(
            global_counter,
            key_on,
            carrier,
            parameters,
        )?)
    }

    /// Tick EG for one physical rhythm operator using an explicit key state.
    ///
    /// The key state and carrier-release policy are deliberately supplied by
    /// the caller: trigger/KEY-ON interaction and whether a single-operator
    /// voice uses carrier EG release semantics are not inferred here. The
    /// global counter and service boundary are also caller-owned.
    pub fn tick_rhythm_operator_envelope(
        &mut self,
        operator: &RhythmOperatorSynthesisConfig,
        key_on: bool,
        carrier_release_path: bool,
        global_counter: u32,
    ) -> Result<u8, Ym2413SynthesisError> {
        self.tick_operator_envelope(
            operator.physical_channel,
            carrier_release_path,
            key_on,
            global_counter,
            operator.envelope,
        )
    }

    /// Evaluate one Rhythm Mode voice from the current PG/EG states.
    ///
    /// This does not advance PG, EG, LFO, LFSR, slots or chip time. The caller
    /// must first service the relevant physical operators at their researched
    /// boundaries. Rhythm trigger bits are not treated as a mute gate here:
    /// EG state controls the current amplitude and may represent a release tail.
    /// BD uses the existing channel-7 OperatorPair and therefore preserves its
    /// feedback history; the other voices use the shared single-carrier path.
    pub fn evaluate_rhythm_voice(
        &mut self,
        configuration: &Ym2413RhythmSynthesisConfig,
        voice: Ym2413RhythmVoice,
        phase_index: Option<u16>,
    ) -> Result<Ym2413RhythmVoiceSample, Ym2413SynthesisError> {
        if !configuration.rhythm_mode_enabled {
            return Err(Ym2413SynthesisError::RhythmModeDisabled);
        }

        let voice_config = match voice {
            Ym2413RhythmVoice::BassDrum => &configuration.voices[0],
            Ym2413RhythmVoice::HiHat => &configuration.voices[1],
            Ym2413RhythmVoice::SnareDrum => &configuration.voices[2],
            Ym2413RhythmVoice::TomTom => &configuration.voices[3],
            Ym2413RhythmVoice::TopCymbal => &configuration.voices[4],
        };
        if voice_config.voice != voice {
            return Err(RhythmVoiceSynthesisError::InvalidOperatorLayout { voice }.into());
        }
        let am_level = self.am_lfo.current_value();

        let output = if voice == Ym2413RhythmVoice::BassDrum {
            if phase_index.is_some() {
                return Err(RhythmVoiceSynthesisError::UnexpectedPhaseInput { voice }.into());
            }
            let (Some(modulator), Some(carrier)) = (voice_config.modulator, voice_config.carrier)
            else {
                return Err(RhythmVoiceSynthesisError::InvalidOperatorLayout { voice }.into());
            };
            if voice_config.single_operator_uses_carrier_path
                || modulator.physical_role != Ym2413Operator::Modulator
                || carrier.physical_role != Ym2413Operator::Carrier
                || modulator.physical_channel != carrier.physical_channel
            {
                return Err(RhythmVoiceSynthesisError::InvalidOperatorLayout { voice }.into());
            }
            let state = self.channel_mut(modulator.physical_channel)?;
            let parameters = voice_config.bass_drum_operator_pair_parameters(
                state.envelope[0].level(),
                state.envelope[1].level(),
                am_level,
            )?;
            state
                .operator_pair
                .evaluate(
                    state.phase[0].phase_index(),
                    state.phase[1].phase_index(),
                    parameters,
                )?
                .carrier_output
        } else {
            let phase_index =
                phase_index.ok_or(RhythmVoiceSynthesisError::MissingPhaseInput { voice })?;
            let (operator, expected_role) = match (voice_config.modulator, voice_config.carrier) {
                (Some(operator), None) => (operator, Ym2413Operator::Modulator),
                (None, Some(operator)) => (operator, Ym2413Operator::Carrier),
                _ => {
                    return Err(RhythmVoiceSynthesisError::InvalidOperatorLayout { voice }.into());
                }
            };
            if operator.physical_role != expected_role {
                return Err(RhythmVoiceSynthesisError::InvalidOperatorLayout { voice }.into());
            }
            if !voice_config.single_operator_uses_carrier_path {
                return Err(RhythmVoiceSynthesisError::InvalidOperatorLayout { voice }.into());
            }
            let carrier = operator.physical_role == Ym2413Operator::Carrier;
            let state = self.channel(operator.physical_channel)?;
            let operator_index = if carrier { 1 } else { 0 };
            voice_config.evaluate_single_operator(
                phase_index,
                state.envelope[operator_index].level(),
                am_level,
            )?
        };

        Ok(Ym2413RhythmVoiceSample { voice, output })
    }

    /// Evaluate all five Rhythm Mode voices as one logical output batch.
    ///
    /// The four single-operator voices are evaluated first because they are
    /// pure reads of the current PG/EG state. Bass Drum is evaluated last
    /// because its two-operator path advances channel-7 feedback history.
    /// The returned array uses stable logical order BD, HH, SD, TOM, TCY;
    /// this order is not a claim about physical slot execution or DAC timing.
    ///
    /// No PG, EG, LFO, LFSR, slot, or chip-clock state advances here. The
    /// caller must provide phase addresses and service every state machine
    /// at independently established hardware boundaries.
    pub fn evaluate_rhythm_voices(
        &mut self,
        configuration: &Ym2413RhythmSynthesisConfig,
        phases: Ym2413RhythmVoicePhases,
    ) -> Result<[Ym2413RhythmVoiceSample; 5], Ym2413SynthesisError> {
        if !configuration.rhythm_mode_enabled {
            return Err(Ym2413SynthesisError::RhythmModeDisabled);
        }

        let hi_hat = self.evaluate_rhythm_voice(
            configuration,
            Ym2413RhythmVoice::HiHat,
            Some(phases.hi_hat),
        )?;
        let snare_drum = self.evaluate_rhythm_voice(
            configuration,
            Ym2413RhythmVoice::SnareDrum,
            Some(phases.snare_drum),
        )?;
        let tom_tom = self.evaluate_rhythm_voice(
            configuration,
            Ym2413RhythmVoice::TomTom,
            Some(phases.tom_tom),
        )?;
        let top_cymbal = self.evaluate_rhythm_voice(
            configuration,
            Ym2413RhythmVoice::TopCymbal,
            Some(phases.top_cymbal),
        )?;

        // Bass Drum is the only evaluator in this batch that mutates state.
        // Keep it last so a rejected phase/layout in another voice cannot
        // leave the channel-7 feedback pipeline partially advanced.
        let bass_drum =
            self.evaluate_rhythm_voice(configuration, Ym2413RhythmVoice::BassDrum, None)?;

        Ok([bass_drum, hi_hat, snare_drum, tom_tom, top_cymbal])
    }

    /// Apply the explicit CarrierDrivesBoth DAMP candidate after a carrier tick.
    ///
    /// This operation is never implicit: the exact modulator/carrier transition
    /// offset is unresolved and must be selected by the caller's model.
    pub fn synchronize_modulator_damp_transition(
        &mut self,
        channel: u8,
    ) -> Result<(), Ym2413SynthesisError> {
        let state = self.channel_mut(channel)?;
        let carrier = state.envelope[1];
        state.envelope[0].synchronize_damp_transition(&carrier);
        Ok(())
    }

    /// Evaluate one channel using the PG and EG states most recently supplied.
    ///
    /// This method does not advance either operator. The caller must invoke PG
    /// and EG updates at the correct slot boundaries before evaluating the
    /// pair. The returned carrier output is one channel's candidate digital
    /// sample, not a mixed YM2413 PCM frame.
    pub fn evaluate_channel(
        &mut self,
        configuration: &ChannelSynthesisConfig,
    ) -> Result<OperatorPairSample, Ym2413SynthesisError> {
        let channel = configuration.channel.channel;
        let am_level = self.am_lfo.current_value();
        let state = self.channel_mut(channel)?;
        let parameters = configuration.operator_pair_parameters(
            state.envelope[0].level(),
            state.envelope[1].level(),
            am_level,
        )?;
        Ok(state.operator_pair.evaluate(
            state.phase[0].phase_index(),
            state.phase[1].phase_index(),
            parameters,
        )?)
    }

    /// Return one operator's current PG phase index.
    pub fn phase_index(&self, channel: u8, carrier: bool) -> Result<u16, Ym2413SynthesisError> {
        let state = self.channel(channel)?;
        let operator_index = if carrier { 1 } else { 0 };
        Ok(state.phase[operator_index].phase_index())
    }

    /// Return one operator's current EG attenuation level.
    pub fn envelope_level(&self, channel: u8, carrier: bool) -> Result<u8, Ym2413SynthesisError> {
        let state = self.channel(channel)?;
        let operator_index = if carrier { 1 } else { 0 };
        Ok(state.envelope[operator_index].level())
    }

    /// Return the feedback pipeline for one channel.
    pub fn feedback_history(&self, channel: u8) -> Result<[i16; 2], Ym2413SynthesisError> {
        Ok(self.channel(channel)?.operator_pair.feedback_history())
    }

    /// Export all eighteen operator states and both global LFO states.
    pub const fn export_state(&self) -> Ym2413SynthesisBankState {
        Ym2413SynthesisBankState {
            format_version: YM2413_SYNTHESIS_STATE_VERSION,
            channels: [
                self.channels[0].export_state(),
                self.channels[1].export_state(),
                self.channels[2].export_state(),
                self.channels[3].export_state(),
                self.channels[4].export_state(),
                self.channels[5].export_state(),
                self.channels[6].export_state(),
                self.channels[7].export_state(),
                self.channels[8].export_state(),
            ],
            pm_lfo: self.pm_lfo.export_state(),
            am_lfo: self.am_lfo.export_state(),
        }
    }

    /// Restore the aggregate bank after validating every nested snapshot.
    pub fn restore_state(state: Ym2413SynthesisBankState) -> Result<Self, Ym2413SynthesisError> {
        if state.format_version != YM2413_SYNTHESIS_STATE_VERSION {
            return Err(Ym2413SynthesisError::InvalidState);
        }
        let mut channels = Vec::with_capacity(YM2413_MELODIC_CHANNELS);
        for channel in state.channels {
            channels.push(Ym2413ChannelSynthesis::restore_state(channel)?);
        }
        let channels: [Ym2413ChannelSynthesis; YM2413_MELODIC_CHANNELS] = channels
            .try_into()
            .map_err(|_| Ym2413SynthesisError::InvalidState)?;
        Ok(Self {
            channels,
            pm_lfo: PhaseModulationLfo::restore_state(state.pm_lfo)?,
            am_lfo: AmLfo::restore_state(state.am_lfo)?,
        })
    }

    /// Borrow a channel after validating its index.
    fn channel(&self, channel: u8) -> Result<&Ym2413ChannelSynthesis, Ym2413SynthesisError> {
        self.channels
            .get(usize::from(channel))
            .ok_or(Ym2413SynthesisError::InvalidChannel(channel))
    }

    /// Mutably borrow a channel after validating its index.
    fn channel_mut(
        &mut self,
        channel: u8,
    ) -> Result<&mut Ym2413ChannelSynthesis, Ym2413SynthesisError> {
        self.channels
            .get_mut(usize::from(channel))
            .ok_or(Ym2413SynthesisError::InvalidChannel(channel))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Ym2413ChannelInitialState, Ym2413SynthesisBank, Ym2413SynthesisBankState,
        Ym2413SynthesisError, Ym2413SynthesisInitialState,
    };
    use crate::channel::decode_channel_synthesis;
    use crate::envelope::{
        AttackRate15Transition, DampTransitionConfig, DampTransitionCoordination, EnvelopeStage,
    };
    use crate::rhythm_channel::decode_rhythm_synthesis;
    use crate::slot::{Ym2413Operator, Ym2413RhythmVoice};

    fn initial_state() -> Ym2413SynthesisInitialState {
        Ym2413SynthesisInitialState {
            channels: std::array::from_fn(|channel| Ym2413ChannelInitialState {
                phase: [channel as u32 * 0x1000, channel as u32 * 0x1800],
                envelope_level: [127, 127],
                envelope_stage: [EnvelopeStage::Damp; 2],
                envelope_key_on: [false; 2],
                feedback_history: [0; 2],
                previous_modulator_phase_index: None,
            }),
            pm_lfo_counter: 0,
            am_lfo_phase: 0,
            damp_config: DampTransitionConfig::new(
                124,
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            )
            .unwrap(),
        }
    }

    fn configuration(channel: u8) -> crate::channel::ChannelSynthesisConfig {
        let mut registers = [0; 64];
        registers[..8].copy_from_slice(&[0x71, 0x61, 0x1e, 0x17, 0xd0, 0x78, 0, 0x17]);
        let index = usize::from(channel);
        registers[0x10 + index] = 0xc0;
        registers[0x20 + index] = 0x35;
        registers[0x30 + index] = 0x10;
        decode_channel_synthesis(&registers, channel).unwrap()
    }

    #[test]
    fn aggregate_constructor_retains_all_eighteen_explicit_pg_and_eg_states() {
        let bank = Ym2413SynthesisBank::with_initial_state(initial_state()).unwrap();
        for channel in 0..9 {
            assert_eq!(
                bank.phase_index(channel, false).unwrap(),
                ((channel as u32 * 0x1000) >> 9) as u16
            );
            assert_eq!(
                bank.phase_index(channel, true).unwrap(),
                ((channel as u32 * 0x1800) >> 9) as u16
            );
            assert_eq!(bank.envelope_level(channel, false).unwrap(), 127);
            assert_eq!(bank.envelope_level(channel, true).unwrap(), 127);
        }
    }

    #[test]
    fn per_operator_updates_and_channel_evaluation_are_deterministic() {
        let mut bank = Ym2413SynthesisBank::with_initial_state(initial_state()).unwrap();
        let config = configuration(0);

        bank.advance_operator_phase(0, false, config.modulator_phase)
            .unwrap();
        bank.advance_operator_phase(0, true, config.carrier_phase)
            .unwrap();
        bank.tick_operator_envelope(0, false, true, 0, config.modulator_envelope)
            .unwrap();
        bank.tick_operator_envelope(0, true, true, 0, config.carrier_envelope)
            .unwrap();

        let snapshot = bank.export_state();
        let mut restored = Ym2413SynthesisBank::restore_state(snapshot).unwrap();
        let mut uninterrupted = bank.clone();
        let first = uninterrupted.evaluate_channel(&config).unwrap();
        let second = restored.evaluate_channel(&config).unwrap();
        assert_eq!(first, second);
        assert_eq!(uninterrupted.export_state(), restored.export_state());
    }

    #[test]
    fn lfo_sample_advancement_is_shared_and_partition_invariant() {
        let mut continuous = Ym2413SynthesisBank::with_initial_state(initial_state()).unwrap();
        let mut chunked = continuous.clone();

        continuous.advance_lfo_samples(25_000);
        for samples in [1, 63, 1_024, 13_440, 10_472] {
            chunked.advance_lfo_samples(samples);
        }

        assert_eq!(continuous.pm_position(), chunked.pm_position());
        assert_eq!(continuous.am_level(), chunked.am_level());
        assert_eq!(continuous.export_state(), chunked.export_state());
    }

    #[test]
    fn snapshot_round_trip_preserves_all_channel_and_global_state() {
        let mut bank = Ym2413SynthesisBank::with_initial_state(initial_state()).unwrap();
        bank.advance_lfo_samples(777);
        bank.advance_operator_phase(8, true, configuration(8).carrier_phase)
            .unwrap();
        let snapshot = bank.export_state();
        assert_eq!(
            Ym2413SynthesisBank::restore_state(snapshot)
                .unwrap()
                .export_state(),
            snapshot
        );
    }

    #[test]
    fn snapshot_rejects_bad_version_and_nested_lfo_state() {
        let bank = Ym2413SynthesisBank::with_initial_state(initial_state()).unwrap();
        let mut state = bank.export_state();
        state.format_version = 99;
        assert_eq!(
            Ym2413SynthesisBank::restore_state(state),
            Err(Ym2413SynthesisError::InvalidState)
        );

        let mut state: Ym2413SynthesisBankState = bank.export_state();
        state.pm_lfo.counter = 8_192;
        assert!(matches!(
            Ym2413SynthesisBank::restore_state(state),
            Err(Ym2413SynthesisError::PhaseModulationLfo(_))
        ));
    }

    #[test]
    fn invalid_channel_and_initial_phase_are_rejected() {
        let bank = Ym2413SynthesisBank::with_initial_state(initial_state()).unwrap();
        assert_eq!(
            bank.phase_index(9, false),
            Err(Ym2413SynthesisError::InvalidChannel(9))
        );

        let mut initial = initial_state();
        initial.channels[4].phase[1] = 1 << 19;
        assert!(matches!(
            Ym2413SynthesisBank::with_initial_state(initial),
            Err(Ym2413SynthesisError::Phase(_))
        ));
    }

    fn rhythm_configuration() -> crate::rhythm_channel::Ym2413RhythmSynthesisConfig {
        let mut registers = [0; 64];
        registers[0x0e] = 0x20;
        registers[0x10 + 6] = 0x55;
        registers[0x20 + 6] = 0x35;
        registers[0x10 + 7] = 0xaa;
        registers[0x20 + 7] = 0x26;
        registers[0x10 + 8] = 0x33;
        registers[0x20 + 8] = 0x3b;
        registers[0x36] = 0xa3;
        registers[0x37] = 0x5c;
        registers[0x38] = 0xe7;
        decode_rhythm_synthesis(&registers).unwrap()
    }

    fn audible_initial_state() -> Ym2413SynthesisInitialState {
        let mut initial = initial_state();
        for channel in 6..9 {
            initial.channels[channel].envelope_level = [0, 0];
            initial.channels[channel].envelope_stage = [EnvelopeStage::Sustain; 2];
            initial.channels[channel].envelope_key_on = [true; 2];
        }
        initial
    }

    #[test]
    fn rhythm_envelope_helper_maps_the_physical_slot_to_shared_eg_state() {
        let config = rhythm_configuration();
        let operator = config.voices[1].modulator.unwrap();
        let mut helper_bank =
            Ym2413SynthesisBank::with_initial_state(audible_initial_state()).unwrap();
        let mut direct_bank = helper_bank.clone();
        let helper_level = helper_bank
            .tick_rhythm_operator_envelope(&operator, true, false, 17)
            .unwrap();
        let direct_level = direct_bank
            .tick_operator_envelope(
                operator.physical_channel,
                false,
                true,
                17,
                operator.envelope,
            )
            .unwrap();
        assert_eq!(helper_level, direct_level);
        assert_eq!(helper_bank.export_state(), direct_bank.export_state());
    }

    #[test]
    fn all_single_operator_rhythm_voices_use_explicit_phase_inputs_and_physical_eg_state() {
        use crate::rhythm_phase::{
            hi_hat_phase_candidate, snare_drum_phase_candidate, top_cymbal_phase_candidate,
        };

        let config = rhythm_configuration();
        let mut bank = Ym2413SynthesisBank::with_initial_state(audible_initial_state()).unwrap();
        let before = bank.export_state();
        let mod7 = bank.phase_index(6, false).unwrap();
        let car8 = bank.phase_index(7, true).unwrap();
        let phases = [
            (
                Ym2413RhythmVoice::HiHat,
                hi_hat_phase_candidate(mod7, car8, false).unwrap(),
            ),
            (
                Ym2413RhythmVoice::TomTom,
                bank.phase_index(8, false).unwrap(),
            ),
            (
                Ym2413RhythmVoice::SnareDrum,
                snare_drum_phase_candidate(mod7, false).unwrap(),
            ),
            (
                Ym2413RhythmVoice::TopCymbal,
                top_cymbal_phase_candidate(mod7, car8).unwrap(),
            ),
        ];
        for (voice, phase_index) in phases {
            let index = match voice {
                Ym2413RhythmVoice::HiHat => 1,
                Ym2413RhythmVoice::TomTom => 3,
                Ym2413RhythmVoice::SnareDrum => 2,
                Ym2413RhythmVoice::TopCymbal => 4,
                Ym2413RhythmVoice::BassDrum => unreachable!(),
            };
            let operator = config.voices[index]
                .modulator
                .or(config.voices[index].carrier)
                .unwrap();
            let carrier = operator.physical_role == Ym2413Operator::Carrier;
            let expected = config.voices[index]
                .evaluate_single_operator(
                    phase_index,
                    bank.envelope_level(operator.physical_channel, carrier)
                        .unwrap(),
                    bank.am_level(),
                )
                .unwrap();
            let sample = bank
                .evaluate_rhythm_voice(&config, voice, Some(phase_index))
                .unwrap();
            assert_eq!(sample.voice, voice);
            assert_eq!(sample.output, expected);
        }
        assert_eq!(bank.export_state(), before);
    }

    #[test]
    fn bass_drum_evaluation_reuses_channel_seven_feedback_pipeline() {
        let config = rhythm_configuration();
        let mut bank = Ym2413SynthesisBank::with_initial_state(audible_initial_state()).unwrap();
        let before = bank.feedback_history(6).unwrap();
        let sample = bank
            .evaluate_rhythm_voice(&config, Ym2413RhythmVoice::BassDrum, None)
            .unwrap();
        assert_eq!(sample.voice, Ym2413RhythmVoice::BassDrum);
        assert_ne!(bank.feedback_history(6).unwrap(), before);
    }

    #[test]
    fn rhythm_batch_returns_all_five_voices_in_stable_order_and_updates_bd_feedback_once() {
        let config = rhythm_configuration();
        let phases = super::Ym2413RhythmVoicePhases {
            hi_hat: 0x134,
            snare_drum: 0x200,
            tom_tom: 0x2a1,
            top_cymbal: 0x300,
        };
        let mut batch_bank =
            Ym2413SynthesisBank::with_initial_state(audible_initial_state()).unwrap();
        let mut direct_bank = batch_bank.clone();

        let batch = batch_bank.evaluate_rhythm_voices(&config, phases).unwrap();
        let direct = [
            direct_bank
                .evaluate_rhythm_voice(&config, Ym2413RhythmVoice::BassDrum, None)
                .unwrap(),
            direct_bank
                .evaluate_rhythm_voice(&config, Ym2413RhythmVoice::HiHat, Some(phases.hi_hat))
                .unwrap(),
            direct_bank
                .evaluate_rhythm_voice(
                    &config,
                    Ym2413RhythmVoice::SnareDrum,
                    Some(phases.snare_drum),
                )
                .unwrap(),
            direct_bank
                .evaluate_rhythm_voice(&config, Ym2413RhythmVoice::TomTom, Some(phases.tom_tom))
                .unwrap(),
            direct_bank
                .evaluate_rhythm_voice(
                    &config,
                    Ym2413RhythmVoice::TopCymbal,
                    Some(phases.top_cymbal),
                )
                .unwrap(),
        ];

        assert_eq!(batch, direct);
        assert_eq!(
            batch.map(|sample| sample.voice),
            [
                Ym2413RhythmVoice::BassDrum,
                Ym2413RhythmVoice::HiHat,
                Ym2413RhythmVoice::SnareDrum,
                Ym2413RhythmVoice::TomTom,
                Ym2413RhythmVoice::TopCymbal,
            ]
        );
        assert_eq!(batch_bank.export_state(), direct_bank.export_state());
    }

    #[test]
    fn rhythm_batch_rejects_invalid_single_voice_phase_before_mutating_bd_feedback() {
        let config = rhythm_configuration();
        let phases = super::Ym2413RhythmVoicePhases {
            hi_hat: 0x0400,
            snare_drum: 0,
            tom_tom: 0,
            top_cymbal: 0,
        };
        let mut bank = Ym2413SynthesisBank::with_initial_state(audible_initial_state()).unwrap();
        let before = bank.export_state();

        assert!(matches!(
            bank.evaluate_rhythm_voices(&config, phases),
            Err(Ym2413SynthesisError::RhythmVoice(_))
        ));
        assert_eq!(bank.export_state(), before);
    }

    #[test]
    fn rhythm_voice_evaluation_requires_phase_only_for_single_operator_voices() {
        let config = rhythm_configuration();
        let mut bank = Ym2413SynthesisBank::with_initial_state(audible_initial_state()).unwrap();
        assert_eq!(
            bank.evaluate_rhythm_voice(&config, Ym2413RhythmVoice::HiHat, None),
            Err(Ym2413SynthesisError::RhythmVoice(
                crate::rhythm_channel::RhythmVoiceSynthesisError::MissingPhaseInput {
                    voice: Ym2413RhythmVoice::HiHat,
                }
            ))
        );
        assert_eq!(
            bank.evaluate_rhythm_voice(&config, Ym2413RhythmVoice::BassDrum, Some(0)),
            Err(Ym2413SynthesisError::RhythmVoice(
                crate::rhythm_channel::RhythmVoiceSynthesisError::UnexpectedPhaseInput {
                    voice: Ym2413RhythmVoice::BassDrum,
                }
            ))
        );
    }

    #[test]
    fn rhythm_voice_evaluation_rejects_disabled_rhythm_mode() {
        let mut config = rhythm_configuration();
        config.rhythm_mode_enabled = false;
        let mut bank = Ym2413SynthesisBank::with_initial_state(audible_initial_state()).unwrap();
        let before = bank.export_state();
        assert_eq!(
            bank.evaluate_rhythm_voice(&config, Ym2413RhythmVoice::HiHat, Some(0)),
            Err(Ym2413SynthesisError::RhythmModeDisabled)
        );
        assert_eq!(bank.export_state(), before);
    }

    #[test]
    fn rhythm_mode_channels_are_not_resolved_as_melodic_synthesis() {
        let mut registers = [0; 64];
        registers[0x0e] = 0x20;
        registers[0x36] = 0x10;
        assert!(decode_channel_synthesis(&registers, 6).is_err());
    }
}
