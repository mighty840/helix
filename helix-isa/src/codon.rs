use thiserror::Error;

/// A single DNA nucleotide base (2 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Base {
    A = 0b00,
    C = 0b01,
    G = 0b10,
    T = 0b11,
}

impl Base {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => Base::A,
            0b01 => Base::C,
            0b10 => Base::G,
            _ => Base::T,
        }
    }

    pub fn as_char(self) -> char {
        match self {
            Base::A => 'A',
            Base::C => 'C',
            Base::G => 'G',
            Base::T => 'T',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'A' | 'a' => Some(Base::A),
            'C' | 'c' => Some(Base::C),
            'G' | 'g' => Some(Base::G),
            'T' | 't' => Some(Base::T),
            _ => None,
        }
    }

    /// GC = true, AT = false.
    pub fn is_gc(self) -> bool {
        matches!(self, Base::G | Base::C)
    }
}

/// A triplet of bases = 6 bits, values 0..=63.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Codon(pub u8); // only low 6 bits used

impl Codon {
    pub fn new(value: u8) -> Self {
        Self(value & 0x3F)
    }

    pub fn from_bases(b0: Base, b1: Base, b2: Base) -> Self {
        Self((b0 as u8) << 4 | (b1 as u8) << 2 | (b2 as u8))
    }

    pub fn bases(self) -> [Base; 3] {
        [
            Base::from_bits(self.0 >> 4),
            Base::from_bits(self.0 >> 2),
            Base::from_bits(self.0),
        ]
    }

    pub fn value(self) -> u8 {
        self.0
    }

    pub fn to_dna(self) -> [char; 3] {
        let b = self.bases();
        [b[0].as_char(), b[1].as_char(), b[2].as_char()]
    }

    pub fn gc_count(self) -> u8 {
        self.bases().iter().filter(|b| b.is_gc()).count() as u8
    }
}

impl std::fmt::Display for Codon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let chars = self.to_dna();
        write!(f, "{}{}{}", chars[0], chars[1], chars[2])
    }
}

/// A full DNA nucleotide sequence (ACGT string).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnaSequence(pub Vec<Base>);

impl DnaSequence {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn parse(s: &str) -> Result<Self, DnaParseError> {
        let bases = s
            .chars()
            .map(|c| Base::from_char(c).ok_or(DnaParseError::InvalidChar(c)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(bases))
    }

    pub fn push_codon(&mut self, codon: Codon) {
        for b in codon.bases() {
            self.0.push(b);
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Read codon at nucleotide offset `pos` (must be multiple of 3).
    pub fn codon_at(&self, nt_pos: usize) -> Option<Codon> {
        if nt_pos + 2 < self.0.len() {
            Some(Codon::from_bases(
                self.0[nt_pos],
                self.0[nt_pos + 1],
                self.0[nt_pos + 2],
            ))
        } else {
            None
        }
    }

    pub fn codons(&self) -> impl Iterator<Item = Codon> + '_ {
        (0..self.0.len() / 3)
            .map(move |i| Codon::from_bases(self.0[i * 3], self.0[i * 3 + 1], self.0[i * 3 + 2]))
    }

    /// GC content as a fraction [0.0, 1.0].
    pub fn gc_content(&self) -> f64 {
        if self.0.is_empty() {
            return 0.0;
        }
        let gc = self.0.iter().filter(|b| b.is_gc()).count();
        gc as f64 / self.0.len() as f64
    }

    pub fn as_str(&self) -> String {
        self.0.iter().map(|b| b.as_char()).collect()
    }
}

impl Default for DnaSequence {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DnaSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Error)]
pub enum DnaParseError {
    #[error("invalid DNA character: '{0}'")]
    InvalidChar(char),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_roundtrip() {
        for bits in 0u8..4 {
            let b = Base::from_bits(bits);
            assert_eq!(b as u8, bits);
        }
    }

    #[test]
    fn codon_from_bases() {
        let c = Codon::from_bases(Base::G, Base::C, Base::A);
        assert_eq!(c.value(), 0b10_01_00);
        let chars = c.to_dna();
        assert_eq!(chars, ['G', 'C', 'A']);
    }

    #[test]
    fn codon_gc_count() {
        let c = Codon::from_bases(Base::G, Base::C, Base::A);
        assert_eq!(c.gc_count(), 2);
    }

    #[test]
    fn dna_sequence_roundtrip() {
        let s = "GCATCGATT";
        let seq = DnaSequence::parse(s).unwrap();
        assert_eq!(seq.as_str(), s);
        assert_eq!(seq.len(), 9);
    }

    #[test]
    fn dna_sequence_codons() {
        let seq = DnaSequence::parse("GCATCG").unwrap();
        let codons: Vec<_> = seq.codons().collect();
        assert_eq!(codons.len(), 2);
        assert_eq!(codons[0].to_dna(), ['G', 'C', 'A']);
        assert_eq!(codons[1].to_dna(), ['T', 'C', 'G']);
    }

    #[test]
    fn dna_invalid_char() {
        assert!(DnaSequence::parse("GCAX").is_err());
    }
}
