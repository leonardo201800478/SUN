//! Production-shaped latency and allocation harness for `MachineScheduler`.
//!
//! Why this exists (plan §4.7): the first audit mixed measurement boundaries —
//! an instrumented window around `dispatch_next` was compared with an isolated
//! microbenchmark that used a smaller token type and skipped part of the
//! production work. This harness reproduces one production-shaped scheduler
//! operation (schedule one event, then dispatch it) with the real `ClockAdvance`
//! type and counts every allocation, so before/after numbers share one boundary.
//!
//! What is included in the measured region:
//!   - `schedule_at_with_phase()` for the event being measured
//!   - `events.next_timestamp()` probe
//!   - same-timestamp budget bookkeeping
//!   - `calculate_advances` (one rational preview per registered domain)
//!   - `apply_advances` (validate every domain, then commit every domain)
//!   - `pop_next`
//!   - the dispatch `trace!` call as it exists in the release profile
//!   - construction of the returned `MachineDispatch`
//!
//! What is excluded:
//!   - consumer work (`sms_core`, VDP, PSG) — that belongs to their own audits
//!
//! Run with `--release`; debug timings are meaningless here.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use sega_machine_scheduler::{ClockDomainId, ClockRatio, MachineScheduler};
use sega_temporal_core::{HardwarePhase, MachineTime, PlatformPriority};

struct CountingAllocator;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static REALLOCATIONS: AtomicU64 = AtomicU64::new(0);

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

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        REALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[derive(Clone, Copy)]
struct Payload(u64);

struct Measurement {
    label: &'static str,
    domains: usize,
    ns_per_operation: f64,
    allocations_per_operation: f64,
    bytes_per_operation: f64,
    reallocations_per_operation: f64,
    advances_len: usize,
}

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

/// One scheduler operation: schedule one event, then dispatch it.
#[inline(never)]
fn schedule_and_dispatch_once(
    scheduler: &mut MachineScheduler<Payload>,
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
    let dispatch = scheduler.dispatch_next().expect("dispatch").expect("event");
    let advances = &dispatch.clock_advances;
    // Touch the payload so the advances cannot be optimized away.
    let mut accumulator = dispatch.event.payload.0;
    for advance in advances.iter() {
        accumulator = accumulator.wrapping_add(advance.elapsed_cycles as u64);
        accumulator = accumulator.wrapping_add(advance.total_cycles as u64);
    }
    black_box(accumulator);
    advances.len()
}

fn measure(
    label: &'static str,
    domains: usize,
    iterations: u64,
    ticks_per_tstate: u64,
) -> Measurement {
    let mut scheduler = build_scheduler(domains, ticks_per_tstate);
    let step = ticks_per_tstate as u128;
    let mut tick: u128 = 0;

    // Warm-up: page in the queue, the clock map and the trace machinery.
    for _ in 0..2_000 {
        tick += step;
        black_box(schedule_and_dispatch_once(
            &mut scheduler,
            MachineTime::from_ticks(tick),
        ));
    }

    let before_allocations = ALLOCATIONS.load(Ordering::Relaxed);
    let before_bytes = ALLOCATED_BYTES.load(Ordering::Relaxed);
    let before_reallocations = REALLOCATIONS.load(Ordering::Relaxed);

    let started = Instant::now();
    let mut advances_len = 0;
    for _ in 0..iterations {
        tick += step;
        advances_len = schedule_and_dispatch_once(&mut scheduler, MachineTime::from_ticks(tick));
    }
    let elapsed = started.elapsed();

    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - before_allocations;
    let bytes = ALLOCATED_BYTES.load(Ordering::Relaxed) - before_bytes;
    let reallocations = REALLOCATIONS.load(Ordering::Relaxed) - before_reallocations;

    Measurement {
        label,
        domains,
        ns_per_operation: elapsed.as_nanos() as f64 / iterations as f64,
        allocations_per_operation: allocations as f64 / iterations as f64,
        bytes_per_operation: bytes as f64 / iterations as f64,
        reallocations_per_operation: reallocations as f64 / iterations as f64,
        advances_len,
    }
}

