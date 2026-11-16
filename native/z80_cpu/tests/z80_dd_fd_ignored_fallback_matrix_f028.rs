//! F-028: exhaustive dispatch/timing smoke matrix for effective DD/FD Ignored bytes.
//!
//! The expected byte set comes from the reviewed 256-byte semantic ledger.
//! This test does not claim an independent semantic oracle: it proves that
//! each effective Ignored byte reaches the existing BASE microsequencer,
//! retires without refetching the terminal opcode, and adds exactly one 4T M1
//! prefix fetch for both DD and FD.

use sega_z80_cpu::{Z80BusOperation, Z80MicroSequencer, Z80MicroStep};

const DD_FD_LEDGER: &str =
    include_str!("../../../docs/audits/z80-dd-fd-post-prefix-byte-ledger-2026-11-16.csv");
const EXPECTED_IGNORED_BYTES: usize = 167;

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

fn execute_one_instruction(opcode_stream: &[u8]) -> (u32, Vec<u16>, u8) {
    let mut cpu = Z80MicroSequencer::default();
    let mut stream = opcode_stream.iter().copied();
    let mut opcode_fetch_addresses = Vec::new();
    let mut step = cpu.begin_instruction().expect("instruction starts");

    for _ in 0..128 {
        step = match step {
            Z80MicroStep::BusRequest(request) => {
                let data = match request.operation {
                    Z80BusOperation::OpcodeFetch => {
                        assert_eq!(
                            request.base_tstates, 4,
                            "M1 fetch must retain its four-T-state base cost"
                        );
                        opcode_fetch_addresses.push(request.address);
                        Some(stream.next().unwrap_or_else(|| {
                            panic!(
                                "unexpected opcode refetch at address {:04X}; stream={opcode_stream:02X?}",
                                request.address
                            )
                        }))
                    }
                    Z80BusOperation::InterruptAcknowledge => {
                        panic!("unexpected interrupt acknowledge in F-028 fallback matrix")
                    }
                    Z80BusOperation::MemoryRead | Z80BusOperation::PortRead => Some(0x5A),
                    Z80BusOperation::MemoryWrite { .. } | Z80BusOperation::PortWrite { .. } => {
                        None
                    }
                };
                cpu.complete_bus_cycle(request.id, data)
                    .expect("BASE bus operation completes")
            }
            Z80MicroStep::InternalCycle { .. } => cpu
                .complete_internal_cycle()
                .expect("BASE internal cycle completes"),
            Z80MicroStep::InstructionRetired { opcode, tstates } => {
                assert_eq!(
                    opcode,
                    *opcode_stream.last().expect("opcode stream is non-empty")
                );
                assert!(
                    stream.next().is_none(),
                    "instruction retired before consuming the complete opcode stream"
                );
                return (tstates, opcode_fetch_addresses, cpu.refresh & 0x7F);
            }
            Z80MicroStep::InterruptAcknowledge { .. }
            | Z80MicroStep::InterruptServiced { .. } => {
                panic!("unexpected interrupt service in F-028 fallback matrix")
            }
        };
    }

    panic!("opcode stream {opcode_stream:02X?} did not retire within 128 micro-operations");
}

#[test]
fn all_167_effective_ignored_bytes_fall_back_without_refetch_for_dd_and_fd() {
    let ignored = ignored_opcodes_from_ledger();
    assert_eq!(
        ignored.len(),
        EXPECTED_IGNORED_BYTES,
        "the effective Ignored inventory must match the reviewed 256-byte ledger"
    );

    for opcode in ignored {
        let (base_tstates, base_fetches, base_refresh) = execute_one_instruction(&[opcode]);
        assert_eq!(base_fetches, [0], "BASE opcode {opcode:02X}");
        assert_eq!(base_refresh, 1, "BASE opcode {opcode:02X}");

        for prefix in [0xDD, 0xFD] {
            let (prefixed_tstates, prefixed_fetches, prefixed_refresh) =
                execute_one_instruction(&[prefix, opcode]);
            assert_eq!(
                prefixed_fetches,
                [0, 1],
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
        }
    }
}
