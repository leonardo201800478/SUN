//! YM2413 external register-bus wait validation.
//!
//! Yamaha specifies a 12-master-clock wait after an address write and an
//! 84-master-clock wait after a data write before the next address or data
//! write. This module validates that host-side protocol contract. It does not
//! emulate the chip's undefined/uncertain behavior when software violates the
//! waits, nor does it model internal register commit or slot-pipeline latency.

use core::fmt;

/// Minimum master-clock interval after an address write.
pub const YM2413_ADDRESS_WRITE_WAIT_CLOCKS: u64 = 12;

/// Minimum master-clock interval after a data write.
pub const YM2413_DATA_WRITE_WAIT_CLOCKS: u64 = 84;

/// Version of the timestamped bus-wait snapshot.
pub const YM2413_BUS_TIMING_STATE_VERSION: u16 = 1;

/// Kind of the most recently accepted external bus write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Ym2413BusWriteKind {
    /// Address-latch write (A0 = 0).
    Address = 0,
    /// Register-data write (A0 = 1).
    Data = 1,
}

impl Ym2413BusWriteKind {
    /// Return the minimum wait required before the next address or data write.
    pub const fn required_wait_clocks(self) -> u64 {
        match self {
            Self::Address => YM2413_ADDRESS_WRITE_WAIT_CLOCKS,
            Self::Data => YM2413_DATA_WRITE_WAIT_CLOCKS,
        }
    }
}

/// Complete serializable state of the bus-wait validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413BusTimingState {
    /// Snapshot format version.
    pub format_version: u16,
    /// Master-clock timestamp of the last accepted write, if any.
    pub last_write_clock: Option<u64>,
    /// Kind of the last accepted write, if any.
    pub last_write_kind: Option<Ym2413BusWriteKind>,
}

/// Errors returned by the YM2413 bus-wait validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413BusTimingError {
    /// The supplied timestamp precedes the previous accepted bus write.
    ClockWentBackwards {
        previous_clock: u64,
        actual_clock: u64,
    },
    /// The required post-write wait has not elapsed.
    WaitNotElapsed {
        previous_write: Ym2413BusWriteKind,
        earliest_clock: u64,
        actual_clock: u64,
    },
    /// Computing the earliest legal timestamp overflowed u64.
    ClockOverflow,
    /// The snapshot format version is unsupported.
    StateFormatVersionMismatch { expected: u16, actual: u16 },
    /// The snapshot has a timestamp without a write kind, or vice versa.
    InvalidState,
}

impl fmt::Display for Ym2413BusTimingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClockWentBackwards {
                previous_clock,
                actual_clock,
            } => write!(
                f,
                "YM2413 bus clock moved backwards from {previous_clock} to {actual_clock}"
            ),
            Self::WaitNotElapsed {
                previous_write,
                earliest_clock,
                actual_clock,
            } => write!(
                f,
                "YM2413 {:?} write requires the next write at clock {earliest_clock} or later (got {actual_clock})",
                previous_write
            ),
            Self::ClockOverflow => f.write_str("YM2413 bus wait timestamp overflow"),
            Self::StateFormatVersionMismatch { expected, actual } => write!(
                f,
                "YM2413 bus timing state version {actual} is unsupported (expected {expected})"
            ),
            Self::InvalidState => f.write_str("YM2413 bus timing state is inconsistent"),
        }
    }
}

impl std::error::Error for Ym2413BusTimingError {}

/// Deterministic validator for Yamaha's external register-write wait contract.
///
/// The caller supplies timestamps in YM2413 master-clock cycles. The first
/// write is accepted at any timestamp. After an address write, the next write
/// must be at least 12 clocks later. After a data write, the next write of
/// either kind must be at least 84 clocks later.
///
/// A rejected write does not mutate this validator. The caller should invoke
/// the corresponding method at the same timestamp as the infallible core
/// register write. This type intentionally remains separate from Ym2413Core:
/// it does not decide how an adapter schedules or commits writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ym2413BusTiming {
    last_write_clock: Option<u64>,
    last_write_kind: Option<Ym2413BusWriteKind>,
}

