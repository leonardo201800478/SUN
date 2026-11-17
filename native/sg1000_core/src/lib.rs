//! Native Rust SG-1000 machine core.
//!
//! The active machine uses the shared Z80 micro-sequencer and MachineScheduler.
//! The former atomic CPU/ALU/bus/machine path is compiled only under cfg(test)
//! as a functional regression oracle; it is not part of the product runtime.
//!
//! Host controller APIs are normalized at the application boundary; the core
//! accepts deterministic active-low snapshots without polling SDL or Windows.

/// Size of the Z80's logical address space.
pub const SG1000_CPU_ADDRESS_SPACE_SIZE: usize = 0x1_0000;

#[cfg(test)]
mod alu;
#[cfg(test)]
mod bus;
#[cfg(test)]
mod cpu;
pub mod input;
pub mod io;
// Keep the old instruction-boundary implementation only as a regression oracle.
#[allow(dead_code)]
#[cfg(test)]
mod machine;
pub mod memory;
pub mod psg;
pub mod temporal;
pub mod temporal_bus;
pub mod temporal_m1;
pub mod temporal_machine;
pub mod vdp;

#[cfg(test)]
pub use bus::CpuBus;
#[cfg(test)]
pub use cpu::{CpuError, Z80};
pub use input::{
    ControllerBackend, ControllerBackendPreference, ControllerButtons, ControllerFrame,
};
pub use io::{IoError, PsgWriteEvent, Sg1000Io};
pub use memory::{MemoryConfig, MemoryError, Sg1000Memory};
pub use psg::{PsgError, Sn76489};
pub use temporal::{
    DEFAULT_SG1000_CPU_CLOCK_HZ, SG1000_PSG_CLOCK_DOMAIN_ID, SG1000_VDP_CLOCK_DOMAIN_ID,
    SG1000_Z80_CLOCK_DOMAIN_ID, Sg1000TemporalCoordinator, Sg1000TemporalCoordinatorError,
    Sg1000TemporalCoordinatorState, Sg1000TemporalEvent,
};
pub use temporal_bus::{
    Sg1000BusCompletion, Sg1000InternalCycleCompletion, Sg1000TemporalBus, Sg1000TemporalBusError,
    Sg1000TemporalBusState, Sg1000TemporalDispatch,
};
pub use temporal_m1::{Sg1000TemporalM1, Sg1000TemporalM1Error, Sg1000TemporalM1State};
pub use temporal_machine::{
    MachineError, Sg1000Machine, Sg1000MachineState, Sg1000TemporalMachineError,
};
pub use vdp::Tms9918A;
