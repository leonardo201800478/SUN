//! Chip-global YM2413 envelope timer and rate-latch pipeline.
//!
//! This component advances once per operator slot (18 slots per rotation).
//! It is intentionally separate from per-operator envelope state: the timer
//! and its rate latches are shared by every operator. The caller supplies the
//! rate to be latched for the following slot; the returned phase describes the
//! rate consumed by the current slot.

const SLOT_COUNT: u8 = 18;
const TIMER_MASK: u32 = 0x3ffff;
const TIMER_SHIFT_LOCK_LIMIT: u8 = 13;

// Rate-low versus the timer's two-bit low-phase latch.
const STEP_HIGH: [[u8; 4]; 4] = [[0, 0, 0, 0], [1, 0, 0, 0], [1, 0, 1, 0], [1, 1, 1, 0]];

/// Serializable state of the chip-global envelope timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EnvelopeTimerState {
    /// Eighteen-bit shift-register state.
    pub timer: u32,
    /// Carry retained between timer shifts.
    pub carry: u8,
    /// Eight-bit completed-rotation counter.
    pub counter_state: u8,
    /// Counter value latched by the preceding slot.
    pub counter_state_prev: u8,
    /// Slot at which the timer's high phase was first observed.
    pub shift: u8,
    /// Prevents repeated high-phase captures within one rotation.
    pub shift_stop: bool,
    /// Two-bit low-phase value captured at the lock boundary.
    pub low_lock: u8,
    /// High-phase slot captured at the lock boundary.
    pub shift_lock: u8,
    /// Rate prepared for consumption by the next slot.
    pub pending_rate: u8,
    /// Base-rate-zero flag prepared for consumption by the next slot.
    pub pending_zero_rate: bool,
    /// Next slot expected by this state machine.
    pub next_slot: u8,
}

/// Rate and counter phase prepared for the following operator EG update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EnvelopeRatePhase {
    /// Previous rate that generated the increment latches for the next slot.
    pub rate_latch_source: u8,
    /// Newly calculated rate, consumed by the timer pipeline on the next slot.
    pub next_rate: u8,
    /// Whether the newly calculated base rate is zero (independent of KSR).
    pub next_zero_rate: bool,
    /// Upper four bits of the consumed rate.
    pub rate_hi: u8,
    /// Low-rate increment enable.
    pub inc_lo: bool,
    /// High-rate increment enable.
    pub inc_hi: bool,
    /// Consumed rate is in the maximum-rate class.
    pub max_rate: bool,
    /// Newly prepared base rate is zero, independent of KSR.
    pub zero_rate: bool,
    /// Counter value visible to the current EG update.
    pub counter_state_prev: u8,
}

/// Timer and rate-latch state after one operator slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EnvelopeTimerSnapshot {
    pub timer: u32,
    pub carry: u8,
    pub counter_state: u8,
    pub counter_state_prev: u8,
    pub shift: u8,
    pub shift_stop: bool,
    pub low_lock: u8,
    pub shift_lock: u8,
    pub rate_phase: EnvelopeRatePhase,
}

/// Invalid timer input or an out-of-order slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeTimerError {
    InvalidSlot(u8),
    UnexpectedSlot { expected: u8, actual: u8 },
    InvalidRate(u8),
    InvalidState(&'static str),
}

impl core::fmt::Display for EnvelopeTimerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSlot(slot) => write!(f, "invalid YM2413 slot {slot}"),
            Self::UnexpectedSlot { expected, actual } => {
                write!(f, "expected YM2413 slot {expected}, received {actual}")
            }
            Self::InvalidRate(rate) => write!(f, "invalid YM2413 envelope rate {rate}"),
            Self::InvalidState(reason) => {
                write!(f, "invalid YM2413 envelope timer state: {reason}")
            }
        }
    }
}

impl std::error::Error for EnvelopeTimerError {}

/// Shared YM2413 envelope timer.
///
/// The timer is clocked once for each operator slot, not from the audio output
/// callback. It owns the delayed rate latch and the phase locks used by the
/// per-operator EG. Integration with the chip scheduler is intentionally left
/// to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EnvelopeTimer {
    state: EnvelopeTimerState,
}

impl EnvelopeTimer {
    /// Create a reset timer with slot zero next.
    pub const fn new() -> Self {
        Self {
            state: EnvelopeTimerState {
                timer: 0,
                carry: 0,
                counter_state: 0,
                counter_state_prev: 0,
                shift: 0,
                shift_stop: false,
                low_lock: 0,
                shift_lock: 0,
                pending_rate: 0,
                pending_zero_rate: false,
                next_slot: 0,
            },
        }
    }

