//! Diagnostic replay of the production YM2413 EnvelopeGenerator against
//! per-operator samples extracted from a pinned Nuked-OPLL trace.
//!
//! CSV carries the active operator's AR/DR/RR, SL, KSR, FNUM, block,
//! SUS and EG-TYPE values so the same example can replay a rate matrix.
//! This is a diagnostic candidate harness, not a production integration.

use sega_ym2413::{
    AttackRate15Transition, DampTransitionConfig, DampTransitionCoordination, EnvelopeGenerator,
    EnvelopeParameters, EnvelopeStage,
};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
struct Sample {
    call_index: u64,
    slot: u8,
    channel: u8,
    is_carrier: bool,
    global_counter: u32,
    key_on: bool,
    observed_level: u8,
    observed_state: u8,
    parameters: EnvelopeParameters,
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
    if fields.len() != 17 {
        return Err(format!("line {line_number}: expected 17 CSV columns").into());
    }
    let slot: u8 = fields[1].parse()?;
    if slot >= 18 {
        return Err(format!("line {line_number}: invalid EG slot {slot}").into());
    }
    Ok(Sample {
        call_index: fields[0].parse()?,
        slot,
        channel: fields[2].parse()?,
        is_carrier: parse_bool(fields[3], line_number)?,
        global_counter: fields[4].parse()?,
        key_on: parse_bool(fields[5], line_number)?,
        observed_level: fields[6].parse()?,
        observed_state: fields[7].parse()?,
        parameters: EnvelopeParameters {
            attack_rate: fields[8].parse()?,
            decay_rate: fields[9].parse()?,
            release_rate: fields[10].parse()?,
            sustain_level: fields[11].parse()?,
            key_scale_rate: parse_bool(fields[12], line_number)?,
            fnum: fields[13].parse()?,
            block: fields[14].parse()?,
            sustain_on: parse_bool(fields[15], line_number)?,
            sustained_tone: parse_bool(fields[16], line_number)?,
        },
    })
}

fn make_generator(initial_level: u8) -> Result<EnvelopeGenerator, Box<dyn Error>> {
    let config = DampTransitionConfig::new(
        124,
        DampTransitionCoordination::IndependentPerOperator,
        AttackRate15Transition::SkipAttackToDecay,
    )
    .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    EnvelopeGenerator::with_initial_state(initial_level, EnvelopeStage::Damp, false, config)
        .map_err(|error| std::io::Error::other(format!("{error:?}")))
        .map_err(Into::into)
}

fn make_generators(initial_level: u8) -> Result<[EnvelopeGenerator; 18], Box<dyn Error>> {
    let mut generators = Vec::with_capacity(18);
    for _ in 0..18 {
        generators.push(make_generator(initial_level)?);
    }
    generators
        .try_into()
        .map_err(|_| std::io::Error::other("failed to construct 18 EG generators").into())
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
    let mut initial_zero = make_generators(0)?;
    let mut initial_silent = make_generators(127)?;
    let mut initial_silent_wrapped = make_generators(127)?;
    let mut initial_silent_wrapped_deferred = make_generators(127)?;
    let mut initial_silent_deferred_unwrapped = make_generators(127)?;

    let mut sample_count = 0_u64;
    for (line_index, line) in input.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let sample = parse_sample(trimmed, line_index + 1)?;
        let generator_index = sample.slot as usize;

        let zero_result = initial_zero[generator_index]
            .tick_with_events(
                sample.global_counter,
                sample.key_on,
                sample.is_carrier,
                sample.parameters,
            )
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
        let silent_result = initial_silent[generator_index]
            .tick_with_events(
                sample.global_counter,
                sample.key_on,
                sample.is_carrier,
                sample.parameters,
            )
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
        let wrapped_result = initial_silent_wrapped[generator_index]
            .tick_with_events(
                sample.global_counter & 0xff,
                sample.key_on,
                sample.is_carrier,
                sample.parameters,
            )
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;

        // Candidate: preserve DAMP -> ATTACK but defer the first ATTACK increment.
        let wrapped_before = initial_silent_wrapped_deferred[generator_index].export_state();
        let wrapped_deferred_result = initial_silent_wrapped_deferred[generator_index]
            .tick_with_events(
                sample.global_counter & 0xff,
                sample.key_on,
                sample.is_carrier,
                sample.parameters,
            )
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
        if wrapped_deferred_result.phase_reset_required {
            let mut state = initial_silent_wrapped_deferred[generator_index].export_state();
            state.level = wrapped_before.level;
            initial_silent_wrapped_deferred[generator_index] =
                EnvelopeGenerator::restore_state(state)
                    .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
        }

        let unwrapped_before =
            initial_silent_deferred_unwrapped[generator_index].export_state();
        let unwrapped_deferred_result =
            initial_silent_deferred_unwrapped[generator_index].tick_with_events(
                sample.global_counter,
                sample.key_on,
                sample.is_carrier,
                sample.parameters,
            )
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
        if unwrapped_deferred_result.phase_reset_required {
            let mut state = initial_silent_deferred_unwrapped[generator_index].export_state();
            state.level = unwrapped_before.level;
            initial_silent_deferred_unwrapped[generator_index] =
                EnvelopeGenerator::restore_state(state)
                    .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
        }

        println!(
            concat!(
                "{{\"call_index\":{},\"slot\":{},\"channel\":{},\"is_carrier\":{},",
                "\"global_counter\":{},\"key_on\":{},\"observed_level\":{},\"observed_state\":{},",
                "\"candidate_level_initial_0\":{},\"candidate_stage_initial_0\":{},",
                "\"candidate_level_initial_127\":{},\"candidate_stage_initial_127\":{},",
                "\"candidate_level_initial_127_wrapped\":{},\"candidate_stage_initial_127_wrapped\":{},",
                "\"candidate_level_initial_127_wrapped_deferred\":{},",
                "\"candidate_stage_initial_127_wrapped_deferred\":{},",
                "\"candidate_level_initial_127_deferred_unwrapped\":{},",
                "\"candidate_stage_initial_127_deferred_unwrapped\":{}}}"
            ),
            sample.call_index,
            sample.slot,
            sample.channel,
            u8::from(sample.is_carrier),
            sample.global_counter,
            u8::from(sample.key_on),
            sample.observed_level,
            sample.observed_state,
            zero_result.level,
            stage_code(initial_zero[generator_index].stage()),
            silent_result.level,
            stage_code(initial_silent[generator_index].stage()),
            wrapped_result.level,
            stage_code(initial_silent_wrapped[generator_index].stage()),
            initial_silent_wrapped_deferred[generator_index].level(),
            stage_code(initial_silent_wrapped_deferred[generator_index].stage()),
            initial_silent_deferred_unwrapped[generator_index].level(),
            stage_code(initial_silent_deferred_unwrapped[generator_index].stage()),
        );
        sample_count += 1;
    }

    if sample_count == 0 {
        return Err("input contains no active EG samples".into());
    }
    Ok(())
}
