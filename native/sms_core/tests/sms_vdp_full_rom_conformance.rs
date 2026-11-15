//! Full upstream SmsVdpTest ROM execution for SMS1 VDP conformance.
//!
//! The dedicated sms-vdp-full-rom workflow builds the pinned upstream source
//! with WLA-DX and supplies SMSVDPTEST_ROM_PATH. This test is ignored in the
//! ordinary workspace suite so the external assembler is not a normal build
//! dependency.

use std::{env, fs};

use sega_sms_core::{SmsTemporalMachine, SmsVdpProfile, Territory};

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

    let mut completed_at_frame = None;
    for frame in 0..600 {
        machine
            .step_frame(100_000)
            .unwrap_or_else(|error| panic!("SmsVdpTest failed while advancing frame {frame}: {error}"));
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

    assert!(
        completed_at_frame.is_some(),
        "SmsVdpTest did not reach the post-RunTest34 marker within 600 frames; results={results:?}"
    );

    for (test, result) in results {
        assert_eq!(
            result, 0,
            "upstream SmsVdpTest v1.31 RunTest{test} failed (result byte at its RAM slot)"
        );
    }
}
