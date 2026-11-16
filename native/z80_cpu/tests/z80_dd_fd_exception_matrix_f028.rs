//! F-028: DD/FD ignored-prefix exception matrix.
//!
//! Covers EX AF,AF', EXX, EX DE,HL, HALT and ED NEG with single, repeated,
//! and alternating DD/FD prefixes. Both atomic and resumable microsequencer
//! paths are compared architecturally; the micro path snapshots around every
//! M1 WAIT and bus completion.

use sega_z80_cpu::{
    Z80, Z80AlternateRegisters, Z80ArchitecturalState, Z80BusOperation, Z80MicroSequencer,
    Z80MicroSequencerState, Z80MicroStep, Z80PrefixState, Z80Registers, bus::CpuBus,
};

const START_PC: u16 = 0xFFFF;
const INITIAL_R: u8 = 0xFE;
const PREFIX_SEQUENCES: &[&[u8]] = &[
    &[0xDD],
    &[0xFD],
    &[0xDD, 0xDD],
    &[0xDD, 0xFD],
    &[0xFD, 0xDD],
    &[0xFD, 0xFD],
    &[0xDD, 0xFD, 0xDD],
    &[0xFD, 0xDD, 0xFD],
    &[0xDD, 0xFD, 0xDD, 0xFD],
];

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

fn initial_state_with_q(q: u8) -> Z80ArchitecturalState {
    Z80ArchitecturalState {
        registers: Z80Registers {
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
        },
        alternate: Z80AlternateRegisters {
            a: 0xAA,
            f: 0xBB,
            b: 0xCC,
            c: 0xDD,
            d: 0xEE,
            e: 0xF0,
            h: 0x12,
            l: 0x34,
        },
        ix: 0x1357,
        iy: 0x2468,
        i: 0x81,
        r: INITIAL_R,
        iff1: true,
        iff2: false,
        interrupt_mode: 2,
        wz: 0xABCD,
        ei_delay: 0,
        nmi_pending: false,
        int_pending: None,
        q,
        q_previous: 0xA5,
        p: true,
        halted: false,
    }
}

fn initial_state() -> Z80ArchitecturalState {
    initial_state_with_q(0x5A)
}

fn seed_atomic_with_state(state: Z80ArchitecturalState) -> Z80 {
    let mut cpu = Z80::default();
    cpu.import_architectural_state(state)
        .expect("initial state imports");
    cpu.total_tstates = 0;
    cpu
}

fn increment_refresh(value: u8) -> u8 {
    (value & 0x80) | (value.wrapping_add(1) & 0x7F)
}

fn increment_refresh_by(mut value: u8, count: usize) -> u8 {
    for _ in 0..count {
        value = increment_refresh(value);
    }
    value
}

fn run_atomic(bytes: &[u8]) -> (Z80, u32) {
    run_atomic_with_state(bytes, initial_state())
}

fn run_atomic_with_state(bytes: &[u8], state: Z80ArchitecturalState) -> (Z80, u32) {
    let mut bus = FlatBus::default();
    for (offset, byte) in bytes.iter().copied().enumerate() {
        let address = START_PC.wrapping_add(offset as u16);
        bus.memory[usize::from(address)] = byte;
    }

    let mut cpu = seed_atomic_with_state(state);
    let tstates = cpu.step(&mut bus).expect("instruction executes atomically");
    (cpu, tstates)
}

fn round_trip(cpu: Z80MicroSequencer) -> Z80MicroSequencer {
    let encoded = cpu.export_state().encode().expect("snapshot encodes");
    let decoded = Z80MicroSequencerState::decode(&encoded).expect("snapshot decodes");
    Z80MicroSequencer::restore_state(decoded).expect("snapshot restores")
}

