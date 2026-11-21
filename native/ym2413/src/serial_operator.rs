//! YM2413 serial operator datapath and its delayed parameter/output latches.
//!
//! This is the serial OPLL_Operator stage, not a sample-domain two-operator
//! approximation. It owns the log/EXP pipeline, feedback histories, waveform
//! delay bits, and the EG-output/KSL-TL latches consumed between serial slots.
//! Slot scheduling, PG advancement, and the authoritative EG sequencer remain
//! owned by Ym2413Core and the serial slot service.

use crate::synthesis_tables::{YM2413_EXP_TABLE, YM2413_LOG_SINE_TABLE};

pub const YM2413_SERIAL_OPERATOR_STATE_VERSION: u16 = 1;
pub const YM2413_SERIAL_OPERATOR_FEEDBACK_SLOTS: usize = 9;
pub const YM2413_SERIAL_OPERATOR_SLOTS: u8 = 18;

/// Errors returned by the serial operator datapath.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413SerialOperatorError {
    /// The Nuked-compatible serial cycle must be in 0..18.
    InvalidCycle(u8),
    /// PG output is the eleven-bit value produced by PhaseGenerate.
    InvalidPgOutput(u16),
    /// KSL attenuation must be in the 0..112 EG domain.
    InvalidKslAttenuation(u8),
    /// The TL latch is six bits wide.
    InvalidTotalLevel(u8),
    /// Feedback is a three-bit field.
    InvalidFeedback(u8),
    /// An EG level is outside the seven-bit 0..127 range.
    InvalidEnvelopeLevel(u8),
    /// The saved state has an unsupported version or impossible latch values.
    InvalidState,
}

/// Complete snapshot of the serial operator and its parameter/output latches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413SerialOperatorState {
    pub format_version: u16,
    /// First delayed modulator output for each of the nine channel feedback rings.
    pub op_fb1: [i16; YM2413_SERIAL_OPERATOR_FEEDBACK_SLOTS],
    /// Second delayed modulator output for each of the nine channel feedback rings.
    pub op_fb2: [i16; YM2413_SERIAL_OPERATOR_FEEDBACK_SLOTS],
    /// Feedback sum latch selected three serial cycles ahead.
    pub op_fbsum: i16,
    /// Nine-bit operator modulation latch.
    pub op_mod: u16,
    /// Two-stage negative-polarity pipeline.
    pub op_neg: u8,
    /// Log-sine ROM output latch.
    pub op_logsin: u16,
    /// EXP ROM mantissa latch.
    pub op_exp_m: u16,
    /// EXP ROM exponent/shift latch.
    pub op_exp_s: u8,
    /// Envelope output consumed by the next OPLL_Operator stage.
    pub eg_out: u8,
    /// Delayed EG silence flag consumed by the next OPLL_Operator stage.
    pub eg_silent: bool,
    /// KSL+TL attenuation latch consumed by EnvelopeOutput.
    pub eg_ksltl: u16,
    /// PreparePatch1 total-level latch.
    pub c_tl: u8,
    /// PreparePatch1 KSL attenuation, already converted to EG units.
    pub c_ksl_attenuation: u8,
    /// PreparePatch2 feedback latch.
    pub c_fb: u8,
    /// PreparePatch2 delayed carrier waveform bits.
    pub c_dc: u8,
    /// PreparePatch2 delayed modulator waveform bits.
    pub c_dm: u8,
    /// PreparePatch2 AM-enable latch for the currently selected operator.
    pub c_am: bool,
    /// Last raw signed operator output produced by OPLL_Operator.
    pub last_operator_output: i16,
    /// Last melodic channel-output latch produced by OPLL_Operator.
    pub last_channel_output: i16,
    /// Last mirrored ten-bit phase address.
    pub last_phase_index: u16,
    /// Last effective feedback phase contribution before PG addition.
    pub last_feedback_phase: i32,
}

