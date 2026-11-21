//! Compare the actual Nuked OPLL_PhaseGenerate reset decision with the
//! production Rust serial-slot selector for Rhythm Mode cycles 13 and 14,
//! using the post-shift mode latch and pre-update EG DOKON state.
//!
//! Input is the external oracle's YM2413_PHASE_RESET CSV stream. Nuked source
//! remains an external CI-only reference; this example invokes only the
//! independently authored Rust implementation.

use sega_ym2413::nuked_slot_service_event_with_testmode;
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;

fn parse<T: std::str::FromStr>(value: &str, field: &str) -> Result<T, Box<dyn Error>>
where
    T::Err: Error + 'static,
{
    value
        .parse::<T>()
        .map_err(|error| format!("invalid {field} value {value:?}: {error}").into())
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: phase_reset_trace <Nuked phase-reset CSV>")?;
    let contents = fs::read_to_string(path)?;
    let mut compared = 0usize;
    let mut probes = BTreeSet::new();

    for (line_index, line) in contents.lines().enumerate() {
        if !line.starts_with("YM2413_PHASE_RESET,") {
            continue;
        }

        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 7 {
            return Err(format!(
                "line {}: expected 7 CSV fields, found {}",
                line_index + 1,
                fields.len()
            )
            .into());
        }

        let cycle: u8 = parse(fields[1], "cycle")?;
        let rm_enable: u8 = parse(fields[2], "rm_enable")?;
        let eg_dokon: u32 = parse(fields[3], "eg_dokon")?;
        let testmode: u8 = parse(fields[4], "testmode")?;
        let nuked_is_modulator: u32 = parse(fields[5], "ismod")?;
        let nuked_reset_required: u32 = parse(fields[6], "reset_required")?;

        if !matches!(cycle, 13 | 14) {
            return Err(format!("line {}: unexpected cycle {cycle}", line_index + 1).into());
        }
        if rm_enable & 0x40 == 0 {
            return Err(format!(
                "line {}: post-shift Rhythm Mode selector is not active: {rm_enable:#04x}",
                line_index + 1
            )
            .into());
        }
        if nuked_is_modulator != 0 {
            return Err(format!(
                "line {}: Nuked did not apply the Rhythm Mode carrier override",
                line_index + 1
            )
            .into());
        }

        let event = nuked_slot_service_event_with_testmode(cycle, rm_enable, eg_dokon, testmode)
            .ok_or_else(|| format!("line {}: Rust rejected cycle {cycle}", line_index + 1))?;
        let rust_reset_required = if event.phase_reset_required { 1 } else { 0 };
        if rust_reset_required != nuked_reset_required {
            return Err(format!(
                "line {}: reset decision mismatch: Nuked={} Rust={} (cycle={cycle}, rm_enable={rm_enable:#04x}, eg_dokon={eg_dokon:#010x}, testmode={testmode:#04x})",
                line_index + 1,
                nuked_reset_required,
                rust_reset_required
            )
            .into());
        }

        probes.insert((cycle, eg_dokon, testmode));
        compared += 1;
    }

    if compared != 16 || probes.len() != 16 {
        return Err(format!(
            "expected 16 distinct actual Nuked decisions, compared {compared} across {} probes",
            probes.len()
        )
        .into());
    }

    println!(
        "{{\"status\":\"PASS\",\"compared_actual_nuked_decisions\":{compared},\"cycles\":[13,14],\"eg_dokon_values\":[32768,1,0,32769],\"testmode_values\":[0,4],\"selector\":\"Ym2413Core production slot-service event\"}}"
    );
    Ok(())
}
