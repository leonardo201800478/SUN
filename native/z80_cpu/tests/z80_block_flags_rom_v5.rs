//! Execute the hardware-derived Z80 Block Instruction Flags Test v5.0.
//!
//! Fixture provenance: Kevin Watkins, ZXSpectrumNextTests, MIT License.
//! The SNA is fetched by CI from pinned commit 8adcbc8384b2311fb6d36d424478aef583d5c307
//! and verified against its Git blob SHA before this ignored test is run.
//! This harness models a Spectrum 48K 50 Hz interrupt cadence and the ROM
//! print entry points used only for presentation by the test program.

use std::{collections::VecDeque, env, fs};

use sega_z80_cpu::{Z80, Z80AlternateRegisters, Z80ArchitecturalState, Z80Registers, bus::CpuBus};

const SNA_SIZE: usize = 49_179;
const FRAME_TSTATES: u64 = 69_888;
const INT_PULSE_TSTATES: u64 = 32;
// The ROM's six B-loop groups run 65 frame-synchronized cases each. Every
// case includes a calibrated ~3,500-iteration LDIR delay (~73,500 T-states),
// so the previous 50M budget stopped before the finite test sequence ended.
// 80M leaves margin over the estimated ~61M T-state full run.
const MAX_TSTATES: u64 = 80_000_000;
const MAX_STEPS: u64 = 20_000_000;
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

fn append_spectrum_print_character(output: &mut Vec<u8>, character: u8) {
    if character == b'\r' {
        output.push(b'\n');
    } else if character >= 0x20 {
        output.push(character);
    }
}

