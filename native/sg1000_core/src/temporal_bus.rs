//! Transactional SG-1000 memory bus for the Scheduler-driven path.
//!
//! Memory reads and writes become visible only at their scheduled completion
//! boundary. Completion remains latched until acknowledgement so redispatch
//! cannot repeat a write side effect.

use core::fmt;

use crate::{
    DEFAULT_SG1000_CPU_CLOCK_HZ, IoError, MemoryConfig, MemoryError, PsgError,
    SG1000_PSG_CLOCK_DOMAIN_ID, SG1000_VDP_CLOCK_DOMAIN_ID, Sg1000Io, Sg1000Memory,
    Sg1000TemporalCoordinator, Sg1000TemporalCoordinatorError, Sg1000TemporalCoordinatorState,
    Sg1000TemporalEvent, Sn76489, Tms9918A,
};
use sega_temporal_core::MachineTime;
use sega_z80_cpu::{Z80_INT_ACK_AUTOMATIC_WAIT_TSTATES, Z80BusOperation, Z80BusRequest};

pub const SG1000_TEMPORAL_BUS_STATE_VERSION: u16 = 3;
const DEFAULT_INTERRUPT_VECTOR: u8 = 0xFF;
const DEFAULT_AUDIO_SAMPLE_RATE: u64 = 48_000;

fn elapsed_tstates(
    started_at: MachineTime,
    completed_at: MachineTime,
    master_ticks_per_tstate: u64,
) -> Option<u32> {
    let scale = u128::from(master_ticks_per_tstate);
    if scale == 0 {
        return None;
    }
    let elapsed = completed_at.checked_duration_since(started_at)?;
    if elapsed == 0 || elapsed % scale != 0 {
        return None;
    }
    u32::try_from(elapsed / scale).ok()
}

fn valid_bus_deadline(
    started_at: MachineTime,
    deadline: MachineTime,
    master_ticks_per_tstate: u64,
    minimum_tstates: u16,
) -> bool {
    minimum_tstates > 0
        && elapsed_tstates(started_at, deadline, master_ticks_per_tstate)
            .is_some_and(|total| total >= u32::from(minimum_tstates))
}

fn valid_internal_deadline(
    started_at: MachineTime,
    deadline: MachineTime,
    master_ticks_per_tstate: u64,
) -> bool {
    elapsed_tstates(started_at, deadline, master_ticks_per_tstate).is_some()
}

fn is_supported_bus_operation(operation: Z80BusOperation) -> bool {
    matches!(
        operation,
        Z80BusOperation::OpcodeFetch
            | Z80BusOperation::MemoryRead
            | Z80BusOperation::MemoryWrite { .. }
            | Z80BusOperation::PortRead
            | Z80BusOperation::PortWrite { .. }
            | Z80BusOperation::InterruptAcknowledge
    )
}

/// One memory-bus response latched at its scheduler deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sg1000BusCompletion {
    pub request_id: u64,
    pub data: Option<u8>,
    pub completed_at: MachineTime,
}

/// Completion of a CPU-internal interval without a memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sg1000InternalCycleCompletion {
    pub continuation_id: u64,
    pub completed_at: MachineTime,
}

/// One event dispatched by the SG-1000 temporal bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sg1000TemporalDispatch {
    BusCompletion(Sg1000BusCompletion),
    InternalCycleComplete(Sg1000InternalCycleCompletion),
}

/// Complete in-memory continuation of the temporal memory bus.
#[derive(Debug, Clone, PartialEq)]
pub struct Sg1000TemporalBusState {
    pub format_version: u16,
    pub interrupt_vector: u8,
    pub memory: Sg1000Memory,
    pub io: Sg1000Io,
    pub vdp: Tms9918A,
    pub psg: Sn76489,
    pub coordinator: Sg1000TemporalCoordinatorState,
    pub pending_request: Option<Z80BusRequest>,
    pub request_started_at: Option<MachineTime>,
    pub completion: Option<Sg1000BusCompletion>,
    pub pending_internal_cycle: Option<u64>,
    pub internal_cycle_started_at: Option<MachineTime>,
    pub internal_completion: Option<Sg1000InternalCycleCompletion>,
    #[cfg(test)]
    pub write_dispatch_count: u64,
}

/// Errors from the SG-1000 transactional memory bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sg1000TemporalBusError {
    Memory(MemoryError),
    Audio(PsgError),
    Coordinator(Sg1000TemporalCoordinatorError),
    Busy,
    NoPendingRequest,
    WrongRequestId { expected: u64, received: u64 },
    UnsupportedOperation,
    InvalidState,
}

impl fmt::Display for Sg1000TemporalBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Memory(error) => write!(f, "SG-1000 memory initialization failed: {error}"),
            Self::Audio(error) => write!(f, "SG-1000 PSG initialization failed: {error}"),
            Self::Coordinator(error) => write!(f, "SG-1000 temporal coordinator failed: {error}"),
            Self::Busy => f.write_str("SG-1000 temporal bus already owns an unacknowledged event"),
            Self::NoPendingRequest => f.write_str("SG-1000 temporal bus has no pending request"),
            Self::WrongRequestId { expected, received } => write!(
                f,
                "SG-1000 temporal bus expected request {expected}, received {received}"
            ),
            Self::UnsupportedOperation => f.write_str("unsupported SG-1000 temporal bus operation"),
            Self::InvalidState => f.write_str("SG-1000 temporal bus snapshot is inconsistent"),
        }
    }
}

