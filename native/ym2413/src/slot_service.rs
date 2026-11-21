//! Internal serial-slot service contract between the YM2413 core and synthesis.
//!
//! The host never receives per-slot callbacks. Ym2413Core invokes this service
//! internally while advancing coarse-grained input-clock blocks.

use crate::Ym2413Error;
use crate::envelope_sequencer::{EnvelopeSequencerState, EnvelopeSequencerTick, EnvelopeSlotInput};
use crate::slot::Ym2413SlotServiceEvent;

/// Read-only snapshot at the serial boundary after mode latches have shifted
/// and before the core advances the EG operator for this cycle.
#[expect(
    dead_code,
    reason = "Staged FM-1B context; consumed by the serial synthesis service integration."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ym2413SlotServiceContext {
    pub event: Ym2413SlotServiceEvent,
    /// Register image after the mode-write stage and before the normal write stage.
    pub registers: [u8; crate::YM2413_REGISTER_COUNT],
    /// Decoded input that will be consumed by the core-owned serial EG.
    pub envelope_input: EnvelopeSlotInput,
    /// Complete pre-update serial EG state, including DOKON and key pipelines.
    pub envelope_state: EnvelopeSequencerState,
    /// Post-shift/post-mode-write value used by Nuked's phase-reset selector.
    pub rm_enable: u8,
    pub rm_select: u8,
}

/// Internal service for the stages surrounding the core-owned serial EG update.
///
/// Implementations may update PG/operator/feedback/LFO state, but must not tick
/// or reconstruct the serial EG. The core performs exactly one EG update
/// between these two callbacks.
#[expect(
    dead_code,
    reason = "Staged FM-1B service seam; consumed by the serial synthesis service integration."
)]
pub(crate) trait Ym2413SlotService {
    type Error;

    /// Run PG, Operator, phase-increment, EG-output and KSL/TL stages before
    /// the current serial EG operator is advanced.
    fn before_eg_update(&mut self, context: &Ym2413SlotServiceContext) -> Result<(), Self::Error>;

    /// Run post-EG stages (LFO/rhythm/output preparation) after the one core EG
    /// update and before the timed register write is applied.
    fn after_eg_update(
        &mut self,
        context: &Ym2413SlotServiceContext,
        tick: EnvelopeSequencerTick,
        post_state: EnvelopeSequencerState,
    ) -> Result<(), Self::Error>;
}

/// Core/service error boundary for the internal slot callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ym2413SlotServiceAdvanceError<E> {
    Core(Ym2413Error),
    #[expect(
        dead_code,
        reason = "Constructed by the staged serial synthesis service hook."
    )]
    Service(E),
}
