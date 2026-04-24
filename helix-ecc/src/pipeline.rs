use crate::reed_solomon;
use crate::sync::{find_next_sync, sync_codons};
use crate::tmr;
use helix_isa::Codon;
use thiserror::Error;

/// ECC configuration flags.
#[derive(Debug, Clone, Copy)]
pub struct EccConfig {
    /// Enable Triple Modular Redundancy (store 3 copies).
    pub tmr: bool,
    /// Number of data instructions per RS block (default 8).
    pub block_size: usize,
}

impl Default for EccConfig {
    fn default() -> Self {
        Self {
            tmr: true,
            block_size: 8,
        }
    }
}

/// A successfully decoded ECC block.
#[derive(Debug, Clone)]
pub struct DecodedBlock {
    /// The 8 data codons (one per instruction-codon, so 8 * 4 = 32 instruction codons).
    pub data: Vec<Codon>,
    /// Whether RS correction was applied.
    pub rs_corrected: bool,
}

#[derive(Debug, Error)]
pub enum EccError {
    #[error("no SYNC marker found at expected position")]
    SyncMissing,
    #[error("RS decode failed: {0}")]
    RsDecode(#[from] reed_solomon::RsError),
    #[error("not enough data to form a complete block")]
    Truncated,
}

/// Encodes a flat stream of instruction codons into an ECC-protected codon sequence.
///
/// Input: raw instruction codons (multiples of 4 per instruction, N instructions total).
/// Output: SYNC + 8-instr data block + 2 RS parity instrs, repeated; optionally TMR.
///
/// Each "instruction codon" block is 4 codons (one instruction). We group 8 instructions
/// = 32 codons as one data block. The RS code operates on the first codon of each
/// instruction (the opcode codon), treating each 4-codon instruction as one RS symbol.
/// For simplicity, we apply RS over the concatenated raw codon values (8 * 4 = 32 symbols
/// chunked into 8-symbol groups by taking every 4th starting codon).
///
/// Simplified approach used here: treat each instruction (4 codons = one logical unit)
/// as an 8-codon-value group of 8 instructions. RS is applied over the 8 opcode codon
/// values (first codon of each instruction), producing 2 parity codon values stored as
/// an ECCP instruction.
pub struct EccEncoder {
    config: EccConfig,
}

impl EccEncoder {
    pub fn new(config: EccConfig) -> Self {
        Self { config }
    }

    /// Encode a stream of instruction codons (length must be multiple of 4).
    pub fn encode(&self, instr_codons: &[Codon]) -> Vec<Codon> {
        assert!(
            instr_codons.len().is_multiple_of(4),
            "instruction codons must be multiple of 4"
        );

        let instrs_per_block = self.config.block_size;
        let codons_per_block = instrs_per_block * 4;
        let sync = sync_codons();

        let mut body: Vec<Codon> = Vec::new();

        for chunk in instr_codons.chunks(codons_per_block) {
            // SYNC marker
            body.extend_from_slice(&sync);

            // Data codons
            body.extend_from_slice(chunk);

            // Pad if last chunk is short
            let instrs_in_chunk = chunk.len() / 4;
            let padding_instrs = instrs_per_block - instrs_in_chunk;
            for _ in 0..padding_instrs * 4 {
                body.push(Codon::new(0x1B)); // NOP primary codon
            }

            // RS parity: compute over the 8 opcode codon values
            let opcode_symbols: Vec<u8> = (0..instrs_per_block)
                .map(|i| {
                    let padded_start = i * 4;
                    if padded_start < chunk.len() {
                        chunk[padded_start].value()
                    } else {
                        0x1B // NOP
                    }
                })
                .collect();

            let parity = reed_solomon::encode(&opcode_symbols);
            // Store parity as an ECCP instruction (opcode 0x1E + 0x00 + p0 + p1)
            body.push(Codon::new(0x1E)); // ECCP opcode
            body.push(Codon::new(0x00));
            body.push(Codon::new(parity[0] & 0x3F));
            body.push(Codon::new(parity[1] & 0x3F));
        }

        if self.config.tmr {
            tmr::encode_tmr(&body)
        } else {
            body
        }
    }
}

pub struct EccDecoder {
    config: EccConfig,
}

impl EccDecoder {
    pub fn new(config: EccConfig) -> Self {
        Self { config }
    }

