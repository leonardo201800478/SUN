use crate::{
    CartridgeCompatibilityMetadata, CartridgeHardwareProfile, CartridgeIdentity,
    CartridgeMapper, CartridgeRamKind, CartridgeRamMapping, CartridgeRamProfile,
    CartridgeSystem, DatGameEntry, DatRomEntry, NoIntroDat, ProfileStatus,
    SmsVdpRequirement,
};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::OnceLock;

/// Version of the compatibility metadata structure used by the SMS catalog.
pub const SMS_CARTRIDGE_COMPATIBILITY_SCHEMA_VERSION: u32 = 1;
/// Revision of the evidence-backed SMS compatibility declarations.
pub const SMS_CARTRIDGE_COMPATIBILITY_DATA_VERSION: &str = "2026-11-15";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownCartridgeProfile {
    pub sha1: String,
    pub profile: CartridgeHardwareProfile,
    pub compatibility: CartridgeCompatibilityMetadata,
}

impl KnownCartridgeProfile {
    /// Construct a profile without an independently supported VDP requirement.
    pub fn new(sha1: impl Into<String>, profile: CartridgeHardwareProfile) -> Self {
        Self::with_compatibility(sha1, profile, CartridgeCompatibilityMetadata::default())
    }

    pub fn with_compatibility(
        sha1: impl Into<String>,
        profile: CartridgeHardwareProfile,
        compatibility: CartridgeCompatibilityMetadata,
    ) -> Self {
        Self {
            sha1: sha1.into().to_ascii_lowercase(),
            profile,
            compatibility,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CartridgeMatch<'a> {
    pub game: &'a DatGameEntry,
    pub rom: &'a DatRomEntry,
    pub profile: Option<&'a CartridgeHardwareProfile>,
    pub compatibility: Option<&'a CartridgeCompatibilityMetadata>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogLookupError {
    AmbiguousDatIdentity {
        sha1: String,
        entries: usize,
    },
    AmbiguousHardwareProfile {
        sha1: String,
        profiles: usize,
    },
    DatFingerprintMismatch {
        sha1: String,
        dat_size_bytes: usize,
        identity_size_bytes: usize,
        dat_crc32: u32,
        identity_crc32: u32,
    },
}

impl Display for CatalogLookupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AmbiguousDatIdentity { sha1, entries } => write!(
                formatter,
                "DAT contains {entries} entries with the same SHA-1 {sha1}"
            ),
            Self::AmbiguousHardwareProfile { sha1, profiles } => write!(
                formatter,
                "catalog contains {profiles} hardware profiles for SHA-1 {sha1}"
            ),
            Self::DatFingerprintMismatch {
                sha1,
                dat_size_bytes,
                identity_size_bytes,
                dat_crc32,
                identity_crc32,
            } => write!(
                formatter,
                "DAT fingerprint mismatch for SHA-1 {sha1}: size {identity_size_bytes} != {dat_size_bytes}, CRC32 {identity_crc32:08X} != {dat_crc32:08X}"
            ),
        }
    }
}

impl Error for CatalogLookupError {}

/// Content-addressed ROM catalog joined to explicit hardware profiles.
#[derive(Clone, Debug)]
pub struct CartridgeCatalog {
    dat: NoIntroDat,
    profiles: Vec<KnownCartridgeProfile>,
}

static SMS_CARTRIDGE_CATALOG: OnceLock<Result<CartridgeCatalog, String>> = OnceLock::new();

impl CartridgeCatalog {
    /// Load the SMS DAT committed in this repository and its explicit hardware profiles.
    pub fn sms_default() -> Result<Self, crate::DatParseError> {
        Ok(Self::from_dat(NoIntroDat::sms_default()?, sms_profiles()))
    }

