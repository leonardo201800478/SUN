//! Integer log-sine and exponential conversion used by YM2413 operators.
//!
//! The tables are generated from the published reverse-engineering equations
//! (andete, 2015-04-09 / 2016-07-16). They are checked in as integer constants
//! so the audio hot path is deterministic and performs no floating-point work.
//! This implements the table decode, not the operator-slot pipeline or DAC.

pub const YM2413_LOG_SINE_TABLE: [u16; 256] = [
2137, 1731, 1543, 1419, 1326, 1252, 1190, 1137,
1091, 1050, 1013,  979,  949,  920,  894,  869,
 846,  825,  804,  785,  767,  749,  732,  717,
 701,  687,  672,  659,  646,  633,  621,  609,
 598,  587,  576,  566,  556,  546,  536,  527,
 518,  509,  501,  492,  484,  476,  468,  461,
 453,  446,  439,  432,  425,  418,  411,  405,
 399,  392,  386,  380,  375,  369,  363,  358,
 352,  347,  341,  336,  331,  326,  321,  316,
 311,  307,  302,  297,  293,  289,  284,  280,
 276,  271,  267,  263,  259,  255,  251,  248,
 244,  240,  236,  233,  229,  226,  222,  219,
 215,  212,  209,  205,  202,  199,  196,  193,
 190,  187,  184,  181,  178,  175,  172,  169,
 167,  164,  161,  159,  156,  153,  151,  148,
 146,  143,  141,  138,  136,  134,  131,  129,
 127,  125,  122,  120,  118,  116,  114,  112,
 110,  108,  106,  104,  102,  100,   98,   96,
  94,   92,   91,   89,   87,   85,   83,   82,
  80,   78,   77,   75,   74,   72,   70,   69,
  67,   66,   64,   63,   62,   60,   59,   57,
  56,   55,   53,   52,   51,   49,   48,   47,
  46,   45,   43,   42,   41,   40,   39,   38,
  37,   36,   35,   34,   33,   32,   31,   30,
  29,   28,   27,   26,   25,   24,   23,   23,
  22,   21,   20,   20,   19,   18,   17,   17,
  16,   15,   15,   14,   13,   13,   12,   12,
  11,   10,   10,    9,    9,    8,    8,    7,
   7,    7,    6,    6,    5,    5,    5,    4,
   4,    4,    3,    3,    3,    2,    2,    2,
   2,    1,    1,    1,    1,    1,    1,    1,
   0,    0,    0,    0,    0,    0,    0,    0

];

pub const YM2413_EXP_TABLE: [u16; 256] = [
   0,    3,    6,    8,   11,   14,   17,   20,
  22,   25,   28,   31,   34,   37,   40,   42,
  45,   48,   51,   54,   57,   60,   63,   66,
  69,   72,   75,   78,   81,   84,   87,   90,
  93,   96,   99,  102,  105,  108,  111,  114,
 117,  120,  123,  126,  130,  133,  136,  139,
 142,  145,  148,  152,  155,  158,  161,  164,
 168,  171,  174,  177,  181,  184,  187,  190,
 194,  197,  200,  204,  207,  210,  214,  217,
 220,  224,  227,  231,  234,  237,  241,  244,
 248,  251,  255,  258,  262,  265,  268,  272,
 276,  279,  283,  286,  290,  293,  297,  300,
 304,  308,  311,  315,  318,  322,  326,  329,
 333,  337,  340,  344,  348,  352,  355,  359,
 363,  367,  370,  374,  378,  382,  385,  389,
 393,  397,  401,  405,  409,  412,  416,  420,
 424,  428,  432,  436,  440,  444,  448,  452,
 456,  460,  464,  468,  472,  476,  480,  484,
 488,  492,  496,  501,  505,  509,  513,  517,
 521,  526,  530,  534,  538,  542,  547,  551,
 555,  560,  564,  568,  572,  577,  581,  585,
 590,  594,  599,  603,  607,  612,  616,  621,
 625,  630,  634,  639,  643,  648,  652,  657,
 661,  666,  670,  675,  680,  684,  689,  693,
 698,  703,  708,  712,  717,  722,  726,  731,
 736,  741,  745,  750,  755,  760,  765,  770,
 774,  779,  784,  789,  794,  799,  804,  809,
 814,  819,  824,  829,  834,  839,  844,  849,
 854,  859,  864,  869,  874,  880,  885,  890,
 895,  900,  906,  911,  916,  921,  927,  932,
 937,  942,  948,  953,  959,  964,  969,  975,
 980,  986,  991,  996, 1002, 1007, 1013, 1018

];

