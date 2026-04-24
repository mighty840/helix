use helix_isa::{encode_instruction, Instruction, Register::*};
use helix_vm::{Cpu, Memory};

fn run(instrs: &[Instruction]) -> Cpu {
    let prog: Vec<_> = instrs.iter().map(encode_instruction).collect();
    let mut cpu = Cpu::new(Memory::new());
    let _ = cpu.run(&prog);
    cpu
}

fn run_with_data(instrs: &[Instruction], data: &[(u16, u16)]) -> Cpu {
    let prog: Vec<_> = instrs.iter().map(encode_instruction).collect();
    let mut mem = Memory::new();
    for &(addr, val) in data {
        mem.store(addr, val).unwrap();
    }
    let mut cpu = Cpu::new(mem);
    let _ = cpu.run(&prog);
    cpu
}

// --- Fibonacci ---
// F(10) = 55 using iterative: a=0,b=1, repeat N times (a,b)=(b,a+b)
//
// [0]  IMML R1, 0       a = F(0)
// [1]  IMML R2, 1       b = F(1)
// [2]  IMML R3, 0       counter
// [3]  IMML R4, 10      N
// [4]  CMP  R3, R4      ← loop
// [5]  BGE  6           → pc=6+6-1=11 (HALT) when counter>=N
// [6]  ADD  R5, R1, R2  temp=a+b
// [7]  ADD  R1, R2, R0  a=b
// [8]  ADD  R2, R5, R0  b=temp
// [9]  INC  R3, R3
// [10] JMP  -6          → pc=11-6-1=4 (CMP)
// [11] HALT
#[test]
fn fibonacci_10_equals_55() {
    let prog = [
        Instruction::Imml(R1, 0),
        Instruction::Imml(R2, 1),
        Instruction::Imml(R3, 0),
        Instruction::Imml(R4, 10),
        Instruction::Cmp(R3, R4),
        Instruction::Bge(6),
        Instruction::Add(R5, R1, R2),
        Instruction::Add(R1, R2, R0),
        Instruction::Add(R2, R5, R0),
        Instruction::Inc(R3, R3),
        Instruction::Jmp(-6),
        Instruction::Halt,
    ];
    assert_eq!(run(&prog).regs[1], 55);
}

// --- GCD ---
// GCD(48, 18) = 6 via Euclidean algorithm
//
// [0]  IMML R1, 48
// [1]  IMML R2, 18
// [2]  CMP  R2, R0     ← loop
// [3]  BEQ  5          → pc=4+5-1=8 (HALT)
// [4]  MOD  R3, R1, R2
// [5]  ADD  R1, R2, R0  a=b
// [6]  ADD  R2, R3, R0  b=rem
// [7]  JMP  -5          → pc=8-5-1=2 (CMP)
// [8]  HALT
#[test]
fn gcd_48_18_equals_6() {
    let prog = [
        Instruction::Imml(R1, 48),
        Instruction::Imml(R2, 18),
        Instruction::Cmp(R2, R0),
        Instruction::Beq(5),
        Instruction::Mod(R3, R1, R2),
        Instruction::Add(R1, R2, R0),
        Instruction::Add(R2, R3, R0),
        Instruction::Jmp(-5),
        Instruction::Halt,
    ];
    assert_eq!(run(&prog).regs[1], 6);
}

// --- Sum 1..=10 ---
// = 55
//
// [0]  IMML R1, 0      sum=0
// [1]  IMML R2, 1      i=1
// [2]  IMML R3, 10     limit
// [3]  CMP  R3, R2     ← loop; limit-i
// [4]  BLT  4          → pc=5+4-1=8 (HALT) when limit<i
// [5]  ADD  R1, R1, R2  sum+=i
// [6]  INC  R2, R2
// [7]  JMP  -4          → pc=8-4-1=3 (CMP)
// [8]  HALT
#[test]
fn sum_1_to_10_equals_55() {
    let prog = [
        Instruction::Imml(R1, 0),
        Instruction::Imml(R2, 1),
        Instruction::Imml(R3, 10),
        Instruction::Cmp(R3, R2),
        Instruction::Blt(4),
        Instruction::Add(R1, R1, R2),
        Instruction::Inc(R2, R2),
        Instruction::Jmp(-4),
        Instruction::Halt,
    ];
    assert_eq!(run(&prog).regs[1], 55);
}