impl std::error::Error for Sg1000TemporalBusError {}

impl From<MemoryError> for Sg1000TemporalBusError {
    fn from(error: MemoryError) -> Self {
        Self::Memory(error)
    }
}

impl From<PsgError> for Sg1000TemporalBusError {
    fn from(error: PsgError) -> Self {
        Self::Audio(error)
    }
}

impl From<Sg1000TemporalCoordinatorError> for Sg1000TemporalBusError {
    fn from(error: Sg1000TemporalCoordinatorError) -> Self {
        Self::Coordinator(error)
    }
}

/// SG-1000 memory and its scheduler-owned transaction timeline.
#[derive(Debug, Clone)]
pub struct Sg1000TemporalBus {
    memory: Sg1000Memory,
    io: Sg1000Io,
    vdp: Tms9918A,
    psg: Sn76489,
    coordinator: Sg1000TemporalCoordinator,
    pending_request: Option<Z80BusRequest>,
    request_started_at: Option<MachineTime>,
    completion: Option<Sg1000BusCompletion>,
    pending_internal_cycle: Option<u64>,
    internal_cycle_started_at: Option<MachineTime>,
    internal_completion: Option<Sg1000InternalCycleCompletion>,
    interrupt_vector: u8,
    #[cfg(test)]
    write_dispatch_count: u64,
}

impl Sg1000TemporalBus {
    /// Construct the temporal bus with the provisional default SG-1000 PSG profile.
    pub fn new(
        rom: impl Into<Vec<u8>>,
        memory_config: MemoryConfig,
        master_ticks_per_tstate: u64,
    ) -> Result<Self, Sg1000TemporalBusError> {
        Self::new_with_audio(
            rom,
            memory_config,
            master_ticks_per_tstate,
            DEFAULT_SG1000_CPU_CLOCK_HZ,
            DEFAULT_SG1000_CPU_CLOCK_HZ,
            DEFAULT_AUDIO_SAMPLE_RATE,
        )
    }

    /// Construct the temporal bus with explicit CPU/PSG oscillator frequencies and PCM rate.
    ///
    /// The oscillator values are configuration inputs. The default profile is provisional
    /// and is not a claim of board-level clock fidelity. PSG progression uses the exact
    /// rational PSG/CPU ratio registered with the MachineScheduler.
    pub fn new_with_audio(
        rom: impl Into<Vec<u8>>,
        memory_config: MemoryConfig,
        master_ticks_per_tstate: u64,
        cpu_clock_hz: u64,
        psg_clock_hz: u64,
        sample_rate: u64,
    ) -> Result<Self, Sg1000TemporalBusError> {
        let rom = rom.into();
        let memory = Sg1000Memory::new(&rom, memory_config)?;
        let io = Sg1000Io::default();
        let vdp = Tms9918A::new();
        let psg = Sn76489::new(psg_clock_hz, sample_rate)?;
        let coordinator = Sg1000TemporalCoordinator::new_with_clock_profile(
            master_ticks_per_tstate,
            cpu_clock_hz,
            psg_clock_hz,
        )?;
        Ok(Self {
            memory,
            io,
            vdp,
            psg,
            coordinator,
            pending_request: None,
            request_started_at: None,
            completion: None,
            pending_internal_cycle: None,
            internal_cycle_started_at: None,
            internal_completion: None,
            interrupt_vector: DEFAULT_INTERRUPT_VECTOR,
            #[cfg(test)]
            write_dispatch_count: 0,
        })
    }

    /// Schedule one memory request. WAIT is fixture input, never a physical signal.
    pub fn begin_request(
        &mut self,
        request: Z80BusRequest,
        fixture_wait_tstates: u32,
    ) -> Result<MachineTime, Sg1000TemporalBusError> {
        if self.pending_request.is_some()
            || self.completion.is_some()
            || self.pending_internal_cycle.is_some()
            || self.internal_completion.is_some()
        {
            return Err(Sg1000TemporalBusError::Busy);
        }
        if !is_supported_bus_operation(request.operation) {
            return Err(Sg1000TemporalBusError::UnsupportedOperation);
        }
        let started_at = self.coordinator.now();
        let automatic_waits = if request.operation == Z80BusOperation::InterruptAcknowledge {
            u32::from(Z80_INT_ACK_AUTOMATIC_WAIT_TSTATES)
        } else {
            0
        };
        let total_wait_tstates = fixture_wait_tstates
            .checked_add(automatic_waits)
            .ok_or(Sg1000TemporalBusError::InvalidState)?;
        let deadline = self.coordinator.schedule_bus_completion(
            request.id,
            request.base_tstates,
            total_wait_tstates,
        )?;
        self.pending_request = Some(request);
        self.request_started_at = Some(started_at);
        Ok(deadline)
    }

