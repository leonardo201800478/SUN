//! Dedicated machine-owning thread for the temporal SG-1000 runtime.
//!
//! The worker is the only owner of Sg1000Machine. Host input, video and audio
//! cross this boundary as coarse-grained snapshots or owned frame/sample batches.
//! The GUI and CPAL callback never advance MachineScheduler time.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use sega_audio_runtime::{
    AnalogOutputConfig, AudioOutputMetrics, AudioSession, DEFAULT_PCM_SOURCE_RATE_HZ,
};
use sega_sg1000_core::{
    ControllerButtons, ControllerFrame, MemoryConfig, Sg1000Machine, Sg1000MachineState,
};
use sega_video_runtime::{FrameMailbox, PixelFormat, VideoFrame};

pub const SG1000_WORKER_SAVE_STATE_VERSION: u16 = 1;

const COMMAND_QUEUE_CAPACITY: usize = 64;
const WORKER_REPLY_TIMEOUT: Duration = Duration::from_secs(2);
const FRAME_STEP_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_FRAME_INSTRUCTIONS: usize = 20_000;
const FRAME_WIDTH: u32 = 256;
const FRAME_HEIGHT: u32 = 192;
const FRAME_STRIDE: usize = FRAME_WIDTH as usize * 3;
const CPU_CLOCK_HZ: u64 = sega_sg1000_core::DEFAULT_SG1000_CPU_CLOCK_HZ;

/// In-memory save-state envelope bound to the exact ROM and memory profile.
#[derive(Debug, Clone, PartialEq)]
pub struct Sg1000WorkerSaveState {
    pub format_version: u16,
    pub rom: Vec<u8>,
    pub memory_config: MemoryConfig,
    pub controller_frame: ControllerFrame,
    pub machine: Sg1000MachineState,
}

/// Coherent debugger/telemetry snapshot captured on the machine-owning thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sg1000DebugSnapshot {
    pub frame_number: u64,
    pub total_tstates: u64,
    pub pc: u16,
    pub a: u8,
    pub f: u8,
    pub refresh: u8,
    pub halted: bool,
    pub last_frame_instructions: u64,
    pub audio_output: AudioOutputMetrics,
    pub runtime_error: Option<String>,
    pub audio_error: Option<String>,
    pub video_error: Option<String>,
}

#[derive(Default)]
struct WorkerTelemetry {
    running: AtomicBool,
    frame_number: AtomicU64,
    total_tstates: AtomicU64,
    last_frame_instructions: AtomicU64,
    runtime_error: Mutex<Option<String>>,
    audio_error: Mutex<Option<String>>,
    video_error: Mutex<Option<String>>,
}

impl WorkerTelemetry {
    fn update_machine(&self, machine: &Sg1000Machine, instructions: usize) {
        self.frame_number
            .store(machine.frame_number(), Ordering::Release);
        self.total_tstates
            .store(machine.total_tstates(), Ordering::Release);
        self.last_frame_instructions.store(
            u64::try_from(instructions).unwrap_or(u64::MAX),
            Ordering::Release,
        );
    }

    fn set_runtime_error(&self, error: Option<String>) {
        if let Ok(mut slot) = self.runtime_error.lock() {
            *slot = error;
        }
    }

    fn set_audio_error(&self, error: Option<String>) {
        if let Ok(mut slot) = self.audio_error.lock() {
            *slot = error;
        }
    }

    fn set_video_error(&self, error: Option<String>) {
        if let Ok(mut slot) = self.video_error.lock() {
            *slot = error;
        }
    }

    fn clone_error(slot: &Mutex<Option<String>>) -> Option<String> {
        slot.lock().ok().and_then(|value| value.clone())
    }
}

type AudioSessionFactory = Box<dyn FnOnce() -> Result<Box<dyn AudioSession>, String> + Send>;

