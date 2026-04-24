use thiserror::Error;

/// 8 general-purpose registers. R0 is hardwired zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Register {
    R0 = 0,
    R1 = 1,
    R2 = 2,
    R3 = 3,
    R4 = 4,
    R5 = 5,
    R6 = 6,
    R7 = 7,
}

impl Register {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v & 0x7 {
            0 => Some(Register::R0),
            1 => Some(Register::R1),
            2 => Some(Register::R2),
            3 => Some(Register::R3),
            4 => Some(Register::R4),
            5 => Some(Register::R5),
            6 => Some(Register::R6),
            7 => Some(Register::R7),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

impl std::fmt::Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "R{}", *self as u8)
    }
}

/// All opcodes. 5-bit opcode space (0x00–0x1F = 32 values).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    // Arithmetic
    Add = 0x00,
    Sub = 0x01,
    Mul = 0x02,
    Div = 0x03,
    Mod = 0x04,
    Inc = 0x05,
    Dec = 0x06,
    // Bitwise
    And = 0x07,
    Or = 0x08,
    Xor = 0x09,
    Not = 0x0A,
    Shl = 0x0B,
    Shr = 0x0C,
    // Comparison
    Cmp = 0x0D,
    // Memory
    Load = 0x0E,
    Store = 0x0F,
    Push = 0x10,
    Pop = 0x11,
    // Immediate
    Imml = 0x12,
    Immh = 0x13,
    // Control flow
    Jmp = 0x14,
    Beq = 0x15,
    Bne = 0x16,
    Blt = 0x17,
    Bge = 0x18,
    Call = 0x19,
    Ret = 0x1A,
    // Special
    Nop = 0x1B,
    Halt = 0x1C,
    Sync = 0x1D,
    Eccp = 0x1E,
    Trap = 0x1F,
}

impl Opcode {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v & 0x1F {
            0x00 => Some(Opcode::Add),
            0x01 => Some(Opcode::Sub),
            0x02 => Some(Opcode::Mul),
            0x03 => Some(Opcode::Div),
            0x04 => Some(Opcode::Mod),
            0x05 => Some(Opcode::Inc),
            0x06 => Some(Opcode::Dec),
            0x07 => Some(Opcode::And),
            0x08 => Some(Opcode::Or),
            0x09 => Some(Opcode::Xor),
            0x0A => Some(Opcode::Not),
            0x0B => Some(Opcode::Shl),
            0x0C => Some(Opcode::Shr),
            0x0D => Some(Opcode::Cmp),
            0x0E => Some(Opcode::Load),
            0x0F => Some(Opcode::Store),
            0x10 => Some(Opcode::Push),
            0x11 => Some(Opcode::Pop),
            0x12 => Some(Opcode::Imml),
            0x13 => Some(Opcode::Immh),
            0x14 => Some(Opcode::Jmp),
            0x15 => Some(Opcode::Beq),
            0x16 => Some(Opcode::Bne),
            0x17 => Some(Opcode::Blt),
            0x18 => Some(Opcode::Bge),
            0x19 => Some(Opcode::Call),
            0x1A => Some(Opcode::Ret),
            0x1B => Some(Opcode::Nop),
            0x1C => Some(Opcode::Halt),
            0x1D => Some(Opcode::Sync),
            0x1E => Some(Opcode::Eccp),
            0x1F => Some(Opcode::Trap),
            _ => None,
        }
    }
}

/// A decoded Helix instruction (24-bit, 4 codons).
///
/// Binary layout:
///   [opcode:6][field2:6][field3:6][field4:6]
///
/// R-type: [op][rd|flags][rs1|pad][rs2|pad]
/// I-type: [op][rd|flags][imm_hi:6][imm_lo:6]  — imm8 = low 8 bits of fields 3+4
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // R-type arithmetic / bitwise
    Add(Register, Register, Register),
    Sub(Register, Register, Register),
    Mul(Register, Register, Register),
    Div(Register, Register, Register),
    Mod(Register, Register, Register),
    Inc(Register, Register),
    Dec(Register, Register),
    And(Register, Register, Register),
    Or(Register, Register, Register),
    Xor(Register, Register, Register),
    Not(Register, Register),
    Shl(Register, Register, Register),
    Shr(Register, Register, Register),
    // Comparison
    Cmp(Register, Register),
    // Memory
    Load(Register, Register),
    Store(Register, Register),
    Push(Register),
    Pop(Register),
    // Immediate (rd, imm8)
    Imml(Register, u8),
    Immh(Register, u8),
    // Control (imm8 = signed offset in instruction units)
    Jmp(i8),
    Beq(i8),
    Bne(i8),
    Blt(i8),
    Bge(i8),
    Call(i8),
    Ret,
    // Special
    Nop,
    Halt,
    Sync,
    Eccp(u8, u8), // two parity bytes embedded in fields 3+4
    Trap(u8),     // trap code in field 2 low bits
}

