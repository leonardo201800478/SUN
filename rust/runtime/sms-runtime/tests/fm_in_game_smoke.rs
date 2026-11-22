//! Opt-in in-game FM acceptance campaign for legally obtained Japanese SMS ROMs.
//!
//! This exercises the real ROM -> Z80/Scheduler -> YM2413 -> FM/PSG alignment ->
//! NativeAudioSession mixer/resampler -> PCM sink path. The capture sink is
//! deterministic and headless; no CPAL device is required for this software-path
//! diagnostic test, not an audio-quality acceptance test. Physical CPAL playback
//! remains a separate manual check. Nonzero PCM alone does not establish fidelity.
//!
//! PowerShell:
//!   $env:SEGA_SMS_FM_TEST_ROM_DIR = "D:\ROMs\SMS-FM"
//!   $env:SEGA_SMS_FM_TEST_REGION = "japan"
//!   $env:SEGA_SMS_FM_TEST_SYSTEM = "sms1" # sms1|sms2
//!   $env:SEGA_SMS_FM_TEST_FRAMES = "1800" # optional; default 1800
//!   $env:SEGA_SMS_FM_TEST_WAV_DIR = "D:\\SUN-FM-Captures" # optional; writes FM, PSG and mixed float WAVs
//!   cargo test --release -p sega-sms-runtime --test fm_in_game_smoke real_japanese_sms_roms_emit_non_silent_fm_through_the_native_mixer -- --ignored --nocapture
//!   cargo test --release -p sega-sms-runtime --test fm_in_game_smoke real_japanese_sms_roms_play_fm_through_cpal_and_report_underruns -- --ignored --nocapture
//!
//! The first test uses a deterministic headless sink. The second opens the default
//! CPAL output device and plays the real game audio while reporting underruns.
//!
//! ROM files are read in place and are never copied into the repository.

use std::fs::{self, File};
use std::io::Write;
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
use sega_sms_runtime::{AudioSession, SmsFmOutputPolicy, SmsTemporalWorker, SmsVdpRevision};

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
    samples_f64: Vec<f64>,
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
        capture.samples_f64.extend_from_slice(&samples);
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
    single_source_blocks: u64,
    single_source_samples: u64,
    fm_peak: f64,
    psg_samples_f64: Vec<f64>,
    fm_samples_f64: Vec<f64>,
    single_source_samples_f64: Vec<f64>,
}

struct CapturingAudioSession {
    native: NativeAudioSession<RecordingSink>,
    sources: Arc<Mutex<FmSourceCapture>>,
}

