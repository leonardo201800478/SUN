//! Scheduler adapter for one explicit SG-1000 Z80 M1 T-state at a time.
//!
//! The adapter reuses the SG-1000 machine's existing temporal bus and Scheduler.
//! It owns no clock and performs no memory access; the machine samples opcode data
//! at the dispatched T3 boundary and commits the CPU continuation at T4.

use crate::{Sg1000InternalCycleCompletion, Sg1000TemporalBus, Sg1000TemporalBusError};
use sega_temporal_core::MachineTime;
use sega_z80_cpu::{Z80M1Cycle, Z80M1CycleState, Z80M1Error, Z80M1Phase, Z80M1Pins};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sg1000TemporalM1State {
    pub cycle: Z80M1CycleState,
    pub next_continuation_id: u64,
    pub pending_continuation_id: Option<u64>,
    pub pending_deadline: Option<MachineTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sg1000TemporalM1Error {
    Bus(Sg1000TemporalBusError),
    Cycle(Z80M1Error),
    EventMismatch,
    ContinuationIdExhausted,
    CycleComplete,
    EventAlreadyPending,
    NoEventPending,
}

impl From<Sg1000TemporalBusError> for Sg1000TemporalM1Error {
    fn from(error: Sg1000TemporalBusError) -> Self {
        Self::Bus(error)
    }
}

impl From<Z80M1Error> for Sg1000TemporalM1Error {
    fn from(error: Z80M1Error) -> Self {
        Self::Cycle(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sg1000TemporalM1 {
    cycle: Z80M1Cycle,
    next_continuation_id: u64,
    pending_continuation_id: Option<u64>,
    pending_deadline: Option<MachineTime>,
}

impl Sg1000TemporalM1 {
    pub fn new(address: u16, i: u8, refresh: u8) -> Self {
        Self {
            cycle: Z80M1Cycle::new(address, i, refresh),
            next_continuation_id: 0,
            pending_continuation_id: None,
            pending_deadline: None,
        }
    }

    pub const fn phase(&self) -> Z80M1Phase {
        self.cycle.phase()
    }

    pub const fn pins(&self) -> Z80M1Pins {
        self.cycle.pins()
    }

    pub const fn opcode(&self) -> Option<u8> {
        self.cycle.opcode()
    }

    pub const fn refresh_after_m1(&self) -> u8 {
        self.cycle.refresh_after_m1()
    }

    pub fn schedule_next_tstate(
        &mut self,
        bus: &mut Sg1000TemporalBus,
    ) -> Result<MachineTime, Sg1000TemporalM1Error> {
        if self.phase() == Z80M1Phase::Complete {
            return Err(Sg1000TemporalM1Error::CycleComplete);
        }
        if self.pending_continuation_id.is_some() {
            return Err(Sg1000TemporalM1Error::EventAlreadyPending);
        }
        let next_id = self
            .next_continuation_id
            .checked_add(1)
            .ok_or(Sg1000TemporalM1Error::ContinuationIdExhausted)?;
        let deadline = bus.schedule_internal_cycle(self.next_continuation_id, 1)?;
        self.pending_continuation_id = Some(self.next_continuation_id);
        self.pending_deadline = Some(deadline);
        self.next_continuation_id = next_id;
        Ok(deadline)
    }

    pub fn complete_tstate(
        &mut self,
        completion: Sg1000InternalCycleCompletion,
        wait_asserted: bool,
    ) -> Result<Z80M1Phase, Sg1000TemporalM1Error> {
        let Some(expected_id) = self.pending_continuation_id else {
            return Err(Sg1000TemporalM1Error::NoEventPending);
        };
        if completion.continuation_id != expected_id
            || self.pending_deadline != Some(completion.completed_at)
        {
            return Err(Sg1000TemporalM1Error::EventMismatch);
        }
        let phase = self.cycle.advance_tstate(wait_asserted)?;
        self.pending_continuation_id = None;
        self.pending_deadline = None;
        Ok(phase)
    }

    pub fn sample_opcode(&mut self, opcode: u8) -> Result<(), Sg1000TemporalM1Error> {
        self.cycle.sample_opcode(opcode)?;
        Ok(())
    }

    pub const fn export_state(&self) -> Sg1000TemporalM1State {
        Sg1000TemporalM1State {
            cycle: self.cycle.export_state(),
            next_continuation_id: self.next_continuation_id,
            pending_continuation_id: self.pending_continuation_id,
            pending_deadline: self.pending_deadline,
        }
    }

    pub fn restore_state(state: Sg1000TemporalM1State) -> Result<Self, Sg1000TemporalM1Error> {
        if state
            .pending_continuation_id
            .is_some_and(|id| id >= state.next_continuation_id)
            || state.pending_continuation_id.is_some() != state.pending_deadline.is_some()
            || (state.pending_continuation_id.is_some()
                && state.cycle.phase == Z80M1Phase::Complete)
        {
            return Err(Sg1000TemporalM1Error::EventMismatch);
        }
        Ok(Self {
            cycle: Z80M1Cycle::restore_state(state.cycle)?,
            next_continuation_id: state.next_continuation_id,
            pending_continuation_id: state.pending_continuation_id,
            pending_deadline: state.pending_deadline,
        })
    }
}
