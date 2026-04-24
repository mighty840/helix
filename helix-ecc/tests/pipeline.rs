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

    // Locate the first SECTION_SEP (0x38 = TGA).
    let sep_pos = encoded.iter().position(|c| c.value() == 0x38).unwrap();
    // Corrupt a few codons in the first copy.
    for codon in &mut encoded[4..8.min(sep_pos)] {
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