#[derive(Debug, Error)]
pub enum InstructionError {
    #[error("unknown opcode: 0x{0:02X}")]
    UnknownOpcode(u8),
    #[error("invalid register index: {0}")]
    InvalidRegister(u8),
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Add(d, s1, s2) => write!(f, "ADD {d}, {s1}, {s2}"),
            Instruction::Sub(d, s1, s2) => write!(f, "SUB {d}, {s1}, {s2}"),
            Instruction::Mul(d, s1, s2) => write!(f, "MUL {d}, {s1}, {s2}"),
            Instruction::Div(d, s1, s2) => write!(f, "DIV {d}, {s1}, {s2}"),
            Instruction::Mod(d, s1, s2) => write!(f, "MOD {d}, {s1}, {s2}"),
            Instruction::Inc(d, s) => write!(f, "INC {d}, {s}"),
            Instruction::Dec(d, s) => write!(f, "DEC {d}, {s}"),
            Instruction::And(d, s1, s2) => write!(f, "AND {d}, {s1}, {s2}"),
            Instruction::Or(d, s1, s2) => write!(f, "OR  {d}, {s1}, {s2}"),
            Instruction::Xor(d, s1, s2) => write!(f, "XOR {d}, {s1}, {s2}"),
            Instruction::Not(d, s) => write!(f, "NOT {d}, {s}"),
            Instruction::Shl(d, s1, s2) => write!(f, "SHL {d}, {s1}, {s2}"),
            Instruction::Shr(d, s1, s2) => write!(f, "SHR {d}, {s1}, {s2}"),
            Instruction::Cmp(s1, s2) => write!(f, "CMP {s1}, {s2}"),
            Instruction::Load(d, s) => write!(f, "LOAD {d}, {s}"),
            Instruction::Store(a, s) => write!(f, "STORE {a}, {s}"),
            Instruction::Push(s) => write!(f, "PUSH {s}"),
            Instruction::Pop(d) => write!(f, "POP {d}"),
            Instruction::Imml(d, imm) => write!(f, "IMML {d}, {imm}"),
            Instruction::Immh(d, imm) => write!(f, "IMMH {d}, {imm}"),
            Instruction::Jmp(off) => write!(f, "JMP {off}"),
            Instruction::Beq(off) => write!(f, "BEQ {off}"),
            Instruction::Bne(off) => write!(f, "BNE {off}"),
            Instruction::Blt(off) => write!(f, "BLT {off}"),
            Instruction::Bge(off) => write!(f, "BGE {off}"),
            Instruction::Call(off) => write!(f, "CALL {off}"),
            Instruction::Ret => write!(f, "RET"),
            Instruction::Nop => write!(f, "NOP"),
            Instruction::Halt => write!(f, "HALT"),
            Instruction::Sync => write!(f, "SYNC"),
            Instruction::Eccp(p0, p1) => write!(f, "ECCP {p0:#04x},{p1:#04x}"),
            Instruction::Trap(code) => write!(f, "TRAP {code:#04x}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_roundtrip() {
        for i in 0u8..8 {
            let r = Register::from_u8(i).unwrap();
            assert_eq!(r as u8, i);
        }
    }

    #[test]
    fn opcode_roundtrip() {
        for i in 0u8..32 {
            let op = Opcode::from_u8(i).unwrap();
            assert_eq!(op as u8, i);
        }
    }

    #[test]
    fn instruction_display() {
        let i = Instruction::Add(Register::R1, Register::R2, Register::R3);
        assert_eq!(format!("{i}"), "ADD R1, R2, R3");
    }
}
