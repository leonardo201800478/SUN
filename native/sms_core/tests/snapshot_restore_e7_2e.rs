use sega_sms_core::SmsTemporalMachine;
use sega_temporal_core::MachineTime;
use sega_z80_cpu::{
    Z80InterruptKind, Z80MicroSequencerPhase, Z80MicroSequencerState, Z80MicroStep,
};

fn machine(rom: Vec<u8>) -> SmsTemporalMachine {
    SmsTemporalMachine::new(rom, 3).unwrap()
}

fn snapshot(machine: &SmsTemporalMachine) -> SmsTemporalMachine {
    let mut state = machine.export_state();
    let bytes = state.cpu.encode().unwrap_or_else(|error| {
        panic!(
            "CPU snapshot encode failed: {error:?}; CPU state: {:#?}",
            state.cpu
        )
    });
    let decoded = Z80MicroSequencerState::decode(&bytes).unwrap();
    assert_eq!(decoded, state.cpu);
    state.cpu = decoded;
    let mut restored = SmsTemporalMachine::restore_state(state.clone()).unwrap_or_else(|error| {
        panic!("SMS machine snapshot restore failed: {error:?}; machine state: {state:#?}")
    });
    restored.set_trace_enabled(true);
    restored
}

// Snapshot only at scheduler-owned safe points, after an event has been scheduled.
fn run_boundary(
    machine: &mut SmsTemporalMachine,
    snapshot_each: bool,
    waits: u32,
) -> (
    Z80MicroStep,
    Vec<sega_sms_core::temporal_machine::SmsTemporalTraceEvent>,
) {
    let mut captured_trace = Vec::new();
    let mut step = machine.begin_instruction().unwrap();
    for _ in 0..128 {
        step = match step {
            Z80MicroStep::BusRequest(_) => {
                machine.schedule_pending_bus(waits).unwrap();
                if snapshot_each {
                    captured_trace.extend(machine.take_trace());
                    *machine = snapshot(machine);
                }
                machine.dispatch_next().unwrap()
            }
            Z80MicroStep::InternalCycle { tstates } => {
                machine.schedule_internal_cycle(tstates).unwrap();
                if snapshot_each {
                    captured_trace.extend(machine.take_trace());
                    *machine = snapshot(machine);
                }
                machine.dispatch_next().unwrap()
            }
            Z80MicroStep::InterruptAcknowledge { .. } => {
                if snapshot_each {
                    captured_trace.extend(machine.take_trace());
                    *machine = snapshot(machine);
                }
                machine.cpu_mut().begin_instruction().unwrap()
            }
            terminal @ (Z80MicroStep::InstructionRetired { .. }
            | Z80MicroStep::InterruptServiced { .. }) => {
                captured_trace.extend(machine.take_trace());
                if snapshot_each {
                    *machine = snapshot(machine);
                }
                return (terminal, captured_trace);
            }
        };
    }
    panic!("microsequencer did not reach an instruction boundary");
}

fn compare_runs(
    continuous: &mut SmsTemporalMachine,
    resumed: &mut SmsTemporalMachine,
    waits: u32,
) -> Z80MicroStep {
    continuous.set_trace_enabled(true);
    resumed.set_trace_enabled(true);
    let (expected, continuous_trace) = run_boundary(continuous, false, waits);
    let (actual, resumed_trace) = run_boundary(resumed, true, waits);
    assert_eq!(actual, expected);
    assert_eq!(
        resumed_trace, continuous_trace,
        "continuous and restored Scheduler traces must match"
    );
    assert_eq!(resumed.export_state(), continuous.export_state());
    assert_eq!(resumed.now(), continuous.now());
    actual
}

