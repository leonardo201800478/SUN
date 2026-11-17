//! F-023 independent MEMPTR observation using Mark Woodmass' 2008 ZX Spectrum suite.
//!
//! The TAP fixture is GPL-2.0 and is fetched by CI only (not committed). The
//! original 16K Spectrum 48K ROM is also fetched by CI only from a pinned source
//! blob; its Amstrad copyright notice and redistribution permission are retained.
//! Expected result metadata is cross-checked against redcode/Z80
//! sources/test-Z80.c at commit 266f460fea6b07096404c8e8170b65581dfe6eca.
//!
//! The test reproduces the upstream runner's Spectrum hooks: CHAN_OPEN
//! (opcode-fetch override at 0x1601), CLS (RET at 0x0D6B), and PRINT (RST 10h
//! hook at 0x70F2). ROM writes are ignored, and the original ROM supplies reads.

use std::fs;

use sega_z80_cpu::{Z80, bus::CpuBus};

const TAP_SIZE: usize = 5_573;
const TAP_CODE_OFFSET: usize = 120;
const TAP_CODE_SIZE: usize = 5_452;
const LOAD_ADDRESS: usize = 0x8000;
const START_ADDRESS: u16 = 0x8049;
const EXIT_ADDRESS: u16 = 0x80E6;

const CHAN_OPEN: u16 = 0x1601;
const CLS: u16 = 0x0D6B;
const PRINT_VECTOR: u16 = 0x0010;
const PRINT_HOOK: u16 = 0x70F2;

const EXPECTED_TSTATES: u64 = 50_904_931;
const MAX_TSTATES: u64 = EXPECTED_TSTATES + 2_000_000;
const EXPECTED_FNV1: u32 = 0xF5AE_5140;
const EXPECTED_LINES: usize = 61;
const EXPECTED_COLUMNS: usize = 32;

const FLAG_PRINTABLE_MIN: u8 = 32;
const FLAG_PRINTABLE_MAX: u8 = 126;

struct SpectrumTestBus {
    memory: Box<[u8; 0x1_0000]>,
    opcode_fetch_override: Option<u16>,
}

impl CpuBus for SpectrumTestBus {
    fn read_memory(&mut self, address: u16) -> u8 {
        if self.opcode_fetch_override == Some(address) {
            return 0xC9; // RET, matching the upstream fetch_opcode hook.
        }

        self.memory[usize::from(address)]
    }

    fn write_memory(&mut self, address: u16, value: u8) {
        // Match the upstream ZX Spectrum test runner: ROM writes are ignored.
        if address >= 0x4000 {
            self.memory[usize::from(address)] = value;
        }
    }

    fn read_port(&mut self, port: u16) -> u8 {
        // Match redcode/Z80 test-Z80.c defaults: even ports 0xBF, odd ports 0xFF.
        if port & 1 == 0 { 0xBF } else { 0xFF }
    }
}

#[derive(Debug)]
struct PrintCapture {
    fnv1: u32,
    lines: usize,
    cursor_x: usize,
    columns: usize,
    tab_remaining: u8,
    bad_character: bool,
    output: Vec<u8>,
}

impl Default for PrintCapture {
    fn default() -> Self {
        Self {
            fnv1: 0x811C_9DC5,
            lines: 0,
            cursor_x: 0,
            columns: 0,
            tab_remaining: 0,
            bad_character: false,
            output: Vec::new(),
        }
    }
}

impl PrintCapture {
    fn line_feed(&mut self) {
        self.columns = self.columns.max(self.cursor_x);
        self.cursor_x = 0;
        self.lines += 1;
    }

    fn write_character(&mut self, character: u8) {
        // The upstream runner uses FNV-1 (multiply, then XOR), not FNV-1a.
        self.fnv1 = self.fnv1.wrapping_mul(0x0100_0193) ^ u32::from(character);
        self.output.push(character);

        if self.tab_remaining == 0 {
            match character {
                0x0D => self.line_feed(),
                0x17 => self.tab_remaining = 2,
                0x7F => self.cursor_x += 1,
                FLAG_PRINTABLE_MIN..=FLAG_PRINTABLE_MAX => self.cursor_x += 1,
                _ => self.bad_character = true,
            }
        } else {
            self.tab_remaining -= 1;
            if self.tab_remaining != 0 {
                let target = usize::from(character & 0x1F);
                let current = self.cursor_x & 0x1F;
                if target < current {
                    self.line_feed();
                } else {
                    self.cursor_x += target - current;
                }
            }
        }
    }

    fn finish(&mut self) {
        self.columns = self.columns.max(self.cursor_x);
    }
}

fn output_contains_row(output: &[u8], mnemonic: &[u8], result: &[u8]) -> bool {
    let mut expected = Vec::with_capacity(mnemonic.len() + 3 + result.len());
    expected.extend_from_slice(mnemonic);
    expected.extend_from_slice(&[0x17, 0x13, 0x00]); // Spectrum TAB/INK/PAPER controls.
    expected.extend_from_slice(result);
    output
        .windows(expected.len())
        .any(|window| window == expected)
}

