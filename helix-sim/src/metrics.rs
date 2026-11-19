use crate::inject::{ErrorInjector, InjectionConfig, InjectionStats};
use helix_ecc::{EccConfig, EccDecoder};
use helix_isa::{decode_instruction, Codon, DnaSequence};
use helix_vm::memory::MEM_SIZE;
use helix_vm::{Cpu, Memory, ProgramLoader};

/// What happened to the program, judged by running it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The recovered program ran to the same final state as the original.
    Correct,
    /// Decoding, the checksum, loading or running failed visibly.
    Detected(String),
    /// The recovered program ran and ended in a different state, with no error.
    SilentWrong,
}

impl Outcome {
    pub fn label(&self) -> &'static str {
        match self {
            Outcome::Correct => "correct",
            Outcome::Detected(_) => "detected",
            Outcome::SilentWrong => "SILENT WRONG",
        }
    }
}

/// Full simulation report for one run.
#[derive(Debug)]
pub struct SimReport {
    pub original_len_nt: usize,
    pub corrupted_len_nt: usize,
    pub injection: InjectionStats,
    pub instructions_original: usize,
    /// Instructions equal to the original after ECC, with the checksum ignored
    /// (comparable to v0.1.x numbers).
    pub instructions_recovered: usize,
    pub instructions_corrupted: usize,
    /// Whether decoding with the given `EccConfig` (checksum included) succeeded.
    pub ecc_success: bool,
    /// True when `outcome` is `Correct`.
    pub execution_matches: bool,
    /// End-to-end result: decode with the given config, load, run, compare.
    pub outcome: Outcome,
}

impl SimReport {
    pub fn recovery_rate(&self) -> f64 {
        if self.instructions_original == 0 {
            return 1.0;
        }
        self.instructions_recovered as f64 / self.instructions_original as f64
    }

    pub fn print_summary(&self) {
        println!("=== Simulation Report ===");
        println!("Original:  {} nt", self.original_len_nt);
        println!("Corrupted: {} nt", self.corrupted_len_nt);
        println!(
            "Errors:    {} subs, {} dels, {} ins (total: {})",
            self.injection.substitutions,
            self.injection.deletions,
            self.injection.insertions,
            self.injection.total_errors()
        );
        println!(
            "Recovery:  {}/{} instructions ({:.1}%)",
            self.instructions_recovered,
            self.instructions_original,
            self.recovery_rate() * 100.0
        );
        println!("ECC ok:    {}", self.ecc_success);
        println!("Output match: {}", self.execution_matches);
        match &self.outcome {
            Outcome::Detected(why) => println!("Outcome:   detected ({why})"),
            other => println!("Outcome:   {}", other.label()),
        }
    }
}

/// Final state of one run: how it ended, the registers and all of memory.
#[derive(Debug, PartialEq, Eq)]
struct FinalState {
    result: Result<(), String>,
    regs: [u16; 8],
    memory: Vec<u16>,
}

fn run(instrs: &[[Codon; 4]]) -> FinalState {
    let mut cpu = Cpu::new(Memory::new());
    let result = cpu.run(instrs).map(|_| ()).map_err(|e| e.to_string());
    let memory = (0..MEM_SIZE as u16)
        .map(|a| cpu.memory.load(a).unwrap_or(0))
        .collect();
    FinalState {
        result,
        regs: cpu.regs,
        memory,
    }
}

fn to_instrs(raw: &[Codon]) -> Vec<[Codon; 4]> {
    raw.as_chunks::<4>().0.to_vec()
}

/// Decode `corrupted_dna` with `ecc_cfg`, run it, and compare it with running
/// `original_instrs`. Returns the outcome and whether decoding succeeded.
pub fn evaluate(
    corrupted_dna: &str,
    original_instrs: &[[Codon; 4]],
    ecc_cfg: EccConfig,
) -> (Outcome, bool) {
    let codons: Vec<Codon> = match DnaSequence::parse(corrupted_dna) {
        Ok(seq) => seq.codons().collect(),
        Err(e) => return (Outcome::Detected(format!("parse: {e}")), false),
    };
    let raw = match EccDecoder::new(ecc_cfg).decode(&codons) {
        Ok(raw) => raw,
        Err(e) => return (Outcome::Detected(format!("ECC: {e}")), false),
    };
    let program = match ProgramLoader::from_raw_codons(&raw) {
        Ok(p) if !p.instructions.is_empty() => p,
        Ok(_) => return (Outcome::Detected("load: 0 instructions".into()), true),
        Err(e) => return (Outcome::Detected(format!("load: {e}")), true),
    };
    let expected = run(original_instrs);
    let got = run(&program.instructions);
    let outcome = if got == expected {
        Outcome::Correct
    } else if let (Err(e), Ok(())) = (&got.result, &expected.result) {
        Outcome::Detected(format!("run: {e}"))
    } else {
        Outcome::SilentWrong
    };
    (outcome, true)
}

