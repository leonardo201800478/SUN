//! Differential replay of the production YM2413 core against a pinned Nuked trace.
//!
//! Each OPLL_Clock oracle call is mapped to one completed four-input-clock slot.
//! The core owns the delayed register-write pipeline, rhythm selector, EG timer,
//! and all 18 operator envelope states; no independent test-only bus is used.

use sega_ym2413::{Ym2413Core, YM2413_INPUT_CLOCKS_PER_SLOT};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
struct Sample {
    call_index: u64,
    cycle: u8,
    operator_slot: u8,
    level: u8,
    stage: u8,
    timer: u32,
    carry: u8,
    counter_state: u8,
    counter_state_prev: u8,
    shift: u8,
    shift_stop: bool,
    low_lock: u8,
    shift_lock: u8,
    rate_hi: u8,
    inc_lo: bool,
    inc_hi: bool,
    max_rate: bool,
    zero_rate: bool,
    rate: u8,
    eg_kon: u8,
    eg_off: u8,
    eg_dokon: u32,
    eg_sl: u8,
    pending_sustain_level: u8,
    rm_select: u8,
    rm_enable: u8,
    rhythm_bits: u8,
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
        operator_slot: fields[11].parse()?,
        level: fields[12].parse()?,
        stage: fields[13].parse()?,
        timer: fields[14].parse()?,
        carry: fields[15].parse()?,
        counter_state: fields[16].parse()?,
        counter_state_prev: fields[17].parse()?,
        shift: fields[18].parse()?,
        shift_stop: parse_bool(fields[19], line_number)?,
        low_lock: fields[20].parse()?,
        shift_lock: fields[21].parse()?,
        rate_hi: fields[22].parse()?,
        inc_lo: parse_bool(fields[23], line_number)?,
        inc_hi: parse_bool(fields[24], line_number)?,
        max_rate: parse_bool(fields[25], line_number)?,
        zero_rate: parse_bool(fields[26], line_number)?,
        rate: fields[27].parse()?,
        eg_kon: fields[28].parse()?,
        eg_off: fields[29].parse()?,
        eg_dokon: fields[30].parse()?,
        eg_sl: fields[31].parse()?,
        pending_sustain_level: fields[32].parse()?,
        rm_select: fields[33].parse()?,
        rm_enable: fields[34].parse()?,
        rhythm_bits: fields[35].parse()?,
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
    let mut previous_call = None;
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
        if event.port > 1 || previous_call.is_some_and(|previous| event.call_index <= previous) {
            return Err(format!(
                "event line {}: unordered or invalid OPLL port write",
                line_index + 1
            )
            .into());
        }
        previous_call = Some(event.call_index);
        events.push(event);
    }
    Ok(events)
}