impl CapturingAudioSession {
    fn record_single_source(&self, samples: &[f64]) -> Result<(), String> {
        let mut capture = self
            .sources
            .lock()
            .map_err(|_| "FM source capture mutex was poisoned".to_owned())?;
        capture.single_source_blocks = capture.single_source_blocks.saturating_add(1);
        capture.single_source_samples =
            capture.single_source_samples.saturating_add(samples.len() as u64);
        capture.single_source_samples_f64.extend_from_slice(samples);
        Ok(())
    }

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
        capture.psg_samples_f64.extend_from_slice(psg);
        capture.fm_samples_f64.extend(fm.iter().copied());
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
        self.record_single_source(samples)?;
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

#[derive(Debug, Clone, Copy, Default)]
struct SignalSummary {
    samples: u64,
    nonzero: u64,
    non_finite: u64,
    clipped_or_over_one: u64,
    peak: f64,
    rms: f64,
    mean: f64,
}

fn summarize_signal(samples: &[f64]) -> SignalSummary {
    let mut summary = SignalSummary {
        samples: samples.len() as u64,
        ..SignalSummary::default()
    };
    let mut sum = 0.0_f64;
    let mut sum_squares = 0.0_f64;
    for &sample in samples {
        if !sample.is_finite() {
            summary.non_finite = summary.non_finite.saturating_add(1);
            continue;
        }
        if sample.abs() > NONZERO_EPSILON {
            summary.nonzero = summary.nonzero.saturating_add(1);
        }
        if sample.abs() >= 1.0 {
            summary.clipped_or_over_one = summary.clipped_or_over_one.saturating_add(1);
        }
        summary.peak = summary.peak.max(sample.abs());
        sum += sample;
        sum_squares += sample * sample;
    }
    let finite = summary.samples.saturating_sub(summary.non_finite);
    if finite > 0 {
        summary.mean = sum / finite as f64;
        summary.rms = (sum_squares / finite as f64).sqrt();
    }
    summary
}

/// Write mono IEEE-float WAV without normalization or clipping, preserving the
/// actual digital amplitude delivered by each point in the production pipeline.
fn write_wav_f32(path: &Path, samples: &[f64], sample_rate: u32) -> Result<(), String> {
    let data_len = u32::try_from(samples.len().saturating_mul(4))
        .map_err(|_| format!("WAV is too large: {}", path.display()))?;
    let riff_len = 36_u32
        .checked_add(data_len)
        .ok_or_else(|| format!("WAV is too large: {}", path.display()))?;
    let byte_rate = sample_rate
        .checked_mul(4)
        .ok_or_else(|| format!("invalid WAV sample rate: {sample_rate}"))?;
    let mut file = File::create(path).map_err(|error| format!("{}: {error}", path.display()))?;
    file.write_all(b"RIFF")
        .and_then(|_| file.write_all(&riff_len.to_le_bytes()))
        .and_then(|_| file.write_all(b"WAVEfmt "))
        .and_then(|_| file.write_all(&16_u32.to_le_bytes()))
        .and_then(|_| file.write_all(&3_u16.to_le_bytes())) // IEEE float
        .and_then(|_| file.write_all(&1_u16.to_le_bytes())) // mono
        .and_then(|_| file.write_all(&sample_rate.to_le_bytes()))
        .and_then(|_| file.write_all(&byte_rate.to_le_bytes()))
        .and_then(|_| file.write_all(&4_u16.to_le_bytes())) // block align
        .and_then(|_| file.write_all(&32_u16.to_le_bytes()))
        .and_then(|_| file.write_all(b"data"))
        .and_then(|_| file.write_all(&data_len.to_le_bytes()))
        .map_err(|error| format!("{}: {error}", path.display()))?;
    for &sample in samples {
        let value = if sample.is_finite() { sample as f32 } else { 0.0 };
        file.write_all(&value.to_le_bytes())
            .map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(())
}

fn write_game_audio_captures(
    fixture_name: &str,
    fm: &[f64],
    psg: &[f64],
    single_source: &[f64],
    mixed: &[f64],
) -> Result<(), String> {
    let Some(directory) = std::env::var_os("SEGA_SMS_FM_TEST_WAV_DIR") else {
        return Ok(());
    };
    let directory = PathBuf::from(directory);
    fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let stem = fixture_name
        .to_ascii_lowercase()
        .replace(' ', "-")
        .replace('(', "")
        .replace(')', "")
        .replace("--", "-");
    for (suffix, samples) in [
        ("fm-source", fm),
        ("psg-source", psg),
        ("single-source", single_source),
        ("mixed-output", mixed),
    ] {
        let path = directory.join(format!("{stem}-{suffix}.wav"));
        write_wav_f32(&path, samples, 48_000)?;
        println!(
            "SMS_FM_WAV path={} signal={:?}",
            path.display(),
            summarize_signal(samples)
        );
    }
    Ok(())
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
        let fm_bus_writes = worker
            .debug_snapshot()
            .unwrap_or_else(|error| {
                panic!(
                    "{}: cannot snapshot FM bus counters: {error}",
                    path.display()
                )
            })
            .fm_port_writes;
        worker.close_audio_output().unwrap_or_else(|error| {
            panic!("{}: cannot close native mixer: {error}", path.display())
        });

        let sources = source_capture
            .lock()
            .unwrap_or_else(|_| panic!("{}: FM source capture mutex was poisoned", path.display()));
        let pcm = pcm_capture
            .lock()
            .unwrap_or_else(|_| panic!("{}: PCM capture mutex was poisoned", path.display()));

        write_game_audio_captures(
            fixture_name,
            &sources.fm_samples_f64,
            &sources.psg_samples_f64,
            &sources.single_source_samples_f64,
            &pcm.samples_f64,
        )
        .unwrap_or_else(|error| {
            panic!(
                "{}: cannot write diagnostic WAV: {error}",
                path.display()
            )
        });

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
            "{}: YM2413 source remained silent; committed FM bus writes: {:?}",
            path.display(),
            fm_bus_writes
        );
        assert!(
            pcm.blocks > 0 && pcm.nonzero_samples > 0 && pcm.peak > NONZERO_EPSILON,
            "{}: native mixer/resampler did not submit non-silent PCM",
            path.display()
        );

        println!(
            "SMS_FM_IN_GAME title={} rom={} sha256={} region={} vdp={:?} frames={} fm_port_writes={:?} dual_source_blocks={} single_source_blocks={} single_source_samples={} psg_samples={} fm_samples={} fm_nonzero_samples={} fm_peak={:.8} pcm_blocks={} pcm_samples={} pcm_nonzero_samples={} pcm_peak={:.8} fm_summary={:?} psg_summary={:?} single_source_summary={:?} mixed_summary={:?}",
            fixture_name,
            path.display(),
            sha256,
            region,
            vdp_revision,
            worker.frame_number(),
            fm_bus_writes,
            sources.dual_source_blocks,
            sources.single_source_blocks,
            sources.single_source_samples,
            sources.psg_samples,
            sources.fm_samples,
            sources.fm_nonzero_samples,
            sources.fm_peak,
            pcm.blocks,
            pcm.samples,
            pcm.nonzero_samples,
            pcm.peak,
            summarize_signal(&sources.fm_samples_f64),
            summarize_signal(&sources.psg_samples_f64),
            summarize_signal(&sources.single_source_samples_f64),
            summarize_signal(&pcm.samples_f64),
        );
    }
}

#[test]
fn diagnostic_wav_writer_preserves_ieee_float_samples_without_normalization() {
    let path = std::env::temp_dir().join(format!(
        "sun-ym2413-fm-diagnostic-{}.wav",
        std::process::id()
    ));
    let samples = [0.25_f64, -0.5_f64];
    write_wav_f32(&path, &samples, 48_000).unwrap();

    let bytes = fs::read(&path).unwrap();
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(u16::from_le_bytes([bytes[20], bytes[21]]), 3); // IEEE float
    assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 1); // mono
    assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 48_000);
    assert_eq!(u16::from_le_bytes([bytes[34], bytes[35]]), 32);
    assert_eq!(f32::from_le_bytes(bytes[44..48].try_into().unwrap()), 0.25);
    assert_eq!(f32::from_le_bytes(bytes[48..52].try_into().unwrap()), -0.5);
    fs::remove_file(path).unwrap();
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
