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

fn execute_one_instruction(opcode_stream: &[u8], wait_and_snapshot: bool) -> Execution {
    let mut cpu = Z80MicroSequencer::default();
    cpu.registers.set_bc(0x2345);
    cpu.registers.set_de(0x3456);
    cpu.registers.set_hl(0x4567);
    cpu.registers.sp = 0x5678;
    cpu.registers.a = 0xA5;
    cpu.registers.f = 0x00;
    cpu.ix = 0x6789;
    cpu.iy = 0x789A;

    let mut stream = opcode_stream.iter().copied();
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
                    Z80BusOperation::MemoryRead => request.address == cpu.registers.pc,
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
                assert_eq!(
                    opcode,
                    *opcode_stream.last().expect("opcode stream is non-empty")
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

fn assert_architectural_fallback(
    base: &Z80ArchitecturalState,
    prefixed: &Z80ArchitecturalState,
    prefix: u8,
    opcode: u8,
) {
    let mut expected = *base;
    expected.r = increment_refresh(base.r);

    // Control-flow instructions may land at the same absolute target (delta 0)
    // or at the correspondingly shifted stream-relative target (delta 1).
    let pc_delta = prefixed.registers.pc.wrapping_sub(base.registers.pc);
    assert!(
        pc_delta <= 1,
        "{prefix:02X} {opcode:02X}: prefix may shift final PC by at most one byte, got {pc_delta}"
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
fn all_167_ignored_fallbacks_preserve_architecture_requests_wait_and_snapshot() {
    let ignored = ignored_opcodes_from_ledger();
    assert_eq!(
        ignored.len(),
        EXPECTED_IGNORED_BYTES,
        "effective Ignored inventory must match the reviewed ledger"
    );

    for opcode in ignored {
        let base = execute_one_instruction(&[opcode], true);
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
            let prefixed = execute_one_instruction(&[prefix, opcode], true);
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
            assert_architectural_fallback(&base.state, &prefixed.state, prefix, opcode);
            assert_eq!(
                remove_prefix_m1_and_normalize_stream_addresses(prefixed.trace, prefix, opcode),
                base.trace,
                "{prefix:02X} {opcode:02X}: request type/order/address/payload/timing and internal cycles"
            );
        }
    }
}
