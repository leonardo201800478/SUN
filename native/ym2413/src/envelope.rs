//! Candidate YM2413 envelope generator (EG).
//!
//! The EG is driven by an explicit chip-wide sample counter. It owns no clock
//! and does not infer the counter reset alignment or operator slot boundary.
//! Rate curves follow published hardware measurements by andete (2015-03-20,
//! 2015-12-24, 2015-12-31 and 2017-01-26). This is a measured software model,
//! not a claim of transistor-level or absolute phase fidelity.

pub const YM2413_EG_LEVELS: u8 = 128;
pub const YM2413_EG_MAX_LEVEL: u8 = YM2413_EG_LEVELS - 1;
pub const YM2413_EG_STATE_VERSION: u16 = 2;

/// The four envelope phases described by the reverse-engineering model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EnvelopeStage {
    Damp = 0,
    Attack = 1,
    Decay = 2,
    Sustain = 3,
}

/// Candidate policy for the unresolved DAMP -> ATTACK boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DampTransitionCoordination {
    /// Each operator evaluates its own DAMP completion boundary.
    IndependentPerOperator,
    /// The carrier transition is propagated to the modulator by the caller.
    CarrierDrivesBoth,
}

/// Candidate behavior when AR=15 at the DAMP completion boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackRate15Transition {
    /// Follow the andete software model: skip ATTACK and enter DECAY.
    SkipAttackToDecay,
    /// Enter ATTACK; the rate-15 attack sequencer itself remains separately modeled.
    EnterAttack,
}

/// Explicitly selected DAMP transition candidate; no threshold is assumed by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DampTransitionConfig {
    threshold: u8,
    coordination: DampTransitionCoordination,
    attack_rate_15: AttackRate15Transition,
}

impl DampTransitionConfig {
    /// Construct a candidate threshold/coordination model.
    pub const fn new(
        threshold: u8,
        coordination: DampTransitionCoordination,
        attack_rate_15: AttackRate15Transition,
    ) -> Result<Self, EnvelopeGeneratorError> {
        if threshold > YM2413_EG_MAX_LEVEL {
            return Err(EnvelopeGeneratorError::InvalidDampConfiguration);
        }
        Ok(Self {
            threshold,
            coordination,
            attack_rate_15,
        })
    }

    pub const fn threshold(self) -> u8 {
        self.threshold
    }

    pub const fn coordination(self) -> DampTransitionCoordination {
        self.coordination
    }

    pub const fn attack_rate_15(self) -> AttackRate15Transition {
        self.attack_rate_15
    }
}

/// Register-derived operator and channel fields consumed by one EG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeParameters {
    pub attack_rate: u8,
    pub decay_rate: u8,
    pub release_rate: u8,
    pub sustain_level: u8,
    pub key_scale_rate: bool,
    pub fnum: u16,
    pub block: u8,
    pub sustain_on: bool,
    pub sustained_tone: bool,
}

impl EnvelopeParameters {
    /// Reject values that cannot be represented by the YM2413 registers.
    pub const fn is_valid(self) -> bool {
        self.attack_rate <= 15
            && self.decay_rate <= 15
            && self.release_rate <= 15
            && self.sustain_level <= 15
            && self.fnum <= 0x01ff
            && self.block <= 7
    }

    /// Select the carrier KEY-OFF base rate from EG-TYPE and channel SUS.
    ///
    /// The measured YM2413 behavior is:
    /// - SUS=ON: fixed base rate 5 for either envelope type.
    /// - SUS=OFF + sustained tone: the operator's programmed RR.
    /// - SUS=OFF + percussive tone: fixed base rate 7.
    ///
    /// KSR is applied later by effective_rate(), including to fixed rates.
    const fn carrier_key_off_base_rate(self) -> u8 {
        if self.sustain_on {
            5
        } else if self.sustained_tone {
            self.release_rate
        } else {
            7
        }
    }

    /// Compute RATE = 4 * R + Rks, clipped to six bits; R=0 forces RATE=0.
    pub const fn effective_rate(self, base_rate: u8) -> u8 {
        // A programmed rate of zero disables the EG increment regardless
        // of KSR. The KSR offset is applied only to non-zero base rates.
        if base_rate == 0 {
            return 0;
        }

        let key_code = (self.block << 1) | ((self.fnum >> 8) as u8);
        let rks = if self.key_scale_rate {
            key_code
        } else {
            key_code >> 2
        };
        let rate = (base_rate as u16) * 4 + rks as u16;
        if rate > 63 { 63 } else { rate as u8 }
    }
}

/// Complete serializable state of one operator's envelope generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeGeneratorState {
    pub format_version: u16,
    pub level: u8,
    pub stage: EnvelopeStage,
    pub key_on: bool,
    pub damp_config: DampTransitionConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeGeneratorError {
    InvalidParameters,
    InvalidState,
    InvalidDampConfiguration,
}

/// Result of one EG update, including a candidate phase-reset boundary.
///
/// The reverse-engineered model resets the corresponding PG when DAMP ends.
/// This event exposes that boundary without owning or advancing PG state.
/// Under the explicit `CarrierDrivesBoth` candidate, the caller must also
/// synchronize the modulator EG and reset its PG at the same selected boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeTickResult {
    /// Current seven-bit attenuation (0 is loudest, 127 is silent).
    pub level: u8,
    /// True only on the update that completes DAMP and requests a candidate PG reset.
    ///
    /// The selected ATTACK/DECAY stage may also consume this update's counter,
    /// so the final stage after the update is not part of this event's contract.
    pub phase_reset_required: bool,
}

/// One operator EG. The caller supplies the shared global sample counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeGenerator {
    level: u8,
    stage: EnvelopeStage,
    key_on: bool,
    damp_config: DampTransitionConfig,
}

