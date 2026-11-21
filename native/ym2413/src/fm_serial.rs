//! Composed serial FM service: PG, operator pipeline, core-owned EG boundary.
//!
//! This service preserves the core as the sole EG authority. It runs the
//! pre-EG OPLL stages, lets Ym2413Core perform exactly one EG tick, then runs
//! DoLFO and PreparePatch2 in the post-EG hook.

use core::fmt;

use crate::envelope_sequencer::{EnvelopeSequencerState, EnvelopeSequencerTick};
use crate::operator_serial::{
    Ym2413OperatorSerialError, Ym2413OperatorSerialService, Ym2413OperatorSerialState,
};
use crate::pg_serial::{
    Ym2413PgSerialError, Ym2413PgSerialService, Ym2413PgSerialServiceState,
};
use crate::slot_service::{Ym2413SlotService, Ym2413SlotServiceContext};
use crate::synthesis::Ym2413SynthesisInitialState;

pub const YM2413_FM_SERIAL_STATE_VERSION: u16 = 1;

/// Errors raised by the composed serial FM service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ym2413FmSerialError {
    InvalidState,
    Pg(Ym2413PgSerialError),
    Operator(Ym2413OperatorSerialError),
}

impl From<Ym2413PgSerialError> for Ym2413FmSerialError {
    fn from(value: Ym2413PgSerialError) -> Self {
        Self::Pg(value)
    }
}

impl From<Ym2413OperatorSerialError> for Ym2413FmSerialError {
    fn from(value: Ym2413OperatorSerialError) -> Self {
        Self::Operator(value)
    }
}

impl fmt::Display for Ym2413FmSerialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState => formatter.write_str("invalid YM2413 serial FM state"),
            Self::Pg(error) => write!(formatter, "YM2413 PG serial error: {error}"),
            Self::Operator(error) => write!(formatter, "YM2413 operator serial error: {error:?}"),
        }
    }
}

impl std::error::Error for Ym2413FmSerialError {}

/// Complete resumable state for the composed PG + operator serial service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ym2413FmSerialServiceState {
    pub format_version: u16,
    pub pg: Ym2413PgSerialServiceState,
    pub operator: Ym2413OperatorSerialState,
}

/// Composed FM service. PG and operator state are owned independently and
/// cloned transactionally by the core's slot-service block advancement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ym2413FmSerialService {
    pg: Ym2413PgSerialService,
    operator: Ym2413OperatorSerialService,
}

impl Ym2413FmSerialService {
    /// Compose a previously initialized PG service with a reset operator pipeline.
    pub const fn from_pg_service(pg: Ym2413PgSerialService) -> Self {
        Self {
            pg,
            operator: Ym2413OperatorSerialService::new(),
        }
    }

    /// Construct the complete serial FM service from a synthesis-bank state.
    pub fn with_initial_state(
        initial: Ym2413SynthesisInitialState,
    ) -> Result<Self, Ym2413FmSerialError> {
        Ok(Self::from_pg_service(
            Ym2413PgSerialService::with_initial_state(initial)?,
        ))
    }

    /// Return the PG service for phase-state inspection and diagnostics.
    pub const fn pg_service(&self) -> &Ym2413PgSerialService {
        &self.pg
    }

    /// Return the serial operator service for latch inspection and diagnostics.
    pub const fn operator_service(&self) -> &Ym2413OperatorSerialService {
        &self.operator
    }

    /// Export PG and operator latches as one resumable FM state.
    pub const fn export_state(&self) -> Ym2413FmSerialServiceState {
        Ym2413FmSerialServiceState {
            format_version: YM2413_FM_SERIAL_STATE_VERSION,
            pg: self.pg.export_state(),
            operator: self.operator.export_state(),
        }
    }

    /// Restore both serial pipelines atomically after validating their snapshots.
    pub fn restore_state(
        state: Ym2413FmSerialServiceState,
    ) -> Result<Self, Ym2413FmSerialError> {
        if state.format_version != YM2413_FM_SERIAL_STATE_VERSION {
            return Err(Ym2413FmSerialError::InvalidState);
        }
        Ok(Self {
            pg: Ym2413PgSerialService::restore_state(state.pg)?,
            operator: Ym2413OperatorSerialService::restore_state(state.operator)?,
        })
    }
}

impl Ym2413SlotService for Ym2413FmSerialService {
    type Error = Ym2413FmSerialError;

    fn before_eg_update(
        &mut self,
        context: &Ym2413SlotServiceContext,
    ) -> Result<(), Self::Error> {
        self.pg.ensure_melodic_mode(context)?;

        // PreparePatch1 latches the current slot's TL/KSL contribution.
        let next_eg_ksltl = self.operator.prepare_patch1(context)?;
        self.operator
            .sample_am_lfo_output(context.event.cycle)?;

        // OPLL_Clock order: PhaseGenerate -> Operator -> PhaseCalcIncrement.
        self.pg.generate_phase(context)?;
        let parameters = self.pg.parameter_latch();
        self.operator.clock_operator(
            context.event.cycle,
            self.pg.pg_output(),
            parameters.feedback,
        )?;
        self.pg.calculate_increment()?;

        // EnvelopeOutput -> EnvelopeKSLTL -> DoEG. The EG level is read from
        // the core's pre-update state; the core advances it once after this hook.
        self.operator
            .clock_envelope_output(context, parameters.am_enabled)?;
        self.operator.latch_eg_ksltl(next_eg_ksltl);
        self.operator.clock_eg_silent(context)?;
        Ok(())
    }

    fn after_eg_update(
        &mut self,
        context: &Ym2413SlotServiceContext,
        tick: EnvelopeSequencerTick,
        post_state: EnvelopeSequencerState,
    ) -> Result<(), Self::Error> {
        // DoLFO's AM side samples the PM counter before PG increments it.
        self.operator.clock_am_lfo(
            context.event.cycle,
            context.registers[0x0f],
            self.pg.pm_lfo_counter(),
        )?;

        // PG service performs the PM side of DoLFO and then PreparePatch2.
        <Ym2413PgSerialService as Ym2413SlotService>::after_eg_update(
            &mut self.pg,
            context,
            tick,
            post_state,
        )?;
        Ok(())
    }
}
