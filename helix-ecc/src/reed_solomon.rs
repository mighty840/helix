// Reed-Solomon codes over GF(2^6) (64 elements).
// Each block of 8 data symbols produces 2 parity symbols, correcting up to 1 symbol error.
// Field: GF(2^6) with primitive polynomial x^6 + x + 1 (= 0x43).

const FIELD_SIZE: usize = 64;
const PRIM_POLY: u8 = 0x43; // x^6 + x + 1

/// Precomputed log and antilog (exp) tables for GF(2^6).
struct GfTables {
    exp: [u8; 128], // exp[i] = alpha^i, extended to 2*(FIELD_SIZE-1)
    log: [u8; 64],  // log[v] = i such that alpha^i = v (log[0] undefined)
}

impl GfTables {
    const fn build() -> Self {
        let mut exp = [0u8; 128];
        let mut log = [0u8; 64];
        let mut x = 1u8;
        let mut i = 0usize;
        while i < FIELD_SIZE - 1 {
            exp[i] = x;
            exp[i + FIELD_SIZE - 1] = x;
            log[x as usize] = i as u8;
            x = gf_mul_raw(x, 2);
            i += 1;
        }
        GfTables { exp, log }
    }
}

static TABLES: GfTables = GfTables::build();

/// Multiply two GF(2^6) elements without using the log table (for table init).
const fn gf_mul_raw(mut a: u8, mut b: u8) -> u8 {
    let mut result = 0u8;
    while b > 0 {
        if b & 1 != 0 {
            result ^= a;
        }
        let high = a & 0x20; // bit 5 set?
        a <<= 1;
        a &= 0x3F;
        if high != 0 {
            a ^= PRIM_POLY & 0x3F; // reduce mod primitive poly (low 6 bits)
        }
        b >>= 1;
    }
    result
}

fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let log_sum = (TABLES.log[a as usize] as u16 + TABLES.log[b as usize] as u16) as usize;
    TABLES.exp[log_sum % (FIELD_SIZE - 1)]
}

#[allow(dead_code)]
fn gf_pow(base: u8, exp: u8) -> u8 {
    if exp == 0 {
        return 1;
    }
    if base == 0 {
        return 0;
    }
    let log_base = TABLES.log[base as usize] as u16;
    let log_result = (log_base * exp as u16) % (FIELD_SIZE as u16 - 1);
    TABLES.exp[log_result as usize]
}

fn gf_inv(a: u8) -> u8 {
    assert!(a != 0, "gf_inv(0) undefined");
    TABLES.exp[(FIELD_SIZE - 1 - TABLES.log[a as usize] as usize) % (FIELD_SIZE - 1)]
}

fn gf_div(a: u8, b: u8) -> u8 {
    gf_mul(a, gf_inv(b))
}

/// Compute 2 parity symbols for up to 8 data symbols using RS(10,8) over GF(64).
///
/// Generator polynomial: g(x) = (x - alpha^1)(x - alpha^2)
///   = x^2 + (alpha^1 + alpha^2)*x + alpha^3
///
/// We use systematic encoding: the message is the first 8 symbols, parity appended.
pub fn encode(data: &[u8]) -> [u8; 2] {
    assert!(data.len() <= 8);
    // Generator: g(x) = x^2 + g1*x + g0 where
    //   g1 = alpha^1 XOR alpha^2, g0 = alpha^1 * alpha^2 = alpha^3
    let g1 = TABLES.exp[1] ^ TABLES.exp[2]; // alpha^1 + alpha^2
    let g0 = TABLES.exp[3]; // alpha^3

    let mut r0 = 0u8; // parity[0]
    let mut r1 = 0u8; // parity[1]

    for &d in data {
        let feedback = d ^ r1;
        r1 = r0 ^ gf_mul(feedback, g1);
        r0 = gf_mul(feedback, g0);
    }
    [r1, r0]
}

