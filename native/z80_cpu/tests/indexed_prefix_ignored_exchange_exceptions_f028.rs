//! F-028: DD/FD prefixes are ignored by EX AF,AF', EXX and EX DE,HL.
//!
//! These are explicit exceptions to IX/IY substitution. Both the atomic
//! executor and resumable microsequencer must execute the base exchange while
//! accounting for the extra M1 fetch, R increment, WAIT and snapshot boundary.

use sega_z80_cpu::{
    Z80, Z80AlternateRegisters, Z80ArchitecturalState, Z80BusOperation, Z80MicroSequencer,
    Z80MicroSequencerState, Z80MicroStep, Z80Registers, bus::CpuBus,
};

const START_PC: u16 = 0xFFFF;
const INITIAL_WZ: u16 = 0xABCD;
const INITIAL_R: u8 = 0xFE;

fn increment_refresh(value: u8) -> u8 {
    (value & 0x80) | (value.wrapping_add(1) & 0x7F)
}

struct FlatBus {
    memory: Vec<u8>,
}

impl Default for FlatBus {
    fn default() -> Self {
        Self {
            memory: vec![0; 0x1_0000],
        }
    }
}

impl CpuBus for FlatBus {
    fn read_memory(&mut self, address: u16) -> u8 {
        self.memory[usize::from(address)]
    }

    fn write_memory(&mut self, address: u16, value: u8) {
        self.memory[usize::from(address)] = value;
    }
}

fn initial_registers() -> Z80Registers {
    Z80Registers {
        a: 0x11,
        f: 0x22,
        b: 0x33,
        c: 0x44,
        d: 0x55,
        e: 0x66,
        h: 0x77,
        l: 0x88,
        sp: 0x9900,
        pc: START_PC,
    }
}

fn initial_alternate() -> Z80AlternateRegisters {
    Z80AlternateRegisters {
        a: 0xAA,
        f: 0xBB,
        b: 0xCC,
        c: 0xDD,
        d: 0xEE,
        e: 0xF0,
        h: 0x12,
        l: 0x34,
    }
}

fn initial_architectural_state() -> Z80ArchitecturalState {
    Z80ArchitecturalState {
        registers: initial_registers(),
        alternate: initial_alternate(),
        ix: 0x1357,
        iy: 0x2468,
        i: 0x81,
        r: INITIAL_R,
        iff1: true,
        iff2: false,
        interrupt_mode: 2,
        wz: INITIAL_WZ,
        ei_delay: 0,
        nmi_pending: false,
        int_pending: None,
        q: 0x5A,
        q_previous: 0xA5,
        p: true,
        halted: false,
    }
}

fn seed_atomic() -> Z80 {
    let mut cpu = Z80::default();
    cpu.import_architectural_state(initial_architectural_state())
        .expect("initial state imports");
    cpu.total_tstates = 0;
    cpu
}
fn run_atomic(prefix: Option<u8>, opcode: u8) -> (Z80, u32) {
    let mut bus = FlatBus::default();
    match prefix {
        Some(prefix) => {
            bus.memory[usize::from(START_PC)] = prefix;
            bus.memory[0] = opcode;
        }
        None => bus.memory[usize::from(START_PC)] = opcode,
    }
    let mut cpu = seed_atomic();
    let tstates = cpu.step(&mut bus).expect("atomic exchange executes");
    (cpu, tstates)
}

fn round_trip(cpu: Z80MicroSequencer) -> Z80MicroSequencer {
    let encoded = cpu.export_state().encode().expect("snapshot encodes");
    let decoded = Z80MicroSequencerState::decode(&encoded).expect("snapshot decodes");
    Z80MicroSequencer::restore_state(decoded).expect("snapshot restores")
}

