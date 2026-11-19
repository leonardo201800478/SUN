//! Timestamp-ordered YM2413 register writes bound to the core input-clock timeline.
//!
//! This adapter establishes only the documented external 12/84-clock write
//! waits and the ordering of writes against the core's elapsed input clocks.
//! It does not infer when a register value is consumed by PG, EG, an operator,
//! or the output pipeline.

use crate::{
    Ym2413BusTiming, Ym2413BusTimingError, Ym2413BusTimingState, Ym2413Core, Ym2413Error,
    Ym2413State,
};
use core::fmt;

/// Snapshot format version for the timed register-bus adapter.
pub const YM2413_TIMED_BUS_STATE_VERSION: u16 = 1;

/// Errors returned by the timestamped YM2413 bus adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413TimedBusError {
    /// A bus operation was timestamped before the core's current input clock.
    ClockWentBackwards {
        current_clock: u64,
        actual_clock: u64,
    },
    /// The external bus timing validator rejected the operation.
    BusTiming(Ym2413BusTimingError),
    /// Advancing the core input-clock counter failed.
    Core(Ym2413Error),
    /// The adapter snapshot format is unsupported.
    StateFormatVersionMismatch { expected: u16, actual: u16 },
    /// The nested core or bus-timing snapshot is inconsistent.
    InvalidState,
}

impl fmt::Display for Ym2413TimedBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClockWentBackwards {
                current_clock,
                actual_clock,
            } => write!(
                f,
                "YM2413 bus timestamp {actual_clock} precedes core clock {current_clock}"
            ),
            Self::BusTiming(error) => write!(f, "YM2413 bus timing error: {error}"),
            Self::Core(error) => write!(f, "YM2413 core clock error: {error}"),
            Self::StateFormatVersionMismatch { expected, actual } => write!(
                f,
                "YM2413 timed bus state version {actual} is unsupported (expected {expected})"
            ),
            Self::InvalidState => f.write_str("YM2413 timed bus state is inconsistent"),
        }
    }
}

impl std::error::Error for Ym2413TimedBusError {}

impl From<Ym2413BusTimingError> for Ym2413TimedBusError {
    fn from(value: Ym2413BusTimingError) -> Self {
        Self::BusTiming(value)
    }
}

impl From<Ym2413Error> for Ym2413TimedBusError {
    fn from(value: Ym2413Error) -> Self {
        Self::Core(value)
    }
}

/// Complete snapshot of the timed bus adapter and nested register/clock core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ym2413TimedBusState {
    /// Snapshot format version.
    pub format_version: u16,
    /// Register image, address latch, and core input-clock position.
    pub core: Ym2413State,
    /// Last accepted external register write and its wait contract.
    pub bus_timing: Ym2413BusTimingState,
}

/// Timestamped external register interface for the YM2413 core.
///
/// Timestamps are absolute YM2413 input-clock counts since the adapter's
/// reset. A write at clock T first advances the core through the interval up
/// to T, then applies the external register operation at T. A rejected write
/// changes neither the core nor the bus validator.
///
/// The adapter does not decide how the machine maps CPU/bus time to YM2413
/// input clocks. The machine integration must supply correctly converted,
/// monotonic timestamps from its established clock-domain relationship.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ym2413TimedBus {
    core: Ym2413Core,
    timing: Ym2413BusTiming,
}

impl Ym2413TimedBus {
    /// Construct a reset core and an empty external bus timing history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return read-only access to the register and sequential-slot core.
    pub const fn core(&self) -> &Ym2413Core {
        &self.core
    }

    /// Return read-only access to the external bus timing validator.
    pub const fn timing(&self) -> &Ym2413BusTiming {
        &self.timing
    }

    /// Advance the core to an absolute input-clock timestamp.
    ///
    /// Returns the number of complete 18-slot rotations crossed. This method
    /// does not synthesize audio or alter the external bus write history.
    pub fn advance_to(&mut self, master_clock: u64) -> Result<u64, Ym2413TimedBusError> {
        let current_clock = self.core.elapsed_input_clocks();
        if master_clock < current_clock {
            return Err(Ym2413TimedBusError::ClockWentBackwards {
                current_clock,
                actual_clock: master_clock,
            });
        }

        Ok(self
            .core
            .advance_input_clocks(master_clock - current_clock)?)
    }

