//! F-023: microsequencer observation of RRD/RLD MEMPTR through BIT flags.
//!
//! Each bus/internal boundary is snapshot-serialized and restored. WAIT is
//! inserted on data-memory accesses so the test covers resumable behavior,
//! not just the atomic instruction executor.

use sega_z80_cpu::{
    Z80AlternateRegisters, Z80ArchitecturalState, Z80BusOperation, Z80MicroSequencer,
    Z80MicroSequencerState, Z80MicroStep, Z80Registers,
};

const FLAG_C: u8 = 0x01;
const FLAG_PV: u8 = 0x04;
const FLAG_H: u8 = 0x10;
const FLAG_Z: u8 = 0x40;
const FLAG_XY: u8 = 0x28;

fn round_trip(cpu: Z80MicroSequencer) -> Z80MicroSequencer {
    let encoded = cpu.export_state().encode().expect("snapshot encodes");
    let decoded = Z80MicroSequencerState::decode(&encoded).expect("snapshot decodes");
    Z80MicroSequencer::restore_state(decoded).expect("snapshot restores")
}

fn initial_state(accumulator: u8, carry: bool, hl: u16) -> Z80ArchitecturalState {
    Z80ArchitecturalState {
        registers: Z80Registers {
            a: accumulator,
            f: if carry { FLAG_C } else { 0 },
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: (hl >> 8) as u8,
            l: hl as u8,
            sp: 0xD000,
            pc: 0x0100,
        },
        alternate: Z80AlternateRegisters::default(),
        ix: 0x1357,
        iy: 0x2468,
        i: 0x81,
        r: 0,
        iff1: false,
        iff2: false,
        interrupt_mode: 0,
        wz: 0x4567,
        ei_delay: 0,
        nmi_pending: false,
        int_pending: None,
        q: 0,
        q_previous: 0,
        p: false,
        halted: false,
    }
}

fn run_instruction(
    mut cpu: Z80MicroSequencer,
    memory: &mut [u8; 0x1_0000],
    wait_on_memory: bool,
) -> (Z80MicroSequencer, u32) {
    let mut step = cpu.begin_instruction().expect("instruction begins");

    loop {
        match step {
            Z80MicroStep::BusRequest(request) => {
                let memory_access = matches!(
                    request.operation,
                    Z80BusOperation::MemoryRead | Z80BusOperation::MemoryWrite { .. }
                );
                if wait_on_memory && memory_access {
                    cpu.insert_wait_tstate().expect("WAIT is accepted");
                    cpu = round_trip(cpu);
                }

                let response = match request.operation {
                    Z80BusOperation::OpcodeFetch | Z80BusOperation::MemoryRead => {
                        Some(memory[usize::from(request.address)])
                    }
                    Z80BusOperation::MemoryWrite { value } => {
                        memory[usize::from(request.address)] = value;
                        None
                    }
                    other => panic!("unexpected bus operation {other:?}"),
                };
                step = cpu
                    .complete_bus_cycle(request.id, response)
                    .expect("bus cycle completes");
                cpu = round_trip(cpu);
            }
            Z80MicroStep::InternalCycle { tstates } => {
                assert_eq!(tstates, 1, "internal phases advance one T-state at a time");
                step = cpu
                    .complete_internal_cycle()
                    .expect("internal T-state completes");
                cpu = round_trip(cpu);
            }
            Z80MicroStep::InstructionRetired { tstates, .. } => {
                assert!(cpu.pending_request().is_none());
                assert!(!cpu.instruction_active());
                return (cpu, tstates);
            }
            other => panic!("unexpected micro-step {other:?}"),
        }
    }
}

#[test]
fn rrd_rld_memptr_observation_survives_wait_and_snapshot_restore() {
    let pc = 0x0100_u16;
    let hl_values = [0x07FF_u16, 0x1FFF, 0x27FF, 0xFFFF];
    let accumulator_values = [0x00_u8, 0x5A, 0xFF];
    let memory_values = [0x00_u8, 0x01, 0x7F, 0x80, 0xFF];
    let mut memory = Box::new([0_u8; 0x1_0000]);

    for opcode in [0x67_u8, 0x6F] {
        memory[usize::from(pc)] = 0xED;
        memory[usize::from(pc.wrapping_add(1))] = opcode;
        memory[usize::from(pc.wrapping_add(2))] = 0xCB;
        memory[usize::from(pc.wrapping_add(3))] = 0x46; // BIT 0,(HL)

        for hl in hl_values {
            let expected_wz = hl.wrapping_add(1);
            for accumulator in accumulator_values {
                for memory_value in memory_values {
                    for carry_in in [false, true] {
                        memory[usize::from(hl)] = memory_value;
                        let cpu = Z80MicroSequencer::default();
                        let cpu = cpu
                            .import_architectural_state(initial_state(accumulator, carry_in, hl))
                            .expect("initial state imports");

                        let (cpu, rrd_rld_tstates) = run_instruction(cpu, &mut memory, true);
                        let (expected_accumulator, expected_memory) = if opcode == 0x67 {
                            (
                                (accumulator & 0xF0) | (memory_value & 0x0F),
                                ((accumulator & 0x0F) << 4) | (memory_value >> 4),
                            )
                        } else {
                            (
                                (accumulator & 0xF0) | (memory_value >> 4),
                                (memory_value << 4) | (accumulator & 0x0F),
                            )
                        };
                        assert_eq!(rrd_rld_tstates, 20, "ED {opcode:02X}: WAITed RRD/RLD");
                        assert_eq!(cpu.wz, expected_wz, "ED {opcode:02X}: WZ");
                        assert_eq!(cpu.registers.a, expected_accumulator, "ED {opcode:02X}: A");
                        assert_eq!(
                            memory[usize::from(hl)],
                            expected_memory,
                            "ED {opcode:02X}: memory"
                        );

                        let mut expected_flags =
                            FLAG_H | (((expected_wz >> 8) as u8) & FLAG_XY);
                        if carry_in {
                            expected_flags |= FLAG_C;
                        }
                        if expected_memory & 0x01 == 0 {
                            expected_flags |= FLAG_Z | FLAG_PV;
                        }

                        let (cpu, bit_tstates) = run_instruction(cpu, &mut memory, true);
                        assert_eq!(bit_tstates, 13, "BIT 0,(HL): WAITed T-states");
                        assert_eq!(cpu.registers.f, expected_flags, "ED {opcode:02X} then BIT");
                        assert_eq!(cpu.q, expected_flags, "ED {opcode:02X}: Q after BIT");
                        assert_eq!(cpu.wz, expected_wz, "ED {opcode:02X}: WZ after BIT");
                        assert_eq!(cpu.registers.hl(), hl, "ED {opcode:02X}: HL preserved");
                        assert_eq!(cpu.registers.pc, pc.wrapping_add(4), "ED {opcode:02X}: PC");
                        assert_eq!(cpu.r, 4, "ED {opcode:02X}: refresh");
                    }
                }
            }
        }
    }
}
