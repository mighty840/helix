use helix_isa::Codon;

// Triple Modular Redundancy: majority-vote over 3 copies of a codon stream.
//
// Since v0.1.1 the three copies are stored back to back with no separator, and
// the decoder splits the stream by length (each copy is exactly one third).
// v0.1.0 put the SECTION_SEPARATOR codon (TGA = 0b111000 = 56) between the
// copies, but that value can also occur inside instruction fields, so a
// separator search could cut a copy apart. Streams in the v0.1.0 layout
// (3L + 2 codons with TGA at positions L and 2L + 1) are still accepted.

/// The v0.1.0 separator codon (TGA). No longer written; accepted when decoding.
pub const SECTION_SEP: u8 = 0x38; // TGA = 0b111000 = 56

pub fn section_separator() -> Codon {
    Codon::new(SECTION_SEP)
}

/// Store 3 copies of `body` back to back.
pub fn encode_tmr(body: &[Codon]) -> Vec<Codon> {
    let mut out = Vec::with_capacity(body.len() * 3);
    out.extend_from_slice(body);
    out.extend_from_slice(body);
    out.extend_from_slice(body);
    out
}

/// Split a TMR stream into its 3 copies by length.
///
/// Returns `None` if the length fits neither layout: `3L` codons (v0.1.1) or
/// `3L + 2` codons with TGA at positions `L` and `2L + 1` (v0.1.0). A length
/// that fits neither means codons were inserted or deleted, and the copies
/// cannot be aligned.
pub fn split_copies(stream: &[Codon]) -> Option<[&[Codon]; 3]> {
    let n = stream.len();
    if n.is_multiple_of(3) {
        let l = n / 3;
        return Some([&stream[..l], &stream[l..2 * l], &stream[2 * l..]]);
    }
    if n >= 2 && (n - 2).is_multiple_of(3) {
        let l = (n - 2) / 3;
        if stream[l].value() == SECTION_SEP && stream[2 * l + 1].value() == SECTION_SEP {
            return Some([
                &stream[..l],
                &stream[l + 1..2 * l + 1],
                &stream[2 * l + 2..],
            ]);
        }
    }
    None
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
        assert_eq!(encoded.len(), 3 * body.len());
        let copies = split_copies(&encoded).unwrap();
        let (voted, corrections) = majority_vote(&copies.map(|c| c.to_vec()));
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
    fn separator_value_inside_body_is_not_a_split_point() {
        // v0.1.0 bug: TGA (56) inside a copy cut it apart.
        let body = codons(&[0x01, SECTION_SEP, 0x10, SECTION_SEP, 0x1C]);
        let encoded = encode_tmr(&body);
        let copies = split_copies(&encoded).unwrap();
        for copy in copies {
            assert_eq!(copy, body.as_slice());
        }
    }

    #[test]
    fn v010_layout_with_separators_still_splits() {
        let body = codons(&[0x01, 0x02, 0x03]);
        let sep = section_separator();
        let mut stream = body.clone();
        stream.push(sep);
        stream.extend_from_slice(&body);
        stream.push(sep);
        stream.extend_from_slice(&body);

        let copies = split_copies(&stream).unwrap();
        for copy in copies {
            assert_eq!(copy, body.as_slice());
        }
    }

    #[test]
    fn length_that_fits_no_layout_is_rejected() {
        let body = codons(&[0x01, 0x02, 0x03]);
        let mut stream = encode_tmr(&body);
        stream.pop(); // one codon lost: 8 codons, not 3L or 3L + 2 with separators
        assert!(split_copies(&stream).is_none());
    }
}