#[test]
#[ignore = "requires the pinned Mark Woodmass Z80 Test Suite TAP downloaded by CI"]
fn woodmass_memptr_tap_matches_hardware_reference() {
    let tap_path = std::env::var_os("Z80_WOODMASS_TAP")
        .expect("CI must set Z80_WOODMASS_TAP to the verified TAP fixture");
    let tap = fs::read(tap_path).expect("the verified Woodmass TAP fixture is readable");

    assert_eq!(tap.len(), TAP_SIZE, "Woodmass TAP file size");
    let code_end = TAP_CODE_OFFSET + TAP_CODE_SIZE;
    let code = tap
        .get(TAP_CODE_OFFSET..code_end)
        .expect("the pinned machine-code range exists in the TAP");

    let rom_path = std::env::var_os("Z80_SPECTRUM_ROM")
        .expect("CI must set Z80_SPECTRUM_ROM to the verified original 48K ROM");
    let rom = fs::read(rom_path).expect("the verified Spectrum 48K ROM is readable");
    assert_eq!(rom.len(), 0x4000, "original Spectrum 48K ROM size");

    let mut bus = SpectrumTestBus {
        memory: Box::new([0; 0x1_0000]),
        opcode_fetch_override: None,
    };
    bus.memory[..0x4000].copy_from_slice(&rom);
    bus.memory[LOAD_ADDRESS..LOAD_ADDRESS + TAP_CODE_SIZE].copy_from_slice(code);

    // Woodmass runner setup: RST 10h -> JP 0x70F2 -> PRINT hook (RET).
    bus.memory[usize::from(PRINT_VECTOR)] = 0xC3; // JP PRINT_HOOK
    bus.memory[usize::from(PRINT_VECTOR) + 1] = PRINT_HOOK as u8;
    bus.memory[usize::from(PRINT_VECTOR) + 2] = (PRINT_HOOK >> 8) as u8;
    bus.memory[usize::from(PRINT_HOOK)] = 0xC9; // Z80_HOOK callback returns RET.
    bus.memory[usize::from(CLS)] = 0xC9; // RET
    bus.memory[usize::from(EXIT_ADDRESS)] = 0x76; // HALT, as in test-Z80.c.

    let mut cpu = Z80::default();
    cpu.registers.pc = START_ADDRESS;
    cpu.registers.sp = 0x7FE8;
    cpu.registers.a = 0x32;
    cpu.registers.f = 0x22;
    cpu.i = 0x3F;

    let mut printed = PrintCapture::default();
    let mut halt_pc = None;
    let mut cycles_by_pc = vec![0_u64; 0x1_0000];
    let mut steps_by_pc = vec![0_u64; 0x1_0000];

    while !cpu.halted {
        let pc_before = cpu.registers.pc;

        if pc_before == PRINT_HOOK {
            printed.write_character(cpu.registers.a);
        }

        bus.opcode_fetch_override = (pc_before == CHAN_OPEN).then_some(CHAN_OPEN);
        let step_cycles = cpu.step(&mut bus).unwrap_or_else(|error| {
            panic!(
                "Woodmass MEMPTR test hit unsupported opcode at PC={pc_before:04X}, T={}, error={error:?}",
                cpu.total_tstates
            )
        });
        bus.opcode_fetch_override = None;
        cycles_by_pc[usize::from(pc_before)] += u64::from(step_cycles);
        steps_by_pc[usize::from(pc_before)] += 1;

        if pc_before == EXIT_ADDRESS {
            halt_pc = Some(pc_before);
        }

        assert!(
            cpu.total_tstates <= MAX_TSTATES,
            "Woodmass test did not reach HALT within {MAX_TSTATES} T-states; PC={:04X}, last output={:?}",
            cpu.registers.pc,
            String::from_utf8_lossy(&printed.output[printed.output.len().saturating_sub(256)..])
        );
    }

    printed.finish();

    // The complete screen hash remains the global oracle. These two explicit
    // rows make the RLD/RRD MEMPTR-related results individually auditable.
    assert!(
        output_contains_row(&printed.output, b"RLD", b": 117F passed"),
        "Woodmass MEMPTR RLD row missing or mismatched"
    );
    assert!(
        output_contains_row(&printed.output, b"RRD", b": 34AE passed"),
        "Woodmass MEMPTR RRD row missing or mismatched"
    );

    let mut hot_pcs: Vec<(u64, u16, u64, [u8; 4])> = (0..0x1_0000)
        .filter(|&pc| cycles_by_pc[pc] != 0)
        .map(|pc| {
            let bytes = [
                bus.memory[pc],
                bus.memory[(pc + 1) & 0xFFFF],
                bus.memory[(pc + 2) & 0xFFFF],
                bus.memory[(pc + 3) & 0xFFFF],
            ];
            (cycles_by_pc[pc], pc as u16, steps_by_pc[pc], bytes)
        })
        .collect();
    hot_pcs.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    hot_pcs.truncate(16);

    assert_eq!(
        halt_pc,
        Some(EXIT_ADDRESS),
        "test must exit at its pinned HALT"
    );
    assert_eq!(
        printed.fnv1,
        EXPECTED_FNV1,
        "hardware-reference output hash; bytes={}, output:\n{}",
        printed.output.len(),
        String::from_utf8_lossy(&printed.output)
    );
    assert_eq!(
        printed.lines, EXPECTED_LINES,
        "hardware-reference output lines"
    );
    assert_eq!(
        printed.columns, EXPECTED_COLUMNS,
        "hardware-reference output columns"
    );
    assert!(
        !printed.bad_character,
        "test printed an unsupported control character"
    );
    assert_eq!(
        cpu.total_tstates,
        EXPECTED_TSTATES,
        "hardware-reference T-states; output bytes={}, hot PCs={hot_pcs:?}",
        printed.output.len()
    );
}
