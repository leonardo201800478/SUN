//! F-028: DD/FD ignored fallback architectural and resumability matrix.
//!
//! This test compares every effective Ignored byte with its BASE execution.
//! It captures read values and write payloads, compares the complete request
//! sequence after stream-address normalization, and snapshots/restores around
//! every bus request and internal-cycle boundary while inserting one WAIT per
//! bus request. This is a structural differential against the same BASE
//! microsequencer, not an independent hardware semantic oracle.

use sega_z80_cpu::{
    Z80ArchitecturalState, Z80BusOperation, Z80MicroSequencer, Z80MicroSequencerState, Z80MicroStep,
};

const DD_FD_LEDGER: &str =
    include_str!("../../../docs/audits/z80-dd-fd-post-prefix-byte-ledger-2026-11-16.csv");
const EXPECTED_IGNORED_BYTES: usize = 167;
const CALL_RST_OPCODES: [u8; 17] = [
    0xC4, 0xCC, 0xD4, 0xDC, 0xE4, 0xEC, 0xF4, 0xFC, 0xCD, 0xC7, 0xCF, 0xD7, 0xDF, 0xE7, 0xEF, 0xF7,
    0xFF,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BusTraceKind {
    OpcodeFetch,
    MemoryRead,
    MemoryWrite,
    PortRead,
    PortWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraceEvent {
    Bus {
        kind: BusTraceKind,
        address: u16,
        base_tstates: u8,
        stream: bool,
        read_value: Option<u8>,
        write_value: Option<u8>,
        wait_tstates: u8,
    },
    InternalCycle {
        tstates: u32,
    },
}

#[derive(Debug)]
struct Execution {
    tstates: u32,
    fetch_addresses: Vec<u16>,
    trace: Vec<TraceEvent>,
    state: Z80ArchitecturalState,
    wait_count: usize,
}

fn ignored_opcodes_from_ledger() -> Vec<u8> {
    DD_FD_LEDGER
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut fields = line.split(',');
            let opcode = fields.next()?.trim_matches('"');
            let class = fields.next()?.trim_matches('"');
            let action = fields.next()?.trim_matches('"');

            (class == "Ignored" && action == "ExecuteBaseWithoutRefetch")
                .then(|| u8::from_str_radix(opcode, 16).expect("ledger opcode is hexadecimal"))
        })
        .collect()
}

fn round_trip(cpu: Z80MicroSequencer) -> Z80MicroSequencer {
    let encoded = cpu.export_state().encode().expect("snapshot encodes");
    let decoded = Z80MicroSequencerState::decode(&encoded).expect("snapshot decodes");
    Z80MicroSequencer::restore_state(decoded).expect("snapshot restores")
}

fn increment_refresh(value: u8) -> u8 {
    (value & 0x80) | (value.wrapping_add(1) & 0x7F)
}

fn expected_stream_operand_bytes(opcode: u8) -> usize {
    if matches!(
        opcode,
        0x10 | 0x18 | 0x20 | 0x28 | 0x30 | 0x38 | 0xD3 | 0xDB
    ) || (opcode & 0xC7) == 0x06
        || (opcode & 0xC7) == 0xC6
    {
        1
    } else if matches!(opcode, 0x22 | 0x2A | 0x32 | 0x3A | 0xC3 | 0xCD)
        || (opcode & 0xCF) == 0x01
        || (opcode & 0xC7) == 0xC2
        || (opcode & 0xC7) == 0xC4
    {
        2
    } else {
        0
    }
}