    /// Return the process-wide parsed SMS catalog for repeated runtime preflights.
    pub fn sms_shared() -> Result<&'static Self, String> {
        match SMS_CARTRIDGE_CATALOG
            .get_or_init(|| Self::sms_default().map_err(|error| error.to_string()))
        {
            Ok(catalog) => Ok(catalog),
            Err(error) => Err(error.clone()),
        }
    }

    pub fn from_dat(dat: NoIntroDat, profiles: Vec<KnownCartridgeProfile>) -> Self {
        Self { dat, profiles }
    }

    pub const fn dat(&self) -> &NoIntroDat {
        &self.dat
    }

    pub fn profiles(&self) -> &[KnownCartridgeProfile] {
        &self.profiles
    }

    /// Match an image by SHA-1 and verify the DAT's size and CRC32.
    ///
    /// The ROM filename is never consulted. A matching SHA-1 with inconsistent
    /// auxiliary metadata is reported as an integrity error, not accepted.
    pub fn lookup(
        &self,
        identity: &CartridgeIdentity,
    ) -> Result<Option<CartridgeMatch<'_>>, CatalogLookupError> {
        let sha1 = identity.sha1_hex();
        let matches: Vec<_> = self
            .dat
            .entries
            .iter()
            .flat_map(|game| {
                game.roms
                    .iter()
                    .filter(|rom| rom.sha1.eq_ignore_ascii_case(&sha1))
                    .map(move |rom| (game, rom))
            })
            .collect();

        let (game, rom) = match matches.as_slice() {
            [] => return Ok(None),
            [(game, rom)] => (*game, *rom),
            _ => {
                return Err(CatalogLookupError::AmbiguousDatIdentity {
                    sha1,
                    entries: matches.len(),
                });
            }
        };

        if rom.size_bytes != identity.size_bytes() || rom.crc32 != identity.crc32() {
            return Err(CatalogLookupError::DatFingerprintMismatch {
                sha1,
                dat_size_bytes: rom.size_bytes,
                identity_size_bytes: identity.size_bytes(),
                dat_crc32: rom.crc32,
                identity_crc32: identity.crc32(),
            });
        }

        let matching_profiles: Vec<_> = self
            .profiles
            .iter()
            .filter(|known| known.sha1.eq_ignore_ascii_case(&rom.sha1))
            .collect();
        let (profile, compatibility) = match matching_profiles.as_slice() {
            [] => (None, None),
            [known] => (Some(&known.profile), Some(&known.compatibility)),
            _ => {
                return Err(CatalogLookupError::AmbiguousHardwareProfile {
                    sha1,
                    profiles: matching_profiles.len(),
                });
            }
        };

        Ok(Some(CartridgeMatch {
            game,
            rom,
            profile,
            compatibility,
        }))
    }

    /// Return a profile only when the exact ROM identity has a verified executable mapping.
    pub fn lookup_runtime_profile(
        &self,
        identity: &CartridgeIdentity,
    ) -> Result<Option<&CartridgeHardwareProfile>, CatalogLookupError> {
        Ok(self
            .lookup(identity)?
            .and_then(|matched| matched.profile)
            .filter(|profile| profile.is_runtime_usable()))
    }
}