enum WorkerCommand {
    Start(SyncSender<Result<(), String>>),
    Pause(SyncSender<Result<(), String>>),
    Stop(SyncSender<Result<(), String>>),
    Reset(SyncSender<Result<(), String>>),
    LoadRom(Vec<u8>, SyncSender<Result<(), String>>),
    StepFrame(usize, SyncSender<Result<usize, String>>),
    SetController(u8, ControllerButtons, SyncSender<Result<(), String>>),
    SetSpriteLimit(bool, SyncSender<Result<(), String>>),
    RequestNmi,
    DebugSnapshot(SyncSender<Result<Sg1000DebugSnapshot, String>>),
    SaveState(SyncSender<Result<Sg1000WorkerSaveState, String>>),
    LoadState(Box<Sg1000WorkerSaveState>, SyncSender<Result<(), String>>),
    ConfigureAudio(AudioSessionFactory, SyncSender<Result<(), String>>),
    SetAudioVolume(u8, SyncSender<Result<(), String>>),
    SetAudioDspConfig(AnalogOutputConfig, SyncSender<Result<(), String>>),
    CloseAudio(SyncSender<Result<(), String>>),
    SetVideoMailbox(Option<FrameMailbox>, SyncSender<Result<(), String>>),
    Shutdown,
}

struct WorkerState {
    machine: Sg1000Machine,
    rom: Vec<u8>,
    memory_config: MemoryConfig,
    controller_frame: ControllerFrame,
    running: bool,
    audio_session: Option<Box<dyn AudioSession>>,
    video_mailbox: Option<FrameMailbox>,
    video_sequence: u64,
    next_frame_deadline: Option<Instant>,
}

impl WorkerState {
    fn new(rom: Vec<u8>, memory_config: MemoryConfig) -> Result<Self, String> {
        let machine = build_machine(&rom, memory_config)?;
        Ok(Self {
            machine,
            rom,
            memory_config,
            controller_frame: ControllerFrame::default(),
            running: false,
            audio_session: None,
            video_mailbox: None,
            video_sequence: 0,
            next_frame_deadline: None,
        })
    }

    fn debug_snapshot(&self, telemetry: &WorkerTelemetry) -> Sg1000DebugSnapshot {
        let cpu = self.machine.cpu();
        Sg1000DebugSnapshot {
            frame_number: self.machine.frame_number(),
            total_tstates: self.machine.total_tstates(),
            pc: cpu.registers.pc,
            a: cpu.registers.a,
            f: cpu.registers.f,
            refresh: cpu.refresh,
            halted: cpu.halted,
            last_frame_instructions: telemetry.last_frame_instructions.load(Ordering::Acquire),
            audio_output: self
                .audio_session
                .as_ref()
                .map_or_else(AudioOutputMetrics::default, |session| session.metrics()),
            runtime_error: WorkerTelemetry::clone_error(&telemetry.runtime_error),
            audio_error: WorkerTelemetry::clone_error(&telemetry.audio_error),
            video_error: WorkerTelemetry::clone_error(&telemetry.video_error),
        }
    }

    fn start(&mut self, telemetry: &WorkerTelemetry) -> Result<(), String> {
        if self.running {
            return Ok(());
        }
        if let Some(session) = self.audio_session.as_mut() {
            session.resume()?;
        }
        self.running = true;
        self.next_frame_deadline = None;
        telemetry.running.store(true, Ordering::Release);
        telemetry.set_runtime_error(None);
        Ok(())
    }

    fn pause(&mut self, telemetry: &WorkerTelemetry) -> Result<(), String> {
        self.running = false;
        self.next_frame_deadline = None;
        telemetry.running.store(false, Ordering::Release);
        if let Some(session) = self.audio_session.as_mut() {
            session.suspend_and_flush()?;
        }
        Ok(())
    }

    fn rebuild_machine(
        &mut self,
        next_rom: Vec<u8>,
        preserve_running: bool,
        telemetry: &WorkerTelemetry,
    ) -> Result<(), String> {
        // Construct first: an invalid replacement ROM must not destroy the
        // currently running machine or its emulated state.
        let next_machine = build_machine(&next_rom, self.memory_config)?;
        let was_running = preserve_running && self.running;
        self.running = false;
        telemetry.running.store(false, Ordering::Release);
        self.next_frame_deadline = None;

        if let Some(session) = self.audio_session.as_mut() {
            session.suspend_and_flush()?;
            session.reset_dsp();
        }
        self.machine = next_machine;
        self.rom = next_rom;
        self.controller_frame = ControllerFrame::default();
        self.controller_frame
            .apply_to(&mut self.machine)
            .map_err(|error| error.to_string())?;
        telemetry.update_machine(&self.machine, 0);
        telemetry.set_runtime_error(None);

        if was_running {
            if let Some(session) = self.audio_session.as_mut() {
                session.resume()?;
            }
            self.running = true;
            telemetry.running.store(true, Ordering::Release);
        }
        Ok(())
    }