fn run_boundary_with_repeat_interrupt(
    machine: &mut SmsTemporalMachine,
    snapshot_each: bool,
    waits: u32,
) -> (
    Z80MicroStep,
    Vec<sega_sms_core::temporal_machine::SmsTemporalTraceEvent>,
) {
    let mut captured_trace = Vec::new();
    let mut step = machine.begin_instruction().unwrap();
    for _ in 0..128 {
        step = match step {
            Z80MicroStep::BusRequest(_) => {
                machine.schedule_pending_bus(waits).unwrap();
                if snapshot_each {
                    captured_trace.extend(machine.take_trace());
                    *machine = snapshot(machine);
                }
                machine.dispatch_next().unwrap()
            }
            Z80MicroStep::InternalCycle { tstates } => {
                if matches!(
                    machine.cpu().export_state().phase,
                    Z80MicroSequencerPhase::EdBlockRepeatInternal {
                        remaining_tstates: 1,
                        ..
                    } | Z80MicroSequencerPhase::EdBlockCompareRepeatInternal {
                        remaining_tstates: 1,
                        ..
                    }
                ) {
                    // Assert level-sensitive INT immediately before the final
                    // repeat T-state; the signal change itself consumes no time.
                    machine.set_int_line(Some(0xFF));
                }
                machine.schedule_internal_cycle(tstates).unwrap();
                if snapshot_each {
                    captured_trace.extend(machine.take_trace());
                    *machine = snapshot(machine);
                }
                machine.dispatch_next().unwrap()
            }
            Z80MicroStep::InterruptAcknowledge { .. } => {
                if snapshot_each {
                    captured_trace.extend(machine.take_trace());
                    *machine = snapshot(machine);
                }
                machine.cpu_mut().begin_instruction().unwrap()
            }
            terminal @ (Z80MicroStep::InstructionRetired { .. }
            | Z80MicroStep::InterruptServiced { .. }) => {
                captured_trace.extend(machine.take_trace());
                if snapshot_each {
                    *machine = snapshot(machine);
                }
                return (terminal, captured_trace);
            }
        };
    }
    panic!("repeat/interrupt sequence did not reach a boundary");
}

fn compare_repeat_interrupt_runs(
    continuous: &mut SmsTemporalMachine,
    resumed: &mut SmsTemporalMachine,
    waits: u32,
) -> Z80MicroStep {
    continuous.set_trace_enabled(true);
    resumed.set_trace_enabled(true);
    let (expected, continuous_trace) = run_boundary_with_repeat_interrupt(continuous, false, waits);
    let (actual, resumed_trace) = run_boundary_with_repeat_interrupt(resumed, true, waits);
    assert_eq!(actual, expected);
    assert_eq!(
        resumed_trace, continuous_trace,
        "continuous and restored repeat/INT Scheduler traces must match"
    );
    assert_eq!(resumed.export_state(), continuous.export_state());
    assert_eq!(resumed.now(), continuous.now());
    actual
}

fn compare_program_with_waits(
    rom: Vec<u8>,
    waits: u32,
    configure: impl Fn(&mut SmsTemporalMachine),
) -> Z80MicroStep {
    let mut continuous = machine(rom.clone());
    let mut resumed = machine(rom);
    configure(&mut continuous);
    configure(&mut resumed);
    compare_runs(&mut continuous, &mut resumed, waits)
}

#[test]
fn e7_2e_nmi_restores_response_internal_and_stack_phases_with_waits() {
    let mut continuous = machine(vec![0; 0x4000]);
    let mut resumed = machine(vec![0; 0x4000]);
    for cpu in [&mut continuous, &mut resumed] {
        cpu.cpu_mut().registers.pc = 0x1234;
        cpu.cpu_mut().registers.sp = 0xC100;
        cpu.cpu_mut().refresh = 0xFE;
        cpu.cpu_mut().iff1 = true;
        cpu.request_nmi();
    }
    assert_eq!(
        compare_runs(&mut continuous, &mut resumed, 1),
        Z80MicroStep::InterruptServiced {
            kind: Z80InterruptKind::Nmi,
            tstates: 14,
        }
    );
    assert_eq!(resumed.cpu().registers.pc, 0x0066);
    assert_eq!(resumed.cpu().registers.sp, 0xC0FE);
    assert_eq!(resumed.memory().read(0xC0FF), 0x12);
    assert_eq!(resumed.memory().read(0xC0FE), 0x34);
    assert_eq!(resumed.now(), MachineTime::from_ticks(42));
}