impl Default for Ym2413SerialOperatorState {
    fn default() -> Self {
        Self {
            format_version: YM2413_SERIAL_OPERATOR_STATE_VERSION,
            op_fb1: [0; YM2413_SERIAL_OPERATOR_FEEDBACK_SLOTS],
            op_fb2: [0; YM2413_SERIAL_OPERATOR_FEEDBACK_SLOTS],
            op_fbsum: 0,
            op_mod: 0,
            op_neg: 0,
            op_logsin: 0,
            op_exp_m: 0,
            op_exp_s: 0,
            eg_out: 0,
            eg_silent: false,
            eg_ksltl: 0,
            c_tl: 0,
            c_ksl_attenuation: 0,
            c_fb: 0,
            c_dc: 0,
            c_dm: 0,
            c_am: false,
            last_operator_output: 0,
            last_channel_output: 0,
            last_phase_index: 0,
            last_feedback_phase: 0,
        }
    }
}

/// Observable result of one serial OPLL_Operator invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413SerialOperatorSample {
    /// Raw signed EXP-stage output before channel scaling.
    pub operator_output: i16,
    /// The normal melodic ch_out value; rhythm routing is intentionally separate.
    pub channel_output: i16,
    /// Ten-bit phase address after modulation and waveform mirroring.
    pub phase_index: u16,
    /// Effective feedback phase contribution before adding PG output.
    pub feedback_phase: i32,
}

/// YM2413 serial operator datapath with versioned save-state support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ym2413SerialOperator {
    state: Ym2413SerialOperatorState,
}

impl Ym2413SerialOperator {
    /// Construct the reset candidate state used by Nuked's zero-initialized
    /// operator pipeline. Physical power-on latch values remain unclaimed.
    pub const fn new() -> Self {
        Self {
            state: Ym2413SerialOperatorState {
                format_version: YM2413_SERIAL_OPERATOR_STATE_VERSION,
                op_fb1: [0; YM2413_SERIAL_OPERATOR_FEEDBACK_SLOTS],
                op_fb2: [0; YM2413_SERIAL_OPERATOR_FEEDBACK_SLOTS],
                op_fbsum: 0,
                op_mod: 0,
                op_neg: 0,
                op_logsin: 0,
                op_exp_m: 0,
                op_exp_s: 0,
                eg_out: 0,
                eg_silent: false,
                eg_ksltl: 0,
                c_tl: 0,
                c_ksl_attenuation: 0,
                c_fb: 0,
                c_dc: 0,
                c_dm: 0,
                c_am: false,
                last_operator_output: 0,
                last_channel_output: 0,
                last_phase_index: 0,
                last_feedback_phase: 0,
            },
        }
    }

    /// Return an immutable view of every serial operator latch.
    pub const fn state(&self) -> &Ym2413SerialOperatorState {
        &self.state
    }

    /// Apply the PreparePatch1 KSL/TL latches for the selected operator.
    ///
    /// The caller supplies KSL already converted by the shared channel decoder.
    /// The resulting EG attenuation is consumed only by a later EnvelopeOutput
    /// stage; it does not retroactively change the current operator evaluation.
    pub fn prepare_patch1(
        &mut self,
        ksl_attenuation: u8,
        total_level: u8,
    ) -> Result<(), Ym2413SerialOperatorError> {
        if ksl_attenuation > 112 {
            return Err(Ym2413SerialOperatorError::InvalidKslAttenuation(
                ksl_attenuation,
            ));
        }
        if total_level > 63 {
            return Err(Ym2413SerialOperatorError::InvalidTotalLevel(total_level));
        }
        self.state.c_ksl_attenuation = ksl_attenuation;
        self.state.c_tl = total_level;
        Ok(())
    }

    /// Apply the PreparePatch2 feedback, waveform and AM parameter latches.
    ///
    /// DC/DM are shifted serial latches, not instantaneous booleans. Their
    /// delayed bit 2 is consumed by OPLL_Operator on later serial cycles.
    pub fn prepare_patch2(
        &mut self,
        feedback: u8,
        dc_waveform: bool,
        dm_waveform: bool,
        am_enabled: bool,
    ) -> Result<(), Ym2413SerialOperatorError> {
        if feedback > 7 {
            return Err(Ym2413SerialOperatorError::InvalidFeedback(feedback));
        }
        self.state.c_fb = feedback;
        self.state.c_dc = self.state.c_dc.wrapping_shl(1) | u8::from(dc_waveform);
        self.state.c_dm = self.state.c_dm.wrapping_shl(1) | u8::from(dm_waveform);
        self.state.c_am = am_enabled;
        Ok(())
    }

