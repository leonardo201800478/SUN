//! Full 18-slot YM2413 EG sequencer replay against a pinned Nuked trace.
//!
//! The input CSV contains oracle post-cycle state. When an event fixture is
//! supplied, OPLL address/data writes are replayed into the 64-byte register
//! file and the channel decoder constructs the current-cycle EG input. The
//! sequencer owns all 18 EG states, timer, and delayed control pipelines.

use sega_ym2413::{
    EnvelopeRhythmSelect, EnvelopeSequencer, EnvelopeSlotInput,
    decode_envelope_slot_input_for_cycle,
};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
struct Sample {
    call_index: u64,
    cycle: u8,
    input: EnvelopeSlotInput,
    expected_operator_slot: u8,
    expected_level: u8,
    expected_state: u8,
    expected_timer: u32,
    expected_carry: u8,
    expected_counter_state: u8,
    expected_counter_state_prev: u8,
    expected_shift: u8,
    expected_shift_stop: bool,
    expected_low_lock: u8,
    expected_shift_lock: u8,
    expected_rate_hi: u8,
    expected_inc_lo: bool,
    expected_inc_hi: bool,
    expected_max_rate: bool,
    expected_zero_rate: bool,
    expected_rate: u8,
    expected_eg_kon: u8,
    expected_eg_off: u8,
    expected_eg_dokon: u32,
    expected_eg_sl: u8,
    expected_pending_sustain_level: u8,
    expected_rm_select: u8,
    expected_rm_enable: u8,
    expected_rhythm_bits: u8,
}

fn parse_bool(value: &str, line_number: usize) -> Result<bool, Box<dyn Error>> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(format!("line {line_number}: invalid boolean {other}").into()),
    }
}

fn parse_sample(line: &str, line_number: usize) -> Result<Sample, Box<dyn Error>> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() != 36 {
        return Err(format!("line {line_number}: expected 36 CSV columns").into());
    }
    Ok(Sample {
        call_index: fields[0].parse()?,
        cycle: fields[1].parse()?,
        input: EnvelopeSlotInput {
            attack_rate: fields[2].parse()?,
            decay_rate: fields[3].parse()?,
            release_rate: fields[4].parse()?,
            sustain_level: fields[5].parse()?,
            key_scale_rate: parse_bool(fields[6], line_number)?,
            ksr_frequency: fields[7].parse()?,
            sustain_on: parse_bool(fields[8], line_number)?,
            sustained_tone: parse_bool(fields[9], line_number)?,
            key_on: parse_bool(fields[10], line_number)?,
            rhythm_select: EnvelopeRhythmSelect::from_code(fields[33].parse()?).ok_or_else(
                || std::io::Error::other(format!("line {line_number}: invalid rhythm selector")),
            )?,
            rhythm_bits: fields[35].parse()?,
        },
        expected_operator_slot: fields[11].parse()?,
        expected_level: fields[12].parse()?,
        expected_state: fields[13].parse()?,
        expected_timer: fields[14].parse()?,
        expected_carry: fields[15].parse()?,
        expected_counter_state: fields[16].parse()?,
        expected_counter_state_prev: fields[17].parse()?,
        expected_shift: fields[18].parse()?,
        expected_shift_stop: parse_bool(fields[19], line_number)?,
        expected_low_lock: fields[20].parse()?,
        expected_shift_lock: fields[21].parse()?,
        expected_rate_hi: fields[22].parse()?,
        expected_inc_lo: parse_bool(fields[23], line_number)?,
        expected_inc_hi: parse_bool(fields[24], line_number)?,
        expected_max_rate: parse_bool(fields[25], line_number)?,
        expected_zero_rate: parse_bool(fields[26], line_number)?,
        expected_rate: fields[27].parse()?,
        expected_eg_kon: fields[28].parse()?,
        expected_eg_off: fields[29].parse()?,
        expected_eg_dokon: fields[30].parse()?,
        expected_eg_sl: fields[31].parse()?,
        expected_pending_sustain_level: fields[32].parse()?,
        expected_rm_select: fields[33].parse()?,
        expected_rm_enable: fields[34].parse()?,
        expected_rhythm_bits: fields[35].parse()?,
    })
}

#[derive(Debug, Clone, Copy)]
struct Event {
    call_index: u64,
    port: u8,
    value: u8,
}

