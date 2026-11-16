//! Exhaustive ED NEG, RRD and RLD semantic regressions.
//!
//! Expected values are calculated from the documented arithmetic/nibble
//! transformations, without calling production ALU helpers. These tests
//! validate architectural results and nominal instruction timing, not bus
//! event ordering or clone-specific undocumented behavior.
use sega_sms_core::{CpuBus, Z80};

const FLAG_C: u8 = 0x01;
const FLAG_N: u8 = 0x02;
const FLAG_PV: u8 = 0x04;
const FLAG_H: u8 = 0x10;
const FLAG_Z: u8 = 0x40;
const FLAG_S: u8 = 0x80;
const FLAG_XY: u8 = 0x28;

struct FlatBus {
    memory: Box<[u8]>,
}

impl Default for FlatBus {
    fn default() -> Self {
        Self {
            memory: vec![0; 0x1_0000].into_boxed_slice(),
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

fn even_parity(value: u8) -> bool {
    value.count_ones().is_multiple_of(2)
}

fn szp_xy_flags(value: u8) -> u8 {
    let mut flags = value & (FLAG_S | FLAG_XY);
    if value == 0 {
        flags |= FLAG_Z;
    }
    if even_parity(value) {
        flags |= FLAG_PV;
    }
    flags
}

#[test]
fn all_neg_aliases_are_exhaustive_for_accumulator_and_initial_flag_patterns() {
    let aliases = [0x44_u8, 0x4C, 0x54, 0x5C, 0x64, 0x6C, 0x74, 0x7C];
    let initial_flags = [0x00_u8, 0xFF, 0xA5, 0x5A];
    let mut bus = FlatBus::default();
    let mut cpu = Z80::default();

    for opcode in aliases {
        bus.memory[0] = 0xED;
        bus.memory[1] = opcode;
        for accumulator in u8::MIN..=u8::MAX {
            for flags in initial_flags {
                cpu.registers.pc = 0;
                cpu.registers.a = accumulator;
                cpu.registers.f = flags;
                cpu.wz = 0x4567;

                let result = 0_u8.wrapping_sub(accumulator);
                let mut expected_flags = (result & (FLAG_S | FLAG_XY)) | FLAG_N;
                if result == 0 {
                    expected_flags |= FLAG_Z;
                }
                if accumulator & 0x0F != 0 {
                    expected_flags |= FLAG_H;
                }
                if accumulator == 0x80 {
                    expected_flags |= FLAG_PV;
                }
                if accumulator != 0 {
                    expected_flags |= FLAG_C;
                }

                assert_eq!(cpu.step(&mut bus), Ok(8), "ED {opcode:02X}");
                assert_eq!(
                    cpu.registers.a, result,
                    "ED {opcode:02X} A={accumulator:02X}"
                );
                assert_eq!(
                    cpu.registers.f, expected_flags,
                    "ED {opcode:02X} flags, A={accumulator:02X}, F={flags:02X}"
                );
                assert_eq!(cpu.registers.pc, 2, "ED {opcode:02X} PC");
                assert_eq!(cpu.wz, 0x4567, "ED {opcode:02X} WZ");
            }
        }
    }
}

#[test]
fn rrd_and_rld_are_exhaustive_for_all_accumulator_memory_and_carry_values() {
    let mut bus = FlatBus::default();
    let mut cpu = Z80::default();

    for opcode in [0x67_u8, 0x6F] {
        bus.memory[0] = 0xED;
        bus.memory[1] = opcode;

        for accumulator in u8::MIN..=u8::MAX {
            for memory_value in u8::MIN..=u8::MAX {
                for carry_in in [false, true] {
                    bus.memory[0x8000] = memory_value;
                    cpu.registers.pc = 0;
                    cpu.registers.a = accumulator;
                    cpu.registers.f = if carry_in { FLAG_C } else { 0 };
                    cpu.registers.set_hl(0x8000);
                    cpu.wz = 0x4567;

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
                    let mut expected_flags = szp_xy_flags(expected_accumulator);
                    if carry_in {
                        expected_flags |= FLAG_C;
                    }

                    assert_eq!(cpu.step(&mut bus), Ok(18), "ED {opcode:02X}");
                    assert_eq!(
                        cpu.registers.a, expected_accumulator,
                        "ED {opcode:02X} A={accumulator:02X}, M={memory_value:02X}"
                    );
                    assert_eq!(
                        bus.memory[0x8000], expected_memory,
                        "ED {opcode:02X} memory, A={accumulator:02X}, M={memory_value:02X}"
                    );
                    assert_eq!(
                        cpu.registers.f, expected_flags,
                        "ED {opcode:02X} flags, A={accumulator:02X}, M={memory_value:02X}"
                    );
                    assert_eq!(cpu.registers.pc, 2, "ED {opcode:02X} PC");
                    assert_eq!(cpu.wz, 0x8001, "ED {opcode:02X} WZ");
                }
            }
        }
    }
}


/// RRD/RLD set MEMPTR/WZ to HL+1. The value is architecturally observable
/// through the undocumented X/Y flags of a following BIT n,(HL).
///
/// The MEMPTR rule is independently described by the community hardware
/// research and by the Mark Woodmass suite, which was compared against a
/// NEC D780C-1 and additional Z80 clones. The upstream TAP itself is not
/// executed by this test; this is a focused observable-state regression.
/// Reference: https://github.com/redcode/Z80/wiki/MEMPTR
#[test]
fn rrd_and_rld_memptr_is_observable_through_following_bit_hl_flags() {
    let pc = 0x0100_u16;
    let hl_values = [0x07FF_u16, 0x1FFF, 0x27FF, 0xFFFF];
    let accumulator_values = [0x00_u8, 0x01, 0x10, 0xFF];
    let mut bus = FlatBus::default();
    let mut cpu = Z80::default();

    for opcode in [0x67_u8, 0x6F] {
        bus.memory[usize::from(pc)] = 0xED;
        bus.memory[usize::from(pc.wrapping_add(1))] = opcode;
        bus.memory[usize::from(pc.wrapping_add(2))] = 0xCB;
        bus.memory[usize::from(pc.wrapping_add(3))] = 0x46; // BIT 0,(HL)

        for hl in hl_values {
            let expected_wz = hl.wrapping_add(1);
            for accumulator in accumulator_values {
                for memory_value in u8::MIN..=u8::MAX {
                    for carry_in in [false, true] {
                        bus.memory[usize::from(hl)] = memory_value;
                        cpu.registers.pc = pc;
                        cpu.registers.a = accumulator;
                        cpu.registers.f = if carry_in { FLAG_C } else { 0 };
                        cpu.registers.set_hl(hl);
                        cpu.wz = 0x4567;

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

                        assert_eq!(cpu.step(&mut bus), Ok(18), "ED {opcode:02X}");
                        assert_eq!(cpu.wz, expected_wz, "ED {opcode:02X}: WZ");
                        assert_eq!(cpu.registers.a, expected_accumulator, "ED {opcode:02X}: A");
                        assert_eq!(
                            bus.memory[usize::from(hl)],
                            expected_memory,
                            "ED {opcode:02X}: memory"
                        );

                        let mut expected_flags = FLAG_H | (expected_wz >> 8) as u8 & FLAG_XY;
                        if carry_in {
                            expected_flags |= FLAG_C;
                        }
                        if expected_memory & 0x01 == 0 {
                            expected_flags |= FLAG_Z | FLAG_PV;
                        }

                        assert_eq!(cpu.step(&mut bus), Ok(12), "BIT 0,(HL) after ED {opcode:02X}");
                        assert_eq!(
                            cpu.registers.f, expected_flags,
                            "ED {opcode:02X} then BIT 0,(HL), HL={hl:04X}, A={accumulator:02X}, M={memory_value:02X}, C={carry_in}"
                        );
                        assert_eq!(cpu.q, expected_flags, "ED {opcode:02X}: Q after BIT");
                        assert_eq!(cpu.wz, expected_wz, "ED {opcode:02X}: WZ after BIT");
                        assert_eq!(cpu.registers.hl(), hl, "ED {opcode:02X}: HL preserved");
                        assert_eq!(cpu.registers.pc, pc.wrapping_add(4), "ED {opcode:02X}: PC");
                        assert_eq!(cpu.total_tstates, 30, "ED {opcode:02X}: total T-states");
                    }
                }
            }
        }
    }
}