#[test]
fn e7_2e_im0_restores_acknowledge_and_injected_rst() {
    let mut continuous = machine(vec![0; 0x4000]);
    let mut resumed = machine(vec![0; 0x4000]);
    for cpu in [&mut continuous, &mut resumed] {
        cpu.cpu_mut().iff1 = true;
        cpu.cpu_mut().iff2 = true;
        cpu.cpu_mut().registers.pc = 0x2345;
        cpu.cpu_mut().registers.sp = 0xC100;
        cpu.cpu_mut().wz = 0xA55A;
        cpu.set_int_line(Some(0xC7));
    }
    // ZiLOG UM008011-0816, pp. 13 and 17: IM0 adds two automatic
    // wait states to the supplied instruction's normal execution time.
    // RST is 11T; the three bus transactions each receive one fixture WAIT.
    assert_eq!(
        compare_runs(&mut continuous, &mut resumed, 1),
        Z80MicroStep::InstructionRetired {
            opcode: 0xC7,
            tstates: 16,
        }
    );
    assert_eq!(resumed.cpu().registers.pc, 0x0000);
    assert_eq!(resumed.cpu().wz, 0x0000); // injected RST vector
    assert_eq!(resumed.cpu().registers.sp, 0xC0FE);
    assert_eq!(resumed.memory().read(0xC0FF), 0x23);
    assert_eq!(resumed.memory().read(0xC0FE), 0x45);
}

#[test]
fn e7_2e_im1_restores_dispatch_and_both_stack_writes() {
    let mut continuous = machine(vec![0; 0x4000]);
    let mut resumed = machine(vec![0; 0x4000]);
    for cpu in [&mut continuous, &mut resumed] {
        cpu.cpu_mut().interrupt_mode = 1;
        cpu.cpu_mut().iff1 = true;
        cpu.cpu_mut().iff2 = true;
        cpu.cpu_mut().registers.pc = 0x4567;
        cpu.cpu_mut().registers.sp = 0xC100;
        cpu.cpu_mut().wz = 0xA55A;
        cpu.set_int_line(Some(0xFF));
    }
    assert_eq!(
        compare_runs(&mut continuous, &mut resumed, 1),
        Z80MicroStep::InterruptServiced {
            kind: Z80InterruptKind::Int,
            tstates: 16,
        }
    );
    assert_eq!(resumed.cpu().registers.pc, 0x0038);
    assert_eq!(resumed.cpu().wz, 0xA55A); // IM1 dispatch preserves prior WZ
    assert_eq!(resumed.cpu().registers.sp, 0xC0FE);
    assert_eq!(resumed.memory().read(0xC0FF), 0x45);
    assert_eq!(resumed.memory().read(0xC0FE), 0x67);
}

#[test]
fn e7_2e_im2_restores_stack_and_both_vector_reads() {
    let mut rom = vec![0; 0x4000];
    rom[0x2080] = 0x78;
    rom[0x2081] = 0x56;
    let mut continuous = machine(rom.clone());
    let mut resumed = machine(rom);
    for cpu in [&mut continuous, &mut resumed] {
        cpu.cpu_mut().interrupt_mode = 2;
        cpu.cpu_mut().iff1 = true;
        cpu.cpu_mut().iff2 = true;
        cpu.cpu_mut().i = 0x20;
        cpu.cpu_mut().registers.pc = 0x4567;
        cpu.cpu_mut().registers.sp = 0xC100;
        cpu.set_int_line(Some(0x80));
    }
    assert_eq!(
        compare_runs(&mut continuous, &mut resumed, 1),
        Z80MicroStep::InterruptServiced {
            kind: Z80InterruptKind::Int,
            tstates: 24,
        }
    );
    assert_eq!(resumed.cpu().registers.pc, 0x5678);
    assert_eq!(resumed.cpu().wz, 0x5678);
    assert_eq!(resumed.cpu().registers.sp, 0xC0FE);
    assert_eq!(resumed.memory().read(0xC0FF), 0x45);
    assert_eq!(resumed.memory().read(0xC0FE), 0x67);
}