#[derive(Debug, Clone)]
struct RegisterBus {
    registers: [u8; 64],
    write_data: u8,
    write_a: u8,
    write_d: u8,
    write_a_en: bool,
    write_d_en: bool,
    write_fm_address: bool,
    write_fm_data: bool,
    address: u8,
    data: u8,
    write_mode_address: u8,
    rm_enable: i8,
    rm_select: u8,
    rhythm: u8,
}

impl RegisterBus {
    fn new() -> Self {
        Self {
            registers: [0; 64],
            write_data: 0,
            write_a: 0,
            write_d: 0,
            write_a_en: false,
            write_d_en: false,
            write_fm_address: false,
            write_fm_data: false,
            address: 0,
            data: 0,
            write_mode_address: 0,
            rm_enable: 0,
            rm_select: 6,
            rhythm: 0,
        }
    }

    /// Queue an address/data port write exactly as OPLL_Write does.
    fn write(&mut self, port: u8, value: u8) {
        self.write_data = value;
        if port & 1 != 0 {
            self.write_d |= 1;
        } else {
            self.write_a |= 1;
        }
    }

    /// Apply the delayed user-patch/mode write at the start of OPLL_Clock.
    fn begin_clock(&mut self, cycle: u8) {
        self.rm_enable >>= 1;
        if self.write_mode_address & 0x10 != 0 && self.write_d_en {
            let register = self.write_mode_address & 0x0f;
            match register {
                0x00..=0x07 => self.registers[usize::from(register)] = self.write_data,
                0x0e => {
                    self.rhythm = self.write_data & 0x3f;
                    self.registers[0x0e] = self.rhythm;
                    self.rm_enable = (self.rm_enable & 0x7f) | (((self.rhythm << 2) & 0x80) as i8);
                }
                0x0f => self.registers[0x0f] = self.write_data & 0x0f,
                _ => {}
            }
        }
        self.rm_select = (self.rm_select + 1).min(6);
        if cycle == 11 && (self.rm_enable as u8 & 0x80) != 0 {
            self.rm_select = 0;
        }
    }

    /// Apply cycle-qualified channel writes, then advance OPLL_DoIO latches.
    fn end_clock(&mut self, cycle: u8) {
        if self.write_a_en {
            if self.write_data & 0xc0 == 0 {
                self.write_fm_address = true;
                self.address = self.write_data;
            } else {
                self.write_fm_address = false;
            }
        }
        if self.write_fm_address && self.write_d_en {
            self.data = self.write_data;
        }
        if self.write_fm_data && !self.write_a_en && cycle < 16 && (self.address & 0x0f) == cycle {
            let channel = usize::from(cycle % 9);
            match self.address & 0xf0 {
                0x10 | 0x20 | 0x30 => {
                    self.registers[usize::from(self.address & 0xf0) + channel] = self.data;
                }
                _ => {}
            }
        }
        if self.write_a_en {
            self.write_fm_data = false;
        }
        if self.write_fm_address && self.write_d_en {
            self.write_fm_data = true;
        }
        if self.write_a_en {
            self.write_mode_address = if self.write_data & 0xf0 == 0 {
                0x10 | (self.write_data & 0x0f)
            } else {
                0
            };
        }

        self.write_a_en = self.write_a & 0x03 == 0x01;
        self.write_d_en = self.write_d & 0x03 == 0x01;
        self.write_a <<= 1;
        self.write_d <<= 1;
    }
}

fn read_events(path: &Path) -> Result<Vec<Event>, Box<dyn Error>> {
    let input = BufReader::new(File::open(path)?);
    let mut events = Vec::new();
    let mut previous_call = 0_u64;
    for (line_index, line) in input.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = trimmed.split(',').map(str::trim).collect();
        if fields.len() != 3 {
            return Err(
                format!("event line {}: expected three CSV columns", line_index + 1).into(),
            );
        }
        let event = Event {
            call_index: fields[0].parse()?,
            port: fields[1].parse()?,
            value: fields[2].parse()?,
        };
        if event.call_index < previous_call || event.port > 1 {
            return Err(format!(
                "event line {}: unordered or invalid OPLL port write",
                line_index + 1
            )
            .into());
        }
        previous_call = event.call_index;
        events.push(event);
    }
    Ok(events)
}

fn decode_register_input(
    registers: &[u8; 64],
    cycle: u8,
) -> Result<EnvelopeSlotInput, Box<dyn Error>> {
    decode_envelope_slot_input_for_cycle(registers, cycle)
        .map_err(|error| std::io::Error::other(format!("{error:?}")).into())
}

