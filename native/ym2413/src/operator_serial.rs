//! Serial YM2413 operator pipeline state and slot stages.
//!
//! This module models the serial OPLL_Operator / EnvelopeOutput /
//! EnvelopeKSLTL / AM-LFO latches. It consumes the core-owned EG state and
//! never advances a second envelope generator.

use crate::channel::{ChannelSynthesisError, decode_channel_synthesis};
use crate::operator::{serial_exponential_mantissa, serial_exponential_shift, serial_log_sine};
use crate::slot::Ym2413Operator;
use crate::slot_service::Ym2413SlotServiceContext;

pub const YM2413_OPERATOR_SERIAL_STATE_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413OperatorSerialError {
    InvalidState,
    InvalidCycle(u8),
    InvalidEgOutputIndex(u8),
    InvalidEgUpdateIndex(u8),
    Channel(ChannelSynthesisError),
}

impl From<ChannelSynthesisError> for Ym2413OperatorSerialError {
    fn from(value: ChannelSynthesisError) -> Self {
        Self::Channel(value)
    }
}

/// All operator, feedback, EG-output and AM-LFO latches needed for resumption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413OperatorSerialState {
    pub format_version: u16,
    pub op_logsin: u16,
    pub op_neg: u8,
    pub op_exp_m: u16,
    pub op_exp_s: u8,
    pub op_mod: u16,
    pub op_fbsum: i16,
    pub op_fb1: [i16; 9],
    pub op_fb2: [i16; 9],
    pub eg_ksltl: u16,
    pub eg_out: u8,
    pub eg_silent: bool,
    pub lfo_am_counter: u16,
    pub lfo_am_dir: bool,
    pub lfo_am_car: bool,
    pub lfo_am_step: bool,
    pub lfo_am_out: u8,
    pub channel_output: i16,
    pub last_output: i16,
    pub last_phase: u16,
}

impl Ym2413OperatorSerialState {
    const RESET: Self = Self {
        format_version: YM2413_OPERATOR_SERIAL_STATE_VERSION,
        op_logsin: 0,
        op_neg: 0,
        op_exp_m: 0,
        op_exp_s: 0,
        op_mod: 0,
        op_fbsum: 0,
        op_fb1: [0; 9],
        op_fb2: [0; 9],
        eg_ksltl: 0,
        eg_out: 0x7f,
        eg_silent: false,
        lfo_am_counter: 0,
        lfo_am_dir: false,
        lfo_am_car: false,
        lfo_am_step: false,
        lfo_am_out: 0,
        channel_output: 0,
        last_output: 0,
        last_phase: 0,
    };

    const fn is_valid(self) -> bool {
        self.format_version == YM2413_OPERATOR_SERIAL_STATE_VERSION
            && self.op_logsin <= 0x0fff
            && self.op_exp_m <= 0x07ff
            && self.op_exp_s <= 0x0f
            && self.op_mod <= 0x01ff
            && self.eg_ksltl <= 0x00ff
            && self.eg_out <= 0x7f
            && self.lfo_am_counter <= 0x01ff
            && self.lfo_am_out <= 0x0f
            && self.last_phase <= 0x03ff
    }
}

/// Cloneable serial operator state machine. All mutable state is owned data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413OperatorSerialService {
    state: Ym2413OperatorSerialState,
}

impl Default for Ym2413OperatorSerialService {
    fn default() -> Self {
        Self::new()
    }
}

impl Ym2413OperatorSerialService {
    /// Construct the reset serial operator pipeline.
    pub const fn new() -> Self {
        Self {
            state: Ym2413OperatorSerialState::RESET,
        }
    }

    /// Export every serial operator, feedback, EG-output and AM-LFO latch.
    pub const fn export_state(&self) -> Ym2413OperatorSerialState {
        self.state
    }

    /// Restore a complete serial operator snapshot after validating its latches.
    pub const fn restore_state(
        state: Ym2413OperatorSerialState,
    ) -> Result<Self, Ym2413OperatorSerialError> {
        if !state.is_valid() {
            return Err(Ym2413OperatorSerialError::InvalidState);
        }
        Ok(Self { state })
    }

