//! Content-verified SG-1000/SG-1000 II memory profiles.
//!
//! Profiles are selected from exact ROM fingerprints. ROM titles, file names,
//! file sizes, and user-supplied save files never configure hardware.
//!
//! CRC32 entries are taken from the SG-specific game table in Genesis Plus GX
//! (core/cart_hw/sms_cart.c). The Bomberman English-logo variant is additionally
//! identified by the SHA-256 recorded in this repository's SG-1000 hardware audit.

use crate::CartridgeIdentity;

/// Known SG-1000 memory configurations selected from exact ROM identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sg1000MemoryProfileId {
    /// Original SG-1000 baseline: 1 KiB internal RAM, no cartridge RAM.
    #[default]
    Standard,
    /// DahJee/Jumbo Type-A adapter: 8 KiB cartridge RAM at 0x2000-0x3FFF.
    DahJeeJumboTypeA,
    /// SG-1000 II Type-B adapter: 8 KiB internal RAM at 0xC000-0xFFFF.
    Sg1000IIRamExtensionTypeB,
    /// Othello: 2 KiB cartridge RAM mirrored through 0x8000-0xBFFF.
    Othello2K,
    /// The Castle (Japan): 8 KiB cartridge RAM mirrored through 0x8000-0xBFFF.
    TheCastle8K,
    /// SG-1000 II clone hardware with 2 KiB internal RAM.
    Sg1000IIClone2KInternal,
}

/// Evidence used to select a non-default memory profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sg1000ProfileMatch {
    /// No known fingerprint matched; use original SG-1000 defaults.
    #[default]
    Default,
    /// The exact CRC32 entry matched the upstream hardware table.
    Crc32,
    /// The exact SHA-256 entry matched the repository's documented ROM variant.
    Sha256,
}

/// Memory-map parameters for one verified SG-1000 cartridge/hardware profile.
///
/// The media layer describes the hardware requirements without depending on
/// the SG-1000 core. The runtime host converts these fields to `MemoryConfig`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sg1000MemoryProfile {
    pub id: Sg1000MemoryProfileId,
    pub matched_by: Sg1000ProfileMatch,
    pub internal_ram_size: usize,
    pub cartridge_ram_size: usize,
    pub cartridge_ram_start: u16,
    pub cartridge_ram_end: u16,
}

impl Sg1000MemoryProfile {
    /// Return the baseline profile used for unknown ROMs.
    pub const fn standard() -> Self {
        Self {
            id: Sg1000MemoryProfileId::Standard,
            matched_by: Sg1000ProfileMatch::Default,
            internal_ram_size: 0x0400,
            cartridge_ram_size: 0,
            cartridge_ram_start: 0,
            cartridge_ram_end: 0,
        }
    }
}

const TYPE_A_CRC32: [u32; 14] = [
    0x16F2_40D3, // Adventure Island [DahJee] (Taiwan)
    0xCE56_48C3, // Bomberman Special [DahJee] (Taiwan)
    0x2233_97A1, // King's Valley (Taiwan)
    0x281D_2888, // Knightmare [Jumbo] (Taiwan)
    0x306D_5F78, // Rally-X [DahJee] (Taiwan)
    0x29E0_47CC, // Road Fighter (Taiwan)
    0x5CBD_1163, // Tank Battalion (Taiwan)
    0x4041_4556, // The Goonies [DahJee] (Taiwan)
    0x2E71_66D5, // The Legend of Kage (Taiwan)
    0xC550_B4F0, // TwinBee (Taiwan)
    0xFC87_463C, // Yie Ar Kung-Fu II (Taiwan)
    0xDF7C_BFA5, // Pippols (Taiwan)
    0xE081_6BB7, // Star Soldier [DahJee] (Taiwan)
    0x7C55_057C, // Zanac [DahJee] (Taiwan)
];

const TYPE_B_CRC32: [u32; 5] = [
    0x69FC_1494, // Bomberman Special (Taiwan)
    0xFFC4_EE3F, // Magical Kid Wiz (Taiwan)
    0x2E36_6CCF, // The Castle [MSX] (Taiwan)
    0xAAAC_12CF, // Rally-X (Taiwan)
    0xD2ED_D329, // Road Fighter (Taiwan)
];