fn handle_rom_entry(cpu: &mut Z80, bus: &mut Spectrum48Bus) -> bool {
    match cpu.registers.pc {
        0x0010 => {
            append_spectrum_print_character(&mut bus.output, cpu.registers.a);
            return_from_rom(cpu, bus);
            true
        }
        0x0DAF => {
            // ROM_CLS is used only to clear the display before the report.
            return_from_rom(cpu, bus);
            true
        }
        0x203C => {
            // ZX Spectrum PR-STRING consumes BC bytes from (DE), advances DE,
            // and returns with BC=FFFF, A=0 and Z/PV set by the final OR C.
            let count = cpu.registers.bc();
            let mut address = cpu.registers.de();
            for _ in 0..count {
                let character = bus.memory[usize::from(address)];
                append_spectrum_print_character(&mut bus.output, character);
                address = address.wrapping_add(1);
            }
            cpu.registers.set_de(address);
            cpu.registers.set_bc(u16::MAX);
            cpu.registers.a = 0;
            cpu.registers.f = 0x44;
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

fn validate_b_loop_log(
    memory: &[u8; 0x1_0000],
    block_interrupts: &[String],
    block_instruction_steps: &[String],
    block_delay_summary: &[String],
) {
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

            assert_eq!(
                b_after, expected_b,
                "B-loop B value at {cursor:04X}; delay-loop timing: {block_delay_summary:#?}; first block-I/O instruction steps: {block_instruction_steps:#?}; first block-I/O INT accepts: {block_interrupts:#?}"
            );
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
fn rom_print_stub_matches_spectrum_pr_string_register_side_effects() {
    let mut cpu = Z80::default();
    let mut bus = Spectrum48Bus::default();
    cpu.registers.pc = 0x203C;
    cpu.registers.sp = 0x9000;
    cpu.registers.set_bc(4);
    cpu.registers.set_de(0x8000);
    bus.memory[0x8000..0x8004].copy_from_slice(b"LDIR");
    bus.memory[0x9000..0x9002].copy_from_slice(&0x3456_u16.to_le_bytes());

    assert!(handle_rom_entry(&mut cpu, &mut bus));

    assert_eq!(bus.output, b"LDIR");
    assert_eq!(cpu.registers.de(), 0x8004);
    assert_eq!(cpu.registers.bc(), u16::MAX);
    assert_eq!(cpu.registers.a, 0);
    assert_eq!(cpu.registers.f, 0x44);
    assert_eq!(cpu.registers.pc, 0x3456);
    assert_eq!(cpu.registers.sp, 0x9002);
}

#[test]
#[ignore = "requires pinned Z80 Block Instruction Flags Test v5.0 SNA downloaded by CI"]
fn z80_block_instruction_flags_test_v5_passes_on_spectrum48_timing() {
    let fixture = env::var("SEGA_Z80_BLOCK_FLAGS_SNA")
        .expect("CI must set SEGA_Z80_BLOCK_FLAGS_SNA to the pinned SNA fixture path");
    let bytes = fs::read(&fixture).expect("pinned Z80 block-flags SNA is readable");
    let (mut cpu, mut bus, caller_return_pc, exit_sp) = load_sna(&bytes);

    let initial_pc = cpu.registers.pc;
    let initial_sp = cpu.registers.sp;
    let mut next_interrupt = FRAME_TSTATES;
    let mut pulse_end = None;
    let mut interrupt_requests = 0_u64;
    let mut interrupt_accepts = 0_u64;
    let mut completed = false;
    let mut recent_steps = VecDeque::with_capacity(64);
    let mut output_progress = VecDeque::with_capacity(8);
    let mut block_interrupts = Vec::with_capacity(8);
    let mut block_instruction_steps = Vec::with_capacity(12);
    let mut block_delay_loop: Option<(u16, u64, u16, u64)> = None;
    let mut block_delay_summary = Vec::with_capacity(1);

    for step_index in 0..MAX_STEPS {
        let now = cpu.total_tstates;

        if pulse_end.is_some_and(|end| now >= end) {
            cpu.clear_int_request();
            pulse_end = None;
        }

        if pulse_end.is_none() && now >= next_interrupt {
            cpu.request_int(0xFF);
            interrupt_requests += 1;
            pulse_end = Some(next_interrupt + INT_PULSE_TSTATES);
            next_interrupt = next_interrupt.saturating_add(FRAME_TSTATES);
        }

        let rom_entry = cpu.registers.pc;
        let output_len_before = bus.output.len();
        if handle_rom_entry(&mut cpu, &mut bus) {
            if bus.output.len() > output_len_before {
                if output_progress.len() == 8 {
                    output_progress.pop_front();
                }
                output_progress.push_back(format!(
                    "t={} rom_entry={:04X} return_pc={:04X} output_len={} tail={:?}",
                    now,
                    rom_entry,
                    cpu.registers.pc,
                    bus.output.len(),
                    String::from_utf8_lossy(&bus.output[bus.output.len().saturating_sub(120)..])
                ));
            }
            continue;
        }

        if cpu.registers.pc == caller_return_pc && cpu.registers.sp == exit_sp {
            validate_main_results(&bus.output);
            validate_b_loop_log(
                &bus.memory,
                &block_interrupts,
                &block_instruction_steps,
                &block_delay_summary,
            );
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
            "Z80 block-flags ROM exceeded {MAX_TSTATES} T-states after {step_index} instructions: PC={:04X} SP={:04X} AF={:02X}{:02X} BC={:02X}{:02X} DE={:02X}{:02X} HL={:02X}{:02X} I={:02X} R={:02X} WZ={:04X} IFF1={} IFF2={} IM={} HALT={} INT={:?} caller={:04X} exit_sp={:04X} initial_pc={:04X} initial_sp={:04X} frames={} int_requests={} int_accepts={} output_bytes={} next_int={} pulse_end={:?} code={:02X?} output_tail={} output_progress={:?} recent_steps={:?}",
            cpu.registers.pc,
            cpu.registers.sp,
            cpu.registers.a,
            cpu.registers.f,
            cpu.registers.b,
            cpu.registers.c,
            cpu.registers.d,
            cpu.registers.e,
            cpu.registers.h,
            cpu.registers.l,
            cpu.i,
            cpu.r,
            cpu.wz,
            cpu.iff1,
            cpu.iff2,
            cpu.interrupt_mode,
            cpu.halted,
            cpu.int_pending,
            caller_return_pc,
            exit_sp,
            initial_pc,
            initial_sp,
            cpu.total_tstates / FRAME_TSTATES,
            interrupt_requests,
            interrupt_accepts,
            bus.output.len(),
            next_interrupt,
            pulse_end,
            (0..8)
                .map(|offset| bus.memory[usize::from(cpu.registers.pc.wrapping_add(offset))])
                .collect::<Vec<_>>(),
            String::from_utf8_lossy(&bus.output[bus.output.len().saturating_sub(160)..]),
            output_progress,
            recent_steps
        );

        let pc_before = cpu.registers.pc;
        let sp_before = cpu.registers.sp;
        let iff1_before = cpu.iff1;
        let int_pending_before = cpu.int_pending;
        if int_pending_before.is_some()
            && iff1_before
            && cpu.ei_delay == 0
            && block_interrupts.len() < 8
            && &bus.memory[0x8900..0x8904] == b"OTIR"
        {
            let pc = cpu.registers.pc;
            let opcode = [
                bus.memory[usize::from(pc)],
                bus.memory[usize::from(pc.wrapping_add(1))],
            ];
            if opcode == [0xED, 0xB2]
                || opcode == [0xED, 0xBA]
                || opcode == [0xED, 0xB3]
                || opcode == [0xED, 0xBB]
            {
                block_interrupts.push(format!(
                    "t={} next_int={} pulse_end={:?} pc={:04X} op={:02X?} AF={:02X}{:02X} BC={:04X} DE={:04X} HL={:04X} R={:02X}",
                    cpu.total_tstates,
                    next_interrupt,
                    pulse_end,
                    pc,
                    opcode,
                    cpu.registers.a,
                    cpu.registers.f,
                    cpu.registers.bc(),
                    cpu.registers.de(),
                    cpu.registers.hl(),
                    cpu.r
                ));
            }
        }

        let opcode: [u8; 4] = std::array::from_fn(|offset| {
            bus.memory[usize::from(pc_before.wrapping_add(offset as u16))]
        });
        if block_delay_summary.is_empty() && &bus.memory[0x8900..0x8904] == b"OTIR" {
            if opcode[0] == 0xED && opcode[1] == 0xB0 && cpu.registers.bc() > 100 && block_delay_loop.is_none() {
                let frame_edge = pulse_end
                    .map(|end| end.saturating_sub(32))
                    .unwrap_or(next_interrupt);
                block_delay_loop = Some((
                    pc_before,
                    cpu.total_tstates,
                    cpu.registers.bc(),
                    frame_edge,
                ));
            } else if let Some((ldir_pc, start_t, start_bc, frame_edge)) = block_delay_loop {
                if pc_before != ldir_pc {
                    let expected_tstates =
                        u64::from(start_bc.saturating_sub(1)) * 21 + 16;
                    block_delay_summary.push(format!(
                        "ldir_pc={:04X} start_t={} end_t={} elapsed={} expected={} start_bc={:04X} end_pc={:04X} end_bc={:04X} frame_edge={} end_to_edge={}",
                        ldir_pc,
                        start_t,
                        cpu.total_tstates,
                        cpu.total_tstates.saturating_sub(start_t),
                        expected_tstates,
                        start_bc,
                        pc_before,
                        cpu.registers.bc(),
                        frame_edge,
                        frame_edge.saturating_sub(cpu.total_tstates)
                    ));
                    block_delay_loop = None;
                }
            }
        }

        if block_instruction_steps.len() < 12
            && &bus.memory[0x8900..0x8904] == b"OTIR"
            && opcode[0] == 0xED
            && (opcode[1] == 0xB2 || opcode[1] == 0xBA || opcode[1] == 0xB3 || opcode[1] == 0xBB)
        {
            let frame_edge = pulse_end
                .map(|end| end.saturating_sub(32))
                .unwrap_or(next_interrupt);
            block_instruction_steps.push(format!(
                "t={} frame_edge={} delta={} pc={:04X} op={:02X?} BC={:04X} AF={:02X}{:02X} R={:02X} IFF1={} ei_delay={}",
                cpu.total_tstates,
                frame_edge,
                frame_edge.saturating_sub(cpu.total_tstates),
                pc_before,
                opcode,
                cpu.registers.bc(),
                cpu.registers.a,
                cpu.registers.f,
                cpu.r,
                cpu.iff1,
                cpu.ei_delay
            ));
        }

        let trace = format!(
            "t={} pc={:04X} op={:02X?} AF={:02X}{:02X} BC={:04X} DE={:04X} HL={:04X} SP={:04X} I={:02X} R={:02X} IFF1={} IFF2={} IM={} HALT={} INT={:?}",
            cpu.total_tstates,
            pc_before,
            opcode,
            cpu.registers.a,
            cpu.registers.f,
            cpu.registers.bc(),
            cpu.registers.de(),
            cpu.registers.hl(),
            sp_before,
            cpu.i,
            cpu.r,
            cpu.iff1,
            cpu.iff2,
            cpu.interrupt_mode,
            cpu.halted,
            cpu.int_pending
        );
        if recent_steps.len() == 64 {
            recent_steps.pop_front();
        }
        recent_steps.push_back(trace);

        cpu.step(&mut bus).unwrap_or_else(|error| {
            panic!(
                "Z80 block-flags ROM execution failed at PC={:04X}: {error:?}",
                cpu.registers.pc
            )
        });
        if int_pending_before.is_some()
            && iff1_before
            && cpu.int_pending.is_none()
            && cpu.registers.sp == sp_before.wrapping_sub(2)
        {
            interrupt_accepts += 1;
        }
    }

    assert!(
        completed,
        "Z80 block-flags ROM did not return after {MAX_STEPS} instructions / {MAX_TSTATES} T-states; PC={:04X}, SP={:04X}, caller={caller_return_pc:04X}, exit_sp={exit_sp:04X}, int_requests={interrupt_requests}, int_accepts={interrupt_accepts}, output_bytes={}, output_progress={output_progress:?}, recent_steps={recent_steps:?}",
        cpu.registers.pc,
        cpu.registers.sp,
        bus.output.len()
    );
}