    /// Execute one Nuked-compatible serial OPLL_Operator stage.
    ///
    /// The caller must first service PhaseGenerate for this cycle. The current
    /// EG output and silence latches are deliberately consumed as-is; the
    /// caller updates them only after this method, matching OPLL_Clock order.
    /// Rhythm-specific output routing is excluded from this melodic FM-3 stage.
    pub fn tick_operator(
        &mut self,
        cycle: u8,
        pg_out: u16,
    ) -> Result<Ym2413SerialOperatorSample, Ym2413SerialOperatorError> {
        if cycle >= YM2413_SERIAL_OPERATOR_SLOTS {
            return Err(Ym2413SerialOperatorError::InvalidCycle(cycle));
        }
        if pg_out > 0x07ff {
            return Err(Ym2413SerialOperatorError::InvalidPgOutput(pg_out));
        }

        let ismod1 = ((cycle + 1) / 3) & 1 != 0;
        let ismod2 = ((cycle + 3) / 3) & 1 != 0;
        let ismod3 = (cycle / 3) & 1 != 0;

        let mut op_mod = if ismod3 {
            u32::from(self.state.op_mod) << 1
        } else {
            0
        };
        if ismod2 && self.state.c_fb != 0 {
            let shift = 7 - self.state.c_fb;
            op_mod |= (i32::from(self.state.op_fbsum) >> shift) as u32;
        }

        let mut exp_shift = u16::from(self.state.op_exp_s);
        let negative_wave_suppressed = self.state.op_neg & 2 != 0
            && if ismod1 {
                self.state.c_dm & 4 != 0
            } else {
                self.state.c_dc & 4 != 0
            };
        if self.state.eg_silent || negative_wave_suppressed {
            exp_shift |= 12;
        }

        let mut output = (self.state.op_exp_m >> exp_shift) as i16;
        if !self.state.eg_silent && self.state.op_neg & 2 != 0 {
            output = !output;
        }

        let mut level = u32::from(self.state.op_logsin) + (u32::from(self.state.eg_out) << 4);
        if level >= 4096 {
            level = 4095;
        }
        let mantissa_index = (level & 0xff) as usize;
        self.state.op_exp_m = 1024 + YM2413_EXP_TABLE[255 - mantissa_index];
        self.state.op_exp_s = (level >> 8) as u8;

        let mut phase = op_mod.wrapping_add(u32::from(pg_out)) & 0x03ff;
        if phase & 0x0100 != 0 {
            phase ^= 0x00ff;
        }
        self.state.op_logsin = YM2413_LOG_SINE_TABLE[(phase & 0x00ff) as usize];
        self.state.op_neg = self.state.op_neg.wrapping_shl(1) | ((phase >> 9) as u8);

        let feedback_index = usize::from((cycle + 3) % 9);
        let feedback_sum = i32::from(self.state.op_fb1[feedback_index])
            + i32::from(self.state.op_fb2[feedback_index]);
        self.state.op_fbsum = (feedback_sum >> 1) as i16;

        if ismod1 {
            let feedback_index = usize::from(cycle % 9);
            self.state.op_fb2[feedback_index] = self.state.op_fb1[feedback_index];
            self.state.op_fb1[feedback_index] = output;
        }
        self.state.op_mod = (output as u16) & 0x01ff;

        let sample = Ym2413SerialOperatorSample {
            operator_output: output,
            channel_output: if ismod1 { 0 } else { output >> 3 },
            phase_index: phase as u16,
            feedback_phase: op_mod as i32,
        };
        self.state.last_operator_output = sample.operator_output;
        self.state.last_channel_output = sample.channel_output;
        self.state.last_phase_index = sample.phase_index;
        self.state.last_feedback_phase = sample.feedback_phase;
        Ok(sample)
    }

