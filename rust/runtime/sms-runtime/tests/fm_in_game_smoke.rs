//! Opt-in in-game FM acceptance campaign for legally obtained Japanese SMS ROMs.
//!
//! This exercises the real ROM -> Z80/Scheduler -> YM2413 -> FM/PSG alignment ->
//! NativeAudioSession mixer/resampler -> PCM sink path. The capture sink is
//! deterministic and headless; no CPAL device is required for this software-path
//! acceptance test. Physical CPAL playback remains a separate manual check.
//!
//! PowerShell:
//!   $env:SEGA_SMS_FM_TEST_ROM_DIR = "D:\ROMs\SMS-FM"
//!   $env:SEGA_SMS_FM_TEST_REGION = "japan"
//!   $env:SEGA_SMS_FM_TEST_SYSTEM = "sms1" # sms1|sms2
//!   $env:SEGA_SMS_FM_TEST_FRAMES = "1800" # optional; default 1800
//!   cargo test --release -p sega-sms-runtime --test fm_in_game_smoke real_japanese_sms_roms_emit_non_silent_fm_through_the_native_mixer -- --ignored --nocapture
//!   cargo test --release -p sega-sms-runtime --test fm_in_game_smoke real_japanese_sms_roms_play_fm_through_cpal_and_report_underruns -- --ignored --nocapture
//!
//! The first test uses a deterministic headless sink. The second opens the default
//! CPAL output device and plays the real game audio while reporting underruns.
//!
//! ROM files are read in place and are never copied into the repository.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use sega_audio_output::NativeCpalOutput;
use sega_audio_runtime::{
    AnalogOutputConfig, AudioOutputMetrics, MixSource, NativeAudioSession, PcmSink,
};
use sega_media::CartridgeIdentity;
use sega_sms_core::SmsTemporalTraceEvent;
use sega_sms_runtime::{AudioSession, SmsFmOutputPolicy, SmsTemporalWorker, SmsVdpRevision};
use sega_z80_cpu::Z80BusOperation;

const DEFAULT_FRAMES_PER_ROM: u64 = 1_800;
const FRAME_WAIT_TIMEOUT: Duration = Duration::from_secs(180);
const NONZERO_EPSILON: f64 = 1.0e-9;
const FM_ROM_FIXTURES: &[(&str, &str)] = &[
    (
        "Kenseiden (Japan)",
        "7a9c056edfb1b973482430a41074b0b0a076ee4591d5df8f19007e7058cadc93",
    ),
    (
        "Phantasy Star (Japan) Rev 1",
        "55ba15a1ea5c7db883a7a4c850bea2b1024185605964916ddd04a645d7ddd2b6",
    ),
];

#[derive(Debug, Default)]
struct PcmCapture {
    blocks: u64,
    samples: u64,
    nonzero_samples: u64,
    peak: f64,
}

#[derive(Clone, Default)]
struct RecordingSink {
    capture: Arc<Mutex<PcmCapture>>,
}

