pub mod fixed;
pub mod lpc;

use crate::entropy::rice::{find_best_k, fold};
use fixed::{compute_fixed_residuals, reconstruct_fixed};
use lpc::{
    compute_lpc_coefficients, compute_lpc_residuals, quantize_lpc_coefficients, reconstruct_lpc,
};

#[derive(Clone, Debug)]
pub enum PredictionMode {
    Constant(i64),
    Verbatim(Vec<i64>),
    Fixed {
        order: usize,
        residuals: Vec<i64>,
    },
    Lpc {
        order: usize,
        qlp_coeffs: Vec<i32>,
        qlp_shift: i8,
        qlp_precision: usize,
        residuals: Vec<i64>,
    },
}

pub fn estimate_residual_bits(
    residuals: &[i64],
    warmup_len: usize,
    partition_order: u32,
    bps: usize,
) -> u64 {
    let total_len = residuals.len();
    if total_len <= warmup_len {
        return 0;
    }

    let num_partitions = 1 << partition_order;
    let partition_samples = total_len / num_partitions;
    let mut total_bits = 6;

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
            total_bits += 4;
            continue;
        }

        let slice = &residuals[p_start..end];
        let folded: Vec<u64> = slice.iter().map(|&x| fold(x)).collect();
        let (_, bits, _) = find_best_k(&folded, slice);
        total_bits += 4 + bits;
    }

    total_bits
}

pub fn find_best_partition_order(residuals: &[i64], warmup_len: usize, bps: usize) -> (u32, u64) {
    let mut best_order = 0;
    let mut min_bits = u64::MAX;
    for order in 0..=3 {
        let bits = estimate_residual_bits(residuals, warmup_len, order, bps);
        if bits < min_bits {
            min_bits = bits;
            best_order = order;
        }
    }
    (best_order, min_bits)
}

pub fn search_predictor(samples: &[i64], bps: usize) -> PredictionMode {
    let n = samples.len();

    let first = samples[0];
    let is_constant = samples.iter().all(|&x| x == first);
    if is_constant {
        return PredictionMode::Constant(first);
    }

    let mut best_mode = PredictionMode::Verbatim(samples.to_vec());

    let mut min_bits = (n * bps) as u64;

    for order in 0..=4 {
        let residuals = compute_fixed_residuals(samples, order);
        let (_, res_bits) = find_best_partition_order(&residuals, order, bps);
        let total_bits = (order * bps) as u64 + res_bits;

        if total_bits < min_bits {
            min_bits = total_bits;
            best_mode = PredictionMode::Fixed { order, residuals };
        }
    }

    let lpc_orders = [2, 4, 6, 8, 10, 12, 16, 20, 24, 32];
    let qlp_precision = 15;
    let mut prev_lpc_bits = u64::MAX;

    for &order in &lpc_orders {
        if order > 10 && prev_lpc_bits != u64::MAX {
            let improvement = if min_bits < prev_lpc_bits {
                prev_lpc_bits - min_bits
            } else {
                0
            };

            if improvement < (prev_lpc_bits / 100) {
                break;
            }
        }

        if let Some(coeffs) = compute_lpc_coefficients(samples, order) {
            let (qlp_coeffs, qlp_shift) = quantize_lpc_coefficients(&coeffs, qlp_precision);
            let residuals = compute_lpc_residuals(samples, &qlp_coeffs, qlp_shift, order);
            let (_, res_bits) = find_best_partition_order(&residuals, order, bps);

            let overhead = (order * bps) as u64 + 4 + 5 + (order * qlp_precision) as u64;
            let total_bits = overhead + res_bits;

            if total_bits < min_bits {
                min_bits = total_bits;
                best_mode = PredictionMode::Lpc {
                    order,
                    qlp_coeffs,
                    qlp_shift,
                    qlp_precision,
                    residuals,
                };
            }
            prev_lpc_bits = total_bits;
        }
    }

    best_mode
}

pub fn reconstruct_prediction(mode: &PredictionMode, samples: &mut [i64]) {
    match mode {
        PredictionMode::Constant(val) => {
            for x in samples.iter_mut() {
                *x = *val;
            }
        }
        PredictionMode::Verbatim(raw_samples) => {
            samples.copy_from_slice(raw_samples);
        }
        PredictionMode::Fixed { order, residuals } => {
            reconstruct_fixed(residuals, samples, *order);
        }
        PredictionMode::Lpc {
            order,
            qlp_coeffs,
            qlp_shift,
            residuals,
            ..
        } => {
            reconstruct_lpc(residuals, qlp_coeffs, *qlp_shift, *order, samples);
        }
    }
}
