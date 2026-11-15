use sega_sms_core::{SmsTemporalMachine, SmsTemporalMemoryOptions, SmsVdpProfile, Territory};

#[test]
fn temporal_machine_maps_conventional_bios_in_scheduler_owned_memory() {
    let cartridge = vec![0x3C; 0x4000];
    let mut bios = vec![0xA5; 0x2000];
    bios[0] = 0xF3;
    bios[0x1FFF] = 0x7E;
    let machine = SmsTemporalMachine::new_with_hardware_profile_bios_and_sram_size(
        cartridge,
        SmsTemporalMemoryOptions {
            bios: Some(bios),
            cartridge_sram_size: None,
        },
        1,
        3_579_545,
        48_000,
        SmsVdpProfile::sms1_t0_ntsc(),
        Territory::Export,
    )
    .expect("valid conventional BIOS should initialize temporal memory");
    assert!(machine.memory().bios_enabled());
    assert_eq!(machine.memory().read(0x0000), 0xF3);
    assert_eq!(machine.memory().read(0x1FFF), 0x7E);
    assert_eq!(machine.memory().read(0x2000), 0xFF);
}

#[test]
fn temporal_machine_save_state_retains_bios_image_and_mapping_latch() {
    let machine = SmsTemporalMachine::new_with_hardware_profile_bios_and_sram_size(
        vec![0x3C; 0x4000],
        SmsTemporalMemoryOptions {
            bios: Some(vec![0xA5; 0x2000]),
            cartridge_sram_size: None,
        },
        1,
        3_579_545,
        48_000,
        SmsVdpProfile::sms1_t0_ntsc(),
        Territory::Export,
    )
    .unwrap();
    let restored = SmsTemporalMachine::restore_state(machine.export_state())
        .expect("BIOS-backed temporal snapshot should restore");
    assert!(restored.memory().bios_enabled());
    assert_eq!(restored.memory().read(0x0000), 0xA5);
    assert_eq!(restored.memory().read(0x1FFF), 0xA5);
}

#[test]
fn temporal_machine_rejects_non_8k_bios_before_scheduler_execution() {
    let result = SmsTemporalMachine::new_with_hardware_profile_bios_and_sram_size(
        vec![0x3C; 0x4000],
        SmsTemporalMemoryOptions {
            bios: Some(vec![0xA5; 0x1000]),
            cartridge_sram_size: None,
        },
        1,
        3_579_545,
        48_000,
        SmsVdpProfile::sms1_t0_ntsc(),
        Territory::Export,
    );
    assert!(result.is_err());
}

#[test]
fn temporal_machine_executes_instruction_fetch_from_bios_through_scheduler() {
    let mut cartridge = vec![0x00; 0x4000];
    cartridge[0..2].copy_from_slice(&[0x3E, 0x99]);

    let mut bios = vec![0x00; 0x2000];
    bios[0..2].copy_from_slice(&[0x3E, 0x42]);

    let mut machine = SmsTemporalMachine::new_with_hardware_profile_bios_and_sram_size(
        cartridge,
        SmsTemporalMemoryOptions {
            bios: Some(bios),
            cartridge_sram_size: None,
        },
        1,
        3_579_545,
        48_000,
        SmsVdpProfile::sms1_t0_ntsc(),
        Territory::Export,
    )
    .expect("valid conventional BIOS should initialize temporal memory");

    machine
        .step_complete_instruction()
        .expect("the BIOS instruction should execute on the scheduler timeline");

    assert_eq!(machine.cpu().registers.a, 0x42);
    assert_eq!(machine.cpu().registers.pc, 2);
    assert!(machine.memory().bios_enabled());
}
