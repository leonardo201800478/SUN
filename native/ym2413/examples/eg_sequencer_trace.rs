//! Full 18-slot YM2413 EG sequencer replay against a pinned Nuked trace.
//!
//! The input CSV contains oracle post-cycle state. When an event fixture is
//! supplied, OPLL address/data writes are replayed into the 64-byte register
//! file and the channel decoder constructs the current-cycle EG input. The
//! sequencer owns all 18 EG states, timer, and delayed control pipelines.

use sega_ym2413::{
    ChannelSynthesisConfig, EnvelopeSequencer, EnvelopeSlotInput, decode_channel_synthesis,
    envelope_slot_input_for_cycle,
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
    if fields.len() != 33 {
        return Err(format!("line {line_number}: expected 33 CSV columns").into());
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
    })
}

#[derive(Debug, Clone, Copy)]
struct Event {
    call_index: u64,
    port: u8,
    value: u8,
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
            return Err(format!("event line {}: expected three CSV columns", line_index + 1).into());
        }
        let event = Event {
            call_index: fields[0].parse()?,
            port: fields[1].parse()?,
            value: fields[2].parse()?,
        };
        if event.call_index < previous_call || event.port > 1 {
            return Err(format!("event line {}: unordered or invalid OPLL port write", line_index + 1).into());
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
    let first = decode_channel_synthesis(registers, 0)
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    let mut channels: [ChannelSynthesisConfig; 9] = [first; 9];
    for (index, channel) in channels.iter_mut().enumerate() {
        *channel = decode_channel_synthesis(registers, index as u8)
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    }
    envelope_slot_input_for_cycle(&channels, cycle)
        .map_err(|error| std::io::Error::other(format!("{error:?}")).into())
}

fn input_mismatches(actual: EnvelopeSlotInput, expected: EnvelopeSlotInput) -> Vec<&'static str> {
    let mut bad = Vec::new();
    if actual.attack_rate != expected.attack_rate { bad.push("input_attack_rate"); }
    if actual.decay_rate != expected.decay_rate { bad.push("input_decay_rate"); }
    if actual.release_rate != expected.release_rate { bad.push("input_release_rate"); }
    if actual.sustain_level != expected.sustain_level { bad.push("input_sustain_level"); }
    if actual.key_scale_rate != expected.key_scale_rate { bad.push("input_key_scale_rate"); }
    if actual.ksr_frequency != expected.ksr_frequency { bad.push("input_ksr_frequency"); }
    if actual.sustain_on != expected.sustain_on { bad.push("input_sustain_on"); }
    if actual.sustained_tone != expected.sustained_tone { bad.push("input_sustained_tone"); }
    if actual.key_on != expected.key_on { bad.push("input_key_on"); }
    bad
}

fn state_code(stage: sega_ym2413::EnvelopeOperatorStage) -> u8 {
    stage as u8
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let samples_path = args.next().ok_or(
        "usage: cargo run --manifest-path native/ym2413/Cargo.toml --example eg_sequencer_trace -- samples.csv [events.csv]",
    )?;
    let events_path = args.next();
    let events = events_path
        .as_deref()
        .map(|path| read_events(Path::new(path)))
        .transpose()?;
    let register_mode = events.is_some();
    let events = events.unwrap_or_default();
    let mut event_index = 0_usize;
    let mut address_latch = 0_u8;
    let mut registers = [0_u8; 64];

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
        while event_index < events.len() && events[event_index].call_index < sample.call_index {
            let event = events[event_index];
            if event.port == 0 {
                address_latch = event.value & 0x3f;
            } else {
                registers[usize::from(address_latch)] = event.value;
            }
            event_index += 1;
        }

        let decoded_input = if register_mode {
            decode_register_input(&registers, sample.cycle)?
        } else {
            sample.input
        };
        let input_bad = if register_mode {
            input_mismatches(decoded_input, sample.input)
        } else {
            Vec::new()
        };
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
        "{{\"calls\":{calls},\"register_mode\":{register_mode},\"register_input_mismatches\":{register_input_mismatches},\"input_mismatch_counts\":{{{input_counts_json}}},\"field_mismatches\":{mismatches},\"first_mismatch\":{}}}",
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
