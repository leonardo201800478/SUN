//! Opt-in in-game FM signal/quality diagnostic campaign for verified SMS YM2413 ROMs.
//!
//! The fixture catalog includes Japanese and international game dumps, but all
//! default profiles model Japanese FM-capable hardware. Each fixture carries its
//! own console region and VDP revision; environment variables can override those
//! defaults for controlled compatibility experiments.
//!
//! This exercises the real ROM -> Z80/Scheduler -> YM2413 -> FM/PSG alignment ->
//! NativeAudioSession mixer/resampler -> PCM sink path. The capture sink is
//! deterministic and headless; no CPAL device is required for this software-path
//! diagnostic test, not an audio-quality acceptance test. Physical CPAL playback
//! remains a separate manual check. Nonzero PCM alone does not establish fidelity.
//!
//! PowerShell:
//!   $env:SEGA_SMS_FM_TEST_ROM_DIR = "D:\ROMs\SMS-FM"
//!   $env:SEGA_SMS_FM_TEST_REGION = "japan" # optional override; default is fixture-specific
//!   $env:SEGA_SMS_FM_TEST_SYSTEM = "sms1" # optional override: sms1|sms2
//!   $env:SEGA_SMS_FM_TEST_FRAMES = "1800" # optional; default 1800
//!   $env:SEGA_SMS_FM_TEST_WAV_DIR = "D:\SUN-FM-Captures" # optional; writes FM, PSG, single-source and mixed float WAVs
//!   cargo test --release -p sega-sms-runtime --test fm_in_game_smoke real_japanese_sms_roms_emit_non_silent_fm_through_the_native_mixer -- --ignored --nocapture
//!   cargo test --release -p sega-sms-runtime --test fm_in_game_smoke real_japanese_sms_roms_play_fm_through_cpal_and_report_underruns -- --ignored --nocapture
//!
//! The first test uses a deterministic headless sink. The second opens the default
//! CPAL output device and plays the real game audio while reporting underruns.
//!
//! ROM files are read in place and are never copied into the repository.

use std::fs::{self, File};
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
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

#[derive(Debug, Clone, Copy)]
struct FmRomFixture {
    title: &'static str,
    sha256: &'static str,
    region: &'static str,
    system: &'static str,
    required: bool,
}

