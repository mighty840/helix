use helix_isa::{Instruction, Register};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("line {line}: {msg}")]
    Syntax { line: usize, msg: String },
}

impl ParseError {
    fn at(line: usize, msg: impl Into<String>) -> Self {
        Self::Syntax {
            line,
            msg: msg.into(),
        }
    }
}

/// A single parsed line item.
#[derive(Debug, Clone)]
pub enum Item {
    Label(String),
    Instruction(AsmInstruction),
    DataWords(Vec<u16>),
    Directive(Directive),
}

#[derive(Debug, Clone)]
pub enum Directive {
    Program(String),
    Version(u8),
    Memory(u16),
    Entry(String),
    Data,
    Text,
}

/// An instruction that may contain unresolved label references.
#[derive(Debug, Clone)]
pub enum AsmInstruction {
    /// Fully resolved to an ISA instruction.
    Resolved(Instruction),
    /// Branch/call with a label target (resolved later).
    BranchLabel { mnemonic: String, target: String },
}

/// The result of parsing a .hx source file.
#[derive(Debug, Default)]
pub struct SourceProgram {
    pub items: Vec<(usize, Item)>, // (line number, item)
}

pub fn parse(src: &str) -> Result<SourceProgram, ParseError> {
    let mut prog = SourceProgram::default();

    for (line_idx, raw) in src.lines().enumerate() {
        let line_num = line_idx + 1;
        // Strip comments.
        let line = if let Some(pos) = raw.find(';') {
            &raw[..pos]
        } else {
            raw
        }
        .trim();
        if line.is_empty() {
            continue;
        }

        // Label definition: "name:" alone on the line, or "name: rest"
        // Peel any leading "identifier:" prefix.
        let line = if let Some(colon_pos) = line.find(':') {
            let candidate = &line[..colon_pos];
            // Valid label: word characters only, no spaces
            if !candidate.is_empty() && candidate.chars().all(|c| c.is_alphanumeric() || c == '_') {
                let label = candidate.to_string();
                prog.items.push((line_num, Item::Label(label)));
                let rest = line[colon_pos + 1..].trim();
                if rest.is_empty() {
                    continue; // label-only line
                }
                rest // continue parsing the rest of the line
            } else {
                line // not a label, parse as-is
            }
        } else {
            line
        };

        // Directive: starts with '.'
        if line.starts_with('.') {
            let item = parse_directive(line, line_num)?;
            prog.items.push((line_num, item));
            continue;
        }

        // Instruction.
        let item = parse_instruction(line, line_num)?;
        prog.items.push((line_num, item));
    }

    Ok(prog)
}

fn parse_directive(line: &str, ln: usize) -> Result<Item, ParseError> {
    let mut parts = line.splitn(2, char::is_whitespace);
    let name = parts.next().unwrap();
    let rest = parts.next().unwrap_or("").trim();

    match name {
        ".program" => Ok(Item::Directive(Directive::Program(
            rest.trim_matches('"').to_string(),
        ))),
        ".version" => {
            let v: u8 = rest
                .parse()
                .map_err(|_| ParseError::at(ln, "invalid version"))?;
            Ok(Item::Directive(Directive::Version(v)))
        }
        ".memory" => {
            let m: u16 = rest
                .parse()
                .map_err(|_| ParseError::at(ln, "invalid memory size"))?;
            Ok(Item::Directive(Directive::Memory(m)))
        }
        ".entry" => Ok(Item::Directive(Directive::Entry(rest.to_string()))),
        ".data" => Ok(Item::Directive(Directive::Data)),
        ".text" => Ok(Item::Directive(Directive::Text)),
        ".words" => {
            let words: Result<Vec<u16>, _> =
                rest.split(',').map(|s| s.trim().parse::<u16>()).collect();
            let words = words.map_err(|_| ParseError::at(ln, "invalid .words values"))?;
            Ok(Item::DataWords(words))
        }
        _ => Err(ParseError::at(ln, format!("unknown directive '{name}'"))),
    }
}