    pub const fn state(&self) -> Ym2413OperatorSerialState {
        self.state
    }

    pub const fn eg_out(&self) -> u8 {
        self.state.eg_out
    }

    pub const fn eg_silent(&self) -> bool {
        self.state.eg_silent
    }

    pub const fn eg_ksltl(&self) -> u16 {
        self.state.eg_ksltl
    }

    pub const fn lfo_am_counter(&self) -> u16 {
        self.state.lfo_am_counter
    }

    pub const fn lfo_am_output(&self) -> u8 {
        self.state.lfo_am_out
    }

    pub const fn last_output(&self) -> i16 {
        self.state.last_output
    }

    pub const fn last_phase(&self) -> u16 {
        self.state.last_phase
    }

    pub const fn channel_output(&self) -> i16 {
        self.state.channel_output
    }

    /// Sample the AM-LFO output latch at the beginning of serial cycle zero.
    pub(crate) fn sample_am_lfo_output(&mut self, cycle: u8) -> Result<(), Ym2413OperatorSerialError> {
        if cycle >= 18 {
            return Err(Ym2413OperatorSerialError::InvalidCycle(cycle));
        }
        if cycle == 0 {
            self.state.lfo_am_out = ((self.state.lfo_am_counter >> 3) & 0x0f) as u8;
        }
        Ok(())
    }

    /// PreparePatch1: calculate the KSL/TL value that will be latched after
    /// the current operator and EnvelopeOutput stages.
    pub(crate) fn prepare_patch1(
        &self,
        context: &Ym2413SlotServiceContext,
    ) -> Result<u16, Ym2413OperatorSerialError> {
        let descriptor = context.event.selected_operator;
        let config = decode_channel_synthesis(&context.registers, descriptor.channel)?;
        let pair = config.operator_pair;
        let (total_level, key_scale) = match descriptor.operator {
            Ym2413Operator::Modulator => (pair.modulator_total_level, pair.modulator_ksl),
            Ym2413Operator::Carrier => (pair.carrier_volume << 2, pair.carrier_ksl),
        };
        Ok(u16::from(key_scale) + (u16::from(total_level) << 1))
    }

    /// OPLL_Operator: consume the previous EXP/EG/feedback latches and current
    /// PG output, then advance the serial operator's internal latches.
    pub(crate) fn clock_operator(
        &mut self,
        cycle: u8,
        pg_output: u16,
        feedback: u8,
    ) -> Result<(), Ym2413OperatorSerialError> {
        if cycle >= 18 {
            return Err(Ym2413OperatorSerialError::InvalidCycle(cycle));
        }
        if pg_output > 0x03ff || feedback > 7 {
            return Err(Ym2413OperatorSerialError::InvalidState);
        }

        let ismod1 = ((cycle + 1) / 3) & 1 != 0;
        let ismod2 = ((cycle + 3) / 3) & 1 != 0;
        let ismod3 = (cycle / 3) & 1 != 0;

        // The phase-modulation and feedback inputs are delayed global latches.
        let mut op_mod = if ismod3 {
            u32::from(self.state.op_mod) << 1
        } else {
            0
        };
        if ismod2 && feedback != 0 {
            op_mod |= (self.state.op_fbsum >> (7 - feedback)) as i32 as u32;
        }

        let mut exp_shift = u16::from(self.state.op_exp_s);
        if self.state.eg_silent {
            exp_shift |= 12;
        }
        let mut output_bits = self.state.op_exp_m >> exp_shift;
        if !self.state.eg_silent && (self.state.op_neg & 0x02) != 0 {
            output_bits = !output_bits;
        }
        let output = output_bits as i16;

        let level = (u32::from(self.state.op_logsin) + (u32::from(self.state.eg_out) << 4))
            .min(4095) as u16;
        self.state.op_exp_m = serial_exponential_mantissa(level);
        self.state.op_exp_s = serial_exponential_shift(level);

        let mut phase = (op_mod + u32::from(pg_output)) & 0x03ff;
        self.state.op_logsin = serial_log_sine(phase as u16);
        if phase & 0x0100 != 0 {
            phase ^= 0x00ff;
        }
        self.state.op_neg = self.state.op_neg.wrapping_shl(1) | ((phase >> 9) as u8);

        let feedback_index = usize::from((cycle + 3) % 9);
        self.state.op_fbsum = ((i32::from(self.state.op_fb1[feedback_index])
            + i32::from(self.state.op_fb2[feedback_index]))
            >> 1) as i16;

        if ismod1 {
            let index = usize::from(cycle % 9);
            self.state.op_fb2[index] = self.state.op_fb1[index];
            self.state.op_fb1[index] = output;
        }
        self.state.op_mod = output_bits & 0x01ff;

        // Rhythm routing is intentionally not active in this melodic-only gate.
        self.state.channel_output = if ismod1 { 0 } else { output >> 3 };
        self.state.last_output = output;
        self.state.last_phase = phase as u16;
        Ok(())
    }

