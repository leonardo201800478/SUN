//! Execute the hardware-derived Z80 Block Instruction Flags Test v5.0.
//!
//! Fixture provenance: Kevin Watkins, ZXSpectrumNextTests, MIT License.
//! The SNA is fetched by CI from pinned commit 8adcbc8384b2311fb6d36d424478aef583d5c307
//! and verified against its Git blob SHA before this ignored test is run.
//! This harness models a Spectrum 48K 50 Hz interrupt cadence and the ROM
//! print entry points used only for presentation by the test program.

use std::{env, fs};

use sega_z80_cpu::{Z80, Z80AlternateRegisters, Z80ArchitecturalState, Z80Registers, bus::CpuBus};

const SNA_SIZE: usize = 49_179;
const FRAME_TSTATES: u64 = 69_888;
const INT_PULSE_TSTATES: u64 = 32;
const MAX_TSTATES: u64 = 50_000_000;
const MAX_STEPS: u64 = 12_000_000;
const LOG_AREA: usize = 0x8900;
const B_LOOP_ITERATIONS: usize = 65;
const B_LOOP_GROUPS: [[u8; 4]; 6] = [*b"OTIR", *b"OTIR", *b"OTDR", *b"OTDR", *b"INIR", *b"INDR"];

struct Spectrum48Bus {
    memory: Box<[u8; 0x1_0000]>,
    output: Vec<u8>,
}

impl Default for Spectrum48Bus {
    fn default() -> Self {
        Self {
            memory: Box::new([0; 0x1_0000]),
            output: Vec::new(),
        }
    }
}

impl CpuBus for Spectrum48Bus {
    fn read_memory(&mut self, address: u16) -> u8 {
        self.memory[usize::from(address)]
    }

    fn write_memory(&mut self, address: u16, value: u8) {
        self.memory[usize::from(address)] = value;
    }

    fn read_port(&mut self, port: u16) -> u8 {
        match port as u8 {
            0xFE => 0x80, // ULA keyboard row with no key pressed: bit 7 high.
            0x1F => 0x00, // Kempston joystick absent: bit 7 low.
            0xFF => 0xFF, // Floating-bus value expected by this test setup.
            _ => 0xFF,
        }
    }

    fn write_port(&mut self, _port: u16, _value: u8) {}
}

fn word_at(memory: &[u8; 0x1_0000], address: u16) -> u16 {
    u16::from_le_bytes([
        memory[usize::from(address)],
        memory[usize::from(address.wrapping_add(1))],
    ])
}

fn word_in_header(header: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([header[offset], header[offset + 1]])
}

fn load_sna(bytes: &[u8]) -> (Z80, Spectrum48Bus, u16, u16) {
    assert_eq!(bytes.len(), SNA_SIZE, "unexpected 48K SNA length");

    let mut bus = Spectrum48Bus::default();
    bus.memory[0x4000..].copy_from_slice(&bytes[27..]);

    // In a 48K SNA the PC is stored on the stack at the SP in the header.
    let snapshot_sp = word_in_header(bytes, 23);
    let entry_pc = word_at(&bus.memory, snapshot_sp);
    let entry_sp = snapshot_sp.wrapping_add(2);
    let caller_return_pc = word_at(&bus.memory, entry_sp);
    let exit_sp = entry_sp.wrapping_add(2);

    let iff = bytes[19] & 0x04 != 0;
    let state = Z80ArchitecturalState {
        registers: Z80Registers {
            a: bytes[22],
            f: bytes[21],
            b: bytes[14],
            c: bytes[13],
            d: bytes[12],
            e: bytes[11],
            h: bytes[10],
            l: bytes[9],
            sp: entry_sp,
            pc: entry_pc,
        },
        alternate: Z80AlternateRegisters {
            a: bytes[8],
            f: bytes[7],
            b: bytes[6],
            c: bytes[5],
            d: bytes[4],
            e: bytes[3],
            h: bytes[2],
            l: bytes[1],
        },
        iy: word_in_header(bytes, 15),
        ix: word_in_header(bytes, 17),
        i: bytes[0],
        r: bytes[20],
        iff1: iff,
        iff2: iff,
        interrupt_mode: bytes[25],
        wz: 0,
        ei_delay: 0,
        nmi_pending: false,
        int_pending: None,
        q: 0,
        q_previous: 0,
        p: false,
        halted: false,
    };

    let mut cpu = Z80::default();
    cpu.import_architectural_state(state)
        .expect("SNA architectural state imports");
    cpu.total_tstates = 0;

    (cpu, bus, caller_return_pc, exit_sp)
}

fn return_from_rom(cpu: &mut Z80, bus: &Spectrum48Bus) {
    let sp = cpu.registers.sp;
    cpu.registers.pc = word_at(&bus.memory, sp);
    cpu.registers.sp = sp.wrapping_add(2);
}

