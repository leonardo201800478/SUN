//! Transactional SMS memory/I/O bus adapter for the Scheduler-driven path.
//!
//! Memory and port transactions become visible only at their scheduled
//! completion boundary. Controller/control-port effects, the shared
//! scheduler-driven PSG, optional Japanese SMS YM2413 register writes, and
//! SMS1 VDP port effects are applied there. The YM2413 input clock advances
//! from the same Scheduler timeline as the Z80/PSG. The VDP scanline clock
//! advances from Scheduler-dispatched Z80 cycles; dot/fetch
//! arbitration and mid-scanline effects remain outside this slice.
//! Unconnected reads return open bus, port writes are retained in a bounded
//! diagnostic trace, and INT acknowledge data comes from an explicit external
//! vector input.

use core::fmt;
use std::collections::VecDeque;
use std::time::Instant;

use crate::{
    CartridgeImage, ControllerButtons, PsgError, SMS_PSG_CLOCK_DOMAIN_ID, SmsCoreError, SmsIo,
    SmsIoError, SmsMemory, SmsTemporalCoordinator, SmsTemporalCoordinatorError,
    SmsTemporalCoordinatorState, SmsTemporalEvent, SmsVdp, Territory,
};
use sega_audio_runtime::{DEFAULT_PCM_SOURCE_RATE_HZ, PsgClockError, ScheduledPsg};
use sega_sn76489::{PsgAmplitudeEvent, PsgEventCaptureError, PsgProfile, Sn76489Core};
use sega_temporal_core::{HardwarePhase, MachineTime};
use sega_video::sms::SmsVdpProfile;
use sega_ym2413::{
    AttackRate15Transition, DampTransitionConfig, DampTransitionCoordination, EnvelopeStage,
    YM2413_CHANNEL_OUTPUT_MAX, YM2413_INPUT_CLOCKS_PER_SLOT, YM2413_SLOTS_PER_ROTATION,
    Ym2413ChannelInitialState, Ym2413Core, Ym2413CoreOutputPipelineFrame, Ym2413Error,
    Ym2413PgSerialAdvanceError, Ym2413PgSerialError, Ym2413PgSerialService,
    Ym2413PgSerialServiceState, Ym2413State, Ym2413SynthesisError, Ym2413SynthesisInitialState,
};
use sega_z80_cpu::{Z80_INT_ACK_AUTOMATIC_WAIT_TSTATES, Z80BusOperation, Z80BusRequest};

pub const SMS_TEMPORAL_BUS_STATE_VERSION: u16 = 24;
pub const SMS_YM2413_PCM_SOURCE_RATE_HZ: u32 = 49_716;
const SMS_YM2413_PCM_ACCUMULATOR_STATE_VERSION: u16 = 1;
const DEFAULT_INTERRUPT_VECTOR: u8 = 0xFF;
const DEFAULT_SMS_PSG_CLOCK_HZ: u64 = 3_579_545;
const DEFAULT_AUDIO_SAMPLE_RATE: u64 = DEFAULT_PCM_SOURCE_RATE_HZ as u64;

/// Deterministic software reset hypothesis for the Japanese SMS FM path.
/// These initial conditions are explicit model inputs, not a claim about the
/// chip's unmeasured physical power-on state.
fn sms_ym2413_synthesis_initial_state() -> Ym2413SynthesisInitialState {
    let damp_config = DampTransitionConfig::new(
        124,
        DampTransitionCoordination::IndependentPerOperator,
        AttackRate15Transition::SkipAttackToDecay,
    )
    .expect("the static SMS YM2413 DAMP policy is valid");
    let channels = std::array::from_fn(|channel| Ym2413ChannelInitialState {
        phase: [channel as u32 * 0x1000, channel as u32 * 0x1800],
        envelope_level: [127, 127],
        envelope_stage: [EnvelopeStage::Damp; 2],
        envelope_key_on: [false; 2],
        feedback_history: [0; 2],
        previous_modulator_phase_index: None,
    });
    Ym2413SynthesisInitialState {
        channels,
        pm_lfo_counter: 0,
        am_lfo_phase: 0,
        damp_config,
    }
}

fn elapsed_tstates(
    started_at: MachineTime,
    completed_at: MachineTime,
    master_ticks_per_tstate: u64,
) -> Option<u128> {
    let scale = u128::from(master_ticks_per_tstate);
    if scale == 0 {
        return None;
    }
    let elapsed = completed_at.checked_duration_since(started_at)?;
    if elapsed == 0 || elapsed % scale != 0 {
        return None;
    }
    let tstates = elapsed / scale;
    (tstates <= u128::from(u32::MAX)).then_some(tstates)
}

fn valid_bus_deadline(
    started_at: MachineTime,
    deadline: MachineTime,
    master_ticks_per_tstate: u64,
    minimum_tstates: u16,
) -> bool {
    elapsed_tstates(started_at, deadline, master_ticks_per_tstate)
        .is_some_and(|total| total >= u128::from(minimum_tstates))
}

fn valid_internal_deadline(
    started_at: MachineTime,
    deadline: MachineTime,
    master_ticks_per_tstate: u64,
) -> bool {
    elapsed_tstates(started_at, deadline, master_ticks_per_tstate).is_some()
}

/// Result latched at the completion boundary of one bus transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmsBusCompletion {
    pub request_id: u64,
    pub data: Option<u8>,
    pub completed_at: MachineTime,
}

/// Completion of a CPU-internal interval with no memory-bus transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmsInternalCycleCompletion {
    pub continuation_id: u64,
    pub completed_at: MachineTime,
}

/// One dispatched event owned by the SMS temporal bus/coordinator boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmsTemporalDispatch {
    BusCompletion(SmsBusCompletion),
    InternalCycleComplete(SmsInternalCycleCompletion),
    M1TStateComplete {
        continuation_id: u64,
        completed_at: MachineTime,
    },
    ExternalThTransition {
        player: u8,
        high: bool,
        completed_at: MachineTime,
    },
}

/// Monotonic diagnostic counters for writes committed to the Japanese SMS YM2413 ports.
/// These counters observe completed Z80 bus writes; they are not emulated chip state and
/// intentionally reset when a machine is reconstructed from a save state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmsFmPortWriteSnapshot {
    pub address_writes: u64,
    pub data_writes: u64,
    pub control_writes: u64,
    pub last_address: Option<u8>,
    pub last_control: Option<u8>,
    pub register_writes: [u64; 64],
    pub last_register_values: [Option<u8>; 64],
}

impl Default for SmsFmPortWriteSnapshot {
    fn default() -> Self {
        Self {
            address_writes: 0,
            data_writes: 0,
            control_writes: 0,
            last_address: None,
            last_control: None,
            register_writes: [0; 64],
            last_register_values: [None; 64],
        }
    }
}

/// Complete in-memory continuation of the transactional bus adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct SmsTemporalBusState {
    pub format_version: u16,
    pub interrupt_vector: u8,
    pub memory: SmsMemory,
    pub io: SmsIo,
    pub psg: ScheduledPsg,
    pub pending_psg_samples: VecDeque<f64>,
    pub ym2413: Option<Ym2413State>,
    pub ym2413_pg_service: Option<Ym2413PgSerialServiceState>,
    pub pending_fm_samples: VecDeque<Option<f64>>,
    pub ym2413_pcm_accumulator: SmsYm2413PcmAccumulatorState,
    pub fm_control: u8,
    pub vdp: SmsVdp,
    pub coordinator: SmsTemporalCoordinatorState,
    pub pending_request: Option<Z80BusRequest>,
    pub request_started_at: Option<MachineTime>,
    pub completion: Option<SmsBusCompletion>,
    pub pending_internal_cycle: Option<u64>,
    pub internal_cycle_started_at: Option<MachineTime>,
    pub internal_completion: Option<SmsInternalCycleCompletion>,
}

/// Snapshot of the partial 18-slot YM2413 mono PCM accumulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmsYm2413PcmAccumulatorState {
    pub format_version: u16,
    pub next_cycle: u8,
    pub frame_count: u8,
    pub sum_mo: i64,
    pub sum_ro: i64,
    pub valid: bool,
}

impl Default for SmsYm2413PcmAccumulatorState {
    fn default() -> Self {
        Self {
            format_version: SMS_YM2413_PCM_ACCUMULATOR_STATE_VERSION,
            next_cycle: 0,
            frame_count: 0,
            sum_mo: 0,
            sum_ro: 0,
            valid: true,
        }
    }
}

impl SmsYm2413PcmAccumulatorState {
    fn is_valid(self) -> bool {
        self.format_version == SMS_YM2413_PCM_ACCUMULATOR_STATE_VERSION
            && u64::from(self.next_cycle) < YM2413_SLOTS_PER_ROTATION
            && self.frame_count == self.next_cycle
            && self.sum_mo.unsigned_abs()
                <= ((i64::from(YM2413_CHANNEL_OUTPUT_MAX) + 1) * i64::from(self.frame_count)) as u64
            && self.sum_ro.unsigned_abs()
                <= ((i64::from(YM2413_CHANNEL_OUTPUT_MAX) + 1) * i64::from(self.frame_count)) as u64
    }
}

/// Errors from the transactional SMS memory-bus adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmsTemporalBusError {
    Core(SmsCoreError),
    Psg(PsgError),
    PsgClock(PsgClockError),
    Ym2413(Ym2413Error),
    Ym2413Synthesis(Ym2413SynthesisError),
    Ym2413PgSerial(Ym2413PgSerialError),
    Ym2413PgSerialAdvance(Ym2413PgSerialAdvanceError),
    Coordinator(SmsTemporalCoordinatorError),
    Io(SmsIoError),
    Busy,
    NoPendingRequest,
    WrongRequestId { expected: u64, received: u64 },
    UnsupportedOperation,
    UnsupportedBiosHardwareProfile,
    UnsupportedFmHardwareProfile,
    InvalidState,
}

impl fmt::Display for SmsTemporalBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => write!(f, "SMS memory initialization failed: {error}"),
            Self::Psg(error) => write!(f, "SMS temporal PSG initialization failed: {error}"),
            Self::PsgClock(error) => write!(f, "SMS temporal PSG clock advance failed: {error}"),
            Self::Ym2413(error) => write!(f, "SMS temporal YM2413 clock advance failed: {error}"),
            Self::Ym2413Synthesis(error) => {
                write!(
                    f,
                    "SMS temporal YM2413 synthesis initialization failed: {error:?}"
                )
            }
            Self::Ym2413PgSerial(error) => {
                write!(f, "SMS temporal YM2413 serial service failed: {error:?}")
            }
            Self::Ym2413PgSerialAdvance(error) => {
                write!(f, "SMS temporal YM2413 serial advance failed: {error:?}")
            }
            Self::Coordinator(error) => write!(f, "SMS temporal coordinator failed: {error}"),
            Self::Io(error) => write!(f, "SMS temporal controller input failed: {error}"),
            Self::Busy => {
                f.write_str("SMS temporal bus already owns an unacknowledged transaction")
            }
            Self::NoPendingRequest => f.write_str("SMS temporal bus has no pending request"),
            Self::WrongRequestId { expected, received } => write!(
                f,
                "SMS temporal bus expected request {expected}, received {received}"
            ),
            Self::UnsupportedOperation => f.write_str(
                "this temporal bus operation is not implemented by the SMS temporal slice",
            ),
            Self::UnsupportedBiosHardwareProfile => f.write_str(
                "the conventional 8 KiB BIOS mapping is currently validated only for SMS1 hardware",
            ),
            Self::UnsupportedFmHardwareProfile => f.write_str(
                "the selected SMS board profile does not provide integrated YM2413 hardware",
            ),
            Self::InvalidState => f.write_str("SMS temporal bus snapshot is inconsistent"),
        }
    }
}

impl std::error::Error for SmsTemporalBusError {}

impl From<SmsCoreError> for SmsTemporalBusError {
    fn from(error: SmsCoreError) -> Self {
        Self::Core(error)
    }
}

impl From<PsgClockError> for SmsTemporalBusError {
    fn from(error: PsgClockError) -> Self {
        Self::PsgClock(error)
    }
}

impl From<Ym2413Error> for SmsTemporalBusError {
    fn from(error: Ym2413Error) -> Self {
        Self::Ym2413(error)
    }
}

impl From<Ym2413SynthesisError> for SmsTemporalBusError {
    fn from(error: Ym2413SynthesisError) -> Self {
        Self::Ym2413Synthesis(error)
    }
}

impl From<Ym2413PgSerialError> for SmsTemporalBusError {
    fn from(error: Ym2413PgSerialError) -> Self {
        Self::Ym2413PgSerial(error)
    }
}

impl From<Ym2413PgSerialAdvanceError> for SmsTemporalBusError {
    fn from(error: Ym2413PgSerialAdvanceError) -> Self {
        Self::Ym2413PgSerialAdvance(error)
    }
}

impl From<PsgError> for SmsTemporalBusError {
    fn from(error: PsgError) -> Self {
        Self::Psg(error)
    }
}

impl From<SmsTemporalCoordinatorError> for SmsTemporalBusError {
    fn from(error: SmsTemporalCoordinatorError) -> Self {
        Self::Coordinator(error)
    }
}

impl From<SmsIoError> for SmsTemporalBusError {
    fn from(error: SmsIoError) -> Self {
        Self::Io(error)
    }
}

/// SMS mapper-backed memory and its scheduler-owned bus completion timeline.
#[derive(Debug, Clone)]
pub struct SmsTemporalBus {
    memory: SmsMemory,
    io: SmsIo,
    psg: ScheduledPsg,
    pending_psg_samples: VecDeque<f64>,
    ym2413: Option<Ym2413Core>,
    ym2413_pg_service: Option<Ym2413PgSerialService>,
    pending_fm_samples: VecDeque<Option<f64>>,
    ym2413_pcm_accumulator: SmsYm2413PcmAccumulatorState,
    fm_control: u8,
    vdp: SmsVdp,
    coordinator: SmsTemporalCoordinator,
    pending_request: Option<Z80BusRequest>,
    request_started_at: Option<MachineTime>,
    completion: Option<SmsBusCompletion>,
    pending_internal_cycle: Option<u64>,
    internal_cycle_started_at: Option<MachineTime>,
    internal_completion: Option<SmsInternalCycleCompletion>,
    interrupt_vector: u8,
    /// Set after an in-place YM2413 failure; this machine instance cannot resume.
    ym2413_runtime_faulted: bool,
    /// Host-output capture policy; independent of the physical YM2413 clock.
    fm_audio_capture_enabled: bool,
    /// Wait until slot zero before restarting capture after a mute transition.
    fm_capture_wait_for_rotation: bool,
    /// Optional wall-clock instrumentation; disabled in normal emulation.
    ym2413_timing_enabled: bool,
    ym2413_advance_elapsed_ns: u128,
    /// Diagnostic-only counts of committed F0/F1/F2 writes.
    fm_port_writes: SmsFmPortWriteSnapshot,
}

