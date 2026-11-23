//! Allocation-free schedule-plus-dispatch path (`dispatch_next_into`) with a reused buffer.
//!
//! Kept as a separate binary so `temporal_bench` keeps compiling against the
//! pre-P0 scheduler and the A/B comparison stays honest.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use sega_machine_scheduler::{ClockAdvances, ClockDomainId, ClockRatio, MachineScheduler};
use sega_temporal_core::{HardwarePhase, MachineTime, PlatformPriority};

struct CountingAllocator;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

// SAFETY: every path forwards to `System` unchanged and only adds counters.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[derive(Clone, Copy)]
struct Payload(u64);

fn build_scheduler(domains: usize, ticks_per_tstate: u64) -> MachineScheduler<Payload> {
    let mut scheduler = MachineScheduler::new();
    let ratio = ClockRatio::new(1, ticks_per_tstate).expect("ratio");
    for index in 0..domains {
        scheduler
            .register_clock_domain(ClockDomainId(index as u16), ratio)
            .expect("register");
    }
    scheduler
}

#[inline(never)]
fn schedule_and_dispatch_into(
    scheduler: &mut MachineScheduler<Payload>,
    advances: &mut ClockAdvances,
    timestamp: MachineTime,
) -> usize {
    scheduler
        .schedule_at_with_phase(
            timestamp,
            HardwarePhase::new(1),
            PlatformPriority::new(2),
            Payload(black_box(timestamp.ticks() as u64)),
        )
        .expect("schedule");
    let event = scheduler
        .dispatch_next_into(advances)
        .expect("dispatch")
        .expect("event");
    let mut accumulator = event.payload.0;
    for advance in advances.iter() {
        accumulator = accumulator.wrapping_add(advance.elapsed_cycles as u64);
        accumulator = accumulator.wrapping_add(advance.total_cycles as u64);
    }
    black_box(accumulator);
    advances.len()
}

fn main() {
    let ticks_per_tstate = 3u64;
    let iterations = 200_000u64;
    println!(
        "caller_buffer — schedule + dispatch_next_into, ticks_per_tstate={ticks_per_tstate}, iterations={iterations}"
    );
    println!(
        "{:<22} {:>7} {:>10} {:>10} {:>11}",
        "scenario", "domains", "ns/op", "allocs/op", "bytes/op"
    );

    for (label, domains) in [
        ("schedule_and_dispatch_into_sms_3", 3),
        ("schedule_and_dispatch_into_sms_4", 4),
    ] {
        let mut scheduler = build_scheduler(domains, ticks_per_tstate);
        let mut advances = ClockAdvances::new();
        let step = ticks_per_tstate as u128;
        let mut tick: u128 = 0;
        for _ in 0..2_000 {
            tick += step;
            black_box(schedule_and_dispatch_into(
                &mut scheduler,
                &mut advances,
                MachineTime::from_ticks(tick),
            ));
        }

        let before_allocations = ALLOCATIONS.load(Ordering::Relaxed);
        let before_bytes = ALLOCATED_BYTES.load(Ordering::Relaxed);
        let started = Instant::now();
        for _ in 0..iterations {
            tick += step;
            black_box(schedule_and_dispatch_into(
                &mut scheduler,
                &mut advances,
                MachineTime::from_ticks(tick),
            ));
        }
        let elapsed = started.elapsed();
        let allocations = ALLOCATIONS.load(Ordering::Relaxed) - before_allocations;
        let bytes = ALLOCATED_BYTES.load(Ordering::Relaxed) - before_bytes;
        println!(
            "{:<22} {:>7} {:>10.2} {:>10.3} {:>11.1}",
            label,
            domains,
            elapsed.as_nanos() as f64 / iterations as f64,
            allocations as f64 / iterations as f64,
            bytes as f64 / iterations as f64
        );
    }
}
