use sega_sms_core::SmsTemporalMachine;
use sega_temporal_core::MachineTime;
use sega_z80_cpu::{Z80BusOperation, Z80HaltExit, Z80InterruptKind, Z80MicroStep};

#[test]
fn sms_temporal_nmi_asserted_during_halt_m1_is_sampled_at_t4_and_restores_exactly() {
    let mut machine = SmsTemporalMachine::new(vec![0x76, 0x00], 3).unwrap();
    machine.cpu_mut().registers.sp = 0xC100;
    machine.set_trace_enabled(true);

    assert_eq!(
        machine.step_instruction(0).unwrap(),
        Z80MicroStep::InstructionRetired {
            opcode: 0x76,
            tstates: 4,
        }
    );
    assert!(machine.cpu().halted);
    assert_eq!(machine.cpu().registers.pc, 0x0001);

    // Start the repeated HALT M1 before asserting NMI. The request must be
    // latched at the effective T4 boundary, not retroactively before M1.
    let hold = match machine.begin_instruction().unwrap() {
        Z80MicroStep::BusRequest(request) => request,
        other => panic!("expected repeated HALT M1 request, got {other:?}"),
    };
    assert_eq!(hold.operation, Z80BusOperation::OpcodeFetch);
    assert_eq!(hold.address, 0x0001);
    machine.schedule_pending_bus(0).unwrap();
    machine.request_nmi();

    let snapshot = machine.export_state();
    let mut restored = SmsTemporalMachine::restore_state(snapshot).unwrap();
    restored.set_trace_enabled(true);

    let expected_hold = machine.resume_instruction(0).unwrap();
    let actual_hold = restored.resume_instruction(0).unwrap();
    assert_eq!(actual_hold, expected_hold);
    assert_eq!(
        actual_hold,
        Z80MicroStep::InstructionRetired {
            opcode: 0x00,
            tstates: 4,
        }
    );
    assert!(machine.cpu().halted);
    assert_eq!(machine.cpu().halt_exit_pending, Some(Z80HaltExit::Nmi));
    assert_eq!(restored.cpu().halt_exit_pending, Some(Z80HaltExit::Nmi));
    assert_eq!(machine.now(), MachineTime::from_ticks(24));
    assert_eq!(restored.now(), machine.now());

    let expected_service = machine.step_instruction(0).unwrap();
    let actual_service = restored.step_instruction(0).unwrap();
    assert_eq!(actual_service, expected_service);
    assert_eq!(
        actual_service,
        Z80MicroStep::InterruptServiced {
            kind: Z80InterruptKind::Nmi,
            tstates: 11,
        }
    );

    assert_eq!(restored.cpu().registers.pc, 0x0066);
    assert_eq!(restored.cpu().registers.sp, 0xC0FE);
    assert_eq!(restored.cpu().refresh, 0x03);
    assert!(!restored.cpu().iff1);
    assert_eq!(restored.memory().read(0xC0FF), 0x00);
    assert_eq!(restored.memory().read(0xC0FE), 0x01);
    assert_eq!(restored.now(), MachineTime::from_ticks(57));
    assert_eq!(restored.export_state(), machine.export_state());
    assert_eq!(restored.take_trace(), machine.take_trace());
}