fn handle_rom_entry(cpu: &mut Z80, bus: &mut Spectrum48Bus) -> bool {
    match cpu.registers.pc {
        0x0010 => {
            let character = cpu.registers.a;
            if character == b'\r' {
                bus.output.push(b'\n');
            } else if character >= 0x20 {
                bus.output.push(character);
            }
            return_from_rom(cpu, bus);
            true
        }
        0x0DAF => {
            // ROM_CLS is used only to clear the display before the report.
            return_from_rom(cpu, bus);
            true
        }
        0x203C => {
            // ROM_PRINT is used only to print the four-byte instruction name.
            let mut address = (u16::from(cpu.registers.d) << 8) | u16::from(cpu.registers.e);
            let count =
                (((u16::from(cpu.registers.b) << 8) | u16::from(cpu.registers.c)).min(32)) as usize;
            for _ in 0..count {
                let character = bus.memory[usize::from(address)];
                if character == 0xFF {
                    break;
                }
                bus.output.push(character);
                address = address.wrapping_add(1);
            }
            return_from_rom(cpu, bus);
            true
        }
        _ => false,
    }
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn validate_main_results(output: &[u8]) {
    assert!(
        !output.contains(&b'!'),
        "hardware test reported at least one flag mismatch: {}",
        String::from_utf8_lossy(output)
    );
    assert!(
        !contains_bytes(output, b"failed delay calibration"),
        "Spectrum 48K interrupt timing did not satisfy ROM calibration"
    );
    assert!(
        !contains_bytes(output, b"unexpected IN"),
        "Spectrum port probe rejected the emulated ULA/Kempston/floating-bus values"
    );

    let comparisons = output.iter().filter(|&&byte| byte == b'=').count();
    assert!(
        comparisons >= 60,
        "expected at least 60 main instruction/PC comparisons, captured {comparisons}"
    );
}

fn validate_b_loop_log(memory: &[u8; 0x1_0000]) {
    let mut cursor = LOG_AREA;

    for group in B_LOOP_GROUPS {
        assert_eq!(
            &memory[cursor..cursor + 4],
            &group,
            "unexpected B-loop log group at {cursor:04X}"
        );
        cursor += 4;

        for iteration in 0..B_LOOP_ITERATIONS {
            let b_after = memory[cursor];
            let expected_flags = memory[cursor + 1];
            let actual_flags = memory[cursor + 2];
            let expected_b = 15_u8 + (iteration as u8) * 3;

            assert_eq!(b_after, expected_b, "B-loop B value at {cursor:04X}");
            assert_eq!(
                actual_flags,
                expected_flags,
                "B-loop flags for {} at B={b_after}",
                String::from_utf8_lossy(&group)
            );
            cursor += 3;
        }
    }

    assert_eq!(memory[cursor], 0xFF, "final B-loop log sentinel");
}

#[test]
#[ignore = "requires pinned Z80 Block Instruction Flags Test v5.0 SNA downloaded by CI"]
fn z80_block_instruction_flags_test_v5_passes_on_spectrum48_timing() {
    let fixture = env::var("SEGA_Z80_BLOCK_FLAGS_SNA")
        .expect("CI must set SEGA_Z80_BLOCK_FLAGS_SNA to the pinned SNA fixture path");
    let bytes = fs::read(&fixture).expect("pinned Z80 block-flags SNA is readable");
    let (mut cpu, mut bus, caller_return_pc, exit_sp) = load_sna(&bytes);

    let mut next_interrupt = FRAME_TSTATES;
    let mut pulse_end = None;
    let mut completed = false;

    for step_index in 0..MAX_STEPS {
        let now = cpu.total_tstates;

        if pulse_end.is_some_and(|end| now >= end) {
            if cpu.int_pending == Some(0xFF) {
                cpu.int_pending = None;
            }
            pulse_end = None;
        }

        if pulse_end.is_none() && now >= next_interrupt {
            cpu.int_pending = Some(0xFF);
            pulse_end = Some(next_interrupt + INT_PULSE_TSTATES);
            next_interrupt = next_interrupt.saturating_add(FRAME_TSTATES);
        }

        if handle_rom_entry(&mut cpu, &mut bus) {
            continue;
        }

        if cpu.registers.pc == caller_return_pc && cpu.registers.sp == exit_sp {
            validate_main_results(&bus.output);
            validate_b_loop_log(&bus.memory);
            completed = true;
            break;
        }

        assert!(
            cpu.registers.pc >= 0x4000,
            "unexpected Spectrum ROM execution at PC={:04X}, SP={:04X}, after {step_index} steps",
            cpu.registers.pc,
            cpu.registers.sp
        );
        assert!(
            cpu.total_tstates <= MAX_TSTATES,
            "Z80 block-flags ROM exceeded {MAX_TSTATES} T-states at PC={:04X}",
            cpu.registers.pc
        );

        cpu.step(&mut bus).unwrap_or_else(|error| {
            panic!(
                "Z80 block-flags ROM execution failed at PC={:04X}: {error:?}",
                cpu.registers.pc
            )
        });
    }

    assert!(
        completed,
        "Z80 block-flags ROM did not return after {MAX_STEPS} instructions / {MAX_TSTATES} T-states; PC={:04X}, SP={:04X}",
        cpu.registers.pc, cpu.registers.sp
    );
}
