//! F-022: observe block-repeat flags at the actual IM2 acceptance boundary.
//!
//! The scenarios mirror the hardware-derived interrupted-block behavior
//! documented by Peter Helcmanovsky and David Banks. This is a deterministic
//! CPU-core integration test, not a replacement for running the complete
//! ZX Spectrum v5.0 ROM or testing a physical NMOS/CMOS part.

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
    ports: Box<[u8]>,
}

impl Default for FlatBus {
    fn default() -> Self {
        Self {
            memory: vec![0; 0x1_0000].into_boxed_slice(),
            ports: vec![0; 0x1_0000].into_boxed_slice(),
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

    fn read_port(&mut self, address: u16) -> u8 {
        self.ports[usize::from(address)]
    }

    fn write_port(&mut self, address: u16, value: u8) {
        self.ports[usize::from(address)] = value;
    }
}

fn expected_repeat_flags(
    opcode: u8,
    initial_f: u8,
    a: u8,
    value: u8,
    bc_after: u16,
    pc: u16,
) -> u8 {
    let pc_xy = ((pc >> 8) as u8) & FLAG_XY;
    if matches!(opcode, 0xB0 | 0xB8) {
        (initial_f & (FLAG_S | FLAG_Z | FLAG_C)) | if bc_after != 0 { FLAG_PV } else { 0 } | pc_xy
    } else {
        let result = a.wrapping_sub(value);
        let half_borrow = (a & 0x0F) < (value & 0x0F);
        (initial_f & FLAG_C)
            | FLAG_N
            | (result & FLAG_S)
            | if result == 0 { FLAG_Z } else { 0 }
            | if half_borrow { FLAG_H } else { 0 }
            | if bc_after != 0 { FLAG_PV } else { 0 }
            | pc_xy
    }
}

#[test]
fn ldir_lddr_cpir_cpdr_flags_survive_im2_acceptance_after_repeat() {
    let pcs = [0x8002_u16, 0x8802, 0xA002, 0xA802];
    let opcodes = [0xB0_u8, 0xB8, 0xB1, 0xB9];

    for opcode in opcodes {
        let transfer = matches!(opcode, 0xB0 | 0xB8);
        let decrement = matches!(opcode, 0xB8 | 0xB9);
        let direction = if decrement { -1_i16 } else { 1_i16 };
        let accumulator = if transfer { 0x01 } else { 0x10 };
        let value = if transfer { 0x02 } else { 0x20 };
        let initial_f = if transfer { 0xC1 } else { FLAG_C };

        for pc in pcs {
            for initial_bc in [0_u16, 2] {
                let mut bus = FlatBus::default();
                bus.memory[usize::from(pc)] = 0xED;
                bus.memory[usize::from(pc.wrapping_add(1))] = opcode;
                bus.memory[0xE100] = value;
                bus.memory[0xD0FF] = 0x00;
                bus.memory[0xD100] = 0xE0;

                let mut cpu = Z80::default();
                cpu.registers.pc = pc;
                cpu.registers.sp = 0xF000;
                cpu.registers.a = accumulator;
                cpu.registers.f = initial_f;
                cpu.registers.set_bc(initial_bc);
                cpu.registers.set_hl(0xE100);
                cpu.registers.set_de(0xE200);
                cpu.i = 0xD0;
                cpu.interrupt_mode = 2;
                cpu.iff1 = true;
                cpu.iff2 = true;

                let bc_after = initial_bc.wrapping_sub(1);
                let expected_f =
                    expected_repeat_flags(opcode, initial_f, accumulator, value, bc_after, pc);

                assert_eq!(cpu.step(&mut bus), Ok(21), "ED {opcode:02X} PC={pc:04X}");
                assert_eq!(cpu.registers.pc, pc, "ED {opcode:02X}: repeat PC");
                assert_eq!(cpu.wz, pc.wrapping_add(1), "ED {opcode:02X}: repeat WZ");
                assert_eq!(cpu.registers.bc(), bc_after, "ED {opcode:02X}: BC");
                assert_eq!(cpu.registers.f, expected_f, "ED {opcode:02X}: repeat F");
                assert_eq!(cpu.q, expected_f, "ED {opcode:02X}: repeat Q");
                assert_eq!(
                    cpu.registers.hl(),
                    0xE100_u16.wrapping_add_signed(direction),
                    "ED {opcode:02X}: HL"
                );
                if transfer {
                    assert_eq!(
                        cpu.registers.de(),
                        0xE200_u16.wrapping_add_signed(direction),
                        "ED {opcode:02X}: DE"
                    );
                    assert_eq!(
                        bus.memory[0xE200], value,
                        "ED {opcode:02X}: transferred byte"
                    );
                } else {
                    assert_eq!(cpu.registers.de(), 0xE200, "ED {opcode:02X}: DE preserved");
                }

                // The ROM's IM2 handler observes F after the repeat iteration.
                // Accept INT at the next instruction boundary and ensure entry
                // does not rewrite the flags that the handler is meant to inspect.
                cpu.request_int(0xFF);
                assert_eq!(cpu.step(&mut bus), Ok(19), "ED {opcode:02X}: IM2 entry");
                assert_eq!(cpu.registers.pc, 0xE000, "ED {opcode:02X}: IM2 vector");
                assert_eq!(cpu.wz, 0xE000, "ED {opcode:02X}: IM2 WZ");
                assert_eq!(cpu.registers.sp, 0xEFFE, "ED {opcode:02X}: stack pointer");
                assert_eq!(bus.memory[0xEFFE], pc as u8, "ED {opcode:02X}: stacked PCL");
                assert_eq!(
                    bus.memory[0xEFFF],
                    (pc >> 8) as u8,
                    "ED {opcode:02X}: stacked PCH"
                );
                assert_eq!(
                    cpu.registers.f, expected_f,
                    "ED {opcode:02X}: ISR-visible F"
                );
                assert_eq!(cpu.r, 0x03, "ED {opcode:02X}: refresh");
                assert!(!cpu.iff1);
                assert!(!cpu.iff2);
                assert_eq!(cpu.int_pending, None);
                assert_eq!(cpu.total_tstates, 40);
            }
        }
    }
}
