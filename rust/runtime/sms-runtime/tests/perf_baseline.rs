//! Opt-in, uncapped OutRun throughput baseline for SUN PERF-01/S0.
//!
//! This harness executes the same legally obtained OutRun ROM in three physical
//! board/output configurations:
//! A: generic SMS board without YM2413;
//! B: Japanese SMS board with YM2413 clocked and FM output muted;
//! C: Japanese SMS board with YM2413 clocked and FM mixed into the native
//!    mixer/resampler, then consumed by a deterministic discard sink.
//!
//! Host frame pacing, WGPU presentation and CPAL are deliberately excluded from
//! the throughput pass. CPAL/device behavior is a separate acceptance run.
//! The C path proves software FM mixing is exercised; it does not claim that
//! physical audio was heard. Diagnostics are collected in a separate pass so
//! their instrumentation overhead does not contaminate the throughput result.
//!
//! PowerShell:
//!   $env:SEGA_PERF_ROM_PATH = "D:\\ROMS\\Sega Master System\\OutRun (Japan).sms"
//!   $env:SEGA_PERF_WARMUP_SECS = "5"       # optional, default 5
//!   $env:SEGA_PERF_CAPTURE_SECS = "30"      # optional, default 30
//!   $env:SEGA_PERF_DIAGNOSTIC_SECS = "5"    # optional, default 5
//!   $env:SEGA_PERF_OUTPUT_CSV = "D:\\SUN-Benchmarks\\perf-s0-outrun.csv" # optional
//!   cargo test --release -p sega-sms-runtime --test perf_baseline outrun_a_b_c_uncapped_baseline -- --ignored --nocapture
//!
//! The test never writes ROM bytes into the repository. It records the ROM SHA-256,
//! scenario, frame count, elapsed time, uncapped throughput and diagnostic percentiles.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use sega_audio_runtime::{AudioOutputMetrics, NativeAudioSession, PcmSink};
use sega_media::CartridgeIdentity;
use sega_sms_runtime::{AudioSession, SmsFmOutputPolicy, SmsTemporalWorker, SmsVdpRevision};

const DEFAULT_WARMUP_SECS: u64 = 5;
const DEFAULT_CAPTURE_SECS: u64 = 30;
const DEFAULT_DIAGNOSTIC_SECS: u64 = 5;
const POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Clone, Default)]
struct DiscardSink {
    samples_written: Arc<AtomicU64>,
}

impl PcmSink for DiscardSink {
    fn write_samples(&mut self, samples: Vec<f64>) -> Result<(), String> {
        self.write_samples_ref(&samples)
    }

    fn write_samples_ref(&mut self, samples: &[f64]) -> Result<(), String> {
        self.samples_written
            .fetch_add(samples.len() as u64, Ordering::Relaxed);
        Ok(())
    }