    /// EnvelopeOutput: latch the next operator attenuation from the core-owned
    /// pre-update EG state, the previous KSL/TL latch and the previous AM latch.
    pub(crate) fn clock_envelope_output(
        &mut self,
        context: &Ym2413SlotServiceContext,
        am_enabled: bool,
    ) -> Result<(), Ym2413OperatorSerialError> {
        let index = context.event.eg_output_index;
        let operator = context
            .envelope_state
            .operators
            .get(usize::from(index))
            .ok_or(Ym2413OperatorSerialError::InvalidEgOutputIndex(index))?;
        let mut level = u16::from(operator.level) + self.state.eg_ksltl;
        if am_enabled {
            level += u16::from(self.state.lfo_am_out);
        }
        if level >= 128 {
            level = 127;
        }
        if context.registers[0x0f] & 0x01 != 0 {
            level = 0;
        }
        self.state.eg_out = level as u8;
        Ok(())
    }

    /// Capture the EG silence latch from the core-owned pre-update operator.
    pub(crate) fn clock_eg_silent(
        &mut self,
        context: &Ym2413SlotServiceContext,
    ) -> Result<(), Ym2413OperatorSerialError> {
        let index = context.event.eg_update_index;
        let operator = context
            .envelope_state
            .operators
            .get(usize::from(index))
            .ok_or(Ym2413OperatorSerialError::InvalidEgUpdateIndex(index))?;
        self.state.eg_silent = operator.level == 0x7f;
        Ok(())
    }

    /// EnvelopeKSLTL: publish the value prepared by the current PreparePatch1.
    pub(crate) const fn latch_eg_ksltl(&mut self, value: u16) {
        self.state.eg_ksltl = value;
    }

