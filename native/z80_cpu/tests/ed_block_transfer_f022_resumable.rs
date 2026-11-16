//! F-022: exhaustive resumable-core coverage for simple LDI/LDD/CPI/CPD.
//!
//! Expected flags are computed from the instruction contract independently
//! of the atomic executor. Documented flags follow ZiLOG UM0080; X/Y follow
//! Sean Young, The Undocumented Z80 Documented (v0.4), and remain a software
//! reference rather than a new silicon measurement. WZ/MEMPTR is intentionally
//! not promoted here: its independent provenance remains an open E7 evidence item.
//!
//! References:
//! - https://www.zilog.com/docs/z80/um0080.pdf
//! - https://datasheets.chipdb.org/Zilog/Z80/z80-documented_0.4.pdf

use sega_z80_cpu::{
    Z80AlternateRegisters, Z80ArchitecturalState, Z80BusOperation, Z80MicroSequencer,
    Z80MicroSequencerState, Z80MicroStep, Z80Registers,
};

const FLAG_C: u8 = 0x01;
const FLAG_N: u8 = 0x02;
const FLAG_PV: u8 = 0x04;
const FLAG_H: u8 = 0x10;
const FLAG_Z: u8 = 0x40;
const FLAG_S: u8 = 0x80;

fn is_transfer(opcode: u8) -> bool {
    matches!(opcode, 0xA0 | 0xA8)
}

fn direction(opcode: u8) -> i16 {
    if matches!(opcode, 0xA8 | 0xA9) { -1 } else { 1 }
}

/// Independent transcription of the documented simple block-instruction
/// flag rules. It deliberately does not call production ALU/flag helpers.
fn expected_flags(opcode: u8, initial_f: u8, accumulator: u8, value: u8, count: u16) -> u8 {
    if is_transfer(opcode) {
        let sum = accumulator.wrapping_add(value);
        (initial_f & (FLAG_S | FLAG_Z | FLAG_C))
            | if count != 0 { FLAG_PV } else { 0 }
            | (sum & 0x08)
            | ((sum & 0x02) << 4)
    } else {
        let result = accumulator.wrapping_sub(value);
        let half_borrow = (accumulator & 0x0F) < (value & 0x0F);
        let adjusted = result.wrapping_sub(u8::from(half_borrow));
        (initial_f & FLAG_C)
            | FLAG_N
            | (result & FLAG_S)
            | if result == 0 { FLAG_Z } else { 0 }
            | if half_borrow { FLAG_H } else { 0 }
            | if count != 0 { FLAG_PV } else { 0 }
            | (adjusted & 0x08)
            | ((adjusted & 0x02) << 4)
    }
}

fn initial_state(accumulator: u8, initial_f: u8, count: u16) -> Z80ArchitecturalState {
    Z80ArchitecturalState {
        registers: Z80Registers {
            a: accumulator,
            f: initial_f,
            b: (count >> 8) as u8,
            c: count as u8,
            d: 0xC1,
            e: 0x00,
            h: 0xC0,
            l: 0x00,
            sp: 0xD000,
            pc: 0x1000,
        },
        alternate: Z80AlternateRegisters::default(),
        ix: 0x1357,
        iy: 0x2468,
        i: 0x81,
        r: 0xFE,
        iff1: false,
        iff2: true,
        interrupt_mode: 1,
        wz: 0xA55A,
        ei_delay: 0,
        nmi_pending: false,
        int_pending: None,
        q: 0x28,
        q_previous: 0x5A,
        p: false,
        halted: false,
    }
}

fn round_trip(cpu: Z80MicroSequencer) -> Z80MicroSequencer {
    let encoded = cpu.export_state().encode().expect("snapshot encodes");
    let decoded = Z80MicroSequencerState::decode(&encoded).expect("snapshot decodes");
    Z80MicroSequencer::restore_state(decoded).expect("snapshot restores")
}

fn run_case(
    memory: &mut [u8; 0x1_0000],
    opcode: u8,
    accumulator: u8,
    initial_f: u8,
    value: u8,
    initial_count: u16,
    add_waits: bool,
    snapshot_each_boundary: bool,
) -> (Z80MicroSequencer, u32) {
    let source = 0xC000_u16;
    let destination = 0xC100_u16;
    memory[0x1000] = 0xED;
    memory[0x1001] = opcode;
    memory[usize::from(source)] = value;
    memory[usize::from(destination)] = 0xCC;

    let mut cpu = Z80MicroSequencer::default();
    cpu.import_architectural_state(initial_state(accumulator, initial_f, initial_count))
        .expect("initial architectural state is valid");

    let mut step = cpu.begin_instruction().expect("instruction begins");
    let mut request_index = 0_u8;
    let mut internal_tstates = 0_u8;

    loop {
        match step {
            Z80MicroStep::BusRequest(request) => {
                let (expected_operation, expected_address, expected_base_tstates) =
                    match (is_transfer(opcode), request_index) {
                        (_, 0) => (Z80BusOperation::OpcodeFetch, 0x1000, 4),
                        (_, 1) => (Z80BusOperation::OpcodeFetch, 0x1001, 4),
                        (_, 2) => (Z80BusOperation::MemoryRead, source, 3),
                        (true, 3) => (Z80BusOperation::MemoryWrite { value }, destination, 5),
                        _ => panic!("unexpected bus request {request:?} for ED {opcode:02X}"),
                    };
                assert_eq!(request.operation, expected_operation, "ED {opcode:02X}");
                assert_eq!(request.address, expected_address, "ED {opcode:02X}");
                assert_eq!(
                    request.base_tstates, expected_base_tstates,
                    "ED {opcode:02X}"
                );

                if add_waits {
                    cpu.insert_wait_tstate().expect("WAIT is accepted");
                }

                let response = match request.operation {
                    Z80BusOperation::OpcodeFetch | Z80BusOperation::MemoryRead => {
                        Some(memory[usize::from(request.address)])
                    }
                    Z80BusOperation::MemoryWrite { value } => {
                        memory[usize::from(request.address)] = value;
                        None
                    }
                    other => panic!("unexpected operation {other:?}"),
                };
                step = cpu
                    .complete_bus_cycle(request.id, response)
                    .expect("bus request completes");
                request_index += 1;

                if snapshot_each_boundary {
                    cpu = round_trip(cpu);
                }
            }
            Z80MicroStep::InternalCycle { tstates } => {
                assert_eq!(tstates, 1, "internal phases advance one T-state at a time");
                step = cpu
                    .complete_internal_cycle()
                    .expect("internal T-state completes");
                internal_tstates += 1;

                if snapshot_each_boundary {
                    cpu = round_trip(cpu);
                }
            }
            Z80MicroStep::InstructionRetired {
                opcode: retired_opcode,
                tstates,
            } => {
                assert_eq!(retired_opcode, opcode);
                assert_eq!(request_index, if is_transfer(opcode) { 4 } else { 3 });
                assert_eq!(internal_tstates, if is_transfer(opcode) { 0 } else { 5 });
                return (cpu, tstates);
            }
            other => panic!("unexpected micro-step {other:?} for ED {opcode:02X}"),
        }
    }
}

