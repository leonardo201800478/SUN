//! F-023 independent MEMPTR observation using Mark Woodmass' 2008 ZX Spectrum suite.
//!
//! The TAP fixture is GPL-2.0 and is fetched by CI only (not committed). Its
//! SHA3-512, size, load offset, and expected run metadata are pinned below.
//! Expected result metadata is cross-checked against redcode/Z80
//! sources/test-Z80.c at commit 266f460fea6b07096404c8e8170b65581dfe6eca.
//!
//! The test uses only the Spectrum ROM entry points stubbed by that runner:
//! CHAN_OPEN (opcode-fetch override at 0x1601), CLS (RET at 0x0D6B), and PRINT
//! (RST 10h -> 0x70F2 hook). Any other execution/read from ROM is reported,
//! avoiding a copyrighted ROM dependency.

use std::collections::BTreeSet;
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
    rom_reads: BTreeSet<u16>,
}

impl CpuBus for SpectrumTestBus {
    fn read_memory(&mut self, address: u16) -> u8 {
        if address < 0x4000 {
            self.rom_reads.insert(address);
        }

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

    let mut bus = SpectrumTestBus {
        memory: Box::new([0; 0x1_0000]),
        opcode_fetch_override: None,
        rom_reads: BTreeSet::new(),
    };
    bus.memory[LOAD_ADDRESS..LOAD_ADDRESS + TAP_CODE_SIZE].copy_from_slice(code);

    // Spectrum ROM services installed by the upstream test runner.
    bus.memory[usize::from(PRINT_VECTOR)] = 0xC3; // JP PRINT_HOOK
    bus.memory[usize::from(PRINT_VECTOR) + 1] = PRINT_HOOK as u8;
    bus.memory[usize::from(PRINT_VECTOR) + 2] = (PRINT_HOOK >> 8) as u8;
    bus.memory[usize::from(CLS)] = 0xC9; // RET
    bus.memory[usize::from(PRINT_HOOK)] = 0xC9; // Hook returns as RET.
    bus.memory[usize::from(EXIT_ADDRESS)] = 0x76; // HALT, as in test-Z80.c.

    let mut cpu = Z80::default();
    cpu.registers.pc = START_ADDRESS;
    cpu.registers.sp = 0x7FE8;
    cpu.registers.a = 0x32;
    cpu.registers.f = 0x22;
    cpu.i = 0x3F;

    let mut printed = PrintCapture::default();
    let mut halt_pc = None;

    while !cpu.halted {
        let pc_before = cpu.registers.pc;

        if pc_before < 0x4000 && !matches!(pc_before, PRINT_VECTOR | CLS | CHAN_OPEN) {
            panic!(
                "unexpected Spectrum ROM execution at PC={pc_before:04X}, T={}",
                cpu.total_tstates
            );
        }

        if pc_before == PRINT_HOOK {
            printed.write_character(cpu.registers.a);
        }

        bus.opcode_fetch_override = (pc_before == CHAN_OPEN).then_some(CHAN_OPEN);
        cpu.step(&mut bus).unwrap_or_else(|error| {
            panic!(
                "Woodmass MEMPTR test hit unsupported opcode at PC={pc_before:04X},                  T={}, error={error:?}",
                cpu.total_tstates
            )
        });
        bus.opcode_fetch_override = None;

        if pc_before == EXIT_ADDRESS {
            halt_pc = Some(pc_before);
        }

        assert!(
            cpu.total_tstates <= MAX_TSTATES,
            "Woodmass test did not reach HALT within {MAX_TSTATES} T-states;              PC={:04X}, last output={:?}",
            cpu.registers.pc,
            String::from_utf8_lossy(&printed.output[printed.output.len().saturating_sub(256)..])
        );
    }

    printed.finish();

    assert_eq!(
        halt_pc,
        Some(EXIT_ADDRESS),
        "test must exit at its pinned HALT"
    );
    assert_eq!(
        cpu.total_tstates, EXPECTED_TSTATES,
        "hardware-reference T-states"
    );
    assert_eq!(
        printed.fnv1, EXPECTED_FNV1,
        "hardware-reference output hash"
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

    let allowed_rom_reads = BTreeSet::from([
        PRINT_VECTOR,
        PRINT_VECTOR + 1,
        PRINT_VECTOR + 2,
        CLS,
        CHAN_OPEN,
    ]);
    let unexpected_rom_reads: Vec<u16> = bus
        .rom_reads
        .difference(&allowed_rom_reads)
        .copied()
        .collect();
    assert!(
        unexpected_rom_reads.is_empty(),
        "the ROM-free harness unexpectedly depended on other ROM addresses:          {unexpected_rom_reads:?}"
    );
}
