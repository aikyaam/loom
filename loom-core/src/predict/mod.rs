pub mod fixed;
pub mod lms;
pub mod lpc;
pub mod simd;

use crate::config::{CompressionLevel, RiceSearch};
use crate::entropy::rice::{find_best_k_exhaustive, fold, rice_bits};
use fixed::{compute_fixed_residuals, reconstruct_fixed};
use lpc::{
    compute_lpc_burg, compute_lpc_coefficients, compute_lpc_residuals, irls_refine,
    quantize_lpc_coefficients, reconstruct_lpc,
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
    rice_search: RiceSearch,
    _bps: usize,
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
        match rice_search {
            RiceSearch::Exhaustive => {
                let (_, bits, _) = find_best_k_exhaustive(&folded, slice);
                total_bits += 4 + bits;
            }
            RiceSearch::Limited(dist) => {
                let mean_abs =
                    slice.iter().map(|x| x.unsigned_abs()).sum::<u64>() / slice.len() as u64;
                let est_k = if mean_abs > 0 {
                    (63 - mean_abs.leading_zeros()).min(14) as u32
                } else {
                    0
                };
                let start_k = if est_k > dist { est_k - dist } else { 0 };
                let end_k = (est_k + dist).min(14);
                let mut min_bits = u64::MAX;
                for k in start_k..=end_k {
                    let mut bits = 0u64;
                    for &val in &folded {
                        bits += rice_bits(val, k);
                    }
                    if bits < min_bits {
                        min_bits = bits;
                    }
                }
                total_bits += 4 + min_bits;
            }
            RiceSearch::Estimate => {
                let mean_abs =
                    slice.iter().map(|x| x.unsigned_abs()).sum::<u64>() / slice.len() as u64;
                let k = if mean_abs > 0 {
                    (63 - mean_abs.leading_zeros()).min(14) as u32
                } else {
                    0
                };
                let mut bits = 0u64;
                for &val in &folded {
                    bits += rice_bits(val, k);
                }
                total_bits += 4 + bits;
            }
        }
    }
    total_bits
}

pub fn find_best_partition_order(
    residuals: &[i64],
    warmup_len: usize,
    max_order: u32,
    rice_search: RiceSearch,
    bps: usize,
) -> (u32, u64) {
    let mut best_order = 0;
    let mut min_bits = u64::MAX;
    for order in 0..=max_order {
        let bits = estimate_residual_bits(residuals, warmup_len, order, rice_search, bps);
        if bits < min_bits {
            min_bits = bits;
            best_order = order;
        }
    }
    (best_order, min_bits)
}

pub fn search_predictor(samples: &[i64], bps: usize, level: CompressionLevel) -> PredictionMode {
    let n = samples.len();
    let first = samples[0];
    let is_constant = samples.iter().all(|&x| x == first);
    if is_constant {
        return PredictionMode::Constant(first);
    }
    let mut best_mode = PredictionMode::Verbatim(samples.to_vec());
    let mut min_bits = (n * bps) as u64;
    let rice_search = level.rice_search();
    let max_lpc_order = level.max_lpc_order();
    let max_part_order = level.max_partition_order();
    for order in 0..=4 {
        if max_lpc_order == 0 && order > 0 {
            break;
        }
        let residuals = compute_fixed_residuals(samples, order);
        let (_, res_bits) =
            find_best_partition_order(&residuals, order, max_part_order, rice_search, bps);
        let total_bits = (order * bps) as u64 + res_bits;
        if total_bits < min_bits {
            min_bits = total_bits;
            best_mode = PredictionMode::Fixed { order, residuals };
        }
    }
    if max_lpc_order == 0 {
        return best_mode;
    }
    let apodizations = level.apodizations();
    let qlp_precision_search = level.qlp_precision_search();
    let use_irls = level.use_irls();
    let irls_iters = level.irls_iterations();
    let lpc_orders = [2, 4, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32];
    let mut prev_lpc_bits = u64::MAX;
    for &order in &lpc_orders {
        if order > max_lpc_order {
            break;
        }
        if order > 10 && prev_lpc_bits != u64::MAX {
            let improvement = min_bits.saturating_sub(prev_lpc_bits);
            if improvement < (prev_lpc_bits / 100) {
                break;
            }
        }
        let mut best_order_total = u64::MAX;
        let mut best_order_residuals = None;
        let mut best_order_coeffs: Option<(Vec<i32>, i8, usize)> = None;
        let mut candidate_coeff_sets = Vec::new();
        for &apod in &apodizations {
            if let Some(coeffs) = compute_lpc_coefficients(samples, order, apod) {
                candidate_coeff_sets.push(coeffs);
            }
        }
        if let Some(burg_coeffs) = compute_lpc_burg(samples, order) {
            candidate_coeff_sets.push(burg_coeffs);
        }

        for coeffs in candidate_coeff_sets {
            let refined = if use_irls {
                irls_refine(samples, &coeffs, order, irls_iters).unwrap_or(coeffs)
            } else {
                coeffs
            };
            let precisions: Vec<usize> = if qlp_precision_search {
                vec![8, 10, 12, 15]
            } else {
                vec![15]
            };
            for &qlp_precision in &precisions {
                let (qlp_coeffs, qlp_shift) = quantize_lpc_coefficients(&refined, qlp_precision);
                let residuals = compute_lpc_residuals(samples, &qlp_coeffs, qlp_shift, order);
                let overhead = (order * bps) as u64 + 4 + 5 + (order * qlp_precision) as u64;
                let (_, res_bits) =
                    find_best_partition_order(&residuals, order, max_part_order, rice_search, bps);
                let total_bits = overhead + res_bits;
                if total_bits < best_order_total {
                    best_order_total = total_bits;
                    best_order_residuals = Some(residuals);
                    best_order_coeffs = Some((qlp_coeffs, qlp_shift, qlp_precision));
                }
            }
        }
        if let (Some(residuals), Some((qlp_coeffs, qlp_shift, qlp_precision))) =
            (best_order_residuals, best_order_coeffs)
        {
            if best_order_total < min_bits {
                min_bits = best_order_total;
                best_mode = PredictionMode::Lpc {
                    order,
                    qlp_coeffs,
                    qlp_shift,
                    qlp_precision,
                    residuals,
                };
            }
            prev_lpc_bits = best_order_total;
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