    /// Schedule a CPU-internal interval on the same machine timeline.
    pub fn schedule_internal_cycle(
        &mut self,
        continuation_id: u64,
        tstates: u32,
    ) -> Result<MachineTime, Sg1000TemporalBusError> {
        if self.pending_request.is_some()
            || self.completion.is_some()
            || self.pending_internal_cycle.is_some()
            || self.internal_completion.is_some()
        {
            return Err(Sg1000TemporalBusError::Busy);
        }
        let started_at = self.coordinator.now();
        let deadline = self
            .coordinator
            .schedule_internal_cycle(continuation_id, tstates)?;
        self.pending_internal_cycle = Some(continuation_id);
        self.internal_cycle_started_at = Some(started_at);
        Ok(deadline)
    }

    /// Dispatch the next deadline and apply bus/peripheral side effects exactly once.
    pub fn dispatch_next(
        &mut self,
    ) -> Result<Option<Sg1000TemporalDispatch>, Sg1000TemporalBusError> {
        if let Some(completion) = self.completion {
            return Ok(Some(Sg1000TemporalDispatch::BusCompletion(completion)));
        }
        if let Some(completion) = self.internal_completion {
            return Ok(Some(Sg1000TemporalDispatch::InternalCycleComplete(
                completion,
            )));
        }
        let Some(dispatch) = self.coordinator.dispatch_next()? else {
            return Ok(None);
        };
        if dispatch.clock_advances.len() != 3 {
            return Err(Sg1000TemporalBusError::InvalidState);
        }
        let psg_advance = dispatch
            .clock_advances
            .iter()
            .find(|advance| advance.domain == SG1000_PSG_CLOCK_DOMAIN_ID)
            .ok_or(Sg1000TemporalBusError::InvalidState)?;
        self.psg.advance_clocks(
            u64::try_from(psg_advance.elapsed_cycles)
                .map_err(|_| Sg1000TemporalBusError::InvalidState)?,
        );
        let vdp_advance = dispatch
            .clock_advances
            .iter()
            .find(|advance| advance.domain == SG1000_VDP_CLOCK_DOMAIN_ID)
            .ok_or(Sg1000TemporalBusError::InvalidState)?;
        self.vdp.advance(
            u64::try_from(vdp_advance.elapsed_cycles)
                .map_err(|_| Sg1000TemporalBusError::InvalidState)?,
        );
        match dispatch.event.payload {
            Sg1000TemporalEvent::BusCompletion { request_id } => {
                let request = self
                    .pending_request
                    .ok_or(Sg1000TemporalBusError::NoPendingRequest)?;
                if request.id != request_id {
                    return Err(Sg1000TemporalBusError::WrongRequestId {
                        expected: request.id,
                        received: request_id,
                    });
                }
                let data = match request.operation {
                    Z80BusOperation::OpcodeFetch | Z80BusOperation::MemoryRead => {
                        Some(self.memory.read(request.address))
                    }
                    Z80BusOperation::InterruptAcknowledge => Some(self.interrupt_vector),
                    Z80BusOperation::MemoryWrite { value } => {
                        self.memory.write(request.address, value);
                        #[cfg(test)]
                        {
                            self.write_dispatch_count += 1;
                        }
                        None
                    }
                    Z80BusOperation::PortRead => Some(self.read_port(request.address)),
                    Z80BusOperation::PortWrite { value } => {
                        self.write_port(request.address, value, dispatch.event.timestamp)?;
                        None
                    }
                };
                let completion = Sg1000BusCompletion {
                    request_id,
                    data,
                    completed_at: dispatch.event.timestamp,
                };
                self.completion = Some(completion);
                Ok(Some(Sg1000TemporalDispatch::BusCompletion(completion)))
            }
            Sg1000TemporalEvent::InternalCycleComplete { continuation_id } => {
                if self.pending_internal_cycle != Some(continuation_id) {
                    return Err(Sg1000TemporalBusError::InvalidState);
                }
                let completion = Sg1000InternalCycleCompletion {
                    continuation_id,
                    completed_at: dispatch.event.timestamp,
                };
                self.internal_completion = Some(completion);
                Ok(Some(Sg1000TemporalDispatch::InternalCycleComplete(
                    completion,
                )))
            }
        }
    }

    /// Consume a latched memory response exactly once.
    pub fn acknowledge_completion(
        &mut self,
        request_id: u64,
    ) -> Result<Sg1000BusCompletion, Sg1000TemporalBusError> {
        let completion = self
            .completion
            .ok_or(Sg1000TemporalBusError::NoPendingRequest)?;
        if completion.request_id != request_id {
            return Err(Sg1000TemporalBusError::WrongRequestId {
                expected: completion.request_id,
                received: request_id,
            });
        }
        self.completion = None;
        self.pending_request = None;
        self.request_started_at = None;
        Ok(completion)
    }

    /// Consume an internal interval after the CPU continuation commits.
    pub fn acknowledge_internal_completion(
        &mut self,
        continuation_id: u64,
    ) -> Result<Sg1000InternalCycleCompletion, Sg1000TemporalBusError> {
        let completion = self
            .internal_completion
            .ok_or(Sg1000TemporalBusError::NoPendingRequest)?;
        if completion.continuation_id != continuation_id
            || self.pending_internal_cycle != Some(continuation_id)
        {
            return Err(Sg1000TemporalBusError::InvalidState);
        }
        self.internal_completion = None;
        self.pending_internal_cycle = None;
        self.internal_cycle_started_at = None;
        Ok(completion)
    }

