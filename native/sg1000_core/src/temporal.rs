//! SG-1000 machine-level temporal coordinator.
//!
//! The coordinator owns one deterministic machine timeline. It receives an
//! explicit master-tick/T-state scale and does not infer a board frequency or
//! execute device callbacks.

use core::fmt;

use sega_machine_scheduler::{
    ClockDomainId, MachineDispatch, MachineScheduler, SchedulerError, SchedulerState,
};
use sega_temporal_core::{
    ClockDomainSnapshot, ClockRatio, HardwarePhase, MachineTime, PlatformPriority,
};

pub const SG1000_TEMPORAL_COORDINATOR_STATE_VERSION: u16 = 3;
pub const DEFAULT_SG1000_CPU_CLOCK_HZ: u64 = 3_579_545;
pub const SG1000_Z80_CLOCK_DOMAIN_ID: ClockDomainId = ClockDomainId(0);
/// SG-1000 PSG input-clock domain; its ratio is supplied by the machine profile.
pub const SG1000_PSG_CLOCK_DOMAIN_ID: ClockDomainId = ClockDomainId(1);
/// SG-1000 TMS9918A clock domain, currently expressed in Z80 T-state units.
pub const SG1000_VDP_CLOCK_DOMAIN_ID: ClockDomainId = ClockDomainId(2);

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

/// Typed boundaries consumed by the SG-1000 machine adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sg1000TemporalEvent {
    BusCompletion { request_id: u64 },
    InternalCycleComplete { continuation_id: u64 },
}

/// Versioned scheduler state and explicit machine-time scale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sg1000TemporalCoordinatorState {
    pub format_version: u16,
    pub master_ticks_per_tstate: u64,
    pub cpu_clock_hz: u64,
    pub psg_clock_hz: u64,
    pub scheduler: SchedulerState<Sg1000TemporalEvent>,
}

/// Errors raised while coordinating the SG-1000 timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sg1000TemporalCoordinatorError {
    ZeroTStateScale,
    ZeroDeviceClock,
    ZeroCycleDuration,
    DurationOverflow,
    DeadlineOverflow,
    InvalidState,
    Scheduler(SchedulerError),
}

impl fmt::Display for Sg1000TemporalCoordinatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTStateScale => f.write_str("master ticks per Z80 T-state must be positive"),
            Self::ZeroDeviceClock => f.write_str("CPU and PSG clock frequencies must be positive"),
            Self::ZeroCycleDuration => f.write_str("scheduled cycle duration must be positive"),
            Self::DurationOverflow => f.write_str("scheduled cycle duration overflow"),
            Self::DeadlineOverflow => f.write_str("scheduled cycle deadline overflow"),
            Self::InvalidState => f.write_str("SG-1000 temporal coordinator state is inconsistent"),
            Self::Scheduler(error) => write!(f, "SG-1000 machine scheduler failed: {error}"),
        }
    }
}

impl std::error::Error for Sg1000TemporalCoordinatorError {}

impl From<SchedulerError> for Sg1000TemporalCoordinatorError {
    fn from(error: SchedulerError) -> Self {
        Self::Scheduler(error)
    }
}

/// One SG-1000 timeline. Device execution remains in the owning machine layer.
#[derive(Debug, Clone)]
pub struct Sg1000TemporalCoordinator {
    scheduler: MachineScheduler<Sg1000TemporalEvent>,
    master_ticks_per_tstate: u64,
    cpu_clock_hz: u64,
    psg_clock_hz: u64,
}

impl Sg1000TemporalCoordinator {
    /// Construct the provisional profile in which PSG input clocks match CPU T-states.
    pub fn new(master_ticks_per_tstate: u64) -> Result<Self, Sg1000TemporalCoordinatorError> {
        Self::new_with_clock_profile(
            master_ticks_per_tstate,
            DEFAULT_SG1000_CPU_CLOCK_HZ,
            DEFAULT_SG1000_CPU_CLOCK_HZ,
        )
    }

