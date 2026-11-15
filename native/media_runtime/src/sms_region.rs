//! Deterministic SMS console-region preference and cartridge-region resolution.
//!
//! The selected territory is distinct from the VDP video standard. SMS hardware
//! exposes Japan-vs-Export and PAL-vs-NTSC/PAL-M; Korea and Japan share the
//! Japanese/NTSC hardware signal and remain distinct software preferences.

use crate::{CartridgeCatalog, CartridgeIdentity};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SmsRegion {
    Brazil,
    Usa,
    Europe,
    Japan,
    Korea,
}

impl SmsRegion {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Brazil => "brazil",
            Self::Usa => "usa",
            Self::Europe => "europe",
            Self::Japan => "japan",
            Self::Korea => "korea",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Brazil => "Brasil",
            Self::Usa => "USA",
            Self::Europe => "Europa",
            Self::Japan => "Japão",
            Self::Korea => "Coreia",
        }
    }

    fn automatic_priority() -> [Self; 5] {
        [
            Self::Brazil,
            Self::Usa,
            Self::Europe,
            Self::Japan,
            Self::Korea,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmsRegionPreference {
    Automatic,
    Manual(SmsRegion),
}

impl SmsRegionPreference {
    pub fn parse(value: &str) -> Result<Self, &'static str> {
        match value {
            "auto" | "automatic" => Ok(Self::Automatic),
            "brazil" => Ok(Self::Manual(SmsRegion::Brazil)),
            "usa" => Ok(Self::Manual(SmsRegion::Usa)),
            "europe" => Ok(Self::Manual(SmsRegion::Europe)),
            "japan" => Ok(Self::Manual(SmsRegion::Japan)),
            "korea" => Ok(Self::Manual(SmsRegion::Korea)),
            _ => Err("SMS region must be auto, brazil, usa, europe, japan or korea"),
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Automatic => "auto",
            Self::Manual(region) => region.key(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmsRegionResolutionSource {
    ManualPreference,
    DatWorldDefaultsToUsa,
    DatRegionPriority,
    DatSingleRegion,
    CartridgeHeader,
    SafeDefault,
    DatLookupUnavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmsRegionResolution {
    pub region: SmsRegion,
    pub source: SmsRegionResolutionSource,
    pub dat_title: Option<String>,
    pub detected_regions: Vec<SmsRegion>,
}

pub fn resolve_sms_region(
    preference: SmsRegionPreference,
    rom: &[u8],
    catalog: &CartridgeCatalog,
) -> SmsRegionResolution {
    if let SmsRegionPreference::Manual(region) = preference {
        return SmsRegionResolution {
            region,
            source: SmsRegionResolutionSource::ManualPreference,
            dat_title: None,
            detected_regions: Vec::new(),
        };
    }

    let identity = CartridgeIdentity::from_bytes(rom);
    let (dat_title, dat_regions, lookup_unavailable) = match catalog.lookup(&identity) {
        Ok(Some(matched)) => {
            let (world, regions) = parse_dat_regions(&matched.game.name);
            if world {
                return SmsRegionResolution {
                    region: SmsRegion::Usa,
                    source: SmsRegionResolutionSource::DatWorldDefaultsToUsa,
                    dat_title: Some(matched.game.name.clone()),
                    detected_regions: regions,
                };
            }
            (Some(matched.game.name.clone()), regions, false)
        }
        Ok(None) => (None, Vec::new(), false),
        Err(_) => (None, Vec::new(), true),
    };

    if dat_regions.len() > 1
        && let Some(region) = SmsRegion::automatic_priority()
            .into_iter()
            .find(|candidate| dat_regions.contains(candidate))
    {
        return SmsRegionResolution {
            region,
            source: SmsRegionResolutionSource::DatRegionPriority,
            dat_title,
            detected_regions: dat_regions,
        };
    }

    // Korea is a distinct software/firmware identity but shares the Japan
    // territory bit. Exact DAT evidence must therefore win over a Japan header.
    if dat_regions.as_slice() == [SmsRegion::Korea] {
        return SmsRegionResolution {
            region: SmsRegion::Korea,
            source: SmsRegionResolutionSource::DatSingleRegion,
            dat_title,
            detected_regions: dat_regions,
        };
    }

    // Preserve the established PR #287 precedence: a Japan TMR SEGA header
    // selects Japan; an Export header is refined by a single exact DAT region.
    match sms_header_region(rom) {
        Some(SmsRegion::Japan) => SmsRegionResolution {
            region: SmsRegion::Japan,
            source: SmsRegionResolutionSource::CartridgeHeader,
            dat_title,
            detected_regions: dat_regions,
        },
        Some(SmsRegion::Usa) => {
            let region = dat_regions.first().copied().unwrap_or(SmsRegion::Usa);
            SmsRegionResolution {
                region,
                source: if dat_regions.is_empty() {
                    SmsRegionResolutionSource::CartridgeHeader
                } else {
                    SmsRegionResolutionSource::DatSingleRegion
                },
                dat_title,
                detected_regions: dat_regions,
            }
        }
        Some(region) => SmsRegionResolution {
            region,
            source: SmsRegionResolutionSource::CartridgeHeader,
            dat_title,
            detected_regions: dat_regions,
        },
        None => {
            let region = dat_regions.first().copied().unwrap_or(SmsRegion::Usa);
            SmsRegionResolution {
                region,
                source: if dat_regions.is_empty() {
                    if lookup_unavailable {
                        SmsRegionResolutionSource::DatLookupUnavailable
                    } else {
                        SmsRegionResolutionSource::SafeDefault
                    }
                } else {
                    SmsRegionResolutionSource::DatSingleRegion
                },
                dat_title,
                detected_regions: dat_regions,
            }
        }
    }
}

fn parse_dat_regions(title: &str) -> (bool, Vec<SmsRegion>) {
    let mut world = false;
    let mut found = Vec::new();
    let mut remaining = title;
    while let Some(start) = remaining.find('(') {
        let after_open = &remaining[start + 1..];
        let Some(end) = after_open.find(')') else {
            break;
        };
        for token in after_open[..end].split([',', ';', '/']) {
            let normalized = token
                .trim()
                .chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>();
            match normalized.as_str() {
                "world" | "w" => world = true,
                "brazil" | "br" => found.push(SmsRegion::Brazil),
                "usa" | "us" | "unitedstates" => found.push(SmsRegion::Usa),
                "europe" | "eur" | "eu" => found.push(SmsRegion::Europe),
                "japan" | "jp" | "jpn" => found.push(SmsRegion::Japan),
                "korea" | "kr" | "kor" => found.push(SmsRegion::Korea),
                _ => {}
            }
        }
        remaining = &after_open[end + 1..];
    }
    found.dedup();
    (world, found)
}

fn sms_header_region(rom: &[u8]) -> Option<SmsRegion> {
    const HEADER: &[u8; 8] = b"TMR SEGA";
    [0x7FF0_usize, 0x3FF0, 0x1FF0]
        .into_iter()
        .find_map(|offset| {
            let header = rom.get(offset..offset + 16)?;
            if &header[..8] != HEADER {
                return None;
            }
            match header[15] >> 4 {
                0x3 => Some(SmsRegion::Japan),
                0x4 => Some(SmsRegion::Usa),
                _ => None,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::{
        SmsRegion, SmsRegionPreference, SmsRegionResolutionSource, parse_dat_regions,
        resolve_sms_region, sms_header_region,
    };
    use crate::{CartridgeCatalog, DatGameEntry, DatRomEntry, NoIntroDat};

    fn catalog_for(name: &str, rom: &[u8]) -> CartridgeCatalog {
        let identity = crate::CartridgeIdentity::from_bytes(rom);
        CartridgeCatalog::from_dat(
            NoIntroDat {
                name: Some("Synthetic SMS DAT".to_owned()),
                version: Some("test".to_owned()),
                entries: vec![DatGameEntry {
                    name: name.to_owned(),
                    roms: vec![DatRomEntry {
                        name: "arbitrary.sms".to_owned(),
                        size_bytes: identity.size_bytes(),
                        crc32: identity.crc32(),
                        md5: None,
                        sha1: identity.sha1_hex(),
                        status: None,
                    }],
                }],
            },
            Vec::new(),
        )
    }

    fn header_rom(code: u8) -> Vec<u8> {
        let mut rom = vec![0xFF; 0x8000];
        rom[0x7FF0..0x7FF8].copy_from_slice(b"TMR SEGA");
        rom[0x7FFF] = code << 4;
        rom
    }

    #[test]
    fn parses_multi_region_names_and_applies_required_priority() {
        let (world, regions) = parse_dat_regions("Example (Europe, Brazil, USA)");
        assert!(!world);
        assert_eq!(
            regions,
            vec![SmsRegion::Europe, SmsRegion::Brazil, SmsRegion::Usa]
        );

        let rom = header_rom(3);
        let catalog = catalog_for("Example (Europe, Brazil, USA)", &rom);
        let resolved = resolve_sms_region(SmsRegionPreference::Automatic, &rom, &catalog);
        assert_eq!(resolved.region, SmsRegion::Brazil);
        assert_eq!(
            resolved.source,
            SmsRegionResolutionSource::DatRegionPriority
        );
    }

    #[test]
    fn world_releases_always_use_usa_in_automatic_mode() {
        let rom = header_rom(3);
        let catalog = catalog_for("Example (World)", &rom);
        let resolved = resolve_sms_region(SmsRegionPreference::Automatic, &rom, &catalog);
        assert_eq!(resolved.region, SmsRegion::Usa);
        assert_eq!(
            resolved.source,
            SmsRegionResolutionSource::DatWorldDefaultsToUsa
        );
    }

    #[test]
    fn manual_preference_overrides_dat_and_header() {
        let rom = header_rom(3);
        let catalog = catalog_for("Example (Brazil, Europe)", &rom);
        let resolved = resolve_sms_region(
            SmsRegionPreference::Manual(SmsRegion::Korea),
            &rom,
            &catalog,
        );
        assert_eq!(resolved.region, SmsRegion::Korea);
        assert_eq!(resolved.source, SmsRegionResolutionSource::ManualPreference);
    }

    #[test]
    fn valid_sms_header_is_used_only_when_dat_has_no_region() {
        for (code, expected) in [(3, SmsRegion::Japan), (4, SmsRegion::Usa)] {
            let rom = header_rom(code);
            let catalog = catalog_for("Example (Unl)", &rom);
            let resolved = resolve_sms_region(SmsRegionPreference::Automatic, &rom, &catalog);
            assert_eq!(resolved.region, expected);
            assert_eq!(resolved.source, SmsRegionResolutionSource::CartridgeHeader);
        }
    }

    #[test]
    fn korea_is_distinct_from_japan_even_though_header_cannot_distinguish_them() {
        let (world, regions) = parse_dat_regions("Example (Korea)");
        assert!(!world);
        assert_eq!(regions, vec![SmsRegion::Korea]);
        assert_eq!(sms_header_region(&header_rom(3)), Some(SmsRegion::Japan));
    }

    #[test]
    fn preserves_pr287_header_precedence_and_refines_export_from_dat() {
        let japan_header = header_rom(3);
        let europe_catalog = catalog_for("Example (Europe)", &japan_header);
        let japan = resolve_sms_region(
            SmsRegionPreference::Automatic,
            &japan_header,
            &europe_catalog,
        );
        assert_eq!(japan.region, SmsRegion::Japan);
        assert_eq!(japan.source, SmsRegionResolutionSource::CartridgeHeader);

        let export_header = header_rom(4);
        let europe_catalog = catalog_for("Example (Europe)", &export_header);
        let europe = resolve_sms_region(
            SmsRegionPreference::Automatic,
            &export_header,
            &europe_catalog,
        );
        assert_eq!(europe.region, SmsRegion::Europe);
        assert_eq!(europe.source, SmsRegionResolutionSource::DatSingleRegion);
    }

    #[test]
    fn korea_dat_identity_is_not_collapsed_into_japan_header() {
        let rom = header_rom(3);
        let catalog = catalog_for("Example (Korea)", &rom);
        let resolved = resolve_sms_region(SmsRegionPreference::Automatic, &rom, &catalog);
        assert_eq!(resolved.region, SmsRegion::Korea);
        assert_eq!(resolved.source, SmsRegionResolutionSource::DatSingleRegion);
    }

    #[test]
    fn unknown_and_unmatched_roms_use_documented_usa_safe_default() {
        let rom = b"unknown image";
        let catalog = CartridgeCatalog::sms_default().unwrap();
        let resolved = resolve_sms_region(SmsRegionPreference::Automatic, rom, &catalog);
        assert_eq!(resolved.region, SmsRegion::Usa);
        assert_eq!(resolved.source, SmsRegionResolutionSource::SafeDefault);
    }

    #[test]
    fn manual_preference_parser_accepts_only_supported_regions() {
        assert_eq!(
            SmsRegionPreference::parse("auto").unwrap(),
            SmsRegionPreference::Automatic
        );
        assert_eq!(
            SmsRegionPreference::parse("brazil").unwrap(),
            SmsRegionPreference::Manual(SmsRegion::Brazil)
        );
        assert_eq!(
            SmsRegionPreference::parse("korea").unwrap(),
            SmsRegionPreference::Manual(SmsRegion::Korea)
        );
        assert!(SmsRegionPreference::parse("unl").is_err());
    }
}
