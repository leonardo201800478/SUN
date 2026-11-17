//! Active scheduler-driven SG-1000 machine.
//!
//! The machine composes the shared resumable Z80, the SG-1000 transactional
//! memory/I/O bus, TMS9918A, SN76489 and one MachineScheduler timeline.
//! The old instruction-step implementation is compiled only as a test oracle;
//! it is not an alternate production runtime.

use core::fmt;

use crate::{
    DEFAULT_SG1000_CPU_CLOCK_HZ, MemoryConfig, PsgWriteEvent,
    Sg1000InternalCycleCompletion, Sg1000TemporalBus, Sg1000TemporalBusError,
    Sg1000TemporalBusState, Sg1000TemporalDispatch, Sg1000TemporalM1,
    Sg1000TemporalM1Error, Sg1000TemporalM1State,
};
use sega_temporal_core::MachineTime;
use sega_z80_cpu::{
    Z80BusOperation, Z80MicroSequencer, Z80MicroSequencerError,
    Z80MicroSequencerPhase, Z80MicroSequencerState, Z80M1Phase, Z80MicroStep,
};

pub const SG1000_TEMPORAL_MACHINE_STATE_VERSION: u16 = 1;
const DEFAULT_SG1000_MASTER_TICKS_PER_TSTATE: u64 = 1;
const DEFAULT_AUDIO_SAMPLE_RATE: u64 = 48_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sg1000TemporalMachineError {
    Bus(Sg1000TemporalBusError),
    Cpu(Z80MicroSequencerError),
    M1(Sg1000TemporalM1Error),
    InvalidState,
    NoScheduledEvent,
    FrameStepLimitReached,
    InternalCycleIdExhausted,
}

pub type MachineError = Sg1000TemporalMachineError;

impl fmt::Display for Sg1000TemporalMachineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bus(error) => write!(f, "SG-1000 temporal bus failed: {error}"),
            Self::Cpu(error) => write!(f, "SG-1000 shared Z80 sequencer failed: {error:?}"),
            Self::M1(error) => write!(f, "SG-1000 Z80 M1 continuation failed: {error:?}"),
            Self::InvalidState => f.write_str("SG-1000 temporal machine state is inconsistent"),
            Self::NoScheduledEvent => f.write_str("SG-1000 temporal machine has no event to dispatch"),
            Self::FrameStepLimitReached => f.write_str(
                "SG-1000 did not complete a frame within the configured instruction limit",
            ),
            Self::InternalCycleIdExhausted => {
                f.write_str("SG-1000 internal-cycle continuation ID exhausted")
            }
        }
    }
}

impl std::error::Error for Sg1000TemporalMachineError {}

impl From<Sg1000TemporalBusError> for Sg1000TemporalMachineError {
    fn from(error: Sg1000TemporalBusError) -> Self {
        Self::Bus(error)
    }
}

impl From<Z80MicroSequencerError> for Sg1000TemporalMachineError {
    fn from(error: Z80MicroSequencerError) -> Self {
        Self::Cpu(error)
    }
}

impl From<Sg1000TemporalM1Error> for Sg1000TemporalMachineError {
    fn from(error: Sg1000TemporalM1Error) -> Self {
        Self::M1(error)
    }
}

/// Full continuation required to resume an SG-1000 machine at a scheduler safe point.
#[derive(Debug, Clone, PartialEq)]
pub struct Sg1000MachineState {
    pub format_version: u16,
    pub cpu: Z80MicroSequencerState,
    /// External INT source, kept separate from the VDP's level-sensitive IRQ output.
    pub external_int_line: Option<u8>,
    pub instruction_started_at: Option<MachineTime>,
    pub bus: Sg1000TemporalBusState,
    pub next_internal_cycle_id: u64,
    pub pending_internal_cycle_id: Option<u64>,
    pub pending_internal_tstates: Option<u32>,
    pub m1: Option<Sg1000TemporalM1State>,
    pub m1_fixture_wait_tstates: Option<u32>,
}

fn elapsed_tstates(
    started_at: MachineTime,
    boundary: MachineTime,
    master_ticks_per_tstate: u64,
) -> Option<u32> {
    let scale = u128::from(master_ticks_per_tstate);
    if scale == 0 {
        return None;
    }
    let elapsed = boundary.checked_duration_since(started_at)?;
    if elapsed % scale != 0 {
        return None;
    }
    u32::try_from(elapsed / scale).ok()
}

