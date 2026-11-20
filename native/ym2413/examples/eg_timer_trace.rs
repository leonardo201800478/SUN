//! Replay the shared YM2413 EG timer against fields captured from Nuked-OPLL.
//!
//! The input CSV is extracted from a pinned trace. This example validates the
//! chip-global timer and its one-slot rate-latch pipeline independently from
//! per-operator envelope levels and stages.

use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use sega_ym2413::envelope_timer::EnvelopeTimer;

const FIELD_NAMES: [&str; 13] = [
    "timer",
    "carry",
    "counter_state",
    "counter_state_prev",
    "shift",
    "shift_stop",
    "low_lock",
    "shift_lock",
    "rate_hi",
    "inc_lo",
    "inc_hi",
    "max_rate",
    "zero_rate",
];

#[derive(Debug)]
struct FirstMismatch {
    call_index: u64,
    slot_index: u8,
    field: &'static str,
    expected: u32,
    actual: u32,
}

fn parse_u32(value: &str, field: &str) -> Result<u32, Box<dyn Error>> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {field} value {value:?}: {error}").into())
}

fn record_mismatch(
    index: usize,
    field: &'static str,
    expected: u32,
    actual: u32,
    call_index: u64,
    slot_index: u8,
    counts: &mut [u64; FIELD_NAMES.len()],
    first: &mut Option<FirstMismatch>,
) {
    if expected == actual {
        return;
    }
    counts[index] += 1;
    if first.is_none() {
        *first = Some(FirstMismatch {
            call_index,
            slot_index,
            field,
            expected,
            actual,
        });
    }
}

fn run(path: &Path) -> Result<(), Box<dyn Error>> {
    let input = BufReader::new(File::open(path)?);
    let mut lines = input.lines();
    let _header = lines.next().ok_or("timer CSV is empty")??;

    let mut timer = EnvelopeTimer::new();
    let mut counts = [0_u64; FIELD_NAMES.len()];
    let mut first = None;
    let mut calls = 0_u64;

    for (line_index, line) in lines.enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 16 {
            return Err(format!(
                "line {} has {} columns; expected 16",
                line_index + 2,
                fields.len()
            )
            .into());
        }

        let call_index = u64::from(parse_u32(fields[0], "call_index")?);
        let slot_index = u8::try_from(parse_u32(fields[1], "slot_index")?)?;
        let next_rate = u8::try_from(parse_u32(fields[2], "next_rate")?)?;
        let expected_timer = parse_u32(fields[3], "expected_timer")?;
        let expected_carry = parse_u32(fields[4], "expected_carry")?;
        let expected_counter = parse_u32(fields[5], "expected_counter_state")?;
        let expected_counter_prev = parse_u32(fields[6], "expected_counter_state_prev")?;
        let expected_shift = parse_u32(fields[7], "expected_shift")?;
        let expected_shift_stop = parse_u32(fields[8], "expected_shift_stop")?;
        let expected_low_lock = parse_u32(fields[9], "expected_low_lock")?;
        let expected_shift_lock = parse_u32(fields[10], "expected_shift_lock")?;
        let expected_rate_hi = parse_u32(fields[11], "expected_rate_hi")?;
        let expected_inc_lo = parse_u32(fields[12], "expected_inc_lo")?;
        let expected_inc_hi = parse_u32(fields[13], "expected_inc_hi")?;
        let expected_max_rate = parse_u32(fields[14], "expected_max_rate")?;
        let expected_zero_rate = parse_u32(fields[15], "expected_zero_rate")?;

        let snapshot = timer.tick_slot(slot_index, next_rate)?;
        let actual = [
            snapshot.timer,
            u32::from(snapshot.carry),
            u32::from(snapshot.counter_state),
            u32::from(snapshot.counter_state_prev),
            u32::from(snapshot.shift),
            u32::from(snapshot.shift_stop),
            u32::from(snapshot.low_lock),
            u32::from(snapshot.shift_lock),
            u32::from(snapshot.rate_phase.rate_hi),
            u32::from(snapshot.rate_phase.inc_lo),
            u32::from(snapshot.rate_phase.inc_hi),
            u32::from(snapshot.rate_phase.max_rate),
            u32::from(snapshot.rate_phase.zero_rate),
        ];
        let expected = [
            expected_timer,
            expected_carry,
            expected_counter,
            expected_counter_prev,
            expected_shift,
            expected_shift_stop,
            expected_low_lock,
            expected_shift_lock,
            expected_rate_hi,
            expected_inc_lo,
            expected_inc_hi,
            expected_max_rate,
            expected_zero_rate,
        ];

        for index in 0..FIELD_NAMES.len() {
            record_mismatch(
                index,
                FIELD_NAMES[index],
                expected[index],
                actual[index],
                call_index,
                slot_index,
                &mut counts,
                &mut first,
            );
        }
        calls += 1;
    }

    let total_mismatches: u64 = counts.iter().sum();
    let field_counts = FIELD_NAMES
        .iter()
        .zip(counts.iter())
        .map(|(name, count)| format!("\"{name}\":{count}"))
        .collect::<Vec<_>>()
        .join(",");
    let first_json = match first {
        Some(mismatch) => format!(
            "{{\"call_index\":{},\"slot_index\":{},\"field\":\"{}\",\"expected\":{},\"actual\":{}}}",
            mismatch.call_index,
            mismatch.slot_index,
            mismatch.field,
            mismatch.expected,
            mismatch.actual
        ),
        None => "null".to_owned(),
    };

    println!(
        "{{\"calls\":{calls},\"total_field_mismatches\":{total_mismatches},\"field_mismatches\":{{{field_counts}}},\"first_mismatch\":{first_json}}}"
    );

    if total_mismatches != 0 {
        return Err(format!("{total_mismatches} EG timer field mismatches").into());
    }
    Ok(())
}

fn main() {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("usage: eg_timer_trace <timer-samples.csv>");
        std::process::exit(2);
    };
    if let Err(error) = run(Path::new(&path)) {
        eprintln!("EG timer replay failed: {error}");
        std::process::exit(1);
    }
}