impl EnvelopeGenerator {
    /// Construct from explicit initial hardware state and a DAMP candidate.
    pub const fn with_initial_state(
        level: u8,
        stage: EnvelopeStage,
        key_on: bool,
        damp_config: DampTransitionConfig,
    ) -> Result<Self, EnvelopeGeneratorError> {
        if level > YM2413_EG_MAX_LEVEL {
            return Err(EnvelopeGeneratorError::InvalidState);
        }
        if damp_config.threshold > YM2413_EG_MAX_LEVEL {
            return Err(EnvelopeGeneratorError::InvalidDampConfiguration);
        }
        Ok(Self {
            level,
            stage,
            key_on,
            damp_config,
        })
    }

    /// Current attenuation (0 is loudest, 127 is silent).
    pub const fn level(&self) -> u8 {
        self.level
    }

    /// Current envelope phase.
    pub const fn stage(&self) -> EnvelopeStage {
        self.stage
    }

    /// Return the explicitly selected DAMP candidate.
    pub const fn damp_config(&self) -> DampTransitionConfig {
        self.damp_config
    }

    fn owns_damp_transition(&self, is_carrier: bool) -> bool {
        match self.damp_config.coordination {
            DampTransitionCoordination::IndependentPerOperator => true,
            DampTransitionCoordination::CarrierDrivesBoth => is_carrier,
        }
    }

    fn complete_damp(&mut self, attack_rate: u8) {
        if attack_rate == 15
            && self.damp_config.attack_rate_15 == AttackRate15Transition::SkipAttackToDecay
        {
            // Skipping ATTACK changes the stage, not the attenuation level.
            // The reference model retains the DAMP endpoint and then evaluates
            // DECAY using the same update counter.
            self.stage = EnvelopeStage::Decay;
        } else {
            self.stage = EnvelopeStage::Attack;
        }
    }

    /// Advance one operator update using the externally owned counter value.
    ///
    /// The carrier receives the measured second-release selector: SUS=ON
    /// chooses base rate 5 for both EG types; with SUS=OFF, sustained tone
    /// uses programmed RR and percussive tone uses fixed rate 7. The modulator
    /// retains its first-release/sustain path. DAMP coordination and the AR=15
    /// boundary behavior are explicit candidate configuration.
    pub fn tick(
        &mut self,
        global_counter: u32,
        key_on: bool,
        is_carrier: bool,
        parameters: EnvelopeParameters,
    ) -> Result<u8, EnvelopeGeneratorError> {
        self.tick_with_events(global_counter, key_on, is_carrier, parameters)
            .map(|result| result.level)
    }

    /// Advance one operator EG and report candidate state-transition events.
    ///
    /// The event is emitted when the DAMP threshold is reached. The PG itself
    /// remains caller-owned; this method never resets phase or advances clocks.
    pub fn tick_with_events(
        &mut self,
        global_counter: u32,
        key_on: bool,
        is_carrier: bool,
        parameters: EnvelopeParameters,
    ) -> Result<EnvelopeTickResult, EnvelopeGeneratorError> {
        if !parameters.is_valid() {
            return Err(EnvelopeGeneratorError::InvalidParameters);
        }

        if key_on && !self.key_on {
            self.stage = EnvelopeStage::Damp;
        }
        self.key_on = key_on;

        let mut phase_reset_required = false;

        if !key_on && is_carrier {
            let base_rate = parameters.carrier_key_off_base_rate();
            self.advance_decay(global_counter, parameters.effective_rate(base_rate));
            return Ok(EnvelopeTickResult {
                level: self.level,
                phase_reset_required: false,
            });
        }

        if self.owns_damp_transition(is_carrier)
            && self.stage == EnvelopeStage::Damp
            && self.level >= self.damp_config.threshold
        {
            // DAMP is complete at this update boundary. Select the new stage
            // before the stage evaluator below, matching the reference model:
            // ATTACK may consume this same shared-counter update.
            self.complete_damp(parameters.attack_rate);
            phase_reset_required = true;
        }

        match self.stage {
            EnvelopeStage::Damp => {
                self.advance_decay(global_counter, parameters.effective_rate(12));
                if self.owns_damp_transition(is_carrier) && self.level >= self.damp_config.threshold
                {
                    self.complete_damp(parameters.attack_rate);
                    phase_reset_required = true;
                }
            }
            EnvelopeStage::Attack => {
                self.advance_attack(
                    global_counter,
                    parameters.effective_rate(parameters.attack_rate),
                );
                if self.level == 0 {
                    self.stage = EnvelopeStage::Decay;
                }
            }
            EnvelopeStage::Decay => {
                // Evaluate the measured SL comparator before the next EG
                // increment; SL=0 must stop at level zero.
                if (self.level >> 3) == parameters.sustain_level {
                    self.stage = EnvelopeStage::Sustain;
                    let base_rate = if parameters.sustained_tone {
                        0
                    } else {
                        parameters.release_rate
                    };
                    self.advance_decay(global_counter, parameters.effective_rate(base_rate));
                } else {
                    self.advance_decay(
                        global_counter,
                        parameters.effective_rate(parameters.decay_rate),
                    );
                    if (self.level >> 3) == parameters.sustain_level {
                        self.stage = EnvelopeStage::Sustain;
                    }
                }
            }
            EnvelopeStage::Sustain => {
                let base_rate = if parameters.sustained_tone {
                    0
                } else {
                    parameters.release_rate
                };
                self.advance_decay(global_counter, parameters.effective_rate(base_rate));
            }
        }
        Ok(EnvelopeTickResult {
            level: self.level,
            phase_reset_required,
        })
    }

    /// Apply the carrier's DAMP transition under the CarrierDrivesBoth candidate.
    ///
    /// This is not called automatically: the exact modulator/carrier transition
    /// offset is unresolved. Call after the carrier update only when the owning
    /// model explicitly selects CarrierDrivesBoth. PG phase reset remains external.
    pub fn synchronize_damp_transition(&mut self, carrier: &Self) {
        if self.damp_config.coordination == DampTransitionCoordination::CarrierDrivesBoth
            && self.damp_config == carrier.damp_config
            && self.stage == EnvelopeStage::Damp
            && carrier.stage != EnvelopeStage::Damp
        {
            self.stage = carrier.stage;
            // The hardware model stores KEY-ON at channel scope. This EG keeps
            // a local copy, so propagate the carrier-observed latch together
            // with the candidate DAMP transition. Otherwise a later modulator
            // tick could interpret the same KEY-ON as a fresh edge and restart
            // DAMP after it was synchronized.
            self.key_on = carrier.key_on;
        }
    }

