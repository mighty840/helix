use crate::{gc, homopolymer, repeats, structure};
use helix_isa::table::OPCODE_SYNONYMS;
use helix_isa::{Base, Codon, DnaSequence};

#[derive(Debug, Clone)]
pub enum LintViolation {
    GcImbalance {
        pos: usize,
        gc_fraction: f64,
    },
    Homopolymer {
        pos: usize,
        base: Base,
        run_length: usize,
    },
    DinucRepeat {
        pos: usize,
        repeat_count: usize,
    },
    Palindrome {
        pos: usize,
        length: usize,
    },
}

#[derive(Debug, Default)]
pub struct LintReport {
    pub violations: Vec<LintViolation>,
    pub swaps_applied: usize,
}

impl LintReport {
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Optimizes a sequence of instruction codons (multiple of 4, one instr = 4 codons)
/// by substituting synonymous opcode codons to reduce biological constraint violations.
///
/// Only the first codon of each instruction (the opcode codon) can be swapped, since
/// field codons encode register/immediate values and must not change.
pub struct SequenceOptimizer {
    pub max_passes: usize,
}

impl Default for SequenceOptimizer {
    fn default() -> Self {
        Self { max_passes: 5 }
    }
}

impl SequenceOptimizer {
    pub fn new(max_passes: usize) -> Self {
        Self { max_passes }
    }

    /// Run the linter on a raw codon slice and return a report without modifying anything.
    pub fn lint(&self, codons: &[Codon]) -> LintReport {
        let bases = codons_to_bases(codons);
        self.lint_bases(&bases)
    }

    fn lint_bases(&self, bases: &[Base]) -> LintReport {
        let mut report = LintReport::default();

        for v in gc::scan(bases) {
            report.violations.push(LintViolation::GcImbalance {
                pos: v.pos,
                gc_fraction: v.gc_fraction,
            });
        }
        for v in homopolymer::scan(bases) {
            report.violations.push(LintViolation::Homopolymer {
                pos: v.pos,
                base: v.base,
                run_length: v.run_length,
            });
        }
        for v in repeats::scan(bases) {
            report.violations.push(LintViolation::DinucRepeat {
                pos: v.pos,
                repeat_count: v.repeat_count,
            });
        }
        for v in structure::scan(bases) {
            report.violations.push(LintViolation::Palindrome {
                pos: v.pos,
                length: v.length,
            });
        }

        report
    }

    /// Optimize a codon slice in-place (codons must be instruction-aligned, multiples of 4).
    /// Returns the final lint report after all passes.
    pub fn optimize(&self, codons: &mut [Codon]) -> LintReport {
        assert!(codons.len().is_multiple_of(4));

        let mut swaps = 0usize;

        for _pass in 0..self.max_passes {
            let before = self.lint(codons);
            if before.is_clean() {
                let mut report = before;
                report.swaps_applied = swaps;
                return report;
            }

            // For each violation, try swapping the opcode codon of the nearest instruction.
            let mut changed = false;
            let bases = codons_to_bases(codons);

            for viol in &before.violations {
                let nt_pos = violation_pos(viol);
                // Find the instruction index whose opcode codon overlaps this position.
                // Each instruction = 4 codons = 12 nucleotides.
                let instr_idx = (nt_pos / 12).min(codons.len() / 4 - 1);
                let opcode_codon_idx = instr_idx * 4; // first codon of this instruction

                if try_swap_synonym(codons, opcode_codon_idx) {
                    swaps += 1;
                    changed = true;
                    // Restart inner scan after each swap.
                    break;
                }

                // Also try swapping neighboring instructions.
                for delta in [1usize, 2] {
                    let neighbor = opcode_codon_idx.saturating_sub(delta * 4);
                    if neighbor != opcode_codon_idx && try_swap_synonym(codons, neighbor) {
                        swaps += 1;
                        changed = true;
                        break;
                    }
                }
                if changed {
                    break;
                }
            }

            let _ = bases; // suppress unused warning
            if !changed {
                break;
            }
        }

        let mut report = self.lint(codons);
        report.swaps_applied = swaps;
        report
    }
}

fn violation_pos(v: &LintViolation) -> usize {
    match v {
        LintViolation::GcImbalance { pos, .. } => *pos,
        LintViolation::Homopolymer { pos, .. } => *pos,
        LintViolation::DinucRepeat { pos, .. } => *pos,
        LintViolation::Palindrome { pos, .. } => *pos,
    }
}

/// Try to swap the codon at `idx` to its synonym (if different). Returns true if swapped.
fn try_swap_synonym(codons: &mut [Codon], idx: usize) -> bool {
    if idx >= codons.len() {
        return false;
    }
    let current_val = codons[idx].value();
    // Find which opcode this codon maps to, and what its synonym is.
    for &(primary, synonym) in &OPCODE_SYNONYMS {
        if current_val == primary && synonym != primary {
            codons[idx] = Codon::new(synonym);
            return true;
        }
        if current_val == synonym && synonym != primary {
            codons[idx] = Codon::new(primary);
            return true;
        }
    }
    false
}

fn codons_to_bases(codons: &[Codon]) -> Vec<Base> {
    codons.iter().flat_map(|c| c.bases()).collect()
}

/// Convert a DnaSequence to a codon vec (must be nucleotide-length divisible by 3).
pub fn dna_to_codons(seq: &DnaSequence) -> Vec<Codon> {
    seq.codons().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_isa::{encode_instruction, Instruction, Register};

    fn make_codons(instrs: &[Instruction]) -> Vec<Codon> {
        instrs.iter().flat_map(encode_instruction).collect()
    }

    #[test]
    fn lint_clean_program() {
        let instrs = [
            Instruction::Imml(Register::R1, 5),
            Instruction::Add(Register::R3, Register::R1, Register::R2),
            Instruction::Halt,
        ];
        let codons = make_codons(&instrs);
        let opt = SequenceOptimizer::default();
        let report = opt.lint(&codons);
        // Short sequences rarely violate GC by much; just confirm it runs.
        println!("violations: {}", report.violations.len());
    }

    #[test]
    fn synonym_swap_changes_codon() {
        // NOP primary = 0x1B, synonym = 0x1B (no synonym for ops 0x10+)
        // ADD primary = 0x00, synonym = 0x20
        let mut codons = vec![
            Codon::new(0x00),
            Codon::new(0),
            Codon::new(0),
            Codon::new(0), // ADD
        ];
        let swapped = try_swap_synonym(&mut codons, 0);
        assert!(swapped);
        assert_eq!(codons[0].value(), 0x20); // synonym of ADD
    }

    #[test]
    fn swap_back_to_primary() {
        let mut codons = vec![
            Codon::new(0x20),
            Codon::new(0),
            Codon::new(0),
            Codon::new(0), // ADD synonym
        ];
        let swapped = try_swap_synonym(&mut codons, 0);
        assert!(swapped);
        assert_eq!(codons[0].value(), 0x00); // back to primary
    }

    #[test]
    fn optimize_runs_without_panic() {
        let instrs: Vec<Instruction> = (0..32).map(|_| Instruction::Nop).collect();
        let mut codons = make_codons(&instrs);
        let opt = SequenceOptimizer::new(3);
        let report = opt.optimize(&mut codons);
        println!(
            "swaps: {}, violations: {}",
            report.swaps_applied,
            report.violations.len()
        );
    }
}
