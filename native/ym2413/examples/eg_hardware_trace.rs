//! Full serial EG operator replay against pre/post Nuked trace samples.
//!
//! The example consumes the control latches visible before each OPLL_Clock
//! update. It tests the four-state per-operator update independently of the
//! still-unimplemented chip-wide key/rate pipeline generator.

use sega_ym2413::envelope_timer::EnvelopeRatePhase;
use sega_ym2413::{EnvelopeControlPhase, EnvelopeOperator, EnvelopeOperatorStage};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
struct Sample {
    call_index: u64,
    slot: u8,
    level_before: u8,
    state_before: u8,
    key_on: bool,
    off: bool,
    dokon: bool,
    sustain_level: u8,
    rate_hi: u8,
    inc_lo: bool,
    inc_hi: bool,
    max_rate: bool,
    counter_state_prev: u8,
    expected_level: u8,
    expected_state: u8,
}

fn parse_bool(value: &str, line_number: usize) -> Result<bool, Box<dyn Error>> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(format!("line {line_number}: invalid boolean {other}").into()),
    }
}

fn stage_from_code(value: u8) -> Result<EnvelopeOperatorStage, Box<dyn Error>> {
    match value {
        0 => Ok(EnvelopeOperatorStage::Attack),
        1 => Ok(EnvelopeOperatorStage::Decay),
        2 => Ok(EnvelopeOperatorStage::Sustain),
        3 => Ok(EnvelopeOperatorStage::Release),
        other => Err(format!("unsupported Nuked EG state {other}").into()),
    }
}

fn parse_sample(line: &str, line_number: usize) -> Result<Sample, Box<dyn Error>> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() != 16 {
        return Err(format!("line {line_number}: expected 16 CSV columns").into());
    }
    let slot: u8 = fields[1].parse()?;
    if slot >= 18 {
        return Err(format!("line {line_number}: invalid EG slot {slot}").into());
    }
    Ok(Sample {
        call_index: fields[0].parse()?,
        slot,
        level_before: fields[3].parse()?,
        state_before: fields[4].parse()?,
        key_on: parse_bool(fields[5], line_number)?,
        off: parse_bool(fields[6], line_number)?,
        dokon: parse_bool(fields[7], line_number)?,
        sustain_level: fields[8].parse()?,
        rate_hi: fields[9].parse()?,
        inc_lo: parse_bool(fields[10], line_number)?,
        inc_hi: parse_bool(fields[11], line_number)?,
        max_rate: parse_bool(fields[12], line_number)?,
        counter_state_prev: fields[13].parse()?,
        expected_level: fields[14].parse()?,
        expected_state: fields[15].parse()?,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).ok_or(
        "usage: cargo run --manifest-path native/ym2413/Cargo.toml --example eg_hardware_trace -- input.csv",
    )?;
    let input = BufReader::new(File::open(Path::new(&path))?);
    let mut operators: [Option<EnvelopeOperator>; 18] = [None; 18];
    let mut samples = 0_u64;
    let mut key_on_samples = 0_u64;
    let mut key_off_samples = 0_u64;
    let mut level_mismatches = 0_u64;
    let mut stage_mismatches = 0_u64;
    let mut first_mismatch = None;

    for (line_index, line) in input.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("call_index,") {
            continue;
        }
        let sample = parse_sample(trimmed, line_index + 1)?;
        let slot = usize::from(sample.slot);
        let expected_pre_stage = stage_from_code(sample.state_before)?;
        let operator = operators[slot].get_or_insert(
            EnvelopeOperator::with_initial_state(sample.level_before, expected_pre_stage)
                .map_err(|error| std::io::Error::other(format!("{error:?}")))?,
        );

        if operator.level() != sample.level_before || operator.stage() != expected_pre_stage {
            return Err(format!(
                "operator continuity mismatch at call {} slot {}: Rust state=({}, {:?}), Nuked pre-state=({}, {:?})",
                sample.call_index,
                sample.slot,
                operator.level(),
                operator.stage(),
                sample.level_before,
                expected_pre_stage,
            )
            .into());
        }

        let rate = EnvelopeRatePhase::from_consumed_latches(
            sample.rate_hi,
            sample.inc_lo,
            sample.inc_hi,
            sample.max_rate,
            sample.counter_state_prev,
        );
        let result = operator
            .tick(EnvelopeControlPhase {
                rate,
                key_on: sample.key_on,
                off: sample.off,
                dokon: sample.dokon,
                sustain_level: sample.sustain_level,
            })
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;

        let expected_stage = stage_from_code(sample.expected_state)?;
        let level_bad = result.level != sample.expected_level;
        let stage_bad = result.stage != expected_stage;
        level_mismatches += u64::from(level_bad);
        stage_mismatches += u64::from(stage_bad);
        samples += 1;
        if sample.key_on {
            key_on_samples += 1;
        } else {
            key_off_samples += 1;
        }
        if (level_bad || stage_bad) && first_mismatch.is_none() {
            first_mismatch = Some(format!(
                "call={} slot={} expected=({}, {:?}) actual=({}, {:?}) step={}",
                sample.call_index,
                sample.slot,
                sample.expected_level,
                expected_stage,
                result.level,
                result.stage,
                result.step,
            ));
        }
    }

    println!(
        "{{\"samples\":{samples},\"key_on_samples\":{key_on_samples},\"key_off_samples\":{key_off_samples},\"level_mismatches\":{level_mismatches},\"stage_mismatches\":{stage_mismatches},\"first_mismatch\":{}}}",
        first_mismatch
            .as_ref()
            .map(|value| format!("\"{}\"", value.replace('"', "\\\"")))
            .unwrap_or_else(|| "null".to_owned()),
    );

    if level_mismatches != 0 || stage_mismatches != 0 {
        return Err("latch-driven four-state EG differential mismatch".into());
    }
    Ok(())
}
