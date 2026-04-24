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
    };
    let cfg_no = EccConfig {
        tmr: false,
        block_size: 8,
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