impl SmsTemporalBus {
    /// Construct a bus from a cartridge and an explicit master-tick/T-state scale.
    pub fn new(
        rom: impl Into<Vec<u8>>,
        master_ticks_per_tstate: u64,
    ) -> Result<Self, SmsTemporalBusError> {
        Self::new_with_psg_config(
            rom,
            master_ticks_per_tstate,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            DEFAULT_AUDIO_SAMPLE_RATE,
        )
    }

    /// Construct a bus with an explicit SMS PSG input clock and internal PCM source rate.
    ///
    /// The PSG input clock must follow the selected SMS CPU/PSG clock profile.
    /// The PCM rate controls PSG sample generation; it never clocks the chip or Scheduler.
    /// Host endpoint conversion is performed independently by the native audio session.
    pub fn new_with_psg_config(
        rom: impl Into<Vec<u8>>,
        master_ticks_per_tstate: u64,
        psg_clock_hz: u64,
        sample_rate: u64,
    ) -> Result<Self, SmsTemporalBusError> {
        Self::new_with_hardware_profile(
            rom,
            master_ticks_per_tstate,
            psg_clock_hz,
            sample_rate,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Export,
        )
    }

    /// Construct the temporal bus with explicit VDP ASIC and territory profiles.
    pub fn new_with_hardware_profile(
        rom: impl Into<Vec<u8>>,
        master_ticks_per_tstate: u64,
        psg_clock_hz: u64,
        sample_rate: u64,
        vdp_profile: SmsVdpProfile,
        territory: Territory,
    ) -> Result<Self, SmsTemporalBusError> {
        Self::new_with_hardware_profile_and_sram_size(
            rom,
            master_ticks_per_tstate,
            psg_clock_hz,
            sample_rate,
            vdp_profile,
            territory,
            None,
        )
    }

    /// Construct an explicit Japanese Master System board profile with integrated YM2413.
    ///
    /// This selects an explicit physical board profile, independent of the
    /// software territory. Territory configures console I/O; it does not imply FM.
    pub fn new_with_japanese_fm_hardware_profile(
        rom: impl Into<Vec<u8>>,
        master_ticks_per_tstate: u64,
        psg_clock_hz: u64,
        sample_rate: u64,
        vdp_profile: SmsVdpProfile,
        territory: Territory,
    ) -> Result<Self, SmsTemporalBusError> {
        let bus = Self::new_with_hardware_profile(
            rom,
            master_ticks_per_tstate,
            psg_clock_hz,
            sample_rate,
            vdp_profile,
            territory,
        )?;
        Self::install_japanese_fm_hardware_profile(bus, master_ticks_per_tstate)
    }