fn instruction_timeline_is_valid(state: &Sg1000MachineState) -> bool {
    let cpu = &state.cpu;
    let bus = &state.bus;
    let scale = bus.coordinator.master_ticks_per_tstate;
    let now = bus.coordinator.scheduler.now;
    let active = !matches!(cpu.phase, Z80MicroSequencerPhase::Ready);

    if !active {
        return state.instruction_started_at.is_none()
            && state.m1.is_none()
            && state.m1_fixture_wait_tstates.is_none()
            && state.pending_internal_cycle_id.is_none()
            && state.pending_internal_tstates.is_none()
            && bus.pending_request.is_none()
            && bus.completion.is_none()
            && bus.pending_internal_cycle.is_none()
            && bus.internal_completion.is_none();
    }

    let Some(started_at) = state.instruction_started_at else {
        return false;
    };

    if state.pending_internal_cycle_id != bus.pending_internal_cycle
        || state.pending_internal_cycle_id.is_some() != state.pending_internal_tstates.is_some()
        || state.pending_internal_tstates == Some(0)
        || state.pending_internal_cycle_id.is_some_and(|id| id >= state.next_internal_cycle_id)
    {
        return false;
    }

    if let Some(m1_state) = state.m1 {
        if bus.pending_request.is_some()
            || bus.completion.is_some()
            || state.m1_fixture_wait_tstates.is_none()
            || cpu.handshake.wait_tstates != state.m1_fixture_wait_tstates.unwrap_or(0)
            || !matches!(
                cpu.handshake.pending_request,
                Some(request) if request.operation == Z80BusOperation::OpcodeFetch
            )
            || m1_state.cycle.phase == Z80M1Phase::Complete
        {
            return false;
        }
        let Some(m1) = Sg1000TemporalM1::restore_state(m1_state).ok() else {
            return false;
        };
        let pending_id = m1_state.pending_continuation_id;
        if pending_id != state.pending_internal_cycle_id
            || pending_id.is_some() != m1_state.pending_deadline.is_some()
            || (pending_id.is_some() && state.pending_internal_tstates != Some(1))
        {
            return false;
        }
        if let Some(id) = pending_id {
            let expected_deadline = if let Some(completion) = bus.internal_completion {
                if completion.continuation_id != id {
                    return false;
                }
                completion.completed_at
            } else {
                let Some(event) = bus.coordinator.scheduler.event_queue.pending.first() else {
                    return false;
                };
                if event.payload
                    != (crate::Sg1000TemporalEvent::InternalCycleComplete {
                        continuation_id: id,
                    })
                {
                    return false;
                }
                event.timestamp
            };
            if m1_state.pending_deadline != Some(expected_deadline) {
                return false;
            }
        }
        let current_m1_tstates = match m1.phase() {
            Z80M1Phase::T1 => 0_u32,
            Z80M1Phase::T2 => 1,
            Z80M1Phase::Wait => 2_u32.saturating_add(m1_state.cycle.wait_tstates),
            Z80M1Phase::T3 => 2_u32.saturating_add(m1_state.cycle.wait_tstates),
            Z80M1Phase::T4 => 3_u32.saturating_add(m1_state.cycle.wait_tstates),
            Z80M1Phase::Complete => return false,
        };
        let elapsed_m1 = current_m1_tstates
            .checked_add(u32::from(bus.internal_completion.is_some()));
        let expected = cpu.instruction_tstates.checked_add(elapsed_m1.unwrap_or(u32::MAX));
        return expected.is_some_and(|expected| {
            elapsed_tstates(started_at, now, scale) == Some(expected)
        });
    }

    if state.m1_fixture_wait_tstates.is_some() {
        return false;
    }

    if let (Some(request), Some(request_started_at)) =
        (bus.pending_request, bus.request_started_at)
    {
        if state.pending_internal_cycle_id.is_some()
            || bus.pending_internal_cycle.is_some()
            || bus.internal_completion.is_some()
            || cpu.handshake.pending_request != Some(request)
            || request_started_at > now
        {
            return false;
        }
        if let Some(completion) = bus.completion {
            let automatic_waits = if request.operation == Z80BusOperation::InterruptAcknowledge {
                u32::from(sega_z80_cpu::Z80_INT_ACK_AUTOMATIC_WAIT_TSTATES)
            } else {
                0
            };
            let Some(cycle_tstates) = u32::from(request.base_tstates)
                .checked_add(cpu.handshake.wait_tstates)
                .and_then(|value| value.checked_add(automatic_waits))
            else {
                return false;
            };
            return completion.request_id == request.id
                && completion.completed_at == now
                && elapsed_tstates(started_at, now, scale)
                    == cpu.instruction_tstates.checked_add(cycle_tstates);
        }
        return bus.completion.is_none()
            && elapsed_tstates(started_at, request_started_at, scale)
                == Some(cpu.instruction_tstates);
    }

    if bus.pending_request.is_some() || bus.request_started_at.is_some() || bus.completion.is_some()
    {
        return false;
    }

    // The CPU may have exposed its next request while the machine loop has not
    // yet scheduled it. This is a valid instruction-boundary safe point.
    if cpu.handshake.pending_request.is_some() {
        return state.pending_internal_cycle_id.is_none()
            && state.pending_internal_tstates.is_none()
            && bus.pending_internal_cycle.is_none()
            && bus.internal_completion.is_none()
            && elapsed_tstates(started_at, now, scale) == Some(cpu.instruction_tstates);
    }

    if let (Some(continuation_id), Some(cycle_started_at)) =
        (bus.pending_internal_cycle, bus.internal_cycle_started_at)
    {
        if !cpu_phase_is_internal(cpu.phase)
            || state.pending_internal_cycle_id != Some(continuation_id)
            || cycle_started_at > now
        {
            return false;
        }
        if let Some(completion) = bus.internal_completion {
            return completion.continuation_id == continuation_id
                && completion.completed_at == now
                && state.pending_internal_tstates.is_some_and(|tstates| {
                    elapsed_tstates(started_at, now, scale)
                        == cpu.instruction_tstates.checked_add(tstates)
                });
        }
        return bus.internal_completion.is_none()
            && elapsed_tstates(started_at, cycle_started_at, scale)
                == Some(cpu.instruction_tstates);
    }

    if state.pending_internal_cycle_id.is_some()
        || state.pending_internal_tstates.is_some()
        || bus.internal_completion.is_some()
    {
        return false;
    }

    if matches!(
        cpu.phase,
        Z80MicroSequencerPhase::InterruptAcknowledgeComplete { .. }
    ) || cpu_phase_is_internal(cpu.phase)
    {
        return elapsed_tstates(started_at, now, scale) == Some(cpu.instruction_tstates);
    }

    false
}