fn input_mismatches(actual: EnvelopeSlotInput, expected: EnvelopeSlotInput) -> Vec<&'static str> {
    let mut bad = Vec::new();
    if actual.attack_rate != expected.attack_rate {
        bad.push("input_attack_rate");
    }
    if actual.decay_rate != expected.decay_rate {
        bad.push("input_decay_rate");
    }
    if actual.release_rate != expected.release_rate {
        bad.push("input_release_rate");
    }
    if actual.sustain_level != expected.sustain_level {
        bad.push("input_sustain_level");
    }
    if actual.key_scale_rate != expected.key_scale_rate {
        bad.push("input_key_scale_rate");
    }
    if actual.ksr_frequency != expected.ksr_frequency {
        bad.push("input_ksr_frequency");
    }
    if actual.sustain_on != expected.sustain_on {
        bad.push("input_sustain_on");
    }
    if actual.sustained_tone != expected.sustained_tone {
        bad.push("input_sustained_tone");
    }
    if actual.key_on != expected.key_on {
        bad.push("input_key_on");
    }
    if actual.rhythm_select != expected.rhythm_select {
        bad.push("input_rhythm_select");
    }
    if actual.rhythm_bits != expected.rhythm_bits {
        bad.push("input_rhythm_bits");
    }
    bad
}

