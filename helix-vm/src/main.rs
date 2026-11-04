use anyhow::Result;
use clap::Parser;
use helix_ecc::EccConfig;
use helix_isa::{decode_instruction, Codon};
use helix_vm::{Cpu, CpuError, Memory, ProgramLoader};

#[derive(Parser)]
#[command(name = "helix-vm", about = "Execute a Helix DNA-encoded program")]
struct Cli {
    /// DNA sequence string or path to a .dna file
    input: String,
    /// Show instruction-level trace
    #[arg(long)]
    trace: bool,
    /// Disable TMR (single copy mode)
    #[arg(long)]
    no_tmr: bool,
    /// RS block size (default 8)
    #[arg(long, default_value_t = 8)]
    block_size: usize,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let dna = if std::path::Path::new(&cli.input).exists() {
        std::fs::read_to_string(&cli.input)?.trim().to_string()
    } else {
        cli.input.clone()
    };

    let config = EccConfig {
        tmr: !cli.no_tmr,
        block_size: cli.block_size,
    };

    let loader = ProgramLoader::new(config);
    let prog = loader.from_dna_string(&dna)?;

    eprintln!(
        "[LOAD] {} instructions, entry={}",
        prog.instructions.len(),
        prog.entry
    );

    let mut mem = Memory::new();
    mem.load_data(&prog.data);

    let mut cpu = Cpu::new(mem);

    let result = if cli.trace {
        run_traced(&mut cpu, &prog.instructions)
    } else {
        cpu.run(&prog.instructions).map(|_| ())
    };

    eprintln!("[DONE] {} instructions executed", cpu.instr_count);
    eprintln!(
        "[REGS] R1={} R2={} R3={} R4={} R5={} R6={} R7={}",
        cpu.regs[1], cpu.regs[2], cpu.regs[3], cpu.regs[4], cpu.regs[5], cpu.regs[6], cpu.regs[7]
    );

    match result {
        Ok(()) | Err(CpuError::Halted) => {
            eprintln!("[EXIT] OK");
        }
        Err(e) => {
            eprintln!("[TRAP] {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Run like `Cpu::run`, printing each instruction before it executes and the
/// registers it leaves behind.
fn run_traced(cpu: &mut Cpu, program: &[[Codon; 4]]) -> Result<(), CpuError> {
    loop {
        if let Some(codons) = program.get(cpu.pc) {
            match decode_instruction(codons) {
                Ok(instr) => eprint!("[EXEC] {:04} {:<16}", cpu.pc, instr.to_string()),
                Err(_) => eprint!("[EXEC] {:04} {:<16}", cpu.pc, "?"),
            }
        }
        let step = cpu.step(program);
        if step.is_ok() {
            eprintln!(
                " R1={} R2={} R3={} R4={} R5={} R6={} R7={}",
                cpu.regs[1],
                cpu.regs[2],
                cpu.regs[3],
                cpu.regs[4],
                cpu.regs[5],
                cpu.regs[6],
                cpu.regs[7]
            );
        } else {
            eprintln!();
        }
        match step {
            Ok(()) => {}
            Err(CpuError::Halted) => return Ok(()),
            Err(e) => return Err(e),
        }
    }
}