    /// Apply an address-latch write at an absolute YM2413 input-clock timestamp.
    pub fn write_address_at(
        &mut self,
        master_clock: u64,
        address: u8,
    ) -> Result<(), Ym2413TimedBusError> {
        self.apply_write_at(master_clock, |core| core.write_address(address), true)
    }

    /// Apply a register-data write at an absolute YM2413 input-clock timestamp.
    pub fn write_data_at(
        &mut self,
        master_clock: u64,
        value: u8,
    ) -> Result<(), Ym2413TimedBusError> {
        self.apply_write_at(master_clock, |core| core.write_data(value), false)
    }

    /// Reset core registers, slot phase, elapsed clocks, and bus wait history.
    ///
    /// The machine adapter remains responsible for the documented external
    /// reset-pulse timing and for choosing the correct reset event timestamp.
    pub fn reset(&mut self) {
        self.core.reset();
        self.timing = Ym2413BusTiming::new();
    }

    /// Export the complete timed bus snapshot.
    pub fn export_state(&self) -> Ym2413TimedBusState {
        Ym2413TimedBusState {
            format_version: YM2413_TIMED_BUS_STATE_VERSION,
            core: self.core.export_state(),
            bus_timing: self.timing.export_state(),
        }
    }

    /// Restore the nested core and bus state after cross-validating timestamps.
    pub fn restore_state(
        state: Ym2413TimedBusState,
    ) -> Result<Self, Ym2413TimedBusError> {
        if state.format_version != YM2413_TIMED_BUS_STATE_VERSION {
            return Err(Ym2413TimedBusError::StateFormatVersionMismatch {
                expected: YM2413_TIMED_BUS_STATE_VERSION,
                actual: state.format_version,
            });
        }

        let core = Ym2413Core::restore_state(state.core)?;
        let timing = Ym2413BusTiming::restore_state(state.bus_timing)?;
        if timing
            .last_write()
            .is_some_and(|(last_write_clock, _)| last_write_clock > core.elapsed_input_clocks())
        {
            return Err(Ym2413TimedBusError::InvalidState);
        }

        Ok(Self { core, timing })
    }

    fn apply_write_at<F>(
        &mut self,
        master_clock: u64,
        apply: F,
        is_address: bool,
    ) -> Result<(), Ym2413TimedBusError>
    where
        F: FnOnce(&mut Ym2413Core),
    {
        let current_clock = self.core.elapsed_input_clocks();
        if master_clock < current_clock {
            return Err(Ym2413TimedBusError::ClockWentBackwards {
                current_clock,
                actual_clock: master_clock,
            });
        }

        // Validate against a copy first: an early write must not advance the
        // core clock or alter the last accepted write.
        let mut next_timing = self.timing;
        if is_address {
            next_timing.record_address_write_at(master_clock)?;
        } else {
            next_timing.record_data_write_at(master_clock)?;
        }

        self.core
            .advance_input_clocks(master_clock - current_clock)?;
        apply(&mut self.core);
        self.timing = next_timing;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Ym2413TimedBus, Ym2413TimedBusError, YM2413_TIMED_BUS_STATE_VERSION};
    use crate::{Ym2413BusTimingError, Ym2413BusWriteKind};

    #[test]
    fn timestamped_address_and_data_writes_advance_the_core_timeline() {
        let mut bus = Ym2413TimedBus::new();

        bus.write_address_at(100, 0x20).unwrap();
        assert_eq!(bus.core().elapsed_input_clocks(), 100);
        bus.write_data_at(112, 0x5a).unwrap();

        let state = bus.core().export_state();
        assert_eq!(state.elapsed_input_clocks, 112);
        assert_eq!(state.registers[0x20], 0x5a);
        assert_eq!(bus.timing().last_write(), Some((112, Ym2413BusWriteKind::Data)));
    }

