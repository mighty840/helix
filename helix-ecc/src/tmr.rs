use helix_isa::Codon;

// Triple Modular Redundancy: majority-vote over 3 copies of a codon stream.
// Each copy is separated by the SECTION_SEPARATOR codon (TGA = 0b111000 = 56).
// The decoder aligns the 3 copies and takes the majority value per codon.

pub const SECTION_SEP: u8 = 0x38; // TGA = 0b111000 = 56

pub fn section_separator() -> Codon {
    Codon::new(SECTION_SEP)
}

/// Interleave 3 copies with SECTION_SEP between them.
pub fn encode_tmr(body: &[Codon]) -> Vec<Codon> {
    let sep = section_separator();
    let mut out = Vec::with_capacity(body.len() * 3 + 2);
    out.extend_from_slice(body);
    out.push(sep);
    out.extend_from_slice(body);
    out.push(sep);
    out.extend_from_slice(body);
    out
}

/// Split a TMR-encoded stream into up to 3 copies by SECTION_SEP.
pub fn split_copies(stream: &[Codon]) -> Vec<Vec<Codon>> {
    let mut copies: Vec<Vec<Codon>> = Vec::new();
    let mut current: Vec<Codon> = Vec::new();
    for &c in stream {
        if c.value() == SECTION_SEP {
            copies.push(current);
            current = Vec::new();
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        copies.push(current);
    }
    copies
}

/// Majority vote over up to 3 copies. For each codon position, pick the value
/// that appears in at least 2 of the 3 copies. If no majority exists (all 3
/// differ), use the value from copy A (first).
///
/// Returns the voted codon stream and the number of positions where a disagreement
/// was resolved by majority (i.e., at least one copy differed).
pub fn majority_vote(copies: &[Vec<Codon>]) -> (Vec<Codon>, usize) {
    if copies.is_empty() {
        return (Vec::new(), 0);
    }
    if copies.len() == 1 {
        return (copies[0].clone(), 0);
    }

    let len = copies.iter().map(|c| c.len()).min().unwrap_or(0);
    let mut result = Vec::with_capacity(len);
    let mut corrections = 0usize;

    for i in 0..len {
        let vals: Vec<u8> = copies
            .iter()
            .filter_map(|c| c.get(i))
            .map(|c| c.value())
            .collect();
        let voted = if vals.len() >= 3 {
            // Majority of 3
            if vals[0] == vals[1] || vals[0] == vals[2] {
                vals[0]
            } else if vals[1] == vals[2] {
                vals[1]
            } else {
                // No majority — use first copy
                vals[0]
            }
        } else {
            vals[0]
        };

        if vals.iter().any(|&v| v != voted) {
            corrections += 1;
        }
        result.push(Codon::new(voted));
    }

    (result, corrections)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codons(vals: &[u8]) -> Vec<Codon> {
        vals.iter().map(|&v| Codon::new(v)).collect()
    }

    #[test]
    fn encode_decode_tmr_no_error() {
        let body = codons(&[0x01, 0x0F, 0x10, 0x1C]);
        let encoded = encode_tmr(&body);
        let copies = split_copies(&encoded);
        assert_eq!(copies.len(), 3);
        let (voted, corrections) = majority_vote(&copies);
        assert_eq!(voted, body);
        assert_eq!(corrections, 0);
    }

    #[test]
    fn majority_vote_corrects_one_copy() {
        let good = codons(&[0x01, 0x02, 0x03]);
        let mut bad = good.clone();
        bad[1] = Codon::new(0x0F); // corrupt position 1 in copy B

        let copies = vec![good.clone(), bad, good.clone()];
        let (voted, corrections) = majority_vote(&copies);
        assert_eq!(voted, good);
        assert_eq!(corrections, 1);
    }

    #[test]
    fn majority_vote_no_majority_uses_first() {
        // All 3 differ — falls back to copy A.
        let a = codons(&[0x01]);
        let b = codons(&[0x02]);
        let c = codons(&[0x03]);
        let (voted, corrections) = majority_vote(&[a.clone(), b, c]);
        assert_eq!(voted, a);
        assert_eq!(corrections, 1);
    }

    #[test]
    fn split_two_copies_from_stream() {
        let body = codons(&[0x01, 0x02]);
        let sep = section_separator();
        let mut stream = body.clone();
        stream.push(sep);
        stream.extend_from_slice(&body);

        let copies = split_copies(&stream);
        assert_eq!(copies.len(), 2);
        assert_eq!(copies[0], body);
        assert_eq!(copies[1], body);
    }
}