    /// DoLFO AM: advance the serial AM shift register after the core EG tick.
    /// pm_counter is sampled before the PG service increments it at cycle 17.
    pub(crate) fn clock_am_lfo(
        &mut self,
        cycle: u8,
        testmode: u8,
        pm_counter: u16,
    ) -> Result<(), Ym2413OperatorSerialError> {
        if cycle >= 18 {
            return Err(Ym2413OperatorSerialError::InvalidCycle(cycle));
        }
        if cycle == 17 {
            self.state.lfo_am_step = ((pm_counter & 0x003f) + 1) >> 6 != 0;
        }

        let mut am_increment = 0u8;
        if (self.state.lfo_am_step || (testmode & 0x08) != 0) && cycle < 9 {
            am_increment = u8::from(self.state.lfo_am_dir) | u8::from(cycle == 0);
        }

        if cycle >= 9 {
            self.state.lfo_am_car = false;
        }

        if cycle == 0 {
            if self.state.lfo_am_dir && (self.state.lfo_am_counter & 0x007f) == 0 {
                self.state.lfo_am_dir = false;
            } else if !self.state.lfo_am_dir && (self.state.lfo_am_counter & 0x0069) == 0x0069 {
                self.state.lfo_am_dir = true;
            }
        }

        let mut am_bit = (self.state.lfo_am_counter & 1)
            + u16::from(am_increment)
            + if self.state.lfo_am_car { 1 } else { 0 };
        self.state.lfo_am_car = am_bit >> 1 != 0;
        am_bit &= 1;
        self.state.lfo_am_counter = (am_bit << 8) | (self.state.lfo_am_counter >> 1);

        if testmode & 0x02 != 0 {
            self.state.lfo_am_dir = false;
            self.state.lfo_am_counter &= 0x00ff;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Ym2413OperatorSerialError, Ym2413OperatorSerialService};
    use crate::envelope_sequencer::EnvelopeSequencer;
    use crate::slot::nuked_slot_service_event;
    use crate::slot_service::Ym2413SlotServiceContext;

    fn context(cycle: u8, testmode: u8) -> Ym2413SlotServiceContext {
        let eg = EnvelopeSequencer::new();
        let state = eg.export_state();
        let event = nuked_slot_service_event(cycle, 0, state.eg_dokon).unwrap();
        let mut registers = [0; 64];
        registers[0x0f] = testmode;
        Ym2413SlotServiceContext {
            event,
            registers,
            envelope_input: crate::envelope_sequencer::EnvelopeSlotInput::default(),
            envelope_state: state,
            rm_enable: 0,
            rm_select: 6,
        }
    }

    #[test]
    fn reset_state_contains_nuked_operator_latch_defaults() {
        let service = Ym2413OperatorSerialService::new();
        let state = service.export_state();
        assert_eq!(state.eg_out, 0x7f);
        assert!(!state.eg_silent);
        assert_eq!(state.op_exp_m, 0);
        assert_eq!(state.lfo_am_counter, 0);
    }

    #[test]
    fn operator_clock_updates_exp_log_phase_and_feedback_latches() {
        let mut service = Ym2413OperatorSerialService::new();
        service.clock_operator(0, 0x0123, 0).unwrap();
        let state = service.export_state();
        assert_eq!(state.last_phase, 0x01dc);
        assert_eq!(state.op_logsin, crate::operator::serial_log_sine(0x0123));
        assert_eq!(state.op_exp_m, crate::operator::serial_exponential_mantissa(0x07f0));
        assert_eq!(state.op_exp_s, crate::operator::serial_exponential_shift(0x07f0));
    }

    #[test]
    fn feedback_history_is_written_only_on_serial_carrier_cycles() {
        let mut service = Ym2413OperatorSerialService::new();
        let mut initial = service.export_state();
        initial.op_exp_m = 0x07fa;
        initial.eg_out = 0;
        service = Ym2413OperatorSerialService::restore_state(initial).unwrap();
        service.clock_operator(2, 0x0010, 0).unwrap();
        let after_carrier = service.export_state();
        assert_ne!(after_carrier.op_fb1[2], 0);
        service.clock_operator(5, 0x0020, 0).unwrap();
        let after_modulator = service.export_state();
        assert_eq!(after_modulator.op_fb1, after_carrier.op_fb1);
        assert_eq!(after_modulator.op_fb2, after_carrier.op_fb2);
    }

    #[test]
    fn envelope_output_uses_core_state_and_testmode_max_volume() {
        let mut service = Ym2413OperatorSerialService::new();
        service.clock_envelope_output(&context(0, 0), false).unwrap();
        assert_eq!(service.eg_out(), 0x7f);
        service.clock_envelope_output(&context(0, 0x01), false).unwrap();
        assert_eq!(service.eg_out(), 0);
    }

    #[test]
    fn am_lfo_testmode_bit_three_increments_the_serial_counter() {
        let mut service = Ym2413OperatorSerialService::new();
        service.clock_am_lfo(0, 0x08, 0).unwrap();
        assert_eq!(service.lfo_am_counter(), 0x0100);
    }

    #[test]
    fn am_lfo_testmode_bit_one_resets_direction_and_high_counter_bit() {
        let mut service = Ym2413OperatorSerialService::new();
        let mut state = service.export_state();
        state.lfo_am_counter = 0x01ab;
        state.lfo_am_dir = true;
        service = Ym2413OperatorSerialService::restore_state(state).unwrap();
        service.clock_am_lfo(0, 0x02, 0).unwrap();
        assert_eq!(service.lfo_am_counter(), 0x00d5);
        assert!(!service.state().lfo_am_dir);
    }
}