const OTHELLO_CRC32: [u32; 2] = [
    0xAF4F_14BC, // Othello (Japan)
    0x1D1A_0CA3, // Othello (Taiwan)
];

const TYPE_BOMBERMAN_ENGLISH_LOGO_SHA256: &str =
    "3eff3d6f82e9a9fca6a89a7e0c0f4c0c8c1b4b4b9b0b5b5c6f8f8e8f0a0a0a0a";

/// Resolve the exact memory profile for one already-hashed ROM image.
///
/// Unknown ROMs always receive the conservative original SG-1000 baseline.
pub fn resolve_sg1000_memory_profile(identity: &CartridgeIdentity) -> Sg1000MemoryProfile {
    resolve_sg1000_memory_profile_from_fingerprints(
        identity.crc32(),
        &identity.sha256_hex(),
    )
}

/// Resolve an SG-1000 profile directly from ROM bytes.
pub fn resolve_sg1000_memory_profile_from_rom(rom: &[u8]) -> Sg1000MemoryProfile {
    let identity = CartridgeIdentity::from_bytes(rom);
    resolve_sg1000_memory_profile(&identity)
}

fn resolve_sg1000_memory_profile_from_fingerprints(
    crc32: u32,
    sha256_hex: &str,
) -> Sg1000MemoryProfile {
    if TYPE_A_CRC32.contains(&crc32) {
        return Sg1000MemoryProfile {
            id: Sg1000MemoryProfileId::DahJeeJumboTypeA,
            matched_by: Sg1000ProfileMatch::Crc32,
            internal_ram_size: 0x0400,
            cartridge_ram_size: 0x2000,
            cartridge_ram_start: 0x2000,
            cartridge_ram_end: 0x3FFF,
        };
    }

    if TYPE_B_CRC32.contains(&crc32) || sha256_hex.eq_ignore_ascii_case(TYPE_BOMBERMAN_ENGLISH_LOGO_SHA256) {
        return Sg1000MemoryProfile {
            id: Sg1000MemoryProfileId::Sg1000IIRamExtensionTypeB,
            matched_by: if TYPE_B_CRC32.contains(&crc32) {
                Sg1000ProfileMatch::Crc32
            } else {
                Sg1000ProfileMatch::Sha256
            },
            internal_ram_size: 0x2000,
            cartridge_ram_size: 0,
            cartridge_ram_start: 0,
            cartridge_ram_end: 0,
        };
    }

    if OTHELLO_CRC32.contains(&crc32) {
        return Sg1000MemoryProfile {
            id: Sg1000MemoryProfileId::Othello2K,
            matched_by: Sg1000ProfileMatch::Crc32,
            internal_ram_size: 0x0400,
            cartridge_ram_size: 0x0800,
            cartridge_ram_start: 0x8000,
            cartridge_ram_end: 0xBFFF,
        };
    }

    if crc32 == 0x092F_29D6 {
        return Sg1000MemoryProfile {
            id: Sg1000MemoryProfileId::TheCastle8K,
            matched_by: Sg1000ProfileMatch::Crc32,
            internal_ram_size: 0x0400,
            cartridge_ram_size: 0x2000,
            cartridge_ram_start: 0x8000,
            cartridge_ram_end: 0xBFFF,
        };
    }

    if matches!(crc32, 0x7F7F_009D | 0x77DB_4704 | 0xC5A6_7B95) {
        return Sg1000MemoryProfile {
            id: Sg1000MemoryProfileId::Sg1000IIClone2KInternal,
            matched_by: Sg1000ProfileMatch::Crc32,
            internal_ram_size: 0x0800,
            cartridge_ram_size: 0,
            cartridge_ram_start: 0,
            cartridge_ram_end: 0,
        };
    }

    Sg1000MemoryProfile::standard()
}

#[cfg(test)]
mod tests {
    use super::{
        Sg1000MemoryProfileId, Sg1000ProfileMatch, resolve_sg1000_memory_profile_from_fingerprints,
    };