fn cpu_phase_is_internal(phase: Z80MicroSequencerPhase) -> bool {
    phase.is_internal_cycle()
}

#[derive(Debug, Clone)]
pub struct Sg1000Machine {
    cpu: Z80MicroSequencer,
    bus: Sg1000TemporalBus,
    external_int_line: Option<u8>,
    instruction_started_at: Option<MachineTime>,
    next_internal_cycle_id: u64,
    pending_internal_cycle_id: Option<u64>,
    pending_internal_tstates: Option<u32>,
    m1: Option<Sg1000TemporalM1>,
    m1_fixture_wait_tstates: Option<u32>,
}

impl Sg1000Machine {
    /// Construct the machine with the provisional nominal CPU/PSG profile.
    /// The profile is deterministic configuration, not board-level clock validation.
    pub fn new(rom: &[u8], memory_config: MemoryConfig) -> Result<Self, MachineError> {
        Self::new_with_clock_profile(
            rom.to_vec(),
            memory_config,
            DEFAULT_SG1000_MASTER_TICKS_PER_TSTATE,
            DEFAULT_SG1000_CPU_CLOCK_HZ,
            DEFAULT_SG1000_CPU_CLOCK_HZ,
            DEFAULT_AUDIO_SAMPLE_RATE,
        )
    }

    /// Construct the machine with an explicit PSG input clock and PCM source rate.
    pub fn new_with_audio(
        rom: &[u8],
        memory_config: MemoryConfig,
        psg_clock_hz: u64,
        sample_rate: u64,
    ) -> Result<Self, MachineError> {
        Self::new_with_clock_profile(
            rom.to_vec(),
            memory_config,
            DEFAULT_SG1000_MASTER_TICKS_PER_TSTATE,
            DEFAULT_SG1000_CPU_CLOCK_HZ,
            psg_clock_hz,
            sample_rate,
        )
    }

    /// Construct the machine with explicit timebase and CPU/PSG oscillator inputs.
    /// The oscillator values are profile data; no physical-board accuracy is implied.
    pub fn new_with_clock_profile(
        rom: impl Into<Vec<u8>>,
        memory_config: MemoryConfig,
        master_ticks_per_tstate: u64,
        cpu_clock_hz: u64,
        psg_clock_hz: u64,
        sample_rate: u64,
    ) -> Result<Self, MachineError> {
        let bus = Sg1000TemporalBus::new_with_audio(
            rom,
            memory_config,
            master_ticks_per_tstate,
            cpu_clock_hz,
            psg_clock_hz,
            sample_rate,
        )?;
        let mut machine = Self {
            cpu: Z80MicroSequencer::default(),
            bus,
            external_int_line: None,
            instruction_started_at: None,
            next_internal_cycle_id: 0,
            pending_internal_cycle_id: None,
            pending_internal_tstates: None,
            m1: None,
            m1_fixture_wait_tstates: None,
        };
        machine.sync_vdp_interrupt_line();
        Ok(machine)
    }

