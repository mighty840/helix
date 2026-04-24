/// Model weights → DNA storage demo.
///
/// Shows how neural network weights can be quantized to 6-bit values and stored
/// as a DNA nucleotide sequence using Helix codon encoding.
///
/// Run: cargo run --example weights --manifest-path helix-vm/Cargo.toml
use helix_isa::{Codon, DnaSequence};

fn main() {
    // A tiny 2-layer MLP: input(4) → hidden(4) → output(1)
    // 4×4 weight matrix (16 weights) + 4×1 weight vector (4 weights) + 1 bias
    let weights: Vec<f32> = vec![
        // Layer 1: 4×4 weight matrix
        0.42, -0.31, 0.17, -0.89, 0.55, 0.12, -0.44, 0.73, -0.23, 0.68, 0.11, -0.51, 0.37, -0.19,
        0.82, 0.04, // Layer 2: 4×1 vector
        -0.61, 0.48, -0.27, 0.93,
    ];

    println!("=== Helix Model Weights → DNA Demo ===");
    println!("Architecture: 4→4→1  ({} weights)", weights.len());

    // --- Quantize: f32 → 6-bit unsigned (0..=63) ---
    let min = weights.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = weights.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let scale = (max - min) / 63.0;

    let quantized: Vec<u8> = weights
        .iter()
        .map(|&w| ((w - min) / scale).round().clamp(0.0, 63.0) as u8)
        .collect();

    println!("\nQuantization: range [{min:.3}, {max:.3}] → [0, 63]");
    println!("Scale factor: {scale:.5} per step");
    println!("Quantized:    {:?}", quantized);

    // --- Encode: one weight → one codon → 3 nucleotides ---
    let mut seq = DnaSequence::new();
    for &q in &quantized {
        seq.push_codon(Codon::new(q));
    }
    let dna = seq.as_str();

    println!("\nDNA encoding:");
    println!(
        "  {} weights × 3 nt/codon = {} nucleotides  ({:.1} nt/weight)",
        weights.len(),
        dna.len(),
        dna.len() as f32 / weights.len() as f32
    );
    println!("  First 30 nt: {}", &dna[..30.min(dna.len())]);

    // --- Decode: DNA → quantized values → f32 ---
    let decoded_seq = DnaSequence::parse(&dna).expect("DNA parse failed");
    let recovered: Vec<f32> = decoded_seq
        .codons()
        .map(|c| min + c.value() as f32 * scale)
        .collect();

    // --- Accuracy ---
    let mse: f32 = weights
        .iter()
        .zip(recovered.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        / weights.len() as f32;

    let max_err: f32 = weights
        .iter()
        .zip(recovered.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);

    println!("\nRecovery accuracy:");
    println!("  MSE:      {mse:.7}");
    println!(
        "  Max err:  {max_err:.5}  (theoretical max: {:.5})",
        scale / 2.0
    );

    println!("\nSample: original → recovered");
    for (orig, rec) in weights.iter().zip(recovered.iter()).take(5) {
        println!("  {orig:+.4} → {rec:+.4}");
    }

    // --- Storage density comparison ---
    println!("\nStorage density:");
    println!("  f32 (32-bit):  {} bytes", weights.len() * 4);
    println!("  int8 (8-bit):  {} bytes", weights.len());
    println!(
        "  Helix 6-bit:   {} nucleotides ≈ {:.1} bytes (1.5 bits/nt × 3 nt/weight)",
        dna.len(),
        dna.len() as f32 * 2.0 / 8.0
    );
}
