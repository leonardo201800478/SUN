//! Full upstream SmsVdpTest ROM execution for SMS1 VDP conformance.
//!
//! The dedicated sms-vdp-full-rom workflow builds the pinned upstream source
//! with WLA-DX and supplies SMSVDPTEST_ROM_PATH. This test is ignored in the
//! ordinary workspace suite so the external assembler is not a normal build
//! dependency.

use std::{env, fs};

use sega_sms_core::{SmsTemporalMachine, SmsTemporalTraceEvent, SmsVdpProfile, Territory};
use sega_z80_cpu::Z80BusOperation;

const PSG_CLOCK_HZ: u64 = 3_579_545;
const SAMPLE_RATE_HZ: u64 = 44_100;
const COMPLETION_MARKER: u16 = 0xC9FF;

#[test]
#[ignore = "run by the dedicated SMS VDP full-ROM conformance workflow"]
fn smsvdptest_v131_full_rom_passes_vcounter_and_irq_hcount_tests() {
    let rom_path = env::var_os("SMSVDPTEST_ROM_PATH")
        .expect("SMSVDPTEST_ROM_PATH must point to the patched upstream SmsVdpTest ROM");
    let rom = fs::read(&rom_path).expect("the assembled SmsVdpTest ROM must be readable");

    let mut machine = SmsTemporalMachine::new_with_hardware_profile(
        rom,
        1,
        PSG_CLOCK_HZ,
        SAMPLE_RATE_HZ,
        SmsVdpProfile::sms1_t0_ntsc(),
        Territory::Export,
    )
    .expect("the full SmsVdpTest ROM should initialize on the SMS1 temporal machine");

    machine.set_trace_enabled(true);
    let mut completed_at_frame = None;
    let mut hcounter_reference_reads = Vec::new();
    let mut vcounter_reference_reads = Vec::new();

    for frame in 0..600 {
        if let Err(error) = machine.step_frame(100_000) {
            panic!("SmsVdpTest failed while advancing frame {frame}: {error}");
        }
        for event in machine.take_trace() {
            match event {
                SmsTemporalTraceEvent::BusCompleted {
                    timestamp,
                    operation: Z80BusOperation::PortRead,
                    address,
                    data,
                    ..
                } if address & 0x00FF == 0x7E => {
                    vcounter_reference_reads.push((timestamp.ticks(), data.unwrap_or(0)));
                }
                SmsTemporalTraceEvent::BusCompleted {
                    timestamp,
                    operation: Z80BusOperation::PortRead,
                    address,
                    data,
                    ..
                } if address & 0x00FF == 0x7F && (0x80..=0x8F).contains(&data.unwrap_or(0)) => {
                    hcounter_reference_reads.push((timestamp.ticks(), data.unwrap_or(0)));
                }
                SmsTemporalTraceEvent::BusCompleted {
                    operation: Z80BusOperation::PortWrite { .. },
                    address,
                    ..
                } if address & 0x00FF == 0x3F => {}
                _ => {}
            }
        }
        if machine.inspect_memory(COMPLETION_MARKER) == 0xAA {
            completed_at_frame = Some(frame + 1);
            break;
        }
    }

    let results = [
        (25, 0xC918),
        (31, 0xC91E),
        (32, 0xC91F),
        (33, 0xC920),
        (34, 0xC921),
    ]
    .map(|(test, address)| (test, machine.inspect_memory(address)));
    let all_results = (21u16..=34)
        .map(|test| (test, machine.inspect_memory(0xC900 + test - 1)))
        .collect::<Vec<_>>();
    // prepare_sms_vdp_test_rom.py relocates RunTest25's TestScratch to $C9F0
    // without adding cycles, keeping its three VCounter samples isolated.
    let run_test25_samples = [
        machine.inspect_memory(0xC9F0),
        machine.inspect_memory(0xC9F1),
        machine.inspect_memory(0xC9F2),
    ];

    assert!(
        completed_at_frame.is_some(),
        "SmsVdpTest did not reach the post-RunTest34 marker within 600 frames; results={results:?}; all={all_results:?}"
    );

    assert_eq!(
        results,
        [(25, 0), (31, 0), (32, 0), (33, 0), (34, 0)],
        "upstream SmsVdpTest v1.31 failures; all test results={all_results:?}; RunTest25 VCounter samples at $C880-$C882={run_test25_samples:?}; VCounter port reads={vcounter_reference_reads:?}; HCounter reference reads={hcounter_reference_reads:?}"
    );
}
