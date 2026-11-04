use helix_ecc::{EccConfig, EccDecoder, EccEncoder};
use helix_isa::{encode_instruction, Codon, Instruction, Register::*};

fn sample_codons() -> Vec<Codon> {
    let instrs = [
        Instruction::Imml(R1, 5),
        Instruction::Imml(R2, 3),
        Instruction::Add(R3, R1, R2),
        Instruction::Halt,
        Instruction::Nop,
        Instruction::Nop,
        Instruction::Nop,
        Instruction::Nop,
    ];
    instrs.iter().flat_map(encode_instruction).collect()
}

#[test]
fn no_tmr_encode_decode_roundtrip() {
    let cfg = EccConfig {
        tmr: false,
        block_size: 8,
    };
    let orig = sample_codons();
    let encoded = EccEncoder::new(cfg).encode(&orig);
    let decoded = EccDecoder::new(cfg).decode(&encoded).unwrap();
    assert_eq!(&decoded[..orig.len()], orig.as_slice());
}

#[test]
fn tmr_encode_decode_roundtrip() {
    let cfg = EccConfig {
        tmr: true,
        block_size: 8,
    };
    let orig = sample_codons();
    let encoded = EccEncoder::new(cfg).encode(&orig);
    let decoded = EccDecoder::new(cfg).decode(&encoded).unwrap();
    assert_eq!(&decoded[..orig.len()], orig.as_slice());
}

#[test]
fn single_codon_flip_corrected_by_rs() {
    let cfg = EccConfig {
        tmr: false,
        block_size: 8,
    };
    let orig = sample_codons();
    let mut encoded = EccEncoder::new(cfg).encode(&orig);

    // Flip the opcode codon of the first instruction (RS protects opcode codons only).
    // SYNC occupies [0..3]; data starts at [4]. Instruction opcodes are at [4, 8, 12, ...].
    let flip = 4;
    let old = encoded[flip].value();
    encoded[flip] = Codon::new((old ^ 0x01) & 0x3F);

    let decoded = EccDecoder::new(cfg).decode(&encoded).unwrap();
    assert_eq!(
        &decoded[..orig.len()],
        orig.as_slice(),
        "RS did not correct single flip"
    );
}

#[test]
fn tmr_one_corrupt_copy_recovered() {
    let cfg = EccConfig {
        tmr: true,
        block_size: 8,
    };
    let orig = sample_codons();
    let mut encoded = EccEncoder::new(cfg).encode(&orig);

    // The three copies are stored back to back; the first is the first third.
    let copy_len = encoded.len() / 3;
    // Corrupt a few codons in the first copy.
    for codon in &mut encoded[4..8.min(copy_len)] {
        let v = codon.value();
        *codon = Codon::new((v ^ 0x0F) & 0x3F);
    }

    let decoded = EccDecoder::new(cfg).decode(&encoded).unwrap();
    assert_eq!(&decoded[..orig.len()], orig.as_slice());
}

#[test]
fn two_blocks_roundtrip() {
    let cfg = EccConfig {
        tmr: false,
        block_size: 8,
    };
    // 16 instructions = 2 RS blocks
    let codons: Vec<Codon> = (0u8..16)
        .flat_map(|i| encode_instruction(&Instruction::Imml(R1, i)))
        .collect();
    let encoded = EccEncoder::new(cfg).encode(&codons);
    let decoded = EccDecoder::new(cfg).decode(&encoded).unwrap();
    assert_eq!(&decoded[..codons.len()], codons.as_slice());
}

#[test]
fn encoded_stream_is_valid_acgt() {
    let cfg = EccConfig {
        tmr: false,
        block_size: 8,
    };
    let orig = sample_codons();
    let encoded = EccEncoder::new(cfg).encode(&orig);
    let dna: String = encoded.iter().flat_map(|c| c.to_dna()).collect();
    assert!(dna.chars().all(|c| "ACGT".contains(c)));
}

#[test]
fn tmr_body_containing_tga_roundtrips() {
    // v0.1.0 regression: codon value 56 (TGA) inside an instruction field
    // was taken for the copy separator.
    let cfg = EccConfig {
        tmr: true,
        block_size: 8,
    };
    let mut orig = sample_codons();
    orig[2] = Codon::new(0x38);
    orig[7] = Codon::new(0x38);
    let encoded = EccEncoder::new(cfg).encode(&orig);
    let decoded = EccDecoder::new(cfg).decode(&encoded).unwrap();
    assert_eq!(&decoded[..orig.len()], orig.as_slice());
}

#[test]
fn tmr_stream_with_lost_codon_is_an_error() {
    let cfg = EccConfig {
        tmr: true,
        block_size: 8,
    };
    let mut encoded = EccEncoder::new(cfg).encode(&sample_codons());
    encoded.remove(10);
    assert!(EccDecoder::new(cfg).decode(&encoded).is_err());
}
