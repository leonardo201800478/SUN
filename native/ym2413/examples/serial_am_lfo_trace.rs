//! Replay the serial AM LFO shift/carry network against Nuked OPLL_DoLFO.
//!
//! Input CSV columns:
//! call,cycle,lfo_counter,testmode,am_counter,am_direction,am_carry,am_step,am_output

use sega_ym2413::Ym2413SerialAmLfo;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn parse<T: std::str::FromStr>(fields: &[&str], index: usize) -> Result<T, Box<dyn Error>>
where
    T::Err: Error + 'static,
{
    Ok(fields[index].parse()?)
}

fn parse_bit(fields: &[&str], index: usize, line: usize) -> Result<bool, Box<dyn Error>> {
    match fields[index] {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(format!("line {line}: expected bit 0 or 1, got {other}").into()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).ok_or(
        "usage: cargo run --manifest-path native/ym2413/Cargo.toml --example serial_am_lfo_trace -- input.csv",
    )?;
    let input = BufReader::new(File::open(Path::new(&path))?);
    let mut lfo = Ym2413SerialAmLfo::new();
    let mut compared = 0_u64;

    for (line_number, line) in input.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = trimmed.split(',').map(str::trim).collect();
        if fields.len() != 9 {
            return Err(format!(
                "line {}: expected 9 CSV columns, got {}",
                line_number + 1,
                fields.len()
            )
            .into());
        }

        let call: u64 = parse(&fields, 0)?;
        let cycle: u8 = parse(&fields, 1)?;
        let lfo_counter: u16 = parse(&fields, 2)?;
        let testmode: u8 = parse(&fields, 3)?;
        lfo.capture_output(cycle)
            .map_err(|error| format!("line {}: capture failed: {error:?}", line_number + 1))?;
        lfo.clock(cycle, lfo_counter, testmode).map_err(|error| {
            format!(
                "line {}: serial AM clock failed: {error:?}",
                line_number + 1
            )
        })?;

        let actual = lfo.export_state();
        let expected_counter: u16 = parse(&fields, 4)?;
        let expected_direction = parse_bit(&fields, 5, line_number + 1)?;
        let expected_carry = parse_bit(&fields, 6, line_number + 1)?;
        let expected_step = parse_bit(&fields, 7, line_number + 1)?;
        let expected_output: u8 = parse(&fields, 8)?;

        if actual.counter != expected_counter
            || actual.direction != expected_direction
            || actual.carry != expected_carry
            || actual.step != expected_step
            || actual.output != expected_output
        {
            return Err(format!(
                "call {call} cycle {cycle}: AM LFO mismatch: counter {:#05x}/{:#05x}, direction {}/{}, carry {}/{}, step {}/{}, output {}/{}",
                actual.counter,
                expected_counter,
                actual.direction,
                expected_direction,
                actual.carry,
                expected_carry,
                actual.step,
                expected_step,
                actual.output,
                expected_output
            )
            .into());
        }
        compared += 1;
    }

    println!(
        "{{\"status\":\"PASS\",\"compared_calls\":{compared},\"fields\":[\"lfo_am_counter\",\"lfo_am_dir\",\"lfo_am_car\",\"lfo_am_step\",\"lfo_am_out\"],\"scope\":\"serial Nuked OPLL_DoLFO AM shift/carry state\"}}"
    );
    Ok(())
}