fn measure_idle_advance(domains: usize, iterations: u64, ticks_per_tstate: u64) -> Measurement {
    measure_idle_advance_inner(domains, iterations, ticks_per_tstate, false)
}

fn measure_idle_advance_inner(
    domains: usize,
    iterations: u64,
    ticks_per_tstate: u64,
    _unused: bool,
) -> Measurement {
    let mut scheduler = build_scheduler(domains, ticks_per_tstate);
    let step = ticks_per_tstate as u128;
    let mut tick: u128 = 0;
    for _ in 0..2_000 {
        tick += step;
        black_box(schedule_and_dispatch_once(
            &mut scheduler,
            MachineTime::from_ticks(tick),
        ));
    }

    let before_allocations = ALLOCATIONS.load(Ordering::Relaxed);
    let before_bytes = ALLOCATED_BYTES.load(Ordering::Relaxed);
    let before_reallocations = REALLOCATIONS.load(Ordering::Relaxed);

    let started = Instant::now();
    let mut advances_len = 0;
    for _ in 0..iterations {
        tick += step;
        let advances = scheduler
            .advance_idle_to(MachineTime::from_ticks(tick))
            .expect("idle advance");
        advances_len = advances.len();
        black_box(&advances);
    }
    let elapsed = started.elapsed();

    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - before_allocations;
    let bytes = ALLOCATED_BYTES.load(Ordering::Relaxed) - before_bytes;
    let reallocations = REALLOCATIONS.load(Ordering::Relaxed) - before_reallocations;

    Measurement {
        label: "advance_idle_to",
        domains,
        ns_per_operation: elapsed.as_nanos() as f64 / iterations as f64,
        allocations_per_operation: allocations as f64 / iterations as f64,
        bytes_per_operation: bytes as f64 / iterations as f64,
        reallocations_per_operation: reallocations as f64 / iterations as f64,
        advances_len,
    }
}

fn print_header() {
    println!(
        "{:<22} {:>7} {:>10} {:>10} {:>11} {:>10} {:>10}",
        "scenario", "domains", "ns/op", "allocs/op", "bytes/op", "reallocs/op", "advances"
    );
}

fn print_row(m: &Measurement) {
    println!(
        "{:<22} {:>7} {:>10.2} {:>10.3} {:>11.1} {:>10.3} {:>10}",
        m.label,
        m.domains,
        m.ns_per_operation,
        m.allocations_per_operation,
        m.bytes_per_operation,
        m.reallocations_per_operation,
        m.advances_len
    );
}

fn main() {
    let ticks_per_tstate = 3u64;
    let iterations = 200_000u64;

    println!(
        "temporal_bench — production-shaped schedule + dispatch, ticks_per_tstate={ticks_per_tstate}, \
         iterations={iterations}"
    );
    println!("note: `bytes` counts requested allocation bytes, not resident memory\n");
    print_header();

    // SMS shape: Z80 + PSG + VDP share one ratio (3 domains).
    let sms = measure("schedule_dispatch_sms_3", 3, iterations, ticks_per_tstate);
    print_row(&sms);

    // Japanese SMS adds the YM2413 domain (4 domains).
    let sms_ym = measure(
        "schedule_dispatch_sms_ym2413_4",
        4,
        iterations,
        ticks_per_tstate,
    );
    print_row(&sms_ym);

    // Beyond the inline capacity: proves the fallback never truncates.
    let wide = measure(
        "schedule_dispatch_wide_9",
        9,
        iterations / 4,
        ticks_per_tstate,
    );
    print_row(&wide);

    let idle = measure_idle_advance(3, iterations, ticks_per_tstate);
    print_row(&idle);

    println!();
    println!(
        "per-frame projection at 49_780 scheduler operations/frame: {:.2} ms (3 domains)",
        sms.ns_per_operation * 49_780.0 / 1.0e6
    );
    println!(
        "allocations per frame: {:.0}",
        sms.allocations_per_operation * 49_780.0
    );
}