/// Convert a ten-bit phase-table address into signed log-sine attenuation.
///
/// The returned value is sign-magnitude: bit 15 carries the sign and the low
/// bits carry log-domain attenuation. The half_wave flag selects the half-wave
/// waveform, which suppresses the negative half-cycle.
#[inline]
pub const fn lookup_log_sine(phase: u16, half_wave: bool) -> u16 {
    let phase = phase & 0x03ff;
    let negative = (phase & 0x0200) != 0;
    let mirrored = (phase & 0x0100) != 0;
    let index = (phase & 0x00ff) as usize;
    let index = if mirrored { index ^ 0x00ff } else { index };
    let mut result = YM2413_LOG_SINE_TABLE[index];
    if negative {
        if half_wave {
            result = 0x0fff;
        }
        result |= 0x8000;
    }
    result
}

/// Convert sign-magnitude log-domain attenuation to a signed operator value.
///
/// This follows the YM2413 exp-table decode: the low byte addresses the
/// complemented exponential table, bits 8..14 select the right shift, and
/// bit 15 applies two's-complement negation. The caller applies the operator-specific output shift (modulator or carrier)
/// after this conversion; the lookup itself returns the full signed 13-bit value.
#[inline]
pub const fn lookup_exp(value: u16) -> i16 {
    let index = ((value & 0x00ff) ^ 0x00ff) as usize;
    let table_value = YM2413_EXP_TABLE[index];
    let expanded = ((table_value as i32) << 1) | 0x0800;
    let shift = ((value & 0x7f00) >> 8) as u32;
    let magnitude = if shift >= 32 { 0 } else { expanded >> shift };
    let signed = if (value & 0x8000) != 0 { !magnitude } else { magnitude };
    signed as i16
}

#[cfg(test)]
mod tests {
    use super::{lookup_exp, lookup_log_sine, YM2413_EXP_TABLE, YM2413_LOG_SINE_TABLE};

    #[test]
    fn generated_tables_match_reference_endpoints() {
        assert_eq!(YM2413_LOG_SINE_TABLE[0], 2_137);
        assert_eq!(YM2413_LOG_SINE_TABLE[255], 0);
        assert_eq!(YM2413_EXP_TABLE[0], 0);
        assert_eq!(YM2413_EXP_TABLE[255], 1_018);
    }

    #[test]
    fn sine_lookup_reconstructs_quadrants() {
        assert_eq!(lookup_log_sine(0, false), YM2413_LOG_SINE_TABLE[0]);
        assert_eq!(lookup_log_sine(255, false), YM2413_LOG_SINE_TABLE[255]);
        assert_eq!(lookup_log_sine(256, false), YM2413_LOG_SINE_TABLE[255]);
        assert_eq!(lookup_log_sine(511, false), YM2413_LOG_SINE_TABLE[0]);
        assert_eq!(lookup_log_sine(512, false), 0x8000 | YM2413_LOG_SINE_TABLE[0]);
        assert_eq!(lookup_log_sine(768, false), 0x8000 | YM2413_LOG_SINE_TABLE[255]);
    }

    #[test]
    fn half_wave_suppresses_the_negative_quadrants() {
        assert_eq!(lookup_log_sine(0, true), lookup_log_sine(0, false));
        assert_eq!(lookup_log_sine(512, true), 0x8fff);
        assert_eq!(lookup_log_sine(1023, true), 0x8fff);
    }

    #[test]
    fn exponential_lookup_matches_measured_envelope_endpoints() {
        assert_eq!(lookup_exp(0), 4_084);
        assert_eq!(lookup_exp(16 * 16) >> 4, 127);
        assert_eq!(lookup_exp(0x8000), -4_085);
    }

    #[test]
    fn exponential_lookup_is_monotonic_with_attenuation() {
        let values: Vec<i16> = (0..128).map(|level| lookup_exp(level << 4)).collect();
        assert!(values.windows(2).all(|pair| pair[0] >= pair[1]));
        assert_eq!(values[0], 4_084);
    }

    #[test]
    fn phase_and_attenuation_inputs_are_bounded_to_hardware_fields() {
        assert_eq!(lookup_log_sine(0x0400, false), lookup_log_sine(0, false));
        assert_eq!(lookup_exp(0x0000), lookup_exp(0x8000) * -1 - 1);
    }
}
