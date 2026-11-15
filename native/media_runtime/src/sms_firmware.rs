//! Content-addressed SMS firmware metadata and explicit local-directory scanning.
//!
//! Firmware bytes are read only by the explicit scan/load APIs in this module.
//! No filesystem operation is performed by the emulation core or Scheduler.

use crate::CartridgeIdentity;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

const BIOS_8K: usize = 8 * 1024;
const FIRMWARE_128K: usize = 128 * 1024;
const FIRMWARE_256K: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SmsFirmwareId {
    JapanV21,
    ExportUsEuV13,
    EuropeV20,
    HangOnV34,
    HangOnSafariHuntV24,
    MissileDefense3dV44,
    SamsungGamBoyIiKoreaAlexKidd,
    SonicEurope,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmsFirmwareKind {
    ConventionalBios,
    BuiltInGameFirmware,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmsConsoleVariant {
    Sms1,
    Sms2,
    SamsungGamBoyIi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmsFirmwareRegion {
    Japan,
    Export,
    Europe,
    Korea,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmsBuiltInGame {
    HangOn,
    HangOnAndSafariHunt,
    MissileDefense3d,
    AlexKiddInMiracleWorld,
    SonicTheHedgehog,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmsFirmwareMappingStrategy {
    ConventionalBios8KiB,
    InternalRomSlot2,
    InternalRomSlots1And2,
    HardwareSpecificUnresolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmsFirmwareValidation {
    IdentifiedOnly,
    MappingDocumentedBootPending,
    MappingUnresolvedBootPending,
    BootConfirmed,
}

/// Immutable catalog metadata. SHA-256 is the sole primary identity key;
/// auxiliary fingerprints are retained for diagnostics and interoperability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmsFirmwareDescriptor {
    pub id: SmsFirmwareId,
    pub name: &'static str,
    pub kind: SmsFirmwareKind,
    pub console: SmsConsoleVariant,
    pub region: SmsFirmwareRegion,
    pub built_in_game: Option<SmsBuiltInGame>,
    pub size_bytes: usize,
    pub sha256: String,
    pub crc32: Option<u32>,
    pub sha1: Option<&'static str>,
    pub md5: Option<&'static str>,
    pub mapping: SmsFirmwareMappingStrategy,
    pub validation: SmsFirmwareValidation,
    pub evidence: &'static [&'static str],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmsFirmwareMatch {
    pub descriptor: SmsFirmwareDescriptor,
    pub identity: CartridgeIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SmsFirmwareScanStatus {
    Identified,
    DuplicateHash,
    UnknownContent,
    UnsupportedSize,
    ReadError(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmsFirmwareScanEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub sha256: Option<String>,
    pub firmware_id: Option<SmsFirmwareId>,
    pub status: SmsFirmwareScanStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SmsFirmwareLoadError {
    Io(String),
    SizeMismatch { actual: usize, expected: usize },
    IdentityMismatch { expected: SmsFirmwareId },
}

impl Display for SmsFirmwareLoadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "could not read SMS firmware: {error}"),
            Self::SizeMismatch { actual, expected } => write!(
                formatter,
                "SMS firmware size changed: expected {expected} bytes, received {actual}"
            ),
            Self::IdentityMismatch { expected } => write!(
                formatter,
                "SMS firmware content no longer matches catalog identity {expected:?}"
            ),
        }
    }
}

impl std::error::Error for SmsFirmwareLoadError {}

/// Content-addressed catalog for conventional SMS BIOS and built-in-game firmware.
#[derive(Clone, Debug, Default)]
pub struct SmsFirmwareCatalog {
    entries: Vec<SmsFirmwareDescriptor>,
}

impl SmsFirmwareCatalog {
    pub fn sms_default() -> Self {
        Self {
            entries: sms_firmware_entries(),
        }
    }

    /// Build a catalog from explicit metadata. Primarily useful for isolated tests
    /// and future catalog extensions; duplicate identities are rejected at lookup.
    pub fn new(entries: Vec<SmsFirmwareDescriptor>) -> Self {
        Self { entries }
    }

    pub fn entries(&self) -> &[SmsFirmwareDescriptor] {
        &self.entries
    }

    pub fn descriptor(&self, id: SmsFirmwareId) -> Option<&SmsFirmwareDescriptor> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn identify(&self, bytes: &[u8]) -> Option<SmsFirmwareMatch> {
        if !self.entries.iter().any(|entry| entry.size_bytes == bytes.len()) {
            return None;
        }

        let identity = CartridgeIdentity::from_bytes(bytes);
        let sha256 = identity.sha256_hex();
        let mut matches = self.entries.iter().filter(|entry| {
            entry.size_bytes == bytes.len() && entry.sha256.eq_ignore_ascii_case(&sha256)
        });
        let descriptor = matches.next()?;
        if matches.next().is_some() {
            return None;
        }

        Some(SmsFirmwareMatch {
            descriptor: descriptor.clone(),
            identity,
        })
    }

    /// Scan regular files in one directory. Names and extensions are ignored.
    /// Files with unsupported sizes are rejected before their contents are read.
    /// Results are deterministic: paths are sorted and the first matching hash
    /// is Identified; later files with the same hash are DuplicateHash.
    pub fn scan_directory(
        &self,
        directory: impl AsRef<Path>,
    ) -> Result<Vec<SmsFirmwareScanEntry>, io::Error> {
        let mut paths = fs::read_dir(directory)?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .filter(|kind| kind.is_file())
                    .map(|_| entry.path())
            })
            .collect::<Vec<_>>();
        paths.sort();

        let mut seen_hashes = HashSet::new();
        let mut results = Vec::with_capacity(paths.len());

        for path in paths {
            let metadata = match fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    results.push(SmsFirmwareScanEntry {
                        path,
                        size_bytes: 0,
                        sha256: None,
                        firmware_id: None,
                        status: SmsFirmwareScanStatus::ReadError(error.to_string()),
                    });
                    continue;
                }
            };
            let size_bytes = metadata.len();
            let supported_size = usize::try_from(size_bytes)
                .ok()
                .is_some_and(|size| self.entries.iter().any(|entry| entry.size_bytes == size));
            if !supported_size {
                results.push(SmsFirmwareScanEntry {
                    path,
                    size_bytes,
                    sha256: None,
                    firmware_id: None,
                    status: SmsFirmwareScanStatus::UnsupportedSize,
                });
                continue;
            }

            let mut file = match fs::File::open(&path) {
                Ok(file) => file,
                Err(error) => {
                    results.push(SmsFirmwareScanEntry {
                        path,
                        size_bytes,
                        sha256: None,
                        firmware_id: None,
                        status: SmsFirmwareScanStatus::ReadError(error.to_string()),
                    });
                    continue;
                }
            };
            let expected_size = usize::try_from(size_bytes).unwrap_or_default();
            let mut bytes = vec![0; expected_size];
            let read_result = file.read_exact(&mut bytes).and_then(|()| {
                let mut extra = [0_u8; 1];
                if file.read(&mut extra)? == 0 {
                    Ok(())
                } else {
                    Err(io::Error::new(io::ErrorKind::InvalidData, "file grew during firmware scan"))
                }
            });
            if let Err(error) = read_result {
                results.push(SmsFirmwareScanEntry {
                    path,
                    size_bytes,
                    sha256: None,
                    firmware_id: None,
                    status: SmsFirmwareScanStatus::ReadError(error.to_string()),
                });
                continue;
            }
            let Some(found) = self.identify(&bytes) else {
                results.push(SmsFirmwareScanEntry {
                    path,
                    size_bytes,
                    sha256: Some(CartridgeIdentity::from_bytes(&bytes).sha256_hex()),
                    firmware_id: None,
                    status: SmsFirmwareScanStatus::UnknownContent,
                });
                continue;
            };

            let hash = found.identity.sha256_hex();
            let status = if seen_hashes.insert(hash.clone()) {
                SmsFirmwareScanStatus::Identified
            } else {
                SmsFirmwareScanStatus::DuplicateHash
            };
            results.push(SmsFirmwareScanEntry {
                path,
                size_bytes,
                sha256: Some(hash),
                firmware_id: Some(found.descriptor.id),
                status,
            });
        }

        Ok(results)
    }

    /// Re-read a selected file and verify it again immediately before loading.
    /// The caller should pass the selected catalog ID, not a user-supplied name.
    pub fn load_validated(
        &self,
        path: impl AsRef<Path>,
        expected: SmsFirmwareId,
    ) -> Result<Vec<u8>, SmsFirmwareLoadError> {
        let descriptor = self
            .descriptor(expected)
            .ok_or(SmsFirmwareLoadError::IdentityMismatch { expected })?;
        let expected_size = descriptor.size_bytes;
        let mut file = fs::File::open(path)
            .map_err(|error| SmsFirmwareLoadError::Io(error.to_string()))?;
        let actual_size = file
            .metadata()
            .map_err(|error| SmsFirmwareLoadError::Io(error.to_string()))?
            .len();
        if actual_size != expected_size as u64 {
            return Err(SmsFirmwareLoadError::SizeMismatch {
                actual: usize::try_from(actual_size).unwrap_or(usize::MAX),
                expected: expected_size,
            });
        }
        let mut bytes = vec![0; expected_size];
        file.read_exact(&mut bytes)
            .map_err(|error| SmsFirmwareLoadError::Io(error.to_string()))?;
        let mut extra = [0_u8; 1];
        if file.read(&mut extra).map_err(|error| SmsFirmwareLoadError::Io(error.to_string()))? != 0 {
            return Err(SmsFirmwareLoadError::SizeMismatch {
                actual: expected_size.saturating_add(1),
                expected: expected_size,
            });
        }
        if self
            .identify(&bytes)
            .is_none_or(|found| found.descriptor.id != expected)
        {
            return Err(SmsFirmwareLoadError::IdentityMismatch { expected });
        }
        Ok(bytes)
    }
}

