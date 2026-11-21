use sega_ym2413::Ym2413OutputPipeline;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| invalid_data("usage: output_pipeline_trace <nuked-output-pipeline.csv>"))?;
    let reader = BufReader::new(File::open(path)?);
    let mut pipeline = Ym2413OutputPipeline::new();
    let mut calls = 0_u64;
    let mut qualified_mode_writes = 0_u64;
    let mut nonzero_channel_outputs = 0_u64;

    for (line_index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() || line.starts_with("call_index,") {
            continue;
        }
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 9 {
            return Err(invalid_data(format!(
                "line {}: expected 9 CSV fields, found {}",
                line_index + 1,
                fields.len()
            ))
            .into());
        }

        let call_index = fields[0].parse::<u64>()?;
        let cycle = fields[1].parse::<u8>()?;
        let next_channel_output = fields[2].parse::<i16>()?;
        let mode_write_data = if fields[3].is_empty() {
            None
        } else {
            Some(fields[3].parse::<u8>()?)
        };
        let expected_buffer_m = fields[4].parse::<i16>()?;
        let expected_buffer_r = fields[5].parse::<i16>()?;
        let expected_pin_m = fields[6].parse::<i16>()?;
        let expected_pin_r = fields[7].parse::<i16>()?;
        let expected_rm_enable = fields[8].parse::<i8>()?;

        if call_index != calls || cycle != (call_index % 18) as u8 {
            return Err(invalid_data(format!(
                "non-contiguous trace at line {}: call={} cycle={} expected_call={} expected_cycle={}",
                line_index + 1,
                call_index,
                cycle,
                calls,
                calls % 18
            ))
            .into());
        }

        let frame = pipeline.process_cycle(cycle, next_channel_output, mode_write_data)?;
        let state = pipeline.export_state();

        if frame.mo.value() != expected_buffer_m || frame.ro.value() != expected_buffer_r {
            return Err(invalid_data(format!(
                "buffer mismatch at call {} cycle {}: expected MO/RO={}/{}, got {}/{}; mode_write={:?}; ch_out_next={}",
                call_index,
                cycle,
                expected_buffer_m,
                expected_buffer_r,
                frame.mo.value(),
                frame.ro.value(),
                mode_write_data,
                next_channel_output
            ))
            .into());
        }

        if state.output_m != expected_pin_m
            || state.output_r != expected_pin_r
            || state.rm_enable != expected_rm_enable
            || state.previous_channel_output != next_channel_output
            || state.next_cycle != (cycle + 1) % 18
        {
            return Err(invalid_data(format!(
                "pipeline-state mismatch at call {} cycle {}: expected pins={}/{}, rm_enable={}, ch_out={}, next_cycle={}; got pins={}/{}, rm_enable={}, ch_out={}, next_cycle={}; mode_write={:?}",
                call_index,
                cycle,
                expected_pin_m,
                expected_pin_r,
                expected_rm_enable,
                next_channel_output,
                (cycle + 1) % 18,
                state.output_m,
                state.output_r,
                state.rm_enable,
                state.previous_channel_output,
                state.next_cycle,
                mode_write_data
            ))
            .into());
        }

        if mode_write_data.is_some() {
            qualified_mode_writes += 1;
        }
        if next_channel_output != 0 {
            nonzero_channel_outputs += 1;
        }
        calls += 1;
    }

    if calls == 0 {
        return Err(invalid_data("oracle trace contained no samples").into());
    }
    if qualified_mode_writes < 2 {
        return Err(invalid_data(format!(
            "expected Rhythm Mode enable and disable writes, observed {qualified_mode_writes}"
        ))
        .into());
    }

    println!(
        "{{\"schema\":\"ym2413-output-pipeline-differential-v1\",\"result\":\"PASS\",\"calls\":{calls},\"qualified_rhythm_writes\":{qualified_mode_writes},\"nonzero_channel_outputs\":{nonzero_channel_outputs}}}"
    );
    Ok(())
}