fn parse_instruction(line: &str, ln: usize) -> Result<Item, ParseError> {
    let mut tokens = line.split_whitespace();
    let mnemonic = tokens.next().unwrap().to_uppercase();

    // Collect comma-separated operands from the rest of the line.
    let rest: String = tokens.collect::<Vec<_>>().join(" ");
    let operands: Vec<&str> = if rest.is_empty() {
        vec![]
    } else {
        rest.split(',').map(str::trim).collect()
    };

    let instr = match mnemonic.as_str() {
        "NOP" => AsmInstruction::Resolved(Instruction::Nop),
        "HALT" => AsmInstruction::Resolved(Instruction::Halt),
        "RET" => AsmInstruction::Resolved(Instruction::Ret),
        "SYNC" => AsmInstruction::Resolved(Instruction::Sync),

        // R3 type
        "ADD" | "SUB" | "MUL" | "DIV" | "MOD" | "AND" | "OR" | "XOR" | "SHL" | "SHR" => {
            let (d, s1, s2) = (
                reg(&operands, 0, ln)?,
                reg(&operands, 1, ln)?,
                reg(&operands, 2, ln)?,
            );
            AsmInstruction::Resolved(match mnemonic.as_str() {
                "ADD" => Instruction::Add(d, s1, s2),
                "SUB" => Instruction::Sub(d, s1, s2),
                "MUL" => Instruction::Mul(d, s1, s2),
                "DIV" => Instruction::Div(d, s1, s2),
                "MOD" => Instruction::Mod(d, s1, s2),
                "AND" => Instruction::And(d, s1, s2),
                "OR" => Instruction::Or(d, s1, s2),
                "XOR" => Instruction::Xor(d, s1, s2),
                "SHL" => Instruction::Shl(d, s1, s2),
                _ => Instruction::Shr(d, s1, s2),
            })
        }

        // R2 type
        "INC" | "DEC" | "NOT" | "LOAD" | "STORE" | "CMP" => {
            let (r0, r1) = (reg(&operands, 0, ln)?, reg(&operands, 1, ln)?);
            AsmInstruction::Resolved(match mnemonic.as_str() {
                "INC" => Instruction::Inc(r0, r1),
                "DEC" => Instruction::Dec(r0, r1),
                "NOT" => Instruction::Not(r0, r1),
                "LOAD" => Instruction::Load(r0, r1),
                "STORE" => Instruction::Store(r0, r1),
                _ => Instruction::Cmp(r0, r1),
            })
        }

        // R1 type
        "PUSH" => AsmInstruction::Resolved(Instruction::Push(reg(&operands, 0, ln)?)),
        "POP" => AsmInstruction::Resolved(Instruction::Pop(reg(&operands, 0, ln)?)),

        // Immediate
        "IMML" | "IMMH" => {
            let r = reg(&operands, 0, ln)?;
            let imm = imm8(&operands, 1, ln)?;
            AsmInstruction::Resolved(if mnemonic == "IMML" {
                Instruction::Imml(r, imm)
            } else {
                Instruction::Immh(r, imm)
            })
        }

        // Branches: operand is either a numeric offset or a label.
        "JMP" | "BEQ" | "BNE" | "BLT" | "BGE" | "CALL" => {
            let target = operands
                .first()
                .ok_or_else(|| ParseError::at(ln, "branch needs operand"))?;
            if let Ok(off) = target.parse::<i8>() {
                AsmInstruction::Resolved(match mnemonic.as_str() {
                    "JMP" => Instruction::Jmp(off),
                    "BEQ" => Instruction::Beq(off),
                    "BNE" => Instruction::Bne(off),
                    "BLT" => Instruction::Blt(off),
                    "BGE" => Instruction::Bge(off),
                    _ => Instruction::Call(off),
                })
            } else {
                AsmInstruction::BranchLabel {
                    mnemonic: mnemonic.clone(),
                    target: target.to_string(),
                }
            }
        }

        "TRAP" => {
            let code = imm8(&operands, 0, ln)?;
            AsmInstruction::Resolved(Instruction::Trap(code))
        }

        _ => return Err(ParseError::at(ln, format!("unknown mnemonic '{mnemonic}'"))),
    };

    Ok(Item::Instruction(instr))
}

fn reg(ops: &[&str], idx: usize, ln: usize) -> Result<Register, ParseError> {
    let s = ops
        .get(idx)
        .ok_or_else(|| ParseError::at(ln, format!("missing operand {idx}")))?;
    let upper = s.to_uppercase();
    let n: u8 = upper
        .strip_prefix('R')
        .ok_or_else(|| ParseError::at(ln, format!("expected register, got '{s}'")))?
        .parse()
        .map_err(|_| ParseError::at(ln, format!("invalid register number '{s}'")))?;
    Register::from_u8(n).ok_or_else(|| ParseError::at(ln, format!("register out of range: R{n}")))
}

fn imm8(ops: &[&str], idx: usize, ln: usize) -> Result<u8, ParseError> {
    let s = ops
        .get(idx)
        .ok_or_else(|| ParseError::at(ln, format!("missing operand {idx}")))?;
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u8::from_str_radix(hex, 16).map_err(|_| ParseError::at(ln, format!("invalid hex '{s}'")))
    } else {
        s.parse::<u8>()
            .map_err(|_| ParseError::at(ln, format!("invalid immediate '{s}'")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_program() {
        let src = r#"
; test program
.program "test"
.version 1
.text
.entry main

main:
    IMML R1, 10
    IMML R2, 20
    ADD  R3, R1, R2
    HALT
"#;
        let prog = parse(src).unwrap();
        let instrs: Vec<_> = prog
            .items
            .iter()
            .filter_map(|(_, i)| {
                if let Item::Instruction(_) = i {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(instrs.len(), 4);
    }

    #[test]
    fn parse_branch_label() {
        let src = "JMP loop\n";
        let prog = parse(src).unwrap();
        assert!(matches!(
            &prog.items[0].1,
            Item::Instruction(AsmInstruction::BranchLabel { target, .. }) if target == "loop"
        ));
    }

    #[test]
    fn parse_data_section() {
        let src = ".data\narray: .words 1, 2, 3\n";
        let prog = parse(src).unwrap();
        assert!(prog
            .items
            .iter()
            .any(|(_, i)| matches!(i, Item::DataWords(v) if v == &[1, 2, 3])));
    }

    #[test]
    fn parse_hex_immediate() {
        let src = "IMML R1, 0xFF\n";
        let prog = parse(src).unwrap();
        assert!(matches!(
            &prog.items[0].1,
            Item::Instruction(AsmInstruction::Resolved(Instruction::Imml(
                Register::R1,
                0xFF
            )))
        ));
    }

    #[test]
    fn unknown_mnemonic_error() {
        assert!(parse("FOOBAR R1\n").is_err());
    }
}
