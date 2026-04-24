use helix_isa::Base;

/// Error injection mode.
#[derive(Debug, Clone, Copy)]
pub enum InjectionMode {
    /// Independent per-nucleotide rates.
    Composite {
        sub_rate: f64,
        del_rate: f64,
        ins_rate: f64,
    },
    /// Concentrated errors in a random burst region.
    Burst {
        region_fraction: f64, // fraction of the sequence in the burst zone
        error_rate: f64,      // error rate within the burst zone
    },
}

/// Configuration for a single error injection run.
#[derive(Debug, Clone, Copy)]
pub struct InjectionConfig {
    pub mode: InjectionMode,
    pub seed: u64,
}

impl InjectionConfig {
    pub fn composite(sub_rate: f64, del_rate: f64, ins_rate: f64, seed: u64) -> Self {
        Self {
            mode: InjectionMode::Composite {
                sub_rate,
                del_rate,
                ins_rate,
            },
            seed,
        }
    }

    pub fn burst(region_fraction: f64, error_rate: f64, seed: u64) -> Self {
        Self {
            mode: InjectionMode::Burst {
                region_fraction,
                error_rate,
            },
            seed,
        }
    }
}

/// Statistics from a single injection run.
#[derive(Debug, Default, Clone, Copy)]
pub struct InjectionStats {
    pub substitutions: usize,
    pub deletions: usize,
    pub insertions: usize,
}

impl InjectionStats {
    pub fn total_errors(&self) -> usize {
        self.substitutions + self.deletions + self.insertions
    }
}

pub struct ErrorInjector {
    config: InjectionConfig,
}

impl ErrorInjector {
    pub fn new(config: InjectionConfig) -> Self {
        Self { config }
    }

    /// Inject errors into a nucleotide string and return (corrupted_string, stats).
    pub fn inject(&self, dna: &str) -> (String, InjectionStats) {
        let bases: Vec<Base> = dna.chars().filter_map(Base::from_char).collect();

        let mut rng = LcgRng::new(self.config.seed);
        let mut stats = InjectionStats::default();

        let corrupted = match self.config.mode {
            InjectionMode::Composite {
                sub_rate,
                del_rate,
                ins_rate,
            } => self.inject_composite(&bases, sub_rate, del_rate, ins_rate, &mut rng, &mut stats),
            InjectionMode::Burst {
                region_fraction,
                error_rate,
            } => self.inject_burst(&bases, region_fraction, error_rate, &mut rng, &mut stats),
        };

        let result: String = corrupted.iter().map(|b| b.as_char()).collect();
        (result, stats)
    }

    fn inject_composite(
        &self,
        bases: &[Base],
        sub_rate: f64,
        del_rate: f64,
        ins_rate: f64,
        rng: &mut LcgRng,
        stats: &mut InjectionStats,
    ) -> Vec<Base> {
        let mut out = Vec::with_capacity(bases.len());
        for &b in bases {
            // Possible insertion before this base.
            if rng.next_f64() < ins_rate {
                out.push(random_base(rng));
                stats.insertions += 1;
            }
            let r = rng.next_f64();
            if r < del_rate {
                // Delete this base (skip it).
                stats.deletions += 1;
            } else if r < del_rate + sub_rate {
                // Substitute.
                out.push(random_different_base(b, rng));
                stats.substitutions += 1;
            } else {
                out.push(b);
            }
        }
        out
    }

    fn inject_burst(
        &self,
        bases: &[Base],
        region_fraction: f64,
        error_rate: f64,
        rng: &mut LcgRng,
        stats: &mut InjectionStats,
    ) -> Vec<Base> {
        let len = bases.len();
        let burst_len = ((len as f64 * region_fraction) as usize).max(1);
        let burst_start = if len > burst_len {
            (rng.next_f64() * (len - burst_len) as f64) as usize
        } else {
            0
        };
        let burst_end = burst_start + burst_len;

        let mut out = Vec::with_capacity(len);
        for (i, &b) in bases.iter().enumerate() {
            if i >= burst_start && i < burst_end {
                let r = rng.next_f64();
                if r < error_rate / 3.0 {
                    out.push(random_different_base(b, rng));
                    stats.substitutions += 1;
                } else if r < 2.0 * error_rate / 3.0 {
                    stats.deletions += 1;
                    // skip
                } else if r < error_rate {
                    out.push(b);
                    out.push(random_base(rng));
                    stats.insertions += 1;
                } else {
                    out.push(b);
                }
            } else {
                out.push(b);
            }
        }
        out
    }
}

fn random_base(rng: &mut LcgRng) -> Base {
    Base::from_bits((rng.next_f64() * 4.0) as u8)
}

fn random_different_base(original: Base, rng: &mut LcgRng) -> Base {
    loop {
        let b = random_base(rng);
        if b != original {
            return b;
        }
    }
}

/// Minimal LCG PRNG (no external dep, deterministic, reproducible by seed).
struct LcgRng {
    state: u64,
}

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x12345678ABCDEF01,
        }
    }

    fn next_u64(&mut self) -> u64 {
        // Knuth's multiplicative LCG constants.
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DNA: &str = "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT";

    #[test]
    fn no_errors_at_zero_rate() {
        let cfg = InjectionConfig::composite(0.0, 0.0, 0.0, 42);
        let injector = ErrorInjector::new(cfg);
        let (out, stats) = injector.inject(DNA);
        assert_eq!(out, DNA);
        assert_eq!(stats.total_errors(), 0);
    }

    #[test]
    fn substitutions_introduced() {
        let cfg = InjectionConfig::composite(1.0, 0.0, 0.0, 42);
        let injector = ErrorInjector::new(cfg);
        let (out, stats) = injector.inject(DNA);
        assert!(stats.substitutions > 0);
        assert_eq!(out.len(), DNA.len()); // substitutions don't change length
    }

    #[test]
    fn deletions_shorten_sequence() {
        let cfg = InjectionConfig::composite(0.0, 0.5, 0.0, 99);
        let injector = ErrorInjector::new(cfg);
        let (out, stats) = injector.inject(DNA);
        assert!(stats.deletions > 0);
        assert!(out.len() < DNA.len());
    }

    #[test]
    fn insertions_lengthen_sequence() {
        let cfg = InjectionConfig::composite(0.0, 0.0, 0.5, 7);
        let injector = ErrorInjector::new(cfg);
        let (out, stats) = injector.inject(DNA);
        assert!(stats.insertions > 0);
        assert!(out.len() > DNA.len());
    }

    #[test]
    fn burst_mode_runs() {
        let cfg = InjectionConfig::burst(0.2, 0.9, 13);
        let injector = ErrorInjector::new(cfg);
        let (out, stats) = injector.inject(DNA);
        assert!(stats.total_errors() > 0);
        assert!(!out.is_empty());
    }

    #[test]
    fn deterministic_with_same_seed() {
        let cfg = InjectionConfig::composite(0.05, 0.01, 0.01, 42);
        let (out1, _) = ErrorInjector::new(cfg).inject(DNA);
        let (out2, _) = ErrorInjector::new(cfg).inject(DNA);
        assert_eq!(out1, out2);
    }
}