fn sms_profiles() -> Vec<KnownCartridgeProfile> {
    let mut profiles = Vec::new();

    // Cosmic Spacehead (Europe) is tied to the exact No-Intro identity.
    // Genesis Plus GX independently classifies it as SYSTEM_SMS2 and Codemasters;
    // SMS Power documents its 224-line mode and SMS1 incompatibility.
    profiles.push(KnownCartridgeProfile::with_compatibility(
        "f46f716dd34a1a5013a2d8a59769f6ef7536a567",
        CartridgeHardwareProfile {
            system: CartridgeSystem::MasterSystem,
            mapper: CartridgeMapper::Codemasters,
            ram: None,
            status: ProfileStatus::EvidenceOnly,
            board: None,
            evidence: vec![
                "Project No-Intro SMS DAT 20261013-064924: exact SHA-1/CRC32/size".to_owned(),
                "Genesis Plus GX core/cart_hw/sms_cart.c: CRC32 29822980 is classified as Codemasters and SYSTEM_SMS2".to_owned(),
                "SMS Power Development/CodemastersHeader: Codemasters cartridge header".to_owned(),
            ],
        },
        CartridgeCompatibilityMetadata::new(
            SmsVdpRequirement::Sms2Required,
            vec![
                "Genesis Plus GX core/cart_hw/sms_cart.c: CRC32 29822980 is classified as SYSTEM_SMS2".to_owned(),
                "SMS Power Development/SMS1SMS2VDPs: 224/240-line modes are supported by the 315-5246 SMS2 VDP".to_owned(),
                "SMS Power forums/1484-CosmicSpacehead: Charles MacDonald identifies Cosmic Spacehead as a 224-line title and reports SMS1 incompatibility".to_owned(),
            ],
        ),
    ));

    // Commercial battery-backed 8 KiB SRAM identities explicitly confirmed by
    // the project No-Intro DAT, MAME sms.xml, and the project's SMS mapper audit.
    for sha1 in [
        "fbda0486b393708a89756bb57d116ad6007484e4", // Golden Axe Warrior
        "d0b964dd7cd8ccdd730de4d8e4bb2e87bea7686e", // Golfamania
        "f952406bca4918ee91a89b27e949e224eae96d85", // Miracle Warriors
        "8dd7bb4f666f70f7f57687823d0068c9100af8e5", // Monopoly
        "6bad7176011dd4bbd007498d167399daacde173d", // Monopoly Rev A
        "8762239c339a084dfb8443cc38515301476bde28", // Penguin Land
        "07fcf297be4f4c9d92cd3f119a7ac48467e06838", // Phantasy Star USA/Europe Rev A
        "fd8dad6acb6fa75dc8e9bbaea2a7e9fd486fc2dd", // Phantasy Star Brazil
        "c9a40ddd217c58dddcd6b5c0fe66c3a50d3e68e4", // Phantasy Star Japan
        "257ca76ebcd54c75a414ca7ce968fa59ea42f150", // Phantasy Star USA/Europe
        "a90e21e5961bcf2e10b715a009c04e7c2017a3b1", // Ultima IV
        "614b589080b732e17cc0d253e17216a72a268955", // Ys Japan
        "e73e836c353543e9f48315410b0d72278899ff59", // Ys The Vanished Omens
    ] {
        profiles.push(KnownCartridgeProfile::new(
            sha1,
            battery_sram_8k(ProfileStatus::Verified, false),
        ));
    }

    // Exact prototype, regional, or dump variants whose DAT/MAME RAM metadata
    // is known but whose board/mapping evidence is not sufficient for auto-map.
    for sha1 in [
        "da88dc3e84daa2f8b8d803b00a13b5fb3185d8c5", // Golfamania Beta
        "ebe733ea693fa46edde43260fdb963cab71317e1", // Super Golf Beta
        "46a032004d49fec58099aa6bf0dd796997e95142", // Haja no Fuuin Japan
        "5f1cfc974a8ffc965f18216a89472fd1a980d7cb", // Miracle Warriors Beta
        "68830fb46a010cf2f2d04bdc69e3759f6d42b8de", // Monopoly Beta 1
        "6d100d43cddfb761d3560786c26e345873452a4b", // Monopoly Beta 2
        "c01cf44eee335d509dc20a165add8514e7fbb7c4", // Doki Doki Penguin Land Japan
        "c56417efd78a332a35e4d0e87c7b13c20da85391", // Doki Doki Penguin Land Beta
        "52b2aa52a1c96e15869498a8e42b074705070007", // Phantasy Star Korea unlicensed
        "b3ae447dc739256616b44cbd77cb903c9f19e980", // Phantasy Star Japan Rev 1 (MAME baddump)
        "bb1ae06b62a9f7d3259c51eee4cfded781eb5d30", // Ultima IV Beta
    ] {
        profiles.push(KnownCartridgeProfile::new(
            sha1,
            battery_sram_8k(ProfileStatus::EvidenceOnly, true),
        ));
    }

    // RAM without battery: these profiles are volatile and must never create .srm files.
    for sha1 in [
        "60e2b6ec69d73dd73c1ef846634942c81800655b", // Desert Speedtrap
        "1732b4c13fd00dd5efc5bf1ccb1ab6ed3889c8ba", // Flash, The
    ] {
        profiles.push(KnownCartridgeProfile::new(sha1, volatile_sram_8k()));
    }

    // The Terminator has a 32 KiB volatile RAM region in MAME. Mapping is not
    // confirmed, so these identities remain evidence-only and non-executable.
    for sha1 in [
        "eaf733d385e61526b90c1b194bf605078d43e2d3", // Europe
        "bea0a1d0271faa2273e4cf1e968241d018c81da1", // Brazil
    ] {
        profiles.push(KnownCartridgeProfile::new(
            sha1,
            volatile_sram_32k_evidence(),
        ));
    }

    // Flight of Pigarus is known to use 32 KiB FeRAM, but the mapper/address
    // decode and exact page-selection protocol are still blocked.
    for sha1 in [
        "46941b6ca4c38b07fa55987d3f30c249d5674e2a", // v1.11
        "3bd03c3be402b90e61e6be5b67216c0c6d8056e6", // v1.10
        "e53a16f5bf4a2d804b26c359cf3a727c06379e6d", // v1.00
    ] {
        profiles.push(KnownCartridgeProfile::new(
            sha1,
            flight_of_pigarus_blocked(),
        ));
    }

    profiles
}

