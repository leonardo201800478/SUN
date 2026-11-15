//! Z80 immediate-I/O instruction integration with the SMS1 VDP and Scheduler.
//!
//! These tests validate the CPU-visible path rather than calling VDP ports
//! directly: opcode fetch, immediate operand fetch, I/O M-cycle, Scheduler
//! completion, VDP side effect, and (for IN) the returned byte.

use sega_sms_core::{SmsTemporalMachine, SmsTemporalTraceEvent, SmsVdpProfile, Territory};
use sega_z80_cpu::Z80BusOperation;

const ROM_SIZE: usize = 0x8000;
const PSG_CLOCK_HZ: u64 = 3_579_545;
const SAMPLE_RATE_HZ: u64 = 44_100;

fn machine_with_program(program: &[u8]) -> SmsTemporalMachine {
    let mut rom = vec![0; ROM_SIZE];
    rom[..program.len()].copy_from_slice(program);
    SmsTemporalMachine::new_with_hardware_profile(
        rom,
        1,
        PSG_CLOCK_HZ,
        SAMPLE_RATE_HZ,
        SmsVdpProfile::sms1_t0_ntsc(),
        Territory::Export,
    )
    .expect("SMS1 temporal machine should initialize")
}

fn run_instructions(machine: &mut SmsTemporalMachine, count: usize) {
    for _ in 0..count {
        machine
            .step_complete_instruction()
            .expect("Z80 instruction should complete through the Scheduler");
    }
}

#[test]
fn z80_out_immediate_a_commits_vdp_control_and_data_writes_on_io_boundaries() {
    // LD A,0; OUT ($BF),A; LD A,$40; OUT ($BF),A;
    // LD A,$A5; OUT ($BE),A; HALT.
    // The two control writes select VRAM address 0 for writing.
    let program = [
        0x3E, 0x00, 0xD3, 0xBF, 0x3E, 0x40, 0xD3, 0xBF, 0x3E, 0xA5, 0xD3, 0xBE, 0x76,
    ];
    let mut machine = machine_with_program(&program);
    machine.set_trace_enabled(true);

    run_instructions(&mut machine, 6);

    assert_eq!(
        machine.vdp().vram()[0],
        0xA5,
        "OUT ($BE),A must reach the VDP data port after the control-word writes"
    );

    let writes: Vec<_> = machine
        .take_trace()
        .into_iter()
        .filter_map(|event| match event {
            SmsTemporalTraceEvent::BusCompleted {
                timestamp,
                operation: Z80BusOperation::PortWrite { value },
                address,
                base_tstates,
                data,
                ..
            } => Some((timestamp.ticks(), address, value, base_tstates, data)),
            _ => None,
        })
        .collect();

    assert_eq!(
        writes,
        vec![
            (18, 0x00BF, 0x00, 4, None),
            (36, 0x40BF, 0x40, 4, None),
            (54, 0xA5BE, 0xA5, 4, None),
        ],
        "OUT (n),A must preserve A as the high port-address byte and commit the I/O M-cycle at the expected 4T boundary"
    );
}

#[test]
fn z80_in_immediate_a_reads_the_th_latched_hcounter_through_the_scheduler() {
    // LD A,$DD; OUT ($3F),A; LD A,$DF; OUT ($3F),A;
    // IN A,($7F); LD ($C000),A; HALT.
    // The $DD->$DF transition creates the software TH rising edge.
    let program = [
        0x3E, 0xDD, 0xD3, 0x3F, 0x3E, 0xDF, 0xD3, 0x3F, 0xDB, 0x7F, 0x32, 0x00, 0xC0, 0x76,
    ];
    let mut machine = machine_with_program(&program);
    machine.set_trace_enabled(true);

    run_instructions(&mut machine, 7);

    let captured = machine.inspect_memory(0xC000);
    assert_eq!(captured, 0x03, "TH rising edge at T36 should latch HCounter $03");
    assert_eq!(
        captured,
        machine.vdp().read_h_counter(),
        "IN A,($7F) must return the TH-latched HCounter byte through the temporal I/O bus"
    );

    let reads: Vec<_> = machine
        .take_trace()
        .into_iter()
        .filter_map(|event| match event {
            SmsTemporalTraceEvent::BusCompleted {
                timestamp,
                operation: Z80BusOperation::PortRead,
                address,
                base_tstates,
                data,
                ..
            } => Some((timestamp.ticks(), address, base_tstates, data)),
            _ => None,
        })
        .collect();

    assert_eq!(
        reads.len(),
        1,
        "program should perform one immediate I/O read"
    );
    assert_eq!(reads[0].0, 47, "IN I/O read completes after its 4T I/O M-cycle");
    assert_eq!(reads[0].1, 0xDF7F, "IN A,(n) uses A as the high port byte");
    assert_eq!(reads[0].2, 4, "I/O read M-cycle must take four T-states");
    assert_eq!(reads[0].3, Some(captured));
}
