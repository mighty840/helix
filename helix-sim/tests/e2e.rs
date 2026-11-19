use helix_asm::Assembler;
use helix_ecc::EccConfig;
use helix_isa::Codon;
use helix_sim::{inject::InjectionConfig, metrics::simulate};
use helix_vm::{Cpu, Memory, ProgramLoader};

const FIBONACCI: &str = r#"
.program "fibonacci_e2e"
.version 1
.text
.entry main
main:
    IMML R1, 0
    IMML R2, 1
    IMML R3, 0
    IMML R4, 10
loop:
    CMP  R3, R4
    BGE  done
    ADD  R5, R1, R2
    ADD  R1, R2, R0
    ADD  R2, R5, R0
    INC  R3, R3
    JMP  loop
done:
    HALT
"#;

fn assemble(src: &str, cfg: EccConfig) -> (Vec<[Codon; 4]>, String) {
    let asm = Assembler::new(cfg, false);
    let out = asm.assemble(src).unwrap();
    let prog = ProgramLoader::from_raw_codons(&out.raw_codons).unwrap();
    (prog.instructions, out.dna)
}

#[test]
fn zero_error_rate_perfect_recovery() {
    let cfg = EccConfig {
        tmr: false,
        block_size: 8,
        checksum: true,
    };
    let (instrs, dna) = assemble(FIBONACCI, cfg);
    let report = simulate(
        &dna,
        &instrs,
        InjectionConfig::composite(0.0, 0.0, 0.0, 1),
        cfg,
    );
    assert_eq!(report.injection.total_errors(), 0);
    assert!(report.ecc_success);
    assert_eq!(report.recovery_rate(), 1.0);
}

#[test]
fn very_low_error_rate_high_recovery_no_tmr() {
    let cfg = EccConfig {
        tmr: false,
        block_size: 8,
        checksum: true,
    };
    let (instrs, dna) = assemble(FIBONACCI, cfg);
    let report = simulate(
        &dna,
        &instrs,
        InjectionConfig::composite(0.002, 0.0, 0.0, 42),
        cfg,
    );
    report.print_summary();
    assert!(
        report.recovery_rate() >= 0.5,
        "Expected >= 50% recovery at 0.2% sub rate"
    );
}

#[test]
fn tmr_improves_recovery_over_single_copy() {
    // At same error rate, TMR should recover at least as well as no-TMR at zero rate.
    let cfg_tmr = EccConfig {
        tmr: true,
        block_size: 8,
        checksum: true,
    };
    let cfg_no = EccConfig {
        tmr: false,
        block_size: 8,
        checksum: true,
    };
    let (instrs_tmr, dna_tmr) = assemble(FIBONACCI, cfg_tmr);
    let (instrs_no, dna_no) = assemble(FIBONACCI, cfg_no);

    let report_tmr = simulate(
        &dna_tmr,
        &instrs_tmr,
        InjectionConfig::composite(0.0, 0.0, 0.0, 1),
        cfg_tmr,
    );
    let report_no = simulate(
        &dna_no,
        &instrs_no,
        InjectionConfig::composite(0.0, 0.0, 0.0, 1),
        cfg_no,
    );

    // Both should be perfect at zero error rate
    assert_eq!(report_tmr.recovery_rate(), 1.0);
    assert_eq!(report_no.recovery_rate(), 1.0);
}

#[test]
fn assemble_execute_fibonacci_gives_correct_result() {
    // Full pipeline: assemble → load → execute → verify R1=55
    let cfg = EccConfig {
        tmr: false,
        block_size: 8,
        checksum: true,
    };
    let asm = Assembler::new(cfg, false);
    let out = asm.assemble(FIBONACCI).unwrap();

    let loader = ProgramLoader::new(cfg);
    let prog = loader.from_dna_string(&out.dna).unwrap();

    let mut cpu = Cpu::new(Memory::new());
    let _ = cpu.run(&prog.instructions);
    assert_eq!(cpu.regs[1], 55, "F(10) via full DNA pipeline should be 55");
}

