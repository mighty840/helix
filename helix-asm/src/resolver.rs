use crate::parser::{AsmInstruction, Directive, Item, SourceProgram};
use helix_isa::Instruction;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("undefined label '{0}'")]
    UndefinedLabel(String),
    #[error("branch offset to '{0}' is out of i8 range ({1})")]
    OffsetOutOfRange(String, i64),
    #[error("no .entry directive found")]
    NoEntry,
}

/// A fully resolved program ready for encoding.
#[derive(Debug)]
pub struct ResolvedProgram {
    pub name: String,
    pub version: u8,
    pub memory_words: u16,
    pub entry: usize, // instruction index
    pub instructions: Vec<Instruction>,
    pub data: Vec<u16>,
}

/// Resolve all labels and produce a flat instruction list.
pub fn resolve(src: &SourceProgram) -> Result<ResolvedProgram, ResolveError> {
    // Pass 1: collect labels and instruction indices.
    let mut labels: HashMap<String, usize> = HashMap::new();
    let mut instr_count = 0usize;
    let mut name = String::from("unnamed");
    let mut version = 1u8;
    let mut memory_words = 64u16;
    let mut data: Vec<u16> = Vec::new();
    let mut entry_label: Option<String> = None;

    for (_, item) in &src.items {
        match item {
            Item::Label(lbl) => {
                labels.insert(lbl.clone(), instr_count);
            }
            Item::Instruction(_) => {
                instr_count += 1;
            }
            Item::Directive(d) => match d {
                Directive::Program(n) => name = n.clone(),
                Directive::Version(v) => version = *v,
                Directive::Memory(m) => memory_words = *m,
                Directive::Entry(lbl) => entry_label = Some(lbl.clone()),
                Directive::Data | Directive::Text => {}
            },
            Item::DataWords(words) => data.extend_from_slice(words),
        }
    }

    // Pass 2: resolve instructions and branch labels.
    let mut instructions: Vec<Instruction> = Vec::with_capacity(instr_count);

    for (_, item) in &src.items {
        if let Item::Instruction(asm_instr) = item {
            let instr_idx = instructions.len();
            let instr = match asm_instr {
                AsmInstruction::Resolved(i) => *i,
                AsmInstruction::BranchLabel { mnemonic, target } => {
                    let target_idx = labels
                        .get(target)
                        .copied()
                        .ok_or_else(|| ResolveError::UndefinedLabel(target.clone()))?;

                    // Branch offset: target_idx - (instr_idx + 1) + 1
                    // Because the CPU increments PC before executing the branch,
                    // and the formula is new_pc = (pc_after_inc) + off - 1.
                    // So off = target_idx - instr_idx.
                    let off_raw: i64 = target_idx as i64 - instr_idx as i64;
                    let off = i8::try_from(off_raw)
                        .map_err(|_| ResolveError::OffsetOutOfRange(target.clone(), off_raw))?;

                    match mnemonic.as_str() {
                        "JMP" => Instruction::Jmp(off),
                        "BEQ" => Instruction::Beq(off),
                        "BNE" => Instruction::Bne(off),
                        "BLT" => Instruction::Blt(off),
                        "BGE" => Instruction::Bge(off),
                        "CALL" => Instruction::Call(off),
                        _ => unreachable!(),
                    }
                }
            };
            instructions.push(instr);
        }
    }

    let entry = if let Some(lbl) = entry_label {
        *labels.get(&lbl).ok_or(ResolveError::UndefinedLabel(lbl))?
    } else {
        0
    };

    Ok(ResolvedProgram {
        name,
        version,
        memory_words,
        entry,
        instructions,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn resolve_simple() {
        let src = r#"
.program "add_test"
.version 1
.text
.entry main
main:
    IMML R1, 5
    IMML R2, 3
    ADD R3, R1, R2
    HALT
"#;
        let parsed = parse(src).unwrap();
        let resolved = resolve(&parsed).unwrap();
        assert_eq!(resolved.instructions.len(), 4);
        assert_eq!(resolved.entry, 0);
        assert_eq!(resolved.name, "add_test");
    }

    #[test]
    fn resolve_label_branch() {
        let src = r#"
.text
.entry start
start:
    IMML R1, 0
loop:
    INC R1, R1
    IMML R2, 5
    CMP R1, R2
    BLT loop
    HALT
"#;
        let parsed = parse(src).unwrap();
        let resolved = resolve(&parsed).unwrap();
        // Instructions: [0]=IMML, [1]=INC(loop), [2]=IMML, [3]=CMP, [4]=BLT, [5]=HALT
        // BLT at index 4, loop at index 1 → off = 1 - 4 = -3
        assert!(matches!(resolved.instructions[4], Instruction::Blt(-3)));
    }

    #[test]
    fn undefined_label_error() {
        let src = "JMP nonexistent\n";
        let parsed = parse(src).unwrap();
        assert!(matches!(
            resolve(&parsed),
            Err(ResolveError::UndefinedLabel(_))
        ));
    }

    #[test]
    fn data_section_collected() {
        let src = ".data\nvals: .words 10, 20, 30\n.text\nHALT\n";
        let parsed = parse(src).unwrap();
        let resolved = resolve(&parsed).unwrap();
        assert_eq!(resolved.data, vec![10, 20, 30]);
    }
}
