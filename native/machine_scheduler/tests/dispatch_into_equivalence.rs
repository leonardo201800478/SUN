//! The allocating and allocation-free dispatch paths must be indistinguishable.
//!
//! `dispatch_next` keeps the historical `Vec` payload while
//! `dispatch_next_into` writes into a caller buffer. They share the preflight,
//! the commit and the pop, but they are separate bodies for performance, so the
//! equivalence has to be proven rather than assumed: same event sequence, same
//! advances, same clock state, same dispatch history.

use sega_machine_scheduler::{
    ClockAdvances, ClockDomainId, ClockRatio, DeltaEventPolicy, MachineScheduler,
};
use sega_temporal_core::{HardwarePhase, MachineTime, PlatformPriority};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Payload(u64);

fn build(domains: &[(u16, u64, u64)]) -> MachineScheduler<Payload> {
    let mut scheduler =
        MachineScheduler::with_delta_event_policy(DeltaEventPolicy::new(64).expect("policy"));
    for &(id, numerator, denominator) in domains {
        scheduler
            .register_clock_domain(
                ClockDomainId(id),
                ClockRatio::new(numerator, denominator).expect("ratio"),
            )
            .expect("register");
    }
    scheduler
}

fn schedule(scheduler: &mut MachineScheduler<Payload>, tick: u128, sequence: u64) {
    scheduler
        .schedule_at_with_phase(
            MachineTime::from_ticks(tick),
            HardwarePhase::new((sequence % 3) as u16),
            PlatformPriority::new((sequence % 2) as u16),
            Payload(sequence),
        )
        .expect("schedule");
}

#[test]
fn both_entry_points_produce_identical_events_advances_and_clock_state() {
    // Mixed ratios and a fourth domain exercise the inline capacity and the
    // non-shared-ratio path.
    let domains = [(0, 1, 3), (1, 1, 3), (2, 1, 2), (3, 1, 3)];
    let mut owned = build(&domains);
    let mut buffered = build(&domains);
    let mut buffer = ClockAdvances::new();

    let mut tick: u128 = 0;
    for sequence in 0..400u64 {
        tick += 1 + (sequence % 5) as u128;
        schedule(&mut owned, tick, sequence);
        schedule(&mut buffered, tick, sequence);

        let left = owned
            .dispatch_next()
            .expect("owned dispatch")
            .expect("event");
        let right = buffered
            .dispatch_next_into(&mut buffer)
            .expect("buffered dispatch")
            .expect("event");

        assert_eq!(left.event, right, "event sequence diverged");
        assert_eq!(
            left.clock_advances.as_slice(),
            buffer.as_slice(),
            "clock advances diverged at sequence {sequence}"
        );
        assert_eq!(
            left.clock_advances.len(),
            domains.len(),
            "one advance per registered domain"
        );
        for (index, advance) in left.clock_advances.iter().enumerate() {
            assert_eq!(
                advance.domain,
                ClockDomainId(domains[index].0),
                "advances must stay in ascending domain order"
            );
        }
        assert_eq!(owned.now(), buffered.now(), "machine time diverged");
        assert_eq!(owned.pending_events(), buffered.pending_events());
        for &(id, _, _) in &domains {
            assert_eq!(
                owned.clock_snapshot(ClockDomainId(id)).expect("snapshot"),
                buffered
                    .clock_snapshot(ClockDomainId(id))
                    .expect("snapshot"),
                "clock domain {id} state diverged"
            );
        }
    }

    // Dispatch history must match too, otherwise save states would differ.
    assert_eq!(owned.export_state(), buffered.export_state());
}

#[test]
fn buffered_dispatch_empties_the_buffer_when_the_queue_is_empty() {
    let mut scheduler = build(&[(0, 1, 3)]);
    let mut buffer = ClockAdvances::new();
    schedule(&mut scheduler, 3, 1);
    scheduler
        .dispatch_next_into(&mut buffer)
        .expect("dispatch")
        .expect("event");
    assert_eq!(buffer.len(), 1);

    assert!(
        scheduler
            .dispatch_next_into(&mut buffer)
            .expect("empty dispatch")
            .is_none()
    );
    assert!(
        buffer.is_empty(),
        "a dispatch with no event must not expose stale advances"
    );
}

#[test]
fn buffered_dispatch_never_truncates_beyond_inline_capacity() {
    let mut scheduler = MachineScheduler::<Payload>::new();
    let ratio = ClockRatio::new(1, 3).expect("ratio");
    let domain_count = 9u16;
    for id in 0..domain_count {
        scheduler
            .register_clock_domain(ClockDomainId(id), ratio)
            .expect("register");
    }
    let mut buffer = ClockAdvances::new();
    schedule(&mut scheduler, 3, 1);
    scheduler
        .dispatch_next_into(&mut buffer)
        .expect("dispatch")
        .expect("event");

    assert_eq!(buffer.len(), usize::from(domain_count));
    assert!(
        buffer.spilled(),
        "nine domains must use the heap fallback instead of truncating"
    );
    for (index, advance) in buffer.iter().enumerate() {
        assert_eq!(advance.domain, ClockDomainId(index as u16));
    }
}
