//! ROM-hash based SG-1000 cartridge and RAM-expansion profiles.
//!
//! Unknown ROMs deliberately retain the baseline 1 KiB internal RAM map.
//! The database is a bounded compatibility list, not a generic mapper detector.

use sega_sg1000_core::MemoryConfig;
use sha2::{Digest, Sha256};

/// Hardware/RAM profile selected by an exact known ROM hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sg1000CartridgeProfile {
    /// Standard SG-1000: 1 KiB internal RAM mirrored at C000h–FFFFh.
    Standard,
    /// SG-1000 II Type-A adapter: 8 KiB external RAM at 2000h–3FFFh.
    TypeAExternalRam8K,
    /// SG-1000 II Type-B adapter: 8 KiB internal RAM at C000h–FFFFh.
    TypeBInternalRam8K,
    /// SG-1000 II clone hardware: 2 KiB internal RAM mirrored at C000h–FFFFh.
    Sg1000IiCloneInternalRam2K,
    /// Othello: 2 KiB external RAM mirrored at 8000h–BFFFh.
    OthelloExternalRam2K,
    /// The Castle (Japan): 8 KiB external RAM mirrored at 8000h–BFFFh.
    TheCastleExternalRam8K,
}

impl Sg1000CartridgeProfile {
    /// Return the explicit memory map associated with this profile.
    pub const fn memory_config(self) -> MemoryConfig {
        match self {
            Self::Standard => MemoryConfig {
                internal_ram_size: 0x0400,
                cartridge_ram_size: 0,
                cartridge_ram_start: 0,
                cartridge_ram_end: 0,
            },
            Self::TypeAExternalRam8K => MemoryConfig {
                internal_ram_size: 0x0400,
                cartridge_ram_size: 0x2000,
                cartridge_ram_start: 0x2000,
                cartridge_ram_end: 0x3FFF,
            },
            Self::Sg1000IiCloneInternalRam2K => MemoryConfig {
                internal_ram_size: 0x0800,
                cartridge_ram_size: 0,
                cartridge_ram_start: 0,
                cartridge_ram_end: 0,
            },
            Self::TypeBInternalRam8K => MemoryConfig {
                internal_ram_size: 0x2000,
                cartridge_ram_size: 0,
                cartridge_ram_start: 0,
                cartridge_ram_end: 0,
            },
            Self::OthelloExternalRam2K => MemoryConfig {
                internal_ram_size: 0x0400,
                cartridge_ram_size: 0x0800,
                cartridge_ram_start: 0x8000,
                cartridge_ram_end: 0xBFFF,
            },
            Self::TheCastleExternalRam8K => MemoryConfig {
                internal_ram_size: 0x0400,
                cartridge_ram_size: 0x2000,
                cartridge_ram_start: 0x8000,
                cartridge_ram_end: 0xBFFF,
            },
        }
    }
}

/// Identify known SG-1000 hardware profiles by CRC-32 and the documented
/// Bomberman English-logo SHA-256 exception. ROM size/title are never heuristics.
pub fn identify_cartridge_profile(rom: &[u8]) -> Sg1000CartridgeProfile {
    let crc = crc32fast::hash(rom);
    let digest: [u8; 32] = Sha256::digest(rom).into();
    profile_for_hashes(crc, &digest)
}

fn profile_for_hashes(crc: u32, sha256: &[u8; 32]) -> Sg1000CartridgeProfile {
    match crc {
        // DahJee/Jumbo Type-A 8 KiB RAM adapter (Genesis Plus GX game list).
        0x16F2_40D3 | 0xCE56_48C3 | 0x2233_97A1 | 0x281D_2888 | 0x306D_5F78 | 0x29E0_47CC
        | 0x5CBD_1163 | 0x4041_4556 | 0x2E71_66D5 | 0xC550_B4F0 | 0xFC87_463C | 0xDF7C_BFA5
        | 0xE081_6BB7 | 0x7C55_057C => Sg1000CartridgeProfile::TypeAExternalRam8K,

        // SG-1000 II Type-B adapter: 8 KiB replaces the internal RAM window.
        0x69FC_1494 | 0xFFC4_EE3F | 0x2E36_6CCF | 0xAAAC_12CF | 0xD2ED_D329 => {
            Sg1000CartridgeProfile::TypeBInternalRam8K
        }

        0x7F7F_009D | 0x77DB_4704 | 0xC5A6_7B95 => {
            Sg1000CartridgeProfile::Sg1000IiCloneInternalRam2K
        }
        0xAF4F_14BC | 0x1D1A_0CA3 => Sg1000CartridgeProfile::OthelloExternalRam2K,
        0x092F_29D6 => Sg1000CartridgeProfile::TheCastleExternalRam8K,
        _ if *sha256 == BOMBERMAN_ENGLISH_SHA256 => Sg1000CartridgeProfile::TypeBInternalRam8K,
        _ => Sg1000CartridgeProfile::Standard,
    }
}