    pub const fn pending_request(&self) -> Option<Z80BusRequest> {
        self.pending_request
    }

    pub const fn request_started_at(&self) -> Option<MachineTime> {
        self.request_started_at
    }

    /// Set the external byte driven during an INT acknowledge cycle.
    /// This changes device input state without advancing scheduler time.
    pub fn set_interrupt_vector(&mut self, vector: u8) {
        self.interrupt_vector = vector;
    }

    pub const fn completion(&self) -> Option<Sg1000BusCompletion> {
        self.completion
    }

    pub const fn internal_cycle_started_at(&self) -> Option<MachineTime> {
        self.internal_cycle_started_at
    }

    pub const fn now(&self) -> MachineTime {
        self.coordinator.now()
    }

    pub const fn master_ticks_per_tstate(&self) -> u64 {
        self.coordinator.master_ticks_per_tstate()
    }

    pub const fn memory(&self) -> &Sg1000Memory {
        &self.memory
    }

    pub const fn io(&self) -> &Sg1000Io {
        &self.io
    }

    pub const fn vdp(&self) -> &Tms9918A {
        &self.vdp
    }

    pub const fn psg(&self) -> &Sn76489 {
        &self.psg
    }

    /// Configure the TMS9918A sprite-per-line diagnostic limit.
    pub fn set_sprite_limit_enabled(&mut self, enabled: bool) {
        self.vdp.set_sprite_limit_enabled(enabled);
    }

    pub fn sprite_limit_enabled(&self) -> bool {
        self.vdp.sprite_limit_enabled()
    }

    pub fn frame_number(&self) -> u64 {
        self.vdp.frame_number()
    }

    pub fn framebuffer(&self) -> &[u8] {
        self.vdp.framebuffer()
    }

    /// Update the logical active-low controller snapshot without advancing time.
    pub fn set_joystick(&mut self, player: u8, value: u8) -> Result<(), IoError> {
        self.io.set_joystick(player, value)
    }

    pub fn drain_psg_write_events(&mut self) -> Vec<crate::PsgWriteEvent> {
        self.io.drain_psg_write_events()
    }

    pub fn drain_audio_samples(&mut self, max_samples: Option<usize>) -> Vec<f64> {
        self.psg.drain_samples(max_samples)
    }

    fn read_port(&mut self, port: u16) -> u8 {
        match (port as u8) & 0xC1 {
            0x80 | 0x81 => self.vdp.read_port(port),
            _ => self.io.read_port(port),
        }
    }

    fn write_port(
        &mut self,
        port: u16,
        value: u8,
        completed_at: MachineTime,
    ) -> Result<(), Sg1000TemporalBusError> {
        let tstate = u64::try_from(
            completed_at.ticks() / u128::from(self.coordinator.master_ticks_per_tstate()),
        )
        .map_err(|_| Sg1000TemporalBusError::InvalidState)?;
        match (port as u8) & 0xC1 {
            0x80 | 0x81 => self.vdp.write_port(port, value),
            _ if (0x40..=0x7F).contains(&(port as u8)) => {
                self.io.write_port_at(port, value, tstate);
                self.psg.write(value);
            }
            _ => self.io.write_port_at(port, value, tstate),
        }
        Ok(())
    }

    #[cfg(test)]
    pub const fn write_dispatch_count_for_test(&self) -> u64 {
        self.write_dispatch_count
    }

    #[cfg(test)]
    pub(crate) fn write_memory_for_test(&mut self, address: u16, value: u8) {
        self.memory.write(address, value);
    }

    /// Export memory, I/O devices, Scheduler and all pending transaction state.
    pub fn export_state(&self) -> Sg1000TemporalBusState {
        Sg1000TemporalBusState {
            format_version: SG1000_TEMPORAL_BUS_STATE_VERSION,
            interrupt_vector: self.interrupt_vector,
            memory: self.memory.clone(),
            io: self.io.clone(),
            vdp: self.vdp.clone(),
            psg: self.psg.clone(),
            coordinator: self.coordinator.export_state(),
            pending_request: self.pending_request,
            request_started_at: self.request_started_at,
            completion: self.completion,
            pending_internal_cycle: self.pending_internal_cycle,
            internal_cycle_started_at: self.internal_cycle_started_at,
            internal_completion: self.internal_completion,
            #[cfg(test)]
            write_dispatch_count: self.write_dispatch_count,
        }
    }

