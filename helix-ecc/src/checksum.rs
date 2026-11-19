//! Program checksum (v0.2): a CRC over the program's canonical form.
//!
//! The footer `[TAA, c0, c1, c2]` follows the last block. `TAA` (value 48)
//! can never be an opcode (opcodes and their synonyms are 0 to 47), so the
//! footer cannot be mistaken for an instruction. The 16-bit CRC is split over
//! three codons: 4, 6 and 6 bits.
//!
//! The CRC covers the *canonical* form of the instructions: each instruction
//! is decoded and re-encoded with its primary opcode codon and zeroed padding.
//! A change that the decoder ignores anyway (a padding bit, an unused field,
//! a synonymous opcode codon) leaves the checksum valid; any change to what
//! the program means makes it fail, unless it collides (1 in 65,536 for a
//! random change).

use helix_isa::{decode_instruction, encode_instruction, Codon, DecodeError};

/// First codon of the footer: TAA = 0b110000 = 48 (PROGRAM_END in SPEC 4.1).
pub const FOOTER_MARKER: u8 = 0x30;

/// CRC-16/CCITT-FALSE (polynomial 0x1021, initial value 0xFFFF), the same
/// CRC ferrite-sdk uses for its chunks.
pub fn crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// CRC over the canonical form of a stream of instruction codons
/// (length a multiple of 4). Fails if any instruction does not decode.
pub fn program_checksum(instr_codons: &[Codon]) -> Result<u16, DecodeError> {
    let mut canonical: Vec<u8> = Vec::with_capacity(instr_codons.len());
    let (instructions, _) = instr_codons.as_chunks::<4>();
    for arr in instructions {
        let instr = decode_instruction(arr)?;
        canonical.extend(encode_instruction(&instr).iter().map(|c| c.value()));
    }
    Ok(crc16_ccitt(&canonical))
}

/// The 4-codon footer carrying `crc`.
pub fn footer(crc: u16) -> [Codon; 4] {
    [
        Codon::new(FOOTER_MARKER),
        Codon::new((crc >> 12) as u8),
        Codon::new(((crc >> 6) & 0x3F) as u8),
        Codon::new((crc & 0x3F) as u8),
    ]
}

/// Read the CRC back from a footer, or `None` if `codons` is not a footer.
pub fn read_footer(codons: &[Codon]) -> Option<u16> {
    if codons.len() != 4 || codons[0].value() != FOOTER_MARKER || codons[1].value() > 0x0F {
        return None;
    }
    Some(
        ((codons[1].value() as u16) << 12)
            | ((codons[2].value() as u16) << 6)
            | codons[3].value() as u16,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_isa::{Instruction, Register::*};

    fn codons(instrs: &[Instruction]) -> Vec<Codon> {
        instrs.iter().flat_map(encode_instruction).collect()
    }

    #[test]
    fn crc_matches_ccitt_false_check_value() {
        // Standard check value for CRC-16/CCITT-FALSE over "123456789".
        assert_eq!(crc16_ccitt(b"123456789"), 0x29B1);
    }

    #[test]
    fn footer_roundtrip() {
        for crc in [0u16, 1, 0x29B1, 0xFFFF, 0x8001] {
            assert_eq!(read_footer(&footer(crc)), Some(crc));
        }
    }

    #[test]
    fn padding_bits_do_not_change_the_checksum() {
        let prog = codons(&[Instruction::Imml(R1, 7), Instruction::Halt]);
        let mut changed = prog.clone();
        changed[3] = Codon::new(changed[3].value() | 0b01); // padding bit of IMML's last codon
        assert_eq!(
            program_checksum(&prog).unwrap(),
            program_checksum(&changed).unwrap()
        );
    }

    #[test]
    fn synonym_opcode_does_not_change_the_checksum() {
        let prog = codons(&[Instruction::Add(R3, R1, R2), Instruction::Halt]);
        let mut changed = prog.clone();
        changed[0] = Codon::new(0x20); // GAA, ADD's synonym
        assert_eq!(
            program_checksum(&prog).unwrap(),
            program_checksum(&changed).unwrap()
        );
    }

    #[test]
    fn changed_constant_changes_the_checksum() {
        let a = codons(&[Instruction::Imml(R1, 7), Instruction::Halt]);
        let b = codons(&[Instruction::Imml(R1, 5), Instruction::Halt]);
        assert_ne!(program_checksum(&a).unwrap(), program_checksum(&b).unwrap());
    }
}