/// Run a full simulation: inject errors into a DNA string, decode, compare
/// instructions, then run the program and compare its final state.
pub fn simulate(
    original_dna: &str,
    original_instrs: &[[Codon; 4]],
    inject_cfg: InjectionConfig,
    ecc_cfg: EccConfig,
) -> SimReport {
    let injector = ErrorInjector::new(inject_cfg);
    let (corrupted_dna, injection_stats) = injector.inject(original_dna);

    // Instruction-level recovery, as in v0.1.x: what ECC got back, with the
    // checksum ignored.
    let seq = DnaSequence::parse(&corrupted_dna).unwrap_or_default();
    let codons: Vec<Codon> = seq.codons().collect();
    let unchecked = EccConfig {
        checksum: false,
        ..ecc_cfg
    };
    let recovered_instrs = EccDecoder::new(unchecked)
        .decode(&codons)
        .map(|raw| to_instrs(&raw))
        .unwrap_or_default();

    let n_orig = original_instrs.len();
    let n_cmp = n_orig.min(recovered_instrs.len());
    let mut matching = 0usize;
    for i in 0..n_cmp {
        let orig = decode_instruction(&original_instrs[i]).ok();
        let recv = decode_instruction(&recovered_instrs[i]).ok();
        if orig == recv && orig.is_some() {
            matching += 1;
        }
    }

    let (outcome, ecc_success) = evaluate(&corrupted_dna, original_instrs, ecc_cfg);

    SimReport {
        original_len_nt: original_dna.len(),
        corrupted_len_nt: corrupted_dna.len(),
        injection: injection_stats,
        instructions_original: n_orig,
        instructions_recovered: matching,
        instructions_corrupted: n_orig.saturating_sub(matching),
        ecc_success,
        execution_matches: outcome == Outcome::Correct,
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_ecc::EccEncoder;
    use helix_isa::{encode_instruction, Instruction, Register};

    fn make_program() -> (Vec<[Codon; 4]>, String) {
        let instrs = [
            Instruction::Imml(Register::R1, 5),
            Instruction::Imml(Register::R2, 3),
            Instruction::Add(Register::R3, Register::R1, Register::R2),
            Instruction::Halt,
        ];
        let raw: Vec<Codon> = instrs.iter().flat_map(encode_instruction).collect();

        let ecc_cfg = EccConfig {
            tmr: false,
            block_size: 8,
            checksum: true,
        };
        let encoder = EccEncoder::new(ecc_cfg);
        let encoded = encoder.encode(&raw);
        let dna: String = encoded.iter().flat_map(|c| c.to_dna()).collect();

        let instr_arrays: Vec<[Codon; 4]> = raw.as_chunks::<4>().0.to_vec();

        (instr_arrays, dna)
    }

    #[test]
    fn zero_error_rate_full_recovery() {
        let (instrs, dna) = make_program();
        let inject_cfg = InjectionConfig::composite(0.0, 0.0, 0.0, 1);
        let ecc_cfg = EccConfig {
            tmr: false,
            block_size: 8,
            checksum: true,
        };
        let report = simulate(&dna, &instrs, inject_cfg, ecc_cfg);
        assert!(report.ecc_success);
        assert_eq!(report.injection.total_errors(), 0);
    }

    #[test]
    fn low_error_rate_high_recovery() {
        let (instrs, dna) = make_program();
        let inject_cfg = InjectionConfig::composite(0.002, 0.0, 0.0, 42);
        let ecc_cfg = EccConfig {
            tmr: false,
            block_size: 8,
            checksum: true,
        };
        let report = simulate(&dna, &instrs, inject_cfg, ecc_cfg);
        report.print_summary();
        // At 0.2% sub rate recovery should be near perfect.
        assert!(report.recovery_rate() >= 0.5);
    }

    /// The same program and ECC, with one letter changed by hand.
    fn flip(dna: &str, pos: usize) -> String {
        let mut v: Vec<char> = dna.chars().collect();
        v[pos] = if v[pos] == 'A' { 'C' } else { 'A' };
        v.into_iter().collect()
    }

    #[test]
    fn clean_strand_is_correct() {
        let (instrs, dna) = make_program();
        let cfg = EccConfig {
            tmr: false,
            ..EccConfig::default()
        };
        assert_eq!(evaluate(&dna, &instrs, cfg).0, Outcome::Correct);
    }

    #[test]
    fn operand_change_is_silent_without_checksum_and_detected_with_it() {
        // Letter 22 (0-based) is inside IMML R1's constant: 5 becomes another
        // value, which Reed-Solomon (opcodes only) does not cover.
        let (instrs, _) = make_program();
        let raw: Vec<Codon> = instrs.iter().flatten().copied().collect();
        for checksum in [false, true] {
            let cfg = EccConfig {
                tmr: false,
                block_size: 8,
                checksum,
            };
            let dna: String = helix_ecc::EccEncoder::new(cfg)
                .encode(&raw)
                .iter()
                .flat_map(|c| c.to_dna())
                .collect();
            let (outcome, _) = evaluate(&flip(&dna, 22), &instrs, cfg);
            if checksum {
                assert!(matches!(outcome, Outcome::Detected(_)), "{outcome:?}");
            } else {
                assert_eq!(outcome, Outcome::SilentWrong);
            }
        }
    }
}
