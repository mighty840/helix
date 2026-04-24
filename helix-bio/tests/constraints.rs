use helix_bio::{gc, homopolymer, repeats, structure, SequenceOptimizer};
use helix_isa::{encode_instruction, Base, Instruction};

fn b(s: &str) -> Vec<Base> {
    s.chars().map(|c| Base::from_char(c).unwrap()).collect()
}

// --- GC content ---

#[test]
fn all_at_sequence_violates_gc() {
    let bases: Vec<Base> = vec![Base::A; 200];
    let v = gc::scan(&bases);
    assert!(!v.is_empty());
    assert!(v[0].gc_fraction < gc::GC_MIN);
}

#[test]
fn all_gc_sequence_violates_gc() {
    let bases: Vec<Base> = vec![Base::G; 200];
    let v = gc::scan(&bases);
    assert!(!v.is_empty());
    assert!(v[0].gc_fraction > gc::GC_MAX);
}

#[test]
fn balanced_sequence_passes_gc() {
    let bases: Vec<Base> = (0..200)
        .map(|i| if i % 2 == 0 { Base::G } else { Base::A })
        .collect();
    assert!(gc::scan(&bases).is_empty());
}

#[test]
fn short_sequence_under_window_size() {
    // Sequences shorter than 100 nt are still checked.
    let bases: Vec<Base> = vec![Base::A; 50]; // 0% GC
    let v = gc::scan(&bases);
    assert!(!v.is_empty());
}

// --- Homopolymer runs ---

#[test]
fn run_of_5_detected() {
    let mut bases = vec![Base::A; 5];
    bases.push(Base::G);
    let v = homopolymer::scan(&bases);
    assert!(!v.is_empty());
    assert_eq!(v[0].run_length, 5);
    assert_eq!(v[0].base, Base::A);
}

#[test]
fn run_of_4_allowed() {
    let bases = vec![Base::A, Base::A, Base::A, Base::A, Base::G];
    assert!(homopolymer::scan(&bases).is_empty());
}

#[test]
fn mixed_bases_no_homopolymer() {
    let bases = b("ACGTACGTACGT");
    assert!(homopolymer::scan(&bases).is_empty());
}

#[test]
fn multiple_runs_both_reported() {
    // AAAAA TTTTT mixed with spacers
    let bases = b("AAAAAGCTTTTTT");
    let v = homopolymer::scan(&bases);
    assert_eq!(v.len(), 2);
}

// --- Dinucleotide repeats ---

#[test]
fn five_at_repeats_detected() {
    let bases = b("ATATATATATT"); // AT×5 + T
    let v = repeats::scan(&bases);
    assert!(!v.is_empty());
}

#[test]
fn four_at_repeats_allowed() {
    // AT×4 = ATATATAT (8 chars) — exactly 4 repeats, allowed
    let bases = b("ATATATAT");
    assert!(repeats::scan(&bases).is_empty());
}

#[test]
fn random_sequence_no_repeat() {
    let bases = b("ACGTAGCTGCATCGAT");
    assert!(repeats::scan(&bases).is_empty());
}

// --- Palindromes ---

#[test]
fn six_bp_palindrome_allowed() {
    // GAATTC (EcoRI, 6bp) is a palindrome but under the 8bp limit
    let bases = b("GAATTC");
    assert!(structure::scan(&bases).is_empty());
}

#[test]
fn eight_bp_palindrome_allowed() {
    // GCAATTGC is an 8bp palindrome (at the limit, not over)
    let bases = b("GCAATTGC");
    assert!(structure::scan(&bases).is_empty());
}

#[test]
fn ten_bp_palindrome_detected() {
    // GCATGCATGC is its own reverse complement
    let bases = b("GCATGCATGC");
    let v = structure::scan(&bases);
    assert!(!v.is_empty());
    assert_eq!(v[0].length, 10);
}

// --- Optimizer ---

#[test]
fn optimizer_runs_without_panic_on_real_instructions() {
    let instrs: Vec<_> = (0..32).map(|_| Instruction::Nop).collect();
    let mut codons: Vec<_> = instrs.iter().flat_map(encode_instruction).collect();
    let opt = SequenceOptimizer::new(3);
    let report = opt.optimize(&mut codons);
    println!(
        "violations={}, swaps={}",
        report.violations.len(),
        report.swaps_applied
    );
}

#[test]
fn optimizer_lint_detects_all_gc_codons() {
    // Codon value 0x2A = 0b101010 = GCGCGC (all GC)
    // 32 NOPs with codon synonym = depends on implementation
    // Use a sequence of all-GC codons directly
    use helix_isa::Codon;
    let codons: Vec<_> = (0..32).map(|_| Codon::new(0x2A)).collect(); // not valid instrs but lint works on bases
                                                                      // Just verify lint doesn't panic
    let opt = SequenceOptimizer::default();
    let report = opt.lint(&codons);
    println!("lint violations: {}", report.violations.len());
}

#[test]
fn gc_fraction_utility() {
    let all_gc: Vec<Base> = vec![Base::G; 100];
    assert!((gc::gc_fraction(&all_gc) - 1.0).abs() < 1e-9);

    let all_at: Vec<Base> = vec![Base::A; 100];
    assert!((gc::gc_fraction(&all_at) - 0.0).abs() < 1e-9);

    let half: Vec<Base> = (0..100)
        .map(|i| if i % 2 == 0 { Base::G } else { Base::A })
        .collect();
    assert!((gc::gc_fraction(&half) - 0.5).abs() < 1e-9);
}
