use helix_isa::Codon;

/// The SYNC marker: 4 codon values that appear at the start of every ECC block.
/// These values are in the opcode range but chosen so that the 4-codon pattern
/// is unique and cannot appear as a valid data instruction sequence.
///
/// Values come from SPEC §6.2 Layer 2: [0x1D, 0x2A, 0x15, 0x2A].
/// 0x1D = SYNC opcode primary
/// 0x2A = NOT synonym (used as padding — cannot be a valid opcode primary)
/// 0x15 = BEQ primary
/// 0x2A = NOT synonym (repeated — makes the pattern distinctive)
pub const SYNC_CODON_PATTERN: [u8; 4] = [0x1D, 0x2A, 0x15, 0x2A];

/// Check whether the 4 codons starting at `codons[0]` match the SYNC pattern.
pub fn is_sync(codons: &[Codon]) -> bool {
    if codons.len() < 4 {
        return false;
    }
    codons[0].value() == SYNC_CODON_PATTERN[0]
        && codons[1].value() == SYNC_CODON_PATTERN[1]
        && codons[2].value() == SYNC_CODON_PATTERN[2]
        && codons[3].value() == SYNC_CODON_PATTERN[3]
}

/// Build the 4-codon SYNC marker sequence.
pub fn sync_codons() -> [Codon; 4] {
    SYNC_CODON_PATTERN.map(Codon::new)
}

/// Scan `codons` for the next SYNC pattern starting at or after `start`.
/// Returns the codon index of the SYNC pattern, or None.
pub fn find_next_sync(codons: &[Codon], start: usize) -> Option<usize> {
    for i in start..codons.len().saturating_sub(3) {
        if is_sync(&codons[i..]) {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_detection() {
        let sync = sync_codons();
        assert!(is_sync(&sync));
    }

    #[test]
    fn sync_not_detected_in_garbage() {
        let codons = [Codon::new(0), Codon::new(1), Codon::new(2), Codon::new(3)];
        assert!(!is_sync(&codons));
    }

    #[test]
    fn find_sync_in_stream() {
        let mut stream: Vec<Codon> = (0..8).map(|i| Codon::new(i as u8)).collect();
        let sync = sync_codons();
        stream.extend_from_slice(&sync);
        stream.extend((0..4).map(|_| Codon::new(0)));

        let pos = find_next_sync(&stream, 0).unwrap();
        assert_eq!(pos, 8);
    }

    #[test]
    fn find_sync_not_present() {
        let stream: Vec<Codon> = (0..12).map(|i| Codon::new(i as u8)).collect();
        assert!(find_next_sync(&stream, 0).is_none());
    }
}