    /// Restore only if device clocks, pending transactions and Scheduler events agree.
    pub fn restore_state(state: Sg1000TemporalBusState) -> Result<Self, Sg1000TemporalBusError> {
        if state.format_version != SG1000_TEMPORAL_BUS_STATE_VERSION {
            return Err(Sg1000TemporalBusError::InvalidState);
        }
        let coordinator = Sg1000TemporalCoordinator::restore_state(state.coordinator.clone())?;
        if state.psg.clock_hz() != state.coordinator.psg_clock_hz
            || state.psg.elapsed_clocks() as u128 != coordinator.psg_clock_snapshot()?.total_cycles
            || state.vdp.elapsed_tstates() != coordinator.vdp_clock_snapshot()?.total_cycles
        {
            return Err(Sg1000TemporalBusError::InvalidState);
        }
        let pending_events = &state.coordinator.scheduler.event_queue.pending;
        let scale = state.coordinator.master_ticks_per_tstate;
        let now = coordinator.now();
        let last_dispatch = state.coordinator.scheduler.last_dispatch_timestamp;

        match (
            state.pending_request,
            state.request_started_at,
            state.completion,
            state.pending_internal_cycle,
            state.internal_cycle_started_at,
            state.internal_completion,
        ) {
            (None, None, None, None, None, None) if pending_events.is_empty() => {}
            (Some(request), Some(started_at), None, None, None, None)
                if is_supported_bus_operation(request.operation)
                    && pending_events.len() == 1
                    && pending_events[0].payload
                        == (Sg1000TemporalEvent::BusCompletion {
                            request_id: request.id,
                        })
                    && started_at <= now
                    && pending_events[0].timestamp >= now
                    && valid_bus_deadline(
                        started_at,
                        pending_events[0].timestamp,
                        scale,
                        u16::from(request.base_tstates)
                            + if request.operation == Z80BusOperation::InterruptAcknowledge {
                                u16::from(Z80_INT_ACK_AUTOMATIC_WAIT_TSTATES)
                            } else {
                                0
                            },
                    ) => {}
            (Some(request), Some(started_at), Some(completion), None, None, None)
                if is_supported_bus_operation(request.operation)
                    && request.id == completion.request_id
                    && pending_events.is_empty()
                    && completion.completed_at == now
                    && last_dispatch == Some(completion.completed_at)
                    && valid_bus_deadline(
                        started_at,
                        completion.completed_at,
                        scale,
                        u16::from(request.base_tstates)
                            + if request.operation == Z80BusOperation::InterruptAcknowledge {
                                u16::from(Z80_INT_ACK_AUTOMATIC_WAIT_TSTATES)
                            } else {
                                0
                            },
                    ) => {}
            (None, None, None, Some(continuation_id), Some(started_at), None)
                if pending_events.len() == 1
                    && pending_events[0].payload
                        == (Sg1000TemporalEvent::InternalCycleComplete { continuation_id })
                    && started_at <= now
                    && pending_events[0].timestamp >= now
                    && valid_internal_deadline(started_at, pending_events[0].timestamp, scale) => {}
            (None, None, None, Some(continuation_id), Some(started_at), Some(completion))
                if continuation_id == completion.continuation_id
                    && pending_events.is_empty()
                    && completion.completed_at == now
                    && last_dispatch == Some(completion.completed_at)
                    && valid_internal_deadline(started_at, completion.completed_at, scale) => {}
            _ => return Err(Sg1000TemporalBusError::InvalidState),
        }

        Ok(Self {
            memory: state.memory,
            io: state.io,
            vdp: state.vdp,
            psg: state.psg,
            coordinator,
            pending_request: state.pending_request,
            request_started_at: state.request_started_at,
            completion: state.completion,
            pending_internal_cycle: state.pending_internal_cycle,
            internal_cycle_started_at: state.internal_cycle_started_at,
            internal_completion: state.internal_completion,
            interrupt_vector: state.interrupt_vector,
            #[cfg(test)]
            write_dispatch_count: state.write_dispatch_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Sg1000TemporalBus, Sg1000TemporalBusError, Sg1000TemporalDispatch};
    use crate::MemoryConfig;
    use sega_temporal_core::MachineTime;
    use sega_z80_cpu::{Z80BusOperation, Z80BusRequest};

    fn request(
        id: u64,
        operation: Z80BusOperation,
        address: u16,
        base_tstates: u8,
    ) -> Z80BusRequest {
        Z80BusRequest {
            id,
            operation,
            address,
            base_tstates,
        }
    }

    fn dispatch_write(
        bus: &mut Sg1000TemporalBus,
        id: u64,
        address: u16,
        value: u8,
        wait_tstates: u32,
    ) {
        bus.begin_request(
            request(id, Z80BusOperation::MemoryWrite { value }, address, 3),
            wait_tstates,
        )
        .unwrap();
        let Some(Sg1000TemporalDispatch::BusCompletion(completion)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected memory write completion");
        };
        assert_eq!(completion.request_id, id);
        assert_eq!(completion.data, None);
        bus.acknowledge_completion(id).unwrap();
    }

    #[test]
    fn memory_write_is_invisible_until_deadline_then_updates_mirrored_ram_once() {
        let mut bus = Sg1000TemporalBus::new(vec![0x00], MemoryConfig::default(), 2).unwrap();
        let deadline = bus
            .begin_request(
                request(1, Z80BusOperation::MemoryWrite { value: 0xA5 }, 0xC000, 3),
                2,
            )
            .unwrap();
        assert_eq!(deadline, MachineTime::from_ticks(10));
        assert_eq!(bus.memory().read(0xC000), 0);
        assert_eq!(bus.memory().read(0xC400), 0);
        assert_eq!(bus.now(), MachineTime::ZERO);

        let restored = Sg1000TemporalBus::restore_state(bus.export_state()).unwrap();
        let mut restored = restored;
        let Some(Sg1000TemporalDispatch::BusCompletion(completion)) =
            restored.dispatch_next().unwrap()
        else {
            panic!("expected memory write completion");
        };
        assert_eq!(completion.completed_at, deadline);
        assert_eq!(restored.memory().read(0xC000), 0xA5);
        assert_eq!(restored.memory().read(0xC400), 0xA5);
        assert_eq!(restored.acknowledge_completion(1).unwrap(), completion);
        assert_eq!(
            restored.acknowledge_completion(1),
            Err(Sg1000TemporalBusError::NoPendingRequest)
        );
        assert!(restored.dispatch_next().unwrap().is_none());
    }

    #[test]
    fn interrupt_acknowledge_uses_external_vector_and_automatic_waits() {
        let mut bus = Sg1000TemporalBus::new(vec![0; 0x4000], MemoryConfig::default(), 3).unwrap();
        bus.set_interrupt_vector(0xCF);
        let request = request(71, Z80BusOperation::InterruptAcknowledge, 0x1234, 3);
        let started_at = bus.now();
        let deadline = bus.begin_request(request, 1).unwrap();
        assert_eq!(deadline.checked_duration_since(started_at), Some(18));

        let restored = Sg1000TemporalBus::restore_state(bus.export_state()).unwrap();
        assert_eq!(restored.export_state(), bus.export_state());
        let Some(Sg1000TemporalDispatch::BusCompletion(completion)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected INT acknowledge completion");
        };
        assert_eq!(completion.data, Some(0xCF));
        assert_eq!(completion.completed_at, deadline);
    }

    #[test]
    fn memory_read_samples_the_value_at_dispatch_deadline() {
        let mut bus = Sg1000TemporalBus::new(vec![0x00], MemoryConfig::default(), 3).unwrap();
        bus.begin_request(request(2, Z80BusOperation::MemoryRead, 0xC000, 3), 1)
            .unwrap();
        bus.write_memory_for_test(0xC000, 0x5A);
        let Some(Sg1000TemporalDispatch::BusCompletion(completion)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected memory read completion");
        };
        assert_eq!(completion.data, Some(0x5A));
        bus.acknowledge_completion(2).unwrap();
    }

    #[test]
    fn rom_window_mirrors_and_ignores_scheduled_writes() {
        let mut bus = Sg1000TemporalBus::new(vec![0x42, 0x99], MemoryConfig::default(), 1).unwrap();
        dispatch_write(&mut bus, 3, 0x0000, 0xFF, 0);
        assert_eq!(bus.memory().read(0x0000), 0x42);
        assert_eq!(bus.memory().read(0x0002), 0x42);
        assert_eq!(bus.memory().read(0xBFFE), 0x42);
        assert_eq!(bus.memory().read(0xBFFF), 0x99);
    }

    #[test]
    fn configured_eight_kib_internal_ram_and_cartridge_ram_keep_their_mirrors() {
        let config = MemoryConfig {
            internal_ram_size: 0x2000,
            cartridge_ram_size: 0x0800,
            cartridge_ram_start: 0x8000,
            cartridge_ram_end: 0xBFFF,
        };
        let mut bus = Sg1000TemporalBus::new(vec![0x00], config, 1).unwrap();
        dispatch_write(&mut bus, 4, 0x8000, 0xA7, 0);
        assert_eq!(bus.memory().read(0x8000), 0xA7);
        assert_eq!(bus.memory().read(0xA000), 0xA7);
        dispatch_write(&mut bus, 5, 0xC000, 0x5C, 0);
        assert_eq!(bus.memory().read(0xC000), 0x5C);
        assert_eq!(bus.memory().read(0xE000), 0x5C);
        dispatch_write(&mut bus, 6, 0xBFFF, 0x3C, 0);
        assert_eq!(bus.memory().read(0xBFFF), 0x3C);
        assert_eq!(bus.memory().read(0x87FF), 0x3C);
    }

    #[test]
    fn one_two_and_eight_kib_cartridge_ram_mirror_inside_the_configured_window() {
        for (index, ram_size) in [0x0400_u16, 0x0800, 0x2000].into_iter().enumerate() {
            let config = MemoryConfig {
                cartridge_ram_size: usize::from(ram_size),
                cartridge_ram_start: 0x8000,
                cartridge_ram_end: 0xBFFF,
                ..MemoryConfig::default()
            };
            let mut bus = Sg1000TemporalBus::new(vec![0x00], config, 1).unwrap();
            let start_value = 0xA0 + index as u8;
            dispatch_write(&mut bus, 20 + index as u64 * 2, 0x8000, start_value, 0);
            assert_eq!(bus.memory().read(0x8000 + ram_size), start_value);

            let end_value = 0x50 + index as u8;
            dispatch_write(&mut bus, 21 + index as u64 * 2, 0xBFFF, end_value, 0);
            let mirrored_last = 0x8000 + (0x3FFF % ram_size);
            assert_eq!(bus.memory().read(mirrored_last), end_value);
            assert_eq!(bus.memory().read(0xBFFF), end_value);
        }
    }

    #[test]
    fn completion_latched_snapshot_restore_does_not_repeat_memory_write() {
        let mut bus = Sg1000TemporalBus::new(vec![0x00], MemoryConfig::default(), 2).unwrap();
        bus.begin_request(
            request(6, Z80BusOperation::MemoryWrite { value: 0x33 }, 0xC000, 3),
            0,
        )
        .unwrap();
        let first = bus.dispatch_next().unwrap().unwrap();
        let state = bus.export_state();
        let mut restored = Sg1000TemporalBus::restore_state(state.clone()).unwrap();
        assert_eq!(restored.dispatch_next().unwrap(), Some(first));
        assert_eq!(restored.export_state(), state);
        assert_eq!(restored.write_dispatch_count_for_test(), 1);
        assert_eq!(restored.memory().read(0xC000), 0x33);
        restored.acknowledge_completion(6).unwrap();
        assert!(restored.dispatch_next().unwrap().is_none());
        assert_eq!(
            restored.acknowledge_completion(6),
            Err(Sg1000TemporalBusError::NoPendingRequest)
        );
    }

    #[test]
    fn internal_cycle_is_latched_and_acknowledged_once() {
        let mut bus = Sg1000TemporalBus::new(vec![0x00], MemoryConfig::default(), 2).unwrap();
        let deadline = bus.schedule_internal_cycle(7, 1).unwrap();
        let Some(Sg1000TemporalDispatch::InternalCycleComplete(completion)) =
            bus.dispatch_next().unwrap()
        else {
            panic!("expected internal cycle completion");
        };
        assert_eq!(completion.completed_at, deadline);
        assert_eq!(
            bus.dispatch_next().unwrap(),
            Some(Sg1000TemporalDispatch::InternalCycleComplete(completion))
        );
        bus.acknowledge_internal_completion(7).unwrap();
        assert!(bus.dispatch_next().unwrap().is_none());
    }

    #[test]
    fn configured_psg_clock_advances_by_its_exact_ratio() {
        let mut bus = Sg1000TemporalBus::new_with_audio(
            vec![0x00],
            MemoryConfig::default(),
            2,
            1_000_000,
            2_000_000,
            48_000,
        )
        .unwrap();
        bus.schedule_internal_cycle(90, 5).unwrap();
        let Some(Sg1000TemporalDispatch::InternalCycleComplete(completion)) =
            bus.dispatch_next().unwrap()
        else {
            panic!("expected internal-cycle completion");
        };
        assert_eq!(completion.completed_at, MachineTime::from_ticks(10));
        assert_eq!(bus.psg().elapsed_clocks(), 10);
        assert_eq!(bus.vdp().cycles_into_line(), 5);

        let state = bus.export_state();
        let restored = Sg1000TemporalBus::restore_state(state.clone()).unwrap();
        assert_eq!(restored.export_state(), state);
        assert_eq!(restored.psg().clock_hz(), 2_000_000);
    }

    #[test]
    fn restore_rejects_device_clock_state_that_diverges_from_scheduler() {
        let mut bus = Sg1000TemporalBus::new(vec![0x00], MemoryConfig::default(), 1).unwrap();
        bus.schedule_internal_cycle(29, 4).unwrap();
        bus.dispatch_next().unwrap().unwrap();
        let state = bus.export_state();

        let mut psg_mismatch = state.clone();
        psg_mismatch.psg.advance_clocks(1);
        assert_eq!(
            Sg1000TemporalBus::restore_state(psg_mismatch).err(),
            Some(Sg1000TemporalBusError::InvalidState)
        );

        let mut vdp_mismatch = state;
        vdp_mismatch.vdp.advance(1);
        assert_eq!(
            Sg1000TemporalBus::restore_state(vdp_mismatch).err(),
            Some(Sg1000TemporalBusError::InvalidState)
        );
    }

    #[test]
    fn port_read_samples_controller_at_deadline_and_advances_all_device_domains() {
        let mut bus = Sg1000TemporalBus::new(vec![0x00], MemoryConfig::default(), 2).unwrap();
        bus.set_joystick(1, 0xFE).unwrap();
        let deadline = bus
            .begin_request(request(10, Z80BusOperation::PortRead, 0x00DC, 4), 1)
            .unwrap();

        let Some(Sg1000TemporalDispatch::BusCompletion(completion)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected controller port-read completion");
        };
        assert_eq!(completion.data, Some(0xFE));
        assert_eq!(completion.completed_at, deadline);

        let state = bus.export_state();
        assert_eq!(state.coordinator.scheduler.clock_domains.len(), 3);
        assert!(
            state
                .coordinator
                .scheduler
                .clock_domains
                .iter()
                .all(|domain| domain.total_cycles == 5)
        );
        assert_eq!(state.vdp.cycles_into_line(), 5);
        assert_eq!(state.psg.elapsed_clocks(), 5);
    }

    #[test]
    fn port_read_samples_controller_after_request_start_at_completion() {
        let mut bus = Sg1000TemporalBus::new(vec![0x00], MemoryConfig::default(), 2).unwrap();
        bus.set_joystick(1, 0xFE).unwrap();
        bus.begin_request(request(11, Z80BusOperation::PortRead, 0x00DC, 4), 0)
            .unwrap();

        // Input is sampled when the scheduled transaction completes, not when
        // the request is first queued.
        bus.set_joystick(1, 0xFD).unwrap();
        let Some(Sg1000TemporalDispatch::BusCompletion(completion)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected controller port-read completion");
        };
        assert_eq!(completion.data, Some(0xFD));
        bus.acknowledge_completion(11).unwrap();
    }

    #[test]
    fn vdp_status_port_read_observes_vblank_at_completion_deadline() {
        let mut bus = Sg1000TemporalBus::new(vec![0x00], MemoryConfig::default(), 1).unwrap();
        let before_vblank = 228_u32 * 192 - 2;
        bus.schedule_internal_cycle(99, before_vblank).unwrap();
        let Some(Sg1000TemporalDispatch::InternalCycleComplete(_)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected pre-vblank internal-cycle completion");
        };
        bus.acknowledge_internal_completion(99).unwrap();
        assert_eq!(bus.vdp().status() & 0x80, 0);

        bus.begin_request(request(100, Z80BusOperation::PortRead, 0x0081, 4), 0)
            .unwrap();
        let Some(Sg1000TemporalDispatch::BusCompletion(completion)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected VDP status port-read completion");
        };
        // The VDP crosses into VBlank during the I/O cycle. The status read
        // must observe and clear that event at the transaction deadline.
        assert_eq!(completion.data.map(|status| status & 0x80), Some(0x80));
        assert_eq!(bus.vdp().status() & 0x80, 0);
        bus.acknowledge_completion(100).unwrap();
    }

    #[test]
    fn psg_port_write_is_committed_once_and_survives_completion_restore() {
        let mut bus = Sg1000TemporalBus::new(vec![0x00], MemoryConfig::default(), 2).unwrap();
        let deadline = bus
            .begin_request(
                request(20, Z80BusOperation::PortWrite { value: 0x85 }, 0x0040, 3),
                0,
            )
            .unwrap();
        assert_eq!(bus.psg().tone_periods()[0], 0);

        let Some(Sg1000TemporalDispatch::BusCompletion(completion)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected PSG port-write completion");
        };
        assert_eq!(completion.completed_at, deadline);
        assert_eq!(completion.data, None);
        assert_eq!(bus.psg().tone_periods()[0], 5);

        let snapshot = bus.export_state();
        let mut restored = Sg1000TemporalBus::restore_state(snapshot.clone()).unwrap();
        assert_eq!(
            restored.dispatch_next().unwrap(),
            Some(Sg1000TemporalDispatch::BusCompletion(completion))
        );
        assert_eq!(restored.export_state(), snapshot);
        assert_eq!(
            restored.drain_psg_write_events(),
            vec![crate::PsgWriteEvent {
                tstate: 3,
                value: 0x85,
            }]
        );
        assert!(restored.drain_psg_write_events().is_empty());
    }

    #[test]
    fn vdp_port_writes_apply_at_their_scheduled_completion_boundaries() {
        let mut bus = Sg1000TemporalBus::new(vec![0x00], MemoryConfig::default(), 1).unwrap();
        for (id, port, value) in [(30, 0x81, 0x00), (31, 0x81, 0x40), (32, 0x80, 0xA5)] {
            let deadline = bus
                .begin_request(
                    request(id, Z80BusOperation::PortWrite { value }, port, 3),
                    0,
                )
                .unwrap();
            let Some(Sg1000TemporalDispatch::BusCompletion(completion)) =
                bus.dispatch_next().unwrap()
            else {
                panic!("expected VDP port-write completion");
            };
            assert_eq!(completion.completed_at, deadline);
            assert_eq!(completion.data, None);
            bus.acknowledge_completion(id).unwrap();
        }
        assert_eq!(bus.vdp().vram()[0], 0xA5);
    }

    #[test]
    fn restore_rejects_inconsistent_pending_event_and_request() {
        let mut bus = Sg1000TemporalBus::new(vec![0x00], MemoryConfig::default(), 2).unwrap();
        bus.begin_request(request(8, Z80BusOperation::MemoryRead, 0x0000, 3), 0)
            .unwrap();
        let mut state = bus.export_state();
        state.pending_request.as_mut().unwrap().id = 9;
        assert_eq!(
            Sg1000TemporalBus::restore_state(state).err(),
            Some(Sg1000TemporalBusError::InvalidState)
        );
    }

    #[test]
    fn restore_rejects_zero_base_tstates_in_a_pending_request() {
        let mut bus = Sg1000TemporalBus::new(vec![0x00], MemoryConfig::default(), 2).unwrap();
        bus.begin_request(request(12, Z80BusOperation::MemoryRead, 0x0000, 3), 0)
            .unwrap();
        let mut state = bus.export_state();
        state.pending_request.as_mut().unwrap().base_tstates = 0;
        assert_eq!(
            Sg1000TemporalBus::restore_state(state).err(),
            Some(Sg1000TemporalBusError::InvalidState)
        );
    }
}