    /// Construct one timeline with explicit CPU and PSG oscillator frequencies.
    ///
    /// The Z80 and TMS9918A domains advance in CPU T-state units. The PSG domain
    /// uses the exact rational PSG/CPU frequency ratio; no rounded per-step
    /// conversion or accumulated floating-point phase is used.
    pub fn new_with_clock_profile(
        master_ticks_per_tstate: u64,
        cpu_clock_hz: u64,
        psg_clock_hz: u64,
    ) -> Result<Self, Sg1000TemporalCoordinatorError> {
        if master_ticks_per_tstate == 0 {
            return Err(Sg1000TemporalCoordinatorError::ZeroTStateScale);
        }
        if cpu_clock_hz == 0 || psg_clock_hz == 0 {
            return Err(Sg1000TemporalCoordinatorError::ZeroDeviceClock);
        }
        let psg_divisor = gcd(cpu_clock_hz, psg_clock_hz);
        let psg_numerator = psg_clock_hz / psg_divisor;
        let psg_denominator = (cpu_clock_hz / psg_divisor)
            .checked_mul(master_ticks_per_tstate)
            .ok_or(Sg1000TemporalCoordinatorError::DurationOverflow)?;
        let mut scheduler = MachineScheduler::new();
        let cpu_ratio = ClockRatio::new(1, master_ticks_per_tstate)
            .map_err(|_| Sg1000TemporalCoordinatorError::InvalidState)?;
        let psg_ratio = ClockRatio::new(psg_numerator, psg_denominator)
            .map_err(|_| Sg1000TemporalCoordinatorError::InvalidState)?;
        scheduler.register_clock_domain(SG1000_Z80_CLOCK_DOMAIN_ID, cpu_ratio)?;
        scheduler.register_clock_domain(SG1000_PSG_CLOCK_DOMAIN_ID, psg_ratio)?;
        scheduler.register_clock_domain(SG1000_VDP_CLOCK_DOMAIN_ID, cpu_ratio)?;
        Ok(Self {
            scheduler,
            master_ticks_per_tstate,
            cpu_clock_hz,
            psg_clock_hz,
        })
    }

    /// Current absolute machine timestamp.
    pub const fn now(&self) -> MachineTime {
        self.scheduler.now()
    }

    /// Explicit master ticks consumed by one nominal Z80 T-state.
    pub const fn master_ticks_per_tstate(&self) -> u64 {
        self.master_ticks_per_tstate
    }

    /// Read-only Z80 clock-domain snapshot.
    pub fn z80_clock_snapshot(
        &self,
    ) -> Result<ClockDomainSnapshot, Sg1000TemporalCoordinatorError> {
        self.scheduler
            .clock_snapshot(SG1000_Z80_CLOCK_DOMAIN_ID)
            .map_err(Into::into)
    }

    /// Read-only PSG input-clock domain snapshot.
    pub fn psg_clock_snapshot(
        &self,
    ) -> Result<ClockDomainSnapshot, Sg1000TemporalCoordinatorError> {
        self.scheduler
            .clock_snapshot(SG1000_PSG_CLOCK_DOMAIN_ID)
            .map_err(Into::into)
    }

    /// Read-only VDP clock-domain snapshot.
    pub fn vdp_clock_snapshot(
        &self,
    ) -> Result<ClockDomainSnapshot, Sg1000TemporalCoordinatorError> {
        self.scheduler
            .clock_snapshot(SG1000_VDP_CLOCK_DOMAIN_ID)
            .map_err(Into::into)
    }

    /// Schedule a bus completion after nominal T-states plus fixture WAIT.
    pub fn schedule_bus_completion(
        &mut self,
        request_id: u64,
        base_tstates: u8,
        wait_tstates: u32,
    ) -> Result<MachineTime, Sg1000TemporalCoordinatorError> {
        if base_tstates == 0 {
            return Err(Sg1000TemporalCoordinatorError::ZeroCycleDuration);
        }
        let total_tstates = u32::from(base_tstates)
            .checked_add(wait_tstates)
            .ok_or(Sg1000TemporalCoordinatorError::DurationOverflow)?;
        self.schedule_boundary(
            u128::from(total_tstates),
            Sg1000TemporalEvent::BusCompletion { request_id },
        )
    }

    /// Schedule a CPU-internal interval without inventing a memory transaction.
    pub fn schedule_internal_cycle(
        &mut self,
        continuation_id: u64,
        tstates: u32,
    ) -> Result<MachineTime, Sg1000TemporalCoordinatorError> {
        if tstates == 0 {
            return Err(Sg1000TemporalCoordinatorError::ZeroCycleDuration);
        }
        self.schedule_boundary(
            u128::from(tstates),
            Sg1000TemporalEvent::InternalCycleComplete { continuation_id },
        )
    }

