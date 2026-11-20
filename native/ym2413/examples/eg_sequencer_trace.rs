//! Full 18-slot YM2413 EG sequencer replay against a pinned Nuked trace.
//!
//! The input row supplies the selected current-cycle channel/operator latches.
//! The sequencer owns the 18 EG states, shared timer, and delayed control
//! pipelines, then compares its complete post-cycle state with the oracle.

use sega_ym2413::{EnvelopeSequencer, EnvelopeSlotInput};
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
    if fields.len() != 32 {
        return Err(format!("line {line_number}: expected 32 CSV columns").into());
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
    })
}

fn state_code(stage: sega_ym2413::EnvelopeOperatorStage) -> u8 {
    stage as u8
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).ok_or(
        "usage: cargo run --manifest-path native/ym2413/Cargo.toml --example eg_sequencer_trace -- input.csv",
    )?;
    let input = BufReader::new(File::open(Path::new(&path))?);
    let mut sequencer = EnvelopeSequencer::new();
    let mut calls = 0_u64;
    let mut mismatches = 0_u64;
    let mut first_mismatch = None;

    for (line_index, line) in input.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("call_index,") {
            continue;
        }
        let sample = parse_sample(trimmed, line_index + 1)?;
        let result = sequencer
            .tick_slot(sample.cycle, sample.input)
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;

        let mut bad = Vec::new();
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

    println!(
        "{{\"calls\":{calls},\"field_mismatches\":{mismatches},\"first_mismatch\":{}}}",
        first_mismatch
            .as_ref()
            .map(|value| format!("\"{}\"", value.replace('"', "\\\"")))
            .unwrap_or_else(|| "null".to_owned()),
    );

    if mismatches != 0 {
        return Err("full YM2413 EG sequencer differential mismatch".into());
    }
    Ok(())
}