// Required reference fixtures keep the original acceptance baseline intact.
// Additional entries are optional: they run when their exact dump is present,
// but their absence never fails the campaign.
const FM_ROM_FIXTURES: &[FmRomFixture] = &[
    FmRomFixture {
        title: "Kenseiden (Japan)",
        sha256: "7a9c056edfb1b973482430a41074b0b0a076ee4591d5df8f19007e7058cadc93",
        region: "japan",
        system: "sms1",
        required: true,
    },
    FmRomFixture {
        title: "Phantasy Star (Japan) Rev 1",
        sha256: "55ba15a1ea5c7db883a7a4c850bea2b1024185605964916ddd04a645d7ddd2b6",
        region: "japan",
        system: "sms1",
        required: true,
    },
    FmRomFixture {
        title: "After Burner (World)",
        sha256: "90740cfeb430b95b7460d67ed42260bf9f50a89b24e0ad3724d645c47ef783eb",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Alien Syndrome (Japan)",
        sha256: "b87ef7a23c392ab4b046854ced6a48d1a7b2244f643214b759e305db793f480a",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Aztec Adventure (World)",
        sha256: "b3fd2d4d5bd30e7419080d1e4368cf4a47f9f892df188eb165435354f9e59531",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Bomber Raid (World)",
        sha256: "ea90ca2ba9a5db82f0ab920d70607246f06b45fa9790b600b5c51051b66209d5",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Captain Silver (Japan/Europe/Brazil)",
        sha256: "925d9933b7565e0010111983cc68513a911269a88b602c27ed3d8b9486ccd647",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Cyborg Hunter (USA/Europe/Brazil)",
        sha256: "62caa88da76d2ee5cbd6334254c46d8c6e150a0f56d2cbc1aea8bcc1adbc7d78",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Double Dragon (World)",
        sha256: "26290f94a597ad015da5b74e8d3649aa50b033c280c81fbfa460f947376b776a",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Golvellius - Maou Golvellius (Japan)",
        sha256: "f64b9b546c4387939dc80208482b0cc4a5a6e7685f0291bfe715c170508bdedf",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Lord of the Sword (USA/Europe/Brazil)",
        sha256: "6a9d4a337894059eec44c9be1c0c83f2836cdc7c2f92f5fd35377e4077458e57",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Penguin Land - Doki Doki (Japan)",
        sha256: "c87ec8b700e53c5944e04b68a195e844276ac3b4996bac72ae73cea4c615e006",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Power Strike (USA/Europe/Brazil)",
        sha256: "247fd3d9d16bbf293086567d55ad4d2dd4dff5fb839a3a2c85f59ad1124e73b1",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "R-Type (World)",
        sha256: "8b9b80a16bc73457e93dd855d7398876bdb9b634387472ce1c0058906fc030d1",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Shinobi (Japan/Brazil)",
        sha256: "d66e46388b58d202c32fe652e771ba228195005b26456c75823fdff3a50efa35",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "SpellCaster (USA/Europe/Brazil)",
        sha256: "0d724c18d202063a9ec73689c0d42301d935a23474b0a703478afdb4fd594d21",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Thunder Blade (Japan)",
        sha256: "8ace31d7eae687964c59642e4ccc5adf61811e91ad81d69e34c38fdcf1e8180a",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Wonder Boy in Monster Land (USA/Europe)",
        sha256: "425f24e06d0cacbaec17963bd145ee4e168ce89d972c7fd76e3cc36366d06c80",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Zillion II (World)",
        sha256: "de988cb6535081ff2d3bb7446ae6eb2689ea0b9aefeb13acebe1f6e01e572a45",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Ys (Japan)",
        sha256: "816ddb63d3aa30e7c4fc0efb706ff5e66f4b6f17cf331d5fc97f6681d1905a61",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "OutRun (World)",
        sha256: "069c130135d8b61393d89f26b233509d6f9de76b0fbac53087e698a7fda0ba88",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "OutRun 3-D (Europe/Brazil)",
        sha256: "2f7da259301a0b878229f1ac04d8a0467f7f3da7f34000053260b0cf1975e8a6",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Scramble Spirits (Europe/Brazil)",
        sha256: "1a54d18344f5486babe81e246fd054ff7cfa9ef6db3b5f1bae739bd4803928ec",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Time Soldiers (USA/Europe/Brazil)",
        sha256: "f5181d3908c91669d2832437b1deb3836e12a3519597b77dcd7a8eb8d97c6182",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Vigilante (USA/Europe/Brazil)",
        sha256: "fab0aa155212ff4183d85d28b2a8ede3acf6b178fdd84fd2119b21016af62466",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Rampage (USA/Europe/Brazil)",
        sha256: "a286f5d6268236116da5bf94d771020d7c2f5b278ce777e1cb60d02a00d8010f",
        region: "japan",
        system: "sms1",
        required: false,
    },
    FmRomFixture {
        title: "Rastan (USA/Europe/Brazil)",
        sha256: "7e310ada1a09ace30e9dd8f5738f095c0ec7821edc225221cbba130fb8cbadb3",
        region: "japan",
        system: "sms1",
        required: false,
    },
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
        capture.single_source_samples = capture
            .single_source_samples
            .saturating_add(samples.len() as u64);
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
        let value = if sample.is_finite() {
            sample as f32
        } else {
            0.0
        };
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
        .replace(['(', ')'], "")
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

fn panic_payload_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        return (*message).to_owned();
    }
    "non-string panic payload".to_owned()
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

fn selected_vdp_revision(system: &str) -> SmsVdpRevision {
    match system {
        "sms1" => SmsVdpRevision::Sega3155124,
        "sms2" => SmsVdpRevision::Sega3155246,
        other => panic!("unsupported SMS FM fixture system '{other}'; expected sms1 or sms2"),
    }
}

fn load_known_fm_roms() -> Vec<(PathBuf, &'static FmRomFixture, Vec<u8>)> {
    let directory = std::env::var_os("SEGA_SMS_FM_TEST_ROM_DIR")
        .map(PathBuf::from)
        .expect(
            "set SEGA_SMS_FM_TEST_ROM_DIR to a directory containing the required FM reference ROMs",
        );
    let rom_paths = collect_roms(&directory);
    assert!(
        !rom_paths.is_empty(),
        "no .sms/.bin FM ROMs found in {}",
        directory.display()
    );

    let mut found_fixtures = std::collections::BTreeSet::new();
    let mut seen_hashes = std::collections::BTreeSet::new();
    let mut fixtures = Vec::new();
    for path in rom_paths {
        let rom = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read ROM {}: {error}", path.display()));
        assert!(!rom.is_empty(), "ROM {} is empty", path.display());
        let sha256 = CartridgeIdentity::from_bytes(&rom).sha256_hex();
        let Some(fixture) = FM_ROM_FIXTURES
            .iter()
            .find(|fixture| fixture.sha256.eq_ignore_ascii_case(&sha256))
        else {
            continue;
        };
        if !seen_hashes.insert(fixture.sha256) {
            println!(
                "SMS_FM_DUPLICATE_ROM title={} rom={} sha256={} action=skipped",
                fixture.title,
                path.display(),
                fixture.sha256
            );
            continue;
        }
        found_fixtures.insert(fixture.sha256);
        fixtures.push((path, fixture, rom));
    }

    for fixture in FM_ROM_FIXTURES {
        if found_fixtures.contains(fixture.sha256) {
            continue;
        }
        if fixture.required {
            panic!(
                "missing required FM fixture {} with SHA-256 {}",
                fixture.title, fixture.sha256
            );
        }
        println!(
            "SMS_FM_OPTIONAL_FIXTURE_NOT_FOUND title={} sha256={}",
            fixture.title, fixture.sha256
        );
    }
    fixtures
}