    /// Export all local state required to resume this EG.
    pub const fn export_state(&self) -> EnvelopeGeneratorState {
        EnvelopeGeneratorState {
            format_version: YM2413_EG_STATE_VERSION,
            level: self.level,
            stage: self.stage,
            key_on: self.key_on,
            damp_config: self.damp_config,
        }
    }

    /// Restore a snapshot after checking its version and seven-bit level.
    pub const fn restore_state(
        state: EnvelopeGeneratorState,
    ) -> Result<Self, EnvelopeGeneratorError> {
        if state.format_version != YM2413_EG_STATE_VERSION || state.level > YM2413_EG_MAX_LEVEL {
            return Err(EnvelopeGeneratorError::InvalidState);
        }
        Ok(Self {
            level: state.level,
            stage: state.stage,
            key_on: state.key_on,
            damp_config: state.damp_config,
        })
    }

    fn advance_decay(&mut self, counter: u32, rate: u8) {
        if rate < 4 {
            return;
        }

        let increment = match rate {
            0..=3 => return,
            4..=51 => {
                let shift = 13 - (rate / 4);
                let mask = (1_u32 << shift) - 1;
                if counter & mask != 0 {
                    return;
                }
                let column = ((counter >> shift) & 7) as usize;
                EG_STEP_TABLE[(rate & 3) as usize][column]
            }
            52..=59 => EG_FAST_DECAY_TABLE[(rate - 52) as usize][(counter & 0x0f) as usize],
            60..=63 => 2,
            _ => unreachable!("effective EG rate is clamped to six bits"),
        };
        self.level = self
            .level
            .saturating_add(increment)
            .min(YM2413_EG_MAX_LEVEL);
    }

    fn advance_attack(&mut self, counter: u32, rate: u8) {
        if !(4..60).contains(&rate) {
            return;
        }

        let next = if rate < 48 {
            let shift = 13 - (rate / 4);
            let mask = ((1_u32 << shift) - 1) & !3;
            if counter & mask != 0 {
                return;
            }
            let column = ((counter >> shift) & 7) as usize;
            if EG_STEP_TABLE[(rate & 3) as usize][column] == 0 {
                return;
            }
            Self::attack_formula(self.level, 4)
        } else {
            let column = ((counter & 0x0c) >> 1) as usize;
            let mut shift = 16 - (rate / 4);
            if EG_STEP_TABLE[(rate & 3) as usize][column] != 0 {
                shift -= 1;
            }
            Self::attack_formula(self.level, shift)
        };
        self.level = next;
    }

    /// Measured attack curve: f_m(x) = x - (x >> m) - 1.
    const fn attack_formula(level: u8, shift: u8) -> u8 {
        level.saturating_sub((level >> shift) + 1)
    }
}

/// Measured 16-step decay selectors for effective rates 52..59.
/// These reproduce the measured level-step sequences; they do not imply that
/// the physical chip contains literal ROM tables.
const EG_FAST_DECAY_TABLE: [[u8; 16]; 8] = [
    [0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1],
    [0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1, 1],
    [0, 1, 0, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 1, 1, 1],
    [0, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [2, 2, 2, 2, 1, 1, 1, 1, 2, 2, 2, 2, 1, 1, 1, 1],
    [2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1],
];

/// Measured eight-step rate selector for slow attack and decay timing.
const EG_STEP_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 1, 0, 1, 0, 1],
    [0, 1, 0, 1, 1, 1, 0, 1],
    [0, 1, 1, 1, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 1],
];

#[cfg(test)]
mod tests {
    use super::{
        AttackRate15Transition, DampTransitionConfig, DampTransitionCoordination,
        EnvelopeGenerator, EnvelopeGeneratorError, EnvelopeGeneratorState, EnvelopeParameters,
        EnvelopeStage,
    };

    fn damp_config(
        coordination: DampTransitionCoordination,
        attack_rate_15: AttackRate15Transition,
    ) -> DampTransitionConfig {
        DampTransitionConfig::new(124, coordination, attack_rate_15).unwrap()
    }

    fn eg(level: u8) -> EnvelopeGenerator {
        EnvelopeGenerator::with_initial_state(
            level,
            EnvelopeStage::Damp,
            false,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap()
    }

    fn params() -> EnvelopeParameters {
        EnvelopeParameters {
            attack_rate: 10,
            decay_rate: 3,
            release_rate: 5,
            sustain_level: 4,
            key_scale_rate: false,
            fnum: 0x100,
            block: 0,
            sustain_on: false,
            sustained_tone: true,
        }
    }

    #[test]
    fn effective_rate_uses_documented_ksr_mapping() {
        let mut p = params();
        p.block = 5;
        p.fnum = 0x100;
        assert_eq!(p.effective_rate(3), 14);
        p.key_scale_rate = true;
        assert_eq!(p.effective_rate(3), 23);
    }

    #[test]
    fn zero_programmed_rate_disables_ksr_for_every_key_code() {
        let mut p = params();
        for key_scale_rate in [false, true] {
            p.key_scale_rate = key_scale_rate;
            for block in 0..=7 {
                p.block = block;
                for fnum_msb in 0_u8..=1 {
                    p.fnum = u16::from(fnum_msb) << 8;
                    assert_eq!(
                        p.effective_rate(0),
                        0,
                        "KSR={key_scale_rate}, BLOCK={block}, FNUM8={fnum_msb}"
                    );
                }
            }
        }
    }

    #[test]
    fn ksr_offset_matches_the_complete_block_and_fnum_msb_matrix() {
        let mut p = params();
        for block in 0..=7 {
            p.block = block;
            for fnum_msb in 0..=1 {
                p.fnum = u16::from(fnum_msb) << 8;
                let key_code = (block << 1) | fnum_msb;
                p.key_scale_rate = false;
                assert_eq!(p.effective_rate(3), 12 + (key_code >> 2));
                p.key_scale_rate = true;
                assert_eq!(p.effective_rate(3), 12 + key_code);
            }
        }
    }

    #[test]
    fn sustained_tone_sustain_level_holds_even_with_ksr_enabled() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            32,
            EnvelopeStage::Sustain,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.key_scale_rate = true;
        p.fnum = 0x100;
        p.block = 7;
        p.release_rate = 15;
        p.sustain_level = 4;
        p.sustained_tone = true;

        for counter in 0..=4_096 {
            generator.tick(counter, true, true, p).unwrap();
        }

        assert_eq!(generator.level(), 32);
        assert_eq!(generator.stage(), EnvelopeStage::Sustain);
    }

