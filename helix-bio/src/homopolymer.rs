use helix_isa::Base;

pub const MAX_RUN: usize = 4;

#[derive(Debug, Clone)]
pub struct HomopolymerViolation {
    pub pos: usize,
    pub base: Base,
    pub run_length: usize,
}

/// Find all homopolymer runs exceeding MAX_RUN (4) identical bases.
pub fn scan(bases: &[Base]) -> Vec<HomopolymerViolation> {
    let mut violations = Vec::new();
    if bases.is_empty() {
        return violations;
    }

    let mut run_start = 0;
    let mut run_len = 1;

    for i in 1..bases.len() {
        if bases[i] == bases[i - 1] {
            run_len += 1;
        } else {
            if run_len > MAX_RUN {
                violations.push(HomopolymerViolation {
                    pos: run_start,
                    base: bases[run_start],
                    run_length: run_len,
                });
            }
            run_start = i;
            run_len = 1;
        }
    }
    // Check the final run.
    if run_len > MAX_RUN {
        violations.push(HomopolymerViolation {
            pos: run_start,
            base: bases[run_start],
            run_length: run_len,
        });
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_violation_short_runs() {
        let bases = vec![Base::A, Base::A, Base::A, Base::A, Base::G, Base::G];
        assert!(scan(&bases).is_empty());
    }

    #[test]
    fn violation_5_run() {
        let bases = vec![Base::A, Base::A, Base::A, Base::A, Base::A];
        let v = scan(&bases);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].run_length, 5);
        assert_eq!(v[0].pos, 0);
    }

    #[test]
    fn two_violations() {
        let mut bases = vec![Base::G; 5];
        bases.extend(vec![Base::A; 2]);
        bases.extend(vec![Base::T; 6]);
        let v = scan(&bases);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn exactly_four_ok() {
        let bases = vec![Base::C, Base::C, Base::C, Base::C];
        assert!(scan(&bases).is_empty());
    }
}