fn run_microsequencer(prefix: Option<u8>, opcode: u8) -> (Z80ArchitecturalState, u32) {
    let mut memory = Box::new([0_u8; 0x1_0000]);
    match prefix {
        Some(prefix) => {
            memory[usize::from(START_PC)] = prefix;
            memory[0] = opcode;
        }
        None => memory[usize::from(START_PC)] = opcode,
    }

    let mut cpu = Z80MicroSequencer::default();
    cpu.import_architectural_state(initial_architectural_state())
        .expect("initial state imports");
    let mut step = cpu.begin_instruction().expect("instruction begins");

    loop {
        match step {
            Z80MicroStep::BusRequest(request) => {
                assert_eq!(request.operation, Z80BusOperation::OpcodeFetch);
                cpu.insert_wait_tstate().expect("WAIT is accepted");
                cpu = round_trip(cpu);
                let response = Some(memory[usize::from(request.address)]);
                step = cpu
                    .complete_bus_cycle(request.id, response)
                    .expect("opcode fetch completes");
                cpu = round_trip(cpu);
            }
            Z80MicroStep::InternalCycle { .. } => {
                panic!("EX exchange exceptions have no internal cycle");
            }
            Z80MicroStep::InstructionRetired { tstates, .. } => {
                assert!(cpu.pending_request().is_none());
                assert!(!cpu.instruction_active());
                return (cpu.export_architectural_state(), tstates);
            }
            other => panic!("unexpected micro-step {other:?}"),
        }
    }
}

fn assert_registers_equal_except_pc(base: &Z80Registers, prefixed: &Z80Registers) {
    assert_eq!(prefixed.a, base.a);
    assert_eq!(prefixed.f, base.f);
    assert_eq!(prefixed.b, base.b);
    assert_eq!(prefixed.c, base.c);
    assert_eq!(prefixed.d, base.d);
    assert_eq!(prefixed.e, base.e);
    assert_eq!(prefixed.h, base.h);
    assert_eq!(prefixed.l, base.l);
    assert_eq!(prefixed.sp, base.sp);
    assert_eq!(prefixed.pc, base.pc.wrapping_add(1));
}

fn assert_atomic_equal_except_prefix(base: &Z80, prefixed: &Z80) {
    let base_state = base.export_architectural_state();
    let prefixed_state = prefixed.export_architectural_state();
    assert_architecture_equal_except_prefix(&base_state, &prefixed_state);
    assert_eq!(prefixed.total_tstates, base.total_tstates + 4);
}
fn assert_architecture_equal_except_prefix(
    base: &Z80ArchitecturalState,
    prefixed: &Z80ArchitecturalState,
) {
    assert_registers_equal_except_pc(&base.registers, &prefixed.registers);
    assert_eq!(prefixed.alternate, base.alternate);
    assert_eq!(prefixed.ix, base.ix);
    assert_eq!(prefixed.iy, base.iy);
    assert_eq!(prefixed.i, base.i);
    assert_eq!(prefixed.r, increment_refresh(base.r));
    assert_eq!(prefixed.iff1, base.iff1);
    assert_eq!(prefixed.iff2, base.iff2);
    assert_eq!(prefixed.interrupt_mode, base.interrupt_mode);
    assert_eq!(prefixed.wz, base.wz);
    assert_eq!(prefixed.ei_delay, base.ei_delay);
    assert_eq!(prefixed.nmi_pending, base.nmi_pending);
    assert_eq!(prefixed.int_pending, base.int_pending);
    assert_eq!(prefixed.q, base.q);
    assert_eq!(prefixed.p, base.p);
    assert_eq!(prefixed.halted, base.halted);
}