    fn execute_frame(
        &mut self,
        max_instructions: usize,
        publish_audio: bool,
        telemetry: &WorkerTelemetry,
    ) -> Result<usize, String> {
        if max_instructions == 0 {
            return Err("max_instructions must be positive".to_owned());
        }

        let instructions = self
            .machine
            .step_frame(max_instructions)
            .map_err(|error| error.to_string())?;

        if let Some(mailbox) = self.video_mailbox.as_ref() {
            let next_sequence = self
                .video_sequence
                .checked_add(1)
                .ok_or_else(|| "SG-1000 video frame sequence overflow".to_owned())?;
            let pixels = self.machine.framebuffer().to_vec();
            let frame = VideoFrame::new(
                next_sequence,
                FRAME_WIDTH,
                FRAME_HEIGHT,
                PixelFormat::Rgb888,
                FRAME_STRIDE,
                pixels,
            )
            .map_err(|error| error.to_string());
            let publish_result =
                frame.and_then(|frame| mailbox.publish(frame).map_err(|error| error.to_string()));
            match publish_result {
                Ok(()) => {
                    self.video_sequence = next_sequence;
                    telemetry.set_video_error(None);
                }
                Err(error) => {
                    telemetry.set_video_error(Some(error));
                    self.video_mailbox = None;
                }
            }
        }

        let samples = self.machine.drain_audio_samples(None);
        if let Some(session) = self.audio_session.as_mut() {
            let result = if publish_audio {
                session.process_samples(&samples)
            } else {
                session.process_samples_discarding_output(&samples)
            };
            if let Err(error) = result {
                session.close();
                self.audio_session = None;
                telemetry.set_audio_error(Some(error));
            } else {
                telemetry.set_audio_error(None);
            }
        }

        telemetry.update_machine(&self.machine, instructions);
        Ok(instructions)
    }

    fn save_state(&self) -> Sg1000WorkerSaveState {
        Sg1000WorkerSaveState {
            format_version: SG1000_WORKER_SAVE_STATE_VERSION,
            rom: self.rom.clone(),
            memory_config: self.memory_config,
            controller_frame: self.controller_frame,
            machine: self.machine.export_state(),
        }
    }

    fn load_state(
        &mut self,
        state: Sg1000WorkerSaveState,
        telemetry: &WorkerTelemetry,
    ) -> Result<(), String> {
        if self.running {
            return Err("SG-1000 save states can only be restored while paused".to_owned());
        }
        if state.format_version != SG1000_WORKER_SAVE_STATE_VERSION
            || state.rom != self.rom
            || state.memory_config != self.memory_config
        {
            return Err("SG-1000 save state does not match this ROM or memory profile".to_owned());
        }

        let mut restored =
            Sg1000Machine::restore_state(state.machine).map_err(|error| error.to_string())?;
        state
            .controller_frame
            .apply_to(&mut restored)
            .map_err(|error| error.to_string())?;

        if let Some(session) = self.audio_session.as_mut() {
            session.suspend_and_flush()?;
            session.reset_dsp();
        }
        self.machine = restored;
        self.controller_frame = state.controller_frame;
        self.next_frame_deadline = None;
        telemetry.update_machine(&self.machine, 0);
        telemetry.set_runtime_error(None);
        Ok(())
    }
}

fn build_machine(rom: &[u8], memory_config: MemoryConfig) -> Result<Sg1000Machine, String> {
    Sg1000Machine::new_with_audio(
        rom,
        memory_config,
        CPU_CLOCK_HZ,
        u64::from(DEFAULT_PCM_SOURCE_RATE_HZ),
    )
    .map_err(|error| error.to_string())
}

fn frame_duration_from_tstates(tstates: u64) -> Duration {
    Duration::from_secs_f64(tstates as f64 / CPU_CLOCK_HZ as f64)
}

fn enqueue_worker_command(
    commands: &SyncSender<WorkerCommand>,
    command: WorkerCommand,
) -> Result<(), String> {
    match commands.try_send(command) {
        Ok(()) => Ok(()),
        Err(TrySendError::Full(_)) => {
            Err("SG-1000 worker command queue is full; retry shortly".to_owned())
        }
        Err(TrySendError::Disconnected(_)) => {
            Err("SG-1000 machine worker is unavailable".to_owned())
        }
    }
}

