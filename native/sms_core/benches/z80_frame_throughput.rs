//! Reproducible synthetic workload for the scheduler-driven SMS CPU/frame path.
//!
//! This is a comparative microbenchmark, not a game-performance claim. It executes
//! a deterministic Z80 loop from a synthetic 16 KiB ROM through SmsTemporalMachine,
//! including the real MachineScheduler, SMS bus and VDP frame boundary. It excludes
//! the worker thread, host audio mixer, WGPU presentation and CPAL.
//!
//! Run with:
//!   cargo bench --locked -p sega-sms-core --bench z80_frame_throughput --profile release
//!   cargo bench --locked -p sega-sms-core --bench z80_frame_throughput --profile release-llvm

use std::hint::black_box;
use std::time::Instant;

use sega_sms_core::SmsTemporalMachine;

const ROM_SIZE: usize = 0x4000;
const MAX_INSTRUCTIONS_PER_FRAME: usize = 250_000;

fn env_usize(name: &str, default: usize) -> usize {
    let value = std::env::var(name)
        .ok()
        .map(|raw| raw.parse::<usize>().unwrap_or_else(|_| panic!("{name} must be an integer")))
        .unwrap_or(default);
    assert!(value > 0, "{name} must be positive");
    value
}

/// Build a deterministic loop exercising M1 fetch, register ALU, memory writes
/// and a conditional relative branch without depending on copyrighted game ROMs.
fn synthetic_rom() -> Vec<u8> {
    let mut rom = vec![0x00; ROM_SIZE];
    let program = [
        0x31, 0xF0, 0xDF, // LD SP, $DFF0
        0x21, 0x00, 0xC0, // LD HL, $C000
        0x3E, 0x12,       // LD A, $12
        0x23,             // loop: INC HL
        0x04,             // INC B
        0x80,             // ADD A, B
        0xA9,             // XOR C
        0x77,             // LD (HL), A
        0x0D,             // DEC C
        0x20, 0xF8,       // JR NZ, loop
        0xC3, 0x08, 0x00, // JP loop
    ];
    rom[..program.len()].copy_from_slice(&program);
    rom
}

#[derive(Clone, Copy)]
struct Sample {
    elapsed_ms: f64,
    ms_per_frame: f64,
    fps: f64,
    instructions_per_frame: f64,
}

fn run_trial(warmup_frames: usize, measure_frames: usize) -> Sample {
    let mut machine = SmsTemporalMachine::new(synthetic_rom(), 1)
        .expect("construct synthetic scheduler-driven SMS machine");

    for _ in 0..warmup_frames {
        black_box(
            machine
                .step_frame(MAX_INSTRUCTIONS_PER_FRAME)
                .expect("complete warmup frame"),
        );
    }

    let start_frame = machine.frame_number();
    let started = Instant::now();
    let mut instructions = 0_u64;

    for _ in 0..measure_frames {
        instructions += machine
            .step_frame(MAX_INSTRUCTIONS_PER_FRAME)
            .expect("complete measured frame") as u64;
    }

    let elapsed = started.elapsed();
    let completed_frames = machine.frame_number().saturating_sub(start_frame);
    assert_eq!(
        completed_frames, measure_frames as u64,
        "each step_frame call must retire exactly one VDP frame"
    );

    let elapsed_ms = elapsed.as_secs_f64() * 1_000.0;
    let ms_per_frame = elapsed_ms / completed_frames as f64;
    Sample {
        elapsed_ms,
        ms_per_frame,
        fps: completed_frames as f64 / elapsed.as_secs_f64(),
        instructions_per_frame: instructions as f64 / completed_frames as f64,
    }
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) * 0.5
    } else {
        values[middle]
    }
}

fn main() {
    let warmup_frames = env_usize("SUN_PERF_WARMUP_FRAMES", 5);
    let measure_frames = env_usize("SUN_PERF_MEASURE_FRAMES", 60);
    let repeats = env_usize("SUN_PERF_REPEATS", 5);
    let profile = std::env::var("SUN_PERF_PROFILE").unwrap_or_else(|_| "unspecified".to_owned());

    let mut samples = Vec::with_capacity(repeats);
    for trial in 1..=repeats {
        let sample = run_trial(warmup_frames, measure_frames);
        println!(
            "SUN_PERF_SAMPLE profile={profile} trial={trial} frames={measure_frames} elapsed_ms={:.3} ms_per_frame={:.6} fps={:.3} instructions_per_frame={:.2}",
            sample.elapsed_ms,
            sample.ms_per_frame,
            sample.fps,
            sample.instructions_per_frame,
        );
        samples.push(sample);
    }

    let mut ms_values: Vec<f64> = samples.iter().map(|sample| sample.ms_per_frame).collect();
    let mut fps_values: Vec<f64> = samples.iter().map(|sample| sample.fps).collect();
    let mut instruction_values: Vec<f64> = samples
        .iter()
        .map(|sample| sample.instructions_per_frame)
        .collect();
    let median_ms = median(&mut ms_values);
    let median_fps = median(&mut fps_values);
    let median_instructions = median(&mut instruction_values);
    let min_ms = ms_values[0];
    let max_ms = ms_values[ms_values.len() - 1];

    println!(
        "SUN_PERF_SUMMARY profile={profile} repeats={repeats} warmup_frames={warmup_frames} measured_frames_per_trial={measure_frames} median_ms_per_frame={median_ms:.6} min_ms_per_frame={min_ms:.6} max_ms_per_frame={max_ms:.6} median_fps={median_fps:.3} median_instructions_per_frame={median_instructions:.2}"
    );
}
