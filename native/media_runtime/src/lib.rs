//! Shared ROM identity and cartridge hardware metadata.
//!
//! This crate contains no emulator execution code. Filesystem access is limited
//! to explicit media/firmware scan and load operations; it is never called by
//! the machine Scheduler. Core/runtime layers consume validated metadata/bytes.

mod catalog;
mod dat;
mod identity;
mod profile;
mod sms_firmware;
mod sms_region;

pub use catalog::{
    CartridgeCatalog, CartridgeMatch, CatalogLookupError, KnownCartridgeProfile,
    SMS_CARTRIDGE_COMPATIBILITY_DATA_VERSION, SMS_CARTRIDGE_COMPATIBILITY_SCHEMA_VERSION,
};
pub use dat::{DatGameEntry, DatParseError, DatRomEntry, NoIntroDat, SMS_DAT_VERSION};
pub use identity::CartridgeIdentity;
pub use profile::{
    CartridgeCompatibilityMetadata, CartridgeHardwareProfile, CartridgeMapper, CartridgeRamKind,
    CartridgeRamMapping, CartridgeRamProfile, CartridgeSystem, ProfileStatus,
    SmsCompatibilityStatus, SmsHardwareModel, SmsVdpRequirement, preflight_sms_cartridge,
};
pub use sms_firmware::{
    SmsBuiltInGame, SmsConsoleVariant, SmsFirmwareCatalog, SmsFirmwareDescriptor, SmsFirmwareId,
    SmsFirmwareKind, SmsFirmwareLoadError, SmsFirmwareMappingStrategy, SmsFirmwareMatch,
    SmsFirmwareRegion, SmsFirmwareScanEntry, SmsFirmwareScanStatus, SmsFirmwareValidation,
};
pub use sms_region::{
    SmsRegion, SmsRegionPreference, SmsRegionResolution, SmsRegionResolutionSource,
    resolve_sms_region,
};
