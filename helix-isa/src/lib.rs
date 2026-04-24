pub mod codon;
pub mod decode;
pub mod encode;
pub mod instruction;
pub mod table;

pub use codon::{Base, Codon, DnaSequence};
pub use decode::{decode_instruction, DecodeError};
pub use encode::encode_instruction;
pub use instruction::{Instruction, Opcode, Register};
pub use table::CODON_TABLE;
