use crate::codon::Codon;
use crate::instruction::{Instruction, Opcode, Register};
use crate::table::OPCODE_SYNONYMS;

/// Encode an instruction into exactly 4 codons (24 bits).
///
/// Layout:
///   codon[0] = opcode codon (primary)
///   codon[1] = field2: rd[2:0] | flags/mode[2:0]  (packed into 6 bits)
///   codon[2] = field3: rs1[2:0] | pad[2:0]  OR  imm_hi[5:0]
///   codon[3] = field4: rs2[2:0] | pad[2:0]  OR  imm_lo[5:0]
///
/// For immediate instructions the 8-bit immediate is packed as:
///   imm[7:4] in bits [3:0] of codon[2]
///   imm[3:0] in bits [5:2] of codon[3]
pub fn encode_instruction(instr: &Instruction) -> [Codon; 4] {
    match instr {
        // R-type: 3 registers
        Instruction::Add(d, s1, s2) => r3(Opcode::Add, *d, *s1, *s2),
        Instruction::Sub(d, s1, s2) => r3(Opcode::Sub, *d, *s1, *s2),
        Instruction::Mul(d, s1, s2) => r3(Opcode::Mul, *d, *s1, *s2),
        Instruction::Div(d, s1, s2) => r3(Opcode::Div, *d, *s1, *s2),
        Instruction::Mod(d, s1, s2) => r3(Opcode::Mod, *d, *s1, *s2),
        Instruction::And(d, s1, s2) => r3(Opcode::And, *d, *s1, *s2),
        Instruction::Or(d, s1, s2) => r3(Opcode::Or, *d, *s1, *s2),
        Instruction::Xor(d, s1, s2) => r3(Opcode::Xor, *d, *s1, *s2),
        Instruction::Shl(d, s1, s2) => r3(Opcode::Shl, *d, *s1, *s2),
        Instruction::Shr(d, s1, s2) => r3(Opcode::Shr, *d, *s1, *s2),
        // R-type: 2 registers
        Instruction::Inc(d, s) => r2(Opcode::Inc, *d, *s),
        Instruction::Dec(d, s) => r2(Opcode::Dec, *d, *s),
        Instruction::Not(d, s) => r2(Opcode::Not, *d, *s),
        Instruction::Load(d, s) => r2(Opcode::Load, *d, *s),
        Instruction::Store(a, s) => r2(Opcode::Store, *a, *s),
        Instruction::Cmp(s1, s2) => r2(Opcode::Cmp, *s1, *s2),
        // R-type: 1 register
        Instruction::Push(s) => r1(Opcode::Push, *s),
        Instruction::Pop(d) => r1(Opcode::Pop, *d),
        Instruction::Ret => r1(Opcode::Ret, Register::R0),
        // I-type: register + imm8
        Instruction::Imml(d, imm) => imm8(Opcode::Imml, *d, *imm),
        Instruction::Immh(d, imm) => imm8(Opcode::Immh, *d, *imm),
        // I-type: imm8 as signed (no register)
        Instruction::Jmp(off) => branch(Opcode::Jmp, *off),
        Instruction::Beq(off) => branch(Opcode::Beq, *off),
        Instruction::Bne(off) => branch(Opcode::Bne, *off),
        Instruction::Blt(off) => branch(Opcode::Blt, *off),
        Instruction::Bge(off) => branch(Opcode::Bge, *off),
        Instruction::Call(off) => branch(Opcode::Call, *off),
        // No operands
        Instruction::Nop => no_op(Opcode::Nop),
        Instruction::Halt => no_op(Opcode::Halt),
        Instruction::Sync => no_op(Opcode::Sync),
        // Special
        Instruction::Eccp(p0, p1) => parity(Opcode::Eccp, *p0, *p1),
        Instruction::Trap(code) => trap(Opcode::Trap, *code),
    }
}

fn opcode_codon(op: Opcode) -> Codon {
    Codon::new(OPCODE_SYNONYMS[op as usize].0)
}