impl Ym2413BusTiming {
    /// Construct a validator with no previous bus write.
    pub const fn new() -> Self {
        Self {
            last_write_clock: None,
            last_write_kind: None,
        }
    }

    /// Record an address write if the previous write's wait has elapsed.
    pub fn record_address_write_at(
        &mut self,
        master_clock: u64,
    ) -> Result<(), Ym2413BusTimingError> {
        self.record_write_at(master_clock, Ym2413BusWriteKind::Address)
    }

    /// Record a data write if the previous write's wait has elapsed.
    pub fn record_data_write_at(
        &mut self,
        master_clock: u64,
    ) -> Result<(), Ym2413BusTimingError> {
        self.record_write_at(master_clock, Ym2413BusWriteKind::Data)
    }

    /// Return the timestamp and kind of the most recently accepted write.
    pub const fn last_write(&self) -> Option<(u64, Ym2413BusWriteKind)> {
        match (self.last_write_clock, self.last_write_kind) {
            (Some(clock), Some(kind)) => Some((clock, kind)),
            _ => None,
        }
    }

    /// Export the complete validator state.
    pub const fn export_state(&self) -> Ym2413BusTimingState {
        Ym2413BusTimingState {
            format_version: YM2413_BUS_TIMING_STATE_VERSION,
            last_write_clock: self.last_write_clock,
            last_write_kind: self.last_write_kind,
        }
    }

    /// Restore a snapshot after validating its version and paired optional fields.
    pub const fn restore_state(
        state: Ym2413BusTimingState,
    ) -> Result<Self, Ym2413BusTimingError> {
        if state.format_version != YM2413_BUS_TIMING_STATE_VERSION {
            return Err(Ym2413BusTimingError::StateFormatVersionMismatch {
                expected: YM2413_BUS_TIMING_STATE_VERSION,
                actual: state.format_version,
            });
        }
        if state.last_write_clock.is_some() != state.last_write_kind.is_some() {
            return Err(Ym2413BusTimingError::InvalidState);
        }

        Ok(Self {
            last_write_clock: state.last_write_clock,
            last_write_kind: state.last_write_kind,
        })
    }

    fn record_write_at(
        &mut self,
        master_clock: u64,
        kind: Ym2413BusWriteKind,
    ) -> Result<(), Ym2413BusTimingError> {
        if let Some((previous_clock, previous_kind)) = self.last_write() {
            if master_clock < previous_clock {
                return Err(Ym2413BusTimingError::ClockWentBackwards {
                    previous_clock,
                    actual_clock: master_clock,
                });
            }

            let earliest_clock = previous_clock
                .checked_add(previous_kind.required_wait_clocks())
                .ok_or(Ym2413BusTimingError::ClockOverflow)?;
            if master_clock < earliest_clock {
                return Err(Ym2413BusTimingError::WaitNotElapsed {
                    previous_write: previous_kind,
                    earliest_clock,
                    actual_clock: master_clock,
                });
            }
        }

        self.last_write_clock = Some(master_clock);
        self.last_write_kind = Some(kind);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        YM2413_ADDRESS_WRITE_WAIT_CLOCKS, YM2413_BUS_TIMING_STATE_VERSION,
        YM2413_DATA_WRITE_WAIT_CLOCKS, Ym2413BusTiming, Ym2413BusTimingError,
        Ym2413BusTimingState, Ym2413BusWriteKind,
    };

    #[test]
    fn first_address_or_data_write_is_accepted_without_a_prior_wait() {
        let mut address_first = Ym2413BusTiming::new();
        assert_eq!(address_first.record_address_write_at(0), Ok(()));

        let mut data_first = Ym2413BusTiming::new();
        assert_eq!(data_first.record_data_write_at(37), Ok(()));
    }