fn sms_firmware_entries() -> Vec<SmsFirmwareDescriptor> {
    use SmsBuiltInGame as Game;
    use SmsConsoleVariant as Console;
    use SmsFirmwareId as Id;
    use SmsFirmwareKind as Kind;
    use SmsFirmwareMappingStrategy as Mapping;
    use SmsFirmwareRegion as Region;
    use SmsFirmwareValidation as Validation;

    const LOCAL_HASH: &[&str] = &["User-supplied image: size and cryptographic hashes calculated from extracted .sms bytes"];
    const SMS_POWER: &[&str] = &["https://www.smspower.org/Development/BIOSes"];
    const BIOS_EVIDENCE: &[&str] = &["Existing project SMS BIOS catalog; conventional 8 KiB BIOS"];
    [
        SmsFirmwareDescriptor {
            id: Id::JapanV21,
            name: "Master System Japan v2.1",
            kind: Kind::ConventionalBios,
            console: Console::Sms1,
            region: Region::Japan,
            built_in_game: None,
            size_bytes: BIOS_8K,
            sha256: "67846e26764bd862f19179294347f7353a4166b62ac4198a5ec32933b7da486e".to_owned(),
            crc32: None,
            sha1: None,
            md5: None,
            mapping: Mapping::ConventionalBios8KiB,
            validation: Validation::IdentifiedOnly,
            evidence: BIOS_EVIDENCE,
        },
        SmsFirmwareDescriptor {
            id: Id::ExportUsEuV13,
            name: "Master System USA/Europe v1.3",
            kind: Kind::ConventionalBios,
            console: Console::Sms1,
            region: Region::Export,
            built_in_game: None,
            size_bytes: BIOS_8K,
            sha256: "477617917a12a30f9f43844909dc2de6e6a617430f5c9a36306c86414a670d50".to_owned(),
            crc32: None,
            sha1: None,
            md5: None,
            mapping: Mapping::ConventionalBios8KiB,
            validation: Validation::IdentifiedOnly,
            evidence: BIOS_EVIDENCE,
        },
        SmsFirmwareDescriptor {
            id: Id::EuropeV20,
            name: "Master System Europe v2.0",
            kind: Kind::ConventionalBios,
            console: Console::Sms1,
            region: Region::Europe,
            built_in_game: None,
            size_bytes: BIOS_8K,
            sha256: "1eb16fc06ceaf05651d7cfd69d1713e6ddccc4349a511f6fd17e037d71f9d85b".to_owned(),
            crc32: None,
            sha1: None,
            md5: None,
            mapping: Mapping::ConventionalBios8KiB,
            validation: Validation::IdentifiedOnly,
            evidence: BIOS_EVIDENCE,
        },
        SmsFirmwareDescriptor {
            id: Id::HangOnV34,
            name: "Hang On USA/Europe v3.4",
            kind: Kind::BuiltInGameFirmware,
            console: Console::Sms1,
            region: Region::Export,
            built_in_game: Some(Game::HangOn),
            size_bytes: FIRMWARE_128K,
            sha256: "1a73bc6e1a9973b98432774a1480b3ac1259027a97590d5e67d52745159abbbc".to_owned(),
            crc32: Some(0x8EDF_7AC6),
            sha1: Some("51fd6d7990f62cd9d18c9ecfc62ed7936169107e"),
            md5: Some("5cd8f62cd8786af0226e6d2248279338"),
            mapping: Mapping::InternalRomSlot2,
            validation: Validation::MappingDocumentedBootPending,
            evidence: &[LOCAL_HASH[0], SMS_POWER[0], "SMS Power documents 128 KiB ROM and slot 2 mapping"],
        },
        SmsFirmwareDescriptor {
            id: Id::HangOnSafariHuntV24,
            name: "Hang On & Safari Hunt USA/Europe/Brazil v2.4",
            kind: Kind::BuiltInGameFirmware,
            console: Console::Sms1,
            region: Region::Export,
            built_in_game: Some(Game::HangOnAndSafariHunt),
            size_bytes: FIRMWARE_128K,
            sha256: "e9eb04ec80ded8658f765c4eca75cdbe23cbc7d8fed73f9c3db7a868dbf649ff".to_owned(),
            crc32: Some(0x91E9_3385),
            sha1: Some("9e179392cd416af14024d8f31c981d9ee9a64517"),
            md5: Some("02cbb2e348945c9ac41e37502a58ca76"),
            mapping: Mapping::InternalRomSlot2,
            validation: Validation::MappingDocumentedBootPending,
            evidence: &[LOCAL_HASH[0], SMS_POWER[0], "SMS Power documents 128 KiB ROM and slot 2 mapping"],
        },
        SmsFirmwareDescriptor {
            id: Id::MissileDefense3dV44,
            name: "Missile Defense 3-D USA/Europe v4.4",
            kind: Kind::BuiltInGameFirmware,
            console: Console::Sms1,
            region: Region::Export,
            built_in_game: Some(Game::MissileDefense3d),
            size_bytes: FIRMWARE_128K,
            sha256: "13c0639cc037167f53e6151dbdea013758c3d09a76b064a894bf15c39c0a3897".to_owned(),
            crc32: Some(0xE79B_B689),
            sha1: Some("aa92ae576ca670b00855e278378d89e9f85e0351"),
            md5: Some("08b81aa6be18b92daef1b875deecf824"),
            mapping: Mapping::InternalRomSlot2,
            validation: Validation::MappingDocumentedBootPending,
            evidence: &[LOCAL_HASH[0], SMS_POWER[0], "SMS Power documents 128 KiB ROM and slot 2 mapping"],
        },
        SmsFirmwareDescriptor {
            id: Id::SamsungGamBoyIiKoreaAlexKidd,
            name: "Samsung Gam*Boy II Korea — Alex Kidd",
            kind: Kind::BuiltInGameFirmware,
            console: Console::SamsungGamBoyIi,
            region: Region::Korea,
            built_in_game: Some(Game::AlexKiddInMiracleWorld),
            size_bytes: FIRMWARE_128K,
            sha256: "57abf3c00c82a8e042ebeb2bfe412adac9ba005e9009845236390de3ee645248".to_owned(),
            crc32: Some(0x9C5B_AD91),
            sha1: Some("2feafd8f1c40fdf1bd5668f8c5c02e5560945b17"),
            md5: Some("e49e63328c78fe3d24ae32bf903583e0"),
            mapping: Mapping::HardwareSpecificUnresolved,
            validation: Validation::MappingUnresolvedBootPending,
            evidence: &[LOCAL_HASH[0], SMS_POWER[0], "SMS Power documents Samsung SMS2-variant slot detection; exact internal game mapping remains unaudited"],
        },
        SmsFirmwareDescriptor {
            id: Id::SonicEurope,
            name: "Sonic The Hedgehog Europe",
            kind: Kind::BuiltInGameFirmware,
            console: Console::Sms2,
            region: Region::Europe,
            built_in_game: Some(Game::SonicTheHedgehog),
            size_bytes: FIRMWARE_256K,
            sha256: "c51ec80b7a609c5ee3d203ceee2b602ae4dfb4479caf15dc1d030c40c0364e93".to_owned(),
            crc32: Some(0x81C3_476B),
            sha1: Some("6aca0e3dffe461ba1cb11a86cd4caf5b97e1b8df"),
            md5: Some("4187d96beaf36385e681a3cf3bd1663d"),
            mapping: Mapping::InternalRomSlots1And2,
            validation: Validation::MappingDocumentedBootPending,
            evidence: &[LOCAL_HASH[0], SMS_POWER[0], "SMS Power documents 256 KiB ROM requiring slot 1 and slot 2 mapping; used only in Master System II"],
        },
    ].into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "sega-sms-firmware-test-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn synthetic_catalog(bytes: &[u8]) -> SmsFirmwareCatalog {
        let identity = CartridgeIdentity::from_bytes(bytes);
        SmsFirmwareCatalog::new(vec![SmsFirmwareDescriptor {
            id: SmsFirmwareId::HangOnV34,
            name: "Synthetic firmware",
            kind: SmsFirmwareKind::BuiltInGameFirmware,
            console: SmsConsoleVariant::Sms1,
            region: SmsFirmwareRegion::Export,
            built_in_game: Some(SmsBuiltInGame::HangOn),
            size_bytes: bytes.len(),
            sha256: identity.sha256_hex(),
            crc32: None,
            sha1: None,
            md5: None,
            mapping: SmsFirmwareMappingStrategy::InternalRomSlot2,
            validation: SmsFirmwareValidation::IdentifiedOnly,
            evidence: &["synthetic test fixture"],
        }])
    }

    #[test]
    fn default_catalog_keeps_conventional_and_builtin_firmware_distinct() {
        let catalog = SmsFirmwareCatalog::sms_default();
        assert_eq!(catalog.entries().len(), 8);
        assert_eq!(
            catalog.entries().iter().filter(|entry| entry.kind == SmsFirmwareKind::ConventionalBios).count(),
            3
        );
        assert_eq!(
            catalog.entries().iter().filter(|entry| entry.kind == SmsFirmwareKind::BuiltInGameFirmware).count(),
            5
        );
        assert!(catalog.entries().iter().all(|entry| entry.validation != SmsFirmwareValidation::BootConfirmed));
    }

    #[test]
    fn directory_scan_ignores_names_and_extensions_and_detects_duplicates() {
        let directory = TempDirectory::new();
        let bytes = vec![0xA5; FIRMWARE_128K];
        let catalog = synthetic_catalog(&bytes);
        fs::write(directory.path().join("renamed-without-extension"), &bytes).unwrap();
        fs::write(directory.path().join("anything.unusual"), &bytes).unwrap();

        let results = catalog.scan_directory(directory.path()).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].status, SmsFirmwareScanStatus::Identified);
        assert_eq!(results[1].status, SmsFirmwareScanStatus::DuplicateHash);
        assert_eq!(results[0].firmware_id, Some(SmsFirmwareId::HangOnV34));
    }

    #[test]
    fn directory_scan_rejects_unsupported_size_before_hashing() {
        let directory = TempDirectory::new();
        fs::write(directory.path().join("truncated.sms"), vec![0xA5; 4096]).unwrap();
        let catalog = SmsFirmwareCatalog::sms_default();

        let results = catalog.scan_directory(directory.path()).unwrap();
        assert_eq!(results[0].status, SmsFirmwareScanStatus::UnsupportedSize);
        assert_eq!(results[0].sha256, None);
    }

    #[test]
    fn directory_scan_reports_unknown_content_at_a_supported_size() {
        let directory = TempDirectory::new();
        fs::write(directory.path().join("wrong-content.bin"), vec![0x5A; BIOS_8K]).unwrap();
        let catalog = SmsFirmwareCatalog::sms_default();

        let results = catalog.scan_directory(directory.path()).unwrap();
        assert_eq!(results[0].status, SmsFirmwareScanStatus::UnknownContent);
        assert!(results[0].sha256.is_some());
    }

    #[test]
    fn selected_file_is_revalidated_before_loading() {
        let directory = TempDirectory::new();
        let bytes = vec![0xA5; FIRMWARE_128K];
        let catalog = synthetic_catalog(&bytes);
        let path = directory.path().join("renamed.data");
        fs::write(&path, &bytes).unwrap();

        assert_eq!(
            catalog.load_validated(&path, SmsFirmwareId::HangOnV34).unwrap(),
            bytes
        );

        fs::write(&path, vec![0x5A; FIRMWARE_128K]).unwrap();
        assert!(matches!(
            catalog.load_validated(&path, SmsFirmwareId::HangOnV34),
            Err(SmsFirmwareLoadError::IdentityMismatch { .. })
        ));
    }

    #[test]
    fn selected_file_rejects_truncation_before_reading() {
        let directory = TempDirectory::new();
        let bytes = vec![0xA5; FIRMWARE_128K];
        let catalog = synthetic_catalog(&bytes);
        let path = directory.path().join("firmware");
        fs::write(&path, &bytes[..4096]).unwrap();

        assert!(matches!(
            catalog.load_validated(&path, SmsFirmwareId::HangOnV34),
            Err(SmsFirmwareLoadError::SizeMismatch { .. })
        ));
    }
}