    fn schedule_boundary(
        &mut self,
        tstates: u128,
        event: Sg1000TemporalEvent,
    ) -> Result<MachineTime, Sg1000TemporalCoordinatorError> {
        let master_ticks = tstates
            .checked_mul(u128::from(self.master_ticks_per_tstate))
            .ok_or(Sg1000TemporalCoordinatorError::DurationOverflow)?;
        let deadline = self
            .now()
            .checked_add(master_ticks)
            .ok_or(Sg1000TemporalCoordinatorError::DeadlineOverflow)?;
        self.scheduler.schedule_at_with_phase(
            deadline,
            HardwarePhase::new(0),
            PlatformPriority::new(0),
            event,
        )?;
        Ok(deadline)
    }

    /// Dispatch the next event after advancing the registered clock domain.
    pub fn dispatch_next(
        &mut self,
    ) -> Result<Option<MachineDispatch<Sg1000TemporalEvent>>, Sg1000TemporalCoordinatorError> {
        self.scheduler.dispatch_next().map_err(Into::into)
    }

    /// Advance idle time without skipping any queued event.
    pub fn advance_idle_to(
        &mut self,
        target: MachineTime,
    ) -> Result<(), Sg1000TemporalCoordinatorError> {
        self.scheduler.advance_idle_to(target)?;
        Ok(())
    }

    /// Export the complete scheduler timeline and profile scale.
    pub fn export_state(&self) -> Sg1000TemporalCoordinatorState {
        Sg1000TemporalCoordinatorState {
            format_version: SG1000_TEMPORAL_COORDINATOR_STATE_VERSION,
            master_ticks_per_tstate: self.master_ticks_per_tstate,
            cpu_clock_hz: self.cpu_clock_hz,
            psg_clock_hz: self.psg_clock_hz,
            scheduler: self.scheduler.export_state(),
        }
    }

