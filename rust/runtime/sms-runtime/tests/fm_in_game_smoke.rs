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
//!   cargo test --release -p sega-sms-runtime --test fm_in_game_smoke -- --ignored --nocapture
//!
//! ROM files are read in place and are never copied into the repository.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use sega_audio_runtime::{
    AnalogOutputConfig, AudioOutputMetrics, MixSource, NativeAudioSession, PcmSink,
};
use sega_sms_runtime::{AudioSession, SmsFmOutputPolicy, SmsTemporalWorker, SmsVdpRevision};

const DEFAULT_FRAMES_PER_ROM: u64 = 1_800;
const FRAME_WAIT_TIMEOUT: Duration = Duration::from_secs(180);
const NONZERO_EPSILON: f64 = 1.0e-9;

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
    single_source_blocks: u64,
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
            capture.single_source_blocks = capture.single_source_blocks.saturating_add(1);
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
            panic!("cannot read FM ROM directory {}: {error}", directory.display())
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

#[test]
#[ignore = "requires legally obtained Japanese SMS FM game ROMs; see module documentation"]
fn real_japanese_sms_roms_emit_non_silent_fm_through_the_native_mixer() {
    let directory = std::env::var_os("SEGA_SMS_FM_TEST_ROM_DIR")
        .map(PathBuf::from)
        .expect("set SEGA_SMS_FM_TEST_ROM_DIR to a directory containing Japanese SMS FM ROMs");
    let region =
        std::env::var("SEGA_SMS_FM_TEST_REGION").unwrap_or_else(|_| "japan".to_owned());
    let vdp_revision = selected_vdp_revision();
    let frames_per_rom = std::env::var("SEGA_SMS_FM_TEST_FRAMES")
        .map(|value| {
            value
                .parse::<u64>()
                .expect("SEGA_SMS_FM_TEST_FRAMES must be an integer")
        })
        .unwrap_or(DEFAULT_FRAMES_PER_ROM);
    assert!(frames_per_rom > 0, "SEGA_SMS_FM_TEST_FRAMES must be positive");

    let rom_paths = collect_roms(&directory);
    assert!(
        !rom_paths.is_empty(),
        "no .sms/.bin FM ROMs found in {}",
        directory.display()
    );

    for path in rom_paths {
        let rom = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read ROM {}: {error}", path.display()));
        assert!(!rom.is_empty(), "ROM {} is empty", path.display());

        let worker = SmsTemporalWorker::new_with_japanese_fm_hardware_profile_and_output_policy(
            rom,
            &region,
            vdp_revision,
            None,
            None,
            SmsFmOutputPolicy::Enabled,
        )
        .unwrap_or_else(|error| panic!("{}: cannot create Japanese FM machine: {error}", path.display()));

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
            .unwrap_or_else(|error| panic!("{}: cannot configure native mixer: {error}", path.display()));

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
        worker
            .close_audio_output()
            .unwrap_or_else(|error| panic!("{}: cannot close native mixer: {error}", path.display()));

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
            sources.alignment_errors, 0,
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
            "{}: YM2413 source remained silent; verify the ROM's FM initialization and F2 control writes",
            path.display()
        );
        assert!(
            pcm.blocks > 0 && pcm.nonzero_samples > 0 && pcm.peak > NONZERO_EPSILON,
            "{}: native mixer/resampler did not submit non-silent PCM",
            path.display()
        );

        println!(
            "SMS_FM_IN_GAME rom={} region={} vdp={:?} frames={} dual_source_blocks={} psg_samples={} fm_samples={} fm_nonzero_samples={} fm_peak={:.8} pcm_blocks={} pcm_samples={} pcm_nonzero_samples={} pcm_peak={:.8}",
            path.display(),
            region,
            vdp_revision,
            worker.frame_number(),
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
