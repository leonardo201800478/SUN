use serde::{Deserialize, Serialize};

/// Console family to which a cartridge profile applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CartridgeSystem {
    MasterSystem,
}

/// Hardware mapper family. Unknown is intentionally non-executable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CartridgeMapper {
    SmsSega,
    Codemasters,
    Unknown,
}

/// Minimum SMS VDP capability required by a cartridge, independent of region and mapper.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmsVdpRequirement {
    /// No independently supported minimum-VDP declaration is available.
    #[default]
    Unknown,
    /// The cartridge is documented to work with the SMS1 315-5124 VDP and later.
    Sms1Compatible,
    /// The cartridge requires the SMS2 315-5246 extra-height capabilities.
    Sms2Required,
}

/// Per-cartridge compatibility evidence, separate from mapper and RAM metadata.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartridgeCompatibilityMetadata {
    pub sms_vdp_requirement: SmsVdpRequirement,
    pub evidence: Vec<String>,
}

impl CartridgeCompatibilityMetadata {
    pub fn new(sms_vdp_requirement: SmsVdpRequirement, evidence: Vec<String>) -> Self {
        Self {
            sms_vdp_requirement,
            evidence,
        }
    }
}

/// Explicitly selected SMS hardware model. It is never inferred from region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmsHardwareModel {
    Sms1,
    Sms2,
}

impl SmsHardwareModel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sms1 => "sms1",
            Self::Sms2 => "sms2",
        }
    }
}

/// Result of comparing a cartridge requirement with the selected SMS hardware.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmsCompatibilityStatus {
    Compatible,
    Incompatible,
    Unknown,
}

impl SmsCompatibilityStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Incompatible => "incompatible",
            Self::Unknown => "unknown",
        }
    }
}

impl SmsVdpRequirement {
    /// Evaluate compatibility without inferring hardware from ROM region or filename.
    pub const fn evaluate(self, selected: SmsHardwareModel) -> SmsCompatibilityStatus {
        match self {
            Self::Unknown => SmsCompatibilityStatus::Unknown,
            Self::Sms1Compatible => SmsCompatibilityStatus::Compatible,
            Self::Sms2Required => match selected {
                SmsHardwareModel::Sms1 => SmsCompatibilityStatus::Incompatible,
                SmsHardwareModel::Sms2 => SmsCompatibilityStatus::Compatible,
            },
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Sms1Compatible => "sms1_compatible",
            Self::Sms2Required => "sms2_required",
        }
    }
}

/// Run cartridge preparation only after the selected hardware passes the metadata gate.
///
/// Callers must invoke this before taking or replacing the currently active worker.
pub fn preflight_sms_cartridge<T>(
    requirement: SmsVdpRequirement,
    selected: SmsHardwareModel,
    prepare: impl FnOnce() -> Result<T, String>,
) -> Result<(T, SmsCompatibilityStatus), String> {
    let status = requirement.evaluate(selected);
    if status == SmsCompatibilityStatus::Incompatible {
        return Err(
            "ROM exige SMS2 (VDP 315-5246), mas o hardware selecionado é SMS1 (VDP 315-5124)"
                .to_owned(),
        );
    }

    prepare().map(|prepared| (prepared, status))
}

/// Physical RAM technology and persistence semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CartridgeRamKind {
    /// SRAM with no battery or other retention source.
    VolatileSram,
    /// SRAM retained by a cartridge battery.
    BatteryBackedSram,
    /// Nonvolatile ferroelectric RAM; it does not require a battery.
    FeRam,
}

/// Address decoder known to expose the cartridge RAM.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CartridgeRamMapping {
    /// Sega mapper slot 2 ($8000-$BFFF), with capacity-specific mirroring/banking.
    SegaMapperSlot2,
    /// Address mapping is not sufficiently documented to execute.
    Unknown,
}

/// Evidence state controls whether a profile may configure a live machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileStatus {
    /// Identity and hardware behavior are sufficiently supported for execution.
    Verified,
    /// Some properties are known, but mapper/address behavior is incomplete.
    EvidenceOnly,
    /// A known uncertainty blocks executable mapping.
    Blocked,
}

/// Physical cartridge RAM description. Persistence is independent of mapping.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartridgeRamProfile {
    pub kind: CartridgeRamKind,
    pub capacity_bytes: usize,
    pub mapping: CartridgeRamMapping,
    /// Size of the mapped CPU window, when documented.
    pub mapped_window_bytes: usize,
    /// Physical bank size used by the mapper.
    pub bank_size_bytes: usize,
    /// Whether a smaller physical RAM is mirrored across the mapped window.
    pub mirrored: bool,
}