fn receive_reply<T>(
    receiver: Receiver<T>,
    operation: &str,
    timeout: Duration,
) -> Result<T, String> {
    receiver.recv_timeout(timeout).map_err(|error| match error {
        RecvTimeoutError::Timeout => {
            format!("SG-1000 worker timed out while {operation}")
        }
        RecvTimeoutError::Disconnected => {
            format!("SG-1000 worker stopped while {operation}")
        }
    })
}

fn handle_command(
    command: WorkerCommand,
    state: &mut WorkerState,
    telemetry: &WorkerTelemetry,
) -> bool {
    match command {
        WorkerCommand::Start(reply) => {
            let _ = reply.send(state.start(telemetry));
        }
        WorkerCommand::Pause(reply) => {
            let _ = reply.send(state.pause(telemetry));
        }
        WorkerCommand::Stop(reply) => {
            let result = state.rebuild_machine(state.rom.clone(), false, telemetry);
            let _ = reply.send(result);
        }
        WorkerCommand::Reset(reply) => {
            let result = state.rebuild_machine(state.rom.clone(), true, telemetry);
            let _ = reply.send(result);
        }
        WorkerCommand::LoadRom(rom, reply) => {
            let result = state.rebuild_machine(rom, true, telemetry);
            let _ = reply.send(result);
        }
        WorkerCommand::StepFrame(max_instructions, reply) => {
            if state.running {
                let _ = reply.send(Err(
                    "step_frame is available only while the SG-1000 worker is paused".to_owned(),
                ));
            } else {
                let result = state.execute_frame(max_instructions, false, telemetry);
                if let Err(error) = result.as_ref() {
                    telemetry.set_runtime_error(Some(error.clone()));
                }
                let _ = reply.send(result);
            }
        }
        WorkerCommand::SetController(player, buttons, reply) => {
            let mut next = state.controller_frame;
            match player {
                1 => next.player_1 = buttons,
                2 => next.player_2 = buttons,
                _ => {
                    let _ = reply.send(Err(format!(
                        "SG-1000 controller player must be 1 or 2, got {player}"
                    )));
                    return false;
                }
            }
            let result = next
                .apply_to(&mut state.machine)
                .map_err(|error| error.to_string());
            if result.is_ok() {
                state.controller_frame = next;
            }
            let _ = reply.send(result);
        }
        WorkerCommand::SetSpriteLimit(enabled, reply) => {
            state.machine.set_sprite_limit_enabled(enabled);
            let _ = reply.send(Ok(()));
        }
        WorkerCommand::RequestNmi => state.machine.request_nmi(),
        WorkerCommand::DebugSnapshot(reply) => {
            let _ = reply.send(Ok(state.debug_snapshot(telemetry)));
        }
        WorkerCommand::SaveState(reply) => {
            let _ = reply.send(Ok(state.save_state()));
        }
        WorkerCommand::LoadState(snapshot, reply) => {
            let result = state.load_state(*snapshot, telemetry);
            let _ = reply.send(result);
        }
        WorkerCommand::ConfigureAudio(factory, reply) => match factory() {
            Ok(mut session) => {
                let previous_result = state
                    .audio_session
                    .as_mut()
                    .map_or(Ok(()), |previous| previous.suspend_and_flush());
                if let Err(error) = previous_result {
                    if let Some(mut previous) = state.audio_session.take() {
                        previous.close();
                    }
                    session.close();
                    telemetry.set_audio_error(Some(error.clone()));
                    let _ = reply.send(Err(error));
                } else {
                    if let Some(mut previous) = state.audio_session.take() {
                        previous.close();
                    }
                    let result = if state.running {
                        session.resume()
                    } else {
                        session.suspend_and_flush()
                    };
                    match result {
                        Ok(()) => {
                            state.audio_session = Some(session);
                            telemetry.set_audio_error(None);
                            let _ = reply.send(Ok(()));
                        }
                        Err(error) => {
                            session.close();
                            telemetry.set_audio_error(Some(error.clone()));
                            let _ = reply.send(Err(error));
                        }
                    }
                }
            }
            Err(error) => {
                telemetry.set_audio_error(Some(error.clone()));
                let _ = reply.send(Err(error));
            }
        },
        WorkerCommand::SetAudioVolume(volume, reply) => {
            let result = state.audio_session.as_mut().map_or_else(
                || Err("SG-1000 audio output is not configured".to_owned()),
                |session| session.set_volume(volume),
            );
            let _ = reply.send(result);
        }
        WorkerCommand::SetAudioDspConfig(config, reply) => {
            let result = state.audio_session.as_mut().map_or_else(
                || Err("SG-1000 audio output is not configured".to_owned()),
                |session| session.set_analog_output_config(config),
            );
            let _ = reply.send(result);
        }
        WorkerCommand::CloseAudio(reply) => {
            let result = if let Some(mut session) = state.audio_session.take() {
                let result = session.suspend_and_flush();
                session.close();
                result
            } else {
                Ok(())
            };
            telemetry.set_audio_error(result.as_ref().err().cloned());
            let _ = reply.send(result);
        }
        WorkerCommand::SetVideoMailbox(mailbox, reply) => {
            let result = mailbox.as_ref().map_or(Ok(()), |attached| {
                attached
                    .last_sequence()
                    .map(|sequence| {
                        if let Some(sequence) = sequence {
                            state.video_sequence = state.video_sequence.max(sequence);
                        }
                    })
                    .map_err(|error| error.to_string())
            });
            if result.is_ok() {
                state.video_mailbox = mailbox;
                telemetry.set_video_error(None);
            }
            let _ = reply.send(result);
        }
        WorkerCommand::Shutdown => {
            state.running = false;
            state.next_frame_deadline = None;
            telemetry.running.store(false, Ordering::Release);
            if let Some(mut session) = state.audio_session.take() {
                let _ = session.suspend_and_flush();
                session.close();
            }
            // The FrameMailbox is externally owned and is never closed here.
            return true;
        }
    }
    false
}

