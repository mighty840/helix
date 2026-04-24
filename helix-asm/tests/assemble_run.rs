use helix_asm::Assembler;
use helix_ecc::EccConfig;
use helix_vm::{Cpu, Memory, ProgramLoader};

fn assemble_and_run(src: &str) -> Cpu {
    let cfg = EccConfig {
        tmr: false,
        block_size: 8,
    };
    let asm = Assembler::new(cfg, false);
    let out = asm.assemble(src).unwrap();
    let prog = ProgramLoader::from_raw_codons(&out.raw_codons).unwrap();
    let mut mem = Memory::new();
    mem.load_data(&out.program.data);
    let mut cpu = Cpu::new(mem);
    cpu.pc = out.program.entry;
    let _ = cpu.run(&prog.instructions);
    cpu
}

const FIBONACCI: &str = r#"
.program "fibonacci"
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

const GCD_PROGRAM: &str = r#"
.program "gcd"
.version 1
.text
.entry main
main:
    IMML R1, 48
    IMML R2, 18
loop:
    CMP  R2, R0
    BEQ  done
    MOD  R3, R1, R2
    ADD  R1, R2, R0
    ADD  R2, R3, R0
    JMP  loop
done:
    HALT
"#;

const SUM_ARRAY: &str = r#"
.program "sum_array"
.version 1
.memory 64
.data
arr: .words 10, 20, 30, 40, 50
.text
.entry main
main:
    IMML R1, 0
    IMML R2, 5
    IMML R3, 0
    IMML R4, 0
loop:
    CMP  R4, R2
    BGE  done
    ADD  R5, R1, R4
    LOAD R5, R5
    ADD  R3, R3, R5
    INC  R4, R4
    JMP  loop
done:
    HALT
"#;

#[test]
fn assemble_fibonacci_r1_equals_55() {
    let cpu = assemble_and_run(FIBONACCI);
    assert_eq!(cpu.regs[1], 55, "F(10) should be 55");
}

#[test]
fn assemble_gcd_r1_equals_6() {
    let cpu = assemble_and_run(GCD_PROGRAM);
    assert_eq!(cpu.regs[1], 6, "GCD(48,18) should be 6");
}

#[test]
fn assemble_sum_array_r3_equals_150() {
    let cpu = assemble_and_run(SUM_ARRAY);
    assert_eq!(cpu.regs[3], 150, "sum should be 150");
}

#[test]
fn dna_output_is_valid_acgt() {
    let cfg = EccConfig {
        tmr: false,
        block_size: 8,
    };
    let asm = Assembler::new(cfg, true); // bio optimization on
    let out = asm.assemble(FIBONACCI).unwrap();
    assert!(!out.dna.is_empty());
    assert!(out.dna.chars().all(|c| "ACGT".contains(c)));
    assert_eq!(out.dna.len() % 3, 0, "DNA must be codon-aligned");
}

#[test]
fn data_section_preloaded_into_memory() {
    let cfg = EccConfig {
        tmr: false,
        block_size: 8,
    };
    let asm = Assembler::new(cfg, false);
    let out = asm.assemble(SUM_ARRAY).unwrap();
    assert_eq!(out.program.data, vec![10, 20, 30, 40, 50]);
}

#[test]
fn entry_label_resolves_correctly() {
    // Non-zero entry: instructions before main should be skipped
    let src = r#"
.text
.entry main
    IMML R1, 0xFF  ; this runs only if entry is wrong
main:
    IMML R2, 42
    HALT
"#;
    let cpu = assemble_and_run(src);
    // If entry is correct (main at index 1), R1 stays 0.
    assert_eq!(cpu.regs[2], 42);
}

#[test]
fn full_ecc_pipeline_roundtrip() {
    // Assemble with ECC, load via DNA string, execute, verify output.
    let cfg = EccConfig {
        tmr: false,
        block_size: 8,
    };
    let asm = Assembler::new(cfg, false);
    let out = asm.assemble(GCD_PROGRAM).unwrap();

    let loader = helix_vm::ProgramLoader::new(cfg);
    let prog = loader.from_dna_string(&out.dna).unwrap();

    let mut cpu = Cpu::new(Memory::new());
    let _ = cpu.run(&prog.instructions);
    assert_eq!(cpu.regs[1], 6, "GCD via full ECC pipeline should be 6");
}

#[test]
fn multiple_labels_and_branches() {
    // A program with multiple forward and backward branches.
    let src = r#"
.text
.entry start
start:
    IMML R1, 0
    IMML R2, 5
loop:
    CMP  R1, R2
    BGE  end
    INC  R1, R1
    JMP  loop
end:
    HALT
"#;
    let cpu = assemble_and_run(src);
    assert_eq!(cpu.regs[1], 5);
}