#[test]
fn e7_2e_ei_delay_survives_machine_snapshot_until_following_instruction() {
    let rom = vec![0xFB, 0x00, 0x00, 0x00];
    let mut continuous = machine(rom.clone());
    let mut resumed = machine(rom);
    for cpu in [&mut continuous, &mut resumed] {
        cpu.cpu_mut().interrupt_mode = 1;
        cpu.set_int_line(Some(0xFF));
    }
    assert_eq!(
        compare_runs(&mut continuous, &mut resumed, 0),
        Z80MicroStep::InstructionRetired {
            opcode: 0xFB,
            tstates: 4
        }
    );
    assert_eq!(resumed.cpu().ei_delay, 1);
    assert_eq!(
        compare_runs(&mut continuous, &mut resumed, 0),
        Z80MicroStep::InstructionRetired {
            opcode: 0x00,
            tstates: 4
        }
    );
    assert_eq!(resumed.cpu().ei_delay, 0);
    assert_eq!(
        compare_runs(&mut continuous, &mut resumed, 0),
        Z80MicroStep::InterruptServiced {
            kind: Z80InterruptKind::Int,
            tstates: 13
        }
    );
}

#[test]
fn e7_2e_halt_m1_hold_restores_each_tstate_and_ignores_data() {
    let rom = vec![0x76, 0xDD, 0x00, 0x00];
    let mut continuous = machine(rom.clone());
    let mut resumed = machine(rom);
    assert_eq!(
        compare_runs(&mut continuous, &mut resumed, 1),
        Z80MicroStep::InstructionRetired {
            opcode: 0x76,
            tstates: 5
        }
    );
    assert!(resumed.cpu().halted);
    assert_eq!(
        compare_runs(&mut continuous, &mut resumed, 1),
        Z80MicroStep::InstructionRetired {
            opcode: 0x00,
            tstates: 5
        }
    );
    assert!(resumed.cpu().halted);
    assert_eq!(resumed.cpu().registers.pc, 1);
    assert_eq!(resumed.cpu().refresh, 2);
}