impl CartridgeRamProfile {
    /// True for nonvolatile RAM that must be retained across emulator sessions.
    pub const fn is_persistent(&self) -> bool {
        matches!(
            self.kind,
            CartridgeRamKind::BatteryBackedSram | CartridgeRamKind::FeRam
        )
    }
}

/// Exact cartridge hardware profile selected by ROM content identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartridgeHardwareProfile {
    pub system: CartridgeSystem,
    pub mapper: CartridgeMapper,
    pub ram: Option<CartridgeRamProfile>,
    pub status: ProfileStatus,
    pub board: Option<String>,
    pub evidence: Vec<String>,
}

impl CartridgeHardwareProfile {
    /// Only verified, fully mapped profiles may configure an executing machine.
    pub fn is_runtime_usable(&self) -> bool {
        self.status == ProfileStatus::Verified
            && self.mapper != CartridgeMapper::Unknown
            && self.ram.as_ref().is_some_and(|ram| {
                ram.mapping != CartridgeRamMapping::Unknown && ram.capacity_bytes > 0
            })
    }

    /// Whether this cartridge has nonvolatile memory requiring persistent storage.
    pub fn requires_persistent_store(&self) -> bool {
        self.ram
            .as_ref()
            .is_some_and(CartridgeRamProfile::is_persistent)
    }

    /// Return the declared RAM capacity, without inferring it from ROM or save size.
    pub fn ram_capacity_bytes(&self) -> Option<usize> {
        self.ram.as_ref().map(|ram| ram.capacity_bytes)
    }
}

#[cfg(test)]
mod sms_compatibility_tests {
    use super::{
        SmsCompatibilityStatus, SmsHardwareModel, SmsVdpRequirement, preflight_sms_cartridge,
    };

    #[test]
    fn sms1_compatible_requirement_accepts_both_selected_models() {
        assert_eq!(
            SmsVdpRequirement::Sms1Compatible.evaluate(SmsHardwareModel::Sms1),
            SmsCompatibilityStatus::Compatible
        );
        assert_eq!(
            SmsVdpRequirement::Sms1Compatible.evaluate(SmsHardwareModel::Sms2),
            SmsCompatibilityStatus::Compatible
        );
    }

    #[test]
    fn sms2_required_rejects_sms1_and_accepts_sms2() {
        assert_eq!(
            SmsVdpRequirement::Sms2Required.evaluate(SmsHardwareModel::Sms1),
            SmsCompatibilityStatus::Incompatible
        );
        assert_eq!(
            SmsVdpRequirement::Sms2Required.evaluate(SmsHardwareModel::Sms2),
            SmsCompatibilityStatus::Compatible
        );
    }

    #[test]
    fn unknown_requirement_remains_unknown_for_both_models() {
        assert_eq!(
            SmsVdpRequirement::Unknown.evaluate(SmsHardwareModel::Sms1),
            SmsCompatibilityStatus::Unknown
        );
        assert_eq!(
            SmsVdpRequirement::Unknown.evaluate(SmsHardwareModel::Sms2),
            SmsCompatibilityStatus::Unknown
        );
    }

    #[test]
    fn incompatible_preflight_does_not_build_or_replace_the_active_session() {
        let mut active_session = Some("current ROM");
        let mut preparation_called = false;

        let result = preflight_sms_cartridge(
            SmsVdpRequirement::Sms2Required,
            SmsHardwareModel::Sms1,
            || {
                preparation_called = true;
                Ok("next ROM")
            },
        );

        match result {
            Ok((next, _)) => active_session = Some(next),
            Err(error) => {
                assert!(error.contains("SMS2"));
                assert!(error.contains("315-5124"));
            }
        }
        assert!(!preparation_called);
        assert_eq!(active_session, Some("current ROM"));
    }

    #[test]
    fn sms2_required_preflight_runs_preparation_when_sms2_is_selected() {
        let mut preparation_called = false;
        let (prepared, status) = preflight_sms_cartridge(
            SmsVdpRequirement::Sms2Required,
            SmsHardwareModel::Sms2,
            || {
                preparation_called = true;
                Ok("prepared")
            },
        )
        .unwrap();

        assert_eq!(prepared, "prepared");
        assert_eq!(status, SmsCompatibilityStatus::Compatible);
        assert!(preparation_called);
    }

    #[test]
    fn unknown_preflight_allows_load_without_changing_selected_hardware() {
        let selected = SmsHardwareModel::Sms1;
        let (prepared, status) =
            preflight_sms_cartridge(SmsVdpRequirement::Unknown, selected, || Ok("ROM")).unwrap();

        assert_eq!(prepared, "ROM");
        assert_eq!(status, SmsCompatibilityStatus::Unknown);
        assert_eq!(selected, SmsHardwareModel::Sms1);
    }
}