    /// Return the complete serializable timer state.
    pub const fn export_state(&self) -> EnvelopeTimerState {
        self.state
    }

    /// Restore a previously exported timer state after validating its bounds.
    pub fn restore_state(state: EnvelopeTimerState) -> Result<Self, EnvelopeTimerError> {
        if state.timer > TIMER_MASK {
            return Err(EnvelopeTimerError::InvalidState("timer exceeds 18 bits"));
        }
        if state.carry > 1 {
            return Err(EnvelopeTimerError::InvalidState("carry is not a bit"));
        }
        if state.shift >= SLOT_COUNT || state.next_slot >= SLOT_COUNT {
            return Err(EnvelopeTimerError::InvalidState(
                "slot latch is out of range",
            ));
        }
        if state.low_lock > 3 {
            return Err(EnvelopeTimerError::InvalidState(
                "low lock exceeds two bits",
            ));
        }
        if state.shift_lock > TIMER_SHIFT_LOCK_LIMIT {
            return Err(EnvelopeTimerError::InvalidState(
                "shift lock exceeds its hardware range",
            ));
        }
        if state.pending_rate > 63 {
            return Err(EnvelopeTimerError::InvalidState(
                "pending rate exceeds six bits",
            ));
        }
        Ok(Self { state })
    }