#[test]
fn all_simple_block_transfer_and_compare_inputs_match_independent_flags() {
    let mut memory = Box::new([0_u8; 0x1_0000]);

    for opcode in [0xA0_u8, 0xA8, 0xA1, 0xA9] {
        for accumulator in u8::MIN..=u8::MAX {
            for value in u8::MIN..=u8::MAX {
                for initial_count in [0_u16, 1, 2] {
                    // Across the exhaustive A/value domain, all eight
                    // combinations of preserved S/Z/C are exercised. H/N/PV
                    // are seeded independently and must be replaced.
                    let initial_f = (accumulator & FLAG_S)
                        | ((value >> 1) & FLAG_Z)
                        | ((accumulator ^ value) & FLAG_C)
                        | ((accumulator ^ value) & (FLAG_H | FLAG_PV | FLAG_N));
                    let count = initial_count.wrapping_sub(1);
                    let expected_f = expected_flags(opcode, initial_f, accumulator, value, count);

                    let (cpu, tstates) = run_case(
                        &mut memory,
                        opcode,
                        accumulator,
                        initial_f,
                        value,
                        initial_count,
                        false,
                        false,
                    );
                    let state = cpu.export_architectural_state();
                    let delta = direction(opcode);
                    assert_eq!(tstates, 16, "ED {opcode:02X}");
                    assert_eq!(state.registers.a, accumulator, "ED {opcode:02X}: A");
                    assert_eq!(state.registers.f, expected_f, "ED {opcode:02X}: F");
                    assert_eq!(state.q, expected_f, "ED {opcode:02X}: Q");
                    assert_eq!(state.registers.bc(), count, "ED {opcode:02X}: BC");
                    assert_eq!(
                        state.registers.hl(),
                        0xC000_u16.wrapping_add_signed(delta),
                        "ED {opcode:02X}: HL"
                    );
                    assert_eq!(state.registers.pc, 0x1002, "ED {opcode:02X}: PC");
                    assert_eq!(state.r, 0x80, "ED {opcode:02X}: refresh");
                    assert_eq!(state.ix, 0x1357, "ED {opcode:02X}: IX");
                    assert_eq!(state.iy, 0x2468, "ED {opcode:02X}: IY");

                    if is_transfer(opcode) {
                        assert_eq!(
                            state.registers.de(),
                            0xC100_u16.wrapping_add_signed(delta),
                            "ED {opcode:02X}: DE"
                        );
                        assert_eq!(memory[0xC100], value, "ED {opcode:02X}: destination");
                    } else {
                        assert_eq!(
                            state.registers.de(),
                            0xC100,
                            "ED {opcode:02X}: DE preserved"
                        );
                        assert_eq!(memory[0xC100], 0xCC, "ED {opcode:02X}: no write-back");
                    }
                }
            }
        }
    }
}

#[test]
fn simple_block_forms_preserve_waits_and_snapshot_restore_at_every_boundary() {
    let mut memory = Box::new([0_u8; 0x1_0000]);

    for opcode in [0xA0_u8, 0xA8, 0xA1, 0xA9] {
        let accumulator = 0x80;
        let value = 0x01;
        let initial_f = 0xD7;
        let (cpu, tstates) = run_case(
            &mut memory,
            opcode,
            accumulator,
            initial_f,
            value,
            2,
            true,
            true,
        );
        let expected = expected_flags(opcode, initial_f, accumulator, value, 1);
        let expected_waits = if is_transfer(opcode) { 4 } else { 3 };
        assert_eq!(tstates, 16 + expected_waits, "ED {opcode:02X}");
        assert_eq!(cpu.registers.f, expected, "ED {opcode:02X}: F after resume");
        assert_eq!(cpu.registers.pc, 0x1002, "ED {opcode:02X}: PC after resume");
        assert_eq!(cpu.registers.bc(), 1, "ED {opcode:02X}: BC after resume");
        assert!(cpu.pending_request().is_none(), "ED {opcode:02X}");
        assert!(!cpu.instruction_active(), "ED {opcode:02X}");
    }
}