    /// Construct the Japanese Master System FM board profile with explicit cartridge SRAM capacity.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_japanese_fm_hardware_profile_and_sram_size(
        rom: impl Into<Vec<u8>>,
        master_ticks_per_tstate: u64,
        psg_clock_hz: u64,
        sample_rate: u64,
        vdp_profile: SmsVdpProfile,
        territory: Territory,
        cartridge_sram_size: Option<usize>,
    ) -> Result<Self, SmsTemporalBusError> {
        let bus = Self::new_with_hardware_profile_and_sram_size(
            rom,
            master_ticks_per_tstate,
            psg_clock_hz,
            sample_rate,
            vdp_profile,
            territory,
            cartridge_sram_size,
        )?;
        Self::install_japanese_fm_hardware_profile(bus, master_ticks_per_tstate)
    }

    fn install_japanese_fm_hardware_profile(
        mut bus: Self,
        master_ticks_per_tstate: u64,
    ) -> Result<Self, SmsTemporalBusError> {
        bus.coordinator = SmsTemporalCoordinator::new_with_ym2413(master_ticks_per_tstate)?;
        bus.ym2413 = Some(Ym2413Core::new());
        bus.ym2413_pg_service = Some(Ym2413PgSerialService::with_initial_state(
            sms_ym2413_synthesis_initial_state(),
        )?);
        bus.pending_fm_samples.clear();
        bus.ym2413_pcm_accumulator = SmsYm2413PcmAccumulatorState::default();
        Ok(bus)
    }

    /// Construct the temporal bus with an explicit cartridge SRAM hardware capacity.
    /// Capacity comes from the cartridge profile, never from ROM or save-file size.
    pub fn new_with_hardware_profile_and_sram_size(
        rom: impl Into<Vec<u8>>,
        master_ticks_per_tstate: u64,
        psg_clock_hz: u64,
        sample_rate: u64,
        vdp_profile: SmsVdpProfile,
        territory: Territory,
        cartridge_sram_size: Option<usize>,
    ) -> Result<Self, SmsTemporalBusError> {
        sega_observability::tracing::info!(
            target: "sega::machine",
            master_ticks_per_tstate,
            psg_clock_hz,
            sample_rate,
            vdp_profile = ?vdp_profile,
            territory = ?territory,
            bios_present = false,
            "SMS temporal bus configuration selected"
        );
        let cartridge = CartridgeImage::new(rom)?;
        let memory = SmsMemory::with_cartridge_sram_size(cartridge, cartridge_sram_size)?;
        let coordinator = SmsTemporalCoordinator::new(master_ticks_per_tstate)?;
        let psg = ScheduledPsg::new(
            SMS_PSG_CLOCK_DOMAIN_ID,
            PsgProfile::Sms,
            psg_clock_hz,
            sample_rate,
        )?;
        Ok(Self {
            memory,
            io: SmsIo::new(territory),
            psg,
            pending_psg_samples: VecDeque::new(),
            ym2413: None,
            ym2413_pg_service: None,
            pending_fm_samples: VecDeque::new(),
            ym2413_pcm_accumulator: SmsYm2413PcmAccumulatorState::default(),
            fm_control: 0,
            vdp: SmsVdp::new_with_profile(vdp_profile),
            coordinator,
            pending_request: None,
            request_started_at: None,
            completion: None,
            pending_internal_cycle: None,
            internal_cycle_started_at: None,
            internal_completion: None,
            interrupt_vector: DEFAULT_INTERRUPT_VECTOR,
            ym2413_runtime_faulted: false,
            fm_audio_capture_enabled: true,
            fm_capture_wait_for_rotation: false,
            ym2413_timing_enabled: false,
            ym2413_advance_elapsed_ns: 0,
            fm_port_writes: SmsFmPortWriteSnapshot::default(),
        })
    }

    /// Construct the temporal bus with an actual 8 KiB SMS BIOS image.
    ///
    /// The CPU still starts at its hardware reset vector. The BIOS executes on
    /// the shared Scheduler and establishes the cartridge handoff state through
    /// ordinary hardware accesses.
    pub fn new_with_hardware_profile_and_bios(
        rom: impl Into<Vec<u8>>,
        bios: Vec<u8>,
        master_ticks_per_tstate: u64,
        psg_clock_hz: u64,
        sample_rate: u64,
        vdp_profile: SmsVdpProfile,
        territory: Territory,
    ) -> Result<Self, SmsTemporalBusError> {
        Self::new_with_hardware_profile_and_bios_and_sram_size(
            rom,
            bios,
            master_ticks_per_tstate,
            psg_clock_hz,
            sample_rate,
            vdp_profile,
            territory,
            None,
        )
    }

    /// Construct the temporal bus with BIOS firmware and explicit cartridge SRAM hardware.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_hardware_profile_and_bios_and_sram_size(
        rom: impl Into<Vec<u8>>,
        bios: Vec<u8>,
        master_ticks_per_tstate: u64,
        psg_clock_hz: u64,
        sample_rate: u64,
        vdp_profile: SmsVdpProfile,
        territory: Territory,
        cartridge_sram_size: Option<usize>,
    ) -> Result<Self, SmsTemporalBusError> {
        // SmsMemory currently models SMS1 slot priority. SMS2 has distinct
        // wired-AND arbitration, so reject BIOS mapping under a 315-5246 profile.
        if vdp_profile.revision.asic_number() != 3_155_124 {
            return Err(SmsTemporalBusError::UnsupportedBiosHardwareProfile);
        }
        sega_observability::tracing::info!(
            target: "sega::machine",
            master_ticks_per_tstate,
            psg_clock_hz,
            sample_rate,
            vdp_profile = ?vdp_profile,
            territory = ?territory,
            bios_present = true,
            "SMS temporal bus configuration selected"
        );
        let cartridge = CartridgeImage::new(rom)?;
        let memory =
            SmsMemory::with_bios_and_sram_size(cartridge, Some(bios), cartridge_sram_size)?;
        let coordinator = SmsTemporalCoordinator::new(master_ticks_per_tstate)?;
        let psg = ScheduledPsg::new(
            SMS_PSG_CLOCK_DOMAIN_ID,
            PsgProfile::Sms,
            psg_clock_hz,
            sample_rate,
        )?;
        Ok(Self {
            memory,
            io: SmsIo::new(territory),
            psg,
            pending_psg_samples: VecDeque::new(),
            ym2413: None,
            ym2413_pg_service: None,
            pending_fm_samples: VecDeque::new(),
            ym2413_pcm_accumulator: SmsYm2413PcmAccumulatorState::default(),
            fm_control: 0,
            vdp: SmsVdp::new_with_profile(vdp_profile),
            coordinator,
            pending_request: None,
            request_started_at: None,
            completion: None,
            pending_internal_cycle: None,
            internal_cycle_started_at: None,
            internal_completion: None,
            interrupt_vector: DEFAULT_INTERRUPT_VECTOR,
            ym2413_runtime_faulted: false,
            fm_audio_capture_enabled: true,
            fm_capture_wait_for_rotation: false,
            ym2413_timing_enabled: false,
            ym2413_advance_elapsed_ns: 0,
            fm_port_writes: SmsFmPortWriteSnapshot::default(),
        })
    }

    /// Construct an explicit Japanese Master System board profile with BIOS and YM2413.
    pub fn new_with_japanese_fm_hardware_profile_and_bios(
        rom: impl Into<Vec<u8>>,
        bios: Vec<u8>,
        master_ticks_per_tstate: u64,
        psg_clock_hz: u64,
        sample_rate: u64,
        vdp_profile: SmsVdpProfile,
        territory: Territory,
    ) -> Result<Self, SmsTemporalBusError> {
        let bus = Self::new_with_hardware_profile_and_bios(
            rom,
            bios,
            master_ticks_per_tstate,
            psg_clock_hz,
            sample_rate,
            vdp_profile,
            territory,
        )?;
        Self::install_japanese_fm_hardware_profile(bus, master_ticks_per_tstate)
    }

    /// Construct the Japanese Master System FM board profile with BIOS and cartridge SRAM capacity.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_japanese_fm_hardware_profile_and_bios_and_sram_size(
        rom: impl Into<Vec<u8>>,
        bios: Vec<u8>,
        master_ticks_per_tstate: u64,
        psg_clock_hz: u64,
        sample_rate: u64,
        vdp_profile: SmsVdpProfile,
        territory: Territory,
        cartridge_sram_size: Option<usize>,
    ) -> Result<Self, SmsTemporalBusError> {
        let bus = Self::new_with_hardware_profile_and_bios_and_sram_size(
            rom,
            bios,
            master_ticks_per_tstate,
            psg_clock_hz,
            sample_rate,
            vdp_profile,
            territory,
            cartridge_sram_size,
        )?;
        Self::install_japanese_fm_hardware_profile(bus, master_ticks_per_tstate)
    }

    /// Schedule an external TH transition without advancing the machine immediately.
    pub fn schedule_external_th_transition(
        &mut self,
        player: u8,
        high: bool,
        timestamp: MachineTime,
    ) -> Result<MachineTime, SmsTemporalBusError> {
        Ok(self
            .coordinator
            .schedule_external_th_transition(player, high, timestamp)?)
    }

    /// Begin one memory or port request and schedule its completion deadline.
    ///
    /// WAIT is supplied by a deterministic fixture; this API does not sample
    /// or claim a physical SMS WAIT signal.
    pub fn begin_request(
        &mut self,
        request: Z80BusRequest,
        fixture_wait_tstates: u32,
    ) -> Result<MachineTime, SmsTemporalBusError> {
        if self.pending_request.is_some()
            || self.completion.is_some()
            || self.pending_internal_cycle.is_some()
            || self.internal_completion.is_some()
        {
            sega_observability::tracing::warn!(
                target: "sega::bus",
                request_id = request.id,
                address = request.address,
                operation = ?request.operation,
                "SMS bus request rejected while another operation is pending"
            );
            return Err(SmsTemporalBusError::Busy);
        }
        if !matches!(
            request.operation,
            Z80BusOperation::OpcodeFetch
                | Z80BusOperation::MemoryRead
                | Z80BusOperation::MemoryWrite { .. }
                | Z80BusOperation::PortRead
                | Z80BusOperation::PortWrite { .. }
                | Z80BusOperation::InterruptAcknowledge
        ) {
            sega_observability::tracing::error!(
                target: "sega::bus",
                request_id = request.id,
                address = request.address,
                operation = ?request.operation,
                "unsupported Z80 bus operation rejected by SMS temporal bus"
            );
            return Err(SmsTemporalBusError::UnsupportedOperation);
        }
        let started_at = self.coordinator.now();
        let automatic_waits = if request.operation == Z80BusOperation::InterruptAcknowledge {
            u32::from(Z80_INT_ACK_AUTOMATIC_WAIT_TSTATES)
        } else {
            0
        };
        let total_wait_tstates = fixture_wait_tstates
            .checked_add(automatic_waits)
            .ok_or(SmsTemporalBusError::InvalidState)?;
        let deadline = self.coordinator.schedule_bus_completion(
            request.id,
            request.base_tstates,
            total_wait_tstates,
        )?;
        self.pending_request = Some(request);
        self.request_started_at = Some(started_at);
        Ok(deadline)
    }

    /// Schedule one internal CPU interval without creating a fake bus access.
    pub fn schedule_internal_cycle(
        &mut self,
        continuation_id: u64,
        tstates: u32,
    ) -> Result<MachineTime, SmsTemporalBusError> {
        if self.pending_request.is_some()
            || self.completion.is_some()
            || self.pending_internal_cycle.is_some()
        {
            sega_observability::tracing::warn!(
                target: "sega::bus",
                continuation_id,
                tstates,
                "SMS internal cycle rejected while bus is busy"
            );
            return Err(SmsTemporalBusError::Busy);
        }
        let started_at = self.coordinator.now();
        let deadline = self
            .coordinator
            .schedule_internal_cycle(continuation_id, tstates)?;
        self.pending_internal_cycle = Some(continuation_id);
        self.internal_cycle_started_at = Some(started_at);
        Ok(deadline)
    }

    /// Schedule one explicit M1 T-state on the same coordinator as memory cycles.
    pub fn schedule_m1_tstate(
        &mut self,
        continuation_id: u64,
    ) -> Result<MachineTime, SmsTemporalBusError> {
        if self.pending_request.is_some()
            || self.completion.is_some()
            || self.pending_internal_cycle.is_some()
            || self.internal_completion.is_some()
        {
            sega_observability::tracing::warn!(
                target: "sega::bus",
                continuation_id,
                "SMS M1 T-state rejected while bus is busy"
            );
            return Err(SmsTemporalBusError::Busy);
        }
        Ok(self.coordinator.schedule_m1_tstate(continuation_id)?)
    }

    /// Dispatch the next scheduled boundary and apply its bus side effect once.
    ///
    /// A read samples memory at this completion boundary. A write mutates memory
    /// at this completion boundary. The response is latched until acknowledged.
    pub fn dispatch_next(&mut self) -> Result<Option<SmsTemporalDispatch>, SmsTemporalBusError> {
        if self.ym2413_runtime_faulted {
            return Err(SmsTemporalBusError::InvalidState);
        }
        // Dispatched boundaries remain latched until the CPU accepts them.
        if let Some(completion) = self.completion {
            return Ok(Some(SmsTemporalDispatch::BusCompletion(completion)));
        }
        if let Some(completion) = self.internal_completion {
            return Ok(Some(SmsTemporalDispatch::InternalCycleComplete(completion)));
        }
        let Some(dispatch) = self.coordinator.dispatch_next()? else {
            return Ok(None);
        };
        let psg_advance = dispatch
            .clock_advances
            .iter()
            .find(|advance| advance.domain == SMS_PSG_CLOCK_DOMAIN_ID)
            .ok_or(SmsTemporalBusError::InvalidState)?;
        self.psg.apply_clock_advance(psg_advance)?;
        if self.ym2413.is_some() {
            let generated_psg_samples = self.psg.core_mut().drain_samples(None);
            if self.psg_output_enabled() {
                self.pending_psg_samples.extend(generated_psg_samples);
            } else {
                self.pending_psg_samples
                    .extend(std::iter::repeat_n(0.0, generated_psg_samples.len()));
            }
        }
        if self.ym2413.is_some() {
            let advance = dispatch
                .clock_advances
                .iter()
                .find(|advance| advance.domain == crate::SMS_YM2413_CLOCK_DOMAIN_ID)
                .ok_or(SmsTemporalBusError::InvalidState)?;
            self.advance_ym2413(advance.elapsed_cycles)?;
        }
        let z80_advance = dispatch
            .clock_advances
            .iter()
            .find(|advance| advance.domain == crate::SMS_VDP_CLOCK_DOMAIN_ID)
            .ok_or(SmsTemporalBusError::InvalidState)?;
        let vdp_tstates = u64::try_from(z80_advance.elapsed_cycles)
            .map_err(|_| SmsTemporalBusError::InvalidState)?;
        self.vdp.advance(vdp_tstates);
        match dispatch.event.payload {
            SmsTemporalEvent::ExternalThTransition { player, high } => {
                self.io.set_external_th_level(player, high)?;
                Ok(Some(SmsTemporalDispatch::ExternalThTransition {
                    player,
                    high,
                    completed_at: dispatch.event.timestamp,
                }))
            }
            SmsTemporalEvent::BusCompletion { request_id } => {
                let request = self
                    .pending_request
                    .ok_or(SmsTemporalBusError::NoPendingRequest)?;
                if request.id != request_id {
                    sega_observability::tracing::error!(
                        target: "sega::bus",
                        expected_request_id = request.id,
                        received_request_id = request_id,
                        "SMS bus completion request identity mismatch"
                    );
                    return Err(SmsTemporalBusError::WrongRequestId {
                        expected: request.id,
                        received: request_id,
                    });
                }
                if self.completion.is_some() {
                    return Err(SmsTemporalBusError::Busy);
                }

                let data = match request.operation {
                    Z80BusOperation::OpcodeFetch | Z80BusOperation::MemoryRead => {
                        Some(self.memory.read(request.address))
                    }
                    Z80BusOperation::InterruptAcknowledge => Some(self.interrupt_vector),
                    Z80BusOperation::MemoryWrite { value } => {
                        self.memory.write(request.address, value);
                        None
                    }
                    Z80BusOperation::PortRead => Some(self.read_port(request.address)),
                    Z80BusOperation::PortWrite { value } => {
                        self.write_port(request.address, value);
                        None
                    }
                };
                let completion = SmsBusCompletion {
                    request_id,
                    data,
                    completed_at: dispatch.event.timestamp,
                };
                self.completion = Some(completion);
                Ok(Some(SmsTemporalDispatch::BusCompletion(completion)))
            }
            SmsTemporalEvent::InternalCycleComplete { continuation_id } => {
                if self.pending_internal_cycle != Some(continuation_id) {
                    return Err(SmsTemporalBusError::InvalidState);
                }
                let completion = SmsInternalCycleCompletion {
                    continuation_id,
                    completed_at: dispatch.event.timestamp,
                };
                self.internal_completion = Some(completion);
                Ok(Some(SmsTemporalDispatch::InternalCycleComplete(completion)))
            }
            SmsTemporalEvent::M1TStateComplete { continuation_id } => {
                if self.pending_request.is_some()
                    || self.pending_internal_cycle.is_some()
                    || self.completion.is_some()
                    || self.internal_completion.is_some()
                {
                    return Err(SmsTemporalBusError::InvalidState);
                }
                Ok(Some(SmsTemporalDispatch::M1TStateComplete {
                    continuation_id,
                    completed_at: dispatch.event.timestamp,
                }))
            }
        }
    }

    /// Consume the latched response exactly once after the CPU accepts it.
    pub fn acknowledge_completion(
        &mut self,
        request_id: u64,
    ) -> Result<SmsBusCompletion, SmsTemporalBusError> {
        let completion = self
            .completion
            .ok_or(SmsTemporalBusError::NoPendingRequest)?;
        if completion.request_id != request_id {
            return Err(SmsTemporalBusError::WrongRequestId {
                expected: completion.request_id,
                received: request_id,
            });
        }
        self.completion = None;
        self.pending_request = None;
        self.request_started_at = None;
        Ok(completion)
    }

    /// Acknowledge an internal interval only after the CPU continuation commits.
    pub fn acknowledge_internal_completion(
        &mut self,
        continuation_id: u64,
    ) -> Result<SmsInternalCycleCompletion, SmsTemporalBusError> {
        let completion = self
            .internal_completion
            .ok_or(SmsTemporalBusError::NoPendingRequest)?;
        if completion.continuation_id != continuation_id
            || self.pending_internal_cycle != Some(continuation_id)
        {
            return Err(SmsTemporalBusError::InvalidState);
        }
        self.internal_completion = None;
        self.pending_internal_cycle = None;
        self.internal_cycle_started_at = None;
        Ok(completion)
    }

    /// Return the currently outstanding CPU request, if any.
    pub const fn pending_request(&self) -> Option<Z80BusRequest> {
        self.pending_request
    }

    /// Timestamp at which the outstanding bus request was scheduled.
    pub const fn request_started_at(&self) -> Option<MachineTime> {
        self.request_started_at
    }

    /// Timestamp at which the outstanding internal interval was scheduled.
    pub const fn internal_cycle_started_at(&self) -> Option<MachineTime> {
        self.internal_cycle_started_at
    }

    /// Return the latched bus completion, if dispatch has reached its deadline.
    pub const fn completion(&self) -> Option<SmsBusCompletion> {
        self.completion
    }

    /// Return the current machine timestamp.
    pub const fn now(&self) -> MachineTime {
        self.coordinator.now()
    }

    pub(crate) fn coordinator_mut(&mut self) -> &mut SmsTemporalCoordinator {
        &mut self.coordinator
    }

    #[cfg(test)]
    pub(crate) fn write_memory_for_test(&mut self, address: u16, value: u8) {
        self.memory.write(address, value);
    }

    pub fn io(&self) -> &SmsIo {
        &self.io
    }

    pub fn io_mut(&mut self) -> &mut SmsIo {
        &mut self.io
    }

    /// Update controller levels without advancing scheduler time.
    pub fn set_controller(
        &mut self,
        player: u8,
        buttons: ControllerButtons,
    ) -> Result<(), SmsTemporalBusError> {
        self.io.set_controller(player, buttons)?;
        Ok(())
    }

    /// Set the byte driven by the external interrupting device during acknowledge.
    /// This changes the input level only; it does not advance scheduler time.
    pub fn set_interrupt_vector(&mut self, vector: u8) {
        self.interrupt_vector = vector;
    }

    /// Inspect the integrated SMS-profile PSG without advancing machine time.
    pub fn psg(&self) -> &Sn76489Core {
        self.psg.core()
    }

    /// Current normalized digital PSG mix before the production FIR.
    pub fn psg_mixed_amplitude(&self) -> f64 {
        self.psg.core().mixed_amplitude()
    }

    /// Enable/disable host-bound FM PCM capture without stopping YM2413 clocks.
    ///
    /// Disabling capture drops any partial rotation and queued samples. Re-enabling
    /// waits for slot zero so the first emitted sample always covers a full rotation.
    pub fn set_fm_audio_capture_enabled(&mut self, enabled: bool) {
        if self.fm_audio_capture_enabled == enabled {
            return;
        }
        self.fm_audio_capture_enabled = enabled;
        self.pending_fm_samples.clear();
        self.ym2413_pcm_accumulator = SmsYm2413PcmAccumulatorState::default();
        self.fm_capture_wait_for_rotation = enabled
            && self
                .ym2413
                .as_ref()
                .is_some_and(|core| core.slot_index() != 0);
    }

    /// Enable diagnostic wall-clock measurement for YM2413 advancement.
    ///
    /// Timing is opt-in and never feeds back into emulated clocks or scheduler state.
    pub fn set_ym2413_timing_enabled(&mut self, enabled: bool) {
        self.ym2413_timing_enabled = enabled;
        self.ym2413_advance_elapsed_ns = 0;
    }

    /// Take the YM2413 advance time accumulated since the previous reset.
    pub fn take_ym2413_advance_elapsed_ns(&mut self) -> u64 {
        std::mem::take(&mut self.ym2413_advance_elapsed_ns).min(u128::from(u64::MAX)) as u64
    }

    fn advance_ym2413(&mut self, clocks: u128) -> Result<(), SmsTemporalBusError> {
        if !self.ym2413_timing_enabled {
            return self.advance_ym2413_inner(clocks);
        }
        let started = Instant::now();
        let result = self.advance_ym2413_inner(clocks);
        self.ym2413_advance_elapsed_ns = self
            .ym2413_advance_elapsed_ns
            .saturating_add(started.elapsed().as_nanos());
        result
    }

    fn advance_ym2413_inner(&mut self, clocks: u128) -> Result<(), SmsTemporalBusError> {
        let mut remaining = clocks;
        let mut phase = self
            .ym2413
            .as_ref()
            .ok_or(SmsTemporalBusError::InvalidState)?
            .input_clock_phase();
        while remaining > 0 {
            if u64::from(phase) >= YM2413_INPUT_CLOCKS_PER_SLOT {
                return Err(SmsTemporalBusError::InvalidState);
            }
            let clocks_to_slot = YM2413_INPUT_CLOCKS_PER_SLOT - u64::from(phase);
            let step = remaining.min(u128::from(clocks_to_slot)) as u64;
            let completed_slot = step == clocks_to_slot;
            let advance_result = {
                let core = self
                    .ym2413
                    .as_mut()
                    .ok_or(SmsTemporalBusError::InvalidState)?;
                let service = self
                    .ym2413_pg_service
                    .as_mut()
                    .ok_or(SmsTemporalBusError::InvalidState)?;
                core.advance_input_clocks_with_pg_service_in_place(step, service)
            };
            if let Err(error) = advance_result {
                self.ym2413_runtime_faulted = true;
                return Err(error.into());
            }
            let capture_output =
                self.fm_audio_capture_enabled && !self.fm_capture_wait_for_rotation;
            let output_frame = if completed_slot && capture_output {
                self.ym2413_pg_service
                    .as_ref()
                    .and_then(Ym2413PgSerialService::output_frame)
            } else {
                None
            };
            phase = ((u64::from(phase) + step) % YM2413_INPUT_CLOCKS_PER_SLOT) as u8;
            remaining -= u128::from(step);
            if completed_slot && self.fm_audio_capture_enabled {
                if self.fm_capture_wait_for_rotation {
                    let next_slot = self
                        .ym2413
                        .as_ref()
                        .map(Ym2413Core::slot_index)
                        .ok_or(SmsTemporalBusError::InvalidState)?;
                    if next_slot == 0 {
                        self.fm_capture_wait_for_rotation = false;
                        self.ym2413_pcm_accumulator = SmsYm2413PcmAccumulatorState::default();
                    }
                } else {
                    let Some(frame) = output_frame else {
                        self.ym2413_runtime_faulted = true;
                        return Err(SmsTemporalBusError::InvalidState);
                    };
                    if let Err(error) = self.capture_ym2413_output_frame(frame) {
                        self.ym2413_runtime_faulted = true;
                        return Err(error);
                    }
                }
            }
        }
        Ok(())
    }

    fn capture_ym2413_output_frame(
        &mut self,
        frame: Ym2413CoreOutputPipelineFrame,
    ) -> Result<(), SmsTemporalBusError> {
        let fm_enabled = self.fm_output_enabled();
        let mut completed_sample = None;
        {
            let accumulator = &mut self.ym2413_pcm_accumulator;
            if !accumulator.is_valid()
                || frame.cycle != accumulator.next_cycle
                || accumulator.frame_count >= YM2413_SLOTS_PER_ROTATION as u8
            {
                return Err(SmsTemporalBusError::InvalidState);
            }

            // The F2 FM gate is applied at each completed serial slot. Muted slots
            // contribute known zero; an enabled slot with unknown MO/RO stays unknown.
            if fm_enabled {
                match (frame.mo, frame.ro) {
                    (Some(mo), Some(ro)) => {
                        accumulator.sum_mo += i64::from(mo.value());
                        accumulator.sum_ro += i64::from(ro.value());
                    }
                    _ => accumulator.valid = false,
                }
            }

            accumulator.frame_count += 1;
            accumulator.next_cycle = (frame.cycle + 1) % YM2413_SLOTS_PER_ROTATION as u8;
            if frame.cycle == (YM2413_SLOTS_PER_ROTATION - 1) as u8 {
                completed_sample = Some(accumulator.valid.then(|| {
                    (accumulator.sum_mo + accumulator.sum_ro) as f64
                        / (2.0 * YM2413_SLOTS_PER_ROTATION as f64 * 256.0)
                }));
                *accumulator = SmsYm2413PcmAccumulatorState::default();
            }
        }
        if let Some(sample) = completed_sample {
            self.pending_fm_samples.push_back(sample);
        }
        Ok(())
    }

    /// Drain complete YM2413 mono PCM samples at the SMS YM2413 source rate.
    /// None marks a rotation whose software-reference MO/RO output was unknown.
    /// Drain FM samples into a caller-owned buffer without allocating per frame.
    pub fn drain_fm_audio_samples_into(
        &mut self,
        output: &mut Vec<Option<f64>>,
        max_samples: Option<usize>,
    ) {
        output.clear();
        let count = max_samples
            .unwrap_or(self.pending_fm_samples.len())
            .min(self.pending_fm_samples.len());
        if output.capacity() < count {
            output.reserve(count);
        }
        output.extend(self.pending_fm_samples.drain(..count));
    }

    pub fn drain_fm_audio_samples(&mut self, max_samples: Option<usize>) -> Vec<Option<f64>> {
        let mut samples = Vec::new();
        self.drain_fm_audio_samples_into(&mut samples, max_samples);
        samples
    }

    /// Discard host-bound FM PCM without allocating; the YM2413 keeps clocking.
    pub fn discard_fm_audio_samples(&mut self) {
        self.pending_fm_samples.clear();
    }

    /// Drain PSG PCM captured at scheduler dispatch boundaries with the F2 gate applied.
    /// Drain PSG samples into a reusable caller-owned buffer.
    pub fn drain_audio_samples_into(&mut self, output: &mut Vec<f64>, max_samples: Option<usize>) {
        output.clear();
        if self.ym2413.is_none() {
            self.psg.core_mut().drain_samples_into(output, max_samples);
            return;
        }
        let count = max_samples
            .unwrap_or(self.pending_psg_samples.len())
            .min(self.pending_psg_samples.len());
        output.reserve(count.saturating_sub(output.capacity()));
        output.extend(self.pending_psg_samples.drain(..count));
    }

    pub fn drain_audio_samples(&mut self, max_samples: Option<usize>) -> Vec<f64> {
        let mut samples = Vec::new();
        self.drain_audio_samples_into(&mut samples, max_samples);
        samples
    }

    /// Return the installed Japanese Master System YM2413, if present.
    pub fn ym2413(&self) -> Option<&Ym2413Core> {
        self.ym2413.as_ref()
    }

    /// Snapshot diagnostic counters for completed YM2413 port writes.
    pub const fn fm_port_write_snapshot(&self) -> SmsFmPortWriteSnapshot {
        self.fm_port_writes
    }

    /// Whether the FM output path is enabled by the Japanese SMS audio-control port.
    pub const fn fm_output_enabled(&self) -> bool {
        self.ym2413.is_some() && self.fm_control & 0x01 != 0
    }

    /// Whether the PSG output path is enabled by the Japanese SMS audio-control mode.
    ///
    /// The two-bit mode is 00 = PSG only, 01 = FM only, 10 = both muted,
    /// and 11 = FM plus PSG. The bit pair is not two independent enable bits.
    pub const fn psg_output_enabled(&self) -> bool {
        self.ym2413.is_none() || (self.fm_control & 0x01) == ((self.fm_control >> 1) & 0x01)
    }

    /// Enable bounded observation of the existing PSG's digital mix transitions.
    pub fn enable_psg_event_capture(
        &mut self,
        capacity: usize,
    ) -> Result<(), PsgEventCaptureError> {
        self.psg.core_mut().enable_event_capture(capacity)
    }

    /// Disable event observation without changing the digital PSG state.
    pub fn disable_psg_event_capture(&mut self) {
        self.psg.core_mut().disable_event_capture();
    }

    /// Drain real PSG amplitude transitions into a reusable worker-owned buffer.
    pub fn drain_psg_amplitude_events(&mut self, output: &mut Vec<PsgAmplitudeEvent>) -> usize {
        self.psg.core_mut().drain_amplitude_events(output)
    }

    /// Report whether bounded PSG event observation overflowed.
    pub fn psg_event_capture_overflowed(&self) -> bool {
        self.psg.core().event_capture_overflowed()
    }

    /// Report the number of PSG transitions lost after event queue overflow.
    pub fn psg_event_capture_overflow_count(&self) -> u64 {
        self.psg.core().event_capture_overflow_count()
    }

    fn read_port(&mut self, port: u16) -> u8 {
        if port as u8 == 0xF2 && self.ym2413.is_some() {
            // D3-D0 read back the last written nibble. D7-D5 remain zero until
            // the temporal machine exposes the physical C-Sync input; D4 is
            // reserved and always reads zero.
            return self.fm_control & 0x0F;
        }
        match (port as u8) & 0xC1 {
            0x40 => self.vdp.read_v_counter(),
            0x41 => self.vdp.read_h_counter(),
            0x80 | 0x81 => self.vdp.read_port(port),
            _ => self.io.read_port(port),
        }
    }

    fn write_port(&mut self, port: u16, value: u8) {
        let port_byte = port as u8;
        if self.ym2413.is_some() {
            match port_byte {
                0xF0 => {
                    self.fm_port_writes.address_writes =
                        self.fm_port_writes.address_writes.saturating_add(1);
                    self.fm_port_writes.last_address = Some(value & 0x3F);
                    if let Some(ym2413) = self.ym2413.as_mut() {
                        ym2413.write_address(value);
                    }
                    return;
                }
                0xF1 => {
                    self.fm_port_writes.data_writes =
                        self.fm_port_writes.data_writes.saturating_add(1);
                    if let Some(register) = self.fm_port_writes.last_address {
                        let index = usize::from(register);
                        self.fm_port_writes.register_writes[index] =
                            self.fm_port_writes.register_writes[index].saturating_add(1);
                        self.fm_port_writes.last_register_values[index] = Some(value);
                    }
                    if let Some(ym2413) = self.ym2413.as_mut() {
                        ym2413.write_data(value);
                    }
                    return;
                }
                0xF2 => {
                    self.fm_port_writes.control_writes =
                        self.fm_port_writes.control_writes.saturating_add(1);
                    self.fm_port_writes.last_control = Some(value & 0x0F);
                    self.fm_control = value & 0x0F;
                    return;
                }
                _ => {}
            }
        }
        match port_byte & 0xC1 {
            0x80 | 0x81 => self.vdp.write_port(port, value),
            _ if (port_byte & 0xC0) == 0x40 => self.psg.write_at_dispatch(value),
            _ if port_byte <= 0x3F && port_byte & 1 == 0 => {
                self.memory.write_control_port(value);
            }
            _ if port_byte <= 0x3F && port_byte & 1 == 1 => {
                // The scheduler has advanced the VDP to this bus-completion
                // boundary before the I/O side effect is applied. A TH rising
                // edge latches H-counter only when the resulting pin is input.
                let previous_control = self.io.io_control();
                self.io.write_port(port, value);
                let io_control = self.io.io_control();
                let th_levels = self.io.read_port(0xDD);

                // Match the SMS I/O-control condition: the TH pin must have
                // been driven low (direction + output level both zero), then
                // become an input whose external level is high. Do not infer
                // the previous electrical level from port readback: Japanese
                // SMS intentionally hides driven TH output levels on $DD.
                let th1_rising =
                    io_control & 0x02 != 0 && previous_control & 0x22 == 0 && th_levels & 0x40 != 0;
                let th2_rising =
                    io_control & 0x08 != 0 && previous_control & 0x88 == 0 && th_levels & 0x80 != 0;
                if th1_rising || th2_rising {
                    self.vdp.latch_h_counter();
                }
            }
            _ => self.io.write_port(port, value),
        }
    }

    /// Read-only access to the mapper-backed memory image.
    pub const fn memory(&self) -> &SmsMemory {
        &self.memory
    }

    /// Load persistent cartridge SRAM before execution without advancing machine time.
    pub fn load_cartridge_sram(&mut self, contents: &[u8]) -> Result<(), crate::SmsCoreError> {
        self.memory.load_cartridge_sram(contents)
    }

    /// Read-only access to the SMS1 VDP connected to the temporal bus.
    pub const fn vdp(&self) -> &SmsVdp {
        &self.vdp
    }

    /// Mutable VDP access for machine-owned configuration commands.
    ///
    /// Callers must hold exclusive ownership of the temporal machine worker.
    pub fn vdp_mut(&mut self) -> &mut SmsVdp {
        &mut self.vdp
    }

    /// Export memory, scheduler, and in-flight response state.
    pub fn export_state(&self) -> SmsTemporalBusState {
        SmsTemporalBusState {
            format_version: if self.ym2413_runtime_faulted {
                0
            } else {
                SMS_TEMPORAL_BUS_STATE_VERSION
            },
            interrupt_vector: self.interrupt_vector,
            memory: self.memory.clone(),
            io: self.io.clone(),
            psg: self.psg.clone(),
            pending_psg_samples: self.pending_psg_samples.clone(),
            ym2413: self.ym2413.as_ref().map(Ym2413Core::export_state),
            ym2413_pg_service: self
                .ym2413_pg_service
                .as_ref()
                .map(Ym2413PgSerialService::export_state),
            pending_fm_samples: self.pending_fm_samples.clone(),
            ym2413_pcm_accumulator: self.ym2413_pcm_accumulator,
            fm_control: self.fm_control,
            vdp: self.vdp.clone(),
            coordinator: self.coordinator.export_state(),
            pending_request: self.pending_request,
            request_started_at: self.request_started_at,
            completion: self.completion,
            pending_internal_cycle: self.pending_internal_cycle,
            internal_cycle_started_at: self.internal_cycle_started_at,
            internal_completion: self.internal_completion,
        }
    }

    /// Restore only when the scheduler event and bus transaction agree.
    pub fn restore_state(state: SmsTemporalBusState) -> Result<Self, SmsTemporalBusError> {
        if state.format_version != SMS_TEMPORAL_BUS_STATE_VERSION {
            return Err(SmsTemporalBusError::InvalidState);
        }
        if state.fm_control & !0x0F != 0
            || (state.ym2413.is_none() && state.fm_control != 0)
            || state.ym2413.is_some() != state.ym2413_pg_service.is_some()
            || !state.ym2413_pcm_accumulator.is_valid()
            || state
                .pending_fm_samples
                .iter()
                .flatten()
                .any(|sample| !sample.is_finite())
            || state
                .pending_psg_samples
                .iter()
                .any(|sample| !sample.is_finite())
            || (state.ym2413.is_none()
                && (!state.pending_psg_samples.is_empty()
                    || !state.pending_fm_samples.is_empty()
                    || state.ym2413_pcm_accumulator != SmsYm2413PcmAccumulatorState::default()))
            || state.psg.clock_domain() != SMS_PSG_CLOCK_DOMAIN_ID
            || state.psg.core().profile() != PsgProfile::Sms
            || state.psg.core().clock_hz() == 0
            || state.psg.core().sample_rate() == 0
        {
            return Err(SmsTemporalBusError::InvalidState);
        }
        let coordinator = SmsTemporalCoordinator::restore_state(state.coordinator.clone())?;
        let ym2413_domain_present = state
            .coordinator
            .scheduler
            .clock_domains
            .iter()
            .any(|domain| domain.id == crate::SMS_YM2413_CLOCK_DOMAIN_ID);
        if ym2413_domain_present != state.ym2413.is_some() {
            return Err(SmsTemporalBusError::InvalidState);
        }
        if let (Some(core_state), Some(service_state)) = (&state.ym2413, &state.ym2413_pg_service)
            && (core_state.slot_index != service_state.rhythm_noise_cursor
                || core_state.slot_index != service_state.output_pipeline.next_cycle
                || (state.ym2413_pcm_accumulator != SmsYm2413PcmAccumulatorState::default()
                    && core_state.slot_index != state.ym2413_pcm_accumulator.next_cycle)
                || u64::from(core_state.input_clock_phase) >= YM2413_INPUT_CLOCKS_PER_SLOT)
        {
            return Err(SmsTemporalBusError::InvalidState);
        }
        let fm_capture_wait_for_rotation = state.ym2413.as_ref().is_some_and(|core_state| {
            core_state.slot_index != 0
                && state.ym2413_pcm_accumulator == SmsYm2413PcmAccumulatorState::default()
        });
        let ym2413 = state.ym2413.map(Ym2413Core::restore_state).transpose()?;
        let ym2413_pg_service = state
            .ym2413_pg_service
            .map(Ym2413PgSerialService::restore_state)
            .transpose()?;
        let pending_events = &state.coordinator.scheduler.event_queue.pending;
        let scale = state.coordinator.master_ticks_per_tstate;
        let now = coordinator.now();
        let last_dispatch = state.coordinator.scheduler.last_dispatch_timestamp;
        let external_events_valid = pending_events.iter().all(|event| match event.payload {
            SmsTemporalEvent::ExternalThTransition { player, .. } => {
                (player == 1 || player == 2)
                    && event.hardware_phase == HardwarePhase::new(1)
                    && event
                        .timestamp
                        .checked_duration_since(now)
                        .is_some_and(|elapsed| elapsed % u128::from(scale) == 0)
            }
            _ => true,
        });
        let non_external_event_count = pending_events
            .iter()
            .filter(|event| !matches!(event.payload, SmsTemporalEvent::ExternalThTransition { .. }))
            .count();
        match (
            state.pending_request,
            state.request_started_at,
            state.completion,
            state.pending_internal_cycle,
            state.internal_cycle_started_at,
            state.internal_completion,
        ) {
            (None, None, None, None, None, None)
                if external_events_valid
                    && (non_external_event_count == 0
                        || (non_external_event_count == 1
                            && pending_events.iter().any(|event| {
                                matches!(event.payload, SmsTemporalEvent::M1TStateComplete { .. })
                            }))) => {}
            (Some(request), Some(started_at), None, None, None, None)
                if external_events_valid
                    && non_external_event_count == 1
                    && started_at <= now
                    && pending_events.iter().any(|event| {
                        event.payload
                            == (SmsTemporalEvent::BusCompletion {
                                request_id: request.id,
                            })
                            && event.timestamp >= now
                            && valid_bus_deadline(
                                started_at,
                                event.timestamp,
                                scale,
                                u16::from(request.base_tstates)
                                    + if request.operation == Z80BusOperation::InterruptAcknowledge
                                    {
                                        u16::from(Z80_INT_ACK_AUTOMATIC_WAIT_TSTATES)
                                    } else {
                                        0
                                    },
                            )
                    }) => {}
            (Some(request), Some(started_at), Some(completion), None, None, None)
                if external_events_valid
                    && request.id == completion.request_id
                    && non_external_event_count == 0
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
                if external_events_valid
                    && non_external_event_count == 1
                    && started_at <= now
                    && pending_events.iter().any(|event| {
                        event.payload
                            == (SmsTemporalEvent::InternalCycleComplete { continuation_id })
                            && event.timestamp >= now
                            && valid_internal_deadline(started_at, event.timestamp, scale)
                    }) => {}
            (None, None, None, Some(continuation_id), Some(started_at), Some(completion))
                if external_events_valid
                    && continuation_id == completion.continuation_id
                    && non_external_event_count == 0
                    && completion.completed_at == now
                    && last_dispatch == Some(completion.completed_at)
                    && valid_internal_deadline(started_at, completion.completed_at, scale) => {}
            _ => return Err(SmsTemporalBusError::InvalidState),
        }
        Ok(Self {
            memory: state.memory,
            io: state.io,
            psg: state.psg,
            pending_psg_samples: state.pending_psg_samples,
            ym2413,
            ym2413_pg_service,
            pending_fm_samples: state.pending_fm_samples,
            ym2413_pcm_accumulator: state.ym2413_pcm_accumulator,
            fm_control: state.fm_control,
            vdp: state.vdp,
            coordinator,
            pending_request: state.pending_request,
            request_started_at: state.request_started_at,
            completion: state.completion,
            pending_internal_cycle: state.pending_internal_cycle,
            internal_cycle_started_at: state.internal_cycle_started_at,
            internal_completion: state.internal_completion,
            interrupt_vector: state.interrupt_vector,
            ym2413_runtime_faulted: false,
            fm_audio_capture_enabled: true,
            fm_capture_wait_for_rotation,
            ym2413_timing_enabled: false,
            ym2413_advance_elapsed_ns: 0,
            fm_port_writes: SmsFmPortWriteSnapshot::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_SMS_PSG_CLOCK_HZ, PsgProfile, SmsTemporalBus, SmsTemporalBusError,
        SmsTemporalDispatch,
    };
    use crate::Territory;
    use sega_temporal_core::MachineTime;
    use sega_video::sms::{SmsVdp, SmsVdpProfile};
    use sega_z80_cpu::{Z80BusOperation, Z80BusRequest};

    fn request(id: u64, operation: Z80BusOperation, address: u16, tstates: u8) -> Z80BusRequest {
        Z80BusRequest {
            id,
            operation,
            address,
            base_tstates: tstates,
        }
    }

    fn advance_internal_cycle(bus: &mut SmsTemporalBus, id: u64, tstates: u32) {
        bus.schedule_internal_cycle(id, tstates).unwrap();
        assert!(matches!(
            bus.dispatch_next().unwrap(),
            Some(SmsTemporalDispatch::InternalCycleComplete(_))
        ));
        bus.acknowledge_internal_completion(id).unwrap();
    }

    fn patterned_rom(bank_count: usize) -> Vec<u8> {
        let mut rom = vec![0; bank_count * 0x4000];
        for bank in 0..bank_count {
            let start = bank * 0x4000;
            rom[start..start + 0x4000].fill(bank as u8);
        }
        rom
    }

    fn vdp_write_register(vdp: &mut SmsVdp, index: u8, value: u8) {
        vdp.write_port(0xBF, value);
        vdp.write_port(0xBF, 0x80 | index);
    }

    fn vdp_write_memory(vdp: &mut SmsVdp, address: u16, value: u8) {
        vdp.write_port(0xBF, address as u8);
        vdp.write_port(0xBF, ((address >> 8) as u8 & 0x3F) | 0x40);
        vdp.write_port(0xBE, value);
    }

    fn vdp_write_cram(vdp: &mut SmsVdp, address: u16, value: u8) {
        vdp.write_port(0xBF, address as u8);
        vdp.write_port(0xBF, ((address >> 8) as u8 & 0x3F) | 0xC0);
        vdp.write_port(0xBE, value);
    }

    #[test]
    fn japanese_fm_profile_routes_register_ports_control_and_clock_state() {
        let mut bus = SmsTemporalBus::new_with_japanese_fm_hardware_profile(
            vec![0; 0x4000],
            3,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            48_000,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Japan,
        )
        .unwrap();

        assert!(bus.ym2413().is_some());
        assert!(!bus.fm_output_enabled());
        assert!(bus.psg_output_enabled());

        bus.write_port(0xF0, 0x10);
        bus.schedule_internal_cycle(90, 12).unwrap();
        assert!(matches!(
            bus.dispatch_next().unwrap(),
            Some(SmsTemporalDispatch::InternalCycleComplete(_))
        ));
        bus.acknowledge_internal_completion(90).unwrap();

        bus.write_port(0xF1, 0xA5);
        bus.schedule_internal_cycle(91, 72).unwrap();
        assert!(matches!(
            bus.dispatch_next().unwrap(),
            Some(SmsTemporalDispatch::InternalCycleComplete(_))
        ));
        assert_eq!(
            bus.ym2413().unwrap().export_state().elapsed_input_clocks,
            84
        );
        assert_eq!(bus.ym2413().unwrap().export_state().registers[0x10], 0xA5);
        bus.acknowledge_internal_completion(91).unwrap();

        // D3-D2 are readable latch bits but do not affect the two routing
        // controls in D1-D0. Exercise all four output modes with D3-D2 set.
        for (value, fm_enabled, psg_enabled) in [
            (0x0C, false, true),
            (0x0D, true, false),
            (0x0E, false, false),
            (0x0F, true, true),
        ] {
            bus.write_port(0xF2, value);
            assert_eq!(bus.read_port(0xF2), value);
            assert_eq!(bus.fm_output_enabled(), fm_enabled);
            assert_eq!(bus.psg_output_enabled(), psg_enabled);
        }

        // The write port does not latch C-Sync/reserved upper bits.
        bus.write_port(0xF2, 0xFD);
        assert_eq!(bus.read_port(0xF2), 0x0D);
        assert!(bus.fm_output_enabled());
        assert!(!bus.psg_output_enabled());

        let state = bus.export_state();
        let mut restored = SmsTemporalBus::restore_state(state).unwrap();
        assert_eq!(
            restored.ym2413().unwrap().export_state(),
            bus.ym2413().unwrap().export_state()
        );
        assert_eq!(restored.read_port(0xF2), 0x0D);
    }

    #[test]
    fn japanese_fm_host_capture_resumes_only_after_a_complete_slot_rotation() {
        let mut bus = SmsTemporalBus::new_with_japanese_fm_hardware_profile(
            vec![0; 0x4000],
            1,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            48_000,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Japan,
        )
        .unwrap();
        bus.write_port(0xF2, 0x01);

        bus.set_fm_audio_capture_enabled(false);
        advance_internal_cycle(&mut bus, 120, 84);
        assert!(bus.drain_fm_audio_samples(None).is_empty());

        bus.set_fm_audio_capture_enabled(true);
        advance_internal_cycle(&mut bus, 121, 60);
        assert!(
            bus.drain_fm_audio_samples(None).is_empty(),
            "capture must not resume with a partial slot rotation"
        );

        advance_internal_cycle(&mut bus, 122, 72);
        let samples = bus.drain_fm_audio_samples(None);
        assert_eq!(
            samples.len(),
            1,
            "capture resumes on the next full rotation"
        );
        assert!(samples.iter().flatten().all(|sample| sample.is_finite()));
    }

    #[test]
    fn japanese_fm_pcm_drain_into_reuses_buffers_without_changing_order() {
        let mut bus = SmsTemporalBus::new_with_japanese_fm_hardware_profile(
            vec![0; 0x4000],
            1,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            48_000,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Japan,
        )
        .unwrap();
        bus.write_port(0xF2, 0x01);
        advance_internal_cycle(&mut bus, 123, 216);

        let mut fm = Vec::with_capacity(16);
        fm.push(None);
        let fm_capacity = fm.capacity();
        bus.drain_fm_audio_samples_into(&mut fm, None);
        assert_eq!(fm.len(), 3);
        assert_eq!(fm.capacity(), fm_capacity);

        let mut psg = Vec::with_capacity(16);
        psg.push(123.0);
        let psg_capacity = psg.capacity();
        bus.drain_audio_samples_into(&mut psg, None);
        assert_eq!(psg.capacity(), psg_capacity);
        assert!(psg.iter().all(|sample| *sample != 123.0));
    }

    #[test]
    fn japanese_fm_serial_pcm_samples_restore_deterministically() {
        let mut uninterrupted = SmsTemporalBus::new_with_japanese_fm_hardware_profile(
            vec![0; 0x4000],
            1,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            48_000,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Japan,
        )
        .unwrap();
        uninterrupted.write_port(0xF2, 0x01);
        advance_internal_cycle(&mut uninterrupted, 100, 216);
        let expected = uninterrupted.drain_fm_audio_samples(None);
        assert_eq!(expected.len(), 3);
        assert!(expected.iter().any(Option::is_some));
        assert!(
            expected
                .iter()
                .flatten()
                .all(|sample| sample.is_finite() && sample.abs() <= 1.0)
        );

        let mut split = SmsTemporalBus::new_with_japanese_fm_hardware_profile(
            vec![0; 0x4000],
            1,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            48_000,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Japan,
        )
        .unwrap();
        split.write_port(0xF2, 0x01);
        advance_internal_cycle(&mut split, 101, 84);
        let snapshot = split.export_state();
        let mut restored = SmsTemporalBus::restore_state(snapshot).unwrap();
        advance_internal_cycle(&mut restored, 102, 132);

        assert_eq!(restored.drain_fm_audio_samples(None), expected);
    }

    #[test]
    fn japanese_fm_profile_applies_psg_gate_at_scheduler_boundaries() {
        let mut bus = SmsTemporalBus::new_with_japanese_fm_hardware_profile(
            vec![0; 0x4000],
            1,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            48_000,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Japan,
        )
        .unwrap();
        bus.write_port(0x7F, 0x80);
        bus.write_port(0x7F, 0x04);
        bus.write_port(0x7F, 0x90);

        bus.write_port(0xF2, 0x01);
        assert!(!bus.psg_output_enabled());
        advance_internal_cycle(&mut bus, 110, 12_000);
        let muted = bus.drain_audio_samples(None);
        assert!(!muted.is_empty());
        assert!(muted.iter().all(|sample| *sample == 0.0));

        bus.write_port(0xF2, 0x00);
        assert!(bus.psg_output_enabled());
        advance_internal_cycle(&mut bus, 111, 12_000);
        let audible = bus.drain_audio_samples(None);
        assert!(!audible.is_empty());
        assert!(audible.iter().any(|sample| sample.abs() > 1.0e-9));
    }

    #[test]
    fn japanese_fm_hardware_profile_is_independent_of_software_territory() {
        let bus = SmsTemporalBus::new_with_japanese_fm_hardware_profile(
            vec![0; 0x4000],
            3,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            48_000,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Export,
        )
        .unwrap();

        assert!(bus.ym2413().is_some());
        assert_eq!(bus.io.territory(), Territory::Export);

        let restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        assert!(restored.ym2413().is_some());
        assert_eq!(restored.io.territory(), Territory::Export);
    }

    #[test]
    fn sms1_offscreen_vertical_border_collision_crosses_scheduler_frame_boundary_and_restores() {
        const FRAME_TSTATES: u32 = 262 * 228;

        fn advance_internal(bus: &mut SmsTemporalBus, id: u64, tstates: u32) {
            bus.schedule_internal_cycle(id, tstates).unwrap();
            assert!(matches!(
                bus.dispatch_next().unwrap(),
                Some(SmsTemporalDispatch::InternalCycleComplete(_))
            ));
            bus.acknowledge_internal_completion(id).unwrap();
        }

        let mut initial = SmsTemporalBus::new_with_hardware_profile(
            vec![0; 0x4000],
            3,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            44_100,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Export,
        )
        .unwrap();

        {
            let vdp = initial.vdp_mut();
            vdp_write_register(vdp, 0, 0x04); // Mode 4.
            vdp_write_register(vdp, 1, 0x40); // Display enabled.
            vdp_write_register(vdp, 5, 0x7F); // SAT at $3F00.

            vdp_write_memory(vdp, 0x3F00, 0xF0); // Sprite 0 Y: top border.
            vdp_write_memory(vdp, 0x3F01, 0xF0); // Sprite 1 Y: top border.
            vdp_write_memory(vdp, 0x3F02, 0xD0); // SAT terminator.
            vdp_write_memory(vdp, 0x3F80, 0x7C); // Sprite 0 X.
            vdp_write_memory(vdp, 0x3F81, 0x01); // Sprite 0 tile.
            vdp_write_memory(vdp, 0x3F82, 0x7C); // Sprite 1 X.
            vdp_write_memory(vdp, 0x3F83, 0x02); // Sprite 1 tile.

            for row in 0..8_u16 {
                for plane in 0..4_u16 {
                    vdp_write_memory(vdp, 32 + row * 4 + plane, 0xFF);
                    vdp_write_memory(vdp, 64 + row * 4 + plane, 0xFF);
                }
            }
        }

        let mut continuous = initial.clone();
        advance_internal(&mut continuous, 1, FRAME_TSTATES);

        let mut chunked = SmsTemporalBus::restore_state(initial.export_state()).unwrap();
        advance_internal(&mut chunked, 1, FRAME_TSTATES - 1);
        let mut restored = SmsTemporalBus::restore_state(chunked.export_state()).unwrap();
        advance_internal(&mut restored, 2, 1);

        let read_status = |bus: &SmsTemporalBus| {
            let mut vdp = bus.vdp().clone();
            vdp.read_port(0xBF)
        };
        let continuous_status = read_status(&continuous);
        let restored_status = read_status(&restored);

        assert_ne!(
            continuous_status & 0x20,
            0,
            "SMS1 must latch collision for overlapping sprites at Y=$F0"
        );
        assert_eq!(restored_status, continuous_status);
        assert_eq!(restored.vdp().scanline(), continuous.vdp().scanline());
        assert_eq!(
            restored.vdp().cycles_into_line(),
            continuous.vdp().cycles_into_line()
        );
    }

    #[test]
    fn sms1_vint_boundary_is_chunk_invariant_and_survives_snapshot_restore() {
        const VBLANK_START_TSTATES: u32 = 193 * 228;

        fn advance_internal(bus: &mut SmsTemporalBus, id: u64, tstates: u32) {
            bus.schedule_internal_cycle(id, tstates).unwrap();
            assert!(matches!(
                bus.dispatch_next().unwrap(),
                Some(SmsTemporalDispatch::InternalCycleComplete(_))
            ));
            bus.acknowledge_internal_completion(id).unwrap();
        }

        let mut continuous = SmsTemporalBus::new_with_hardware_profile(
            vec![0; 0x4000],
            3,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            44_100,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Export,
        )
        .unwrap();
        vdp_write_register(continuous.vdp_mut(), 1, 0x20); // VINT enabled.
        let mut chunked = continuous.clone();

        advance_internal(&mut continuous, 1, VBLANK_START_TSTATES + 17);
        advance_internal(&mut chunked, 1, VBLANK_START_TSTATES + 12);
        assert!(!chunked.vdp().interrupt_pending());

        // Snapshot immediately before the T13 VINT boundary.
        let mut restored = SmsTemporalBus::restore_state(chunked.export_state()).unwrap();
        advance_internal(&mut restored, 2, 1);
        assert!(restored.vdp().interrupt_pending());
        let status_at_t13 = {
            let mut vdp = restored.vdp().clone();
            vdp.read_port(0xBF)
        };
        assert_eq!(status_at_t13 & 0x80, 0, "VBlank is not set at T13");

        advance_internal(&mut restored, 3, 3);
        let status_at_t16 = {
            let mut vdp = restored.vdp().clone();
            vdp.latch_h_counter();
            assert_eq!(vdp.read_h_counter(), 0xF4);
            vdp.read_port(0xBF)
        };
        assert_eq!(status_at_t16 & 0x80, 0, "HCount $F4 precedes VBlank status");

        advance_internal(&mut restored, 4, 1);
        let status_at_t17 = {
            let mut vdp = restored.vdp().clone();
            vdp.latch_h_counter();
            assert_eq!(vdp.read_h_counter(), 0xF5);
            vdp.read_port(0xBF)
        };
        let continuous_status = {
            let mut vdp = continuous.vdp().clone();
            vdp.read_port(0xBF)
        };

        assert_ne!(status_at_t17 & 0x80, 0);
        assert_eq!(status_at_t17, continuous_status);
        assert_eq!(
            restored.vdp().interrupt_pending(),
            continuous.vdp().interrupt_pending()
        );
        assert_eq!(restored.vdp().scanline(), continuous.vdp().scanline());
        assert_eq!(
            restored.vdp().cycles_into_line(),
            continuous.vdp().cycles_into_line()
        );
    }

    #[test]
    fn sms1_hint_r10_boundary_is_preserved_by_scheduler_snapshot_restore() {
        fn advance_internal(bus: &mut SmsTemporalBus, id: u64, tstates: u32) {
            bus.schedule_internal_cycle(id, tstates).unwrap();
            assert!(matches!(
                bus.dispatch_next().unwrap(),
                Some(SmsTemporalDispatch::InternalCycleComplete(_))
            ));
            bus.acknowledge_internal_completion(id).unwrap();
        }

        let mut bus = SmsTemporalBus::new_with_hardware_profile(
            vec![0; 0x4000],
            3,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            44_100,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Export,
        )
        .unwrap();
        vdp_write_register(bus.vdp_mut(), 0, 0x10); // HINT enabled.
        vdp_write_register(bus.vdp_mut(), 10, 0); // First scanline expires at T14.

        advance_internal(&mut bus, 1, 13);
        assert!(!bus.vdp().interrupt_pending());

        let mut restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        advance_internal(&mut restored, 2, 1);
        assert!(restored.vdp().interrupt_pending());
        assert_eq!(restored.vdp().cycles_into_line(), 14);
    }

    #[test]
    fn sms1_hint_r10_192_boundary_is_preserved_by_scheduler_snapshot_restore() {
        fn advance_internal(bus: &mut SmsTemporalBus, id: u64, tstates: u32) {
            bus.schedule_internal_cycle(id, tstates).unwrap();
            assert!(matches!(
                bus.dispatch_next().unwrap(),
                Some(SmsTemporalDispatch::InternalCycleComplete(_))
            ));
            bus.acknowledge_internal_completion(id).unwrap();
        }

        let mut bus = SmsTemporalBus::new_with_hardware_profile(
            vec![0; 0x4000],
            3,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            44_100,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Export,
        )
        .unwrap();
        vdp_write_register(bus.vdp_mut(), 0, 0x10); // HINT enabled.
        vdp_write_register(bus.vdp_mut(), 10, 192);

        // Complete one frame so the SMS1 frame-start path reloads R10 into
        // the line counter. Acknowledge the initial line-0 HINT/VBlank state.
        advance_internal(&mut bus, 1, 262 * 228);
        bus.vdp_mut().read_port(0xBF);

        // Stop one T-state before R10=192 expires on scanline 192.
        advance_internal(&mut bus, 2, 192 * 228 + 13);
        assert_eq!(bus.vdp().scanline(), 192);
        assert_eq!(bus.vdp().cycles_into_line(), 13);
        assert!(!bus.vdp().interrupt_pending());

        let mut restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        advance_internal(&mut restored, 3, 1);
        assert!(restored.vdp().interrupt_pending());
        assert_eq!(restored.vdp().scanline(), 192);
        assert_eq!(restored.vdp().cycles_into_line(), 14);
        assert_eq!(restored.vdp().status() & 0x80, 0);
    }

    #[test]
    fn external_th_transition_is_applied_at_scheduler_boundary_and_survives_restore() {
        let mut bus = SmsTemporalBus::new_with_hardware_profile(
            vec![0; 0x4000],
            3,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            44_100,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Export,
        )
        .unwrap();
        let timestamp = MachineTime::from_ticks(15);
        bus.schedule_external_th_transition(1, false, timestamp)
            .unwrap();

        let mut restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        assert_eq!(
            restored.dispatch_next().unwrap(),
            Some(SmsTemporalDispatch::ExternalThTransition {
                player: 1,
                high: false,
                completed_at: timestamp,
            })
        );
        assert_eq!(restored.io().external_th_level(1), Ok(false));
        assert_eq!(restored.io().read_port(0xDD) & (1 << 6), 0);
        assert_ne!(restored.io().read_port(0xDD) & (1 << 7), 0);
    }

    #[test]
    fn same_timestamp_external_th_transition_follows_port_read_completion() {
        let mut bus = SmsTemporalBus::new_with_hardware_profile(
            vec![0; 0x4000],
            3,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            44_100,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Export,
        )
        .unwrap();
        let port_read = request(1, Z80BusOperation::PortRead, 0x00DD, 4);
        let deadline = bus.begin_request(port_read, 0).unwrap();
        bus.schedule_external_th_transition(1, false, deadline)
            .unwrap();

        let mut restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        let Some(SmsTemporalDispatch::BusCompletion(completion)) =
            restored.dispatch_next().unwrap()
        else {
            panic!("phase-zero port read must complete before phase-one TH input");
        };
        assert_eq!(completion.data, Some(0xFF));

        // A snapshot taken after the bus sample but before the phase-one input
        // event must retain the same-timestamp external transition.
        let mut after_completion = SmsTemporalBus::restore_state(restored.export_state()).unwrap();
        after_completion
            .acknowledge_completion(port_read.id)
            .unwrap();

        assert_eq!(
            after_completion.dispatch_next().unwrap(),
            Some(SmsTemporalDispatch::ExternalThTransition {
                player: 1,
                high: false,
                completed_at: deadline,
            })
        );
        assert_eq!(after_completion.io().read_port(0xDD) & (1 << 6), 0);
    }

    #[test]
    fn sms1_sprite_line_buffer_survives_temporal_bus_snapshot_restore() {
        let mut bus = SmsTemporalBus::new_with_hardware_profile(
            vec![0; 0x4000],
            3,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            44_100,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Export,
        )
        .unwrap();

        {
            let vdp = bus.vdp_mut();
            vdp_write_register(vdp, 0, 0x04);
            vdp_write_register(vdp, 1, 0x40);
            vdp_write_register(vdp, 5, 0x7F);
            vdp_write_cram(vdp, 0x11, 0x03);
            vdp_write_memory(vdp, 0x0020, 0xFF);
            vdp_write_memory(vdp, 0x3F00, 0);
            vdp_write_memory(vdp, 0x3F01, 0xD0);
            vdp_write_memory(vdp, 0x3F80, 10);
            vdp_write_memory(vdp, 0x3F81, 1);
        }

        let first_line = request(1, Z80BusOperation::MemoryRead, 0, 228);
        bus.begin_request(first_line, 0).unwrap();
        assert!(matches!(
            bus.dispatch_next().unwrap(),
            Some(SmsTemporalDispatch::BusCompletion(_))
        ));
        bus.acknowledge_completion(first_line.id).unwrap();

        // The VDP has prepared the line-1 SAT selection. Snapshot it as part
        // of the bus continuation before changing the live SAT Y byte.
        let mut restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        for candidate in [&mut bus, &mut restored] {
            vdp_write_memory(candidate.vdp_mut(), 0x3F00, 20);
            let second_line = request(2, Z80BusOperation::MemoryRead, 0, 228);
            candidate.begin_request(second_line, 0).unwrap();
            assert!(matches!(
                candidate.dispatch_next().unwrap(),
                Some(SmsTemporalDispatch::BusCompletion(_))
            ));
            candidate.acknowledge_completion(second_line.id).unwrap();
        }

        assert_eq!(
            bus.vdp(),
            restored.vdp(),
            "the pending sprite selection must be part of SmsTemporalBusState"
        );
    }

    #[test]
    fn sms1_partial_cram_raster_span_survives_temporal_bus_snapshot_restore() {
        let mut bus = SmsTemporalBus::new_with_hardware_profile(
            vec![0; 0x4000],
            3,
            DEFAULT_SMS_PSG_CLOCK_HZ,
            44_100,
            SmsVdpProfile::sms1_t0_ntsc(),
            Territory::Export,
        )
        .unwrap();

        {
            let vdp = bus.vdp_mut();
            vdp_write_register(vdp, 0, 0x04);
            vdp_write_register(vdp, 1, 0x40);
            vdp_write_register(vdp, 2, 0x0E);
            vdp_write_register(vdp, 5, 0x7F);
            vdp_write_cram(vdp, 1, 0x03);
            for row in 0..8_u16 {
                vdp_write_memory(vdp, row * 4, 0xFF);
            }
            vdp_write_memory(vdp, 0x3F00, 0xD0);
        }

        let first_half = request(1, Z80BusOperation::MemoryRead, 0, 114);
        bus.begin_request(first_half, 0).unwrap();
        assert!(matches!(
            bus.dispatch_next().unwrap(),
            Some(SmsTemporalDispatch::BusCompletion(_))
        ));
        bus.acknowledge_completion(first_half.id).unwrap();

        // The VDP has committed the prefix with the old CRAM value. Snapshot
        // the raster cursor and pending line state before changing the suffix.
        vdp_write_cram(bus.vdp_mut(), 1, 0x0C);
        let mut restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();

        for candidate in [&mut bus, &mut restored] {
            let second_half = request(2, Z80BusOperation::MemoryRead, 0, 114);
            candidate.begin_request(second_half, 0).unwrap();
            assert!(matches!(
                candidate.dispatch_next().unwrap(),
                Some(SmsTemporalDispatch::BusCompletion(_))
            ));
            candidate.acknowledge_completion(second_half.id).unwrap();
        }

        assert_eq!(
            bus.vdp(),
            restored.vdp(),
            "raster cursor and already-committed framebuffer prefix must survive the bus snapshot"
        );
    }

    #[test]
    fn sega_mapper_writes_take_effect_at_scheduler_deadline_and_reads_see_selected_bank() {
        let mut bus = SmsTemporalBus::new(patterned_rom(4), 3).unwrap();
        let cases = [
            (10, 0xFFFD, 0x0400, 0, 0xDFFD),
            (11, 0xFFFE, 0x4000, 1, 0xDFFE),
            (12, 0xFFFF, 0x8000, 2, 0xDFFF),
        ];

        for (request_id, register, banked_address, slot, ram_mirror) in cases {
            let before = bus.memory().selected_banks();
            assert_eq!(bus.memory().read(banked_address), before[slot]);

            let write_request = request(
                request_id,
                Z80BusOperation::MemoryWrite { value: 3 },
                register,
                3,
            );
            let started_at = bus.now();
            let deadline = bus.begin_request(write_request, 2).unwrap();
            assert_eq!(
                deadline.checked_duration_since(started_at),
                Some(15),
                "three nominal T-states plus two fixture WAIT T-states at three master ticks each"
            );

            // Snapshot/restore before dispatch must not expose the mapper write.
            let mut restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
            assert_eq!(restored.memory().selected_banks(), before);
            assert_eq!(restored.memory().read(banked_address), before[slot]);
            assert_eq!(
                restored.memory().read(ram_mirror),
                0,
                "the mirrored work-RAM byte must remain unchanged until the mapper write deadline"
            );

            let Some(SmsTemporalDispatch::BusCompletion(completion)) =
                restored.dispatch_next().unwrap()
            else {
                panic!("expected mapper-register write completion");
            };
            assert_eq!(completion.request_id, request_id);
            assert_eq!(completion.completed_at, deadline);
            assert_eq!(restored.memory().selected_banks()[slot], 3);
            assert_eq!(
                restored.memory().mapper_register_write_count_for_test(),
                request_id - 9,
                "each Sega mapper register write is applied exactly once"
            );
            assert_eq!(restored.memory().read(ram_mirror), 3);
            assert_eq!(
                restored.memory().read(0x03FF),
                0,
                "the Sega mapper's first 1 KiB remains fixed to bank zero"
            );
            assert_eq!(restored.memory().read(banked_address), 3);

            restored.acknowledge_completion(request_id).unwrap();
            let read_request = request(
                request_id + 100,
                Z80BusOperation::MemoryRead,
                banked_address,
                3,
            );
            restored.begin_request(read_request, 0).unwrap();
            let Some(SmsTemporalDispatch::BusCompletion(read_completion)) =
                restored.dispatch_next().unwrap()
            else {
                panic!("expected selected-bank read completion");
            };
            assert_eq!(read_completion.data, Some(3));
            restored.acknowledge_completion(read_request.id).unwrap();

            // Continue the next register case from the restored timeline.
            bus = restored;
        }
    }

    #[test]
    fn mapper_write_completion_remains_latched_across_restore_without_reapplying_effect() {
        let mut bus = SmsTemporalBus::new(patterned_rom(4), 3).unwrap();
        let request = request(20, Z80BusOperation::MemoryWrite { value: 3 }, 0xFFFD, 3);
        bus.begin_request(request, 1).unwrap();
        let first = bus.dispatch_next().unwrap().unwrap();
        let SmsTemporalDispatch::BusCompletion(completion) = first else {
            panic!("expected mapper-register write completion");
        };
        assert_eq!(bus.memory().selected_banks(), [3, 1, 2]);
        assert_eq!(bus.memory().read(0x0400), 3);
        assert_eq!(bus.memory().mapper_register_write_count_for_test(), 1);

        let mut restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        let latched_state = restored.export_state();
        assert_eq!(restored.dispatch_next().unwrap(), Some(first));
        assert_eq!(
            restored.export_state(),
            latched_state,
            "a latched completion must not dispatch the scheduler event or mutate memory twice"
        );
        assert_eq!(restored.memory().selected_banks(), [3, 1, 2]);
        assert_eq!(restored.memory().read(0x0400), 3);
        assert_eq!(restored.memory().mapper_register_write_count_for_test(), 1);
        assert_eq!(restored.acknowledge_completion(20).unwrap(), completion);
        assert_eq!(restored.dispatch_next().unwrap(), None);
        assert_eq!(restored.memory().selected_banks(), [3, 1, 2]);
    }

    #[test]
    fn interrupt_acknowledge_returns_external_vector_after_two_automatic_wait_states() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        bus.set_interrupt_vector(0xCF);
        let request = request(90, Z80BusOperation::InterruptAcknowledge, 0x1234, 3);
        let started_at = bus.now();
        let deadline = bus.begin_request(request, 1).unwrap();

        assert_eq!(
            deadline.checked_duration_since(started_at),
            Some(18),
            "three base T-states plus two automatic and one external WAIT"
        );

        let restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        assert_eq!(restored.export_state(), bus.export_state());
        let Some(SmsTemporalDispatch::BusCompletion(completion)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected interrupt acknowledge completion");
        };
        assert_eq!(completion.data, Some(0xCF));
        assert_eq!(completion.completed_at, deadline);
    }

    #[test]
    fn memory_read_is_sampled_at_dispatch_and_acknowledged_once() {
        let mut bus = SmsTemporalBus::new(vec![0xA5; 0x4000], 3).unwrap();
        let request = request(1, Z80BusOperation::MemoryRead, 0x0100, 3);
        bus.begin_request(request, 2).unwrap();
        let SmsTemporalDispatch::BusCompletion(completion) = bus.dispatch_next().unwrap().unwrap()
        else {
            panic!("expected bus completion");
        };
        assert_eq!(completion.request_id, 1);
        assert_eq!(completion.data, Some(0xA5));
        assert_eq!(bus.acknowledge_completion(1).unwrap(), completion);
        assert_eq!(
            bus.acknowledge_completion(1),
            Err(SmsTemporalBusError::NoPendingRequest)
        );
    }

    #[test]
    fn memory_write_occurs_only_at_its_scheduled_completion() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        let request = request(2, Z80BusOperation::MemoryWrite { value: 0x5A }, 0xC000, 3);
        bus.begin_request(request, 0).unwrap();
        assert_eq!(bus.memory().read(0xC000), 0);
        let SmsTemporalDispatch::BusCompletion(completion) = bus.dispatch_next().unwrap().unwrap()
        else {
            panic!("expected bus completion");
        };
        assert_eq!(completion.data, None);
        assert_eq!(bus.memory().read(0xC000), 0x5A);
        bus.acknowledge_completion(2).unwrap();
        assert_eq!(bus.memory().read(0xC000), 0x5A);
    }

    #[test]
    fn snapshot_restore_preserves_pending_transaction_before_effect() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        let request = request(3, Z80BusOperation::MemoryWrite { value: 0x77 }, 0xC001, 3);
        bus.begin_request(request, 1).unwrap();
        let mut restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        assert_eq!(restored.memory().read(0xC001), 0);
        let SmsTemporalDispatch::BusCompletion(completion) =
            restored.dispatch_next().unwrap().unwrap()
        else {
            panic!("expected bus completion");
        };
        assert_eq!(restored.memory().read(0xC001), 0x77);
        restored
            .acknowledge_completion(completion.request_id)
            .unwrap();
    }

    #[test]
    fn snapshot_restore_preserves_completed_unacknowledged_write() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        let request = request(4, Z80BusOperation::MemoryWrite { value: 0x99 }, 0xC002, 3);
        bus.begin_request(request, 0).unwrap();
        let SmsTemporalDispatch::BusCompletion(completion) = bus.dispatch_next().unwrap().unwrap()
        else {
            panic!("expected bus completion");
        };
        let mut restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        assert_eq!(restored.memory().read(0xC002), 0x99);
        assert_eq!(restored.acknowledge_completion(4).unwrap(), completion);
        assert_eq!(restored.memory().read(0xC002), 0x99);
    }

    #[test]
    fn restore_rejects_bus_deadlines_shorter_than_nominal_cycle() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        let request = request(1, Z80BusOperation::OpcodeFetch, 0, 4);
        bus.begin_request(request, 0).unwrap();

        let mut state = bus.export_state();
        state.coordinator.scheduler.event_queue.pending[0].timestamp = MachineTime::from_ticks(3);
        assert_eq!(
            SmsTemporalBus::restore_state(state).err(),
            Some(SmsTemporalBusError::InvalidState)
        );
    }

    #[test]
    fn restore_rejects_bus_deadlines_not_aligned_to_tstate_scale() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        let request = request(1, Z80BusOperation::OpcodeFetch, 0, 4);
        bus.begin_request(request, 0).unwrap();

        let mut state = bus.export_state();
        state.coordinator.scheduler.event_queue.pending[0].timestamp = MachineTime::from_ticks(13);
        assert_eq!(
            SmsTemporalBus::restore_state(state).err(),
            Some(SmsTemporalBusError::InvalidState)
        );
    }

    #[test]
    fn internal_cycle_dispatch_has_no_memory_side_effect() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        let deadline = bus.schedule_internal_cycle(77, 1).unwrap();
        let Some(SmsTemporalDispatch::InternalCycleComplete(completion)) =
            bus.dispatch_next().unwrap()
        else {
            panic!("expected internal-cycle completion");
        };
        assert_eq!(completion.continuation_id, 77);
        assert_eq!(completion.completed_at, deadline);
        assert_eq!(bus.memory().read(0xC000), 0);
    }

    #[test]
    fn internal_completion_is_latched_until_acknowledged_and_survives_restore() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        let deadline = bus.schedule_internal_cycle(77, 1).unwrap();
        let first = bus.dispatch_next().unwrap().unwrap();
        assert_eq!(
            first,
            SmsTemporalDispatch::InternalCycleComplete(super::SmsInternalCycleCompletion {
                continuation_id: 77,
                completed_at: deadline,
            })
        );
        assert_eq!(bus.dispatch_next().unwrap(), Some(first));

        let mut restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        assert_eq!(restored.dispatch_next().unwrap(), Some(first));
        assert_eq!(
            restored.acknowledge_internal_completion(77).unwrap(),
            match first {
                SmsTemporalDispatch::InternalCycleComplete(completion) => completion,
                _ => unreachable!(),
            }
        );
        assert_eq!(restored.dispatch_next().unwrap(), None);
    }

    #[test]
    fn psg_writes_commit_at_the_bus_deadline_and_restore_with_clock_phase() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        assert_eq!(bus.psg().profile(), PsgProfile::Sms);
        assert_eq!(bus.psg().clock_hz(), DEFAULT_SMS_PSG_CLOCK_HZ);
        assert_eq!(bus.psg().clock_divider_phase(), 0);

        let write = request(70, Z80BusOperation::PortWrite { value: 0x85 }, 0x7E, 4);
        let deadline = bus.begin_request(write, 0).unwrap();
        let mut restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        assert_eq!(restored.psg().clock_divider_phase(), 0);
        assert_eq!(restored.psg().latched_register(), 0);

        let Some(SmsTemporalDispatch::BusCompletion(completion)) =
            restored.dispatch_next().unwrap()
        else {
            panic!("expected PSG port-write completion");
        };
        assert_eq!(completion.completed_at, deadline);
        assert_eq!(deadline, MachineTime::from_ticks(12));
        assert_eq!(restored.psg().clock_divider_phase(), 4);
        assert_eq!(restored.psg().latched_register(), 0);
        assert_eq!(restored.psg().tone_periods()[0], 5);
        restored.acknowledge_completion(70).unwrap();

        let data = request(71, Z80BusOperation::PortWrite { value: 0x12 }, 0x7F, 4);
        restored.begin_request(data, 0).unwrap();
        restored.dispatch_next().unwrap();
        assert_eq!(restored.psg().tone_periods()[0], 0x125);
        assert_eq!(restored.psg().clock_divider_phase(), 8);
        restored.acknowledge_completion(71).unwrap();

        let volume = request(72, Z80BusOperation::PortWrite { value: 0x90 }, 0x40, 4);
        restored.begin_request(volume, 0).unwrap();
        restored.dispatch_next().unwrap();
        assert_eq!(restored.psg().attenuations()[0], 0);
        assert_eq!(restored.psg().clock_divider_phase(), 12);
        restored.acknowledge_completion(72).unwrap();

        let snapshot = restored.export_state();
        let round_trip = SmsTemporalBus::restore_state(snapshot.clone()).unwrap();
        assert_eq!(round_trip.export_state(), snapshot);
        assert_eq!(round_trip.psg().clock_divider_phase(), 12);
    }

    #[test]
    fn psg_output_is_identical_for_continuous_and_chunked_scheduler_advancement() {
        let mut whole = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        let mut chunked = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();

        whole.schedule_internal_cycle(1, 100_000).unwrap();
        let Some(SmsTemporalDispatch::InternalCycleComplete(_)) = whole.dispatch_next().unwrap()
        else {
            panic!("expected one whole-interval completion");
        };
        whole.acknowledge_internal_completion(1).unwrap();

        for continuation_id in 1..=10 {
            chunked
                .schedule_internal_cycle(continuation_id, 10_000)
                .unwrap();
            let Some(SmsTemporalDispatch::InternalCycleComplete(_)) =
                chunked.dispatch_next().unwrap()
            else {
                panic!("expected chunked interval completion");
            };
            chunked
                .acknowledge_internal_completion(continuation_id)
                .unwrap();
        }

        assert_eq!(whole.psg(), chunked.psg());
        assert_eq!(
            whole.drain_audio_samples(None),
            chunked.drain_audio_samples(None)
        );
        assert_eq!(whole.psg(), chunked.psg());
        assert_eq!(whole.now(), chunked.now());
    }

    #[test]
    fn psg_clock_advances_with_cpu_clock_domain_for_each_dispatched_boundary() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 5).unwrap();
        let request = request(73, Z80BusOperation::MemoryRead, 0x0000, 7);
        bus.begin_request(request, 2).unwrap();
        bus.dispatch_next().unwrap();
        assert_eq!(
            bus.coordinator.z80_clock_snapshot().unwrap().total_cycles,
            9
        );
        assert_eq!(
            bus.coordinator.psg_clock_snapshot().unwrap().total_cycles,
            9
        );
        assert_eq!(bus.psg().clock_divider_phase(), 9);
    }

    #[test]
    fn port_reads_are_accepted_and_complete_at_the_scheduled_boundary() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        let deadline = bus
            .begin_request(request(5, Z80BusOperation::PortRead, 0x00DC, 4), 0)
            .unwrap();
        let Some(SmsTemporalDispatch::BusCompletion(completion)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected port-read completion");
        };
        assert_eq!(completion.request_id, 5);
        assert_eq!(completion.data, Some(0xFF));
        assert_eq!(completion.completed_at, deadline);
        bus.acknowledge_completion(5).unwrap();
    }
}