// --- Factorial ---
// 5! = 120
//
// [0]  IMML R1, 5      n=5
// [1]  IMML R2, 1      result=1
// [2]  CMP  R1, R0     ← loop; n-0
// [3]  BEQ  4          → pc=4+4-1=7 (HALT)
// [4]  MUL  R2, R2, R1  result*=n
// [5]  DEC  R1, R1
// [6]  JMP  -4          → pc=7-4-1=2 (CMP)
// [7]  HALT
#[test]
fn factorial_5_equals_120() {
    let prog = [
        Instruction::Imml(R1, 5),
        Instruction::Imml(R2, 1),
        Instruction::Cmp(R1, R0),
        Instruction::Beq(4),
        Instruction::Mul(R2, R2, R1),
        Instruction::Dec(R1, R1),
        Instruction::Jmp(-4),
        Instruction::Halt,
    ];
    assert_eq!(run(&prog).regs[2], 120);
}

// --- Array sum ---
// arr=[10,20,30,40,50] → sum=150
//
// [0]  IMML R1, 0      base=0
// [1]  IMML R2, 5      N=5
// [2]  IMML R3, 0      sum=0
// [3]  IMML R4, 0      i=0
// [4]  CMP  R4, R2     ← loop
// [5]  BGE  6          → pc=6+6-1=11 (HALT)
// [6]  ADD  R5, R1, R4  addr=base+i
// [7]  LOAD R5, R5
// [8]  ADD  R3, R3, R5  sum+=val
// [9]  INC  R4, R4
// [10] JMP  -6          → pc=11-6-1=4 (CMP)
// [11] HALT
#[test]
fn array_sum_equals_150() {
    let prog = [
        Instruction::Imml(R1, 0),
        Instruction::Imml(R2, 5),
        Instruction::Imml(R3, 0),
        Instruction::Imml(R4, 0),
        Instruction::Cmp(R4, R2),
        Instruction::Bge(6),
        Instruction::Add(R5, R1, R4),
        Instruction::Load(R5, R5),
        Instruction::Add(R3, R3, R5),
        Instruction::Inc(R4, R4),
        Instruction::Jmp(-6),
        Instruction::Halt,
    ];
    let cpu = run_with_data(&prog, &[(0, 10), (1, 20), (2, 30), (3, 40), (4, 50)]);
    assert_eq!(cpu.regs[3], 150);
}

// --- Power ---
// 2^8 = 256
//
// [0]  IMML R1, 2      base=2
// [1]  IMML R2, 1      result=1
// [2]  IMML R3, 8      exp=8
// [3]  CMP  R3, R0     ← loop
// [4]  BEQ  4          → pc=5+4-1=8 (HALT)
// [5]  MUL  R2, R2, R1  result*=base
// [6]  DEC  R3, R3
// [7]  JMP  -4          → pc=8-4-1=3 (CMP)
// [8]  HALT
#[test]
fn power_2_to_8_equals_256() {
    let prog = [
        Instruction::Imml(R1, 2),
        Instruction::Imml(R2, 1),
        Instruction::Imml(R3, 8),
        Instruction::Cmp(R3, R0),
        Instruction::Beq(4),
        Instruction::Mul(R2, R2, R1),
        Instruction::Dec(R3, R3),
        Instruction::Jmp(-4),
        Instruction::Halt,
    ];
    assert_eq!(run(&prog).regs[2], 256);
}
