//! CPU-visible SMS1 sprite collision and overflow integration.
//!
//! The fixtures program SAT and pattern data through Z80 OUT instructions,
//! let the MachineScheduler advance the raster, then observe status through
//! Z80 IN A,($BF). No direct VDP mutation or status inspection is used.

use sega_sms_core::{SmsTemporalMachine, SmsVdpProfile, Territory};

const ROM_SIZE: usize = 0x8000;
const PSG_CLOCK_HZ: u64 = 3_579_545;
const SAMPLE_RATE_HZ: u64 = 44_100;
const STATUS_ADDRESS: u16 = 0xC000;

fn emit_port_write(program: &mut Vec<u8>, port: u8, value: u8) {
    // LD A,n ; OUT (n),A
    program.extend_from_slice(&[0x3E, value, 0xD3, port]);
}

fn emit_vdp_register(program: &mut Vec<u8>, index: u8, value: u8) {
    emit_port_write(program, 0xBF, value);
    emit_port_write(program, 0xBF, 0x80 | index);
}

fn emit_vram_write(program: &mut Vec<u8>, address: u16, bytes: &[u8]) {
    emit_port_write(program, 0xBF, address as u8);
    emit_port_write(program, 0xBF, ((address >> 8) as u8 & 0x3F) | 0x40);
    for byte in bytes {
        emit_port_write(program, 0xBE, *byte);
    }
}

fn sprite_status_rom(overflow: bool) -> Vec<u8> {
    let mut program = Vec::new();
    // Mode 4, display enabled, name table at $3800, SAT at $3F00.
    emit_vdp_register(&mut program, 0, 0x04);
    emit_vdp_register(&mut program, 1, 0x40);
    emit_vdp_register(&mut program, 2, 0x0E);
    emit_vdp_register(&mut program, 5, 0x7F);

    // Place the sprites on output line 21 (raw Y=20). The tenth Y byte
    // terminates the SAT list after nine entries in the overflow fixture.
    let mut y_table = vec![0xD0; 0x0A];
    let sprite_count = if overflow { 9 } else { 2 };
    y_table[..sprite_count].fill(20);
    emit_vram_write(&mut program, 0x3F00, &y_table);

    let mut attributes = vec![0; 0x14];
    if overflow {
        for index in 0..9 {
            attributes[index * 2] = (index * 28) as u8;
            attributes[index * 2 + 1] = 1;
        }
    } else {
        // Two opaque sprites at the same X coordinate must collide.
        attributes[0] = 40;
        attributes[1] = 1;
        attributes[2] = 40;
        attributes[3] = 2;
    }
    emit_vram_write(&mut program, 0x3F80, &attributes);

    // Tile 1: opaque color 1 on row zero. Tile 2: opaque color 2 on row zero.
    emit_vram_write(&mut program, 0x0020, &[0xFF, 0x00, 0x00, 0x00]);
    if !overflow {
        emit_vram_write(&mut program, 0x0040, &[0x00, 0xFF, 0x00, 0x00]);
    }

    // Poll the CPU-visible status port until the selected sticky flag appears.
    // IN A,($BF); LD ($C000),A; AND mask; JR Z,loop; HALT.
    let mask = if overflow { 0x40 } else { 0x20 };
    program.extend_from_slice(&[0xDB, 0xBF, 0x32, 0x00, 0xC0, 0xE6, mask, 0x28, 0xF7, 0x76]);

    let mut rom = vec![0; ROM_SIZE];
    rom[..program.len()].copy_from_slice(&program);
    rom
}

fn run_until_status(overflow: bool) -> u8 {
    let mut machine = SmsTemporalMachine::new_with_hardware_profile(
        sprite_status_rom(overflow),
        1,
        PSG_CLOCK_HZ,
        SAMPLE_RATE_HZ,
        SmsVdpProfile::sms1_t0_ntsc(),
        Territory::Export,
    )
    .expect("SMS1 temporal machine should initialize");

    for _ in 0..80_000 {
        let status = machine.inspect_memory(STATUS_ADDRESS);
        if status != 0 {
            return status;
        }
        machine
            .step_complete_instruction()
            .expect("Z80 instruction should complete through the MachineScheduler");
    }

    panic!(
        "timed out waiting for CPU-visible sprite status; last sample={:#04X}",
        machine.inspect_memory(STATUS_ADDRESS)
    );
}

#[test]
fn sms1_sprite_collision_is_visible_through_z80_status_port() {
    let status = run_until_status(false);
    assert_ne!(
        status & 0x20,
        0,
        "two opaque overlapping sprites must set the sticky collision bit"
    );
    assert_eq!(
        status & 0x40,
        0,
        "two sprites must not set the ninth-sprite overflow bit"
    );
}

#[test]
fn sms1_ninth_sprite_overflow_and_index_are_visible_through_z80_status_port() {
    let status = run_until_status(true);
    assert_ne!(
        status & 0x40,
        0,
        "the ninth in-range sprite must set the overflow bit"
    );
    assert_eq!(
        status & 0x1F,
        8,
        "SMS1 overflow status must report the zero-based ninth-sprite index"
    );
    assert_eq!(
        status & 0x20,
        0,
        "spatially separated sprites must not set collision"
    );
}