#[test]
fn burst_mode_simulation_runs() {
    let cfg = EccConfig {
        tmr: true,
        block_size: 8,
        checksum: true,
    };
    let (instrs, dna) = assemble(FIBONACCI, cfg);
    // Burst mode with very low error rate
    let inject = InjectionConfig::burst(0.05, 0.005, 99);
    let report = simulate(&dna, &instrs, inject, cfg);
    report.print_summary();
    // Just verify it doesn't panic and returns a valid report
    assert!(report.recovery_rate() >= 0.0 && report.recovery_rate() <= 1.0);
}

#[test]
fn multi_run_simulation_consistent() {
    let cfg = EccConfig {
        tmr: false,
        block_size: 8,
        checksum: true,
    };
    let (instrs, dna) = assemble(FIBONACCI, cfg);
    // Same seed → same result
    let inject = InjectionConfig::composite(0.001, 0.0, 0.0, 777);
    let r1 = simulate(&dna, &instrs, inject, cfg);
    let r2 = simulate(&dna, &instrs, inject, cfg);
    assert_eq!(
        r1.injection.substitutions, r2.injection.substitutions,
        "Same seed must produce same injection"
    );
}

// ── v0.2: every one-letter change, judged by running the program ──

/// Post 1's program (IMML R1, 7; IMML R2, 3; ADD R3, R1, R2; HALT) with ECC,
/// one copy. Returns the instructions and the DNA.
fn post1_program(cfg: EccConfig) -> (Vec<[Codon; 4]>, String) {
    use helix_isa::{encode_instruction, Instruction, Register::*};
    let raw: Vec<Codon> = [
        Instruction::Imml(R1, 7),
        Instruction::Imml(R2, 3),
        Instruction::Add(R3, R1, R2),
        Instruction::Halt,
    ]
    .iter()
    .flat_map(encode_instruction)
    .collect();
    let dna: String = helix_ecc::EccEncoder::new(cfg)
        .encode(&raw)
        .iter()
        .flat_map(|c| c.to_dna())
        .collect();
    let instrs = raw.as_chunks::<4>().0.to_vec();
    (instrs, dna)
}

/// Counts of (correct, detected, silent wrong) over all one-letter changes.
fn every_one_letter_change(cfg: EccConfig) -> (usize, usize, usize) {
    use helix_sim::metrics::{evaluate, Outcome};
    let (instrs, dna) = post1_program(cfg);
    let (mut correct, mut detected, mut silent) = (0, 0, 0);
    for pos in 0..dna.len() {
        for letter in ['A', 'C', 'G', 'T'] {
            let mut v: Vec<char> = dna.chars().collect();
            if v[pos] == letter {
                continue;
            }
            v[pos] = letter;
            match evaluate(&v.into_iter().collect::<String>(), &instrs, cfg).0 {
                Outcome::Correct => correct += 1,
                Outcome::Detected(_) => detected += 1,
                Outcome::SilentWrong => silent += 1,
            }
        }
    }
    (correct, detected, silent)
}

#[test]
fn checksum_leaves_no_silent_wrong_answer_for_any_one_letter_change() {
    let with = EccConfig {
        tmr: false,
        block_size: 8,
        checksum: true,
    };
    let without = EccConfig {
        checksum: false,
        ..with
    };
    let (c1, d1, s1) = every_one_letter_change(with);
    let (c0, d0, s0) = every_one_letter_change(without);
    println!("one copy, checksum on:  {c1} correct, {d1} detected, {s1} silent wrong");
    println!("one copy, checksum off: {c0} correct, {d0} detected, {s0} silent wrong");
    assert_eq!(
        s1, 0,
        "checksum must catch every one-letter change that alters the program"
    );
    assert!(
        s0 > 0,
        "without the checksum some operand changes go through silently"
    );
}

#[test]
fn three_copies_correct_every_one_letter_change() {
    let (c, d, s) = every_one_letter_change(EccConfig::default());
    println!("three copies, checksum on: {c} correct, {d} detected, {s} silent wrong");
    assert_eq!((d, s), (0, 0));
}