    fn suspend_and_flush(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn resume(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn metrics(&self) -> AudioOutputMetrics {
        AudioOutputMetrics {
            sample_rate_hz: 48_000,
            ..AudioOutputMetrics::default()
        }
    }

    fn close(&mut self) {}
}

#[derive(Debug, Clone, Copy)]
enum Scenario {
    ANoYm2413,
    BYm2413Muted,
    CYm2413Mixed,
}

impl Scenario {
    const ALL: [Self; 3] = [Self::ANoYm2413, Self::BYm2413Muted, Self::CYm2413Mixed];

    fn name(self) -> &'static str {
        match self {
            Self::ANoYm2413 => "A-no-ym2413",
            Self::BYm2413Muted => "B-ym2413-clocked-muted",
            Self::CYm2413Mixed => "C-ym2413-fm-mixed",
        }
    }
}

#[derive(Debug)]
struct ThroughputResult {
    scenario: &'static str,
    frames: u64,
    elapsed: Duration,
    fps: f64,
    sink_samples: u64,
    fm_address_writes: u64,
    fm_data_writes: u64,
}

fn env_seconds(name: &str, default: u64) -> Duration {
    let seconds = std::env::var(name)
        .ok()
        .map(|value| {
            value
                .parse::<u64>()
                .unwrap_or_else(|_| panic!("{name} must be a positive integer"))
        })
        .unwrap_or(default);
    assert!(seconds > 0, "{name} must be positive");
    Duration::from_secs(seconds)
}

fn load_outrun_rom() -> (PathBuf, Vec<u8>, String) {
    let path = std::env::var_os("SEGA_PERF_ROM_PATH")
        .map(PathBuf::from)
        .expect("set SEGA_PERF_ROM_PATH to a legally obtained OutRun SMS ROM");
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    assert!(
        file_name.to_ascii_lowercase().contains("outrun"),
        "S0 primary benchmark must be OutRun; supplied file name was '{file_name}'"
    );
    let rom = fs::read(&path)
        .unwrap_or_else(|error| panic!("cannot read ROM {}: {error}", path.display()));
    assert!(!rom.is_empty(), "ROM {} is empty", path.display());
    let sha256 = CartridgeIdentity::from_bytes(&rom).sha256_hex();
    (path, rom, sha256)
}

fn build_worker(rom: Vec<u8>, scenario: Scenario) -> SmsTemporalWorker {
    match scenario {
        Scenario::ANoYm2413 => SmsTemporalWorker::new(rom, "japan"),
        Scenario::BYm2413Muted => {
            SmsTemporalWorker::new_with_japanese_fm_hardware_profile_and_output_policy(
                rom,
                "japan",
                SmsVdpRevision::Sega3155124,
                None,
                None,
                SmsFmOutputPolicy::Muted,
            )
        }
        Scenario::CYm2413Mixed => {
            SmsTemporalWorker::new_with_japanese_fm_hardware_profile_and_output_policy(
                rom,
                "japan",
                SmsVdpRevision::Sega3155124,
                None,
                None,
                SmsFmOutputPolicy::Enabled,
            )
        }
    }
    .unwrap_or_else(|error| panic!("cannot construct {} machine: {error}", scenario.name()))
}

fn configure_headless_audio(worker: &SmsTemporalWorker) -> Arc<AtomicU64> {
    let samples_written = Arc::new(AtomicU64::new(0));
    let sink_counter = Arc::clone(&samples_written);
    worker
        .configure_audio_output(move || {
            let sink = DiscardSink {
                samples_written: sink_counter,
            };
            let session = NativeAudioSession::new(48_000, 48_000, 100, sink)?;
            Ok(Box::new(session) as Box<dyn AudioSession>)
        })
        .unwrap_or_else(|error| panic!("cannot configure headless audio session: {error}"));
    samples_written
}

fn check_worker(worker: &SmsTemporalWorker, scenario: Scenario) {
    if let Some(error) = worker.runtime_error() {
        panic!("{} runtime failed: {error}", scenario.name());
    }
    if let Some(error) = worker.audio_output_error() {
        panic!("{} audio pipeline failed: {error}", scenario.name());
    }
}

fn wait_running_for(worker: &SmsTemporalWorker, scenario: Scenario, duration: Duration) {
    let started = Instant::now();
    while started.elapsed() < duration {
        check_worker(worker, scenario);
        thread::sleep(POLL_INTERVAL);
    }
}

fn run_throughput(
    rom: &[u8],
    scenario: Scenario,
    warmup: Duration,
    capture: Duration,
) -> ThroughputResult {
    let worker = build_worker(rom.to_vec(), scenario);
    let samples_written = configure_headless_audio(&worker);

    // Disable only host pacing. MachineScheduler remains the sole authority for
    // virtual hardware time, and no frame deadline is fed back into emulation.
    worker
        .set_frame_pacing_enabled(false)
        .unwrap_or_else(|error| panic!("cannot disable host pacing: {error}"));
    worker
        .start()
        .unwrap_or_else(|error| panic!("cannot start {}: {error}", scenario.name()));

    wait_running_for(&worker, scenario, warmup);
    let start_frame = worker.frame_number();
    let started = Instant::now();
    wait_running_for(&worker, scenario, capture);
    let elapsed = started.elapsed();
    worker
        .pause()
        .unwrap_or_else(|error| panic!("cannot pause {}: {error}", scenario.name()));

    let frames = worker.frame_number().saturating_sub(start_frame);
    assert!(
        frames > 0,
        "{} produced no completed frames",
        scenario.name()
    );
    let fps = frames as f64 / elapsed.as_secs_f64();
    let snapshot = worker
        .debug_snapshot()
        .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", scenario.name()));
    let _ = worker.close_audio_output();

    ThroughputResult {
        scenario: scenario.name(),
        frames,
        elapsed,
        fps,
        sink_samples: samples_written.load(Ordering::Relaxed),
        fm_address_writes: snapshot.fm_port_writes.address_writes,
        fm_data_writes: snapshot.fm_port_writes.data_writes,
    }
}

fn run_diagnostic_pass(rom: &[u8], scenario: Scenario, duration: Duration) {
    let worker = build_worker(rom.to_vec(), scenario);
    let _samples_written = configure_headless_audio(&worker);
    worker.set_diagnostics_enabled(true);
    worker
        .set_frame_pacing_enabled(false)
        .unwrap_or_else(|error| panic!("cannot disable host pacing: {error}"));
    worker
        .start()
        .unwrap_or_else(|error| panic!("cannot start diagnostic {}: {error}", scenario.name()));

    wait_running_for(&worker, scenario, duration);
    worker
        .pause()
        .unwrap_or_else(|error| panic!("cannot pause diagnostic {}: {error}", scenario.name()));
    let metrics = worker.frame_pacing_metrics();

    println!(
        "SUN_PERF_DIAGNOSTICS scenario={} samples={} total_samples={} frame_interval_ns_p50={} frame_interval_ns_p95={} frame_interval_ns_p99={} step_frame_ns_p50={} step_frame_ns_p95={} step_frame_ns_p99={} frame_processing_ns_p50={} frame_processing_ns_p95={} frame_processing_ns_p99={} video_publish_ns_p50={} video_publish_ns_p95={} video_publish_ns_p99={} audio_submit_ns_p50={} audio_submit_ns_p95={} audio_submit_ns_p99={}",
        scenario.name(),
        metrics.sample_count,
        metrics.total_samples,
        metrics.frame_interval_p50_ns,
        metrics.frame_interval_p95_ns,
        metrics.frame_interval_p99_ns,
        metrics.step_frame_p50_ns,
        metrics.step_frame_p95_ns,
        metrics.step_frame_p99_ns,
        metrics.frame_processing_p50_ns,
        metrics.frame_processing_p95_ns,
        metrics.frame_processing_p99_ns,
        metrics.video_publish_p50_ns,
        metrics.video_publish_p95_ns,
        metrics.video_publish_p99_ns,
        metrics.audio_submit_p50_ns,
        metrics.audio_submit_p95_ns,
        metrics.audio_submit_p99_ns,
    );

    let _ = worker.close_audio_output();
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[test]
#[ignore = "requires a legally obtained OutRun SMS ROM; see module documentation"]
fn outrun_a_b_c_uncapped_baseline() {
    let warmup = env_seconds("SEGA_PERF_WARMUP_SECS", DEFAULT_WARMUP_SECS);
    let capture = env_seconds("SEGA_PERF_CAPTURE_SECS", DEFAULT_CAPTURE_SECS);
    let diagnostic = env_seconds("SEGA_PERF_DIAGNOSTIC_SECS", DEFAULT_DIAGNOSTIC_SECS);
    let (rom_path, rom, sha256) = load_outrun_rom();

    println!(
        "SUN_PERF_MANIFEST rom={} sha256={} region=japan vdp=sms1 warmup_secs={} capture_secs={} diagnostic_secs={} pacing=disabled presentation=disabled audio_sink=headless",
        rom_path.display(),
        sha256,
        warmup.as_secs(),
        capture.as_secs(),
        diagnostic.as_secs(),
    );

    let mut results = Vec::with_capacity(Scenario::ALL.len());
    for scenario in Scenario::ALL {
        let result = run_throughput(&rom, scenario, warmup, capture);
        println!(
            "SUN_PERF_RESULT scenario={} frames={} elapsed_ns={} throughput_fps={:.4} sink_samples={} fm_address_writes={} fm_data_writes={}",
            result.scenario,
            result.frames,
            result.elapsed.as_nanos(),
            result.fps,
            result.sink_samples,
            result.fm_address_writes,
            result.fm_data_writes,
        );
        results.push(result);
    }

    for scenario in Scenario::ALL {
        run_diagnostic_pass(&rom, scenario, diagnostic);
    }

    if let Some(output_path) = std::env::var_os("SEGA_PERF_OUTPUT_CSV") {
        let output_path = PathBuf::from(output_path);
        if let Some(parent) = output_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).unwrap_or_else(|error| {
                panic!(
                    "cannot create output directory {}: {error}",
                    parent.display()
                )
            });
        }
        let mut csv = String::from(
            "scenario,rom_path,rom_sha256,region,vdp,frames,elapsed_ns,throughput_fps,sink_samples,fm_address_writes,fm_data_writes\n",
        );
        for result in &results {
            csv.push_str(&format!(
                "{},{},{},japan,sms1,{},{},{:.4},{},{},{}\n",
                csv_escape(result.scenario),
                csv_escape(&rom_path.to_string_lossy()),
                sha256,
                result.frames,
                result.elapsed.as_nanos(),
                result.fps,
                result.sink_samples,
                result.fm_address_writes,
                result.fm_data_writes,
            ));
        }
        fs::write(&output_path, csv)
            .unwrap_or_else(|error| panic!("cannot write CSV {}: {error}", output_path.display()));
        println!("SUN_PERF_CSV path={}", output_path.display());
    }
}