    #[test]
    fn early_write_is_rejected_without_advancing_or_mutating_either_state() {
        let mut bus = Ym2413TimedBus::new();
        bus.write_address_at(100, 0x10).unwrap();
        let before = bus.export_state();

        assert_eq!(
            bus.write_data_at(111, 0xff),
            Err(Ym2413TimedBusError::BusTiming(
                Ym2413BusTimingError::WaitNotElapsed {
                    previous_write: Ym2413BusWriteKind::Address,
                    earliest_clock: 112,
                    actual_clock: 111,
                }
            ))
        );
        assert_eq!(bus.export_state(), before);
    }

    #[test]
    fn data_wait_applies_before_both_address_and_data_writes() {
        let mut bus = Ym2413TimedBus::new();
        bus.write_address_at(0, 0x20).unwrap();
        bus.write_data_at(12, 0x01).unwrap();

        assert!(matches!(
            bus.write_address_at(95, 0x21),
            Err(Ym2413TimedBusError::BusTiming(
                Ym2413BusTimingError::WaitNotElapsed { .. }
            ))
        ));
        bus.write_address_at(96, 0x21).unwrap();
        assert_eq!(bus.core().address_latch(), 0x21);
    }

    #[test]
    fn write_before_current_core_clock_is_rejected_without_mutation() {
        let mut bus = Ym2413TimedBus::new();
        bus.advance_to(500).unwrap();
        let before = bus.export_state();

        assert_eq!(
            bus.write_address_at(499, 0x20),
            Err(Ym2413TimedBusError::ClockWentBackwards {
                current_clock: 500,
                actual_clock: 499,
            })
        );
        assert_eq!(bus.export_state(), before);
    }

    #[test]
    fn continuous_and_chunked_advance_to_produce_identical_state() {
        let mut continuous = Ym2413TimedBus::new();
        let mut chunked = Ym2413TimedBus::new();

        continuous.advance_to(5_003).unwrap();
        for clock in [1, 4, 17, 72, 511, 1_024, 3_371, 5_003] {
            chunked.advance_to(clock).unwrap();
        }

        assert_eq!(continuous.export_state(), chunked.export_state());
    }

    #[test]
    fn snapshot_restore_preserves_bus_deadline_and_core_position() {
        let mut bus = Ym2413TimedBus::new();
        bus.write_address_at(100, 0x20).unwrap();
        bus.write_data_at(112, 0x5a).unwrap();
        bus.advance_to(250).unwrap();

        let restored = Ym2413TimedBus::restore_state(bus.export_state()).unwrap();
        assert_eq!(restored.export_state(), bus.export_state());
        assert_eq!(restored.core().elapsed_input_clocks(), 250);
        assert_eq!(restored.timing().last_write(), Some((112, Ym2413BusWriteKind::Data)));
    }

    #[test]
    fn restore_rejects_a_bus_write_timestamp_after_the_core_clock() {
        let mut bus = Ym2413TimedBus::new();
        bus.write_address_at(100, 0x20).unwrap();
        let mut state = bus.export_state();
        state.bus_timing.last_write_clock = Some(101);

        assert_eq!(
            Ym2413TimedBus::restore_state(state),
            Err(Ym2413TimedBusError::InvalidState)
        );
    }

    #[test]
    fn reset_clears_registers_clock_and_bus_wait_history() {
        let mut bus = Ym2413TimedBus::new();
        bus.write_address_at(100, 0x20).unwrap();
        bus.write_data_at(112, 0x5a).unwrap();

        bus.reset();

        assert_eq!(bus.core().elapsed_input_clocks(), 0);
        assert_eq!(bus.core().address_latch(), 0);
        assert_eq!(bus.timing().last_write(), None);
        assert_eq!(bus.core().export_state().registers, [0; 0x40]);
    }

    #[test]
    fn restore_rejects_unknown_snapshot_version() {
        let mut bus = Ym2413TimedBus::new();
        let mut state = bus.export_state();
        state.format_version = YM2413_TIMED_BUS_STATE_VERSION + 1;

        assert_eq!(
            Ym2413TimedBus::restore_state(state),
            Err(Ym2413TimedBusError::StateFormatVersionMismatch {
                expected: YM2413_TIMED_BUS_STATE_VERSION,
                actual: YM2413_TIMED_BUS_STATE_VERSION + 1,
            })
        );
    }
}