#[test]
fn e7_2e_nmi_restore_matches_trace_with_zero_and_multiple_wait_states() {
    for waits in [0, 3] {
        let mut continuous = machine(vec![0; 0x4000]);
        let mut resumed = machine(vec![0; 0x4000]);
        for cpu in [&mut continuous, &mut resumed] {
            cpu.cpu_mut().registers.pc = 0x2345;
            cpu.cpu_mut().registers.sp = 0xC100;
            cpu.cpu_mut().iff1 = true;
            cpu.request_nmi();
        }
        assert!(matches!(
            compare_runs(&mut continuous, &mut resumed, waits),
            Z80MicroStep::InterruptServiced {
                kind: Z80InterruptKind::Nmi,
                ..
            }
        ));
    }
}
#[test]
fn e7_2e_wait_matrix_covers_m1_memory_io_internal_and_interrupt_cycles() {
    for waits in [0, 1, 3] {
        assert_eq!(
            compare_program_with_waits(vec![0x00], waits, |_| {}),
            Z80MicroStep::InstructionRetired {
                opcode: 0x00,
                tstates: 4 + waits,
            }
        );

        assert_eq!(
            compare_program_with_waits(vec![0x7E], waits, |machine| {
                machine.cpu_mut().registers.h = 0xC0;
                machine.cpu_mut().registers.l = 0x00;
            }),
            Z80MicroStep::InstructionRetired {
                opcode: 0x7E,
                tstates: 7 + 2 * waits,
            }
        );

        assert_eq!(
            compare_program_with_waits(vec![0x77], waits, |machine| {
                machine.cpu_mut().registers.h = 0xC0;
                machine.cpu_mut().registers.l = 0x00;
            }),
            Z80MicroStep::InstructionRetired {
                opcode: 0x77,
                tstates: 7 + 2 * waits,
            }
        );

        assert_eq!(
            compare_program_with_waits(vec![0xDB, 0x10], waits, |_| {}),
            Z80MicroStep::InstructionRetired {
                opcode: 0xDB,
                tstates: 11 + 3 * waits,
            }
        );

        assert_eq!(
            compare_program_with_waits(vec![0xD3, 0x10], waits, |_| {}),
            Z80MicroStep::InstructionRetired {
                opcode: 0xD3,
                tstates: 11 + 3 * waits,
            }
        );

        assert_eq!(
            compare_program_with_waits(vec![0x03], waits, |_| {}),
            Z80MicroStep::InstructionRetired {
                opcode: 0x03,
                tstates: 6 + waits,
            }
        );

        assert_eq!(
            compare_program_with_waits(vec![0; 0x4000], waits, |machine| {
                machine.cpu_mut().interrupt_mode = 0;
                machine.cpu_mut().iff1 = true;
                machine.cpu_mut().iff2 = true;
                machine.cpu_mut().registers.pc = 0x2345;
                machine.cpu_mut().registers.sp = 0xC100;
                machine.set_int_line(Some(0xC7));
            }),
            Z80MicroStep::InstructionRetired {
                opcode: 0xC7,
                tstates: 13 + 3 * waits,
            }
        );

        assert_eq!(
            compare_program_with_waits(vec![0; 0x4000], waits, |machine| {
                machine.cpu_mut().interrupt_mode = 1;
                machine.cpu_mut().iff1 = true;
                machine.cpu_mut().iff2 = true;
                machine.cpu_mut().registers.pc = 0x2345;
                machine.cpu_mut().registers.sp = 0xC100;
                machine.set_int_line(Some(0xFF));
            }),
            Z80MicroStep::InterruptServiced {
                kind: Z80InterruptKind::Int,
                tstates: 13 + 3 * waits,
            }
        );

        let mut im2_rom = vec![0; 0x4000];
        im2_rom[0x2080] = 0x78;
        im2_rom[0x2081] = 0x56;
        assert_eq!(
            compare_program_with_waits(im2_rom, waits, |machine| {
                machine.cpu_mut().interrupt_mode = 2;
                machine.cpu_mut().iff1 = true;
                machine.cpu_mut().iff2 = true;
                machine.cpu_mut().i = 0x20;
                machine.cpu_mut().registers.pc = 0x2345;
                machine.cpu_mut().registers.sp = 0xC100;
                machine.set_int_line(Some(0x80));
            }),
            Z80MicroStep::InterruptServiced {
                kind: Z80InterruptKind::Int,
                tstates: 19 + 5 * waits,
            }
        );
    }
}

