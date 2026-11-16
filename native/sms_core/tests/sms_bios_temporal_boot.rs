//! BIOS-backed SMS temporal boot executes real bootstrap code before cartridge handoff.
//!
//! The fixture is synthetic and contains no commercial BIOS or cartridge bytes.

use sega_sms_core::{
    SmsTemporalBusError, SmsTemporalMachine, SmsTemporalMachineError, SmsVdpProfile,
    SmsVdpRevision, SpriteLimitMode, Territory, VideoStandard,
};

const PSG_CLOCK_HZ: u64 = 3_579_545;
const SAMPLE_RATE_HZ: u64 = 44_100;
const BIOS_SIZE: usize = 0x2000;

fn synthetic_boot_bios() -> Vec<u8> {
    let mut bios = vec![0x00; BIOS_SIZE];
    let program = [
        0x31, 0xF0, 0xDF, // LD SP,$DFF0
        0x3E, 0xAB, // LD A,$AB
        0x32, 0x00, 0xC0, // LD ($C000),A: selected cartridge sentinel
        0x21, 0x00, 0xC7, // LD HL,$C700
        0x36, 0x3E, // LD (HL),$3E
        0x23, // INC HL
        0x36, 0xAB, // LD (HL),$AB
        0x23, // INC HL
        0x36, 0xD3, // LD (HL),$D3
        0x23, // INC HL
        0x36, 0x3E, // LD (HL),$3E
        0x23, // INC HL
        0x36, 0xC3, // LD (HL),$C3
        0x23, // INC HL
        0x36, 0x00, // LD (HL),$00
        0x23, // INC HL
        0x36, 0x00, // LD (HL),$00
        0x21, 0x00, 0xC7, // LD HL,$C700
        0xE9, // JP (HL)
    ];
    bios[..program.len()].copy_from_slice(&program);
    bios
}

#[test]
fn bios_executes_on_scheduler_and_establishes_real_cartridge_handoff_state() {
    let mut rom = vec![0xFF; 0x4000];
    rom[..3].copy_from_slice(&[0x3E, 0x42, 0x76]); // LD A,$42; HALT

    let mut machine = SmsTemporalMachine::new_with_hardware_profile_and_bios(
        rom,
        synthetic_boot_bios(),
        1,
        PSG_CLOCK_HZ,
        SAMPLE_RATE_HZ,
        SmsVdpProfile::sms1_t0_ntsc(),
        Territory::Export,
    )
    .expect("BIOS-backed SMS machine should initialize");

    assert_eq!(machine.cpu().registers.pc, 0x0000);
    assert_eq!(machine.cpu().registers.sp, 0x0000);
    assert!(machine.memory().bios_enabled());
    assert_eq!(machine.memory().read(0x0000), 0x31);
    assert_eq!(machine.memory().read(0xC000), 0x00);

    let mut cartridge_handoff_seen = false;
    for _ in 0..64 {
        if !machine.memory().bios_enabled() && machine.cpu().registers.pc == 0x0000 {
            cartridge_handoff_seen = true;
            break;
        }
        machine
            .step_complete_instruction()
            .expect("BIOS instruction should complete through the MachineScheduler");
    }

    assert!(
        cartridge_handoff_seen,
        "BIOS must hand control to cartridge ROM"
    );
    assert_eq!(machine.cpu().registers.sp, 0xDFF0);
    assert_eq!(machine.memory().control_port(), 0xAB);
    assert_eq!(machine.memory().read(0xC000), 0xAB);
    assert_eq!(machine.memory().selected_banks(), [0, 1, 2]);
    assert!(!machine.memory().bios_enabled());
    assert_eq!(machine.memory().read(0x0000), 0x3E);

    machine
        .step_complete_instruction()
        .expect("first cartridge instruction should execute after BIOS handoff");
    assert_eq!(machine.cpu().registers.a, 0x42);
    assert_eq!(machine.cpu().registers.pc, 0x0002);
}

#[test]
fn conventional_bios_rejects_sms2_asic_until_slot_arbitration_is_modeled() {
    let sms2_profile = SmsVdpProfile::new(
        SmsVdpRevision::Sega3155246,
        VideoStandard::Ntsc,
        SpriteLimitMode::HardwareLimit,
    );
    let result = SmsTemporalMachine::new_with_hardware_profile_and_bios(
        vec![0x00; 0x4000],
        vec![0xA5; BIOS_SIZE],
        1,
        PSG_CLOCK_HZ,
        SAMPLE_RATE_HZ,
        sms2_profile,
        Territory::Export,
    );

    assert!(matches!(
        result,
        Err(SmsTemporalMachineError::Bus(
            SmsTemporalBusError::UnsupportedBiosHardwareProfile
        ))
    ));
}
