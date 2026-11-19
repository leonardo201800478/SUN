//! Characterization of the candidate operator-output / measured-DAC boundary.
//!
//! These tests pin software representation only. The operator candidate uses
//! one's-complement encoding inside an i16; this is not a physical DAC capture.

use sega_ym2413::{
    Ym2413DacCode, Ym2413RhythmOutputError,
    operator::{SingleOperatorCarrierParameters, evaluate_single_operator_as_carrier},
};

#[test]
fn negative_peak_operator_intermediate_uses_ones_complement_encoding() {
    // At phase 0x300 the current log-sine table selects entry 255, whose
    // attenuation is zero. With no additional attenuation, the current
    // one's-complement EXP path followed by >> 4 returns the i16 value -256.
    // In the model's one’s-complement notation, that bit pattern represents
    // amplitude -255; the i16 display value is not the signed amplitude.
    let intermediate = evaluate_single_operator_as_carrier(
        0x0300,
        SingleOperatorCarrierParameters {
            volume: 0,
            ksl: 0,
            envelope: 0,
            am: 0,
            half_wave: false,
        },
    )
    .expect("valid phase and attenuation parameters");

    assert_eq!(intermediate, -256);
    assert_eq!(Ym2413DacCode::new(-255).map(Ym2413DacCode::value), Ok(-255));

    // Ym2413DacCode accepts conventional signed amplitudes, not this encoded
    // intermediate. Rejection here demonstrates a representation mismatch;
    // it says nothing about the physical DAC input encoding.
    assert_eq!(
        Ym2413DacCode::new(intermediate),
        Err(Ym2413RhythmOutputError::InvalidDacCode(-256))
    );
}