fn execute_one_instruction(
    opcode_stream: &[u8],
    wait_and_snapshot: bool,
    initial_f: u8,
    initial_bc: u16,
) -> Execution {
    let mut cpu = Z80MicroSequencer::default();
    cpu.registers.set_bc(initial_bc);
    cpu.registers.set_de(0x3456);
    cpu.registers.set_hl(0x4567);
    cpu.registers.sp = 0x5678;
    cpu.registers.a = 0xA5;
    cpu.registers.f = initial_f;
    cpu.ix = 0x6789;
    cpu.iy = 0x789A;

    let mut stream = opcode_stream.iter().copied();
    let terminal_opcode = *opcode_stream.last().expect("opcode stream is non-empty");
    let mut stream_reads_remaining = expected_stream_operand_bytes(terminal_opcode);
    let mut fetch_addresses = Vec::new();
    let mut trace = Vec::new();
    let mut wait_count = 0usize;
    let mut step = cpu.begin_instruction().expect("instruction starts");

    for _ in 0..128 {
        step = match step {
            Z80MicroStep::BusRequest(request) => {
                let data = match request.operation {
                    Z80BusOperation::OpcodeFetch => {
                        assert_eq!(request.base_tstates, 4, "M1 base cost");
                        fetch_addresses.push(request.address);
                        Some(stream.next().unwrap_or_else(|| {
                            panic!(
                                "unexpected opcode refetch at {:04X}; stream={opcode_stream:?}",
                                request.address
                            )
                        }))
                    }
                    Z80BusOperation::InterruptAcknowledge => {
                        panic!("unexpected interrupt acknowledge in F-028 fallback matrix")
                    }
                    Z80BusOperation::MemoryRead | Z80BusOperation::PortRead => Some(0x5A),
                    Z80BusOperation::MemoryWrite { .. } | Z80BusOperation::PortWrite { .. } => None,
                };

                let stream_access = match request.operation {
                    Z80BusOperation::OpcodeFetch => true,
                    Z80BusOperation::MemoryRead if stream_reads_remaining > 0 => {
                        assert_eq!(
                            request.address, cpu.registers.pc,
                            "{opcode_stream:?}: immediate operand read must use the current PC"
                        );
                        stream_reads_remaining -= 1;
                        true
                    }
                    Z80BusOperation::MemoryRead => false,
                    _ => false,
                };
                let (kind, write_value) = match request.operation {
                    Z80BusOperation::OpcodeFetch => (BusTraceKind::OpcodeFetch, None),
                    Z80BusOperation::MemoryRead => (BusTraceKind::MemoryRead, None),
                    Z80BusOperation::MemoryWrite { value } => {
                        (BusTraceKind::MemoryWrite, Some(value))
                    }
                    Z80BusOperation::PortRead => (BusTraceKind::PortRead, None),
                    Z80BusOperation::PortWrite { value } => (BusTraceKind::PortWrite, Some(value)),
                    Z80BusOperation::InterruptAcknowledge => unreachable!(),
                };

                if wait_and_snapshot {
                    cpu.insert_wait_tstate()
                        .expect("WAIT is accepted for every request class");
                    cpu = round_trip(cpu);
                    assert_eq!(
                        cpu.pending_request().map(|pending| pending.id),
                        Some(request.id),
                        "pending bus request survives WAIT and snapshot restore"
                    );
                    wait_count += 1;
                }

                trace.push(TraceEvent::Bus {
                    kind,
                    address: request.address,
                    base_tstates: request.base_tstates,
                    stream: stream_access,
                    read_value: data,
                    write_value,
                    wait_tstates: u8::from(wait_and_snapshot),
                });

                let next = cpu
                    .complete_bus_cycle(request.id, data)
                    .expect("BASE bus operation completes");
                if wait_and_snapshot {
                    cpu = round_trip(cpu);
                }
                next
            }
            Z80MicroStep::InternalCycle { tstates } => {
                trace.push(TraceEvent::InternalCycle { tstates });
                if wait_and_snapshot {
                    cpu = round_trip(cpu);
                }
                let next = cpu
                    .complete_internal_cycle()
                    .expect("BASE internal cycle completes");
                if wait_and_snapshot {
                    cpu = round_trip(cpu);
                }
                next
            }
            Z80MicroStep::InstructionRetired { opcode, tstates } => {
                assert_eq!(opcode, terminal_opcode);
                assert_eq!(
                    stream_reads_remaining, 0,
                    "{opcode_stream:?}: all expected immediate operand reads must be consumed"
                );
                assert!(
                    stream.next().is_none(),
                    "instruction retired before consuming the complete opcode stream"
                );
                assert!(cpu.pending_request().is_none());
                assert!(!cpu.instruction_active());
                return Execution {
                    tstates,
                    fetch_addresses,
                    trace,
                    state: cpu.export_architectural_state(),
                    wait_count,
                };
            }
            Z80MicroStep::InterruptAcknowledge { .. } | Z80MicroStep::InterruptServiced { .. } => {
                panic!("unexpected interrupt service in F-028 fallback matrix")
            }
        };
    }

    panic!("opcode stream {opcode_stream:?} did not retire within 128 micro-operations");
}