#[cfg(test)]
mod immediate_port_transaction_tests {
    use super::{SmsTemporalBus, SmsTemporalDispatch};
    use crate::input::ControllerButtons;
    use sega_temporal_core::MachineTime;
    use sega_z80_cpu::{Z80BusOperation, Z80BusRequest};

    fn request(id: u64, operation: Z80BusOperation, address: u16) -> Z80BusRequest {
        Z80BusRequest {
            id,
            operation,
            address,
            base_tstates: 4,
        }
    }

    #[test]
    fn port_reads_writes_and_controller_state_commit_at_scheduled_boundaries() {
        let mut bus = SmsTemporalBus::new(vec![0x00], 3).unwrap();
        bus.io_mut()
            .set_controller(
                1,
                ControllerButtons {
                    up: true,
                    ..ControllerButtons::default()
                },
            )
            .unwrap();

        bus.begin_request(request(0, Z80BusOperation::PortRead, 0x12DC), 0)
            .unwrap();
        let read = bus.dispatch_next().unwrap().unwrap();
        assert_eq!(
            read,
            SmsTemporalDispatch::BusCompletion(super::SmsBusCompletion {
                request_id: 0,
                data: Some(0xFE),
                completed_at: MachineTime::from_ticks(12),
            })
        );
        bus.acknowledge_completion(0).unwrap();

        bus.begin_request(
            request(1, Z80BusOperation::PortWrite { value: 0xD5 }, 0xD53F),
            0,
        )
        .unwrap();
        assert!(matches!(
            bus.dispatch_next().unwrap(),
            Some(SmsTemporalDispatch::BusCompletion(_))
        ));
        bus.acknowledge_completion(1).unwrap();
        assert_eq!(bus.io().io_control(), 0xD5);

        bus.begin_request(
            request(2, Z80BusOperation::PortWrite { value: 0x55 }, 0x553E),
            0,
        )
        .unwrap();
        assert!(matches!(
            bus.dispatch_next().unwrap(),
            Some(SmsTemporalDispatch::BusCompletion(_))
        ));
        bus.acknowledge_completion(2).unwrap();
        assert_eq!(bus.memory().control_port(), 0x55);
        let restored = SmsTemporalBus::restore_state(bus.export_state()).unwrap();
        assert_eq!(restored.io().io_control(), 0xD5);
        assert_eq!(restored.memory().control_port(), 0x55);
    }
}

