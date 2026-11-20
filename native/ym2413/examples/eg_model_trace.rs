//! Diagnostic replay of the production YM2413 EnvelopeGenerator against
//! selected per-operator samples extracted from a pinned Nuked-OPLL trace.
//!
//! CSV columns:
//! call_index,slot,global_counter,key_on,observed_level,observed_state
//!
//! The two initial levels are deliberately separate candidates. This example
//! does not choose a reset-state interpretation or alter production behavior.

use sega_ym2413::{
    AttackRate15Transition, DampTransitionConfig, DampTransitionCoordination,
    EnvelopeGenerator, EnvelopeParameters, EnvelopeStage,
};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
struct Sample {
    call_index: u64,
    slot: u8,
    global_counter: u32,
    key_on: bool,
    observed_level: u8,
    observed_state: u8,
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
    if fields.len() != 6 {
        return Err(format!("line {line_number}: expected 6 CSV columns").into());
    }
    let slot: u8 = fields[1].parse()?;
    if slot != 2 && slot != 17 {
        return Err(format!("line {line_number}: unsupported EG slot {slot}").into());
    }
    Ok(Sample {
        call_index: fields[0].parse()?,
        slot,
        global_counter: fields[2].parse()?,
        key_on: parse_bool(fields[3], line_number)?,
        observed_level: fields[4].parse()?,
        observed_state: fields[5].parse()?,
    })
}

fn make_generator(initial_level: u8) -> Result<EnvelopeGenerator, Box<dyn Error>> {
    let config = DampTransitionConfig::new(
        124,
        DampTransitionCoordination::IndependentPerOperator,
        AttackRate15Transition::SkipAttackToDecay,
    )?;
    Ok(EnvelopeGenerator::with_initial_state(
        initial_level,
        EnvelopeStage::Damp,
        false,
        config,
    )?)
}

fn stage_code(stage: EnvelopeStage) -> u8 {
    match stage {
        EnvelopeStage::Damp => 0,
        EnvelopeStage::Attack => 1,
        EnvelopeStage::Decay => 2,
        EnvelopeStage::Sustain => 3,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).ok_or(
        "usage: cargo run --manifest-path native/ym2413/Cargo.toml --example eg_model_trace -- input.csv",
    )?;
    let input = BufReader::new(File::open(Path::new(&path))?);
    let mut initial_zero: [EnvelopeGenerator; 2] =
        [make_generator(0)?, make_generator(0)?];
    let mut initial_silent: [EnvelopeGenerator; 2] =
        [make_generator(127)?, make_generator(127)?];

    let parameters = EnvelopeParameters {
        attack_rate: 10,
        decay_rate: 5,
        release_rate: 7,
        sustain_level: 4,
        key_scale_rate: false,
        fnum: 128,
        block: 4,
        sustain_on: false,
        sustained_tone: true,
    };

    let mut sample_count = 0_u64;
    for (line_index, line) in input.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let sample = parse_sample(trimmed, line_index + 1)?;
        let generator_index = if sample.slot == 2 { 0 } else { 1 };
        let is_carrier = sample.slot == 2;

        let zero_result = initial_zero[generator_index].tick_with_events(
            sample.global_counter,
            sample.key_on,
            is_carrier,
            parameters,
        )?;
        let silent_result = initial_silent[generator_index].tick_with_events(
            sample.global_counter,
            sample.key_on,
            is_carrier,
            parameters,
        )?;

        println!(
            concat!(
                "{{\"call_index\":{},\"slot\":{},\"global_counter\":{},",
                "\"key_on\":{},\"observed_level\":{},\"observed_state\":{},",
                "\"candidate_level_initial_0\":{},\"candidate_stage_initial_0\":{},",
                "\"candidate_reset_initial_0\":{},",
                "\"candidate_level_initial_127\":{},\"candidate_stage_initial_127\":{},",
                "\"candidate_reset_initial_127\":{}}}"
            ),
            sample.call_index,
            sample.slot,
            sample.global_counter,
            u8::from(sample.key_on),
            sample.observed_level,
            sample.observed_state,
            zero_result.level,
            stage_code(initial_zero[generator_index].stage()),
            u8::from(zero_result.phase_reset_required),
            silent_result.level,
            stage_code(initial_silent[generator_index].stage()),
            u8::from(silent_result.phase_reset_required),
        );
        sample_count += 1;
    }

    if sample_count == 0 {
        return Err("input contains no active channel-0 EG samples".into());
    }
    Ok(())
}
