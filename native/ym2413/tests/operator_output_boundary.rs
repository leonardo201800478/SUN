//! Characterization of the candidate operator-output / measured-level boundary.
//!
//! These tests pin software representation only. The post-shift operator
//! result follows one's-complement semantics while being transported in an
//! i16; it is not a physical DAC capture.

use sega_ym2413::{
    Ym2413DacCode, Ym2413RhythmOutputError,
    operator::{SingleOperatorCarrierParameters, evaluate_single_operator_as_carrier},
};

#[test]
fn negative_peak_operator_intermediate_uses_ones_complement_encoding() {
    // At phase 0x300 the current log-sine table selects entry 255, whose
    // attenuation is zero. With no additional attenuation, the one's-complement
    // EXP path followed by >> 4 returns the host i16 value -256.
    // In the model's post-shift one's-complement domain, this is the negative
    // peak corresponding to amplitude -255; it is not a physical DAC capture.
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

    // Ym2413DacCode wraps conventional signed measured levels, not the
    // operator's one's-complement transport representation. This rejection
    // demonstrates a type-domain mismatch, not a physical DAC limitation.
    assert_eq!(
        Ym2413DacCode::new(intermediate),
        Err(Ym2413RhythmOutputError::InvalidDacCode(-256))
    );
}
