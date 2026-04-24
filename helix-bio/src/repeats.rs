use helix_isa::Base;

/// Max allowed consecutive dinucleotide repeats, e.g. ATATATATAT = 5 repeats.
pub const MAX_DINUC_REPEATS: usize = 4;

#[derive(Debug, Clone)]
pub struct DinucRepeatViolation {
    pub pos: usize,
    pub dinuc: [Base; 2],
    pub repeat_count: usize,
}

/// Scan for excessive dinucleotide repeats (e.g., ATATATATAT).
pub fn scan(bases: &[Base]) -> Vec<DinucRepeatViolation> {
    let mut violations = Vec::new();
    if bases.len() < 4 {
        return violations;
    }

    let mut i = 0;
    while i + 1 < bases.len() {
        let dinuc = [bases[i], bases[i + 1]];
        // Count consecutive repetitions of this dinucleotide.
        let mut count = 1;
        let mut j = i + 2;
        while j + 1 < bases.len() && bases[j] == dinuc[0] && bases[j + 1] == dinuc[1] {
            count += 1;
            j += 2;
        }
        if count > MAX_DINUC_REPEATS {
            violations.push(DinucRepeatViolation {
                pos: i,
                dinuc,
                repeat_count: count,
            });
            // Skip past this run.
            i = j;
        } else {
            i += 2;
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_violation_short_repeat() {
        // ATATAT = 3 repeats, ok
        let bases = vec![Base::A, Base::T, Base::A, Base::T, Base::A, Base::T];
        assert!(scan(&bases).is_empty());
    }

    #[test]
    fn violation_five_repeats() {
        // ATATATATAT = 5 repeats of AT
        let bases = vec![
            Base::A,
            Base::T,
            Base::A,
            Base::T,
            Base::A,
            Base::T,
            Base::A,
            Base::T,
            Base::A,
            Base::T,
        ];
        let v = scan(&bases);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].repeat_count, 5);
    }

    #[test]
    fn exactly_four_ok() {
        // ATATATAT = 4 repeats, ok
        let bases = vec![
            Base::A,
            Base::T,
            Base::A,
            Base::T,
            Base::A,
            Base::T,
            Base::A,
            Base::T,
        ];
        assert!(scan(&bases).is_empty());
    }
}