fn worker_loop(
    mut state: WorkerState,
    commands: Receiver<WorkerCommand>,
    telemetry: Arc<WorkerTelemetry>,
) {
    loop {
        let command = if state.running {
            let timeout = state
                .next_frame_deadline
                .map(|deadline| deadline.saturating_duration_since(Instant::now()))
                .unwrap_or(Duration::ZERO);
            match commands.recv_timeout(timeout) {
                Ok(command) => Some(command),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match commands.recv() {
                Ok(command) => Some(command),
                Err(_) => break,
            }
        };

        if let Some(command) = command {
            if handle_command(command, &mut state, &telemetry) {
                break;
            }
            continue;
        }

        if !state.running {
            continue;
        }

        let frame_started = Instant::now();
        let frame_start_tstates = state.machine.total_tstates();
        match state.execute_frame(MAX_FRAME_INSTRUCTIONS, true, &telemetry) {
            Ok(instructions) => {
                let elapsed_tstates = state
                    .machine
                    .total_tstates()
                    .saturating_sub(frame_start_tstates);
                let frame_duration = frame_duration_from_tstates(elapsed_tstates);
                let previous_deadline = state.next_frame_deadline.unwrap_or(frame_started);
                state.next_frame_deadline = Some(previous_deadline + frame_duration);
                telemetry.last_frame_instructions.store(
                    u64::try_from(instructions).unwrap_or(u64::MAX),
                    Ordering::Release,
                );
            }
            Err(error) => {
                telemetry.set_runtime_error(Some(error));
                state.running = false;
                state.next_frame_deadline = None;
                telemetry.running.store(false, Ordering::Release);
                if let Some(session) = state.audio_session.as_mut() {
                    let _ = session.suspend_and_flush();
                }
            }
        }
    }

    if let Some(mut session) = state.audio_session.take() {
        let _ = session.suspend_and_flush();
        session.close();
    }
    telemetry.running.store(false, Ordering::Release);
}

/// Handle to the dedicated thread that exclusively owns the SG-1000 machine.
pub struct Sg1000TemporalWorker {
    commands: SyncSender<WorkerCommand>,
    worker: Option<JoinHandle<()>>,
    telemetry: Arc<WorkerTelemetry>,
}

impl Sg1000TemporalWorker {
    /// Start a paused worker using the baseline SG-1000 memory profile.
    pub fn new(rom: Vec<u8>) -> Result<Self, String> {
        Self::new_with_memory_config(rom, MemoryConfig::default())
    }

    /// Start a paused worker with an explicit cartridge/internal RAM profile.
    pub fn new_with_memory_config(
        rom: Vec<u8>,
        memory_config: MemoryConfig,
    ) -> Result<Self, String> {
        let (commands, receiver) = mpsc::sync_channel(COMMAND_QUEUE_CAPACITY);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let telemetry = Arc::new(WorkerTelemetry::default());
        let worker_telemetry = Arc::clone(&telemetry);
        let worker = thread::Builder::new()
            .name("sg1000-temporal-worker".to_owned())
            .spawn(move || {
                // Construct the machine and its non-Send host audio slot on the
                // owning thread. Only ROM/configuration and channel handles
                // cross the thread boundary.
                match WorkerState::new(rom, memory_config) {
                    Ok(state) => {
                        worker_telemetry.update_machine(&state.machine, 0);
                        let _ = ready_sender.send(Ok(()));
                        worker_loop(state, receiver, worker_telemetry);
                    }
                    Err(error) => {
                        let _ = ready_sender.send(Err(error));
                    }
                }
            })
            .map_err(|error| format!("could not start SG-1000 worker thread: {error}"))?;

        match receive_reply(
            ready_receiver,
            "initializing the SG-1000 machine",
            WORKER_REPLY_TIMEOUT,
        )? {
            Ok(()) => Ok(Self {
                commands,
                worker: Some(worker),
                telemetry,
            }),
            Err(error) => {
                let _ = worker.join();
                Err(error)
            }
        }
    }

    fn request_unit(
        &self,
        command: impl FnOnce(SyncSender<Result<(), String>>) -> WorkerCommand,
        operation: &str,
    ) -> Result<(), String> {
        let (reply_sender, reply_receiver) = mpsc::sync_channel(1);
        enqueue_worker_command(&self.commands, command(reply_sender))?;
        receive_reply(reply_receiver, operation, WORKER_REPLY_TIMEOUT)?
    }

    /// Whether continuous frame execution is active.
    pub fn is_running(&self) -> bool {
        self.telemetry.running.load(Ordering::Acquire)
    }

    /// Latest frame counter published by the machine-owning thread.
    pub fn frame_number(&self) -> u64 {
        self.telemetry.frame_number.load(Ordering::Acquire)
    }

    /// Latest scheduler-derived CPU T-state count.
    pub fn machine_time_tstates(&self) -> u64 {
        self.telemetry.total_tstates.load(Ordering::Acquire)
    }

    pub fn start(&self) -> Result<(), String> {
        self.request_unit(WorkerCommand::Start, "starting the machine")
    }

    pub fn pause(&self) -> Result<(), String> {
        self.request_unit(WorkerCommand::Pause, "pausing the machine")
    }

    /// POWER OFF: pause and rebuild the machine while retaining the loaded ROM.
    pub fn stop(&self) -> Result<(), String> {
        self.request_unit(WorkerCommand::Stop, "stopping the machine")
    }

    /// Rebuild power-on hardware state and resume if the worker was running.
    pub fn reset(&self) -> Result<(), String> {
        self.request_unit(WorkerCommand::Reset, "resetting the machine")
    }

    /// Replace the cartridge ROM at a safe worker boundary.
    pub fn load_rom(&self, rom: Vec<u8>) -> Result<(), String> {
        self.request_unit(|reply| WorkerCommand::LoadRom(rom, reply), "loading a ROM")
    }

    /// Execute one complete VDP frame while paused.
    pub fn step_frame(&self, max_instructions: usize) -> Result<usize, String> {
        let (reply_sender, reply_receiver) = mpsc::sync_channel(1);
        enqueue_worker_command(
            &self.commands,
            WorkerCommand::StepFrame(max_instructions, reply_sender),
        )?;
        receive_reply(
            reply_receiver,
            "completing a frame step",
            FRAME_STEP_TIMEOUT,
        )?
    }

    /// Apply one normalized controller snapshot without host polling in the worker.
    pub fn set_controller(&self, player: u8, buttons: ControllerButtons) -> Result<(), String> {
        self.request_unit(
            |reply| WorkerCommand::SetController(player, buttons, reply),
            "updating controller input",
        )
    }

    pub fn set_sprite_limit_enabled(&self, enabled: bool) -> Result<(), String> {
        self.request_unit(
            |reply| WorkerCommand::SetSpriteLimit(enabled, reply),
            "updating the VDP sprite limit",
        )
    }

    /// Latch a non-maskable interrupt at the next CPU instruction boundary.
    pub fn request_nmi(&self) -> Result<(), String> {
        enqueue_worker_command(&self.commands, WorkerCommand::RequestNmi)
    }

    pub fn debug_snapshot(&self) -> Result<Sg1000DebugSnapshot, String> {
        let (reply_sender, reply_receiver) = mpsc::sync_channel(1);
        enqueue_worker_command(&self.commands, WorkerCommand::DebugSnapshot(reply_sender))?;
        receive_reply(
            reply_receiver,
            "capturing a machine snapshot",
            WORKER_REPLY_TIMEOUT,
        )?
    }

    /// Capture a complete in-memory state at a worker-safe command boundary.
    pub fn save_state(&self) -> Result<Sg1000WorkerSaveState, String> {
        let (reply_sender, reply_receiver) = mpsc::sync_channel(1);
        enqueue_worker_command(&self.commands, WorkerCommand::SaveState(reply_sender))?;
        receive_reply(
            reply_receiver,
            "capturing a save state",
            WORKER_REPLY_TIMEOUT,
        )?
    }

    /// Restore a state only when its ROM and memory profile match this worker.
    pub fn load_state(&self, state: Sg1000WorkerSaveState) -> Result<(), String> {
        self.request_unit(
            |reply| WorkerCommand::LoadState(Box::new(state), reply),
            "restoring a save state",
        )
    }

    /// Configure a host audio session on the machine-owning thread.
    ///
    /// The factory is transferred as a command and invoked by the worker;
    /// CPAL streams and DSP sessions never enter the GUI thread.
    pub fn configure_audio_output<F>(&self, factory: F) -> Result<(), String>
    where
        F: FnOnce() -> Result<Box<dyn AudioSession>, String> + Send + 'static,
    {
        self.request_unit(
            |reply| WorkerCommand::ConfigureAudio(Box::new(factory), reply),
            "configuring audio output",
        )
    }

    pub fn set_audio_volume(&self, volume: u8) -> Result<(), String> {
        self.request_unit(
            |reply| WorkerCommand::SetAudioVolume(volume, reply),
            "updating audio volume",
        )
    }

    pub fn set_audio_dsp_config(&self, config: AnalogOutputConfig) -> Result<(), String> {
        self.request_unit(
            |reply| WorkerCommand::SetAudioDspConfig(config, reply),
            "updating audio DSP configuration",
        )
    }

    pub fn close_audio_output(&self) -> Result<(), String> {
        self.request_unit(WorkerCommand::CloseAudio, "closing audio output")
    }

    fn set_video_mailbox(&self, mailbox: Option<FrameMailbox>) -> Result<(), String> {
        let (reply_sender, reply_receiver) = mpsc::sync_channel(1);
        enqueue_worker_command(
            &self.commands,
            WorkerCommand::SetVideoMailbox(mailbox, reply_sender),
        )?;
        receive_reply(
            reply_receiver,
            "configuring video output",
            WORKER_REPLY_TIMEOUT,
        )?
    }

    /// Attach an externally owned frame mailbox; graphics objects stay in the host.
    pub fn attach_video_mailbox(&self, mailbox: FrameMailbox) -> Result<(), String> {
        self.set_video_mailbox(Some(mailbox))
    }

    /// Detach the externally owned frame mailbox without closing it.
    pub fn detach_video_mailbox(&self) -> Result<(), String> {
        self.set_video_mailbox(None)
    }
}

impl Drop for Sg1000TemporalWorker {
    fn drop(&mut self) {
        let _ = self.commands.send(WorkerCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioOutputMetrics, SG1000_WORKER_SAVE_STATE_VERSION, Sg1000TemporalWorker};
    use sega_sg1000_core::ControllerButtons;
    use sega_video_runtime::{FrameMailbox, PixelFormat};

    #[test]
    fn paused_worker_steps_one_complete_frame_into_the_external_mailbox() {
        let worker = Sg1000TemporalWorker::new(vec![0x76]).unwrap();
        let mailbox = FrameMailbox::new();
        worker.attach_video_mailbox(mailbox.clone()).unwrap();

        let instructions = worker.step_frame(20_000).unwrap();
        let frame = mailbox.take_latest().unwrap().unwrap();

        assert_eq!(instructions, 14_934);
        assert_eq!(worker.frame_number(), 1);
        assert_eq!(frame.sequence(), 1);
        assert_eq!(frame.width(), 256);
        assert_eq!(frame.height(), 192);
        assert_eq!(frame.pixel_format(), PixelFormat::Rgb888);
        assert_eq!(frame.pixels().len(), 256 * 192 * 3);
        assert!(!worker.is_running());
    }

    #[test]
    fn worker_applies_controller_snapshots_before_scheduled_port_reads() {
        let worker = Sg1000TemporalWorker::new(vec![0xDB, 0xDC, 0x76]).unwrap();
        worker
            .set_controller(
                1,
                ControllerButtons {
                    up: true,
                    ..ControllerButtons::default()
                },
            )
            .unwrap();

        worker.step_frame(20_000).unwrap();

        assert_eq!(worker.debug_snapshot().unwrap().a, 0xFE);
    }

    #[test]
    fn save_state_roundtrip_is_bound_to_rom_and_memory_profile() {
        let worker = Sg1000TemporalWorker::new(vec![0x76]).unwrap();
        worker.step_frame(20_000).unwrap();
        let snapshot = worker.save_state().unwrap();

        assert_eq!(snapshot.format_version, SG1000_WORKER_SAVE_STATE_VERSION);
        worker.reset().unwrap();
        worker.load_state(snapshot.clone()).unwrap();
        assert_eq!(worker.save_state().unwrap(), snapshot);

        let other = Sg1000TemporalWorker::new(vec![0x00]).unwrap();
        assert!(
            other
                .load_state(snapshot)
                .unwrap_err()
                .contains("does not match")
        );
    }

    #[test]
    fn invalid_rom_replacement_keeps_the_existing_machine_usable() {
        let worker = Sg1000TemporalWorker::new(vec![0x00, 0x76]).unwrap();
        assert!(worker.load_rom(Vec::new()).is_err());

        worker.step_frame(20_000).unwrap();
        assert_eq!(worker.frame_number(), 1);
    }
    #[derive(Default)]
    struct RecordingSinkState {
        writes: usize,
        suspend_count: usize,
        resume_count: usize,
        closed: bool,
    }

    struct RecordingSink(std::sync::Arc<std::sync::Mutex<RecordingSinkState>>);

    impl sega_audio_runtime::PcmSink for RecordingSink {
        fn write_samples(&mut self, samples: Vec<f64>) -> Result<(), String> {
            self.0.lock().unwrap().writes += samples.len();
            Ok(())
        }

        fn suspend_and_flush(&mut self) -> Result<(), String> {
            self.0.lock().unwrap().suspend_count += 1;
            Ok(())
        }

        fn resume(&mut self) -> Result<(), String> {
            self.0.lock().unwrap().resume_count += 1;
            Ok(())
        }

        fn metrics(&self) -> AudioOutputMetrics {
            AudioOutputMetrics::default()
        }

        fn close(&mut self) {
            self.0.lock().unwrap().closed = true;
        }
    }

    #[test]
    fn audio_factory_and_session_lifecycle_stay_on_the_machine_worker_thread() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        use sega_audio_runtime::{
            AudioSession, DEFAULT_PCM_SOURCE_RATE_HZ, NativeAudioSession,
        };

        let caller_thread = thread::current().id();
        let factory_thread = Arc::new(Mutex::new(None));
        let sink_state = Arc::new(Mutex::new(RecordingSinkState::default()));
        {
            let worker = Sg1000TemporalWorker::new(vec![0x76]).unwrap();
            let factory_thread_for_worker = Arc::clone(&factory_thread);
            let sink_state_for_worker = Arc::clone(&sink_state);
            worker
                .configure_audio_output(move || {
                    *factory_thread_for_worker.lock().unwrap() = Some(thread::current().id());
                    let session = NativeAudioSession::new(
                        DEFAULT_PCM_SOURCE_RATE_HZ,
                        DEFAULT_PCM_SOURCE_RATE_HZ,
                        100,
                        RecordingSink(sink_state_for_worker),
                    )?;
                    Ok(Box::new(session) as Box<dyn AudioSession>)
                })
                .unwrap();

            // A debugger frame step advances the same DSP path but must not
            // submit PCM to the suspended physical sink.
            worker.step_frame(20_000).unwrap();
            assert_eq!(sink_state.lock().unwrap().writes, 0);

            worker.start().unwrap();
            worker.pause().unwrap();
            assert!(!worker.is_running());
        }

        assert_ne!(factory_thread.lock().unwrap().unwrap(), caller_thread);
        let state = sink_state.lock().unwrap();
        assert!(state.suspend_count >= 2);
        assert!(state.resume_count >= 1);
        assert!(state.closed);
    }
}