fn remove_prefix_m1_and_normalize_stream_addresses(
    trace: Vec<TraceEvent>,
    prefix: u8,
    opcode: u8,
) -> Vec<TraceEvent> {
    let mut events = trace.into_iter();
    assert_eq!(
        events.next(),
        Some(TraceEvent::Bus {
            kind: BusTraceKind::OpcodeFetch,
            address: 0,
            base_tstates: 4,
            stream: true,
            read_value: Some(prefix),
            write_value: None,
            wait_tstates: 1,
        }),
        "the first prefixed event must be the DD/FD M1"
    );

    let mut normalized: Vec<_> = events.collect();
    for event in &mut normalized {
        if let TraceEvent::Bus {
            kind,
            address,
            stream,
            write_value,
            ..
        } = event
        {
            if *stream && matches!(*kind, BusTraceKind::OpcodeFetch | BusTraceKind::MemoryRead) {
                *address = address.wrapping_sub(1);
            }

            // CALL/conditional CALL/RST push the address after the complete
            // instruction. The ignored prefix shifts that return address by
            // one byte; normalize only its low-byte stack write (SP-2).
            if *kind == BusTraceKind::MemoryWrite
                && *address == 0x5676
                && CALL_RST_OPCODES.contains(&opcode)
            {
                *write_value = write_value.map(|value| value.wrapping_sub(1));
            }
        }
    }
    normalized
}

fn condition_true(condition: u8, flags: u8) -> bool {
    match condition & 0x07 {
        0 => flags & 0x40 == 0, // NZ
        1 => flags & 0x40 != 0, // Z
        2 => flags & 0x01 == 0, // NC
        3 => flags & 0x01 != 0, // C
        4 => flags & 0x04 == 0, // PO
        5 => flags & 0x04 != 0, // PE
        6 => flags & 0x80 == 0, // P
        7 => flags & 0x80 != 0, // M
        _ => unreachable!(),
    }
}

fn expected_pc_delta(opcode: u8, initial_f: u8) -> u16 {
    if opcode & 0xC7 == 0xC0 || opcode & 0xC7 == 0xC2 || opcode & 0xC7 == 0xC4 {
        if condition_true((opcode >> 3) & 0x07, initial_f) {
            0
        } else {
            1
        }
    } else if matches!(opcode, 0xC3 | 0xC9 | 0xCD) || opcode & 0xC7 == 0xC7 {
        0
    } else {
        // Relative branches shift their target by the prefix byte; ordinary
        // instructions consume one extra byte. Both therefore differ by +1.
        1
    }
}

fn assert_architectural_fallback(
    base: &Z80ArchitecturalState,
    prefixed: &Z80ArchitecturalState,
    prefix: u8,
    opcode: u8,
    initial_f: u8,
) {
    let mut expected = *base;
    expected.r = increment_refresh(base.r);

    let pc_delta = prefixed.registers.pc.wrapping_sub(base.registers.pc);
    assert_eq!(
        pc_delta,
        expected_pc_delta(opcode, initial_f),
        "{prefix:02X} {opcode:02X}: final PC must reflect the exact BASE control-flow outcome"
    );

    // WZ can follow a stream-relative branch target, so it is tracked by the
    // dedicated F-023 matrix rather than normalized as a generic fallback field.
    expected.registers.pc = prefixed.registers.pc;
    expected.wz = prefixed.wz;
    assert_eq!(
        prefixed.q_previous, 0,
        "{prefix:02X} {opcode:02X}: ignored prefix clears previous-Q"
    );
    expected.q_previous = prefixed.q_previous;

    assert_eq!(
        *prefixed, expected,
        "{prefix:02X} {opcode:02X}: BASE architectural effects must be preserved"
    );
}

