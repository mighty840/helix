use helix_ecc::{EccConfig, EccDecoder};
use helix_isa::{decode_instruction, Codon, DnaSequence};
use thiserror::Error;

/// A decoded, ready-to-execute program.
#[derive(Debug)]
pub struct Program {
    /// Instructions as 4-codon arrays, ready for the CPU.
    pub instructions: Vec<[Codon; 4]>,
    /// Data words to preload into memory at address 0.
    pub data: Vec<u16>,
    /// Entry point instruction index.
    pub entry: usize,
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("invalid DNA string: {0}")]
    InvalidDna(#[from] helix_isa::codon::DnaParseError),
    #[error("sequence too short to contain a valid program")]
    TooShort,
    #[error("ECC decode failed: {0}")]
    Ecc(#[from] helix_ecc::EccError),
    #[error("instruction decode failed: {0}")]
    Decode(#[from] helix_isa::DecodeError),
    #[error("codon count is not a multiple of 4")]
    UnalignedCodons,
}

pub struct ProgramLoader {
    config: EccConfig,
}

impl ProgramLoader {
    pub fn new(config: EccConfig) -> Self {
        Self { config }
    }

    /// Load a program directly from a raw instruction codon slice (no ECC, no DNA string).
    /// Used by the assembler and for tests.
    pub fn from_raw_codons(instr_codons: &[Codon]) -> Result<Program, LoadError> {
        if !instr_codons.len().is_multiple_of(4) {
            return Err(LoadError::UnalignedCodons);
        }
        let instructions = instr_codons
            .chunks_exact(4)
            .map(|chunk| {
                let arr: [Codon; 4] = chunk.try_into().unwrap();
                // Validate each instruction decodes without error.
                decode_instruction(&arr)?;
                Ok(arr)
            })
            .collect::<Result<Vec<_>, helix_isa::DecodeError>>()?;

        Ok(Program {
            instructions,
            data: Vec::new(),
            entry: 0,
        })
    }

    /// Load a program from a DNA string with full ECC pipeline.
    pub fn from_dna_string(&self, dna: &str) -> Result<Program, LoadError> {
        let seq = DnaSequence::parse(dna)?;
        let codons: Vec<Codon> = seq.codons().collect();

        if codons.is_empty() {
            return Err(LoadError::TooShort);
        }

        let decoder = EccDecoder::new(self.config);
        let instr_codons = decoder.decode(&codons)?;

        Self::from_raw_codons(&instr_codons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_ecc::EccEncoder;
    use helix_isa::{encode_instruction, Instruction, Register};

    #[test]
    fn load_raw_codons() {
        let instrs = [Instruction::Imml(Register::R1, 42), Instruction::Halt];
        let codons: Vec<Codon> = instrs.iter().flat_map(encode_instruction).collect();
        let prog = ProgramLoader::from_raw_codons(&codons).unwrap();
        assert_eq!(prog.instructions.len(), 2);
    }

    #[test]
    fn raw_codons_must_be_multiple_of_4() {
        let bad = vec![Codon::new(0), Codon::new(1), Codon::new(2)];
        assert!(matches!(
            ProgramLoader::from_raw_codons(&bad),
            Err(LoadError::UnalignedCodons)
        ));
    }

    #[test]
    fn load_dna_string_round_trip() {
        let instrs = [
            Instruction::Imml(Register::R1, 7),
            Instruction::Imml(Register::R2, 3),
            Instruction::Add(Register::R3, Register::R1, Register::R2),
            Instruction::Halt,
        ];
        let raw: Vec<Codon> = instrs.iter().flat_map(encode_instruction).collect();

        let config = EccConfig {
            tmr: false,
            block_size: 8,
        };
        let encoder = EccEncoder::new(config);
        let encoded_codons = encoder.encode(&raw);

        // Convert to DNA string.
        let dna: String = encoded_codons.iter().flat_map(|c| c.to_dna()).collect();

        let loader = ProgramLoader::new(config);
        let prog = loader.from_dna_string(&dna).unwrap();

        // First 4 instructions match (rest are NOP padding).
        assert!(prog.instructions.len() >= 4);
        for (i, instr) in instrs.iter().enumerate() {
            let decoded = decode_instruction(&prog.instructions[i]).unwrap();
            assert_eq!(decoded, *instr);
        }
    }
}