fn run_micro_with_state(
    bytes: &[u8],
    state: Z80ArchitecturalState,
) -> (Z80ArchitecturalState, u32) {
    let mut cpu = Z80MicroSequencer::default();
    cpu.import_architectural_state(state)
        .expect("initial state imports");
    let mut step = cpu.begin_instruction().expect("instruction begins");

    for (offset, byte) in bytes.iter().copied().enumerate() {
        let request = match step {
            Z80MicroStep::BusRequest(request) => request,
            other => panic!("expected M1 opcode fetch at offset {offset}, got {other:?}"),
        };
        assert_eq!(request.operation, Z80BusOperation::OpcodeFetch);
        assert_eq!(
            request.address,
            START_PC.wrapping_add(offset as u16),
            "M1 fetch address at byte {offset}"
        );
        assert_eq!(request.base_tstates, 4);

        cpu.insert_wait_tstate().expect("one M1 WAIT is accepted");
        cpu = round_trip(cpu);
        step = cpu
            .complete_bus_cycle(request.id, Some(byte))
            .expect("M1 fetch completes");
        cpu = round_trip(cpu);
    }

    let tstates = match step {
        Z80MicroStep::InstructionRetired { opcode, tstates } => {
            assert_eq!(opcode, *bytes.last().expect("non-empty instruction"));
            tstates
        }
        other => panic!("expected instruction retirement, got {other:?}"),
    };
    assert!(cpu.pending_request().is_none());
    assert!(!cpu.instruction_active());
    assert_eq!(cpu.export_state().prefix_state, Z80PrefixState::None);
    (cpu.export_architectural_state(), tstates)
}

fn assert_equal_except_prefix_count(
    base: &Z80ArchitecturalState,
    prefixed: &Z80ArchitecturalState,
    prefix_count: usize,
) {
    assert_eq!(prefixed.registers.a, base.registers.a);
    assert_eq!(prefixed.registers.f, base.registers.f);
    assert_eq!(prefixed.registers.b, base.registers.b);
    assert_eq!(prefixed.registers.c, base.registers.c);
    assert_eq!(prefixed.registers.d, base.registers.d);
    assert_eq!(prefixed.registers.e, base.registers.e);
    assert_eq!(prefixed.registers.h, base.registers.h);
    assert_eq!(prefixed.registers.l, base.registers.l);
    assert_eq!(prefixed.registers.sp, base.registers.sp);
    assert_eq!(
        prefixed.registers.pc,
        base.registers.pc.wrapping_add(prefix_count as u16)
    );
    assert_eq!(prefixed.alternate, base.alternate);
    assert_eq!(prefixed.ix, base.ix);
    assert_eq!(prefixed.iy, base.iy);
    assert_eq!(prefixed.i, base.i);
    assert_eq!(
        prefixed.r,
        increment_refresh_by(base.r, prefix_count),
        "each ignored DD/FD byte is one additional M1 refresh"
    );
    assert_eq!(prefixed.iff1, base.iff1);
    assert_eq!(prefixed.iff2, base.iff2);
    assert_eq!(prefixed.interrupt_mode, base.interrupt_mode);
    assert_eq!(prefixed.wz, base.wz);
    assert_eq!(prefixed.ei_delay, base.ei_delay);
    assert_eq!(prefixed.nmi_pending, base.nmi_pending);
    assert_eq!(prefixed.int_pending, base.int_pending);
    assert_eq!(prefixed.q, base.q);
    assert_eq!(prefixed.q_previous, 0, "DD/FD prefix clears previous-Q");
    assert_eq!(prefixed.p, base.p);
    assert_eq!(prefixed.halted, base.halted);
}

fn run_prefix_matrix(opcode_bytes: &[u8], base_m1_count: usize, is_halt: bool) {
    let state = initial_state();
    run_prefix_matrix_with_states(opcode_bytes, base_m1_count, is_halt, state, state);
}