#[cfg(test)]
mod temporal_vdp_tests {
    use super::{SmsTemporalBus, SmsTemporalDispatch, SmsVdpProfile};
    use sega_temporal_core::MachineTime;
    use sega_z80_cpu::{Z80BusOperation, Z80BusRequest};

    fn transact(
        bus: &mut SmsTemporalBus,
        id: u64,
        operation: Z80BusOperation,
        address: u16,
    ) -> super::SmsBusCompletion {
        let request = Z80BusRequest {
            id,
            operation,
            address,
            base_tstates: 4,
        };
        bus.begin_request(request, 0).unwrap();
        let Some(SmsTemporalDispatch::BusCompletion(completion)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected scheduled port transaction");
        };
        bus.acknowledge_completion(id).unwrap();
        completion
    }

    fn write_vdp_control(bus: &mut SmsTemporalBus, id: u64, value: u8) {
        transact(bus, id, Z80BusOperation::PortWrite { value }, 0x00BF);
    }

    #[test]
    fn default_sms_temporal_bus_uses_the_canonical_48khz_pcm_source_rate() {
        let bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        assert_eq!(
            bus.psg().sample_rate(),
            super::DEFAULT_PCM_SOURCE_RATE_HZ as u64
        );
    }

    #[test]
    fn vdp_data_write_is_invisible_until_the_scheduled_port_boundary() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        assert_eq!(bus.vdp().vram()[0], 0);