    #[test]
    fn measured_damp_ramp_reaches_candidate_threshold_in_about_ten_ms() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            0,
            EnvelopeStage::Damp,
            false,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.attack_rate = 10;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;
        p.sustain_level = 15;

        let mut transition_counter = None;
        for counter in 0..=494 {
            generator.tick(counter, true, true, p).unwrap();
            if generator.stage() != EnvelopeStage::Damp {
                transition_counter = Some(counter);
                break;
            }
        }

        // The candidate threshold 124 is reached on counter 494:
        // 494 / (3_579_545 / 72) ~= 9.94 ms.
        assert_eq!(transition_counter, Some(494));
        assert_eq!(generator.level(), 124);
        assert_eq!(generator.stage(), EnvelopeStage::Attack);
    }

    #[test]
    fn attack_formula_reproduces_measured_f4_sequence() {
        let mut level = 127_u8;
        let expected = [
            119, 111, 104, 97, 90, 84, 78, 73, 68, 63, 59, 55, 51, 47, 44, 41, 38, 35, 32, 29, 27,
            25, 23, 21, 19, 17, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
        ];
        for value in expected {
            level = level.saturating_sub((level >> 4) + 1);
            assert_eq!(level, value);
        }
    }

    #[test]
    fn slow_attack_uses_shared_counter_gate_and_selector() {
        let mut eg = eg(127);
        eg.stage = EnvelopeStage::Attack;
        eg.key_on = true;
        let p = params();
        eg.tick(1, true, true, p).unwrap();
        assert_eq!(eg.level(), 127);
        eg.tick(8, true, true, p).unwrap();
        assert!(eg.level() < 127);
    }

    #[test]
    fn zero_rate_holds_and_rate_fifteen_does_not_restart_attack_mid_phase() {
        let mut eg = eg(32);
        eg.stage = EnvelopeStage::Decay;
        let mut p = params();
        p.decay_rate = 0;
        eg.tick(0, true, true, p).unwrap();
        assert_eq!(eg.level(), 32);
        eg.stage = EnvelopeStage::Attack;
        eg.level = 64;
        p.attack_rate = 15;
        eg.tick(0, true, true, p).unwrap();
        assert_eq!(eg.level(), 64);
    }

    #[test]
    fn fast_decay_rates_52_to_59_match_measured_sixteen_step_sequences() {
        const EXPECTED: [[u8; 16]; 8] = [
            [0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1],
            [0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1, 1],
            [0, 1, 0, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 1, 1, 1],
            [0, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            [2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            [2, 2, 2, 2, 1, 1, 1, 1, 2, 2, 2, 2, 1, 1, 1, 1],
            [2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1],
        ];

        for (offset, expected) in EXPECTED.iter().enumerate() {
            let rate = 52 + offset as u8;
            let mut generator = eg(0);
            generator.stage = EnvelopeStage::Decay;
            generator.key_on = true;
            let mut p = params();
            p.decay_rate = rate / 4;
            p.block = (rate % 4) * 2;
            p.fnum = 0;
            for (counter, expected_increment) in expected.iter().enumerate() {
                let before = generator.level();
                generator.tick(counter as u32, true, true, p).unwrap();
                assert_eq!(
                    generator.level() - before,
                    *expected_increment,
                    "effective rate {rate}, counter {counter}"
                );
            }
        }
    }

    #[test]
    fn fastest_decay_advances_two_levels_per_update() {
        let mut eg = eg(10);
        eg.stage = EnvelopeStage::Decay;
        eg.key_on = true;
        let mut p = params();
        p.decay_rate = 15;
        eg.tick(1, true, true, p).unwrap();
        assert_eq!(eg.level(), 12);
    }

    #[test]
    fn modulator_damp_transition_follows_carrier_only_in_selected_candidate() {
        let config = damp_config(
            DampTransitionCoordination::CarrierDrivesBoth,
            AttackRate15Transition::SkipAttackToDecay,
        );
        let mut modulator =
            EnvelopeGenerator::with_initial_state(127, EnvelopeStage::Damp, false, config).unwrap();
        let mut carrier =
            EnvelopeGenerator::with_initial_state(127, EnvelopeStage::Damp, false, config).unwrap();
        let mut p = params();
        p.attack_rate = 10;

        modulator.tick(0, true, false, p).unwrap();
        assert_eq!(modulator.stage(), EnvelopeStage::Damp);

        carrier.tick(0, true, true, p).unwrap();
        assert_eq!(carrier.stage(), EnvelopeStage::Attack);
        modulator.synchronize_damp_transition(&carrier);
        assert_eq!(modulator.stage(), EnvelopeStage::Attack);
    }

    #[test]
    fn carrier_damp_sync_preserves_transition_on_later_modulator_tick() {
        let config = damp_config(
            DampTransitionCoordination::CarrierDrivesBoth,
            AttackRate15Transition::SkipAttackToDecay,
        );
        let mut modulator =
            EnvelopeGenerator::with_initial_state(127, EnvelopeStage::Damp, false, config).unwrap();
        let mut carrier =
            EnvelopeGenerator::with_initial_state(127, EnvelopeStage::Damp, false, config).unwrap();
        let mut p = params();
        p.attack_rate = 10;

        carrier.tick(0, true, true, p).unwrap();
        modulator.synchronize_damp_transition(&carrier);
        assert_eq!(modulator.stage(), EnvelopeStage::Attack);

        // The modulator's first serviced tick may occur after the carrier
        // event. It must not reinterpret the already-observed KEY-ON as new.
        modulator.tick(0, true, false, p).unwrap();
        assert_eq!(modulator.stage(), EnvelopeStage::Attack);
    }

    #[test]
    fn independent_damp_candidate_allows_modulator_own_transition() {
        let mut modulator = eg(127);
        let mut p = params();
        p.attack_rate = 10;
        modulator.tick(0, true, false, p).unwrap();
        assert_eq!(modulator.stage(), EnvelopeStage::Attack);
    }

    #[test]
    fn damp_threshold_is_an_explicit_candidate_parameter() {
        let config = DampTransitionConfig::new(
            120,
            DampTransitionCoordination::IndependentPerOperator,
            AttackRate15Transition::SkipAttackToDecay,
        )
        .unwrap();
        let mut generator =
            EnvelopeGenerator::with_initial_state(120, EnvelopeStage::Damp, false, config).unwrap();
        let mut p = params();
        p.attack_rate = 10;
        generator.tick(0, true, false, p).unwrap();
        assert_eq!(generator.stage(), EnvelopeStage::Attack);
    }

    #[test]
    fn damp_completion_at_update_entry_advances_attack_with_same_counter() {
        let config = damp_config(
            DampTransitionCoordination::IndependentPerOperator,
            AttackRate15Transition::EnterAttack,
        );
        let mut generator =
            EnvelopeGenerator::with_initial_state(124, EnvelopeStage::Damp, false, config).unwrap();
        let mut p = params();
        p.attack_rate = 10;

        // Effective AR is 40. Counter 8 selects an attack step, so the
        // reference sequence transitions DAMP -> ATTACK and advances 124 -> 116.
        let result = generator.tick_with_events(8, true, true, p).unwrap();

        assert!(result.phase_reset_required);
        assert_eq!(result.level, 116);
        assert_eq!(generator.stage(), EnvelopeStage::Attack);
    }

    #[test]
    fn attack_rate_fifteen_transition_is_an_explicit_candidate_parameter() {
        let config = damp_config(
            DampTransitionCoordination::IndependentPerOperator,
            AttackRate15Transition::EnterAttack,
        );
        let mut generator =
            EnvelopeGenerator::with_initial_state(124, EnvelopeStage::Damp, false, config).unwrap();
        let mut p = params();
        p.attack_rate = 15;
        generator.tick(0, true, true, p).unwrap();
        assert_eq!(generator.level(), 124);
        assert_eq!(generator.stage(), EnvelopeStage::Attack);
    }

    #[test]
    fn damp_and_attack_fifteen_shortcut_are_explicit() {
        let mut eg = eg(124);
        let mut p = params();
        p.attack_rate = 15;
        eg.tick(0, true, true, p).unwrap();
        assert_eq!(eg.level(), 124);
        assert_eq!(eg.stage(), EnvelopeStage::Decay);
    }

    #[test]
    fn sustain_transition_uses_four_bit_level_comparison() {
        let mut eg = eg(31);
        eg.stage = EnvelopeStage::Decay;
        eg.key_on = true;
        let mut p = params();
        p.decay_rate = 15;
        p.sustain_level = 4;
        eg.tick(0, true, true, p).unwrap();
        assert_eq!(eg.level(), 33);
        assert_eq!(eg.stage(), EnvelopeStage::Sustain);
    }

    #[test]
    fn sustain_level_zero_does_not_decay_past_zero() {
        let mut eg = eg(0);
        eg.stage = EnvelopeStage::Decay;
        eg.key_on = true;
        let mut p = params();
        p.decay_rate = 15;
        p.sustain_level = 0;
        eg.tick(0, true, true, p).unwrap();
        assert_eq!(eg.level(), 0);
        assert_eq!(eg.stage(), EnvelopeStage::Sustain);
    }

    #[test]
    fn carrier_key_off_base_rate_matches_eg_type_and_sus_truth_table() {
        let mut p = params();
        p.release_rate = 6;

        // EG-TYPE=0: SUS selects fixed rate 5 or 7.
        // EG-TYPE=1: SUS=ON selects 5; SUS=OFF selects programmed RR.
        for (sustained_tone, sustain_on, expected) in [
            (false, false, 7),
            (false, true, 5),
            (true, false, 6),
            (true, true, 5),
        ] {
            p.sustained_tone = sustained_tone;
            p.sustain_on = sustain_on;
            assert_eq!(
                p.carrier_key_off_base_rate(),
                expected,
                "EG-TYPE={sustained_tone}, SUS={sustain_on}"
            );
        }
    }

    #[test]
    fn carrier_key_off_effective_rate_matrix_applies_ksr_after_sus_selection() {
        let mut p = params();
        p.release_rate = 6;
        p.block = 3;
        p.fnum = 0x100;

        let cases = [
            (false, false, 7, 29, 35),
            (false, true, 5, 21, 27),
            (true, false, 6, 25, 31),
            (true, true, 5, 21, 27),
        ];
        for (sustained_tone, sustain_on, base_rate, ksr_off, ksr_on) in cases {
            p.sustained_tone = sustained_tone;
            p.sustain_on = sustain_on;
            p.key_scale_rate = false;
            assert_eq!(
                p.effective_rate(p.carrier_key_off_base_rate()),
                ksr_off,
                "KSR=0, EG-TYPE={sustained_tone}, SUS={sustain_on}"
            );
            p.key_scale_rate = true;
            assert_eq!(
                p.effective_rate(p.carrier_key_off_base_rate()),
                ksr_on,
                "KSR=1, EG-TYPE={sustained_tone}, SUS={sustain_on}"
            );
            assert_eq!(p.carrier_key_off_base_rate(), base_rate);
        }
    }

    #[test]
    fn sustained_carrier_key_off_sus_on_uses_fixed_rate_five_not_programmed_rr() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            20,
            EnvelopeStage::Sustain,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.sustained_tone = true;
        p.sustain_on = true;
        p.release_rate = 0;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;

        // Effective rate 20 at counter 256 selects a +1 decay step.
        // The old sustained_tone-first selector chose RR=0 and held at 20.
        generator.tick(256, false, true, p).unwrap();
        assert_eq!(generator.level(), 21);
    }

    #[test]
    fn key_off_percussive_release_uses_sus_rate_five() {
        let mut eg = eg(20);
        eg.stage = EnvelopeStage::Sustain;
        eg.key_on = true;
        let mut p = params();
        p.sustained_tone = false;
        p.sustain_on = true;
        p.release_rate = 0;
        eg.tick(256, false, true, p).unwrap();
        assert_eq!(eg.level(), 21);
    }

    #[test]
    fn carrier_key_off_uses_release_path_from_each_envelope_phase() {
        use super::EnvelopeStage;

        for stage in [
            EnvelopeStage::Damp,
            EnvelopeStage::Attack,
            EnvelopeStage::Decay,
            EnvelopeStage::Sustain,
        ] {
            let mut generator = EnvelopeGenerator::with_initial_state(
                20,
                stage,
                true,
                damp_config(
                    DampTransitionCoordination::IndependentPerOperator,
                    AttackRate15Transition::SkipAttackToDecay,
                ),
            )
            .unwrap();
            let mut p = params();
            p.sustained_tone = false;
            p.sustain_on = false;
            p.release_rate = 0;
            p.key_scale_rate = false;
            p.fnum = 0;
            p.block = 0;

            // Percussive SUS=OFF selects base RR'=7 (effective rate 28).
            // Counter 64 selects the first +1 decay step for this rate.
            generator.tick(64, false, true, p).unwrap();
            assert_eq!(generator.level(), 21, "stage {stage:?}");
            assert_eq!(generator.stage(), stage, "stage {stage:?}");
        }
    }

    #[test]
    fn modulator_key_off_keeps_its_first_release_or_sustain_behavior() {
        // EG-TYPE=1: the modulator remains at its sustain level on KEY-OFF.
        let mut sustained = EnvelopeGenerator::with_initial_state(
            20,
            EnvelopeStage::Sustain,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut sustained_parameters = params();
        sustained_parameters.sustained_tone = true;
        sustained_parameters.release_rate = 15;
        sustained
            .tick(0, false, false, sustained_parameters)
            .unwrap();
        assert_eq!(sustained.level(), 20);
        assert_eq!(sustained.stage(), EnvelopeStage::Sustain);

        // EG-TYPE=0: the modulator continues at the first release rate RR,
        // rather than entering the carrier's second KEY-OFF release path.
        let mut percussive = EnvelopeGenerator::with_initial_state(
            20,
            EnvelopeStage::Sustain,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut percussive_parameters = params();
        percussive_parameters.sustained_tone = false;
        percussive_parameters.sustain_on = true;
        percussive_parameters.release_rate = 4;
        percussive_parameters.key_scale_rate = false;
        percussive_parameters.fnum = 0;
        percussive_parameters.block = 0;
        // Effective RR=16; counter 512 selects the first +1 step.
        percussive
            .tick(512, false, false, percussive_parameters)
            .unwrap();
        assert_eq!(percussive.level(), 21);
        assert_eq!(percussive.stage(), EnvelopeStage::Sustain);
    }

    #[test]
    fn snapshot_round_trip_preserves_complete_local_state() {
        let mut eg = eg(61);
        eg.stage = EnvelopeStage::Attack;
        eg.key_on = true;
        assert_eq!(
            EnvelopeGenerator::restore_state(eg.export_state()).unwrap(),
            eg
        );
    }

    #[test]
    fn snapshot_rejects_bad_version_and_level() {
        assert_eq!(
            EnvelopeGenerator::restore_state(EnvelopeGeneratorState {
                format_version: 99,
                level: 0,
                stage: EnvelopeStage::Damp,
                key_on: false,
                damp_config: damp_config(
                    DampTransitionCoordination::IndependentPerOperator,
                    AttackRate15Transition::SkipAttackToDecay,
                ),
            }),
            Err(EnvelopeGeneratorError::InvalidState)
        );
        assert_eq!(
            EnvelopeGenerator::restore_state(EnvelopeGeneratorState {
                format_version: 2,
                level: 128,
                stage: EnvelopeStage::Damp,
                key_on: false,
                damp_config: damp_config(
                    DampTransitionCoordination::IndependentPerOperator,
                    AttackRate15Transition::SkipAttackToDecay,
                ),
            }),
            Err(EnvelopeGeneratorError::InvalidState)
        );
    }

    #[test]
    fn invalid_parameters_do_not_mutate_state() {
        let mut eg = eg(127);
        let before = eg;
        let mut p = params();
        p.fnum = 0x200;
        assert_eq!(
            eg.tick(0, true, true, p),
            Err(EnvelopeGeneratorError::InvalidParameters)
        );
        assert_eq!(eg, before);
    }

    fn measured_decay_transition_counters(
        effective_rate: u8,
        start_counter: u32,
        transition_count: usize,
    ) -> Vec<u32> {
        let mut generator = EnvelopeGenerator::with_initial_state(
            0,
            EnvelopeStage::Decay,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.decay_rate = effective_rate / 4;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = (effective_rate % 4) * 2;
        p.sustain_level = 15;
        assert_eq!(p.effective_rate(p.decay_rate), effective_rate);

        let mut transitions = Vec::with_capacity(transition_count);
        let mut previous_level = generator.level();
        for counter in start_counter..start_counter + 32_768 {
            generator.tick(counter, true, true, p).unwrap();
            if generator.level() != previous_level {
                transitions.push(counter);
                previous_level = generator.level();
                if transitions.len() == transition_count {
                    break;
                }
            }
        }
        transitions
    }

    #[test]
    fn measured_decay_rate_four_matches_8192_sample_spacing() {
        // andete measured RATE=4 as one EG increment every 8192 samples.
        // Start at selector column 1 so the first transition is observable.
        assert_eq!(
            measured_decay_transition_counters(4, 4_096, 3),
            [4_096, 12_288, 20_480]
        );
    }

    #[test]
    fn measured_decay_rate_nine_matches_published_long_short_pattern() {
        // Published RATE=9 sequence: 4096, 4096, 4096, 2048, 2048 samples.
        // Start at selector column 5 to make that measured phase visible.
        assert_eq!(
            measured_decay_transition_counters(9, 10_240, 6),
            [10_240, 14_336, 18_432, 22_528, 24_576, 26_624]
        );
    }

    #[test]
    fn measured_decay_rate_fourteen_matches_2048_1024_1024_pattern() {
        // Published RATE=14 sequence, with the shared counter phase selected
        // at selector column 3: the first transition is immediate, then the
        // measured intervals are 2048, 1024, 1024 samples.
        assert_eq!(
            measured_decay_transition_counters(14, 3_072, 5),
            [3_072, 5_120, 6_144, 7_168, 9_216]
        );
    }

    #[test]
    fn measured_decay_rate_nineteen_matches_1024_then_six_512_intervals() {
        // Published RATE=19 sequence; start at selector column 7 to expose
        // the long wrap interval followed by the six short intervals.
        assert_eq!(
            measured_decay_transition_counters(19, 3_584, 7),
            [3_584, 4_608, 5_120, 5_632, 6_144, 6_656, 7_168]
        );
    }

    #[test]
    fn carrier_release_uses_the_measured_decay_rate_selector() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            0,
            EnvelopeStage::Decay,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.release_rate = 3;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 4;
        p.sustain_level = 15;
        assert_eq!(p.effective_rate(p.release_rate), 14);

        let mut transitions = Vec::new();
        let mut previous_level = generator.level();
        for counter in 3_072..=9_216 {
            generator.tick(counter, false, true, p).unwrap();
            if generator.level() != previous_level {
                transitions.push(counter);
                previous_level = generator.level();
            }
        }

        assert_eq!(transitions, [3_072, 5_120, 6_144, 7_168, 9_216]);
    }

    #[test]
    fn measured_attack_rate_four_uses_the_shared_counter_selector() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            127,
            EnvelopeStage::Attack,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.attack_rate = 1;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;
        assert_eq!(p.effective_rate(p.attack_rate), 4);

        let mut transitions = Vec::new();
        let mut previous_level = generator.level();
        for counter in 4_096..=20_480 {
            generator.tick(counter, true, true, p).unwrap();
            if generator.level() != previous_level {
                transitions.push((counter, generator.level()));
                previous_level = generator.level();
                if transitions.len() == 8 {
                    break;
                }
            }
        }

        assert_eq!(
            transitions,
            [
                (4_096, 119),
                (4_097, 111),
                (4_098, 104),
                (4_099, 97),
                (12_288, 90),
                (12_289, 84),
                (12_290, 78),
                (12_291, 73),
            ]
        );
    }

    #[test]
    fn damp_completion_emits_one_candidate_phase_reset_event() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            124,
            EnvelopeStage::Damp,
            false,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::EnterAttack,
            ),
        )
        .unwrap();

        let first = generator.tick_with_events(0, true, true, params()).unwrap();
        assert_eq!(first.level, 124);
        assert!(first.phase_reset_required);
        assert_eq!(generator.stage(), EnvelopeStage::Attack);

        let second = generator.tick_with_events(1, true, true, params()).unwrap();
        assert!(!second.phase_reset_required);
    }

    #[test]
    fn damp_completion_with_attack_rate_fifteen_emits_reset_and_skips_attack() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            124,
            EnvelopeStage::Damp,
            false,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.attack_rate = 15;

        let result = generator.tick_with_events(0, true, true, p).unwrap();
        assert!(result.phase_reset_required);
        assert_eq!(result.level, 124);
        assert_eq!(generator.stage(), EnvelopeStage::Decay);
    }

    #[test]
    fn key_on_below_damp_threshold_does_not_emit_phase_reset() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            100,
            EnvelopeStage::Sustain,
            false,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::EnterAttack,
            ),
        )
        .unwrap();

        let result = generator.tick_with_events(0, true, true, params()).unwrap();
        assert!(!result.phase_reset_required);
        assert_eq!(generator.stage(), EnvelopeStage::Damp);
    }

    #[test]
    fn dynamic_attack_rate_change_takes_effect_on_next_matching_counter_gate() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            64,
            EnvelopeStage::Attack,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.attack_rate = 0;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;

        generator.tick(0, true, true, p).unwrap();
        assert_eq!(generator.level(), 64);

        p.attack_rate = 1; // RATE=4: selector column 1 advances at counter 4096.
        generator.tick(4_096, true, true, p).unwrap();
        assert_eq!(generator.level(), 59);
        assert_eq!(generator.stage(), EnvelopeStage::Attack);
    }

    #[test]
    fn dynamic_decay_rate_change_takes_effect_without_restarting_decay() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            16,
            EnvelopeStage::Decay,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.decay_rate = 0;
        p.sustain_level = 15;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;

        generator.tick(0, true, true, p).unwrap();
        assert_eq!(generator.level(), 16);
        assert_eq!(generator.stage(), EnvelopeStage::Decay);

        p.decay_rate = 1; // RATE=4; counter 4096 selects a +1 step.
        generator.tick(4_096, true, true, p).unwrap();
        assert_eq!(generator.level(), 17);
        assert_eq!(generator.stage(), EnvelopeStage::Decay);
    }

    #[test]
    fn dynamic_release_rate_change_takes_effect_in_modulator_first_release() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            20,
            EnvelopeStage::Sustain,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.sustained_tone = false;
        p.release_rate = 0;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;

        generator.tick(0, true, false, p).unwrap();
        assert_eq!(generator.level(), 20);

        p.release_rate = 1; // The percussive first-release path consumes RR live.
        generator.tick(4_096, true, false, p).unwrap();
        assert_eq!(generator.level(), 21);
        assert_eq!(generator.stage(), EnvelopeStage::Sustain);
    }

    #[test]
    fn dynamic_release_rate_change_takes_effect_in_sustained_carrier_key_off() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            20,
            EnvelopeStage::Sustain,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.sustained_tone = true;
        p.sustain_on = false;
        p.release_rate = 0;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;

        generator.tick(0, false, true, p).unwrap();
        assert_eq!(generator.level(), 20);

        p.release_rate = 1; // EG-TYPE=1/SUS=OFF selects the programmed RR.
        generator.tick(4_096, false, true, p).unwrap();
        assert_eq!(generator.level(), 21);
    }

    #[test]
    fn dynamic_eg_type_switches_between_sustain_and_first_release_behavior() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            32,
            EnvelopeStage::Sustain,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.sustained_tone = true;
        p.release_rate = 1;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;

        generator.tick(0, true, true, p).unwrap();
        assert_eq!(generator.level(), 32);

        p.sustained_tone = false; // The same state now uses RR, as percussive RR.
        generator.tick(4_096, true, true, p).unwrap();
        assert_eq!(generator.level(), 33);

        p.sustained_tone = true; // Switching back to sustained holds the level.
        generator.tick(8_192, true, true, p).unwrap();
        assert_eq!(generator.level(), 33);
        assert_eq!(generator.stage(), EnvelopeStage::Sustain);
    }

    #[test]
    fn dynamic_eg_type_switches_carrier_second_release_rate_selection() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            20,
            EnvelopeStage::Sustain,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.sustained_tone = true;
        p.sustain_on = false;
        p.release_rate = 0;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;

        generator.tick(0, false, true, p).unwrap();
        assert_eq!(generator.level(), 20);

        p.sustained_tone = false; // EG-TYPE=0/SUS=OFF selects fixed base rate 7.
        generator.tick(64, false, true, p).unwrap();
        assert_eq!(generator.level(), 21);

        p.sustained_tone = true; // EG-TYPE=1/SUS=OFF returns to programmed RR=0.
        generator.tick(128, false, true, p).unwrap();
        assert_eq!(generator.level(), 21);
    }

    #[test]
    fn dynamic_sus_change_takes_effect_during_carrier_second_release() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            20,
            EnvelopeStage::Sustain,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.sustained_tone = true;
        p.sustain_on = false;
        p.release_rate = 0;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;

        generator.tick(0, false, true, p).unwrap();
        assert_eq!(generator.level(), 20);

        p.sustain_on = true; // SUS changes the active release rate to fixed base 5.
        generator.tick(256, false, true, p).unwrap();
        assert_eq!(generator.level(), 21);

        p.sustain_on = false; // Back to programmed RR=0 without a new KEY-OFF edge.
        generator.tick(512, false, true, p).unwrap();
        assert_eq!(generator.level(), 21);
    }

    #[test]
    fn dynamic_sustain_level_change_during_decay_is_used_as_live_stop_condition() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            33,
            EnvelopeStage::Decay,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.sustained_tone = true;
        p.decay_rate = 0;
        p.sustain_level = 3;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;

        generator.tick(0, true, true, p).unwrap();
        assert_eq!(generator.stage(), EnvelopeStage::Decay);
        assert_eq!(generator.level(), 33);

        p.sustain_level = 4; // Current level 33 belongs to the candidate SL=4 bucket.
        generator.tick(1, true, true, p).unwrap();
        assert_eq!(generator.stage(), EnvelopeStage::Sustain);
        assert_eq!(generator.level(), 33);
    }

    #[test]
    fn dynamic_sustain_level_change_after_sustain_entry_does_not_move_level() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            32,
            EnvelopeStage::Sustain,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.sustained_tone = true;
        p.sustain_level = 4;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;

        p.sustain_level = 10;
        generator.tick(0, true, true, p).unwrap();
        assert_eq!(generator.stage(), EnvelopeStage::Sustain);
        assert_eq!(generator.level(), 32);
    }

    #[test]
    fn sustain_level_one_candidate_comparator_accepts_observed_odd_level_nine() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            9,
            EnvelopeStage::Decay,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.sustained_tone = true;
        p.decay_rate = 0;
        p.sustain_level = 1;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;

        generator.tick(0, true, true, p).unwrap();
        assert_eq!(generator.level(), 9);
        assert_eq!(generator.stage(), EnvelopeStage::Sustain);
    }


    #[test]
    fn dynamic_sustain_level_change_to_already_passed_target_keeps_decay_active() {
        let mut generator = EnvelopeGenerator::with_initial_state(
            33,
            EnvelopeStage::Decay,
            true,
            damp_config(
                DampTransitionCoordination::IndependentPerOperator,
                AttackRate15Transition::SkipAttackToDecay,
            ),
        )
        .unwrap();
        let mut p = params();
        p.sustained_tone = true;
        p.decay_rate = 0;
        p.sustain_level = 3;
        p.key_scale_rate = false;
        p.fnum = 0;
        p.block = 0;

        generator.tick(0, true, true, p).unwrap();
        assert_eq!(generator.level(), 33);
        assert_eq!(generator.stage(), EnvelopeStage::Decay);

        p.sustain_level = 2; // SL=2 is already below the current attenuation level.
        generator.tick(1, true, true, p).unwrap();
        assert_eq!(generator.level(), 33);
        assert_eq!(generator.stage(), EnvelopeStage::Decay);
    }

}
