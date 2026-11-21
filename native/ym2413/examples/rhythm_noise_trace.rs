//! Emit the slot-rate YM2413 rhythm-noise register after each DoRhythm step.
//!
//! This is a deterministic component trace for differential comparison with
//! the pinned Nuked-OPLL OPLL_Clock output. Zero is Nuked's software reset
//! register value, not a claim about the physical YM2413 power-on seed.

use sega_ym2413::rhythm::Ym2413RhythmSlotNoiseLfsr;

fn main() {
    let calls = std::env::args()
        .nth(1)
        .map(|value| value.parse::<u64>().expect("calls must be an integer"))
        .unwrap_or(144);
    let mut lfsr = Ym2413RhythmSlotNoiseLfsr::with_register(0)
        .expect("zero is a valid explicit register candidate");

    for call_index in 0..calls {
        let _ = lfsr.clock_with_test_mode(0);
        println!(
            "{{\"call_index\":{call_index},\"rm_noise\":{}}}",
            lfsr.register()
        );
    }
}
