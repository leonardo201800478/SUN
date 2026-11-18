//! YM2413 register, clock, and candidate phase-generator foundation.
//!
//! The phase generator is a standalone candidate model. It does not yet
//! synthesize PCM or participate in the 18-slot operator pipeline.

pub mod phase;
pub use phase::{
    PhaseGenerator, PhaseGeneratorError, PhaseGeneratorState, YM2413_ML_TABLE,
    YM2413_PG_STATE_VERSION, YM2413_PHASE_BITS, YM2413_PHASE_MASK, YM2413_PM_POSITIONS,
    YM2413_PM_TABLE,
};

use core::fmt;