fn compare_sample(core: &Ym2413Core, sample: Sample) -> Vec<&'static str> {
    let state = core.export_state();
    let eg = state.envelope;
    let timer = eg.timer;
    let operator = eg.operators[usize::from(sample.operator_slot)];
    let mut bad = Vec::new();

    if operator.level != sample.level {
        bad.push("operator_level");
    }
    if operator.stage as u8 != sample.stage {
        bad.push("operator_stage");
    }
    if timer.timer != sample.timer {
        bad.push("timer");
    }
    if timer.carry != sample.carry {
        bad.push("timer_carry");
    }
    if timer.counter_state != sample.counter_state {
        bad.push("counter_state");
    }
    if timer.counter_state_prev != sample.counter_state_prev {
        bad.push("counter_state_prev");
    }
    if timer.shift != sample.shift {
        bad.push("timer_shift");
    }
    if timer.shift_stop != sample.shift_stop {
        bad.push("timer_shift_stop");
    }
    if timer.low_lock != sample.low_lock {
        bad.push("timer_low_lock");
    }
    if timer.shift_lock != sample.shift_lock {
        bad.push("timer_shift_lock");
    }
    if timer.latched_rate_hi != sample.rate_hi {
        bad.push("rate_hi");
    }
    if timer.latched_inc_lo != sample.inc_lo {
        bad.push("inc_lo");
    }
    if timer.latched_inc_hi != sample.inc_hi {
        bad.push("inc_hi");
    }
    if timer.latched_max_rate != sample.max_rate {
        bad.push("max_rate");
    }
    if timer.latched_zero_rate != sample.zero_rate {
        bad.push("zero_rate");
    }
    if timer.pending_rate != sample.rate {
        bad.push("rate");
    }
    if eg.eg_kon != sample.eg_kon {
        bad.push("eg_kon");
    }
    if eg.eg_off != sample.eg_off {
        bad.push("eg_off");
    }
    if eg.eg_dokon != sample.eg_dokon {
        bad.push("eg_dokon");
    }
    if eg.eg_sl != sample.eg_sl {
        bad.push("eg_sl");
    }
    if eg.pending_sustain_level != sample.pending_sustain_level {
        bad.push("pending_sustain_level");
    }
    if state.rm_select != sample.rm_select {
        bad.push("rm_select");
    }
    if state.rm_enable as u8 != sample.rm_enable {
        bad.push("rm_enable");
    }
    if state.registers[0x0e] & 0x3f != sample.rhythm_bits {
        bad.push("rhythm_bits");
    }
    if core.slot_index() != (sample.cycle + 1) % 18 {
        bad.push("slot_index");
    }

    bad
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let samples_path = args.next().ok_or(
        "usage: cargo run --manifest-path native/ym2413/Cargo.toml --example eg_core_trace -- samples.csv events.csv",
    )?;
    let events_path = args.next().ok_or("missing OPLL port event fixture")?;
    let events = read_events(Path::new(&events_path))?;
    let mut event_index = 0_usize;
    let mut core = Ym2413Core::new();
    let mut calls = 0_u64;
    let mut mismatches = 0_u64;
    let mut first_mismatch = None;

    let input = BufReader::new(File::open(Path::new(&samples_path))?);
    for (line_index, line) in input.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("call_index,") {
            continue;
        }
        let sample = parse_sample(trimmed, line_index + 1)?;
        if core.slot_index() != sample.cycle {
            return Err(format!(
                "call {}: core cycle {} differs from oracle cycle {}",
                sample.call_index,
                core.slot_index(),
                sample.cycle
            )
            .into());
        }

        while event_index < events.len() && events[event_index].call_index == sample.call_index {
            let event = events[event_index];
            if event.port == 0 {
                core.write_address(event.value);
            } else {
                core.write_data(event.value);
            }
            event_index += 1;
        }
        if event_index < events.len() && events[event_index].call_index < sample.call_index {
            return Err("register event call index was skipped".into());
        }

        core.advance_input_clocks(YM2413_INPUT_CLOCKS_PER_SLOT)?;
        let bad = compare_sample(&core, sample);
        if !bad.is_empty() {
            mismatches += bad.len() as u64;
            if first_mismatch.is_none() {
                first_mismatch = Some(format!(
                    "call={} cycle={} operator_slot={} fields={}",
                    sample.call_index,
                    sample.cycle,
                    sample.operator_slot,
                    bad.join("|")
                ));
            }
        }

        calls += 1;
        if calls.is_multiple_of(127) {
            let state = core.export_state();
            let restored = Ym2413Core::restore_state(state.clone())?;
            if restored.export_state() != state {
                return Err(format!(
                    "core save-state round trip changed at call {}",
                    sample.call_index
                )
                .into());
            }
            core = restored;
        }
    }

    if event_index != events.len() {
        return Err("one or more port events occur after the requested trace window".into());
    }

    println!(
        "{{\"calls\":{calls},\"field_mismatches\":{mismatches},\"first_mismatch\":{}}}",
        first_mismatch
            .as_ref()
            .map(|value| format!("\"{}\"", value.replace('"', "\\\"")))
            .unwrap_or_else(|| "null".to_owned())
    );

    if mismatches != 0 {
        return Err("integrated YM2413 core EG differential mismatch".into());
    }
    Ok(())
}
