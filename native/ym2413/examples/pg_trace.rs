//! Emit YM2413 PG increments for the normalized Nuked oracle inputs.
//!
//! Input CSV columns:
//! call_index,fnum,block,mul,pm_enabled,pm_position
//!
//! This example exercises the production PhaseGenerator implementation. It
//! deliberately does not advance a slot clock or claim a Nuked slot mapping.

use sega_ym2413::PhaseGenerator;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).ok_or(
        "usage: cargo run --manifest-path native/ym2413/Cargo.toml --example pg_trace -- input.csv",
    )?;
    let input = BufReader::new(File::open(Path::new(&path))?);

    for (line_number, line) in input.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let fields: Vec<&str> = trimmed.split(',').map(str::trim).collect();
        if fields.len() != 6 {
            return Err(format!("line {}: expected 6 CSV columns", line_number + 1).into());
        }

        let call_index: u64 = fields[0].parse()?;
        let fnum: u16 = fields[1].parse()?;
        let block: u8 = fields[2].parse()?;
        let multiplier: u8 = fields[3].parse()?;
        let pm_enabled = match fields[4] {
            "0" => false,
            "1" => true,
            other => {
                return Err(format!("line {}: invalid pm_enabled {other}", line_number + 1).into());
            }
        };
        let pm_position: u8 = fields[5].parse()?;

        let phase_step = PhaseGenerator::phase_step(
            fnum,
            block,
            multiplier,
            pm_enabled,
            pm_position,
        )
        .map_err(|error| format!("invalid PG input at line {}: {error:?}", line_number + 1))?;

        println!("{{\"call_index\":{call_index},\"phase_step\":{phase_step}}}");
    }

    Ok(())
}