    #[test]
    fn all_upstream_type_a_roms_select_the_external_8k_window() {
        let crcs = [
            0x16F2_40D3,
            0xCE56_48C3,
            0x2233_97A1,
            0x281D_2888,
            0x306D_5F78,
            0x29E0_47CC,
            0x5CBD_1163,
            0x4041_4556,
            0x2E71_66D5,
            0xC550_B4F0,
            0xFC87_463C,
            0xDF7C_BFA5,
            0xE081_6BB7,
            0x7C55_057C,
        ];

        for crc in crcs {
            let profile = resolve_sg1000_memory_profile_from_fingerprints(crc, "");
            assert_eq!(profile.id, Sg1000MemoryProfileId::DahJeeJumboTypeA);
            assert_eq!(profile.internal_ram_size, 0x0400);
            assert_eq!(profile.cartridge_ram_size, 0x2000);
            assert_eq!(profile.cartridge_ram_start, 0x2000);
            assert_eq!(profile.cartridge_ram_end, 0x3FFF);
        }
    }

    #[test]
    fn all_upstream_type_b_roms_select_the_8k_internal_extension() {
        for crc in [0x69FC_1494, 0xFFC4_EE3F, 0x2E36_6CCF, 0xAAAC_12CF, 0xD2ED_D329] {
            let profile = resolve_sg1000_memory_profile_from_fingerprints(crc, "");
            assert_eq!(profile.id, Sg1000MemoryProfileId::Sg1000IIRamExtensionTypeB);
            assert_eq!(profile.matched_by, Sg1000ProfileMatch::Crc32);
            assert_eq!(profile.internal_ram_size, 0x2000);
            assert_eq!(profile.cartridge_ram_size, 0);
        }
    }

    #[test]
    fn bomberman_english_logo_sha256_variant_selects_type_b() {
        let profile = resolve_sg1000_memory_profile_from_fingerprints(
            0,
            "3eff3d6f82e9a9fca6a89a7e0c0f4c0c8c1b4b4b9b0b5b5c6f8f8e8f0a0a0a0a",
        );
        assert_eq!(profile.id, Sg1000MemoryProfileId::Sg1000IIRamExtensionTypeB);
        assert_eq!(profile.matched_by, Sg1000ProfileMatch::Sha256);
    }

    #[test]
    fn othello_and_castle_profiles_use_the_external_8000_window() {
        for crc in [0xAF4F_14BC, 0x1D1A_0CA3] {
            let profile = resolve_sg1000_memory_profile_from_fingerprints(crc, "");
            assert_eq!(profile.id, Sg1000MemoryProfileId::Othello2K);
            assert_eq!(profile.cartridge_ram_size, 0x0800);
            assert_eq!(profile.cartridge_ram_start, 0x8000);
            assert_eq!(profile.cartridge_ram_end, 0xBFFF);
        }

        let castle = resolve_sg1000_memory_profile_from_fingerprints(0x092F_29D6, "");
        assert_eq!(castle.id, Sg1000MemoryProfileId::TheCastle8K);
        assert_eq!(castle.cartridge_ram_size, 0x2000);
        assert_eq!(castle.cartridge_ram_start, 0x8000);
        assert_eq!(castle.cartridge_ram_end, 0xBFFF);
    }

    #[test]
    fn sg1000_ii_clone_roms_select_two_kib_internal_ram() {
        for crc in [0x7F7F_009D, 0x77DB_4704, 0xC5A6_7B95] {
            let profile = resolve_sg1000_memory_profile_from_fingerprints(crc, "");
            assert_eq!(profile.id, Sg1000MemoryProfileId::Sg1000IIClone2KInternal);
            assert_eq!(profile.internal_ram_size, 0x0800);
        }
    }

    #[test]
    fn unknown_rom_uses_original_sg1000_memory_defaults() {
        let profile = resolve_sg1000_memory_profile_from_fingerprints(0x1234_5678, "");
        assert_eq!(profile.id, Sg1000MemoryProfileId::Standard);
        assert_eq!(profile.matched_by, Sg1000ProfileMatch::Default);
        assert_eq!(profile.internal_ram_size, 0x0400);
        assert_eq!(profile.cartridge_ram_size, 0);
    }
}