    /// Restore only if the registered Z80 clock domain matches the saved scale.
    pub fn restore_state(
        state: Sg1000TemporalCoordinatorState,
    ) -> Result<Self, Sg1000TemporalCoordinatorError> {
        if state.format_version != SG1000_TEMPORAL_COORDINATOR_STATE_VERSION
            || state.master_ticks_per_tstate == 0
            || state.cpu_clock_hz == 0
            || state.psg_clock_hz == 0
        {
            return Err(Sg1000TemporalCoordinatorError::InvalidState);
        }
        let z80_domain = state
            .scheduler
            .clock_domains
            .iter()
            .find(|domain| domain.id == SG1000_Z80_CLOCK_DOMAIN_ID)
            .ok_or(Sg1000TemporalCoordinatorError::InvalidState)?;
        let psg_domain = state
            .scheduler
            .clock_domains
            .iter()
            .find(|domain| domain.id == SG1000_PSG_CLOCK_DOMAIN_ID)
            .ok_or(Sg1000TemporalCoordinatorError::InvalidState)?;
        let vdp_domain = state
            .scheduler
            .clock_domains
            .iter()
            .find(|domain| domain.id == SG1000_VDP_CLOCK_DOMAIN_ID)
            .ok_or(Sg1000TemporalCoordinatorError::InvalidState)?;
        let psg_divisor = gcd(state.cpu_clock_hz, state.psg_clock_hz);
        let expected_psg_numerator = state.psg_clock_hz / psg_divisor;
        let expected_psg_denominator = (state.cpu_clock_hz / psg_divisor)
            .checked_mul(state.master_ticks_per_tstate)
            .ok_or(Sg1000TemporalCoordinatorError::InvalidState)?;
        if state.scheduler.clock_domains.len() != 3
            || z80_domain.ratio_numerator != 1
            || z80_domain.ratio_denominator != state.master_ticks_per_tstate
            || psg_domain.ratio_numerator != expected_psg_numerator
            || psg_domain.ratio_denominator != expected_psg_denominator
            || vdp_domain.ratio_numerator != 1
            || vdp_domain.ratio_denominator != state.master_ticks_per_tstate
            || z80_domain.current_time != psg_domain.current_time
            || z80_domain.registration_start != psg_domain.registration_start
            || z80_domain.initial_phase != psg_domain.initial_phase
            || z80_domain.current_time != vdp_domain.current_time
            || z80_domain.registration_start != vdp_domain.registration_start
            || z80_domain.initial_phase != vdp_domain.initial_phase
        {
            return Err(Sg1000TemporalCoordinatorError::InvalidState);
        }
        let scheduler = MachineScheduler::restore_state(state.scheduler)?;
        Ok(Self {
            scheduler,
            master_ticks_per_tstate: state.master_ticks_per_tstate,
            cpu_clock_hz: state.cpu_clock_hz,
            psg_clock_hz: state.psg_clock_hz,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SG1000_PSG_CLOCK_DOMAIN_ID, SG1000_VDP_CLOCK_DOMAIN_ID, SG1000_Z80_CLOCK_DOMAIN_ID,
        Sg1000TemporalCoordinator, Sg1000TemporalCoordinatorError, Sg1000TemporalEvent,
    };
    use sega_temporal_core::MachineTime;

    #[test]
    fn scheduled_boundary_advances_cpu_vdp_and_psg_clock_domains_together() {
        let mut timeline = Sg1000TemporalCoordinator::new(2).unwrap();
        let deadline = timeline.schedule_bus_completion(7, 3, 2).unwrap();
        assert_eq!(deadline, MachineTime::from_ticks(10));
        assert_eq!(timeline.now(), MachineTime::ZERO);

        let dispatch = timeline.dispatch_next().unwrap().unwrap();
        assert_eq!(dispatch.event.timestamp, deadline);
        assert_eq!(
            dispatch.event.payload,
            Sg1000TemporalEvent::BusCompletion { request_id: 7 }
        );
        assert_eq!(timeline.now(), deadline);
        assert_eq!(dispatch.clock_advances.len(), 3);
        for domain in [
            SG1000_Z80_CLOCK_DOMAIN_ID,
            SG1000_PSG_CLOCK_DOMAIN_ID,
            SG1000_VDP_CLOCK_DOMAIN_ID,
        ] {
            let advance = dispatch
                .clock_advances
                .iter()
                .find(|advance| advance.domain == domain)
                .unwrap();
            assert_eq!(advance.elapsed_cycles, 5);
        }
        assert_eq!(timeline.z80_clock_snapshot().unwrap().total_cycles, 5);
        assert_eq!(timeline.psg_clock_snapshot().unwrap().total_cycles, 5);
        assert_eq!(timeline.vdp_clock_snapshot().unwrap().total_cycles, 5);
    }

    #[test]
    fn psg_clock_domain_uses_exact_configured_frequency_ratio() {
        let mut timeline =
            Sg1000TemporalCoordinator::new_with_clock_profile(2, 1_000_000, 2_000_000)
                .unwrap();
        timeline.schedule_internal_cycle(91, 5).unwrap();
        let dispatch = timeline.dispatch_next().unwrap().unwrap();
        assert_eq!(dispatch.clock_advances.len(), 3);
        assert_eq!(timeline.z80_clock_snapshot().unwrap().total_cycles, 5);
        assert_eq!(timeline.vdp_clock_snapshot().unwrap().total_cycles, 5);
        assert_eq!(timeline.psg_clock_snapshot().unwrap().total_cycles, 10);

        let snapshot = timeline.export_state();
        let restored = Sg1000TemporalCoordinator::restore_state(snapshot.clone()).unwrap();
        assert_eq!(restored.export_state(), snapshot);
    }

    #[test]
    fn scheduler_snapshot_restore_preserves_pending_deadline_and_clock_phase() {
        let mut timeline = Sg1000TemporalCoordinator::new(3).unwrap();
        let deadline = timeline.schedule_bus_completion(9, 4, 1).unwrap();
        let restored = Sg1000TemporalCoordinator::restore_state(timeline.export_state()).unwrap();
        assert_eq!(restored.now(), MachineTime::ZERO);
        assert_eq!(restored.export_state(), timeline.export_state());

        let mut restored = restored;
        let dispatch = restored.dispatch_next().unwrap().unwrap();
        assert_eq!(dispatch.event.timestamp, deadline);
        assert_eq!(dispatch.clock_advances[0].elapsed_cycles, 5);
    }

    #[test]
    fn coordinator_rejects_zero_scale_and_zero_duration() {
        assert_eq!(
            Sg1000TemporalCoordinator::new(0).err(),
            Some(Sg1000TemporalCoordinatorError::ZeroTStateScale)
        );
        let mut timeline = Sg1000TemporalCoordinator::new(2).unwrap();
        assert_eq!(
            timeline.schedule_bus_completion(1, 0, 0),
            Err(Sg1000TemporalCoordinatorError::ZeroCycleDuration)
        );
    }

    #[test]
    fn internal_cycle_is_a_typed_scheduler_event() {
        let mut timeline = Sg1000TemporalCoordinator::new(2).unwrap();
        let deadline = timeline.schedule_internal_cycle(3, 1).unwrap();
        let dispatch = timeline.dispatch_next().unwrap().unwrap();
        assert_eq!(dispatch.event.timestamp, deadline);
        assert_eq!(
            dispatch.event.payload,
            Sg1000TemporalEvent::InternalCycleComplete { continuation_id: 3 }
        );
    }
}