#[test]
fn fm_fixture_catalog_has_unique_sha256_and_two_required_references() {
    let mut hashes = std::collections::BTreeSet::new();
    let mut required_count = 0;

    for fixture in FM_ROM_FIXTURES {
        assert_eq!(
            fixture.sha256.len(),
            64,
            "invalid SHA-256 length for {}",
            fixture.title
        );
        assert!(
            fixture.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "non-hexadecimal SHA-256 for {}",
            fixture.title
        );
        assert!(
            hashes.insert(fixture.sha256),
            "duplicate SHA-256 fixture for {}",
            fixture.title
        );
        if fixture.required {
            required_count += 1;
        }
    }

    assert_eq!(
        required_count, 2,
        "the original Kenseiden and Phantasy Star reference fixtures must remain required"
    );
}

#[test]
#[ignore = "requires legally obtained required reference ROMs; optional catalog ROMs are detected by SHA-256"]
fn real_japanese_sms_roms_emit_non_silent_fm_through_the_native_mixer() {
    let region_override = std::env::var("SEGA_SMS_FM_TEST_REGION").ok();
    let system_override = std::env::var("SEGA_SMS_FM_TEST_SYSTEM").ok();
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

    let mut failures = Vec::new();
    for (path, fixture, rom) in load_known_fm_roms() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let region = region_override.as_deref().unwrap_or(fixture.region);
            let system = system_override.as_deref().unwrap_or(fixture.system);
            let vdp_revision = selected_vdp_revision(system);
            let worker =
                SmsTemporalWorker::new_with_japanese_fm_hardware_profile_and_output_policy(
                    rom,
                    region,
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

            worker.start().unwrap_or_else(|error| {
                panic!("{}: cannot start FM runtime: {error}", path.display())
            });

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

            worker.pause().unwrap_or_else(|error| {
                panic!("{}: cannot pause FM runtime: {error}", path.display())
            });
            let performance = worker.performance_metrics();
            let frame_pacing = worker.frame_pacing_metrics();
            let total_instructions = worker.total_instructions();
            let machine_time_ticks = worker.machine_time_ticks();
            let machine_clock_hz = worker.machine_clock_hz();
            let debug_snapshot = worker.debug_snapshot().unwrap_or_else(|error| {
                panic!(
                    "{}: cannot snapshot machine diagnostics: {error}",
                    path.display()
                )
            });
            println!(
                "SMS_FM_RUNTIME_DIAGNOSTICS title={} frame={} instructions={} machine_ticks={} machine_clock_hz={} pc=0x{:04X} sp=0x{:04X} halted={} scanline={} vdp_cycles_into_line={} fm_port_writes={:?} performance={:?} frame_pacing={:?}",
                fixture.title,
                worker.frame_number(),
                total_instructions,
                machine_time_ticks,
                machine_clock_hz,
                debug_snapshot.pc,
                debug_snapshot.sp,
                debug_snapshot.halted,
                debug_snapshot.vdp_scanline,
                debug_snapshot.vdp_cycles_into_line,
                debug_snapshot.fm_port_writes,
                performance,
                frame_pacing,
            );
            let fm_bus_writes = debug_snapshot.fm_port_writes;
            worker.close_audio_output().unwrap_or_else(|error| {
                panic!("{}: cannot close native mixer: {error}", path.display())
            });

            let sources = source_capture.lock().unwrap_or_else(|_| {
                panic!("{}: FM source capture mutex was poisoned", path.display())
            });
            let pcm = pcm_capture
                .lock()
                .unwrap_or_else(|_| panic!("{}: PCM capture mutex was poisoned", path.display()));

            write_game_audio_captures(
                fixture.title,
                &sources.fm_samples_f64,
                &sources.psg_samples_f64,
                &sources.single_source_samples_f64,
                &pcm.samples_f64,
            )
            .unwrap_or_else(|error| {
                panic!("{}: cannot write diagnostic WAV: {error}", path.display())
            });

            assert!(
                fm_bus_writes.address_writes > 0 && fm_bus_writes.data_writes > 0,
                "{}: game did not commit YM2413 F0/F1 register writes during gameplay: {:?}",
                path.display(),
                fm_bus_writes
            );
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
                "SMS_FM_IN_GAME title={} rom={} sha256={} region={} system={} vdp={:?} frames={} fm_port_writes={:?} dual_source_blocks={} single_source_blocks={} single_source_samples={} psg_samples={} fm_samples={} fm_nonzero_samples={} fm_peak={:.8} pcm_blocks={} pcm_samples={} pcm_nonzero_samples={} pcm_peak={:.8} fm_summary={:?} psg_summary={:?} single_source_summary={:?} mixed_summary={:?}",
                fixture.title,
                path.display(),
                fixture.sha256,
                region,
                system,
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
        }));
        if let Err(payload) = result {
            let message = panic_payload_message(payload);
            eprintln!(
                "SMS_FM_FIXTURE_FAILED title={} rom={} error={}",
                fixture.title,
                path.display(),
                message
            );
            failures.push(format!("{}: {}", fixture.title, message));
        }
    }
    assert!(
        failures.is_empty(),
        "one or more headless FM fixtures failed: {}",
        failures.join(" | ")
    );
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
    assert_eq!(
        u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
        48_000
    );
    assert_eq!(u16::from_le_bytes([bytes[34], bytes[35]]), 32);
    assert_eq!(f32::from_le_bytes(bytes[44..48].try_into().unwrap()), 0.25);
    assert_eq!(f32::from_le_bytes(bytes[48..52].try_into().unwrap()), -0.5);
    fs::remove_file(path).unwrap();
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "requires legally obtained required reference ROMs and a working physical CPAL output device"]
fn real_japanese_sms_roms_play_fm_through_cpal_and_report_underruns() {
    let region_override = std::env::var("SEGA_SMS_FM_TEST_REGION").ok();
    let system_override = std::env::var("SEGA_SMS_FM_TEST_SYSTEM").ok();
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

    let mut failures = Vec::new();
    for (path, fixture, rom) in load_known_fm_roms() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let region = region_override.as_deref().unwrap_or(fixture.region);
            let system = system_override.as_deref().unwrap_or(fixture.system);
            let vdp_revision = selected_vdp_revision(system);
            let worker =
                SmsTemporalWorker::new_with_japanese_fm_hardware_profile_and_output_policy(
                    rom,
                    region,
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
                .unwrap_or_else(|error| {
                    panic!("{}: cannot open CPAL output: {error}", path.display())
                });
            worker.start().unwrap_or_else(|error| {
                panic!("{}: cannot start FM runtime: {error}", path.display())
            });

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

            worker.pause().unwrap_or_else(|error| {
                panic!("{}: cannot pause FM runtime: {error}", path.display())
            });
            let metrics = worker.audio_output_metrics();
            let performance = worker.performance_metrics();
            let frame_pacing = worker.frame_pacing_metrics();
            let total_instructions = worker.total_instructions();
            let machine_time_ticks = worker.machine_time_ticks();
            let machine_clock_hz = worker.machine_clock_hz();
            let debug_snapshot = worker.debug_snapshot().unwrap_or_else(|error| {
                panic!(
                    "{}: cannot snapshot CPAL diagnostics: {error}",
                    path.display()
                )
            });
            println!(
                "SMS_FM_CPAL_DIAGNOSTICS title={} frame={} instructions={} machine_ticks={} machine_clock_hz={} pc=0x{:04X} halted={} audio_metrics={:?} performance={:?} frame_pacing={:?}",
                fixture.title,
                worker.frame_number(),
                total_instructions,
                machine_time_ticks,
                machine_clock_hz,
                debug_snapshot.pc,
                debug_snapshot.halted,
                metrics,
                performance,
                frame_pacing,
            );
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
                "SMS_FM_CPAL title={} rom={} sha256={} region={} system={} vdp={:?} frames={} sample_rate_hz={} accepted_frames={} consumed_frames={} underrun_events={} underrun_frames={} dropped_blocks={} dropped_frames={} queue_high_water_frames={} queue_capacity_frames={}",
                fixture.title,
                path.display(),
                fixture.sha256,
                region,
                system,
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
        }));
        if let Err(payload) = result {
            let message = panic_payload_message(payload);
            eprintln!(
                "SMS_FM_CPAL_FIXTURE_FAILED title={} rom={} error={}",
                fixture.title,
                path.display(),
                message
            );
            failures.push(format!("{}: {}", fixture.title, message));
        }
    }
    assert!(
        failures.is_empty(),
        "one or more CPAL FM fixtures failed: {}",
        failures.join(" | ")
    );
}
