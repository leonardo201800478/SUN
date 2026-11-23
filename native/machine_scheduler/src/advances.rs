//! Allocation-free storage for the clock advances of one dispatch.
//!
//! A dispatch reports one [`ClockAdvance`] per registered clock domain. Every
//! SMS-family profile registers three domains (Z80, PSG, VDP) and the Japanese
//! Master System registers four (plus YM2413), so a `Vec` here allocated on
//! every single dispatch — once per scheduler event, which is roughly 50.000
//! allocations per emulated frame.
//!
//! [`ClockAdvances`] keeps up to [`INLINE_CLOCK_ADVANCES`] advances inline and
//! spills to the heap only when a machine registers more domains than that. The
//! fallback is explicit and lossless: an over-capacity dispatch never truncates
//! the sequence and never reorders it.
//!
//! Safety: the inline slots are `MaybeUninit` because `ClockAdvance` is `Copy`
//! and has no drop glue, so a partially filled buffer cannot leak or double
//! free. The static assertion below turns any future `Drop` implementation into
//! a compile error instead of a silent leak.

use std::fmt;
use std::mem::MaybeUninit;
use std::ops::Deref;

use crate::ClockAdvance;

/// Number of clock advances stored without touching the heap.
///
/// Four covers every registered SMS-family profile, including the Japanese
/// Master System with its integrated YM2413. Machines with more domains keep
/// working through the spill path.
pub const INLINE_CLOCK_ADVANCES: usize = 4;

/// `ClockAdvance` must stay trivially copyable for the inline slots to be sound.
const _: () = assert!(
    !std::mem::needs_drop::<ClockAdvance>(),
    "ClockAdvance must not implement Drop: inline slots rely on Copy semantics"
);

/// The clock advances reported by one scheduler dispatch.
///
/// Dereferences to `[ClockAdvance]`, so indexing, iteration, `len()` and
/// comparisons keep working exactly as they did when this was a `Vec`.
pub struct ClockAdvances {
    inline: [MaybeUninit<ClockAdvance>; INLINE_CLOCK_ADVANCES],
    inline_len: usize,
    spill: Vec<ClockAdvance>,
}

impl ClockAdvances {
    /// Create an empty container. Does not allocate.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inline: [const { MaybeUninit::uninit() }; INLINE_CLOCK_ADVANCES],
            inline_len: 0,
            spill: Vec::new(),
        }
    }

    /// Append one advance, preserving insertion order.
    ///
    /// Beyond the inline capacity the sequence moves to the heap; it is never
    /// truncated.
    pub fn push(&mut self, advance: ClockAdvance) {
        if self.spill.is_empty() && self.inline_len < INLINE_CLOCK_ADVANCES {
            self.inline[self.inline_len].write(advance);
            self.inline_len += 1;
            return;
        }
        if self.spill.is_empty() {
            // First overflow: migrate the inline prefix so the sequence stays
            // in insertion order.
            self.spill.reserve(INLINE_CLOCK_ADVANCES + 1);
            for slot in &self.inline[..self.inline_len] {
                // SAFETY: the first `inline_len` slots were written by `push`.
                self.spill.push(unsafe { slot.assume_init() });
            }
            self.inline_len = 0;
        }
        self.spill.push(advance);
    }

    /// Discard every stored advance without releasing the spill capacity.
    ///
    /// Used by callers that reuse one buffer across dispatches.
    pub fn clear(&mut self) {
        self.inline_len = 0;
        self.spill.clear();
    }

    /// Number of stored advances.
    #[must_use]
    pub fn len(&self) -> usize {
        if self.spill.is_empty() {
            self.inline_len
        } else {
            self.spill.len()
        }
    }

    /// Whether no advance is stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether this dispatch needed the heap fallback.
    ///
    /// Exposed so tests and benchmarks can prove the common path stays inline.
    #[must_use]
    pub fn spilled(&self) -> bool {
        !self.spill.is_empty()
    }

    /// Borrow the advances as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[ClockAdvance] {
        if self.spill.is_empty() {
            // SAFETY: the first `inline_len` slots were written by `push`, and
            // `inline_len` never exceeds the array length.
            unsafe {
                std::slice::from_raw_parts(
                    self.inline.as_ptr().cast::<ClockAdvance>(),
                    self.inline_len,
                )
            }
        } else {
            &self.spill
        }
    }

    /// Copy the advances into an owned vector.
    ///
    /// Only for cold paths that still expose `Vec<ClockAdvance>` in their
    /// public signature.
    #[must_use]
    pub fn to_vec(&self) -> Vec<ClockAdvance> {
        self.as_slice().to_vec()
    }
}

impl Default for ClockAdvances {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for ClockAdvances {
    type Target = [ClockAdvance];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl AsRef<[ClockAdvance]> for ClockAdvances {
    fn as_ref(&self) -> &[ClockAdvance] {
        self.as_slice()
    }
}

impl Clone for ClockAdvances {
    fn clone(&self) -> Self {
        let mut clone = Self::new();
        for advance in self.as_slice() {
            clone.push(*advance);
        }
        clone
    }
}

impl PartialEq for ClockAdvances {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for ClockAdvances {}

impl fmt::Debug for ClockAdvances {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_slice().fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::{ClockAdvances, INLINE_CLOCK_ADVANCES};
    use crate::{ClockAdvance, ClockDomainId};
    use sega_temporal_core::{ClockDomain, ClockDomainAdvance, ClockRatio, MachineTime};

    fn advance(id: u16, elapsed: u128) -> ClockAdvance {
        let ratio = ClockRatio::new(1, 3).expect("ratio");
        let domain = ClockDomain::new(ratio, MachineTime::from_ticks(0));
        let preview: ClockDomainAdvance = domain
            .preview_advance(MachineTime::from_ticks(elapsed * 3))
            .expect("preview");
        ClockAdvance::from_preview(ClockDomainId(id), preview)
    }

    #[test]
    fn empty_container_reports_zero_and_stays_inline() {
        let advances = ClockAdvances::new();
        assert_eq!(advances.len(), 0);
        assert!(advances.is_empty());
        assert!(!advances.spilled());
        assert_eq!(advances.as_slice(), &[]);
    }

    #[test]
    fn inline_capacity_preserves_insertion_order() {
        let mut advances = ClockAdvances::new();
        for index in 0..INLINE_CLOCK_ADVANCES {
            advances.push(advance(index as u16, index as u128 + 1));
        }
        assert!(!advances.spilled());
        assert_eq!(advances.len(), INLINE_CLOCK_ADVANCES);
        for (index, stored) in advances.iter().enumerate() {
            assert_eq!(stored.domain, ClockDomainId(index as u16));
        }
    }

    #[test]
    fn overflow_spills_without_truncating_or_reordering() {
        let total = INLINE_CLOCK_ADVANCES + 5;
        let mut advances = ClockAdvances::new();
        for index in 0..total {
            advances.push(advance(index as u16, index as u128 + 1));
        }
        assert!(advances.spilled());
        assert_eq!(advances.len(), total);
        for (index, stored) in advances.iter().enumerate() {
            assert_eq!(stored.domain, ClockDomainId(index as u16));
        }
    }

    #[test]
    fn clone_and_eq_follow_slice_semantics() {
        let mut left = ClockAdvances::new();
        left.push(advance(1, 4));
        left.push(advance(2, 7));
        let right = left.clone();
        assert_eq!(left, right);
        assert_eq!(format!("{left:?}"), format!("{:?}", left.as_slice()));
        assert_eq!(left.to_vec(), left.as_slice().to_vec());
    }
}