fn run_prefix_matrix_with_states(
    opcode_bytes: &[u8],
    base_m1_count: usize,
    is_halt: bool,
    base_initial_state: Z80ArchitecturalState,
    prefixed_initial_state: Z80ArchitecturalState,
) {
    let (base_atomic, base_atomic_tstates) =
        run_atomic_with_state(opcode_bytes, base_initial_state);
    let base_atomic_state = base_atomic.export_architectural_state();
    assert_eq!(
        base_atomic_state.halted, is_halt,
        "base instruction halt state"
    );

    let (base_micro_state, base_micro_tstates) =
        run_micro_with_state(opcode_bytes, base_initial_state);
    assert_eq!(
        base_micro_state, base_atomic_state,
        "base atomic/micro state"
    );
    assert_eq!(
        base_micro_tstates,
        base_atomic_tstates + base_m1_count as u32,
        "one WAIT T-state per base M1"
    );

    for prefixes in PREFIX_SEQUENCES {
        let mut bytes = prefixes.to_vec();
        bytes.extend_from_slice(opcode_bytes);
        let prefix_count = prefixes.len();

        let (prefixed_atomic, prefixed_atomic_tstates) =
            run_atomic_with_state(&bytes, prefixed_initial_state);
        let prefixed_atomic_state = prefixed_atomic.export_architectural_state();
        assert_equal_except_prefix_count(&base_atomic_state, &prefixed_atomic_state, prefix_count);
        assert_eq!(
            prefixed_atomic_tstates,
            base_atomic_tstates + 4 * prefix_count as u32,
            "one additional M1 per ignored prefix: {bytes:02X?}"
        );
        assert_eq!(
            prefixed_atomic.total_tstates,
            u64::from(prefixed_atomic_tstates)
        );

        let (prefixed_micro_state, prefixed_micro_tstates) =
            run_micro_with_state(&bytes, prefixed_initial_state);
        assert_eq!(
            prefixed_micro_state, prefixed_atomic_state,
            "atomic/micro architectural state for {bytes:02X?}"
        );
        assert_eq!(
            prefixed_micro_tstates,
            base_micro_tstates + 5 * prefix_count as u32,
            "one M1 and one WAIT per ignored prefix: {bytes:02X?}"
        );
        assert_eq!(
            prefixed_micro_state.halted, is_halt,
            "halt state for {bytes:02X?}"
        );
    }
}

#[test]
fn exchange_exceptions_cover_repeated_and_alternating_prefix_chains() {
    for opcode in [0x08_u8, 0xD9, 0xEB] {
        let (base, _) = run_atomic(&[opcode]);
        match opcode {
            0x08 => {
                assert_eq!((base.registers.a, base.registers.f), (0xAA, 0xBB));
                assert_eq!((base.alternate.a, base.alternate.f), (0x11, 0x22));
            }
            0xD9 => {
                assert_eq!((base.registers.b, base.registers.c), (0xCC, 0xDD));
                assert_eq!((base.alternate.b, base.alternate.c), (0x33, 0x44));
            }
            0xEB => {
                assert_eq!((base.registers.d, base.registers.e), (0x77, 0x88));
                assert_eq!((base.registers.h, base.registers.l), (0x55, 0x66));
            }
            _ => unreachable!(),
        }
        run_prefix_matrix(&[opcode], 1, false);
    }
}

#[test]
fn halt_is_terminal_after_single_repeated_and_alternating_dd_fd_prefixes() {
    let (base, _) = run_atomic(&[0x76]);
    assert!(base.halted);
    assert_eq!(base.registers.f, 0x22);
    run_prefix_matrix(&[0x76], 1, true);
}

#[test]
fn dd_fd_prefix_chains_cancel_before_ed_neg_in_both_executors() {
    let (base, _) = run_atomic(&[0xED, 0x44]);
    assert_eq!(base.registers.a, 0xEF);
    assert_ne!(base.registers.f, 0x22);
    run_prefix_matrix(&[0xED, 0x44], 2, false);
}

#[test]
fn ignored_flag_and_interrupt_opcodes_preserve_base_semantics_across_prefix_chains() {
    // NOP, DAA, CPL, SCF, CCF, DI and EI do not consume IX/IY.
    // SCF/CCF also exercise the Q/previous-Q boundary after multiple prefixes.
    for opcode in [0x00_u8, 0x27, 0x2F, 0xF3, 0xFB] {
        run_prefix_matrix(&[opcode], 1, false);
    }

    // SCF/CCF derive undocumented flags from previous-Q. Compare a prefixed
    // instruction with the BASE oracle initialized with previous-Q already
    // cleared, while the prefixed path starts with nonzero previous-Q.
    for opcode in [0x37_u8, 0x3F] {
        run_prefix_matrix_with_states(
            &[opcode],
            1,
            false,
            initial_state_with_q(0),
            initial_state_with_q(0x5A),
        );
    }
}
