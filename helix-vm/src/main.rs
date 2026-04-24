use anyhow::Result;
use clap::Parser;
use helix_ecc::EccConfig;
use helix_vm::{Cpu, Memory, ProgramLoader};

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

    let result = cpu.run(&prog.instructions);

    eprintln!("[DONE] {} instructions executed", cpu.instr_count);
    eprintln!(
        "[REGS] R1={} R2={} R3={} R4={} R5={} R6={} R7={}",
        cpu.regs[1], cpu.regs[2], cpu.regs[3], cpu.regs[4], cpu.regs[5], cpu.regs[6], cpu.regs[7]
    );

    match result {
        Ok(_) | Err(helix_vm::CpuError::Halted) => {
            eprintln!("[EXIT] OK");
        }
        Err(e) => {
            eprintln!("[TRAP] {e}");
        }
    }

    Ok(())
}