    /// Current CPU sequencer state. The machine remains the owner of its continuation.
    pub const fn cpu(&self) -> &Z80MicroSequencer {
        &self.cpu
    }

    /// Mutable CPU access for deterministic setup and diagnostics.
    /// Call only at a safe instruction boundary.
    pub fn cpu_mut(&mut self) -> &mut Z80MicroSequencer {
        &mut self.cpu
    }

    pub const fn now(&self) -> MachineTime {
        self.bus.now()
    }

    /// Total elapsed CPU T-states from the Scheduler-owned clock domain.
    pub fn total_tstates(&self) -> u64 {
        u64::try_from(self.bus.now().ticks() / u128::from(self.bus.master_ticks_per_tstate()))
            .unwrap_or(u64::MAX)
    }

    pub fn frame_number(&self) -> u64 {
        self.bus.frame_number()
    }

    pub fn framebuffer(&self) -> &[u8] {
        self.bus.framebuffer()
    }

    pub fn inspect_memory(&self, address: u16) -> u8 {
        self.bus.memory().read(address)
    }

    pub fn set_sprite_limit_enabled(&mut self, enabled: bool) {
        self.bus.set_sprite_limit_enabled(enabled);
    }

    pub fn sprite_limit_enabled(&self) -> bool {
        self.bus.sprite_limit_enabled()
    }

    pub fn set_joystick(&mut self, player: u8, value: u8) -> Result<(), crate::IoError> {
        self.bus.set_joystick(player, value)
    }

    pub fn drain_psg_write_events(&mut self) -> Vec<PsgWriteEvent> {
        self.bus.drain_psg_write_events()
    }

    pub fn drain_audio_samples(&mut self, max_samples: Option<usize>) -> Vec<f64> {
        self.bus.drain_audio_samples(max_samples)
    }

    pub fn set_interrupt_vector(&mut self, vector: u8) {
        self.external_int_line = Some(vector);
        self.bus.set_interrupt_vector(vector);
        self.sync_vdp_interrupt_line();
    }

    pub fn set_int_line(&mut self, vector: Option<u8>) {
        self.external_int_line = vector;
        self.bus.set_interrupt_vector(vector.unwrap_or(0xFF));
        self.sync_vdp_interrupt_line();
    }

    pub fn request_nmi(&mut self) {
        self.cpu.request_nmi();
    }

    fn sync_vdp_interrupt_line(&mut self) {
        let vector = self
            .external_int_line
            .or_else(|| self.bus.vdp().interrupt_pending().then_some(0xFF));
        self.cpu.set_int_line(vector);
    }

    pub fn begin_instruction(&mut self) -> Result<Z80MicroStep, MachineError> {
        self.sync_vdp_interrupt_line();
        let step = self.cpu.begin_instruction()?;
        self.instruction_started_at = Some(self.bus.now());
        Ok(step)
    }

    pub fn schedule_pending_bus(&mut self, fixture_wait_tstates: u32) -> Result<MachineTime, MachineError> {
        let request = self.cpu.pending_request().ok_or(MachineError::InvalidState)?;
        if request.operation == Z80BusOperation::OpcodeFetch {
            if self.bus.pending_request().is_some()
                || self.bus.completion().is_some()
                || self.pending_internal_cycle_id.is_some()
            {
                return Err(MachineError::InvalidState);
            }
            if let Some(m1) = self.m1.as_mut() {
                if self.m1_fixture_wait_tstates != Some(fixture_wait_tstates)
                    || m1.export_state().pending_continuation_id.is_some()
                {
                    return Err(MachineError::InvalidState);
                }
                let deadline = m1.schedule_next_tstate(&mut self.bus)?;
                let state = m1.export_state();
                self.pending_internal_cycle_id = state.pending_continuation_id;
                self.pending_internal_tstates = state.pending_continuation_id.map(|_| 1);
                return Ok(deadline);
            }

            let mut next_cpu = self.cpu.clone();
            next_cpu.insert_wait_tstates(fixture_wait_tstates)?;
            let mut m1 = Sg1000TemporalM1::new(request.address, next_cpu.i, next_cpu.refresh);
            let deadline = m1.schedule_next_tstate(&mut self.bus)?;
            let m1_state = m1.export_state();
            self.cpu = next_cpu;
            self.m1 = Some(m1);
            self.m1_fixture_wait_tstates = Some(fixture_wait_tstates);
            self.pending_internal_cycle_id = m1_state.pending_continuation_id;
            self.pending_internal_tstates = m1_state.pending_continuation_id.map(|_| 1);
            return Ok(deadline);
        }

        if self.bus.pending_request().is_some()
            || self.m1.is_some()
            || self.pending_internal_cycle_id.is_some()
        {
            return Err(MachineError::InvalidState);
        }
        let cpu_before = self.cpu.export_state();
        self.cpu.insert_wait_tstates(fixture_wait_tstates)?;
        match self.bus.begin_request(request, fixture_wait_tstates) {
            Ok(deadline) => Ok(deadline),
            Err(error) => {
                self.cpu = Z80MicroSequencer::restore_state(cpu_before)?;
                Err(error.into())
            }
        }
    }