    /// Execute the EnvelopeOutput and EnvelopeKSLTL latches after Operator and
    /// before the core-owned serial EG update.
    ///
    /// The level array is borrowed from Ym2413Core's EnvelopeSequencerState;
    /// this method never advances, reconstructs, or duplicates the EG.
    pub fn advance_envelope_output(
        &mut self,
        cycle: u8,
        eg_levels: &[u8; 18],
        am_level: u8,
        test_mode: u8,
    ) -> Result<(), Ym2413SerialOperatorError> {
        if cycle >= YM2413_SERIAL_OPERATOR_SLOTS {
            return Err(Ym2413SerialOperatorError::InvalidCycle(cycle));
        }
        if am_level > 15 {
            return Err(Ym2413SerialOperatorError::InvalidEnvelopeLevel(am_level));
        }
        if let Some(level) = eg_levels.iter().copied().find(|level| *level > 127) {
            return Err(Ym2413SerialOperatorError::InvalidEnvelopeLevel(level));
        }

        let output_index = usize::from((cycle + 17) % 18);
        let silent_index = usize::from((cycle + 16) % 18);
        let am = if self.state.c_am { am_level } else { 0 };
        let mut output_level =
            u16::from(eg_levels[output_index]) + self.state.eg_ksltl + u16::from(am);
        if output_level >= 128 {
            output_level = 127;
        }
        if test_mode & 0x01 != 0 {
            output_level = 0;
        }

        // These are delayed values for the next OPLL_Operator invocation.
        self.state.eg_out = output_level as u8;
        self.state.eg_silent = eg_levels[silent_index] == 0x7f;

        // OPLL_EnvelopeKSLTL runs after EnvelopeOutput, so the new KSL/TL
        // value is intentionally not used by the output calculated above.
        self.state.eg_ksltl =
            u16::from(self.state.c_ksl_attenuation) + (u16::from(self.state.c_tl) << 1);
        Ok(())
    }

    /// Export all operator, feedback, waveform and envelope-output latches.
    pub const fn export_state(&self) -> Ym2413SerialOperatorState {
        self.state
    }

    /// Restore a serial operator snapshot after validating all latch ranges.
    pub const fn restore_state(
        state: Ym2413SerialOperatorState,
    ) -> Result<Self, Ym2413SerialOperatorError> {
        if state.format_version != YM2413_SERIAL_OPERATOR_STATE_VERSION
            || state.op_mod > 0x01ff
            || state.op_logsin > 0x0fff
            || state.op_exp_m > 0x07ff
            || state.op_exp_s > 15
            || state.eg_out > 127
            || state.eg_ksltl > 238
            || state.c_tl > 63
            || state.c_ksl_attenuation > 112
            || state.c_fb > 7
            || state.last_phase_index > 0x03ff
        {
            return Err(Ym2413SerialOperatorError::InvalidState);
        }
        Ok(Self { state })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        YM2413_SERIAL_OPERATOR_STATE_VERSION, Ym2413SerialOperator, Ym2413SerialOperatorError,
        Ym2413SerialOperatorState,
    };
    use crate::synthesis_tables::YM2413_EXP_TABLE;

    #[test]
    fn exp_rom_reconstruction_matches_the_nuked_2413_table_domain() {
        for index in 0..256 {
            let reconstructed = 1024 + YM2413_EXP_TABLE[255 - index];
            assert!((1024..=2042).contains(&reconstructed));
        }
        assert_eq!(1024 + YM2413_EXP_TABLE[255], 2042);
        assert_eq!(1024 + YM2413_EXP_TABLE[0], 1024);
    }

    #[test]
    fn prepare_patch_latches_are_delayed_and_snapshot_safe() {
        let mut operator = Ym2413SerialOperator::new();
        operator.prepare_patch1(28, 31).unwrap();
        operator.prepare_patch2(5, true, false, true).unwrap();
        let state = operator.export_state();
        assert_eq!(state.c_ksl_attenuation, 28);
        assert_eq!(state.c_tl, 31);
        assert_eq!(state.c_fb, 5);
        assert_eq!(state.c_dc & 1, 1);
        assert_eq!(state.c_dm & 1, 0);
        assert!(state.c_am);
        assert_eq!(Ym2413SerialOperator::restore_state(state), Ok(operator));
    }