#[test]
fn e7_f022_repeat_forms_match_scheduler_wait_snapshot_replay() {
    // The same four repeat forms already have direct microsequencer coverage.
    // This integration fixture exercises them through SmsTemporalMachine and
    // MachineScheduler, with one WAIT on every bus request and a machine/CPU
    // snapshot round-trip at every bus and internal-cycle safe point.
    let cases = [
        // opcode, initial HL, initial DE, initial F, expected HL, expected DE, final WZ
        (
            0xB0_u8, 0x0100_u16, 0xC000_u16, 0xC1_u8, 0x0102_u16, 0xC002_u16, 0x0001_u16,
        ),
        (
            0xB8_u8, 0x0101_u16, 0xC001_u16, 0xC1_u8, 0x00FF_u16, 0xBFFF_u16, 0x0001_u16,
        ),
        (
            0xB1_u8, 0x0100_u16, 0xC000_u16, 0x01_u8, 0x0102_u16, 0xC000_u16, 0x0002_u16,
        ),
        (
            0xB9_u8, 0x0101_u16, 0xC001_u16, 0x01_u8, 0x00FF_u16, 0xC001_u16, 0x0000_u16,
        ),
    ];

    for (opcode, initial_hl, initial_de, initial_f, expected_hl, expected_de, expected_wz) in cases
    {
        let mut rom = vec![0; 0x4000];
        rom[0] = 0xED;
        rom[1] = opcode;
        rom[0x0100] = 0x21;
        rom[0x0101] = 0x22;

        let mut continuous = machine(rom.clone());
        let mut resumed = machine(rom);
        for candidate in [&mut continuous, &mut resumed] {
            let cpu = candidate.cpu_mut();
            cpu.registers.set_hl(initial_hl);
            cpu.registers.set_de(initial_de);
            cpu.registers.set_bc(2);
            cpu.registers.a = 0x10;
            cpu.registers.f = initial_f;
            cpu.wz = 0xA55A;
        }

        // First iteration repeats: PC rewinds and BC remains non-zero.
        let first = compare_runs(&mut continuous, &mut resumed, 1);
        assert!(matches!(
            first,
            Z80MicroStep::InstructionRetired { opcode: retired, .. } if retired == opcode
        ));
        assert_eq!(
            continuous.cpu().registers.pc,
            0,
            "ED {opcode:02X}: repeat PC"
        );
        assert_eq!(
            continuous.cpu().registers.bc(),
            1,
            "ED {opcode:02X}: first BC"
        );
        assert_eq!(continuous.cpu().wz, 1, "ED {opcode:02X}: repeat WZ");
        assert_eq!(
            continuous.cpu().q,
            continuous.cpu().registers.f,
            "ED {opcode:02X}: repeat Q"
        );
        let repeat_q = continuous.cpu().q;
        let direction = if matches!(opcode, 0xB8 | 0xB9) { -1 } else { 1 };
        assert_eq!(
            continuous.cpu().registers.hl(),
            initial_hl.wrapping_add_signed(direction),
            "ED {opcode:02X}: first HL"
        );

        // Second iteration terminates: PC advances past ED/opcode and BC reaches zero.
        let second = compare_runs(&mut continuous, &mut resumed, 1);
        assert!(matches!(
            second,
            Z80MicroStep::InstructionRetired { opcode: retired, .. } if retired == opcode
        ));
        assert_eq!(
            continuous.cpu().registers.pc,
            2,
            "ED {opcode:02X}: final PC"
        );
        assert_eq!(
            continuous.cpu().registers.bc(),
            0,
            "ED {opcode:02X}: final BC"
        );
        assert_eq!(
            continuous.cpu().registers.hl(),
            expected_hl,
            "ED {opcode:02X}: final HL"
        );
        assert_eq!(
            continuous.cpu().registers.de(),
            expected_de,
            "ED {opcode:02X}: final DE"
        );
        assert_eq!(
            continuous.cpu().wz,
            expected_wz,
            "ED {opcode:02X}: final WZ"
        );
        assert_eq!(
            continuous.cpu().q_previous,
            repeat_q,
            "ED {opcode:02X}: previous-Q after resumed iteration"
        );
        assert_eq!(
            continuous.cpu().q,
            continuous.cpu().registers.f,
            "ED {opcode:02X}: final Q"
        );
        assert_eq!(continuous.export_state(), resumed.export_state());
        assert_eq!(continuous.now(), resumed.now());
    }
}