fn state_code(stage: sega_ym2413::EnvelopeOperatorStage) -> u8 {
    stage as u8
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let samples_path = args.next().ok_or(
        "usage: cargo run --manifest-path native/ym2413/Cargo.toml --example eg_sequencer_trace -- samples.csv [events.csv] [--oracle-inputs]",
    )?;
    let events_path = args.next();
    let events = events_path
        .as_deref()
        .map(|path| read_events(Path::new(path)))
        .transpose()?;
    let register_mode = events.is_some();
    let oracle_inputs = args.next().as_deref() == Some("--oracle-inputs");
    if args.next().is_some() || (oracle_inputs && !register_mode) {
        return Err("invalid EG trace arguments".into());
    }
    let register_input_mode = register_mode && !oracle_inputs;
    let events = events.unwrap_or_default();
    let mut event_index = 0_usize;
    let mut register_bus = RegisterBus::new();

    let input = BufReader::new(File::open(Path::new(&samples_path))?);
    let mut sequencer = EnvelopeSequencer::new();
    let mut calls = 0_u64;
    let mut mismatches = 0_u64;
    let mut register_input_mismatches = 0_u64;
    let mut input_mismatch_counts = BTreeMap::<&'static str, u64>::new();
    let mut first_mismatch = None;

    for (line_index, line) in input.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("call_index,") {
            continue;
        }
        let sample = parse_sample(trimmed, line_index + 1)?;

        // Fixture timestamps are applied after the named OPLL_Clock call.
        // Therefore a write at call N first affects the register selection at N+1.
        while event_index < events.len() && events[event_index].call_index == sample.call_index {
            let event = events[event_index];
            register_bus.write(event.port, event.value);
            event_index += 1;
        }
        if event_index < events.len() && events[event_index].call_index < sample.call_index {
            return Err("register event call index was skipped".into());
        }
        if register_mode {
            register_bus.begin_clock(sample.cycle);
        }

        let mut decoded_input = if register_input_mode {
            decode_register_input(&register_bus.registers, sample.cycle)?
        } else {
            sample.input
        };
        if register_mode {
            decoded_input.rhythm_select =
                EnvelopeRhythmSelect::from_code(register_bus.rm_select)
                    .ok_or_else(|| std::io::Error::other("invalid local rhythm selector"))?;
            decoded_input.rhythm_bits = register_bus.rhythm;
        }
        let mut input_bad = if register_input_mode {
            input_mismatches(decoded_input, sample.input)
        } else {
            Vec::new()
        };
        if register_mode && register_bus.rm_select != sample.expected_rm_select {
            input_bad.push("rm_select");
        }
        if register_mode && register_bus.rm_enable as u8 != sample.expected_rm_enable {
            input_bad.push("rm_enable");
        }
        if register_mode && register_bus.rhythm != sample.expected_rhythm_bits {
            input_bad.push("rhythm_bits");
        }
        register_input_mismatches += input_bad.len() as u64;
        for field in &input_bad {
            *input_mismatch_counts.entry(*field).or_default() += 1;
        }

        let result = sequencer
            .tick_slot(sample.cycle, decoded_input)
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;

        let mut bad = input_bad.clone();
        if result.operator_slot != sample.expected_operator_slot {
            bad.push("operator_slot");
        }
        if result.operator.level != sample.expected_level {
            bad.push("operator_level");
        }
        if state_code(result.operator.stage) != sample.expected_state {
            bad.push("operator_stage");
        }
        if result.timer.timer != sample.expected_timer {
            bad.push("timer");
        }
        if result.timer.carry != sample.expected_carry {
            bad.push("timer_carry");
        }
        if result.timer.counter_state != sample.expected_counter_state {
            bad.push("counter_state");
        }
        if result.timer.counter_state_prev != sample.expected_counter_state_prev {
            bad.push("counter_state_prev");
        }
        if result.timer.shift != sample.expected_shift {
            bad.push("timer_shift");
        }
        if result.timer.shift_stop != sample.expected_shift_stop {
            bad.push("timer_shift_stop");
        }
        if result.timer.low_lock != sample.expected_low_lock {
            bad.push("timer_low_lock");
        }
        if result.timer.shift_lock != sample.expected_shift_lock {
            bad.push("timer_shift_lock");
        }
        if result.timer.rate_phase.rate_hi != sample.expected_rate_hi {
            bad.push("rate_hi");
        }
        if result.timer.rate_phase.inc_lo != sample.expected_inc_lo {
            bad.push("inc_lo");
        }
        if result.timer.rate_phase.inc_hi != sample.expected_inc_hi {
            bad.push("inc_hi");
        }
        if result.timer.rate_phase.max_rate != sample.expected_max_rate {
            bad.push("max_rate");
        }
        if result.timer.rate_phase.zero_rate != sample.expected_zero_rate {
            bad.push("zero_rate");
        }
        if result.next_rate != sample.expected_rate {
            bad.push("next_rate");
        }
        if result.eg_kon != sample.expected_eg_kon {
            bad.push("eg_kon");
        }
        if result.eg_off != sample.expected_eg_off {
            bad.push("eg_off");
        }
        if result.eg_dokon != sample.expected_eg_dokon {
            bad.push("eg_dokon");
        }
        if result.eg_sl != sample.expected_eg_sl {
            bad.push("eg_sl");
        }
        if result.pending_sustain_level != sample.expected_pending_sustain_level {
            bad.push("pending_sustain_level");
        }

        if register_mode {
            register_bus.end_clock(sample.cycle);
        }

        if !bad.is_empty() {
            mismatches += bad.len() as u64;
            if first_mismatch.is_none() {
                first_mismatch = Some(format!(
                    "call={} cycle={} operator_slot={} fields={}",
                    sample.call_index,
                    sample.cycle,
                    sample.expected_operator_slot,
                    bad.join("|"),
                ));
                if register_mode && !input_bad.is_empty() {
                    first_mismatch = Some(format!(
                        "call={} cycle={} operator_slot={} fields={} expected_input={:?} decoded_input={:?}",
                        sample.call_index,
                        sample.cycle,
                        sample.expected_operator_slot,
                        bad.join("|"),
                        sample.input,
                        decoded_input,
                    ));
                }
            }
        }

        calls += 1;
        if calls.is_multiple_of(127) {
            let state = sequencer.export_state();
            let restored = EnvelopeSequencer::restore_state(state)
                .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
            if restored.export_state() != state {
                return Err(format!(
                    "save-state round trip changed EG sequencer state at call {}",
                    sample.call_index
                )
                .into());
            }
            sequencer = restored;
        }
    }

    let input_counts_json = input_mismatch_counts
        .iter()
        .map(|(field, count)| format!("\"{field}\":{count}"))
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{{\"calls\":{calls},\"register_mode\":{register_mode},\"register_inputs_used\":{register_input_mode},\"register_input_mismatches\":{register_input_mismatches},\"input_mismatch_counts\":{{{input_counts_json}}},\"field_mismatches\":{mismatches},\"first_mismatch\":{}}}",
        first_mismatch
            .as_ref()
            .map(|value| format!("\"{}\"", value.replace('"', "\\\"")))
            .unwrap_or_else(|| "null".to_owned()),
    );

    if register_input_mismatches != 0 || mismatches != 0 {
        return Err("full YM2413 EG sequencer differential mismatch".into());
    }
    Ok(())
}