    pub fn schedule_internal_cycle(&mut self, tstates: u32) -> Result<MachineTime, MachineError> {
        let cpu_state = self.cpu.export_state();
        if !cpu_state.phase.is_internal_cycle()
            || self.pending_internal_cycle_id.is_some()
            || self.bus.pending_request().is_some()
            || self.bus.completion().is_some()
            || self.m1.is_some()
            || tstates != 1
        {
            return Err(MachineError::InvalidState);
        }
        if cpu_state.instruction_tstates == u32::MAX {
            return Err(MachineError::Cpu(Z80MicroSequencerError::InstructionTStateOverflow));
        }
        if cpu_state.handshake.next_request_id == u64::MAX {
            return Err(MachineError::InvalidState);
        }
        let continuation_id = self.next_internal_cycle_id;
        let next_id = continuation_id.checked_add(1).ok_or(MachineError::InternalCycleIdExhausted)?;
        let deadline = self.bus.schedule_internal_cycle(continuation_id, tstates)?;
        self.next_internal_cycle_id = next_id;
        self.pending_internal_cycle_id = Some(continuation_id);
        self.pending_internal_tstates = Some(tstates);
        Ok(deadline)
    }

    fn finish_m1_fetch(&mut self) -> Result<Z80MicroStep, MachineError> {
        let m1 = self.m1.ok_or(MachineError::InvalidState)?;
        if m1.phase() != Z80M1Phase::Complete {
            return Err(MachineError::InvalidState);
        }
        let opcode = m1.opcode().ok_or(MachineError::InvalidState)?;
        let request = self
            .cpu
            .pending_request()
            .filter(|request| request.operation == Z80BusOperation::OpcodeFetch)
            .ok_or(MachineError::InvalidState)?;
        let mut next_cpu = self.cpu.clone();
        let step = next_cpu.complete_bus_cycle(request.id, Some(opcode))?;
        if next_cpu.halted {
            next_cpu.sample_halt_interrupts_at_t4()?;
        }
        self.cpu = next_cpu;
        self.m1 = None;
        self.m1_fixture_wait_tstates = None;
        if matches!(
            step,
            Z80MicroStep::InstructionRetired { .. } | Z80MicroStep::InterruptServiced { .. }
        ) {
            self.instruction_started_at = None;
        }
        Ok(step)
    }

