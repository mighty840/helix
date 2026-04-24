use helix_isa::Base;

/// Max palindrome length before flagging (hairpin risk).
pub const MAX_PALINDROME: usize = 8;

#[derive(Debug, Clone)]
pub struct PalindromeViolation {
    pub pos: usize,
    pub length: usize,
}

/// Watson-Crick complement of a base.
fn complement(b: Base) -> Base {
    match b {
        Base::A => Base::T,
        Base::T => Base::A,
        Base::G => Base::C,
        Base::C => Base::G,
    }
}

/// Check if bases[pos..pos+len] is a Watson-Crick palindrome
/// (reverse complement of itself — forms a hairpin stem).
fn is_palindrome(bases: &[Base], pos: usize, len: usize) -> bool {
    if !len.is_multiple_of(2) {
        return false; // palindromes must be even-length for WC pairing
    }
    for i in 0..len / 2 {
        if bases[pos + i] != complement(bases[pos + len - 1 - i]) {
            return false;
        }
    }
    true
}

/// Find all Watson-Crick palindromes longer than MAX_PALINDROME (8bp).
pub fn scan(bases: &[Base]) -> Vec<PalindromeViolation> {
    let mut violations = Vec::new();
    let min_len = MAX_PALINDROME + 2; // first length to flag

    for pos in 0..bases.len() {
        // Only check even lengths starting from min_len.
        let mut len = if min_len.is_multiple_of(2) {
            min_len
        } else {
            min_len + 1
        };
        while pos + len <= bases.len() {
            if is_palindrome(bases, pos, len) {
                violations.push(PalindromeViolation { pos, length: len });
                // Report the longest palindrome at this position, don't double-count.
                break;
            }
            len += 2;
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_bases(s: &str) -> Vec<Base> {
        s.chars().map(|c| Base::from_char(c).unwrap()).collect()
    }

    #[test]
    fn short_palindrome_ok() {
        // GAATTC (EcoRI) — 6bp palindrome, under limit
        let bases = parse_bases("GAATTC");
        assert!(scan(&bases).is_empty());
    }

    #[test]
    fn eight_bp_palindrome_ok() {
        // GAATTCTT is not a palindrome — construct exact 8bp palindrome
        // GCAATTGC: rev-comp = GCAATTGC ✓ (G↔C, C↔G, A↔T, A↔T, T↔A, T↔A, G↔C, C↔G)
        let bases = parse_bases("GCAATTGC");
        assert!(scan(&bases).is_empty());
    }

    #[test]
    fn ten_bp_palindrome_violation() {
        // Construct a 10bp palindrome: GCAATTTTGC... let's use GAATTCGAATTC
        // Actually: GAATTCGAATTC is 12bp. Let's do GGATCGATCC:
        // A 10bp palindrome: GCATGCATGC? No...
        // Easiest: CCGGAATTCCGG is 12bp palindrome.
        // 10bp: GGATCCGATCC? No.
        // Let me just construct one: AACCGGTTAACCGGTT is 16bp.
        // Simple 10bp: seq and rev-comp(seq) match.
        // seq = AGATCGATCT: rev-comp = AGATCGATCT ✓ (A-T, G-C, A-T, T-A, C-G, G-C, A-T, T-A, C-G, T-A)
        // A=T,G=C,A=T,T=A,C=G — wait let me be more careful.
        // Let's use: GCATGCATGC (10 bp)
        // rev-comp: complement each base then reverse
        // G→C, C→G, A→T, T→A, G→C, C→G, A→T, T→A, G→C, C→G
        // = CGTACGTACG, reversed = GCATGCATGC ✓
        let bases = parse_bases("GCATGCATGC");
        let v = scan(&bases);
        assert!(!v.is_empty(), "expected palindrome violation");
        assert_eq!(v[0].length, 10);
    }

    #[test]
    fn complement_bases() {
        assert_eq!(complement(Base::A), Base::T);
        assert_eq!(complement(Base::T), Base::A);
        assert_eq!(complement(Base::G), Base::C);
        assert_eq!(complement(Base::C), Base::G);
    }
}