    /// Advance one operator slot and prepare rate latches for the following slot.
    ///
    /// next_rate is calculated for the following EG operator update. The
    /// returned phase is the complete set of latches visible at the next slot:
    /// increment enables and maximum-rate status derive from the previously
    /// pending rate, while zero-rate status derives from the newly prepared
    /// rate. This mixed pipeline is intentional and matches the chip timing.
    pub fn tick_slot(
        &mut self,
        slot_index: u8,
        next_rate: u8,
        next_zero_rate: bool,
    ) -> Result<EnvelopeTimerSnapshot, EnvelopeTimerError> {
        if slot_index >= SLOT_COUNT {
            return Err(EnvelopeTimerError::InvalidSlot(slot_index));
        }
        if slot_index != self.state.next_slot {
            return Err(EnvelopeTimerError::UnexpectedSlot {
                expected: self.state.next_slot,
                actual: slot_index,
            });
        }
        if next_rate > 63 {
            return Err(EnvelopeTimerError::InvalidRate(next_rate));
        }

        let rate_latch_source = self.state.pending_rate;
        let rate_hi = rate_latch_source >> 2;
        let rate_lo = rate_latch_source & 0x03;
        let zero_rate_latch_source = self.state.pending_zero_rate;

        // The fractional timer input is enabled only at the terminal
        // two-bit counter phase. Slot zero seeds the carry; later slots
        // propagate the carry generated by the preceding shift.
        let timer_increment = if self.state.counter_state & 0x03 == 0x03 {
            if slot_index == 0 { 1 } else { self.state.carry }
        } else {
            0
        };

        let timer_low_before_shift = (self.state.timer & 0x03) as u8;
        let timer_bit = (self.state.timer & 1) + u32::from(timer_increment);
        self.state.carry = (timer_bit >> 1) as u8;
        self.state.timer = (((timer_bit & 1) << 17) | (self.state.timer >> 1)) & TIMER_MASK;

        let timer_high_phase = (self.state.timer & (1 << 16)) != 0;
        if !self.state.shift_stop && timer_high_phase {
            self.state.shift = slot_index;
        }

        // The low and high phase locks use the counter value retained from
        // the preceding slot, not the counter value updated at this boundary.
        if slot_index == 0 && (self.state.counter_state_prev & 1) != 0 {
            self.state.low_lock = timer_low_before_shift;
            self.state.shift_lock = if self.state.shift > TIMER_SHIFT_LOCK_LIMIT {
                0
            } else {
                self.state.shift
            };
            self.state.shift = 0;
        }

        if slot_index == 0 {
            self.state.shift_stop = false;
        } else {
            self.state.shift_stop |= timer_high_phase;
        }

        self.state.counter_state_prev = self.state.counter_state;
        if slot_index == SLOT_COUNT - 1 {
            self.state.counter_state = self.state.counter_state.wrapping_add(1);
        }

        let phase_sum = self.state.shift_lock.wrapping_add(rate_hi) & 0x0f;
        let inc_hi = STEP_HIGH[rate_lo as usize][self.state.low_lock as usize] != 0;
        let inc_lo = if rate_hi >= 12 || zero_rate_latch_source {
            false
        } else {
            match phase_sum {
                12 => true,
                13 => rate_lo & 0x02 != 0,
                14 => rate_lo & 0x01 != 0,
                _ => false,
            }
        };

        let rate_phase = EnvelopeRatePhase {
            rate_latch_source,
            next_rate,
            next_zero_rate,
            rate_hi,
            inc_lo,
            inc_hi,
            max_rate: rate_hi == 15,
            zero_rate: next_zero_rate,
            counter_state_prev: self.state.counter_state_prev,
        };

        self.state.pending_rate = next_rate;
        self.state.pending_zero_rate = next_zero_rate;
        self.state.next_slot = (slot_index + 1) % SLOT_COUNT;

        Ok(EnvelopeTimerSnapshot {
            timer: self.state.timer,
            carry: self.state.carry,
            counter_state: self.state.counter_state,
            counter_state_prev: self.state.counter_state_prev,
            shift: self.state.shift,
            shift_stop: self.state.shift_stop,
            low_lock: self.state.low_lock,
            shift_lock: self.state.shift_lock,
            rate_phase,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_state_is_zeroed_and_starts_at_slot_zero() {
        let timer = EnvelopeTimer::new();
        assert_eq!(timer.export_state(), EnvelopeTimerState::default());
    }

    #[test]
    fn counter_advances_only_after_slot_seventeen() {
        let mut timer = EnvelopeTimer::new();

        for slot in 0..SLOT_COUNT - 1 {
            let snapshot = timer.tick_slot(slot, 0, false).unwrap();
            assert_eq!(snapshot.counter_state, 0);
            assert_eq!(snapshot.counter_state_prev, 0);
        }

        let snapshot = timer.tick_slot(SLOT_COUNT - 1, 0, false).unwrap();
        assert_eq!(snapshot.counter_state, 1);
        assert_eq!(snapshot.counter_state_prev, 0);
    }

    #[test]
    fn rate_pipeline_consumes_previous_rate_and_tracks_next_zero_rate() {
        let mut timer = EnvelopeTimer::new();

        let first = timer.tick_slot(0, 60, false).unwrap();
        assert_eq!(first.rate_phase.rate_latch_source, 0);
        assert_eq!(first.rate_phase.rate_hi, 0);
        assert!(!first.rate_phase.max_rate);
        assert!(!first.rate_phase.zero_rate);
        assert!(!first.rate_phase.next_zero_rate);

        let second = timer.tick_slot(1, 0, true).unwrap();
        assert_eq!(second.rate_phase.rate_latch_source, 60);
        assert_eq!(second.rate_phase.rate_hi, 15);
        assert!(second.rate_phase.max_rate);
        assert!(second.rate_phase.zero_rate);
        assert!(second.rate_phase.next_zero_rate);
    }

    #[test]
    fn slot_sequence_is_strict_and_rejects_invalid_rates() {
        let mut timer = EnvelopeTimer::new();
        timer.tick_slot(0, 1, false).unwrap();

        assert_eq!(
            timer.tick_slot(2, 1, false),
            Err(EnvelopeTimerError::UnexpectedSlot {
                expected: 1,
                actual: 2,
            })
        );
        assert_eq!(
            timer.tick_slot(1, 64, false),
            Err(EnvelopeTimerError::InvalidRate(64))
        );
        assert_eq!(
            timer.tick_slot(18, 1, false),
            Err(EnvelopeTimerError::InvalidSlot(18))
        );
    }

    #[test]
    fn restore_state_rejects_invalid_timer_and_lock_ranges() {
        let state = EnvelopeTimerState {
            timer: TIMER_MASK + 1,
            ..EnvelopeTimerState::default()
        };
        assert!(matches!(
            EnvelopeTimer::restore_state(state),
            Err(EnvelopeTimerError::InvalidState(_))
        ));

        let state = EnvelopeTimerState {
            shift_lock: TIMER_SHIFT_LOCK_LIMIT + 1,
            ..EnvelopeTimerState::default()
        };
        assert!(matches!(
            EnvelopeTimer::restore_state(state),
            Err(EnvelopeTimerError::InvalidState(_))
        ));
    }

    #[test]
    fn rate_phase_uses_the_four_by_four_high_increment_lut() {
        assert_eq!(STEP_HIGH[0], [0, 0, 0, 0]);
        assert_eq!(STEP_HIGH[1], [1, 0, 0, 0]);
        assert_eq!(STEP_HIGH[2], [1, 0, 1, 0]);
        assert_eq!(STEP_HIGH[3], [1, 1, 1, 0]);
    }
}
