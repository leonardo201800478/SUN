//! Release-mode A/B workload for the DP0 essential trace.
//!
//! Run against the same source example on the base and PR revisions. The
//! workload is deterministic and uses only in-memory ROM and machine state.

use std::hint::black_box;
use std::time::Instant;

use sega_sms_core::SmsTemporalMachine;

const FRAMES_PER_SAMPLE: usize = 30;
const SAMPLE_COUNT: usize = 5;
const MAX_INSTRUCTIONS_PER_FRAME: usize = 50_000;

fn rom_with_program(program: &[u8]) -> Vec<u8> {
    let mut rom = vec![0; 0x4000];
    rom[..program.len()].copy_from_slice(program);
    rom
}

fn run_scenario(name: &str, program: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // Warm caches and one-time initialization outside the measured samples.
    let mut warmup = SmsTemporalMachine::new(rom_with_program(program), 3)?;
    for _ in 0..2 {
        warmup.step_frame(MAX_INSTRUCTIONS_PER_FRAME)?;
    }
    black_box(warmup.now());

    for sample in 0..SAMPLE_COUNT {
        let mut machine = SmsTemporalMachine::new(rom_with_program(program), 3)?;
        let start = Instant::now();
        let mut instructions = 0_u64;

        for _ in 0..FRAMES_PER_SAMPLE {
            instructions += machine.step_frame(MAX_INSTRUCTIONS_PER_FRAME)? as u64;
        }

        let elapsed_ns = start.elapsed().as_nanos();
        let ns_per_instruction = elapsed_ns as f64 / instructions.max(1) as f64;
        let final_ticks = machine.now().ticks();
        black_box(final_ticks);
        println!(
            "DP0_BENCH scenario={name} sample={sample} frames={FRAMES_PER_SAMPLE} instructions={instructions} ticks={final_ticks} elapsed_ns={elapsed_ns} ns_per_instruction={ns_per_instruction:.3}"
        );
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // One branch-heavy low-data workload and one repeated memory-write workload.
    run_scenario("relative-loop", &[0x18, 0xFE])?;
    run_scenario("bus-heavy", &[0x32, 0x00, 0xC0, 0x18, 0xFB])?;
    Ok(())
}
