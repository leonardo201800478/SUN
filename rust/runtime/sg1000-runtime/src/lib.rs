//! Python-independent Rust runtime and host boundary for the temporal SG-1000.

#![forbid(unsafe_code)]

mod worker;

pub use sega_audio_runtime::{AnalogOutputConfig, AudioOutputMetrics, AudioSession};
pub use sega_sg1000_core::{ControllerButtons, ControllerFrame, MemoryConfig, Sg1000MachineState};
pub use worker::{
    SG1000_WORKER_SAVE_STATE_VERSION, Sg1000DebugSnapshot, Sg1000TemporalWorker,
    Sg1000WorkerSaveState,
};