    /// Decode an ECC-encoded codon stream back into instruction codons.
    pub fn decode(&self, encoded: &[Codon]) -> Result<Vec<Codon>, EccError> {
        // Step 1: TMR majority vote if enabled.
        let body: Vec<Codon> = if self.config.tmr {
            let copies = tmr::split_copies(encoded);
            if copies.is_empty() {
                return Ok(Vec::new());
            }
            let (voted, _corrections) = tmr::majority_vote(&copies);
            voted
        } else {
            encoded.to_vec()
        };

        // Step 2: Find SYNC markers and decode blocks.
        let instrs_per_block = self.config.block_size;
        let data_codons_per_block = instrs_per_block * 4;
        let _full_block_codons = 4 + data_codons_per_block + 4; // SYNC + data + ECCP

        let mut output: Vec<Codon> = Vec::new();
        let mut pos = 0usize;

        while pos < body.len() {
            // Find SYNC at or after pos.
            let sync_pos = find_next_sync(&body, pos).ok_or(EccError::SyncMissing)?;
            pos = sync_pos + 4; // skip SYNC

            if pos + data_codons_per_block + 4 > body.len() {
                return Err(EccError::Truncated);
            }

            let data_slice = &body[pos..pos + data_codons_per_block];
            let eccp_slice = &body[pos + data_codons_per_block..pos + data_codons_per_block + 4];

            // Extract stored parity from ECCP instruction codons[2] and [3].
            let stored_p0 = eccp_slice[2].value();
            let stored_p1 = eccp_slice[3].value();

            // Extract opcode symbols from data.
            let opcode_symbols: Vec<u8> = (0..instrs_per_block)
                .map(|i| data_slice[i * 4].value())
                .collect();

            // Build RS block (8 data + 2 parity).
            let mut rs_block = [0u8; 10];
            rs_block[..8].copy_from_slice(&opcode_symbols);
            rs_block[8] = stored_p0;
            rs_block[9] = stored_p1;

            let corrected_opcodes = reed_solomon::decode(&rs_block).map_err(EccError::RsDecode)?;

            // Reconstruct instruction codons: replace opcode codon if RS corrected it.
            let rs_corrected = corrected_opcodes
                .iter()
                .zip(opcode_symbols.iter())
                .any(|(a, b)| a != b);
            let _ = rs_corrected; // stored in DecodedBlock but we flatten here

            let mut corrected_data = data_slice.to_vec();
            for (i, &corrected_op) in corrected_opcodes.iter().enumerate() {
                corrected_data[i * 4] = Codon::new(corrected_op);
            }

            // Trim NOP padding at end (added during encode for short last blocks).
            // We output all data — the VM will see NOPs as valid instructions.
            output.extend_from_slice(&corrected_data);

            pos += data_codons_per_block + 4; // skip data + ECCP
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_isa::{encode_instruction, Instruction, Register};

    fn sample_instructions() -> Vec<Codon> {
        let instrs = [
            Instruction::Imml(Register::R1, 5),
            Instruction::Imml(Register::R2, 10),
            Instruction::Add(Register::R3, Register::R1, Register::R2),
            Instruction::Halt,
        ];
        instrs.iter().flat_map(encode_instruction).collect()
    }

    #[test]
    fn encode_decode_roundtrip_no_tmr() {
        let config = EccConfig {
            tmr: false,
            block_size: 8,
        };
        let encoder = EccEncoder::new(config);
        let decoder = EccDecoder::new(config);

        let original = sample_instructions();
        let encoded = encoder.encode(&original);
        let decoded = decoder.decode(&encoded).unwrap();

        // Decoded contains padded NOPs — just check prefix matches.
        assert_eq!(&decoded[..original.len()], original.as_slice());
    }

    #[test]
    fn encode_decode_roundtrip_with_tmr() {
        let config = EccConfig {
            tmr: true,
            block_size: 8,
        };
        let encoder = EccEncoder::new(config);
        let decoder = EccDecoder::new(config);

        let original = sample_instructions();
        let encoded = encoder.encode(&original);
        let decoded = decoder.decode(&encoded).unwrap();

        assert_eq!(&decoded[..original.len()], original.as_slice());
    }
}
