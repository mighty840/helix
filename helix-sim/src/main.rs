use anyhow::Result;
use clap::Parser;
use helix_ecc::EccConfig;
use helix_isa::{decode_instruction, Codon, DnaSequence};
use helix_sim::inject::InjectionConfig;
use helix_sim::metrics::simulate;

#[derive(Parser)]
#[command(
    name = "helix-sim",
    about = "Inject DNA storage errors and test ECC recovery"
)]
struct Cli {
    /// DNA sequence string or path to a .dna file
    input: String,
    /// Substitution rate (0.0–1.0)
    #[arg(long, default_value_t = 0.005)]
    sub_rate: f64,
    /// Deletion rate (0.0–1.0)
    #[arg(long, default_value_t = 0.001)]
    del_rate: f64,
    /// Insertion rate (0.0–1.0)
    #[arg(long, default_value_t = 0.001)]
    ins_rate: f64,
    /// Use burst mode instead of uniform
    #[arg(long)]
    burst: bool,
    /// Burst region fraction (0.0–1.0)
    #[arg(long, default_value_t = 0.1)]
    burst_frac: f64,
    /// Disable TMR
    #[arg(long)]
    no_tmr: bool,
    /// Random seed
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Number of simulation runs
    #[arg(long, default_value_t = 1)]
    runs: usize,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let dna = if std::path::Path::new(&cli.input).exists() {
        std::fs::read_to_string(&cli.input)?.trim().to_string()
    } else {
        cli.input.clone()
    };

    let ecc_cfg = EccConfig {
        tmr: !cli.no_tmr,
        block_size: 8,
    };

    // Build original instruction list by decoding with ECC.
    let seq = DnaSequence::parse(&dna)?;
    let codons: Vec<Codon> = seq.codons().collect();
    let decoder = helix_ecc::EccDecoder::new(ecc_cfg);
    let raw = decoder.decode(&codons)?;
    let original_instrs: Vec<[Codon; 4]> = raw
        .chunks_exact(4)
        .map(|c: &[Codon]| <[Codon; 4]>::try_from(c).unwrap())
        .collect();

    println!(
        "Program: {} instructions, {} nt",
        original_instrs.len(),
        dna.len()
    );
    println!("Running {} simulation(s)...", cli.runs);

    let mut total_recovery = 0.0f64;

    for run in 0..cli.runs {
        let seed = cli.seed.wrapping_add(run as u64);
        let inject_cfg = if cli.burst {
            InjectionConfig::burst(
                cli.burst_frac,
                cli.sub_rate + cli.del_rate + cli.ins_rate,
                seed,
            )
        } else {
            InjectionConfig::composite(cli.sub_rate, cli.del_rate, cli.ins_rate, seed)
        };

        let report = simulate(&dna, &original_instrs, inject_cfg, ecc_cfg);
        total_recovery += report.recovery_rate();

        if cli.runs == 1 {
            report.print_summary();
        } else {
            println!(
                "Run {:3}: {} errors → {:.1}% recovery | ECC: {}",
                run + 1,
                report.injection.total_errors(),
                report.recovery_rate() * 100.0,
                if report.ecc_success { "OK" } else { "FAIL" }
            );
        }
    }

    if cli.runs > 1 {
        println!(
            "\nAverage recovery: {:.1}%",
            total_recovery / cli.runs as f64 * 100.0
        );
    }

    // Suppress unused warning for decode_instruction import.
    let _ = decode_instruction;

    Ok(())
}