#[test]
fn data_read_at_current_pc_is_not_misclassified_as_instruction_stream() {
    const OPCODE: u8 = 0x0A; // LD A,(BC), an effective DD/FD Ignored fallback.
    assert!(ignored_opcodes_from_ledger().contains(&OPCODE));

    // BASE has PC=1 at the data read; DD+opcode has PC=2. BC=2 therefore
    // collides with PC only in the prefixed execution. It is still a data
    // read, not an immediate operand fetch, and must not be address-normalized.
    let base = execute_one_instruction(&[OPCODE], true, 0x00, 0x0002);
    let prefixed = execute_one_instruction(&[0xDD, OPCODE], true, 0x00, 0x0002);

    let has_unclassified_data_read = |trace: &[TraceEvent]| {
        trace.iter().any(|event| {
            matches!(
                event,
                TraceEvent::Bus {
                    kind: BusTraceKind::MemoryRead,
                    address: 0x0002,
                    stream: false,
                    ..
                }
            )
        })
    };
    assert!(has_unclassified_data_read(&base.trace), "BASE data read");
    assert!(
        has_unclassified_data_read(&prefixed.trace),
        "prefixed data read must remain data even when address == PC"
    );
    assert_eq!(
        remove_prefix_m1_and_normalize_stream_addresses(prefixed.trace, 0xDD, OPCODE),
        base.trace,
        "data reads at the current PC must retain their actual address"
    );
}

#[test]
fn all_167_ignored_fallbacks_preserve_architecture_requests_wait_and_snapshot() {
    let ignored = ignored_opcodes_from_ledger();
    assert_eq!(
        ignored.len(),
        EXPECTED_IGNORED_BYTES,
        "effective Ignored inventory must match the reviewed ledger"
    );

    for opcode in ignored {
        let base = execute_one_instruction(&[opcode], true, 0x00, 0x2345);
        assert_eq!(base.fetch_addresses, vec![0], "BASE {opcode:02X}");
        assert_eq!(
            base.wait_count,
            base.trace
                .iter()
                .filter(|event| matches!(event, TraceEvent::Bus { .. }))
                .count(),
            "BASE {opcode:02X}: WAIT at every bus request"
        );

        for prefix in [0xDD, 0xFD] {
            let prefixed = execute_one_instruction(&[prefix, opcode], true, 0x00, 0x2345);
            assert_eq!(
                prefixed.fetch_addresses,
                vec![0, 1],
                "{prefix:02X} {opcode:02X}: prefix M1 and terminal M1 only"
            );
            assert_eq!(
                prefixed.wait_count,
                base.wait_count + 1,
                "{prefix:02X} {opcode:02X}: the prefix contributes one additional WAIT point"
            );
            assert_eq!(
                prefixed.tstates,
                base.tstates + 5,
                "{prefix:02X} {opcode:02X}: +4T prefix M1 plus its 1T WAIT"
            );
            assert_architectural_fallback(&base.state, &prefixed.state, prefix, opcode, 0x00);
            assert_eq!(
                remove_prefix_m1_and_normalize_stream_addresses(prefixed.trace, prefix, opcode),
                base.trace,
                "{prefix:02X} {opcode:02X}: request type/order/address/payload/timing and internal cycles"
            );
        }
    }
}

#[test]
fn all_167_ignored_fallbacks_preserve_both_conditional_outcomes_and_exact_pc() {
    let ignored = ignored_opcodes_from_ledger();
    assert_eq!(ignored.len(), EXPECTED_IGNORED_BYTES);

    // All condition flags are set and B=1: this flips each conditional
    // CALL/JP/RET/JR outcome relative to the zero-flag baseline and makes
    // DJNZ not taken. The first matrix already exercises the opposite DJNZ
    // path (B=0x23) and the complementary flag conditions.
    for opcode in ignored {
        let base = execute_one_instruction(&[opcode], true, 0xFF, 0x0145);
        assert_eq!(base.fetch_addresses, vec![0], "BASE {opcode:02X}");

        for prefix in [0xDD, 0xFD] {
            let prefixed = execute_one_instruction(&[prefix, opcode], true, 0xFF, 0x0145);
            assert_eq!(
                prefixed.fetch_addresses,
                vec![0, 1],
                "{prefix:02X} {opcode:02X}: prefix M1 and terminal M1 only"
            );
            assert_eq!(prefixed.wait_count, base.wait_count + 1);
            assert_eq!(prefixed.tstates, base.tstates + 5);
            assert_architectural_fallback(&base.state, &prefixed.state, prefix, opcode, 0xFF);
            assert_eq!(
                remove_prefix_m1_and_normalize_stream_addresses(prefixed.trace, prefix, opcode),
                base.trace,
                "{prefix:02X} {opcode:02X}: alternate conditional path request trace"
            );
        }
    }
}