fn assert_exchange_result(opcode: u8, registers: &Z80Registers, alternate: &Z80AlternateRegisters) {
    match opcode {
        0x08 => {
            assert_eq!((registers.a, registers.f), (0xAA, 0xBB));
            assert_eq!((alternate.a, alternate.f), (0x11, 0x22));
            assert_eq!((registers.b, registers.c), (0x33, 0x44));
            assert_eq!((registers.d, registers.e), (0x55, 0x66));
            assert_eq!((registers.h, registers.l), (0x77, 0x88));
            assert_eq!((alternate.b, alternate.c), (0xCC, 0xDD));
            assert_eq!((alternate.d, alternate.e), (0xEE, 0xF0));
            assert_eq!((alternate.h, alternate.l), (0x12, 0x34));
        }
        0xD9 => {
            assert_eq!((registers.a, registers.f), (0x11, 0x22));
            assert_eq!((registers.b, registers.c), (0xCC, 0xDD));
            assert_eq!((registers.d, registers.e), (0xEE, 0xF0));
            assert_eq!((registers.h, registers.l), (0x12, 0x34));
            assert_eq!((alternate.b, alternate.c), (0x33, 0x44));
            assert_eq!((alternate.d, alternate.e), (0x55, 0x66));
            assert_eq!((alternate.h, alternate.l), (0x77, 0x88));
            assert_eq!((alternate.a, alternate.f), (0xAA, 0xBB));
        }
        0xEB => {
            assert_eq!((registers.a, registers.f), (0x11, 0x22));
            assert_eq!((registers.b, registers.c), (0x33, 0x44));
            assert_eq!((registers.d, registers.e), (0x77, 0x88));
            assert_eq!((registers.h, registers.l), (0x55, 0x66));
            assert_eq!(alternate, &initial_alternate());
        }
        _ => unreachable!("only EX AF, EXX and EX DE,HL are tested"),
    }
}

#[test]
fn dd_fd_prefixes_are_ignored_for_exchange_exception_opcodes() {
    for opcode in [0x08_u8, 0xD9, 0xEB] {
        let (base, base_tstates) = run_atomic(None, opcode);
        assert_eq!(base_tstates, 4, "base opcode {opcode:02X}");
        assert_eq!(base.registers.pc, 0x0000, "base PC wraps");
        assert_eq!(base.r, 0xFF, "base refresh wraps");
        assert_eq!(base.wz, INITIAL_WZ);
        assert_eq!(base.ix, 0x1357);
        assert_eq!(base.iy, 0x2468);
        assert_exchange_result(opcode, &base.registers, &base.alternate);
        assert_eq!(base.export_architectural_state().q_previous, 0x5A);

        for prefix in [0xDD_u8, 0xFD] {
            let (prefixed, prefixed_tstates) = run_atomic(Some(prefix), opcode);
            assert_eq!(prefixed_tstates, 8, "{prefix:02X} {opcode:02X}");
            assert_eq!(prefixed.registers.pc, 0x0001, "{prefix:02X} {opcode:02X}");
            assert_eq!(prefixed.r, 0x80, "{prefix:02X} {opcode:02X}");
            assert_atomic_equal_except_prefix(&base, &prefixed);
            assert_eq!(prefixed.export_architectural_state().q_previous, 0);
            assert_exchange_result(opcode, &prefixed.registers, &prefixed.alternate);
        }

        let (base_micro, base_micro_tstates) = run_microsequencer(None, opcode);
        assert_eq!(base_micro_tstates, 5, "base micro opcode {opcode:02X}");
        assert_eq!(base_micro.registers.pc, 0x0000);
        assert_eq!(base_micro.r, 0xFF);
        assert_exchange_result(opcode, &base_micro.registers, &base_micro.alternate);
        assert_eq!(base_micro.q_previous, 0x5A);

        for prefix in [0xDD_u8, 0xFD] {
            let (prefixed_micro, prefixed_micro_tstates) = run_microsequencer(Some(prefix), opcode);
            assert_eq!(
                prefixed_micro_tstates, 10,
                "{prefix:02X} {opcode:02X}: two M1 WAITs"
            );
            assert_eq!(prefixed_micro.registers.pc, 0x0001);
            assert_eq!(prefixed_micro.r, 0x80);
            assert_architecture_equal_except_prefix(&base_micro, &prefixed_micro);
            assert_eq!(prefixed_micro.q_previous, 0);
            assert_exchange_result(opcode, &prefixed_micro.registers, &prefixed_micro.alternate);
        }
    }
}
