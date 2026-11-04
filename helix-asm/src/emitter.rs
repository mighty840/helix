use crate::parser::{parse, ParseError};
use crate::resolver::{resolve, ResolveError, ResolvedProgram};
use anyhow::Result;
use helix_bio::SequenceOptimizer;
use helix_ecc::{EccConfig, EccEncoder};
use helix_isa::{encode_instruction, Codon, DnaSequence, Instruction, Register};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssembleError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("resolve error: {0}")]
    Resolve(#[from] ResolveError),
    #[error(
        "entry point at instruction {0} is out of JMP range for the loader prologue (max 126)"
    )]
    EntryOutOfRange(usize),
}

/// Assembly output.
pub struct AssemblyOutput {
    /// The DNA nucleotide string (ACGT).
    pub dna: String,
    /// Raw instruction codons (before ECC) of the program's own instructions,
    /// without the loader prologue, for direct VM use: run them from
    /// `program.entry` with `program.data` preloaded.
    pub raw_codons: Vec<Codon>,
    /// Number of loader-prologue instructions placed before `raw_codons` in
    /// the DNA. The DNA program starts at instruction 0, writes the data
    /// section into memory and jumps to the entry point (see `loader_prologue`).
    pub prologue_len: usize,
    /// Resolved program metadata.
    pub program: ResolvedProgram,
    /// Number of bio-safety swaps applied.
    pub bio_swaps: usize,
    /// Number of bio-safety violations remaining (may be non-zero if unfixable).
    pub bio_violations: usize,
}

pub struct Assembler {
    pub ecc: EccConfig,
    pub bio_optimize: bool,
}

impl Default for Assembler {
    fn default() -> Self {
        Self {
            ecc: EccConfig::default(),
            bio_optimize: true,
        }
    }
}

impl Assembler {
    pub fn new(ecc: EccConfig, bio_optimize: bool) -> Self {
        Self { ecc, bio_optimize }
    }

    pub fn assemble(&self, source: &str) -> Result<AssemblyOutput, AssembleError> {
        let parsed = parse(source)?;
        let program = resolve(&parsed)?;

        // Loader prologue (data section and entry point), then the program.
        let prologue = loader_prologue(&program.data, program.entry)?;
        let prologue_len = prologue.len();
        let mut image_codons: Vec<Codon> = prologue
            .iter()
            .chain(program.instructions.iter())
            .flat_map(encode_instruction)
            .collect();

        // Bio-safety optimization.
        let (bio_swaps, bio_violations) = if self.bio_optimize {
            let opt = SequenceOptimizer::default();
            let report = opt.optimize(&mut image_codons);
            (report.swaps_applied, report.violations.len())
        } else {
            (0, 0)
        };
        let raw_codons = image_codons[prologue_len * 4..].to_vec();

        // ECC encode.
        let encoder = EccEncoder::new(self.ecc);
        let ecc_codons = encoder.encode(&image_codons);

        // Convert to DNA string.
        let seq: DnaSequence = {
            let mut s = DnaSequence::new();
            for c in &ecc_codons {
                s.push_codon(*c);
            }
            s
        };

        Ok(AssemblyOutput {
            dna: seq.as_str(),
            raw_codons,
            prologue_len,
            program,
            bio_swaps,
            bio_violations,
        })
    }
}

/// Instructions that put a DNA program into its start state when it runs from
/// instruction 0: each `.data` word is stored at addresses 0, 1, 2, ... (where
/// the in-memory loader puts it), the scratch registers R6 and R7 are reset to
/// 0, and a JMP skips to the entry point if it is not the first instruction.
/// Empty when there is no data and the entry is 0.
///
/// Per data word: `IMML R7, lo; IMMH R7, hi; STORE R6, R7; INC R6, R6`.
/// None of these instructions changes the flags (only CMP does).
pub fn loader_prologue(data: &[u16], entry: usize) -> Result<Vec<Instruction>, AssembleError> {
    use Register::{R0, R6, R7};
    let mut out = Vec::new();
    for &word in data {
        out.push(Instruction::Imml(R7, (word & 0xFF) as u8));
        out.push(Instruction::Immh(R7, (word >> 8) as u8));
        out.push(Instruction::Store(R6, R7));
        out.push(Instruction::Inc(R6, R6));
    }
    if !data.is_empty() {
        out.push(Instruction::Add(R6, R0, R0));
        out.push(Instruction::Add(R7, R0, R0));
    }
    if entry != 0 {
        // The JMP sits right before the program's instruction 0, so the entry
        // is `entry + 1` instructions ahead of it.
        let offset = i8::try_from(entry + 1).map_err(|_| AssembleError::EntryOutOfRange(entry))?;
        out.push(Instruction::Jmp(offset));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADD_PROGRAM: &str = r#"
.program "add"
.version 1
.text
.entry main
main:
    IMML R1, 7
    IMML R2, 3
    ADD  R3, R1, R2
    HALT
"#;

    #[test]
    fn assemble_produces_dna() {
        let asm = Assembler::default();
        let out = asm.assemble(ADD_PROGRAM).unwrap();
        assert!(!out.dna.is_empty());
        assert!(out.dna.chars().all(|c| "ACGT".contains(c)));
    }

    #[test]
    fn raw_codons_multiple_of_4() {
        let asm = Assembler::default();
        let out = asm.assemble(ADD_PROGRAM).unwrap();
        assert_eq!(out.raw_codons.len() % 4, 0);
    }

    #[test]
    fn assemble_and_dna_is_valid() {
        let asm = Assembler::default();
        let out = asm.assemble(ADD_PROGRAM).unwrap();
        // DNA string length must be divisible by 3 (codon-aligned).
        assert_eq!(out.dna.len() % 3, 0);
    }

    #[test]
    fn assemble_with_data_section() {
        let src = r#"
.program "sort"
.data
arr: .words 3, 1, 4, 1, 5
.text
.entry main
main:
    IMML R1, 0
    HALT
"#;
        let asm = Assembler::default();
        let out = asm.assemble(src).unwrap();
        assert_eq!(out.program.data, vec![3, 1, 4, 1, 5]);
        assert!(!out.dna.is_empty());
    }
}