const BOMBERMAN_ENGLISH_SHA256: [u8; 32] = [
    0x3e, 0xff, 0x3d, 0x6f, 0x1f, 0x74, 0x04, 0x1f, 0x7b, 0x73, 0x72, 0x45, 0x79, 0x90, 0x97, 0x8a,
    0xb4, 0x97, 0x24, 0x55, 0x2f, 0xf2, 0x98, 0x5f, 0x34, 0xb7, 0x64, 0x78, 0xcd, 0x91, 0x72, 0x1a,
];

/// Resolve the memory configuration for a ROM without mutating it.
pub fn memory_config_for_rom(rom: &[u8]) -> MemoryConfig {
    identify_cartridge_profile(rom).memory_config()
}

#[cfg(test)]
mod tests {
    use super::{
        BOMBERMAN_ENGLISH_SHA256, Sg1000CartridgeProfile, identify_cartridge_profile,
        profile_for_hashes,
    };

    #[test]
    fn identifies_every_catalogued_ram_expansion_family() {
        let type_a = [
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
        for crc in type_a {
            assert_eq!(
                profile_for_hashes(crc, &[0; 32]),
                Sg1000CartridgeProfile::TypeAExternalRam8K,
                "Type-A CRC {crc:08X}"
            );
        }

        for crc in [
            0x69FC_1494,
            0xFFC4_EE3F,
            0x2E36_6CCF,
            0xAAAC_12CF,
            0xD2ED_D329,
        ] {
            assert_eq!(
                profile_for_hashes(crc, &[0; 32]),
                Sg1000CartridgeProfile::TypeBInternalRam8K,
                "Type-B CRC {crc:08X}"
            );
        }

        for crc in [0x7F7F_009D, 0x77DB_4704, 0xC5A6_7B95] {
            assert_eq!(
                profile_for_hashes(crc, &[0; 32]),
                Sg1000CartridgeProfile::Sg1000IiCloneInternalRam2K,
                "SG-1000 II clone CRC {crc:08X}"
            );
        }

        for crc in [0xAF4F_14BC, 0x1D1A_0CA3] {
            assert_eq!(
                profile_for_hashes(crc, &[0; 32]),
                Sg1000CartridgeProfile::OthelloExternalRam2K
            );
        }
        assert_eq!(
            profile_for_hashes(0x092F_29D6, &[0; 32]),
            Sg1000CartridgeProfile::TheCastleExternalRam8K
        );
    }

    #[test]
    fn identifies_bomberman_english_logo_by_documented_sha256() {
        assert_eq!(
            profile_for_hashes(0, &BOMBERMAN_ENGLISH_SHA256),
            Sg1000CartridgeProfile::TypeBInternalRam8K
        );
    }

    #[test]
    fn unknown_roms_keep_the_standard_ram_map() {
        assert_eq!(
            identify_cartridge_profile(&[0x00; 32]),
            Sg1000CartridgeProfile::Standard
        );
        assert_eq!(
            Sg1000CartridgeProfile::Standard
                .memory_config()
                .internal_ram_size,
            0x0400
        );
    }

    #[test]
    fn profiles_produce_distinct_memory_maps() {
        let type_a = Sg1000CartridgeProfile::TypeAExternalRam8K.memory_config();
        assert_eq!(type_a.internal_ram_size, 0x0400);
        assert_eq!(type_a.cartridge_ram_size, 0x2000);
        assert_eq!(type_a.cartridge_ram_start, 0x2000);
        assert_eq!(type_a.cartridge_ram_end, 0x3FFF);

        let clone = Sg1000CartridgeProfile::Sg1000IiCloneInternalRam2K.memory_config();
        assert_eq!(clone.internal_ram_size, 0x0800);
        assert_eq!(clone.cartridge_ram_size, 0);

        let type_b = Sg1000CartridgeProfile::TypeBInternalRam8K.memory_config();
        assert_eq!(type_b.internal_ram_size, 0x2000);
        assert_eq!(type_b.cartridge_ram_size, 0);

        let othello = Sg1000CartridgeProfile::OthelloExternalRam2K.memory_config();
        assert_eq!(othello.cartridge_ram_size, 0x0800);
        assert_eq!(othello.cartridge_ram_start, 0x8000);
        assert_eq!(othello.cartridge_ram_end, 0xBFFF);

        let castle = Sg1000CartridgeProfile::TheCastleExternalRam8K.memory_config();
        assert_eq!(castle.cartridge_ram_size, 0x2000);
        assert_eq!(castle.cartridge_ram_start, 0x8000);
        assert_eq!(castle.cartridge_ram_end, 0xBFFF);
    }
}