#[test]
fn e7_f022_repeat_interrupt_is_sampled_and_restored_by_sms_scheduler() {
    // IM1 handler: RETI back to the rewound block instruction at PC=0000.
    let cases = [
        // opcode, initial HL, initial DE, initial F, expected HL, expected DE, final WZ
        (
            0xB0_u8, 0x0100_u16, 0xC000_u16, 0xC1_u8, 0x0102_u16, 0xC002_u16, 0x0000_u16,
        ),
        (
            0xB8_u8, 0x0101_u16, 0xC001_u16, 0xC1_u8, 0x00FF_u16, 0xBFFF_u16, 0x0000_u16,
        ),
        (
            0xB1_u8, 0x0100_u16, 0xC000_u16, 0x01_u8, 0x0102_u16, 0xC000_u16, 0x0001_u16,
        ),
        (
            0xB9_u8, 0x0101_u16, 0xC001_u16, 0x01_u8, 0x00FF_u16, 0xC001_u16, 0xFFFF_u16,
        ),
    ];

    for (opcode, initial_hl, initial_de, initial_f, expected_hl, expected_de, expected_wz) in cases
    {
        let mut rom = vec![0; 0x4000];
        rom[0] = 0xED;
        rom[1] = opcode;
        rom[0x38] = 0xED;
        rom[0x39] = 0x4D;
        rom[0x0100] = 0x21;
        rom[0x0101] = 0x22;

        let mut continuous = machine(rom.clone());
        let mut resumed = machine(rom);
        for candidate in [&mut continuous, &mut resumed] {
            let cpu = candidate.cpu_mut();
            cpu.registers.set_hl(initial_hl);
            cpu.registers.set_de(initial_de);
            cpu.registers.set_bc(2);
            cpu.registers.sp = 0xC100;
            cpu.registers.a = 0x10;
            cpu.registers.f = initial_f;
            cpu.interrupt_mode = 1;
            cpu.iff1 = true;
            cpu.iff2 = true;
            cpu.wz = 0xA55A;
        }

        // Repeat iteration retires with PC rewound; INT was asserted on its
        // final internal T-state and is pending at the next boundary.
        let first = compare_repeat_interrupt_runs(&mut continuous, &mut resumed, 1);
        assert!(matches!(
            first,
            Z80MicroStep::InstructionRetired { opcode: retired, .. } if retired == opcode
        ));
        assert_eq!(
            continuous.cpu().registers.pc,
            0,
            "ED {opcode:02X}: repeat PC"
        );
        assert_eq!(
            continuous.cpu().registers.bc(),
            1,
            "ED {opcode:02X}: repeat BC"
        );
        assert_eq!(continuous.cpu().wz, 1, "ED {opcode:02X}: repeat WZ");
        assert_eq!(
            continuous.cpu().q,
            continuous.cpu().registers.f,
            "ED {opcode:02X}: repeat Q"
        );
        let repeat_q = continuous.cpu().q;

        // The scheduler now accepts the pending INT in IM1. The stack captures
        // the rewound PC=0000, and IM1 preserves the repeat's WZ.
        assert_eq!(
            compare_runs(&mut continuous, &mut resumed, 1),
            Z80MicroStep::InterruptServiced {
                kind: Z80InterruptKind::Int,
                tstates: 16,
            }
        );
        assert_eq!(continuous.cpu().registers.pc, 0x0038);
        assert_eq!(continuous.cpu().registers.sp, 0xC0FE);
        assert_eq!(continuous.cpu().wz, 1);
        assert_eq!(continuous.cpu().q, 0);
        assert_eq!(continuous.cpu().q_previous, repeat_q);
        assert_eq!(continuous.memory().read(0xC0FF), 0x00);
        assert_eq!(continuous.memory().read(0xC0FE), 0x00);

        // Deassert the level before RETI, then return to the rewound block.
        continuous.set_int_line(None);
        resumed.set_int_line(None);
        let reti = compare_runs(&mut continuous, &mut resumed, 1);
        assert!(matches!(reti, Z80MicroStep::InstructionRetired { .. }));
        assert_eq!(continuous.cpu().registers.pc, 0x0000);
        assert_eq!(continuous.cpu().registers.sp, 0xC100);
        assert_eq!(continuous.cpu().wz, 0x0000); // RETI loads WZ from popped PC

        // The resumed final iteration terminates the block instruction.
        let final_iteration = compare_runs(&mut continuous, &mut resumed, 1);
        assert!(matches!(
            final_iteration,
            Z80MicroStep::InstructionRetired { opcode: retired, .. } if retired == opcode
        ));
        assert_eq!(continuous.cpu().registers.pc, 2);
        assert_eq!(continuous.cpu().registers.bc(), 0);
        assert_eq!(continuous.cpu().registers.hl(), expected_hl);
        assert_eq!(continuous.cpu().registers.de(), expected_de);
        assert_eq!(continuous.cpu().wz, expected_wz);
        assert_eq!(
            continuous.cpu().q,
            continuous.cpu().registers.f,
            "ED {opcode:02X}: final Q"
        );
        assert_eq!(
            continuous.cpu().q_previous,
            0,
            "ED {opcode:02X}: final previous-Q"
        );
        assert_eq!(continuous.export_state(), resumed.export_state());
        assert_eq!(continuous.now(), resumed.now());
    }
}

