//! Direct-boot stack writes at $FFFE/$FFFF are real Sega mapper writes.
//!
//! This protects the hardware semantics relevant to cartridges that execute
//! a CALL before initializing SP. It is intentionally a synthetic ROM fixture;
//! no commercial Ace of Aces bytes are embedded here.

use sega_sms_core::{SmsTemporalMachine, SmsVdpProfile, Territory};

const PSG_CLOCK_HZ: u64 = 3_579_545;
const SAMPLE_RATE_HZ: u64 = 44_100;
const ROM_SIZE: usize = 0x40000;

#[test]
fn direct_boot_call_before_sp_initialization_mutates_sega_mapper_banks() {
    let mut rom = vec![0xFF; ROM_SIZE];

    // Reset vector -> $001C, matching the static startup sequence previously
    // identified in Ace of Aces (Europe, Brazil).
    rom[0x0000..0x0003].copy_from_slice(&[0xC3, 0x1C, 0x00]); // JP $001C
    rom[0x001C..0x001F].copy_from_slice(&[0xCD, 0xB2, 0x02]); // CALL $02B2
    rom[0x001F..0x0022].copy_from_slice(&[0x31, 0xF0, 0xDF]); // LD SP,$DFF0
    rom[0x0022] = 0x76; // HALT
    rom[0x02B2] = 0xC9; // RET

    let mut machine = SmsTemporalMachine::new_with_hardware_profile(
        rom,
        1,
        PSG_CLOCK_HZ,
        SAMPLE_RATE_HZ,
        SmsVdpProfile::sms1_t0_ntsc(),
        Territory::Export,
    )
    .expect("direct-boot SMS machine should initialize");

    let initial = machine.export_state();
    assert_eq!(initial.cpu.registers.pc, 0x0000);
    assert_eq!(initial.cpu.registers.sp, 0x0000);
    assert_eq!(machine.memory().control_port(), 0xAB);
    assert_eq!(machine.memory().selected_banks(), [0, 1, 2]);

    machine
        .step_complete_instruction()
        .expect("reset-vector JP should complete through the Scheduler");
    assert_eq!(machine.export_state().cpu.registers.pc, 0x001C);

    machine
        .step_complete_instruction()
        .expect("CALL should complete through the Scheduler");

    let after_call = machine.export_state();
    assert_eq!(after_call.cpu.registers.pc, 0x02B2);
    assert_eq!(after_call.cpu.registers.sp, 0xFFFE);
    assert_eq!(machine.memory().read(0xFFFE), 0x1F);
    assert_eq!(machine.memory().read(0xFFFF), 0x00);
    assert_eq!(
        machine.memory().selected_banks(),
        [0, 0x1F, 0],
        "CALL from SP=0 pushes its return address through the Sega mapper registers"
    );

    machine
        .step_complete_instruction()
        .expect("RET should restore the return address from mirrored work RAM");
    assert_eq!(machine.export_state().cpu.registers.pc, 0x001F);
    assert_eq!(machine.export_state().cpu.registers.sp, 0x0000);

    machine
        .step_complete_instruction()
        .expect("the later LD SP should complete normally");
    assert_eq!(machine.export_state().cpu.registers.sp, 0xDFF0);
    assert_eq!(
        machine.memory().selected_banks(),
        [0, 0x1F, 0],
        "initializing SP after CALL does not undo mapper writes already performed"
    );
}