/// Attempt to correct up to 1 symbol error in `data` (8 data + 2 parity = 10 symbols).
/// Returns the corrected 8-symbol data slice, or Err if uncorrectable.
pub fn decode(received: &[u8; 10]) -> Result<[u8; 8], RsError> {
    // Compute syndromes S1 = p(alpha^1), S2 = p(alpha^2)
    let s1 = eval_poly(received, TABLES.exp[1]);
    let s2 = eval_poly(received, TABLES.exp[2]);

    if s1 == 0 && s2 == 0 {
        // No errors.
        let mut out = [0u8; 8];
        out.copy_from_slice(&received[..8]);
        return Ok(out);
    }

    // With 2 syndromes we can locate and correct 1 error.
    // Error locator: sigma(x) = 1 + sigma1*x
    // sigma1 = S1 (for single error)
    // Error location: root of sigma is alpha^(-j) where j is error position.
    // error magnitude: e = S1 / sigma(X_j)

    // Find error position j: find j such that eval_poly at alpha^j = S1 condition holds.
    // For 1 error at position j: S1 = e * alpha^j, S2 = e * alpha^(2j)
    // => S2 / S1 = alpha^j => j = log(S2) - log(S1)
    if s1 == 0 {
        return Err(RsError::Uncorrectable);
    }

    let log_s1 = TABLES.log[s1 as usize] as i16;
    let log_s2 = TABLES.log[s2 as usize] as i16;
    let j = ((log_s2 - log_s1).rem_euclid(FIELD_SIZE as i16 - 1)) as usize;

    if j >= 10 {
        return Err(RsError::Uncorrectable);
    }

    let alpha_j = TABLES.exp[j];
    let e = gf_div(s1, alpha_j);

    let mut corrected = *received;
    corrected[9 - j] ^= e; // symbols are indexed from highest degree

    // Re-check syndromes
    let s1c = eval_poly(&corrected, TABLES.exp[1]);
    let s2c = eval_poly(&corrected, TABLES.exp[2]);
    if s1c != 0 || s2c != 0 {
        return Err(RsError::Uncorrectable);
    }

    let mut out = [0u8; 8];
    out.copy_from_slice(&corrected[..8]);
    Ok(out)
}

fn eval_poly(poly: &[u8], x: u8) -> u8 {
    let mut result = 0u8;
    for &coef in poly {
        result = gf_mul(result, x) ^ coef;
    }
    result
}

#[derive(Debug, thiserror::Error)]
pub enum RsError {
    #[error("uncorrectable RS error")]
    Uncorrectable,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gf_tables_correct() {
        // alpha^0 = 1
        assert_eq!(TABLES.exp[0], 1);
        // alpha^(n-1) = 1 (cycle)
        assert_eq!(TABLES.exp[FIELD_SIZE - 1], TABLES.exp[0]);
    }

    #[test]
    fn gf_mul_commutative() {
        assert_eq!(gf_mul(3, 7), gf_mul(7, 3));
        assert_eq!(gf_mul(0x1F, 0x2A), gf_mul(0x2A, 0x1F));
    }

    #[test]
    fn gf_mul_identity() {
        for v in 1u8..64 {
            assert_eq!(gf_mul(v, 1), v);
        }
    }

    #[test]
    fn gf_inv_correct() {
        for v in 1u8..64 {
            assert_eq!(gf_mul(v, gf_inv(v)), 1, "inv failed for {v}");
        }
    }

    #[test]
    fn encode_decode_no_error() {
        let data = [0x01, 0x0F, 0x10, 0x2A, 0x3B, 0x00, 0x1C, 0x07u8];
        let parity = encode(&data);
        let mut block = [0u8; 10];
        block[..8].copy_from_slice(&data);
        block[8..].copy_from_slice(&parity);
        let recovered = decode(&block).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn encode_decode_one_error() {
        let data = [0x05, 0x12, 0x3F, 0x01, 0x08, 0x00, 0x1B, 0x09u8];
        let parity = encode(&data);
        let mut block = [0u8; 10];
        block[..8].copy_from_slice(&data);
        block[8..].copy_from_slice(&parity);

        // Corrupt position 3
        block[3] ^= 0x15;
        let recovered = decode(&block).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn encode_decode_parity_error() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08u8];
        let parity = encode(&data);
        let mut block = [0u8; 10];
        block[..8].copy_from_slice(&data);
        block[8..].copy_from_slice(&parity);

        // Corrupt a parity symbol
        block[9] ^= 0x3C;
        let recovered = decode(&block).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn two_errors_uncorrectable() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08u8];
        let parity = encode(&data);
        let mut block = [0u8; 10];
        block[..8].copy_from_slice(&data);
        block[8..].copy_from_slice(&parity);
        block[1] ^= 0x11;
        block[5] ^= 0x22;
        assert!(decode(&block).is_err());
    }

    #[test]
    fn gf_pow_test() {
        // alpha^0 = 1
        assert_eq!(gf_pow(TABLES.exp[1], 0), 1);
        // alpha^1 = alpha
        assert_eq!(gf_pow(TABLES.exp[1], 1), TABLES.exp[1]);
    }
}
