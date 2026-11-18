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

    /// Compute RATE = 4 * R + Rks, clipped to the six-bit range.
    pub const fn effective_rate(self, base_rate: u8) -> u8 {
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

/// One operator EG. The caller supplies the shared global sample counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeGenerator {
    level: u8,
    stage: EnvelopeStage,
    key_on: bool,
    damp_config: DampTransitionConfig,
}

impl EnvelopeGenerator {
    /// Construct from an explicit initial level and explicit DAMP candidate.
    pub const fn with_level(
        level: u8,
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
            stage: EnvelopeStage::Damp,
            key_on: false,
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
            self.level = 0;
            self.stage = EnvelopeStage::Decay;
        } else {
            self.stage = EnvelopeStage::Attack;
        }
    }

    /// Advance one operator update using the externally owned counter value.
    ///
    /// The carrier receives the measured second-release override. The
    /// modulator retains its first-release/sustain path. DAMP coordination and
    /// the AR=15 boundary behavior are explicit candidate configuration.
    pub fn tick(
        &mut self,
        global_counter: u32,
        key_on: bool,
        is_carrier: bool,
        parameters: EnvelopeParameters,
    ) -> Result<u8, EnvelopeGeneratorError> {
        if !parameters.is_valid() {
            return Err(EnvelopeGeneratorError::InvalidParameters);
        }

        if key_on && !self.key_on {
            self.stage = EnvelopeStage::Damp;
        }
        self.key_on = key_on;

        if !key_on && is_carrier {
            let base_rate = if parameters.sustained_tone {
                parameters.release_rate
            } else if parameters.sustain_on {
                5
            } else {
                7
            };
            self.advance_decay(global_counter, parameters.effective_rate(base_rate));
            return Ok(self.level);
        }

        if self.owns_damp_transition(is_carrier)
            && self.stage == EnvelopeStage::Damp
            && self.level >= self.damp_config.threshold
        {
            self.complete_damp(parameters.attack_rate);
        }

        match self.stage {
            EnvelopeStage::Damp => {
                self.advance_decay(global_counter, parameters.effective_rate(12));
                if self.owns_damp_transition(is_carrier)
                    && self.level >= self.damp_config.threshold
                {
                    self.complete_damp(parameters.attack_rate);
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
        Ok(self.level)
    }

    /// Apply the carrier's DAMP transition under the CarrierDrivesBoth candidate.
    ///
    /// This is not called automatically: the exact modulator/carrier transition
    /// offset is unresolved. Call after the carrier update only when the owning
    /// model explicitly selects CarrierDrivesBoth. PG phase reset remains external.
    pub fn synchronize_damp_transition(&mut self, carrier: &Self) {
        if self.damp_config.coordination == DampTransitionCoordination::CarrierDrivesBoth
            && self.stage == EnvelopeStage::Damp
            && carrier.stage != EnvelopeStage::Damp
        {
            self.stage = carrier.stage;
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

        let increment = match rate / 4 {
            0..=12 => {
                let shift = 13 - (rate / 4);
                let mask = (1_u32 << shift) - 1;
                if counter & mask != 0 {
                    return;
                }
                let column = ((counter >> shift) & 7) as usize;
                EG_STEP_TABLE[(rate & 3) as usize][column]
            }
            13 => {
                let column = (((counter & 0x0c) >> 1) | (counter & 1)) as usize;
                EG_STEP_TABLE[(rate & 3) as usize][column]
            }
            14 => {
                let column = ((counter & 0x0c) >> 1) as usize;
                EG_STEP_TABLE[(rate & 3) as usize][column] + 1
            }
            _ => 2,
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

/// Measured eight-step rate selector for attack and decay timing.
const EG_STEP_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 1, 0, 1, 0, 1],
    [0, 1, 0, 1, 1, 1, 0, 1],
    [0, 1, 1, 1, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 1],
];

#[cfg(test)]
mod tests {
    use super::{
        EnvelopeGenerator, EnvelopeGeneratorError, EnvelopeGeneratorState, EnvelopeParameters,
        EnvelopeStage,
    };

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
        let mut eg = EnvelopeGenerator::with_level(127).unwrap();
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
        let mut eg = EnvelopeGenerator::with_level(32).unwrap();
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
    fn fastest_decay_advances_two_levels_per_update() {
        let mut eg = EnvelopeGenerator::with_level(10).unwrap();
        eg.stage = EnvelopeStage::Decay;
        eg.key_on = true;
        let mut p = params();
        p.decay_rate = 15;
        eg.tick(1, true, true, p).unwrap();
        assert_eq!(eg.level(), 12);
    }

    #[test]
    fn modulator_damp_transition_follows_carrier_boundary() {
        let mut modulator = EnvelopeGenerator::with_level(127).unwrap();
        let mut carrier = EnvelopeGenerator::with_level(127).unwrap();
        let mut p = params();
        p.attack_rate = 10;

        // The modulator cannot independently leave DAMP.
        modulator.tick(0, true, false, p).unwrap();
        assert_eq!(modulator.stage(), EnvelopeStage::Damp);

        // Carrier update owns the shared DAMP -> ATTACK transition.
        carrier.tick(0, true, true, p).unwrap();
        assert_eq!(carrier.stage(), EnvelopeStage::Attack);
        modulator.synchronize_damp_transition(&carrier);
        assert_eq!(modulator.stage(), EnvelopeStage::Attack);
    }

    #[test]
    fn damp_and_attack_fifteen_shortcut_are_explicit() {
        let mut eg = EnvelopeGenerator::with_level(124).unwrap();
        let mut p = params();
        p.attack_rate = 15;
        eg.tick(0, true, true, p).unwrap();
        assert_eq!(eg.level(), 0);
        assert_eq!(eg.stage(), EnvelopeStage::Decay);
    }

    #[test]
    fn sustain_transition_uses_four_bit_level_comparison() {
        let mut eg = EnvelopeGenerator::with_level(31).unwrap();
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
        let mut eg = EnvelopeGenerator::with_level(0).unwrap();
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
    fn key_off_percussive_release_uses_sus_rate_five() {
        let mut eg = EnvelopeGenerator::with_level(20).unwrap();
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
    fn snapshot_round_trip_preserves_complete_local_state() {
        let mut eg = EnvelopeGenerator::with_level(61).unwrap();
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
            }),
            Err(EnvelopeGeneratorError::InvalidState)
        );
        assert_eq!(
            EnvelopeGenerator::restore_state(EnvelopeGeneratorState {
                format_version: 1,
                level: 128,
                stage: EnvelopeStage::Damp,
                key_on: false,
            }),
            Err(EnvelopeGeneratorError::InvalidState)
        );
    }

    #[test]
    fn invalid_parameters_do_not_mutate_state() {
        let mut eg = EnvelopeGenerator::with_level(127).unwrap();
        let before = eg;
        let mut p = params();
        p.fnum = 0x200;
        assert_eq!(
            eg.tick(0, true, true, p),
            Err(EnvelopeGeneratorError::InvalidParameters)
        );
        assert_eq!(eg, before);
    }
}