#[test]
fn e7_f022_simple_forms_match_scheduler_wait_snapshot_replay() {
    // LDI/LDD/CPI/CPD are the four non-repeating forms. Exercise them through
    // SmsTemporalMachine/MachineScheduler with WAIT on every bus request and
    // a snapshot/restore round-trip at every scheduler-owned safe point.
    let cases = [
        // opcode, HL, DE, expected HL, expected DE, expected WZ, expected F
        (
            0xA0_u8, 0x0100_u16, 0xC000_u16, 0x0101_u16, 0xC001_u16, 0xA55A_u16, 0xC5_u8,
        ),
        (
            0xA8_u8, 0x0101_u16, 0xC001_u16, 0x0100_u16, 0xC000_u16, 0xA55A_u16, 0xC5_u8,
        ),
        (
            0xA1_u8, 0x0100_u16, 0xC000_u16, 0x0101_u16, 0xC000_u16, 0xA55B_u16, 0x3F_u8,
        ),
        (
            0xA9_u8, 0x0101_u16, 0xC001_u16, 0x0100_u16, 0xC001_u16, 0xA559_u16, 0x3F_u8,
        ),
    ];

    for (opcode, initial_hl, initial_de, expected_hl, expected_de, expected_wz, expected_f) in cases
    {
        let mut rom = vec![0; 0x4000];
        rom[0] = 0xED;
        rom[1] = opcode;
        rom[0x0100] = 0x01;
        rom[0x0101] = 0x01;

        let mut continuous = machine(rom.clone());
        let mut resumed = machine(rom);
        for candidate in [&mut continuous, &mut resumed] {
            let cpu = candidate.cpu_mut();
            cpu.registers.set_hl(initial_hl);
            cpu.registers.set_de(initial_de);
            cpu.registers.set_bc(2);
            cpu.registers.a = 0x80;
            cpu.registers.f = 0xD7;
            cpu.wz = 0xA55A;
            cpu.q = 0x28;
            cpu.q_previous = 0x5A;
        }

        let retired = compare_runs(&mut continuous, &mut resumed, 1);
        assert!(matches!(
            retired,
            Z80MicroStep::InstructionRetired { opcode: retired_opcode, .. }
                if retired_opcode == opcode
        ));
        assert_eq!(continuous.cpu().registers.pc, 2, "ED {opcode:02X}: PC");
        assert_eq!(continuous.cpu().registers.bc(), 1, "ED {opcode:02X}: BC");
        assert_eq!(
            continuous.cpu().registers.hl(),
            expected_hl,
            "ED {opcode:02X}: HL"
        );
        assert_eq!(
            continuous.cpu().registers.de(),
            expected_de,
            "ED {opcode:02X}: DE"
        );
        assert_eq!(
            continuous.cpu().registers.f,
            expected_f,
            "ED {opcode:02X}: F"
        );
        assert_eq!(
            continuous.cpu().q,
            expected_f,
            "ED {opcode:02X}: Q must equal final F"
        );
        assert_eq!(
            continuous.cpu().q_previous,
            0x28,
            "ED {opcode:02X}: previous-Q"
        );
        assert_eq!(continuous.cpu().wz, expected_wz, "ED {opcode:02X}: WZ");
        assert_eq!(continuous.cpu().registers.a, 0x80, "ED {opcode:02X}: A");

        if matches!(opcode, 0xA0 | 0xA8) {
            let destination = if opcode == 0xA0 { 0xC000 } else { 0xC001 };
            assert_eq!(
                continuous.memory().read(destination),
                0x01,
                "ED {opcode:02X}: transferred byte"
            );
        } else {
            assert_eq!(
                continuous.memory().read(initial_de),
                0,
                "ED {opcode:02X}: compare must not write at DE"
            );
        }

        assert_eq!(continuous.export_state(), resumed.export_state());
        assert_eq!(continuous.now(), resumed.now());
    }
}