    /// Dispatch one Scheduler event and commit exactly one matching CPU continuation.
    pub fn dispatch_next(&mut self) -> Result<Z80MicroStep, MachineError> {
        let dispatch = self.bus.dispatch_next()?.ok_or(MachineError::NoScheduledEvent)?;
        self.sync_vdp_interrupt_line();
        match dispatch {
            Sg1000TemporalDispatch::BusCompletion(completion) => {
                let mut next_cpu = self.cpu.clone();
                let step = next_cpu.complete_bus_cycle(completion.request_id, completion.data)?;
                self.bus.acknowledge_completion(completion.request_id)?;
                self.cpu = next_cpu;
                if matches!(
                    step,
                    Z80MicroStep::InstructionRetired { .. } | Z80MicroStep::InterruptServiced { .. }
                ) {
                    self.instruction_started_at = None;
                }
                Ok(step)
            }
            Sg1000TemporalDispatch::InternalCycleComplete(completion) => {
                if self.pending_internal_cycle_id != Some(completion.continuation_id) {
                    return Err(MachineError::InvalidState);
                }
                if let Some(mut m1) = self.m1 {
                    let phase_before = m1.phase();
                    let pins = m1.pins();
                    let total_waits = self.m1_fixture_wait_tstates.ok_or(MachineError::InvalidState)?;
                    let wait_asserted = match phase_before {
                        Z80M1Phase::T2 => total_waits > 0,
                        Z80M1Phase::Wait => {
                            m1.export_state().cycle.wait_tstates.saturating_add(1) < total_waits
                        }
                        _ => false,
                    };
                    if phase_before == Z80M1Phase::T3 {
                        let opcode = self.bus.memory().read(pins.address);
                        m1.sample_opcode(opcode)?;
                    }
                    let next_phase = m1.complete_tstate(completion, wait_asserted)?;
                    self.bus.acknowledge_internal_completion(completion.continuation_id)?;
                    self.pending_internal_cycle_id = None;
                    self.pending_internal_tstates = None;
                    self.m1 = Some(m1);
                    if next_phase == Z80M1Phase::Complete {
                        self.finish_m1_fetch()
                    } else {
                        let request = self.cpu.pending_request().ok_or(MachineError::InvalidState)?;
                        Ok(Z80MicroStep::BusRequest(request))
                    }
                } else {
                    let tstates = self.pending_internal_tstates.ok_or(MachineError::InvalidState)?;
                    let mut next_cpu = self.cpu.clone();
                    let step = next_cpu.complete_internal_cycle()?;
                    self.bus.acknowledge_internal_completion(completion.continuation_id)?;
                    self.cpu = next_cpu;
                    self.pending_internal_cycle_id = None;
                    self.pending_internal_tstates = None;
                    if matches!(
                        step,
                        Z80MicroStep::InstructionRetired { .. } | Z80MicroStep::InterruptServiced { .. }
                    ) {
                        self.instruction_started_at = None;
                    }
                    let _ = tstates;
                    Ok(step)
                }
            }
        }
    }

    pub fn step_instruction(&mut self, fixture_wait_tstates: u32) -> Result<Z80MicroStep, MachineError> {
        let mut step = self.begin_instruction()?;
        loop {
            step = match step {
                Z80MicroStep::BusRequest(_) => {
                    self.schedule_pending_bus(fixture_wait_tstates)?;
                    self.dispatch_next()?
                }
                Z80MicroStep::InternalCycle { tstates } => {
                    self.schedule_internal_cycle(tstates)?;
                    self.dispatch_next()?
                }
                retired @ Z80MicroStep::InstructionRetired { .. } => return Ok(retired),
                acknowledged @ Z80MicroStep::InterruptAcknowledge { .. } => return Ok(acknowledged),
                serviced @ Z80MicroStep::InterruptServiced { .. } => return Ok(serviced),
            };
        }
    }

    pub fn resume_instruction(&mut self, fixture_wait_tstates: u32) -> Result<Z80MicroStep, MachineError> {
        let mut step = if matches!(
            self.cpu.export_state().phase,
            Z80MicroSequencerPhase::InterruptAcknowledgeComplete { .. }
        ) {
            self.cpu.begin_instruction()?
        } else {
            return Err(MachineError::InvalidState);
        };
        loop {
            step = match step {
                Z80MicroStep::BusRequest(_) => {
                    if self.m1.as_ref().is_some_and(|m1| m1.phase() == Z80M1Phase::Complete) {
                        self.finish_m1_fetch()?
                    } else {
                        if self.m1.is_some() {
                            if self.m1.as_ref().is_some_and(|m1| m1.export_state().pending_continuation_id.is_none()) {
                                let waits = self.m1_fixture_wait_tstates.unwrap_or(fixture_wait_tstates);
                                self.schedule_pending_bus(waits)?;
                            }
                        } else if self.bus.pending_request().is_none() {
                            self.schedule_pending_bus(fixture_wait_tstates)?;
                        }
                        self.dispatch_next()?
                    }
                }
                Z80MicroStep::InternalCycle { tstates } => {
                    if self.pending_internal_cycle_id.is_none() {
                        self.schedule_internal_cycle(tstates)?;
                    }
                    self.dispatch_next()?
                }
                retired @ Z80MicroStep::InstructionRetired { .. } => return Ok(retired),
                acknowledged @ Z80MicroStep::InterruptAcknowledge { .. } => return Ok(acknowledged),
                serviced @ Z80MicroStep::InterruptServiced { .. } => return Ok(serviced),
            };
        }
    }