fn battery_sram_8k(status: ProfileStatus, mapping_unknown: bool) -> CartridgeHardwareProfile {
    CartridgeHardwareProfile {
        system: CartridgeSystem::MasterSystem,
        mapper: if mapping_unknown {
            CartridgeMapper::Unknown
        } else {
            CartridgeMapper::SmsSega
        },
        ram: Some(CartridgeRamProfile {
            kind: CartridgeRamKind::BatteryBackedSram,
            capacity_bytes: 0x2000,
            mapping: if mapping_unknown {
                CartridgeRamMapping::Unknown
            } else {
                CartridgeRamMapping::SegaMapperSlot2
            },
            mapped_window_bytes: if mapping_unknown { 0 } else { 0x4000 },
            bank_size_bytes: 0x2000,
            mirrored: !mapping_unknown,
        }),
        status,
        board: None,
        evidence: vec![
            "Project No-Intro SMS DAT 20261013-064924: exact SHA-1/CRC32/size".to_owned(),
            "MAME hash/sms.xml: battery=yes and 8192-byte RAM dataarea".to_owned(),
            "Project docs/hardware/sms-special-mappers-research.md".to_owned(),
            "SMS Power Development/Mappers".to_owned(),
        ],
    }
}

fn volatile_sram_8k() -> CartridgeHardwareProfile {
    CartridgeHardwareProfile {
        system: CartridgeSystem::MasterSystem,
        mapper: CartridgeMapper::SmsSega,
        ram: Some(CartridgeRamProfile {
            kind: CartridgeRamKind::VolatileSram,
            capacity_bytes: 0x2000,
            mapping: CartridgeRamMapping::SegaMapperSlot2,
            mapped_window_bytes: 0x4000,
            bank_size_bytes: 0x2000,
            mirrored: true,
        }),
        status: ProfileStatus::Verified,
        board: None,
        evidence: vec![
            "Project No-Intro SMS DAT 20261013-064924: exact SHA-1/CRC32/size".to_owned(),
            "MAME hash/sms.xml: 8192-byte RAM dataarea, no battery feature".to_owned(),
            "Project docs/hardware/sms-special-mappers-research.md: RAM without battery".to_owned(),
        ],
    }
}

fn volatile_sram_32k_evidence() -> CartridgeHardwareProfile {
    CartridgeHardwareProfile {
        system: CartridgeSystem::MasterSystem,
        mapper: CartridgeMapper::Unknown,
        ram: Some(CartridgeRamProfile {
            kind: CartridgeRamKind::VolatileSram,
            capacity_bytes: 0x8000,
            mapping: CartridgeRamMapping::Unknown,
            mapped_window_bytes: 0,
            bank_size_bytes: 0x4000,
            mirrored: false,
        }),
        status: ProfileStatus::EvidenceOnly,
        board: Some("Europe PCB 171-6339A; IC2 KM62256BLP-10L".to_owned()),
        evidence: vec![
            "Project No-Intro SMS DAT 20261013-064924: exact regional SHA-1/CRC32/size".to_owned(),
            "MAME hash/sms.xml: 32768-byte RAM dataarea; no battery feature".to_owned(),
            "Europe board/IC identification is not inherited by the Brazil revision".to_owned(),
        ],
    }
}

