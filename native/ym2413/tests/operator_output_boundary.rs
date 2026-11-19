//! Characterization of the candidate operator-output / measured-DAC boundary.
//!
//! These tests pin software arithmetic only. They do not claim that an
//! operator intermediate is the physical YM2413 DAC input or output code.

use sega_ym2413::{
    Ym2413DacCode, Ym2413RhythmOutputError,
    operator::{SingleOperatorCarrierParameters, evaluate_single_operator_as_carrier},
};

#[test]
fn negative_peak_operator_intermediate_is_not_a_measured_dac_code() {
    // At phase 0x300 the current log-sine table selects entry 255, whose
    // attenuation is zero. With no additional attenuation, the current
    // one's-complement EXP path followed by >> 4 returns -256.
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

    // The measured output-domain wrapper deliberately rejects -256.
    // This is a boundary characterization, not a conversion rule.
    assert_eq!(
        Ym2413DacCode::new(intermediate),
        Err(Ym2413RhythmOutputError::InvalidDacCode(-256))
    );
}
