//! Local, ROM-preserving boot trace for investigating SMS direct-boot state.
//!
//! Run from the repository root:
//! cargo run --release -p sega-sms-core --example ace_of_aces_boot_trace -- <rom-path> [europe|brazil|usa] [max-instructions] [trace-path]
//!
//! The ROM is read locally and is never copied into the repository or trace.
//! The trace records CPU checkpoints and completed bus transactions only.

use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};

use sega_sms_core::{
    SmsTemporalMachine, SmsTemporalTraceEvent, SmsVdpProfile, SmsVdpRevision, SpriteLimitMode,
    Territory, VideoStandard,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        return Err(
            "usage: ace_of_aces_boot_trace <rom-path> [europe|brazil|usa] [max-instructions] [trace-path]"
                .into(),
        );
    }

    let rom_path = &args[1];
    let region = args.get(2).map(String::as_str).unwrap_or("europe");
    let max_instructions = args
        .get(3)
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(5_000);
    let trace_path = args
        .get(4)
        .map(String::as_str)
        .unwrap_or("ace_of_aces_boot_trace.log");

    let (video_standard, psg_clock_hz) = match region {
        "europe" => (VideoStandard::Pal, 3_546_895),
        "brazil" => (VideoStandard::PalM, 3_579_545),
        "usa" => (VideoStandard::Ntsc, 3_579_545),
        _ => return Err("region must be europe, brazil or usa".into()),
    };

    let rom = fs::read(rom_path)?;
    let mut machine = SmsTemporalMachine::new_with_hardware_profile(
        rom,
        1,
        psg_clock_hz,
        44_100,
        SmsVdpProfile::new(
            SmsVdpRevision::Sega3155124,
            video_standard,
            SpriteLimitMode::HardwareLimit,
        ),
        Territory::Export,
    )?;
    machine.set_trace_enabled(true);

    let mut output = BufWriter::new(File::create(trace_path)?);
    writeln!(
        output,
        "BOOT rom_bytes={} region={} boot_path=direct_without_bios",
        fs::metadata(rom_path)?.len(),
        region
    )?;
    writeln!(
        output,
        "INITIAL t={:?} PC=${:04X} SP=${:04X} banks={:?} control=${:02X} c000=${:02X} bios_mapped={} slot1_probe=${:02X}",
        machine.now(),
        machine.cpu().registers.pc,
        machine.cpu().registers.sp,
        machine.memory().selected_banks(),
        machine.memory().control_port(),
        machine.memory().read(0xC000),
        machine.memory().bios_enabled(),
        machine.memory().read(0x4000)
    )?;

    let mut mapper_mutation_seen = false;
    for index in 0..max_instructions {
        let before_pc = machine.cpu().registers.pc;
        let before_sp = machine.cpu().registers.sp;
        let before_banks = machine.memory().selected_banks();
        let before_control = machine.memory().control_port();

        machine.step_complete_instruction()?;

        let registers = machine.cpu().registers;
        let banks = machine.memory().selected_banks();
        let control = machine.memory().control_port();
        let mapper_changed = banks != before_banks;
        mapper_mutation_seen |= mapper_changed;

        writeln!(
            output,
            "STEP n={} t={:?} PC=${:04X}->${:04X} SP=${:04X}->${:04X} AF=${:02X}{:02X} BC=${:02X}{:02X} DE=${:02X}{:02X} HL=${:02X}{:02X} banks={:?} control=${:02X} c000=${:02X} slot1_probe=${:02X}",
            index + 1,
            machine.now(),
            before_pc,
            registers.pc,
            before_sp,
            registers.sp,
            registers.a,
            registers.f,
            registers.b,
            registers.c,
            registers.d,
            registers.e,
            registers.h,
            registers.l,
            banks,
            control,
            machine.memory().read(0xC000),
            machine.memory().read(0x4000)
        )?;

        if mapper_changed || control != before_control {
            writeln!(
                output,
                "MAPPER_OR_CONTROL_CHANGE n={} previous_banks={:?} banks={:?} previous_control=${:02X} control=${:02X} read_620A=${:02X}",
                index + 1,
                before_banks,
                banks,
                before_control,
                control,
                machine.memory().read(0x620A)
            )?;
        }

        for event in machine.take_trace() {
            if let SmsTemporalTraceEvent::BusCompleted {
                timestamp,
                operation,
                address,
                base_tstates,
                wait_tstates,
                data,
                ..
            } = event
            {
                writeln!(
                    output,
                    "BUS t={:?} op={:?} addr=${:04X} data={:?} base_tstates={} wait_tstates={}",
                    timestamp, operation, address, data, base_tstates, wait_tstates
                )?;
            }
        }

        // Stop after the observed CALL/RET path has returned to $001F and the
        // cartridge has executed LD SP,$DFF0 at $0022.
        if mapper_mutation_seen && registers.pc == 0x0025 && registers.sp == 0xDFF0 {
            writeln!(
                output,
                "BOOT_CHECKPOINT_REACHED t={:?} PC=${:04X} SP=${:04X} banks={:?} slot1_probe=${:02X}",
                machine.now(),
                registers.pc,
                registers.sp,
                banks,
                machine.memory().read(0x4000)
            )?;
            break;
        }
    }

    output.flush()?;
    println!("Trace written to {trace_path}");
    println!(
        "Final PC=${:04X} SP=${:04X} banks={:?} control=${:02X}",
        machine.cpu().registers.pc,
        machine.cpu().registers.sp,
        machine.memory().selected_banks(),
        machine.memory().control_port()
    );
    Ok(())
}