impl PcmSink for RecordingSink {
    fn write_samples(&mut self, samples: Vec<f64>) -> Result<(), String> {
        let mut capture = self
            .capture
            .lock()
            .map_err(|_| "FM smoke PCM capture mutex was poisoned".to_owned())?;
        capture.blocks = capture.blocks.saturating_add(1);
        capture.samples = capture.samples.saturating_add(samples.len() as u64);
        capture.nonzero_samples = capture.nonzero_samples.saturating_add(
            samples
                .iter()
                .filter(|sample| sample.abs() > NONZERO_EPSILON)
                .count() as u64,
        );
        for sample in samples {
            capture.peak = capture.peak.max(sample.abs());
        }
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

#[derive(Debug, Default)]
struct FmSourceCapture {
    dual_source_blocks: u64,
    psg_samples: u64,
    fm_samples: u64,
    fm_nonzero_samples: u64,
    alignment_errors: u64,
    fm_peak: f64,
}

struct CapturingAudioSession {
    native: NativeAudioSession<RecordingSink>,
    sources: Arc<Mutex<FmSourceCapture>>,
}

impl CapturingAudioSession {
    fn record_sources(&self, sources: &[MixSource<'_>]) -> Result<(), String> {
        let mut capture = self
            .sources
            .lock()
            .map_err(|_| "FM source capture mutex was poisoned".to_owned())?;

        if sources.len() < 2 {
            return Ok(());
        }

        let psg = sources[0].samples;
        let fm = sources[1].samples;
        capture.dual_source_blocks = capture.dual_source_blocks.saturating_add(1);
        capture.psg_samples = capture.psg_samples.saturating_add(psg.len() as u64);
        capture.fm_samples = capture.fm_samples.saturating_add(fm.len() as u64);
        if psg.len() != fm.len() {
            capture.alignment_errors = capture.alignment_errors.saturating_add(1);
        }
        capture.fm_nonzero_samples = capture.fm_nonzero_samples.saturating_add(
            fm.iter()
                .filter(|sample| sample.abs() > NONZERO_EPSILON)
                .count() as u64,
        );
        for sample in fm {
            capture.fm_peak = capture.fm_peak.max(sample.abs());
        }
        Ok(())
    }
}

impl AudioSession for CapturingAudioSession {
    fn process_samples(&mut self, samples: &[f64]) -> Result<(), String> {
        self.native.process_samples(samples)
    }

    fn process_sources(&mut self, sources: &[MixSource<'_>]) -> Result<(), String> {
        self.record_sources(sources)?;
        self.native.process_sources(sources)
    }

    fn process_samples_discarding_output(&mut self, samples: &[f64]) -> Result<(), String> {
        self.native.process_samples_discarding_output(samples)
    }

    fn process_sources_discarding_output(
        &mut self,
        sources: &[MixSource<'_>],
    ) -> Result<(), String> {
        self.native.process_sources_discarding_output(sources)
    }

    fn suspend_and_flush(&mut self) -> Result<(), String> {
        self.native.suspend_and_flush()
    }

    fn resume(&mut self) -> Result<(), String> {
        self.native.resume()
    }

    fn reset_dsp(&mut self) {
        self.native.reset_dsp();
    }

    fn set_volume(&mut self, volume: u8) -> Result<(), String> {
        self.native.set_volume(volume)
    }

    fn set_analog_output_config(&mut self, config: AnalogOutputConfig) -> Result<(), String> {
        self.native.set_analog_output_config(config)
    }

    fn metrics(&self) -> AudioOutputMetrics {
        self.native.metrics()
    }

    fn close(&mut self) {
        self.native.close();
    }
}

#[derive(Debug, Default)]
struct FmPortWriteCapture {
    address_writes: u64,
    data_writes: u64,
    control_writes: u64,
    register_writes: Vec<(u8, u8)>,
    control_values: Vec<u8>,
    pending_address: Option<u8>,
    last_request_id: Option<u64>,
    dropped_events: u64,
}

impl FmPortWriteCapture {
    fn observe(&mut self, events: &[SmsTemporalTraceEvent], dropped_events: u64) {
        self.dropped_events = self.dropped_events.max(dropped_events);
        for event in events {
            let SmsTemporalTraceEvent::BusCompleted {
                request_id,
                operation,
                address,
                ..
            } = event
            else {
                continue;
            };
            if self
                .last_request_id
                .is_some_and(|last_request_id| *request_id <= last_request_id)
            {
                continue;
            }
            self.last_request_id = Some(*request_id);

            let Z80BusOperation::PortWrite { value } = operation else {
                continue;
            };
            match *address as u8 {
                0xF0 => {
                    self.address_writes = self.address_writes.saturating_add(1);
                    self.pending_address = Some(*value & 0x3F);
                }
                0xF1 => {
                    self.data_writes = self.data_writes.saturating_add(1);
                    if let Some(register) = self.pending_address {
                        if self.register_writes.len() < 64 {
                            self.register_writes.push((register, *value));
                        }
                    }
                }
                0xF2 => {
                    self.control_writes = self.control_writes.saturating_add(1);
                    if self.control_values.len() < 32 {
                        self.control_values.push(*value);
                    }
                }
                _ => {}
            }
        }
    }
}

const FM_DIAGNOSTIC_INSTRUCTIONS: usize = 4_096;
const FM_TRACE_BATCH_INSTRUCTIONS: usize = 32;

/// Run a bounded, audio-free startup probe and retain the actual YM2413 port writes.
/// The caller resets the worker afterward, so this diagnostic pass cannot change
/// the state used by the real-game audio acceptance run.
fn capture_startup_fm_port_writes(
    worker: &SmsTemporalWorker,
) -> Result<FmPortWriteCapture, String> {
    let mut capture = FmPortWriteCapture::default();
    for instruction in 0..FM_DIAGNOSTIC_INSTRUCTIONS {
        worker.step_instruction()?;
        if (instruction + 1) % FM_TRACE_BATCH_INSTRUCTIONS == 0 {
            let snapshot = worker.debug_snapshot()?;
            capture.observe(
                &snapshot.essential_trace_events,
                snapshot.essential_trace_dropped,
            );
        }
    }
    let snapshot = worker.debug_snapshot()?;
    capture.observe(
        &snapshot.essential_trace_events,
        snapshot.essential_trace_dropped,
    );
    Ok(capture)
}

fn is_sms_rom(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("sms") || extension.eq_ignore_ascii_case("bin")
        })
}

fn collect_roms(directory: &Path) -> Vec<PathBuf> {
    let mut paths = fs::read_dir(directory)
        .unwrap_or_else(|error| {
            panic!(
                "cannot read FM ROM directory {}: {error}",
                directory.display()
            )
        })
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("cannot read FM ROM directory entry: {error}"))
                .path()
        })
        .filter(|path| path.is_file() && is_sms_rom(path))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn selected_vdp_revision() -> SmsVdpRevision {
    match std::env::var("SEGA_SMS_FM_TEST_SYSTEM")
        .unwrap_or_else(|_| "sms1".to_owned())
        .as_str()
    {
        "sms1" => SmsVdpRevision::Sega3155124,
        "sms2" => SmsVdpRevision::Sega3155246,
        other => panic!("unsupported SEGA_SMS_FM_TEST_SYSTEM '{other}'; expected sms1 or sms2"),
    }
}

fn load_known_fm_roms() -> Vec<(PathBuf, &'static str, &'static str, Vec<u8>)> {
    let directory = std::env::var_os("SEGA_SMS_FM_TEST_ROM_DIR")
        .map(PathBuf::from)
        .expect("set SEGA_SMS_FM_TEST_ROM_DIR to a directory containing the two verified FM ROMs");
    let rom_paths = collect_roms(&directory);
    assert!(
        !rom_paths.is_empty(),
        "no .sms/.bin FM ROMs found in {}",
        directory.display()
    );

    let mut found_fixtures = std::collections::BTreeSet::new();
    let mut fixtures = Vec::new();
    for path in rom_paths {
        let rom = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read ROM {}: {error}", path.display()));
        assert!(!rom.is_empty(), "ROM {} is empty", path.display());
        let sha256 = CartridgeIdentity::from_bytes(&rom).sha256_hex();
        let Some((fixture_name, expected_sha256)) = FM_ROM_FIXTURES
            .iter()
            .find(|(_, expected)| *expected == sha256.as_str())
        else {
            continue;
        };
        found_fixtures.insert((*expected_sha256).to_owned());
        fixtures.push((path, *fixture_name, *expected_sha256, rom));
    }

    for (fixture_name, expected_sha256) in FM_ROM_FIXTURES {
        assert!(
            found_fixtures.contains(*expected_sha256),
            "missing required FM fixture {fixture_name} with SHA-256 {expected_sha256}"
        );
    }
    fixtures
}

#[test]
#[ignore = "requires legally obtained Japanese SMS FM game ROMs; see module documentation"]
fn real_japanese_sms_roms_emit_non_silent_fm_through_the_native_mixer() {
    let region = std::env::var("SEGA_SMS_FM_TEST_REGION").unwrap_or_else(|_| "japan".to_owned());
    let vdp_revision = selected_vdp_revision();
    let frames_per_rom = std::env::var("SEGA_SMS_FM_TEST_FRAMES")
        .map(|value| {
            value
                .parse::<u64>()
                .expect("SEGA_SMS_FM_TEST_FRAMES must be an integer")
        })
        .unwrap_or(DEFAULT_FRAMES_PER_ROM);
    assert!(
        frames_per_rom > 0,
        "SEGA_SMS_FM_TEST_FRAMES must be positive"
    );

    for (path, fixture_name, sha256, rom) in load_known_fm_roms() {
        let worker = SmsTemporalWorker::new_with_japanese_fm_hardware_profile_and_output_policy(
            rom,
            &region,
            vdp_revision,
            None,
            None,
            SmsFmOutputPolicy::Enabled,
        )
        .unwrap_or_else(|error| {
            panic!(
                "{}: cannot create Japanese FM machine: {error}",
                path.display()
            )
        });

        let fm_bus_writes = capture_startup_fm_port_writes(&worker).unwrap_or_else(|error| {
            panic!(
                "{}: failed to capture startup YM2413 port writes: {error}",
                path.display()
            )
        });
        worker.reset().unwrap_or_else(|error| {
            panic!(
                "{}: cannot reset after FM write probe: {error}",
                path.display()
            )
        });

        let source_capture = Arc::new(Mutex::new(FmSourceCapture::default()));
        let pcm_capture = Arc::new(Mutex::new(PcmCapture::default()));
        let factory_sources = Arc::clone(&source_capture);
        let factory_pcm = Arc::clone(&pcm_capture);

        worker
            .configure_audio_output(move || {
                let sink = RecordingSink {
                    capture: factory_pcm,
                };
                let native = NativeAudioSession::new(48_000, 48_000, 100, sink)?;
                Ok(Box::new(CapturingAudioSession {
                    native,
                    sources: factory_sources,
                }) as Box<dyn AudioSession>)
            })
            .unwrap_or_else(|error| {
                panic!("{}: cannot configure native mixer: {error}", path.display())
            });

        worker
            .start()
            .unwrap_or_else(|error| panic!("{}: cannot start FM runtime: {error}", path.display()));

        let deadline = Instant::now() + FRAME_WAIT_TIMEOUT;
        while worker.frame_number() < frames_per_rom {
            if let Some(error) = worker.runtime_error() {
                let _ = worker.pause();
                panic!("{}: SMS runtime failed: {error}", path.display());
            }
            if let Some(error) = worker.audio_output_error() {
                let _ = worker.pause();
                panic!("{}: audio session failed: {error}", path.display());
            }
            assert!(
                Instant::now() < deadline,
                "{}: timed out after {} frames; reached {}",
                path.display(),
                frames_per_rom,
                worker.frame_number()
            );
            thread::sleep(Duration::from_millis(10));
        }

        worker
            .pause()
            .unwrap_or_else(|error| panic!("{}: cannot pause FM runtime: {error}", path.display()));
        worker.close_audio_output().unwrap_or_else(|error| {
            panic!("{}: cannot close native mixer: {error}", path.display())
        });

        let sources = source_capture
            .lock()
            .unwrap_or_else(|_| panic!("{}: FM source capture mutex was poisoned", path.display()));
        let pcm = pcm_capture
            .lock()
            .unwrap_or_else(|_| panic!("{}: PCM capture mutex was poisoned", path.display()));

        assert!(
            sources.dual_source_blocks > 0,
            "{}: runtime never submitted aligned PSG + FM source blocks",
            path.display()
        );
        assert_eq!(
            sources.alignment_errors,
            0,
            "{}: PSG and FM source blocks were not frame-aligned",
            path.display()
        );
        assert!(
            sources.fm_samples > 0,
            "{}: no YM2413 samples reached the native mixer",
            path.display()
        );
        assert!(
            sources.fm_nonzero_samples > 0 && sources.fm_peak > NONZERO_EPSILON,
            "{}: YM2413 source remained silent; startup FM bus writes: {:?}",
            path.display(),
            fm_bus_writes
        );
        assert!(
            pcm.blocks > 0 && pcm.nonzero_samples > 0 && pcm.peak > NONZERO_EPSILON,
            "{}: native mixer/resampler did not submit non-silent PCM",
            path.display()
        );

        println!(
            "SMS_FM_IN_GAME title={} rom={} sha256={} region={} vdp={:?} frames={} fm_bus_writes={:?} dual_source_blocks={} psg_samples={} fm_samples={} fm_nonzero_samples={} fm_peak={:.8} pcm_blocks={} pcm_samples={} pcm_nonzero_samples={} pcm_peak={:.8}",
            fixture_name,
            path.display(),
            sha256,
            region,
            vdp_revision,
            worker.frame_number(),
            fm_bus_writes,
            sources.dual_source_blocks,
            sources.psg_samples,
            sources.fm_samples,
            sources.fm_nonzero_samples,
            sources.fm_peak,
            pcm.blocks,
            pcm.samples,
            pcm.nonzero_samples,
            pcm.peak
        );
    }
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "requires the verified FM ROMs and a working physical CPAL output device"]
fn real_japanese_sms_roms_play_fm_through_cpal_and_report_underruns() {
    let region = std::env::var("SEGA_SMS_FM_TEST_REGION").unwrap_or_else(|_| "japan".to_owned());
    let vdp_revision = selected_vdp_revision();
    let frames_per_rom = std::env::var("SEGA_SMS_FM_TEST_FRAMES")
        .map(|value| {
            value
                .parse::<u64>()
                .expect("SEGA_SMS_FM_TEST_FRAMES must be an integer")
        })
        .unwrap_or(DEFAULT_FRAMES_PER_ROM);
    assert!(
        frames_per_rom > 0,
        "SEGA_SMS_FM_TEST_FRAMES must be positive"
    );

    for (path, fixture_name, sha256, rom) in load_known_fm_roms() {
        let worker = SmsTemporalWorker::new_with_japanese_fm_hardware_profile_and_output_policy(
            rom,
            &region,
            vdp_revision,
            None,
            None,
            SmsFmOutputPolicy::Enabled,
        )
        .unwrap_or_else(|error| {
            panic!(
                "{}: cannot create Japanese FM machine: {error}",
                path.display()
            )
        });
        worker.set_diagnostics_enabled(true);
        worker
            .configure_audio_output(|| {
                let output = NativeCpalOutput::new(512, 16)?;
                let session = output.into_audio_session(100)?;
                Ok(Box::new(session) as Box<dyn AudioSession>)
            })
            .unwrap_or_else(|error| panic!("{}: cannot open CPAL output: {error}", path.display()));
        worker
            .start()
            .unwrap_or_else(|error| panic!("{}: cannot start FM runtime: {error}", path.display()));

        let deadline = Instant::now() + FRAME_WAIT_TIMEOUT;
        while worker.frame_number() < frames_per_rom {
            if let Some(error) = worker.runtime_error() {
                let _ = worker.pause();
                panic!("{}: SMS runtime failed: {error}", path.display());
            }
            if let Some(error) = worker.audio_output_error() {
                let _ = worker.pause();
                panic!("{}: CPAL output failed: {error}", path.display());
            }
            assert!(
                Instant::now() < deadline,
                "{}: timed out after {} frames; reached {}",
                path.display(),
                frames_per_rom,
                worker.frame_number()
            );
            thread::sleep(Duration::from_millis(10));
        }

        worker
            .pause()
            .unwrap_or_else(|error| panic!("{}: cannot pause FM runtime: {error}", path.display()));
        let metrics = worker.audio_output_metrics();
        worker.close_audio_output().unwrap_or_else(|error| {
            panic!("{}: cannot close CPAL output: {error}", path.display())
        });

        assert!(
            metrics.sample_rate_hz > 0,
            "{}: CPAL did not report a sample rate",
            path.display()
        );
        assert!(
            metrics.accepted_frames > 0,
            "{}: no PCM frames reached the CPAL queue",
            path.display()
        );
        assert_eq!(
            metrics.dropped_frames,
            0,
            "{}: CPAL queue dropped PCM frames",
            path.display()
        );
        assert_eq!(
            metrics.dropped_blocks,
            0,
            "{}: CPAL queue rejected PCM blocks",
            path.display()
        );
        let max_underrun_frames = (metrics.accepted_frames / 50).max(2_048);
        assert!(
            metrics.underrun_frames <= max_underrun_frames,
            "{}: excessive CPAL underruns: {} frames out of {} accepted (limit {})",
            path.display(),
            metrics.underrun_frames,
            metrics.accepted_frames,
            max_underrun_frames
        );

        println!(
            "SMS_FM_CPAL title={} rom={} sha256={} region={} vdp={:?} frames={} sample_rate_hz={} accepted_frames={} consumed_frames={} underrun_events={} underrun_frames={} dropped_blocks={} dropped_frames={} queue_high_water_frames={} queue_capacity_frames={}",
            fixture_name,
            path.display(),
            sha256,
            region,
            vdp_revision,
            worker.frame_number(),
            metrics.sample_rate_hz,
            metrics.accepted_frames,
            metrics.consumed_frames,
            metrics.underrun_events,
            metrics.underrun_frames,
            metrics.dropped_blocks,
            metrics.dropped_frames,
            metrics.queue_high_water_frames,
            metrics.queue_capacity_frames
        );
    }
}