    /// Execute one complete instruction/interrupt service and return elapsed T-states.
    pub fn step(&mut self) -> Result<u32, MachineError> {
        let start = self.total_tstates();
        let step = self.step_instruction(0)?;
        if matches!(step, Z80MicroStep::InterruptAcknowledge { .. }) {
            let _ = self.resume_instruction(0)?;
        }
        let elapsed = self.total_tstates().saturating_sub(start);
        u32::try_from(elapsed).map_err(|_| MachineError::InvalidState)
    }

    pub fn run_instructions(&mut self, count: usize) -> Result<u64, MachineError> {
        let start = self.total_tstates();
        for _ in 0..count {
            self.step()?;
        }
        Ok(self.total_tstates().saturating_sub(start))
    }

    pub fn step_frame(&mut self, max_instructions: usize) -> Result<usize, MachineError> {
        self.step_frame_until(max_instructions, |_| false)
            .map(|(instructions, _)| instructions)
    }

    pub fn step_frame_until(
        &mut self,
        max_instructions: usize,
        mut should_break: impl FnMut(u16) -> bool,
    ) -> Result<(usize, bool), MachineError> {
        if max_instructions == 0 {
            return Err(MachineError::FrameStepLimitReached);
        }
        let initial_frame = self.frame_number();
        for completed in 0..max_instructions {
            if should_break(self.cpu.registers.pc) {
                return Ok((completed, true));
            }
            let step = self.step_instruction(0)?;
            if matches!(step, Z80MicroStep::InterruptAcknowledge { .. }) {
                let _ = self.resume_instruction(0)?;
            }
            if self.frame_number() != initial_frame {
                return Ok((completed + 1, false));
            }
        }
        Err(MachineError::FrameStepLimitReached)
    }

    pub fn reset_cpu(&mut self) -> Result<(), MachineError> {
        if self.cpu.export_state().phase != Z80MicroSequencerPhase::Ready
            || self.cpu.pending_request().is_some()
            || self.m1.is_some()
            || self.pending_internal_cycle_id.is_some()
            || self.bus.pending_request().is_some()
            || self.bus.completion().is_some()
            || self.bus.export_state().internal_completion.is_some()
        {
            return Err(MachineError::InvalidState);
        }
        self.cpu.reset();
        self.instruction_started_at = None;
        self.sync_vdp_interrupt_line();
        Ok(())
    }

    pub fn export_state(&self) -> Sg1000MachineState {
        Sg1000MachineState {
            format_version: SG1000_TEMPORAL_MACHINE_STATE_VERSION,
            cpu: self.cpu.export_state(),
            external_int_line: self.external_int_line,
            instruction_started_at: self.instruction_started_at,
            bus: self.bus.export_state(),
            next_internal_cycle_id: self.next_internal_cycle_id,
            pending_internal_cycle_id: self.pending_internal_cycle_id,
            pending_internal_tstates: self.pending_internal_tstates,
            m1: self.m1.map(|m1| m1.export_state()),
            m1_fixture_wait_tstates: self.m1_fixture_wait_tstates,
        }
    }

