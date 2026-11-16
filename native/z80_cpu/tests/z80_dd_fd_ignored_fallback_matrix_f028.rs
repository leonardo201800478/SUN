//! F-028: exhaustive dispatch and bus-trace matrix for effective DD/FD Ignored bytes.
//!
//! The expected byte set comes from the reviewed 256-byte semantic ledger.
//! This is a structural differential against the BASE microsequencer, not an
//! independent semantic oracle. It proves that each effective Ignored byte
//! reaches BASE without terminal-opcode refetch, preserves the BASE request
//! trace after normalizing PC-relative stream addresses, and adds one 4T M1.

use sega_z80_cpu::{Z80BusOperation, Z80MicroSequencer, Z80MicroStep};

const DD_FD_LEDGER: &str =
    include_str!("../../../docs/audits/z80-dd-fd-post-prefix-byte-ledger-2026-11-16.csv");
const EXPECTED_IGNORED_BYTES: usize = 167;

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
    },
    InternalCycle {
        tstates: u32,
    },
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

fn execute_one_instruction(opcode_stream: &[u8]) -> (u32, Vec<u16>, u8, Vec<TraceEvent>) {
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
    let mut opcode_fetch_addresses = Vec::new();
    let mut trace = Vec::new();
    let mut step = cpu.begin_instruction().expect("instruction starts");

    for _ in 0..128 {
        step = match step {
            Z80MicroStep::BusRequest(request) => {
                let (kind, data) = match request.operation {
                    Z80BusOperation::OpcodeFetch => {
                        assert_eq!(
                            request.base_tstates, 4,
                            "M1 fetch must retain its four-T-state base cost"
                        );
                        opcode_fetch_addresses.push(request.address);
                        (
                            BusTraceKind::OpcodeFetch,
                            Some(stream.next().unwrap_or_else(|| {
                                panic!(
                                    "unexpected opcode refetch at address {:04X}; stream={opcode_stream:?}",
                                    request.address
                                )
                            })),
                        )
                    }
                    Z80BusOperation::InterruptAcknowledge => {
                        panic!("unexpected interrupt acknowledge in F-028 fallback matrix")
                    }
                    Z80BusOperation::MemoryRead => (BusTraceKind::MemoryRead, Some(0x5A)),
                    Z80BusOperation::MemoryWrite { .. } => (BusTraceKind::MemoryWrite, None),
                    Z80BusOperation::PortRead => (BusTraceKind::PortRead, Some(0x5A)),
                    Z80BusOperation::PortWrite { .. } => (BusTraceKind::PortWrite, None),
                };
                trace.push(TraceEvent::Bus {
                    kind,
                    address: request.address,
                    base_tstates: request.base_tstates,
                });
                cpu.complete_bus_cycle(request.id, data)
                    .expect("BASE bus operation completes")
            }
            Z80MicroStep::InternalCycle { tstates } => {
                trace.push(TraceEvent::InternalCycle { tstates });
                cpu.complete_internal_cycle()
                    .expect("BASE internal cycle completes")
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
                return (tstates, opcode_fetch_addresses, cpu.refresh & 0x7F, trace);
            }
            Z80MicroStep::InterruptAcknowledge { .. } | Z80MicroStep::InterruptServiced { .. } => {
                panic!("unexpected interrupt service in F-028 fallback matrix")
            }
        };
    }

    panic!("opcode stream {opcode_stream:?} did not retire within 128 micro-operations");
}

fn remove_prefix_m1_and_normalize_pc_stream_addresses(trace: Vec<TraceEvent>) -> Vec<TraceEvent> {
    let mut events = trace.into_iter();
    assert_eq!(
        events.next(),
        Some(TraceEvent::Bus {
            kind: BusTraceKind::OpcodeFetch,
            address: 0,
            base_tstates: 4,
        }),
        "the first prefixed event must be the DD/FD M1"
    );

    let mut normalized: Vec<_> = events.collect();
    for event in &mut normalized {
        if let TraceEvent::Bus { kind, address, .. } = event
            && matches!(*kind, BusTraceKind::OpcodeFetch | BusTraceKind::MemoryRead)
            && *address < 0x0100
        {
            *address = (*address).wrapping_sub(1);
        }
    }
    normalized
}

#[test]
fn all_167_effective_ignored_bytes_preserve_base_bus_trace_for_dd_and_fd() {
    let ignored = ignored_opcodes_from_ledger();
    assert_eq!(
        ignored.len(),
        EXPECTED_IGNORED_BYTES,
        "the effective Ignored inventory must match the reviewed 256-byte ledger"
    );

    for opcode in ignored {
        let (base_tstates, base_fetches, base_refresh, base_trace) =
            execute_one_instruction(&[opcode]);
        assert_eq!(base_fetches, vec![0], "BASE opcode {opcode:02X}");
        assert_eq!(base_refresh, 1, "BASE opcode {opcode:02X}");

        for prefix in [0xDD, 0xFD] {
            let (prefixed_tstates, prefixed_fetches, prefixed_refresh, prefixed_trace) =
                execute_one_instruction(&[prefix, opcode]);
            assert_eq!(
                prefixed_fetches,
                vec![0, 1],
                "{prefix:02X} {opcode:02X}: one prefix M1 and one terminal M1 only"
            );
            assert_eq!(
                prefixed_refresh, 2,
                "{prefix:02X} {opcode:02X}: R increments for both M1 fetches"
            );
            assert_eq!(
                prefixed_tstates,
                base_tstates + 4,
                "{prefix:02X} {opcode:02X}: ignored prefix adds exactly one 4T M1"
            );
            assert_eq!(
                remove_prefix_m1_and_normalize_pc_stream_addresses(prefixed_trace),
                base_trace,
                "{prefix:02X} {opcode:02X}: BASE bus request order/timing must be preserved"
            );
        }
    }
}
