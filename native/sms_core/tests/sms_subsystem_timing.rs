use sega_sms_core::SmsTemporalMachine;

#[test]
fn subsystem_timing_counts_scheduler_psg_and_vdp_without_changing_instruction_result() {
    let mut machine = SmsTemporalMachine::new(vec![0x00; 0x4000], 1)
        .expect("construct profiled temporal SMS machine");
    let mut reference = SmsTemporalMachine::new(vec![0x00], 1)
        .expect("construct reference temporal SMS machine");
    machine.set_subsystem_timing_enabled(true);

    let step = machine
        .step_instruction(0)
        .expect("execute one NOP through the scheduler");
    let reference_step = reference
        .step_instruction(0)
        .expect("execute the reference NOP through the scheduler");
    let state = machine.export_state();
    let reference_state = reference.export_state();
    let timing = machine.take_subsystem_timing();

    assert_eq!(step, reference_step);
    assert_eq!(state, reference_state, "profiling must not mutate emulated state");

    assert!(matches!(
        step,
        sega_z80_cpu::Z80MicroStep::InstructionRetired { .. }
    ));
    assert!(timing.scheduler_dispatch_count > 0);
    assert_eq!(timing.psg_advance_calls, timing.scheduler_dispatch_count);
    assert!(timing.psg_advance_clock_cycles > 0);
    assert_eq!(timing.vdp_advance_calls, timing.scheduler_dispatch_count);
    assert!(timing.vdp_advance_tstates > 0);
}

#[test]
fn subsystem_timing_is_zero_cost_disabled_by_default() {
    let mut machine = SmsTemporalMachine::new(vec![0x00], 1)
        .expect("construct temporal SMS machine");

    machine
        .step_instruction(0)
        .expect("execute one NOP through the scheduler");
    let timing = machine.take_subsystem_timing();

    assert_eq!(timing, Default::default());
}
