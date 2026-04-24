use crate::parser::{parse, ParseError};
use crate::resolver::{resolve, ResolveError, ResolvedProgram};
use anyhow::Result;
use helix_bio::SequenceOptimizer;
use helix_ecc::{EccConfig, EccEncoder};
use helix_isa::{encode_instruction, Codon, DnaSequence};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssembleError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("resolve error: {0}")]
    Resolve(#[from] ResolveError),
}

/// Assembly output.
pub struct AssemblyOutput {
    /// The DNA nucleotide string (ACGT).
    pub dna: String,
    /// Raw instruction codons (before ECC), for direct VM use.
    pub raw_codons: Vec<Codon>,
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

        // Encode instructions to raw codon stream.
        let mut raw_codons: Vec<Codon> = program
            .instructions
            .iter()
            .flat_map(encode_instruction)
            .collect();

        // Bio-safety optimization.
        let (bio_swaps, bio_violations) = if self.bio_optimize {
            let opt = SequenceOptimizer::default();
            let report = opt.optimize(&mut raw_codons);
            (report.swaps_applied, report.violations.len())
        } else {
            (0, 0)
        };

        // ECC encode.
        let encoder = EccEncoder::new(self.ecc);
        let ecc_codons = encoder.encode(&raw_codons);

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
            program,
            bio_swaps,
            bio_violations,
        })
    }
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
