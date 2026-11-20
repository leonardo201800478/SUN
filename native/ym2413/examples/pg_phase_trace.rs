//! Reconstruct the Nuked serial PG phase-array update using production Rust PG.
//!
//! Input CSV:
//! call_index,cycle_before,fnum,block,mul,pm_enabled,pm_position
//!
//! The Nuked adapter's phase array is indexed by the same 18-slot order.
//! OPLL_PhaseGenerate updates the previous slot using the increment calculated
//! by the previous OPLL_Clock call, so this example deliberately applies the
//! previous row's parameters to (current_cycle + 17) % 18.

use sega_ym2413::PhaseGenerator;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
struct Sample {
    call_index: u64,
    cycle_before: usize,
    fnum: u16,
    block: u8,
    multiplier: u8,
    pm_enabled: bool,
    pm_position: u8,
    phase_next_reset: bool,
}

fn parse_sample(line: &str, line_number: usize) -> Result<Sample, Box<dyn Error>> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() != 8 {
        return Err(format!("line {line_number}: expected 8 CSV columns").into());
    }

    let pm_enabled = match fields[5] {
        "0" => false,
        "1" => true,
        other => {
            return Err(format!("line {line_number}: invalid pm_enabled {other}").into());
        }
    };
    let phase_next_reset = match fields[7] {
        "0" => false,
        "1" => true,
        other => {
            return Err(format!("line {line_number}: invalid phase_next_reset {other}").into());
        }
    };

    Ok(Sample {
        call_index: fields[0].parse()?,
        cycle_before: fields[1].parse()?,
        fnum: fields[2].parse()?,
        block: fields[3].parse()?,
        multiplier: fields[4].parse()?,
        pm_enabled,
        pm_position: fields[6].parse()?,
        phase_next_reset,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).ok_or(
        "usage: cargo run --manifest-path native/ym2413/Cargo.toml --example pg_phase_trace -- input.csv",
    )?;
    let input = BufReader::new(File::open(Path::new(&path))?);
    let mut samples = Vec::new();

    for (index, line) in input.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        samples.push(parse_sample(trimmed, index + 1)?);
    }

    let mut phases: [PhaseGenerator; 18] = std::array::from_fn(|_| PhaseGenerator::new());
    let mut previous: Option<Sample> = None;
    let mut previous_call: Option<u64> = None;
    let mut previous_cycle: Option<usize> = None;

    for sample in samples {
        if let Some(call) = previous_call {
            if sample.call_index != call + 1 {
                return Err("call indices must be consecutive".into());
            }
        }
        if let Some(cycle) = previous_cycle {
            if sample.cycle_before != (cycle + 1) % 18 {
                return Err("serial cycle indices must advance modulo 18".into());
            }
        }

        let update_slot = (sample.cycle_before + 17) % 18;
        if let Some(prior) = previous {
            if prior.phase_next_reset {
                phases[update_slot].reset();
            }
            phases[update_slot]
                .advance(
                    prior.fnum,
                    prior.block,
                    prior.multiplier,
                    prior.pm_enabled,
                    prior.pm_position,
                )
                .map_err(|error| format!("invalid PG input: {error:?}"))?;
        }

        let phase_values = phases.iter().map(PhaseGenerator::phase).collect::<Vec<_>>();
        let phase_json = phase_values
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");

        println!(
            "{{\"call_index\":{},\"update_slot\":{},\"phases\":[{}]}}",
            sample.call_index, update_slot, phase_json
        );

        previous = Some(sample);
        previous_call = Some(sample.call_index);
        previous_cycle = Some(sample.cycle_before);
    }

    Ok(())
}
