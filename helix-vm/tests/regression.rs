// Regression tests: capture known-correct behaviors and edge cases.
// Each test documents what it guards against.

use helix_isa::{encode_instruction, Instruction, Register::*};
use helix_vm::{Cpu, CpuError, Memory, TrapCode};

fn prog(instrs: &[Instruction]) -> Vec<[helix_isa::Codon; 4]> {
    instrs.iter().map(encode_instruction).collect()
}

// R0 is hardwired zero — writing to it must have no effect.
#[test]
fn reg_r0_hardwired_zero() {
    let p = prog(&[
        Instruction::Imml(R0, 0xFF),
        Instruction::Immh(R0, 0xFF),
        Instruction::Add(R0, R1, R2),
        Instruction::Halt,
    ]);
    let mut cpu = Cpu::new(Memory::new());
    cpu.run(&p).unwrap();
    assert_eq!(cpu.regs[0], 0, "R0 must remain 0 after write attempts");
}

// IMML sets only the low byte; IMMH sets only the high byte.
#[test]
fn imml_immh_independent_bytes() {
    let p = prog(&[
        Instruction::Imml(R1, 0xAB),
        Instruction::Immh(R1, 0xCD),
        Instruction::Halt,
    ]);
    let mut cpu = Cpu::new(Memory::new());
    cpu.run(&p).unwrap();
    assert_eq!(cpu.regs[1], 0xCDAB, "IMML/IMMH must set independent bytes");
}

// Arithmetic overflow wraps silently (u16 semantics).
#[test]
fn add_wraps_on_overflow() {
    let p = prog(&[
        Instruction::Immh(R1, 0xFF),
        Instruction::Imml(R1, 0xFF), // R1 = 0xFFFF
        Instruction::Imml(R2, 1),
        Instruction::Add(R3, R1, R2),
        Instruction::Halt,
    ]);
    let mut cpu = Cpu::new(Memory::new());
    cpu.run(&p).unwrap();
    assert_eq!(cpu.regs[3], 0, "0xFFFF + 1 must wrap to 0");
}

// SUB underflow wraps.
#[test]
fn sub_wraps_on_underflow() {
    let p = prog(&[
        Instruction::Imml(R1, 0),
        Instruction::Imml(R2, 1),
        Instruction::Sub(R3, R1, R2), // 0 - 1 = 0xFFFF
        Instruction::Halt,
    ]);
    let mut cpu = Cpu::new(Memory::new());
    cpu.run(&p).unwrap();
    assert_eq!(cpu.regs[3], 0xFFFF, "0 - 1 must wrap to 0xFFFF");
}

// CMP sets flags correctly: zero flag when operands are equal.
#[test]
fn cmp_sets_zero_flag() {
    let p = prog(&[
        Instruction::Imml(R1, 42),
        Instruction::Imml(R2, 42),
        Instruction::Cmp(R1, R2),
        Instruction::Beq(3), // if zero, skip IMML and hit HALT
        Instruction::Imml(R3, 0xFF),
        Instruction::Imml(R3, 0xFF),
        Instruction::Halt,
    ]);
    let mut cpu = Cpu::new(Memory::new());
    cpu.run(&p).unwrap();
    assert_eq!(cpu.regs[3], 0, "BEQ should have branched over IMML");
}

// CMP sets negative flag when a < b.
#[test]
fn cmp_sets_negative_flag() {
    let p = prog(&[
        Instruction::Imml(R1, 3),
        Instruction::Imml(R2, 10),
        Instruction::Cmp(R1, R2), // 3 - 10: result has high bit set
        Instruction::Blt(3),      // if negative, skip to HALT
        Instruction::Imml(R3, 0xFF),
        Instruction::Imml(R3, 0xFF),
        Instruction::Halt,
    ]);
    let mut cpu = Cpu::new(Memory::new());
    cpu.run(&p).unwrap();
    assert_eq!(cpu.regs[3], 0, "BLT should have branched over IMML");
}

