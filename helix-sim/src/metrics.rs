use crate::inject::{ErrorInjector, InjectionConfig, InjectionStats};
use helix_ecc::{EccConfig, EccDecoder};
use helix_isa::{decode_instruction, Codon, DnaSequence};

/// Full simulation report for one run.
#[derive(Debug)]
pub struct SimReport {
    pub original_len_nt: usize,
    pub corrupted_len_nt: usize,
    pub injection: InjectionStats,
    pub instructions_original: usize,
    pub instructions_recovered: usize,
    pub instructions_corrupted: usize,
    pub ecc_success: bool,
    pub execution_matches: bool,
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
    }
}

/// Run a full simulation: inject errors into a DNA string, decode, compare instructions.
pub fn simulate(
    original_dna: &str,
    original_instrs: &[[Codon; 4]],
    inject_cfg: InjectionConfig,
    ecc_cfg: EccConfig,
) -> SimReport {
    let injector = ErrorInjector::new(inject_cfg);
    let (corrupted_dna, injection_stats) = injector.inject(original_dna);

    let decoder = EccDecoder::new(ecc_cfg);
    let seq = DnaSequence::parse(&corrupted_dna).unwrap_or_default();
    let codons: Vec<Codon> = seq.codons().collect();

    let (recovered_instrs, ecc_success) = match decoder.decode(&codons) {
        Ok(raw) => {
            let instrs: Vec<[Codon; 4]> = raw
                .chunks_exact(4)
                .map(|c: &[Codon]| <[Codon; 4]>::try_from(c).unwrap())
                .collect();
            (instrs, true)
        }
        Err(_) => (Vec::new(), false),
    };

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

    let corrupted = n_orig.saturating_sub(matching);
    let execution_matches = matching == n_orig;

    SimReport {
        original_len_nt: original_dna.len(),
        corrupted_len_nt: corrupted_dna.len(),
        injection: injection_stats,
        instructions_original: n_orig,
        instructions_recovered: matching,
        instructions_corrupted: corrupted,
        ecc_success,
        execution_matches,
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
        };
        let encoder = EccEncoder::new(ecc_cfg);
        let encoded = encoder.encode(&raw);
        let dna: String = encoded.iter().flat_map(|c| c.to_dna()).collect();

        let instr_arrays: Vec<[Codon; 4]> = raw
            .chunks_exact(4)
            .map(|c: &[Codon]| <[Codon; 4]>::try_from(c).unwrap())
            .collect();

        (instr_arrays, dna)
    }

    #[test]
    fn zero_error_rate_full_recovery() {
        let (instrs, dna) = make_program();
        let inject_cfg = InjectionConfig::composite(0.0, 0.0, 0.0, 1);
        let ecc_cfg = EccConfig {
            tmr: false,
            block_size: 8,
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
        };
        let report = simulate(&dna, &instrs, inject_cfg, ecc_cfg);
        report.print_summary();
        // At 0.2% sub rate recovery should be near perfect.
        assert!(report.recovery_rate() >= 0.5);
    }
}
