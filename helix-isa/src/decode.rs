use crate::codon::Codon;
use crate::instruction::{Instruction, Opcode, Register};
use crate::table::CODON_TABLE;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("unknown codon value {0:#04x} in opcode position")]
    UnknownOpcode(u8),
    #[error("invalid register index {0}")]
    InvalidRegister(u8),
}

/// Decode 4 codons into an Instruction.
pub fn decode_instruction(codons: &[Codon; 4]) -> Result<Instruction, DecodeError> {
    let opcode_val = CODON_TABLE[codons[0].value() as usize]
        .ok_or(DecodeError::UnknownOpcode(codons[0].value()))?;

    let op = Opcode::from_u8(opcode_val).ok_or(DecodeError::UnknownOpcode(opcode_val))?;

    let f2 = codons[1].value();
    let f3 = codons[2].value();
    let f4 = codons[3].value();

    // Extract register from a field: top 3 bits of 6-bit codon.
    let reg = |f: u8| -> Result<Register, DecodeError> {
        Register::from_u8(f >> 3).ok_or(DecodeError::InvalidRegister(f >> 3))
    };

    // Reconstruct imm8 from fields 3 and 4.
    // encode: codon[2] = imm[7:4], codon[3] = (imm[3:0] << 2)
    let imm8 = || -> u8 { ((f3 & 0x0F) << 4) | ((f4 >> 2) & 0x0F) };

    match op {
        Opcode::Add => Ok(Instruction::Add(reg(f2)?, reg(f3)?, reg(f4)?)),
        Opcode::Sub => Ok(Instruction::Sub(reg(f2)?, reg(f3)?, reg(f4)?)),
        Opcode::Mul => Ok(Instruction::Mul(reg(f2)?, reg(f3)?, reg(f4)?)),
        Opcode::Div => Ok(Instruction::Div(reg(f2)?, reg(f3)?, reg(f4)?)),
        Opcode::Mod => Ok(Instruction::Mod(reg(f2)?, reg(f3)?, reg(f4)?)),
        Opcode::Inc => Ok(Instruction::Inc(reg(f2)?, reg(f3)?)),
        Opcode::Dec => Ok(Instruction::Dec(reg(f2)?, reg(f3)?)),
        Opcode::And => Ok(Instruction::And(reg(f2)?, reg(f3)?, reg(f4)?)),
        Opcode::Or => Ok(Instruction::Or(reg(f2)?, reg(f3)?, reg(f4)?)),
        Opcode::Xor => Ok(Instruction::Xor(reg(f2)?, reg(f3)?, reg(f4)?)),
        Opcode::Not => Ok(Instruction::Not(reg(f2)?, reg(f3)?)),
        Opcode::Shl => Ok(Instruction::Shl(reg(f2)?, reg(f3)?, reg(f4)?)),
        Opcode::Shr => Ok(Instruction::Shr(reg(f2)?, reg(f3)?, reg(f4)?)),
        Opcode::Cmp => Ok(Instruction::Cmp(reg(f2)?, reg(f3)?)),
        Opcode::Load => Ok(Instruction::Load(reg(f2)?, reg(f3)?)),
        Opcode::Store => Ok(Instruction::Store(reg(f2)?, reg(f3)?)),
        Opcode::Push => Ok(Instruction::Push(reg(f2)?)),
        Opcode::Pop => Ok(Instruction::Pop(reg(f2)?)),
        Opcode::Imml => Ok(Instruction::Imml(reg(f2)?, imm8())),
        Opcode::Immh => Ok(Instruction::Immh(reg(f2)?, imm8())),
        Opcode::Jmp => Ok(Instruction::Jmp(imm8() as i8)),
        Opcode::Beq => Ok(Instruction::Beq(imm8() as i8)),
        Opcode::Bne => Ok(Instruction::Bne(imm8() as i8)),
        Opcode::Blt => Ok(Instruction::Blt(imm8() as i8)),
        Opcode::Bge => Ok(Instruction::Bge(imm8() as i8)),
        Opcode::Call => Ok(Instruction::Call(imm8() as i8)),
        Opcode::Ret => Ok(Instruction::Ret),
        Opcode::Nop => Ok(Instruction::Nop),
        Opcode::Halt => Ok(Instruction::Halt),
        Opcode::Sync => Ok(Instruction::Sync),
        Opcode::Eccp => Ok(Instruction::Eccp(f3, f4)),
        Opcode::Trap => Ok(Instruction::Trap(f2)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::encode_instruction;

    #[test]
    fn unknown_opcode_error() {
        // Codon 48 (TAA) is a control codon, not a valid opcode.
        let codons = [Codon::new(48), Codon::new(0), Codon::new(0), Codon::new(0)];
        assert!(matches!(
            decode_instruction(&codons),
            Err(DecodeError::UnknownOpcode(48))
        ));
    }

    #[test]
    fn decode_all_no_panic() {
        // Encode every instruction variant and decode it back without panicking.
        let instrs = vec![
            Instruction::Add(Register::R1, Register::R2, Register::R3),
            Instruction::Sub(Register::R0, Register::R7, Register::R7),
            Instruction::Imml(Register::R4, 42),
            Instruction::Immh(Register::R4, 200),
            Instruction::Jmp(-3),
            Instruction::Call(10),
            Instruction::Halt,
            Instruction::Nop,
            Instruction::Sync,
            Instruction::Ret,
            Instruction::Trap(0x01),
            Instruction::Eccp(0x3A, 0x1B),
        ];
        for instr in instrs {
            let codons = encode_instruction(&instr);
            let decoded = decode_instruction(&codons).unwrap();
            assert_eq!(decoded, instr);
        }
    }
}