    #[test]
    fn waveform_bits_shift_through_the_serial_latch() {
        let mut operator = Ym2413SerialOperator::new();
        operator.prepare_patch2(0, true, false, false).unwrap();
        operator.prepare_patch2(0, false, true, false).unwrap();
        assert_eq!(operator.state().c_dc, 2);
        assert_eq!(operator.state().c_dm, 1);
    }

    #[test]
    fn eleven_bit_pg_output_is_masked_to_ten_bit_phase_address() {
        let mut low = Ym2413SerialOperator::new();
        let mut high = Ym2413SerialOperator::new();
        let low_sample = low.tick_operator(0, 0).unwrap();
        let high_sample = high.tick_operator(0, 0x0400).unwrap();
        assert_eq!(low_sample.phase_index, high_sample.phase_index);
        assert_eq!(low.state().op_logsin, high.state().op_logsin);
    }

    #[test]
    fn feedback_uses_two_previous_operator_outputs() {
        let mut operator = Ym2413SerialOperator::new();
        let state = Ym2413SerialOperatorState {
            op_fb1: [128; 9],
            op_fb2: [64; 9],
            op_fbsum: 0,
            ..Ym2413SerialOperatorState::default()
        };
        operator = Ym2413SerialOperator::restore_state(state).unwrap();
        operator.prepare_patch2(7, false, false, false).unwrap();
        let sample = operator.tick_operator(0, 0).unwrap();
        assert_eq!(sample.feedback_phase, 0);
        assert_eq!(operator.state().op_fbsum, 96);
        assert_eq!(operator.state().op_fb1[0], 128);
        assert_eq!(operator.state().op_fb2[0], 64);
    }

    #[test]
    fn envelope_output_uses_old_ksl_tl_before_latching_new_value() {
        let mut operator = Ym2413SerialOperator::new();
        operator.prepare_patch1(28, 10).unwrap();
        let mut levels = [0_u8; 18];
        levels[17] = 12;
        operator.advance_envelope_output(0, &levels, 0, 0).unwrap();
        assert_eq!(operator.state().eg_out, 12);
        assert_eq!(operator.state().eg_ksltl, 48);
        operator.advance_envelope_output(0, &levels, 0, 0).unwrap();
        assert_eq!(operator.state().eg_out, 60);
    }

    #[test]
    fn operator_pipeline_is_deterministic_and_partition_independent() {
        let mut continuous = Ym2413SerialOperator::new();
        let mut partitioned = continuous;
        for cycle in 0..18 {
            continuous
                .tick_operator(cycle, (cycle as u16) * 17)
                .unwrap();
        }
        for cycle in 0..7 {
            partitioned
                .tick_operator(cycle, (cycle as u16) * 17)
                .unwrap();
        }
        let saved = partitioned.export_state();
        partitioned = Ym2413SerialOperator::restore_state(saved).unwrap();
        for cycle in 7..18 {
            partitioned
                .tick_operator(cycle, (cycle as u16) * 17)
                .unwrap();
        }
        assert_eq!(continuous, partitioned);
    }

    #[test]
    fn invalid_inputs_do_not_mutate_operator_state() {
        let mut operator = Ym2413SerialOperator::new();
        let before = operator;
        assert_eq!(
            operator.tick_operator(18, 0),
            Err(Ym2413SerialOperatorError::InvalidCycle(18))
        );
        assert_eq!(
            operator.tick_operator(0, 2048),
            Err(Ym2413SerialOperatorError::InvalidPgOutput(2048))
        );
        assert_eq!(operator, before);
    }

    #[test]
    fn snapshot_rejects_unknown_version_and_invalid_latches() {
        let mut state = Ym2413SerialOperatorState::default();
        state.format_version = 99;
        assert_eq!(
            Ym2413SerialOperator::restore_state(state),
            Err(Ym2413SerialOperatorError::InvalidState)
        );
        state.format_version = YM2413_SERIAL_OPERATOR_STATE_VERSION;
        state.op_exp_s = 16;
        assert_eq!(
            Ym2413SerialOperator::restore_state(state),
            Err(Ym2413SerialOperatorError::InvalidState)
        );
    }
}
