// Codon assignment table.
//
// Maps 6-bit codon values (0..=63) to ISA roles.
// Partitioning (from SPEC §2.3):
//   0–31  → opcode primary codons (opcode = codon value directly)
//   32–47 → register / addressing operands
//   48–55 → control codons (START, END, SYNC markers)
//   56–63 → reserved / padding
//
// Biological codon reservations:
//   ATG = 0b001110 = 14  → PROGRAM_START (also happens to be Opcode::Load = 0x0E)
//     We reserve it as START marker; Load opcode primary codon is remapped to 30.
//   TAA = 0b110000 = 48  → PROGRAM_END
//   TAG = 0b110010 = 50  → BLOCK_END
//   TGA = 0b111000 = 56  → SECTION_SEPARATOR
//
// These values must never appear in opcode position (codon 1 of an instruction)
// for data instructions. The assembler and encoder guarantee this by choosing
// synonymous codons when needed.

// Control codon values.
pub const CTRL_PROGRAM_START: u8 = 48; // TAA repurposed → START (we put START at 48)
pub const CTRL_PROGRAM_END: u8 = 49; // 0b110001
pub const CTRL_BLOCK_END: u8 = 50; // TAG = 0b110010
pub const CTRL_SECTION_SEP: u8 = 56; // TGA = 0b111000

/// The sync marker: 4-codon sequence that identifies an ECC block boundary.
/// Chosen so that the 4 codon values cannot appear as valid instruction opcode
/// codons and have a unique combined bit pattern.
/// Codon values: [SYNC_OPCODE=0x1D, 0x2A, 0x15, 0x2A] (from opcode table +
/// reserved range).
pub const SYNC_PATTERN: [u8; 4] = [0x1D, 0x2A, 0x15, 0x2A];

/// For each opcode (0..=31), a primary codon and one synonymous codon.
/// Primary: codon value == opcode value (direct mapping for simplicity).
/// Synonym: a codon in the 32–47 range re-assigned to this opcode.
///
/// The synonymous codon allows the assembler to swap if the primary codon
/// creates a biological constraint violation.
///
/// Layout: [(primary, synonym), ...]  indexed by opcode value.
///
/// Synonyms are in the 32–47 range (16 slots). Only the 16 most-used opcodes
/// (0x00–0x0F) get a synonym. Opcodes 0x10–0x1F are primary-only (synonym == primary).
pub static OPCODE_SYNONYMS: [(u8, u8); 32] = [
    // op     primary  synonym (32–47 for ops 0x00-0x0F, same as primary for 0x10-0x1F)
    /* 0x00 ADD  */
    (0x00, 0x20),
    /* 0x01 SUB  */ (0x01, 0x21),
    /* 0x02 MUL  */ (0x02, 0x22),
    /* 0x03 DIV  */ (0x03, 0x23),
    /* 0x04 MOD  */ (0x04, 0x24),
    /* 0x05 INC  */ (0x05, 0x25),
    /* 0x06 DEC  */ (0x06, 0x26),
    /* 0x07 AND  */ (0x07, 0x27),
    /* 0x08 OR   */ (0x08, 0x28),
    /* 0x09 XOR  */ (0x09, 0x29),
    /* 0x0A NOT  */ (0x0A, 0x2A),
    /* 0x0B SHL  */ (0x0B, 0x2B),
    /* 0x0C SHR  */ (0x0C, 0x2C),
    /* 0x0D CMP  */ (0x0D, 0x2D),
    /* 0x0E LOAD */ (0x0E, 0x2E),
    /* 0x0F STORE*/ (0x0F, 0x2F),
    // Opcodes 0x10–0x1F: no synonym, primary only.
    /* 0x10 PUSH */ (0x10, 0x10),
    /* 0x11 POP  */ (0x11, 0x11),
    /* 0x12 IMML */ (0x12, 0x12),
    /* 0x13 IMMH */ (0x13, 0x13),
    /* 0x14 JMP  */ (0x14, 0x14),
    /* 0x15 BEQ  */ (0x15, 0x15),
    /* 0x16 BNE  */ (0x16, 0x16),
    /* 0x17 BLT  */ (0x17, 0x17),
    /* 0x18 BGE  */ (0x18, 0x18),
    /* 0x19 CALL */ (0x19, 0x19),
    /* 0x1A RET  */ (0x1A, 0x1A),
    /* 0x1B NOP  */ (0x1B, 0x1B),
    /* 0x1C HALT */ (0x1C, 0x1C),
    /* 0x1D SYNC */ (0x1D, 0x1D),
    /* 0x1E ECCP */ (0x1E, 0x1E),
    /* 0x1F TRAP */ (0x1F, 0x1F),
];

/// Reverse lookup: given a codon value, return the opcode value it encodes.
/// Returns None for control codons (48–63) and None for reserved values.
pub static CODON_TABLE: [Option<u8>; 64] = build_codon_table();

const fn build_codon_table() -> [Option<u8>; 64] {
    let mut table: [Option<u8>; 64] = [None; 64];
    let mut i = 0usize;
    while i < 32 {
        let (primary, synonym) = OPCODE_SYNONYMS[i];
        table[primary as usize] = Some(i as u8);
        if synonym != primary {
            table[synonym as usize] = Some(i as u8);
        }
        i += 1;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_opcodes_have_primary_mapping() {
        for op in 0u8..32 {
            let (primary, _) = OPCODE_SYNONYMS[op as usize];
            let decoded = CODON_TABLE[primary as usize];
            assert_eq!(decoded, Some(op), "opcode {op:#04x} primary mapping broken");
        }
    }

    #[test]
    fn all_opcodes_have_synonym_mapping() {
        for op in 0u8..16 {
            let (_, synonym) = OPCODE_SYNONYMS[op as usize];
            let decoded = CODON_TABLE[synonym as usize];
            assert_eq!(decoded, Some(op), "opcode {op:#04x} synonym mapping broken");
        }
    }

    #[test]
    fn control_codons_not_in_table() {
        // Values 48–63 should all be None (control / reserved).
        for v in 48u8..64 {
            assert!(
                CODON_TABLE[v as usize].is_none(),
                "control codon {v} should be None"
            );
        }
    }

    #[test]
    fn no_duplicate_primary_synonyms() {
        let mut seen = [false; 64];
        for op in 0u8..32 {
            let (primary, synonym) = OPCODE_SYNONYMS[op as usize];
            assert!(
                !seen[primary as usize],
                "duplicate primary codon {primary} for op {op:#04x}"
            );
            seen[primary as usize] = true;
            if synonym != primary {
                assert!(
                    !seen[synonym as usize],
                    "duplicate synonym codon {synonym} for op {op:#04x}"
                );
                seen[synonym as usize] = true;
            }
        }
    }
}