    #[test]
    fn address_write_requires_twelve_clocks_before_any_next_write() {
        let mut timing = Ym2413BusTiming::new();
        timing.record_address_write_at(100).unwrap();

        assert_eq!(
            timing.record_data_write_at(111),
            Err(Ym2413BusTimingError::WaitNotElapsed {
                previous_write: Ym2413BusWriteKind::Address,
                earliest_clock: 100 + YM2413_ADDRESS_WRITE_WAIT_CLOCKS,
                actual_clock: 111,
            })
        );
        assert_eq!(timing.last_write(), Some((100, Ym2413BusWriteKind::Address)));
        assert_eq!(timing.record_data_write_at(112), Ok(()));
    }

    #[test]
    fn data_write_requires_eighty_four_clocks_before_address_or_data() {
        let mut timing = Ym2413BusTiming::new();
        timing.record_address_write_at(10).unwrap();
        timing.record_data_write_at(22).unwrap();

        assert_eq!(
            timing.record_address_write_at(105),
            Err(Ym2413BusTimingError::WaitNotElapsed {
                previous_write: Ym2413BusWriteKind::Data,
                earliest_clock: 22 + YM2413_DATA_WRITE_WAIT_CLOCKS,
                actual_clock: 105,
            })
        );
        assert_eq!(timing.record_address_write_at(106), Ok(()));
    }

    #[test]
    fn address_to_address_also_observes_the_address_wait() {
        let mut timing = Ym2413BusTiming::new();
        timing.record_address_write_at(5).unwrap();

        assert_eq!(
            timing.record_address_write_at(16),
            Err(Ym2413BusTimingError::WaitNotElapsed {
                previous_write: Ym2413BusWriteKind::Address,
                earliest_clock: 5 + YM2413_ADDRESS_WRITE_WAIT_CLOCKS,
                actual_clock: 16,
            })
        );
        assert_eq!(timing.record_address_write_at(17), Ok(()));
    }

    #[test]
    fn backwards_timestamps_are_rejected_without_mutation() {
        let mut timing = Ym2413BusTiming::new();
        timing.record_data_write_at(100).unwrap();

        assert_eq!(
            timing.record_address_write_at(99),
            Err(Ym2413BusTimingError::ClockWentBackwards {
                previous_clock: 100,
                actual_clock: 99,
            })
        );
        assert_eq!(timing.last_write(), Some((100, Ym2413BusWriteKind::Data)));
    }

    #[test]
    fn snapshot_round_trip_preserves_last_write_and_wait_deadline() {
        let mut timing = Ym2413BusTiming::new();
        timing.record_data_write_at(1234).unwrap();

        let restored = Ym2413BusTiming::restore_state(timing.export_state()).unwrap();
        assert_eq!(restored, timing);
        assert_eq!(restored.export_state().last_write_clock, Some(1234));
    }

    #[test]
    fn restore_rejects_unknown_version_and_unpaired_fields() {
        assert_eq!(
            Ym2413BusTiming::restore_state(Ym2413BusTimingState {
                format_version: YM2413_BUS_TIMING_STATE_VERSION + 1,
                last_write_clock: None,
                last_write_kind: None,
            }),
            Err(Ym2413BusTimingError::StateFormatVersionMismatch {
                expected: YM2413_BUS_TIMING_STATE_VERSION,
                actual: YM2413_BUS_TIMING_STATE_VERSION + 1,
            })
        );

        assert_eq!(
            Ym2413BusTiming::restore_state(Ym2413BusTimingState {
                format_version: YM2413_BUS_TIMING_STATE_VERSION,
                last_write_clock: Some(10),
                last_write_kind: None,
            }),
            Err(Ym2413BusTimingError::InvalidState)
        );
    }

    #[test]
    fn wait_deadline_overflow_is_reported_without_mutation() {
        let mut timing = Ym2413BusTiming::new();
        timing
            .record_address_write_at(u64::MAX - YM2413_ADDRESS_WRITE_WAIT_CLOCKS + 1)
            .unwrap();

        assert_eq!(
            timing.record_data_write_at(u64::MAX),
            Err(Ym2413BusTimingError::ClockOverflow)
        );
        assert_eq!(
            timing.last_write(),
            Some((
                u64::MAX - YM2413_ADDRESS_WRITE_WAIT_CLOCKS + 1,
                Ym2413BusWriteKind::Address
            ))
        );
    }
}