        // Select VRAM address zero for writing through the two-byte control word.
        write_vdp_control(&mut bus, 1, 0x00);
        write_vdp_control(&mut bus, 2, 0x40);

        let request = Z80BusRequest {
            id: 3,
            operation: Z80BusOperation::PortWrite { value: 0xA5 },
            address: 0x00BE,
            base_tstates: 4,
        };
        bus.begin_request(request, 0).unwrap();
        assert_eq!(bus.vdp().vram()[0], 0);

        let Some(SmsTemporalDispatch::BusCompletion(completion)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected VDP data-port write completion");
        };
        assert_eq!(completion.completed_at, MachineTime::from_ticks(36));
        assert_eq!(bus.vdp().vram()[0], 0xA5);
        bus.acknowledge_completion(3).unwrap();
    }

    #[test]
    fn vdp_read_ahead_is_sampled_at_the_scheduled_data_port_boundary() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        write_vdp_control(&mut bus, 1, 0x00);
        write_vdp_control(&mut bus, 2, 0x40);
        transact(
            &mut bus,
            3,
            Z80BusOperation::PortWrite { value: 0x5A },
            0x00BE,
        );

        // Select address zero for VRAM read; the control-word completion
        // preloads the VDP read-ahead buffer.
        write_vdp_control(&mut bus, 4, 0x00);
        write_vdp_control(&mut bus, 5, 0x00);

        let request = Z80BusRequest {
            id: 6,
            operation: Z80BusOperation::PortRead,
            address: 0x00BE,
            base_tstates: 4,
        };
        bus.begin_request(request, 0).unwrap();
        let Some(SmsTemporalDispatch::BusCompletion(completion)) = bus.dispatch_next().unwrap()
        else {
            panic!("expected VDP data-port read completion");
        };
        assert_eq!(completion.data, Some(0x5A));
        bus.acknowledge_completion(6).unwrap();
    }

    #[test]
    fn vdp_counters_are_read_through_the_sms_mirrored_ports() {
        let mut bus = SmsTemporalBus::new(vec![0; 0x4000], 3).unwrap();
        let v_counter = transact(&mut bus, 1, Z80BusOperation::PortRead, 0x127E);
        assert_eq!(v_counter.data, Some(0));

        // H-counter is latched and remains at its reset value until TH rises.
        let h_counter = transact(&mut bus, 2, Z80BusOperation::PortRead, 0x127F);
        assert_eq!(h_counter.data, Some(0));

        let h_counter_after_more_cycles = transact(&mut bus, 3, Z80BusOperation::PortRead, 0x007F);
        assert_eq!(h_counter_after_more_cycles.data, Some(0));
    }

    #[test]
    fn h_counter_port_reads_return_the_th_latched_value_without_retiming_it() {
        let mut bus = SmsTemporalBus::new_with_hardware_profile(
            vec![0; 0x4000],
            3,
            super::DEFAULT_SMS_PSG_CLOCK_HZ,
            super::DEFAULT_AUDIO_SAMPLE_RATE,
            SmsVdpProfile::sms1_t0_ntsc(),
            super::Territory::Export,
        )
        .unwrap();

        // Drive TH low, then release it as an input. The rising edge is
        // committed by the temporal bus at T8 / HPos 12 and captures 0xEE.
        transact(
            &mut bus,
            1,
            Z80BusOperation::PortWrite { value: 0xDD },
            0x003F,
        );
        transact(
            &mut bus,
            2,
            Z80BusOperation::PortWrite { value: 0xDF },
            0x003F,
        );
        assert_eq!(bus.vdp().read_h_counter(), 0xEE);

        // Reads through mirrored $7F ports observe the frozen latch. Advancing
        // the Scheduler between reads must not turn H-counter into a live clock.
        let first = transact(&mut bus, 3, Z80BusOperation::PortRead, 0x127F);
        assert_eq!(first.data, Some(0xEE));

        let second = transact(&mut bus, 4, Z80BusOperation::PortRead, 0x007F);
        assert_eq!(second.data, Some(0xEE));
        assert_eq!(bus.vdp().read_h_counter(), 0xEE);
    }

    #[test]
    fn h_counter_th_capture_is_invariant_to_scheduler_chunking() {
        fn advance_internal(bus: &mut SmsTemporalBus, id: u64, tstates: u32) {
            bus.schedule_internal_cycle(id, tstates).unwrap();
            assert!(matches!(
                bus.dispatch_next().unwrap(),
                Some(SmsTemporalDispatch::InternalCycleComplete(_))
            ));
            bus.acknowledge_internal_completion(id).unwrap();
        }

        fn capture_th(bus: &mut SmsTemporalBus) {
            transact(bus, 10, Z80BusOperation::PortWrite { value: 0xDD }, 0x003F);
            transact(bus, 11, Z80BusOperation::PortWrite { value: 0xDF }, 0x003F);
        }

        let mut continuous = SmsTemporalBus::new_with_hardware_profile(
            vec![0; 0x4000],
            3,
            super::DEFAULT_SMS_PSG_CLOCK_HZ,
            super::DEFAULT_AUDIO_SAMPLE_RATE,
            SmsVdpProfile::sms1_t0_ntsc(),
            super::Territory::Export,
        )
        .unwrap();
        let mut chunked = continuous.clone();

        advance_internal(&mut continuous, 1, 8);
        advance_internal(&mut chunked, 1, 3);
        advance_internal(&mut chunked, 2, 5);

        capture_th(&mut continuous);
        capture_th(&mut chunked);

        assert_eq!(continuous.vdp().read_h_counter(), 0xF4);
        assert_eq!(
            continuous.vdp().read_h_counter(),
            chunked.vdp().read_h_counter(),
            "TH must capture the same H-counter after equal elapsed Scheduler time"
        );
    }

    #[test]
    fn vdp_raster_progress_is_chunk_invariant_and_survives_bus_restore() {
        const FRAME_TSTATES: u32 = 228 * 262;
        let mut whole = SmsTemporalBus::new(vec![0; 0x4000], 1).unwrap();
        let mut chunked = SmsTemporalBus::new(vec![0; 0x4000], 1).unwrap();

        whole.schedule_internal_cycle(1, FRAME_TSTATES).unwrap();
        assert!(matches!(
            whole.dispatch_next().unwrap(),
            Some(SmsTemporalDispatch::InternalCycleComplete(_))
        ));
        whole.acknowledge_internal_completion(1).unwrap();

        for (id, tstates) in [(1, FRAME_TSTATES / 2), (2, FRAME_TSTATES / 2)] {
            chunked.schedule_internal_cycle(id, tstates).unwrap();
            assert!(matches!(
                chunked.dispatch_next().unwrap(),
                Some(SmsTemporalDispatch::InternalCycleComplete(_))
            ));
            chunked.acknowledge_internal_completion(id).unwrap();
        }

        assert_eq!(whole.vdp().frame_number(), 1);
        assert_eq!(whole.vdp(), chunked.vdp());

        let snapshot = whole.export_state();
        let restored = SmsTemporalBus::restore_state(snapshot.clone()).unwrap();
        assert_eq!(restored.export_state(), snapshot);
        assert_eq!(restored.vdp(), whole.vdp());
    }

    #[test]
    fn h_counter_latch_survives_bus_snapshot_restore_and_rejects_old_format() {
        let mut bus = SmsTemporalBus::new_with_hardware_profile(
            vec![0; 0x4000],
            3,
            super::DEFAULT_SMS_PSG_CLOCK_HZ,
            super::DEFAULT_AUDIO_SAMPLE_RATE,
            SmsVdpProfile::sms1_t0_ntsc(),
            super::Territory::Export,
        )
        .unwrap();
        transact(
            &mut bus,
            1,
            Z80BusOperation::PortWrite { value: 0xDD },
            0x003F,
        );
        transact(
            &mut bus,
            2,
            Z80BusOperation::PortWrite { value: 0xDF },
            0x003F,
        );

        let captured = bus.vdp().read_h_counter();
        // Two direct four-T-state bus writes complete at T8 (HPos 12).
        assert_eq!(captured, 0xEE);
        let mut snapshot = bus.export_state();
        assert_eq!(snapshot.format_version, 24);

        let restored = SmsTemporalBus::restore_state(snapshot.clone()).unwrap();
        assert_eq!(restored.vdp().read_h_counter(), captured);

        snapshot.format_version = 20;
        assert!(matches!(
            SmsTemporalBus::restore_state(snapshot),
            Err(super::SmsTemporalBusError::InvalidState)
        ));
    }

    #[test]
    fn japanese_output_high_to_input_high_does_not_fake_a_th_rising_edge() {
        fn japanese_bus() -> SmsTemporalBus {
            SmsTemporalBus::new_with_hardware_profile(
                vec![0; 0x4000],
                3,
                super::DEFAULT_SMS_PSG_CLOCK_HZ,
                super::DEFAULT_AUDIO_SAMPLE_RATE,
                SmsVdpProfile::sms1_t0_ntsc(),
                super::Territory::Japan,
            )
            .unwrap()
        }

        let mut th1_bus = japanese_bus();
        // TH1: output-high -> input-high. Japan hides the output-high readback.
        transact(
            &mut th1_bus,
            1,
            Z80BusOperation::PortWrite { value: 0xFD },
            0x003F,
        );
        transact(
            &mut th1_bus,
            2,
            Z80BusOperation::PortWrite { value: 0xFF },
            0x003F,
        );
        assert_eq!(th1_bus.vdp().read_h_counter(), 0);

        let mut th2_bus = japanese_bus();
        // TH2: output-high -> input-high, with TH1 already left as input-high.
        transact(
            &mut th2_bus,
            1,
            Z80BusOperation::PortWrite { value: 0xF7 },
            0x003F,
        );
        transact(
            &mut th2_bus,
            2,
            Z80BusOperation::PortWrite { value: 0xFF },
            0x003F,
        );
        assert_eq!(th2_bus.vdp().read_h_counter(), 0);

        let mut valid_th1_bus = japanese_bus();
        transact(
            &mut valid_th1_bus,
            1,
            Z80BusOperation::PortWrite { value: 0xDD },
            0x003F,
        );
        transact(
            &mut valid_th1_bus,
            2,
            Z80BusOperation::PortWrite { value: 0xDF },
            0x003F,
        );
        assert_ne!(valid_th1_bus.vdp().read_h_counter(), 0);
    }
    #[test]
    fn japanese_fm_port_counters_track_completed_address_data_and_control_writes() {
        let mut bus = SmsTemporalBus::new_with_japanese_fm_hardware_profile(
            vec![0; 0x4000],
            3,
            super::DEFAULT_SMS_PSG_CLOCK_HZ,
            super::DEFAULT_AUDIO_SAMPLE_RATE,
            SmsVdpProfile::sms1_t0_ntsc(),
            super::Territory::Japan,
        )
        .unwrap();

        transact(
            &mut bus,
            1,
            Z80BusOperation::PortWrite { value: 0x20 },
            0x00F0,
        );
        transact(
            &mut bus,
            2,
            Z80BusOperation::PortWrite { value: 0x7F },
            0x00F1,
        );
        transact(
            &mut bus,
            3,
            Z80BusOperation::PortWrite { value: 0x03 },
            0x00F2,
        );

        let writes = bus.fm_port_write_snapshot();
        assert_eq!(writes.address_writes, 1);
        assert_eq!(writes.data_writes, 1);
        assert_eq!(writes.control_writes, 1);
        assert_eq!(writes.last_address, Some(0x20));
        assert_eq!(writes.last_control, Some(0x03));
        assert_eq!(writes.register_writes[0x20], 1);
        assert_eq!(writes.last_register_values[0x20], Some(0x7F));
    }
}