    pub fn restore_state(state: Sg1000MachineState) -> Result<Self, MachineError> {
        if state.format_version != SG1000_TEMPORAL_MACHINE_STATE_VERSION
            || !instruction_timeline_is_valid(&state)
        {
            return Err(MachineError::InvalidState);
        }
        let cpu = Z80MicroSequencer::restore_state(state.cpu.clone())?;
        let bus = Sg1000TemporalBus::restore_state(state.bus.clone())?;
        let m1 = state.m1.map(Sg1000TemporalM1::restore_state).transpose()?;
        let expected_bus_request = if m1.is_some() || state.bus.pending_request.is_none() {
            None
        } else {
            cpu.pending_request()
        };
        let expected_int_line = state
            .external_int_line
            .or_else(|| bus.vdp().interrupt_pending().then_some(0xFF));
        if cpu.export_state().int_pending != expected_int_line
            || state.bus.interrupt_vector != state.external_int_line.unwrap_or(0xFF)
            || expected_bus_request != state.bus.pending_request
            || state.pending_internal_cycle_id != state.bus.pending_internal_cycle
            || state.pending_internal_cycle_id.is_some() != state.pending_internal_tstates.is_some()
            || state.pending_internal_cycle_id.is_some_and(|id| id >= state.next_internal_cycle_id)
            || m1.is_some() != state.m1_fixture_wait_tstates.is_some()
        {
            return Err(MachineError::InvalidState);
        }
        if let Some(m1_state) = state.m1 {
            if m1_state.pending_continuation_id != state.pending_internal_cycle_id
                || cpu.pending_request().is_none_or(|request| request.operation != Z80BusOperation::OpcodeFetch)
            {
                return Err(MachineError::InvalidState);
            }
        } else if state.pending_internal_cycle_id.is_some() && !state.cpu.phase.is_internal_cycle() {
            return Err(MachineError::InvalidState);
        }
        Ok(Self {
            cpu,
            bus,
            external_int_line: state.external_int_line,
            instruction_started_at: state.instruction_started_at,
            next_internal_cycle_id: state.next_internal_cycle_id,
            pending_internal_cycle_id: state.pending_internal_cycle_id,
            pending_internal_tstates: state.pending_internal_tstates,
            m1,
            m1_fixture_wait_tstates: state.m1_fixture_wait_tstates,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Sg1000Machine, Sg1000TemporalMachineError};
    use crate::MemoryConfig;
    use sega_z80_cpu::{Z80MicroStep, Z80MicroSequencerPhase};

    #[test]
    fn nop_runs_on_the_scheduler_timeline_for_four_tstates() {
        let mut machine = Sg1000Machine::new(&[0x00], MemoryConfig::default()).unwrap();
        assert_eq!(machine.step(), Ok(4));
        assert_eq!(machine.cpu().registers.pc, 1);
        assert_eq!(machine.total_tstates(), 4);
    }

    #[test]
    fn immediate_load_and_ram_write_use_scheduler_bus_transactions() {
        let rom = [0x3E, 0x42, 0x21, 0x00, 0xC0, 0x36, 0x99, 0x76];
        let mut machine = Sg1000Machine::new(&rom, MemoryConfig::default()).unwrap();
        assert_eq!(machine.run_instructions(4), Ok(31));
        assert_eq!(machine.cpu().registers.a, 0x42);
        assert_eq!(machine.inspect_memory(0xC000), 0x99);
        assert!(machine.cpu().halted);
        assert_eq!(machine.frame_number(), 0);
    }

    #[test]
    fn immediate_in_reads_controller_at_the_scheduled_io_deadline() {
        let rom = [0x3E, 0x00, 0xDB, 0xDC, 0x76];
        let mut machine = Sg1000Machine::new(&rom, MemoryConfig::default()).unwrap();
        machine.set_joystick(1, 0xFE).unwrap();
        assert_eq!(machine.run_instructions(2), Ok(18));
        assert_eq!(machine.cpu().registers.a, 0xFE);
    }

    #[test]
    fn snapshot_restores_a_pending_m1_tstate_and_retires_once() {
        let mut machine = Sg1000Machine::new(&[0x00], MemoryConfig::default()).unwrap();
        let _ = machine.begin_instruction().unwrap();
        machine.schedule_pending_bus(0).unwrap();
        let snapshot = machine.export_state();
        let mut restored = Sg1000Machine::restore_state(snapshot.clone()).unwrap();
        assert_eq!(restored.export_state(), snapshot);
        let step = restored.dispatch_next().unwrap();
        assert!(matches!(step, Z80MicroStep::BusRequest(_)));
        while !matches!(restored.cpu().export_state().phase, Z80MicroSequencerPhase::Ready) {
            let pending = restored.cpu().pending_request();
            if pending.is_some() {
                restored.schedule_pending_bus(0).unwrap();
            } else if restored.cpu().export_state().phase.is_internal_cycle() {
                restored.schedule_internal_cycle(1).unwrap();
            }
            let _ = restored.dispatch_next().unwrap();
        }
        assert_eq!(restored.cpu().registers.pc, 1);
        assert_eq!(restored.total_tstates(), 4);
    }

    #[test]
    fn continuous_and_chunked_execution_produce_identical_machine_state() {
        let rom = [0x3E, 0x11, 0x3C, 0x32, 0x00, 0xC0, 0x18, 0xFA];
        let mut continuous = Sg1000Machine::new(&rom, MemoryConfig::default()).unwrap();
        let mut chunked = continuous.clone();
        continuous.run_instructions(80).unwrap();
        for _ in 0..8 {
            chunked.run_instructions(10).unwrap();
        }
        assert_eq!(continuous.export_state(), chunked.export_state());
    }

    #[test]
    fn restore_rejects_a_cpu_and_bus_request_mismatch() {
        let mut machine = Sg1000Machine::new(&[0x3E, 0x12], MemoryConfig::default()).unwrap();
        let _ = machine.begin_instruction().unwrap();
        machine.schedule_pending_bus(0).unwrap();
        let mut snapshot = machine.export_state();
        snapshot.bus.pending_internal_cycle = None;
        assert_eq!(
            Sg1000Machine::restore_state(snapshot).err(),
            Some(Sg1000TemporalMachineError::InvalidState)
        );
    }
}
