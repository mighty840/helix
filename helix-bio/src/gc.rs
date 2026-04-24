use helix_isa::Base;

pub const WINDOW: usize = 100; // nucleotides
pub const GC_MIN: f64 = 0.40;
pub const GC_MAX: f64 = 0.60;

#[derive(Debug, Clone)]
pub struct GcViolation {
    /// Nucleotide position of the window start.
    pub pos: usize,
    pub gc_fraction: f64,
}

/// Scan `bases` for 100-nt windows outside [40%, 60%] GC content.
pub fn scan(bases: &[Base]) -> Vec<GcViolation> {
    if bases.len() < WINDOW {
        let frac = gc_fraction(bases);
        if !(GC_MIN..=GC_MAX).contains(&frac) {
            return vec![GcViolation {
                pos: 0,
                gc_fraction: frac,
            }];
        }
        return vec![];
    }

    let mut violations = Vec::new();
    // Step by 10 nt (overlapping windows).
    let mut pos = 0;
    while pos + WINDOW <= bases.len() {
        let frac = gc_fraction(&bases[pos..pos + WINDOW]);
        if !(GC_MIN..=GC_MAX).contains(&frac) {
            violations.push(GcViolation {
                pos,
                gc_fraction: frac,
            });
        }
        pos += 10;
    }
    violations
}

pub fn gc_fraction(bases: &[Base]) -> f64 {
    if bases.is_empty() {
        return 0.5;
    }
    let gc = bases.iter().filter(|b| b.is_gc()).count();
    gc as f64 / bases.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_gc(n: usize) -> Vec<Base> {
        vec![Base::G; n]
    }
    fn all_at(n: usize) -> Vec<Base> {
        vec![Base::A; n]
    }
    fn balanced(n: usize) -> Vec<Base> {
        (0..n)
            .map(|i| if i % 2 == 0 { Base::G } else { Base::A })
            .collect()
    }

    #[test]
    fn balanced_no_violation() {
        let bases = balanced(200);
        assert!(scan(&bases).is_empty());
    }

    #[test]
    fn all_gc_violation() {
        let bases = all_gc(200);
        let v = scan(&bases);
        assert!(!v.is_empty());
        assert!(v[0].gc_fraction > GC_MAX);
    }

    #[test]
    fn all_at_violation() {
        let bases = all_at(200);
        let v = scan(&bases);
        assert!(!v.is_empty());
        assert!(v[0].gc_fraction < GC_MIN);
    }

    #[test]
    fn short_sequence_balanced() {
        let bases = balanced(30);
        assert!(scan(&bases).is_empty());
    }
}
