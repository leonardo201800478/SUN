//! YM2413 Key Scale Level attenuation lookup.
//!
//! The KSL=1 base table is transcribed from the published reverse-engineering
//! measurements/model notes and expressed in EG attenuation units (0.375 dB).
//! Other KSL modes scale that base by 0, 2, or 4 as documented by Yamaha.

pub const YM2413_KSL1_TABLE: [[u8; 16]; 8] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4],
    [0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 5, 6, 6, 7, 7, 8],
    [0, 0, 0, 2, 4, 5, 6, 7, 8, 9, 9, 10, 10, 11, 11, 12],
    [0, 0, 4, 6, 8, 9, 10, 11, 12, 13, 13, 14, 14, 15, 15, 16],
    [0, 4, 8, 10, 12, 13, 14, 15, 16, 17, 17, 18, 18, 19, 19, 20],
    [0, 8, 12, 14, 16, 17, 18, 19, 20, 21, 21, 22, 22, 23, 23, 24],
    [
        0, 12, 16, 18, 20, 21, 22, 23, 24, 25, 25, 26, 26, 27, 27, 28,
    ],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyScaleError {
    InvalidBlock(u8),
    InvalidFNumber(u16),
    InvalidKsl(u8),
}

/// Calculate KSL attenuation in EG units (0.375 dB per step).
///
/// The FNUM index is its four most-significant bits, (FNUM >> 5) & 0x0f.
/// KSL codes map to 0, 1, 2, and 4 times the KSL=1 (1.5 dB/octave) table.
pub const fn key_scale_attenuation(block: u8, fnum: u16, ksl: u8) -> Result<u8, KeyScaleError> {
    if block > 7 {
        return Err(KeyScaleError::InvalidBlock(block));
    }
    if fnum > 0x01ff {
        return Err(KeyScaleError::InvalidFNumber(fnum));
    }
    if ksl > 3 {
        return Err(KeyScaleError::InvalidKsl(ksl));
    }

    let base = YM2413_KSL1_TABLE[block as usize][((fnum >> 5) & 0x0f) as usize];
    let factor = match ksl {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 4,
        _ => return Err(KeyScaleError::InvalidKsl(ksl)),
    };
    Ok(base * factor)
}

#[cfg(test)]
mod tests {
    use super::{KeyScaleError, YM2413_KSL1_TABLE, key_scale_attenuation};

    #[test]
    fn ksl_zero_disables_key_scaling() {
        for block in 0..=7 {
            for fnum_index in 0..=15 {
                assert_eq!(key_scale_attenuation(block, fnum_index * 0x20, 0), Ok(0));
            }
        }
    }

    #[test]
    fn ksl_one_matches_the_published_block_seven_measurement_row() {
        const EXPECTED: [u8; 16] = [
            0, 12, 16, 18, 20, 21, 22, 23, 24, 25, 25, 26, 26, 27, 27, 28,
        ];
        for (index, expected) in EXPECTED.iter().enumerate() {
            assert_eq!(
                key_scale_attenuation(7, (index as u16) << 5, 1),
                Ok(*expected)
            );
        }
    }

    #[test]
    fn ksl_modes_scale_the_base_table_by_documented_octave_rates() {
        for block in 0..=7 {
            for index in 0..=15 {
                let fnum = index * 0x20;
                let base = key_scale_attenuation(block, fnum, 1).unwrap();
                assert_eq!(key_scale_attenuation(block, fnum, 2), Ok(base * 2));
                assert_eq!(key_scale_attenuation(block, fnum, 3), Ok(base * 4));
            }
        }
    }

    #[test]
    fn fnum_uses_the_four_most_significant_bits() {
        assert_eq!(key_scale_attenuation(7, 0x01e0, 1), Ok(28));
        assert_eq!(key_scale_attenuation(7, 0x01ff, 1), Ok(28));
    }

    #[test]
    fn invalid_register_values_are_rejected() {
        assert_eq!(
            key_scale_attenuation(8, 0, 0),
            Err(KeyScaleError::InvalidBlock(8))
        );
        assert_eq!(
            key_scale_attenuation(0, 0x0200, 0),
            Err(KeyScaleError::InvalidFNumber(0x0200))
        );
        assert_eq!(
            key_scale_attenuation(0, 0, 4),
            Err(KeyScaleError::InvalidKsl(4))
        );
    }

    #[test]
    fn table_rows_are_monotonic_and_never_exceed_28_base_steps() {
        for row in &YM2413_KSL1_TABLE {
            assert!(row.windows(2).all(|pair| pair[0] <= pair[1]));
            assert!(row.iter().all(|value| *value <= 28));
        }
    }
}