fn flight_of_pigarus_blocked() -> CartridgeHardwareProfile {
    CartridgeHardwareProfile {
        system: CartridgeSystem::MasterSystem,
        mapper: CartridgeMapper::Unknown,
        ram: Some(CartridgeRamProfile {
            kind: CartridgeRamKind::FeRam,
            capacity_bytes: 0x8000,
            mapping: CartridgeRamMapping::Unknown,
            mapped_window_bytes: 0,
            bank_size_bytes: 0x4000,
            mirrored: false,
        }),
        status: ProfileStatus::Blocked,
        board: Some("Lapis MR48V256 FeRAM (256 Kibit)".to_owned()),
        evidence: vec![
            "Project No-Intro SMS DAT 20261013-064924: three exact ROM fingerprints".to_owned(),
            "MAME hash/sms.xml: 32768-byte RAM dataarea".to_owned(),
            "SMS Power developer Q&A: two logical 16 KiB pages".to_owned(),
            "SMS Power PCB designer discussion: mapper and SRAM changed for two pages".to_owned(),
            "Exact CPU address decode and page-select protocol remain unconfirmed".to_owned(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CartridgeCatalog, CatalogLookupError, KnownCartridgeProfile, flight_of_pigarus_blocked,
        sms_profiles,
    };
    use crate::{
        CartridgeCompatibilityMetadata, CartridgeHardwareProfile, CartridgeIdentity,
        CartridgeMapper, CartridgeRamKind, CartridgeRamMapping, CartridgeRamProfile,
        CartridgeSystem, DatGameEntry, DatRomEntry, NoIntroDat, ProfileStatus,
        SmsVdpRequirement,
    };

    fn test_battery_profile() -> CartridgeHardwareProfile {
        CartridgeHardwareProfile {
            system: CartridgeSystem::MasterSystem,
            mapper: CartridgeMapper::SmsSega,
            ram: Some(CartridgeRamProfile {
                kind: CartridgeRamKind::BatteryBackedSram,
                capacity_bytes: 0x2000,
                mapping: CartridgeRamMapping::SegaMapperSlot2,
                mapped_window_bytes: 0x4000,
                bank_size_bytes: 0x2000,
                mirrored: true,
            }),
            status: ProfileStatus::Verified,
            board: None,
            evidence: vec!["catalog unit test".to_owned()],
        }
    }

    fn synthetic_dat(identity: &CartridgeIdentity, crc32: u32) -> NoIntroDat {
        NoIntroDat {
            name: Some("Synthetic DAT".to_owned()),
            version: Some("test".to_owned()),
            entries: vec![DatGameEntry {
                name: "Synthetic Cartridge".to_owned(),
                roms: vec![DatRomEntry {
                    name: "this-filename-is-not-the-identity.sms".to_owned(),
                    size_bytes: identity.size_bytes(),
                    crc32,
                    md5: None,
                    sha1: identity.sha1_hex(),
                    status: Some("verified".to_owned()),
                }],
            }],
        }
    }

    #[test]
    fn lookup_uses_content_hash_and_returns_the_explicit_profile() {
        let identity = CartridgeIdentity::from_bytes(b"abc");
        let profile = test_battery_profile();
        let catalog = CartridgeCatalog::from_dat(
            synthetic_dat(&identity, identity.crc32()),
            vec![KnownCartridgeProfile::new(
                identity.sha1_hex(),
                profile.clone(),
            )],
        );

        let matched = catalog.lookup(&identity).unwrap().unwrap();
        assert_eq!(matched.game.name, "Synthetic Cartridge");
        assert_eq!(matched.rom.name, "this-filename-is-not-the-identity.sms");
        assert_eq!(matched.profile, Some(&profile));
        assert_eq!(
            matched
                .compatibility
                .map(|metadata| metadata.sms_vdp_requirement),
            Some(SmsVdpRequirement::Unknown)
        );
        assert!(catalog.lookup_runtime_profile(&identity).unwrap().is_some());
    }

    #[test]
    fn lookup_exposes_compatibility_metadata_separately_from_mapper_and_ram() {
        let identity = CartridgeIdentity::from_bytes(b"abc");
        let profile = test_battery_profile();
        let compatibility = CartridgeCompatibilityMetadata::new(
            SmsVdpRequirement::Sms2Required,
            vec!["independent VDP evidence".to_owned()],
        );
        let catalog = CartridgeCatalog::from_dat(
            synthetic_dat(&identity, identity.crc32()),
            vec![KnownCartridgeProfile::with_compatibility(
                identity.sha1_hex(),
                profile.clone(),
                compatibility,
            )],
        );

        let matched = catalog.lookup(&identity).unwrap().unwrap();
        assert_eq!(matched.profile, Some(&profile));
        let metadata = matched.compatibility.unwrap();
        assert_eq!(metadata.sms_vdp_requirement, SmsVdpRequirement::Sms2Required);
        assert_eq!(metadata.evidence.len(), 1);
        assert_eq!(metadata.evidence[0], "independent VDP evidence");
        assert!(catalog.lookup_runtime_profile(&identity).unwrap().is_some());
    }

    #[test]
    fn matching_sha1_with_inconsistent_crc_or_size_is_rejected() {
        let identity = CartridgeIdentity::from_bytes(b"abc");
        let catalog =
            CartridgeCatalog::from_dat(synthetic_dat(&identity, identity.crc32() ^ 1), Vec::new());

        assert!(matches!(
            catalog.lookup(&identity),
            Err(CatalogLookupError::DatFingerprintMismatch { .. })
        ));
    }

    #[test]
    fn every_known_sms_profile_has_an_exact_project_dat_entry() {
        let dat = NoIntroDat::sms_default().unwrap();
        for profile in sms_profiles() {
            assert!(
                dat.find_sha1(&profile.sha1).is_some(),
                "profile SHA-1 {} is absent from the project DAT",
                profile.sha1
            );
        }
    }

    #[test]
    fn shared_sms_catalog_reuses_the_parsed_dat_and_profiles() {
        let first = CartridgeCatalog::sms_shared().unwrap();
        let second = CartridgeCatalog::sms_shared().unwrap();

        assert!(std::ptr::eq(first, second));
    }

    #[test]
    fn cosmic_spacehead_exact_identity_requires_sms2_and_keeps_mapper_separate() {
        let catalog = CartridgeCatalog::sms_default().unwrap();
        let sha1 = "f46f716dd34a1a5013a2d8a59769f6ef7536a567";
        let (_, rom) = catalog.dat().find_sha1(sha1).expect("Cosmic Spacehead DAT entry");

        assert_eq!(rom.size_bytes, 262_144);
        assert_eq!(rom.crc32, 0x2982_2980);
        assert_eq!(rom.sha1, sha1);

        let known = catalog
            .profiles()
            .iter()
            .find(|entry| entry.sha1 == sha1)
            .expect("Cosmic Spacehead compatibility profile");
        assert_eq!(known.profile.mapper, CartridgeMapper::Codemasters);
        assert_eq!(
            known.compatibility.sms_vdp_requirement,
            SmsVdpRequirement::Sms2Required
        );
        assert!(known.profile.ram.is_none());
        assert!(!known.profile.is_runtime_usable());
        assert_eq!(known.compatibility.evidence.len(), 3);
    }

    #[test]
    fn smurfs_brazil_identity_is_absent_from_dat_and_remains_uncharacterized() {
        let catalog = CartridgeCatalog::sms_default().unwrap();
        let sha1 = "15b882ad3263cdc1e78c732d96f9bb18e25f4743";

        assert!(catalog.dat().find_sha1(sha1).is_none());
        assert!(!catalog.profiles().iter().any(|entry| entry.sha1 == sha1));
    }

    #[test]
    fn duplicate_dat_sha1_is_rejected_instead_of_selecting_first_entry() {
        let identity = CartridgeIdentity::from_bytes(b"abc");
        let mut dat = synthetic_dat(&identity, identity.crc32());
        dat.entries.push(dat.entries[0].clone());
        let catalog = CartridgeCatalog::from_dat(dat, Vec::new());

        assert!(matches!(
            catalog.lookup(&identity),
            Err(CatalogLookupError::AmbiguousDatIdentity { entries: 2, .. })
        ));
    }

    #[test]
    fn duplicate_hardware_profiles_for_one_sha1_are_rejected() {
        let identity = CartridgeIdentity::from_bytes(b"abc");
        let profile = test_battery_profile();
        let catalog = CartridgeCatalog::from_dat(
            synthetic_dat(&identity, identity.crc32()),
            vec![
                KnownCartridgeProfile::new(identity.sha1_hex(), profile.clone()),
                KnownCartridgeProfile::new(identity.sha1_hex(), profile),
            ],
        );

        assert!(matches!(
            catalog.lookup(&identity),
            Err(CatalogLookupError::AmbiguousHardwareProfile { profiles: 2, .. })
        ));
    }

    #[test]
    fn eight_commercial_sram_titles_have_verified_8k_profiles() {
        let catalog = CartridgeCatalog::sms_default().unwrap();
        let expected = [
            "fbda0486b393708a89756bb57d116ad6007484e4", // Golden Axe Warrior
            "d0b964dd7cd8ccdd730de4d8e4bb2e87bea7686e", // Golfamania
            "f952406bca4918ee91a89b27e949e224eae96d85", // Miracle Warriors
            "6bad7176011dd4bbd007498d167399daacde173d", // Monopoly Rev A
            "8762239c339a084dfb8443cc38515301476bde28", // Penguin Land
            "257ca76ebcd54c75a414ca7ce968fa59ea42f150", // Phantasy Star USA/Europe
            "a90e21e5961bcf2e10b715a009c04e7c2017a3b1", // Ultima IV
            "e73e836c353543e9f48315410b0d72278899ff59", // Ys: The Vanished Omens
        ];

        for sha1 in expected {
            let profile = catalog
                .profiles()
                .iter()
                .find(|entry| entry.sha1 == sha1)
                .unwrap_or_else(|| panic!("missing commercial SRAM profile {sha1}"));
            assert_eq!(profile.profile.status, ProfileStatus::Verified);
            assert_eq!(
                profile.profile.ram.as_ref().map(|ram| ram.kind),
                Some(CartridgeRamKind::BatteryBackedSram)
            );
            assert_eq!(
                profile.profile.ram.as_ref().map(|ram| ram.capacity_bytes),
                Some(0x2000)
            );
        }
    }

    #[test]
    fn default_catalog_keeps_flight_of_pigarus_blocked() {
        let profile = flight_of_pigarus_blocked();
        assert_eq!(profile.status, ProfileStatus::Blocked);
        assert!(!profile.is_runtime_usable());
        assert_eq!(
            profile.ram.as_ref().map(|ram| ram.kind),
            Some(CartridgeRamKind::FeRam)
        );
    }

    #[test]
    fn catalog_does_not_assign_profiles_to_unknown_roms() {
        let catalog = CartridgeCatalog::sms_default().unwrap();
        let identity = CartridgeIdentity::from_bytes(b"unknown cartridge image");
        assert!(catalog.lookup(&identity).unwrap().is_none());
    }

    #[test]
    fn volatile_ram_profiles_do_not_request_persistent_storage() {
        let catalog = CartridgeCatalog::sms_default().unwrap();
        let profile = catalog
            .profiles()
            .iter()
            .find(|entry| entry.sha1 == "60e2b6ec69d73dd73c1ef846634942c81800655b")
            .unwrap();
        assert_eq!(
            profile.profile.ram.as_ref().map(|ram| ram.kind),
            Some(CartridgeRamKind::VolatileSram)
        );
        assert!(!profile.profile.requires_persistent_store());
    }
}
