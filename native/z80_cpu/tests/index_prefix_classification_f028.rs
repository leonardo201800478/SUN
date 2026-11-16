//! Exhaustive byte classification for DD/FD prefix semantics (F-028).
//!
//! This is a classification ledger, not a semantic oracle: UsesIndex means the
//! opcode consumes IX/IY context; Ignored means it keeps BASE semantics.
//! Special prefix bytes DD/FD/ED are handled by the prefix FSM before this
//! classifier is consulted. CB selects the indexed-CB displacement path.
//!
//! The explicit expected set is intentionally data-driven rather than
//! reproducing the production bit-mask/range predicates.

use sega_z80_cpu::{classify_index_opcode, Z80IndexOpcodeClass};

const USES_INDEX: [u8; 85] = [
    0x09, 0x19, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x29, 0x2A, 0x2B, 0x2C,
    0x2D, 0x2E, 0x34, 0x35, 0x36, 0x39,
    0x44, 0x45, 0x46, 0x4C, 0x4D, 0x4E, 0x54, 0x55, 0x56, 0x5C, 0x5D, 0x5E,
    0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x6B,
    0x6C, 0x6D, 0x6E, 0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x77, 0x7C,
    0x7D, 0x7E,
    0x84, 0x85, 0x86, 0x8C, 0x8D, 0x8E, 0x94, 0x95, 0x96, 0x9C, 0x9D, 0x9E,
    0xA4, 0xA5, 0xA6, 0xAC, 0xAD, 0xAE, 0xB4, 0xB5, 0xB6, 0xBC, 0xBD, 0xBE,
    0xE1, 0xE3, 0xE5, 0xE9, 0xF9,
];

#[test]
fn every_post_dd_fd_byte_matches_the_explicit_classification_ledger() {
    let mut uses_index = 0;
    let mut ignored = 0;
    let mut indexed_cb = 0;

    for raw in 0_u16..=u8::MAX.into() {
        let opcode = raw as u8;
        let expected = if opcode == 0xCB {
            Z80IndexOpcodeClass::IndexedCb
        } else if USES_INDEX.contains(&opcode) {
            Z80IndexOpcodeClass::UsesIndex
        } else {
            Z80IndexOpcodeClass::Ignored
        };

        let actual = classify_index_opcode(opcode);
        assert_eq!(
            actual, expected,
            "DD/FD post-prefix opcode {opcode:02X} classification"
        );

        match actual {
            Z80IndexOpcodeClass::UsesIndex => uses_index += 1,
            Z80IndexOpcodeClass::Ignored => ignored += 1,
            Z80IndexOpcodeClass::IndexedCb => indexed_cb += 1,
        }
    }

    assert_eq!(uses_index, 85);
    assert_eq!(ignored, 170);
    assert_eq!(indexed_cb, 1);
    assert_eq!(uses_index + ignored + indexed_cb, 256);
}

#[test]
fn halt_exchange_ed_and_control_prefix_bytes_are_not_index_substitutions() {
    // DD/FD/ED are special-cased by Z80PrefixState::transition before this
    // helper is called: DD/FD repeat with last-prefix-wins; ED cancels index
    // substitution. Their direct helper result remains Ignored.
    for opcode in [0x08, 0x76, 0xD9, 0xDD, 0xED, 0xEB, 0xFD, 0xF3, 0xFB] {
        assert_eq!(
            classify_index_opcode(opcode),
            Z80IndexOpcodeClass::Ignored,
            "opcode {opcode:02X} must retain BASE meaning or be handled by FSM"
        );
    }

    assert_eq!(
        classify_index_opcode(0xCB),
        Z80IndexOpcodeClass::IndexedCb,
        "CB enters DDCB/FDCB, not ordinary BASE CB"
    );
}

#[test]
fn indexed_memory_hl_exceptions_remain_in_the_uses_index_class() {
    // The instruction uses IX/IY for its memory operand, but H/L remain the
    // ordinary HL register halves in these four documented exceptions.
    for opcode in [0x66, 0x6E, 0x74, 0x75] {
        assert_eq!(
            classify_index_opcode(opcode),
            Z80IndexOpcodeClass::UsesIndex,
            "opcode {opcode:02X} uses indexed memory with ordinary H/L"
        );
    }

    // HALT is not an indexed-memory instruction despite its bit pattern.
    assert_eq!(
        classify_index_opcode(0x76),
        Z80IndexOpcodeClass::Ignored
    );
}
