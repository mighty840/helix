use anyhow::Result;
use clap::Parser;
use helix_asm::Assembler;
use helix_ecc::EccConfig;

#[derive(Parser)]
#[command(
    name = "helix-asm",
    about = "Assemble a Helix .hx program into a DNA sequence"
)]
struct Cli {
    /// Path to .hx source file
    input: String,
    /// Output file (default: stdout)
    #[arg(short, long)]
    output: Option<String>,
    /// Disable TMR (single copy mode)
    #[arg(long)]
    no_tmr: bool,
    /// Disable bio-safety optimization
    #[arg(long)]
    no_bio: bool,
    /// RS block size (default 8)
    #[arg(long, default_value_t = 8)]
    block_size: usize,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let source = std::fs::read_to_string(&cli.input)?;

    let config = EccConfig {
        tmr: !cli.no_tmr,
        block_size: cli.block_size,
    };
    let asm = Assembler::new(config, !cli.no_bio);
    let out = asm.assemble(&source)?;

    eprintln!("[ASM] program: {}", out.program.name);
    eprintln!("[ASM] instructions: {}", out.program.instructions.len());
    eprintln!("[ASM] DNA length: {} nt", out.dna.len());
    eprintln!(
        "[ASM] bio swaps: {}, violations remaining: {}",
        out.bio_swaps, out.bio_violations
    );

    if let Some(path) = cli.output {
        std::fs::write(path, &out.dna)?;
    } else {
        println!("{}", out.dna);
    }

    Ok(())
}
