use helix_isa::{decode_instruction, encode_instruction, Instruction, Register::*};

fn rt(instr: Instruction) -> Instruction {
    decode_instruction(&encode_instruction(&instr)).unwrap()
}

#[test]
fn r3_arithmetic_roundtrip() {
    let cases = [
        Instruction::Add(R1, R2, R3),
        Instruction::Sub(R4, R5, R6),
        Instruction::Mul(R1, R1, R2),
        Instruction::Div(R3, R4, R5),
        Instruction::Mod(R6, R7, R1),
        Instruction::And(R2, R3, R4),
        Instruction::Or(R5, R6, R7),
        Instruction::Xor(R1, R2, R3),
        Instruction::Shl(R4, R5, R6),
        Instruction::Shr(R7, R1, R2),
    ];
    for i in cases {
        assert_eq!(rt(i), i, "roundtrip failed: {i}");
    }
}

#[test]
fn r2_instructions_roundtrip() {
    let cases = [
        Instruction::Inc(R1, R2),
        Instruction::Dec(R3, R4),
        Instruction::Not(R5, R6),
        Instruction::Load(R1, R2),
        Instruction::Store(R3, R4),
        Instruction::Cmp(R5, R6),
    ];
    for i in cases {
        assert_eq!(rt(i), i, "roundtrip failed: {i}");
    }
}

#[test]
fn r1_instructions_roundtrip() {
    for r in [R1, R2, R3, R4, R5, R6, R7] {
        assert_eq!(rt(Instruction::Push(r)), Instruction::Push(r));
        assert_eq!(rt(Instruction::Pop(r)), Instruction::Pop(r));
    }
}

#[test]
fn immediate_all_values_roundtrip() {
    for imm in [0u8, 1, 0x0F, 0x55, 0xAB, 0xFF] {
        assert_eq!(rt(Instruction::Imml(R3, imm)), Instruction::Imml(R3, imm));
        assert_eq!(rt(Instruction::Immh(R5, imm)), Instruction::Immh(R5, imm));
    }
}

#[test]
fn branch_offsets_roundtrip() {
    for off in [-128i8, -64, -1, 0, 1, 64, 127] {
        let cases = [
            Instruction::Jmp(off),
            Instruction::Beq(off),
            Instruction::Bne(off),
            Instruction::Blt(off),
            Instruction::Bge(off),
            Instruction::Call(off),
        ];
        for i in cases {
            assert_eq!(rt(i), i, "branch roundtrip failed: {i}");
        }
    }
}

#[test]
fn special_instructions_roundtrip() {
    for i in [
        Instruction::Nop,
        Instruction::Halt,
        Instruction::Ret,
        Instruction::Sync,
    ] {
        assert_eq!(rt(i), i);
    }
}

#[test]
fn all_register_combinations_add_roundtrip() {
    use helix_isa::Register::*;
    for d in [R0, R1, R2, R3, R4, R5, R6, R7] {
        for s in [R0, R1, R2, R3, R4, R5, R6, R7] {
            let i = Instruction::Add(d, s, R0);
            assert_eq!(rt(i), i);
        }
    }
}

#[test]
fn all_32_opcodes_produce_decodable_codons() {
    use helix_isa::Instruction::*;
    use helix_isa::Register::*;
    let instrs: [Instruction; 32] = [
        Add(R1, R2, R3),
        Sub(R1, R2, R3),
        Mul(R1, R2, R3),
        Div(R1, R2, R3),
        Mod(R1, R2, R3),
        Inc(R1, R2),
        Dec(R1, R2),
        And(R1, R2, R3),
        Or(R1, R2, R3),
        Xor(R1, R2, R3),
        Not(R1, R2),
        Shl(R1, R2, R3),
        Shr(R1, R2, R3),
        Cmp(R1, R2),
        Load(R1, R2),
        Store(R1, R2),
        Push(R1),
        Pop(R1),
        Imml(R1, 0x42),
        Immh(R1, 0x42),
        Jmp(1),
        Beq(1),
        Bne(1),
        Blt(1),
        Bge(1),
        Call(1),
        Ret,
        Nop,
        Halt,
        Sync,
        Eccp(0x1F, 0x2A),
        Trap(0x01),
    ];
    for i in instrs {
        decode_instruction(&encode_instruction(&i))
            .unwrap_or_else(|_| panic!("failed to decode: {i}"));
    }
}
