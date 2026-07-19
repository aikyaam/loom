use crate::bitstream::{BitReader, BitWriter};
use std::io;

#[inline]
pub fn fold(val: i64) -> u64 {
    if val >= 0 {
        (val as u64) << 1
    } else {
        ((!val) as u64) << 1 | 1
    }
}

#[inline]
pub fn unfold(val: u64) -> i64 {
    if (val & 1) == 0 {
        (val >> 1) as i64
    } else {
        !(val >> 1) as i64
    }
}

pub fn rice_bits(val: u64, k: u32) -> u64 {
    let q = val >> k;
    let unary_len = q + 1;
    let binary_len = k as u64;
    unary_len + binary_len
}

pub fn min_bits_2s_complement(val: i64) -> usize {
    if val >= 0 {
        64 - val.leading_zeros() as usize + 1
    } else {
        64 - (!val).leading_zeros() as usize + 1
    }
}

pub fn find_best_k_exhaustive(folded_samples: &[u64], slice: &[i64]) -> (u32, u64, usize) {
    find_best_k(folded_samples, slice)
}

pub fn find_best_k(folded_samples: &[u64], slice: &[i64]) -> (u32, u64, usize) {
    if folded_samples.is_empty() {
        return (0, 0, 0);
    }
    let mut best_k = 0;
    let mut min_bits = u64::MAX;

    for k in 0..15 {
        let mut bits = 0u64;
        for &val in folded_samples {
            bits += rice_bits(val, k);
        }
        if bits < min_bits {
            min_bits = bits;
            best_k = k;
        }
    }

    let mut max_escape_bps = 0;
    for &val in slice {
        let req = min_bits_2s_complement(val);
        if req > max_escape_bps {
            max_escape_bps = req;
        }
    }

    if max_escape_bps > 31 {
        max_escape_bps = 31;
    }
    if max_escape_bps == 0 {
        max_escape_bps = 1;
    }

    let escape_bits = (folded_samples.len() as u64) * (max_escape_bps as u64) + 5;
    if escape_bits < min_bits {
        return (15, escape_bits, max_escape_bps);
    }

    (best_k, min_bits, max_escape_bps)
}

pub fn encode_residuals(
    writer: &mut BitWriter,
    residuals: &[i64],
    warmup_len: usize,
    partition_order: u32,
    allow_escape: bool,
) {
    let num_partitions = 1 << partition_order;
    let total_len = residuals.len();

    writer.write_bits(0, 2);
    writer.write_bits(partition_order as u64, 4);

    let partition_samples = total_len / num_partitions;

    for p in 0..num_partitions {
        let start = p * partition_samples;
        let mut end = start + partition_samples;
        if p == num_partitions - 1 {
            end = total_len;
        }

        let p_start = if p == 0 {
            std::cmp::max(start, warmup_len)
        } else {
            start
        };

        if p_start >= end {
            writer.write_bits(0, 4);
            continue;
        }

        let slice = &residuals[p_start..end];
        let folded: Vec<u64> = slice.iter().map(|&x| fold(x)).collect();

        let (k, _, escape_bps) = if allow_escape {
            find_best_k(&folded, slice)
        } else {
            let mut best_k = 0;
            let mut min_bits = u64::MAX;
            for k in 0..15 {
                let mut bits = 0u64;
                for &val in &folded {
                    bits += rice_bits(val, k);
                }
                if bits < min_bits {
                    min_bits = bits;
                    best_k = k;
                }
            }
            (best_k as u32, min_bits, 0)
        };

        writer.write_bits(k as u64, 4);

        if k == 15 {
            writer.write_bits(escape_bps as u64, 5);
            for &val in slice {
                let mask = if escape_bps == 64 {
                    u64::MAX
                } else {
                    (1u64 << escape_bps) - 1
                };
                writer.write_bits((val as u64) & mask, escape_bps);
            }
        } else {
            for val in folded {
                let q = val >> k;
                let r = val & ((1u64 << k) - 1);
                writer.write_unary(q);
                writer.write_bits(r, k as usize);
            }
        }
    }
}

pub fn decode_residuals(
    reader: &mut BitReader,
    residuals: &mut [i64],
    warmup_len: usize,
) -> io::Result<()> {
    let total_len = residuals.len();

    let coding_method = reader.read_bits(2)?;
    match coding_method {
        0 => {}
        2 => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ANS residual coding method (10b) is reserved and not yet implemented",
            ));
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported residual coding method: {}", coding_method),
            ));
        }
    }

    let partition_order = reader.read_bits(4)? as u32;
    let num_partitions = 1 << partition_order;
    let partition_samples = total_len / num_partitions;

    for p in 0..num_partitions {
        let start = p * partition_samples;
        let mut end = start + partition_samples;
        if p == num_partitions - 1 {
            end = total_len;
        }

        let p_start = if p == 0 {
            std::cmp::max(start, warmup_len)
        } else {
            start
        };

        if p_start >= end {
            let k = reader.read_bits(4)?;
            if k != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Empty partition must have k=0",
                ));
            }
            continue;
        }

        let k = reader.read_bits(4)? as u32;

        if k == 15 {
            let unencoded_bps = reader.read_bits(5)? as usize;
            let sign_bit = 1u64 << (unencoded_bps - 1);
            let mask = (1u64 << unencoded_bps) - 1;
            for i in p_start..end {
                let uval = reader.read_bits(unencoded_bps)?;

                let sval = if (uval & sign_bit) != 0 {
                    (uval | !mask) as i64
                } else {
                    uval as i64
                };
                residuals[i] = sval;
            }
        } else {
            for i in p_start..end {
                let q = reader.read_unary()?;
                let r = reader.read_bits(k as usize)?;
                let folded = (q << k) | r;
                residuals[i] = unfold(folded);
            }
        }
    }

    Ok(())
}