// CALL pushes correct return address; RET restores it.
#[test]
fn call_ret_return_address_is_next_instruction() {
    // [0] IMML R1, 1    ; marker
    // [1] CALL 2        ; push 2, jump to [2+2-1]=3
    // [2] HALT          ; ← return address
    // [3] IMML R2, 99   ; subroutine
    // [4] RET           ; pop 2, jump to [2]=HALT
    let p = prog(&[
        Instruction::Imml(R1, 1),
        Instruction::Call(2),
        Instruction::Halt,
        Instruction::Imml(R2, 99),
        Instruction::Ret,
    ]);
    let mut cpu = Cpu::new(Memory::new());
    cpu.run(&p).unwrap();
    assert_eq!(cpu.regs[2], 99);
}

// Nested CALL/RET.
#[test]
fn nested_call_ret() {
    // main: CALL f1; HALT
    // f1:   CALL f2; RET
    // f2:   IMML R1, 7; RET
    //
    // [0] CALL 2  → jump to [0+1+2-1]=2
    // [1] HALT
    // [2] CALL 2  → jump to [2+1+2-1]=4
    // [3] RET
    // [4] IMML R1, 7
    // [5] RET
    let p = prog(&[
        Instruction::Call(2),     // 0: → 2
        Instruction::Halt,        // 1
        Instruction::Call(2),     // 2: → 4
        Instruction::Ret,         // 3: return to 1+?  wait...
        Instruction::Imml(R1, 7), // 4
        Instruction::Ret,         // 5
    ]);
    let mut cpu = Cpu::new(Memory::new());
    cpu.run(&p).unwrap();
    assert_eq!(cpu.regs[1], 7);
}

// DIV by zero raises trap.
#[test]
fn div_zero_raises_trap() {
    let p = prog(&[
        Instruction::Imml(R1, 10),
        Instruction::Imml(R2, 0),
        Instruction::Div(R3, R1, R2),
        Instruction::Halt,
    ]);
    let mut cpu = Cpu::new(Memory::new());
    let err = cpu.run(&p).unwrap_err();
    assert!(matches!(err, CpuError::Trap(TrapCode::DivZero)));
}

// MOD by zero raises trap.
#[test]
fn mod_zero_raises_trap() {
    let p = prog(&[
        Instruction::Imml(R1, 5),
        Instruction::Imml(R2, 0),
        Instruction::Mod(R3, R1, R2),
        Instruction::Halt,
    ]);
    let mut cpu = Cpu::new(Memory::new());
    assert!(matches!(
        cpu.run(&p).unwrap_err(),
        CpuError::Trap(TrapCode::DivZero)
    ));
}

// JMP with offset 1 is a no-op (jumps to the very next instruction).
#[test]
fn jmp_offset_1_advances_normally() {
    // pc_after_inc = 1, off=1 → new_pc = 1+1-1 = 1 = next instr
    let p = prog(&[
        Instruction::Jmp(1),
        Instruction::Imml(R1, 42),
        Instruction::Halt,
    ]);
    let mut cpu = Cpu::new(Memory::new());
    cpu.run(&p).unwrap();
    assert_eq!(cpu.regs[1], 42);
}

// Bitwise NOT: ~0 = 0xFFFF.
#[test]
fn not_of_zero_is_all_ones() {
    let p = prog(&[Instruction::Not(R1, R0), Instruction::Halt]);
    let mut cpu = Cpu::new(Memory::new());
    cpu.run(&p).unwrap();
    assert_eq!(cpu.regs[1], 0xFFFF);
}

// SHL and SHR.
#[test]
fn shift_operations() {
    let p = prog(&[
        Instruction::Imml(R1, 1),
        Instruction::Imml(R2, 4),     // shift amount
        Instruction::Shl(R3, R1, R2), // 1 << 4 = 16
        Instruction::Shr(R4, R3, R2), // 16 >> 4 = 1
        Instruction::Halt,
    ]);
    let mut cpu = Cpu::new(Memory::new());
    cpu.run(&p).unwrap();
    assert_eq!(cpu.regs[3], 16);
    assert_eq!(cpu.regs[4], 1);
}
