use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use sega_audio_runtime::{AudioOutputMetrics, NativeAudioSession, PcmSink};
use sega_media::CartridgeIdentity;
use sega_sms_runtime::{AudioSession, SmsFmOutputPolicy, SmsTemporalWorker, SmsVdpRevision};

const POLL: Duration = Duration::from_millis(5);
const SIGNAL_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Default)]
struct DiscardSink(Arc<AtomicU64>);

impl PcmSink for DiscardSink {
    fn write_samples(&mut self, samples: Vec<f64>) -> Result<(), String> {
        self.write_samples_ref(&samples)
    }

    fn write_samples_ref(&mut self, samples: &[f64]) -> Result<(), String> {
        self.0.fetch_add(samples.len() as u64, Ordering::Relaxed);
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

#[derive(Clone, Copy)]
enum Scenario {
    A,
    B,
    C,
}

impl Scenario {
    fn from_env() -> Self {
        match std::env::var("SEGA_PERF_PROFILE_SCENARIO").as_deref() {
            Ok("A") => Self::A,
            Ok("B") => Self::B,
            Ok("C") => Self::C,
            _ => panic!("SEGA_PERF_PROFILE_SCENARIO must be A, B or C"),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::A => "A-no-ym2413",
            Self::B => "B-ym2413-clocked-muted",
            Self::C => "C-ym2413-fm-mixed",
        }
    }

    fn worker(self, rom: Vec<u8>) -> SmsTemporalWorker {
        let result = match self {
            Self::A => SmsTemporalWorker::new(rom, "japan"),
            Self::B => SmsTemporalWorker::new_with_japanese_fm_hardware_profile_and_output_policy(
                rom,
                "japan",
                SmsVdpRevision::Sega3155124,
                None,
                None,
                SmsFmOutputPolicy::Muted,
            ),
            Self::C => SmsTemporalWorker::new_with_japanese_fm_hardware_profile_and_output_policy(
                rom,
                "japan",
                SmsVdpRevision::Sega3155124,
                None,
                None,
                SmsFmOutputPolicy::Enabled,
            ),
        };
        result.unwrap_or_else(|error| panic!("cannot construct {} machine: {error}", self.name()))
    }
}

fn seconds(name: &str, default: u64) -> Duration {
    let value = std::env::var(name)
        .ok()
        .map(|v| {
            v.parse::<u64>()
                .unwrap_or_else(|_| panic!("{name} must be an integer"))
        })
        .unwrap_or(default);
    assert!(value > 0, "{name} must be positive");
    Duration::from_secs(value)
}

fn wait(worker: &SmsTemporalWorker, scenario: Scenario, duration: Duration) {
    let start = Instant::now();
    while start.elapsed() < duration {
        if let Some(error) = worker.runtime_error() {
            panic!("{} runtime: {error}", scenario.name());
        }
        if let Some(error) = worker.audio_output_error() {
            panic!("{} audio: {error}", scenario.name());
        }
        thread::sleep(POLL);
    }
}

fn profile_gate(worker: &SmsTemporalWorker, scenario: Scenario) {
    let ready = std::env::var_os("SEGA_PERF_PROFILE_READY_FILE").map(PathBuf::from);
    let go = std::env::var_os("SEGA_PERF_PROFILE_GO_FILE").map(PathBuf::from);
    match (ready, go) {
        (Some(ready), Some(go)) => {
            fs::write(&ready, b"ready").expect("write ETW ready signal");
            let start = Instant::now();
            while !go.is_file() {
                if let Some(error) = worker.runtime_error() {
                    panic!("{} runtime: {error}", scenario.name());
                }
                assert!(
                    start.elapsed() < SIGNAL_TIMEOUT,
                    "ETW runner did not release profile gate"
                );
                thread::sleep(Duration::from_millis(10));
            }
        }
        (None, None) => {}
        _ => panic!("ready and go signal paths must be configured together"),
    }
}

#[test]
#[ignore = "requires a legally obtained OutRun SMS ROM and Windows ETW; see docs/testing/perf-01-z80-cpu-profile.md"]
fn outrun_single_scenario_cpu_profile() {
    let scenario = Scenario::from_env();
    let warmup = seconds("SEGA_PERF_PROFILE_WARMUP_SECS", 5);
    let capture = seconds("SEGA_PERF_PROFILE_CAPTURE_SECS", 30);
    let path = std::env::var_os("SEGA_PERF_ROM_PATH")
        .map(PathBuf::from)
        .expect("set SEGA_PERF_ROM_PATH");
    let name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
    assert!(
        name.to_ascii_lowercase().contains("outrun"),
        "OutRun ROM required"
    );
    let rom = fs::read(&path).expect("read OutRun ROM");
    assert!(!rom.is_empty(), "OutRun ROM is empty");
    let sha256 = CartridgeIdentity::from_bytes(&rom).sha256_hex();

    let worker = scenario.worker(rom);
    let sink_count = Arc::new(AtomicU64::new(0));
    let sink_counter = Arc::clone(&sink_count);
    worker
        .configure_audio_output(move || {
            let sink = DiscardSink(sink_counter);
            let session = NativeAudioSession::new(48_000, 48_000, 100, sink)?;
            Ok(Box::new(session) as Box<dyn AudioSession>)
        })
        .expect("configure headless native audio");
    worker.set_diagnostics_enabled(false);
    worker
        .set_frame_pacing_enabled(false)
        .expect("disable host pacing");
    worker.start().expect("start SMS worker");

    // Worker startup and warmup happen before ETW starts.
    wait(&worker, scenario, warmup);
    profile_gate(&worker, scenario);

    let first_frame = worker.frame_number();
    let start = Instant::now();
    wait(&worker, scenario, capture);
    let elapsed = start.elapsed();
    worker.pause().expect("pause SMS worker");
    let frames = worker.frame_number().saturating_sub(first_frame);
    assert!(frames > 0, "profile run completed no frames");
    let snapshot = worker.debug_snapshot().expect("capture worker snapshot");

    println!(
        "SUN_Z80_CPU_PROFILE_RESULT scenario={} rom_sha256={} frames={} elapsed_ns={} profiled_observed_fps={:.4} pcm_samples={} fm_address_writes={} fm_data_writes={} diagnostics=off pacing=off presentation=off",
        scenario.name(),
        sha256,
        frames,
        elapsed.as_nanos(),
        frames as f64 / elapsed.as_secs_f64(),
        sink_count.load(Ordering::Relaxed),
        snapshot.fm_port_writes.address_writes,
        snapshot.fm_port_writes.data_writes,
    );
    let _ = worker.close_audio_output();
}
