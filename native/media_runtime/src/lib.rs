//! Shared ROM identity and cartridge hardware metadata.
//!
//! This crate deliberately contains no filesystem or emulator execution code.
//! Host layers load ROM/save bytes; core/runtime layers consume explicit profiles.

mod catalog;
mod dat;
mod identity;
mod profile;
mod sms_firmware;

pub use catalog::{CartridgeCatalog, CartridgeMatch, CatalogLookupError, KnownCartridgeProfile};
pub use dat::{DatGameEntry, DatParseError, DatRomEntry, NoIntroDat, SMS_DAT_VERSION};
pub use identity::CartridgeIdentity;
pub use sms_firmware::{
    SmsBuiltInGame, SmsConsoleVariant, SmsFirmwareCatalog, SmsFirmwareDescriptor, SmsFirmwareId,
    SmsFirmwareKind, SmsFirmwareLoadError, SmsFirmwareMappingStrategy, SmsFirmwareMatch,
    SmsFirmwareRegion, SmsFirmwareScanEntry, SmsFirmwareScanStatus, SmsFirmwareValidation,
};
pub use profile::{
    CartridgeHardwareProfile, CartridgeMapper, CartridgeRamKind, CartridgeRamMapping,
    CartridgeRamProfile, CartridgeSystem, ProfileStatus,
};