fn r3(op: Opcode, rd: Register, rs1: Register, rs2: Register) -> [Codon; 4] {
    [
        opcode_codon(op),
        Codon::new((rd as u8) << 3),
        Codon::new((rs1 as u8) << 3),
        Codon::new((rs2 as u8) << 3),
    ]
}

fn r2(op: Opcode, r0: Register, r1: Register) -> [Codon; 4] {
    [
        opcode_codon(op),
        Codon::new((r0 as u8) << 3),
        Codon::new((r1 as u8) << 3),
        Codon::new(0),
    ]
}

fn r1(op: Opcode, r: Register) -> [Codon; 4] {
    [
        opcode_codon(op),
        Codon::new((r as u8) << 3),
        Codon::new(0),
        Codon::new(0),
    ]
}

fn imm8(op: Opcode, rd: Register, imm: u8) -> [Codon; 4] {
    // Pack imm[7:4] into codon[2] bits [3:0], imm[3:0] into codon[3] bits [5:2].
    [
        opcode_codon(op),
        Codon::new((rd as u8) << 3),
        Codon::new(imm >> 4), // upper 4 bits in low nibble of 6-bit codon
        Codon::new((imm & 0x0F) << 2), // lower 4 bits in upper nibble
    ]
}

fn branch(op: Opcode, offset: i8) -> [Codon; 4] {
    let raw = offset as u8;
    [
        opcode_codon(op),
        Codon::new(0),
        Codon::new(raw >> 4),
        Codon::new((raw & 0x0F) << 2),
    ]
}

fn no_op(op: Opcode) -> [Codon; 4] {
    [
        opcode_codon(op),
        Codon::new(0),
        Codon::new(0),
        Codon::new(0),
    ]
}

fn parity(op: Opcode, p0: u8, p1: u8) -> [Codon; 4] {
    [
        opcode_codon(op),
        Codon::new(0),
        Codon::new(p0 & 0x3F),
        Codon::new(p1 & 0x3F),
    ]
}

fn trap(op: Opcode, code: u8) -> [Codon; 4] {
    [
        opcode_codon(op),
        Codon::new(code & 0x3F),
        Codon::new(0),
        Codon::new(0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_instruction;

    fn roundtrip(instr: Instruction) {
        let codons = encode_instruction(&instr);
        let decoded = decode_instruction(&codons).expect("decode failed");
        assert_eq!(decoded, instr, "roundtrip failed for {instr}");
    }

    #[test]
    fn r3_roundtrip() {
        roundtrip(Instruction::Add(Register::R1, Register::R2, Register::R3));
        roundtrip(Instruction::Sub(Register::R7, Register::R0, Register::R4));
        roundtrip(Instruction::Mul(Register::R3, Register::R3, Register::R3));
    }

    #[test]
    fn r2_roundtrip() {
        roundtrip(Instruction::Load(Register::R1, Register::R2));
        roundtrip(Instruction::Cmp(Register::R5, Register::R6));
        roundtrip(Instruction::Inc(Register::R4, Register::R4));
    }

    #[test]
    fn imm_roundtrip() {
        roundtrip(Instruction::Imml(Register::R1, 0x00));
        roundtrip(Instruction::Imml(Register::R1, 0xFF));
        roundtrip(Instruction::Imml(Register::R3, 0xAB));
        roundtrip(Instruction::Immh(Register::R7, 0x55));
    }

    #[test]
    fn branch_roundtrip() {
        roundtrip(Instruction::Jmp(0));
        roundtrip(Instruction::Jmp(10));
        roundtrip(Instruction::Jmp(-5));
        roundtrip(Instruction::Beq(-128));
        roundtrip(Instruction::Bge(127));
    }

    #[test]
    fn special_roundtrip() {
        roundtrip(Instruction::Nop);
        roundtrip(Instruction::Halt);
        roundtrip(Instruction::Sync);
        roundtrip(Instruction::Ret);
        roundtrip(Instruction::Trap(0x03));
    }
}
