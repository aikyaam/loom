use crate::config::Apodization;

fn apply_apodization(samples: &[i64], apod: Apodization) -> Vec<f64> {
    let n = samples.len();
    let n_f = n as f64;
    let pi = std::f64::consts::PI;
    match apod {
        Apodization::Tukey(alpha) => {
            let alpha = alpha.clamp(0.0, 1.0);
            let taper = (alpha * n_f * 0.5).round() as usize;
            let taper = taper.min(n / 2);
            samples
                .iter()
                .enumerate()
                .map(|(i, &x)| {
                    let w = if i < taper {
                        0.5 * (1.0 - (pi * i as f64 / taper as f64).cos())
                    } else if i >= n - taper {
                        0.5 * (1.0 - (pi * (n - i - 1) as f64 / taper as f64).cos())
                    } else {
                        1.0
                    };
                    x as f64 * w
                })
                .collect()
        }
        Apodization::SubdivideTukey(num_sub) => {
            let sub_size = n / num_sub as usize;
            if sub_size < 2 {
                return samples.iter().map(|&x| x as f64).collect();
            }
            samples
                .iter()
                .enumerate()
                .map(|(i, &x)| {
                    let _sub_idx = i / sub_size;
                    let local_i = i % sub_size;
                    let sub_taper = (sub_size as f64 * 0.5).round() as usize;
                    let sub_taper = sub_taper.min(sub_size / 2);
                    let w = if local_i < sub_taper {
                        0.5 * (1.0 - (pi * local_i as f64 / sub_taper as f64).cos())
                    } else if local_i >= sub_size - sub_taper {
                        0.5 * (1.0
                            - (pi * (sub_size - local_i - 1) as f64 / sub_taper as f64).cos())
                    } else {
                        1.0
                    };
                    x as f64 * w
                })
                .collect()
        }
        Apodization::PunchoutTukey(num_punch) => {
            let window_size = (n_f / num_punch as f64).round() as usize;
            let window_size = window_size.max(2).min(n);
            let mut energy = Vec::with_capacity(n);
            let mut e = 0.0f64;
            for i in 0..window_size.min(n) {
                let s = samples[i] as f64;
                e += s * s;
            }
            energy.push(e);
            for i in window_size..n {
                let s_old = samples[i - window_size] as f64;
                let s_new = samples[i] as f64;
                e += s_new * s_new - s_old * s_old;
                energy.push(e);
            }
            let threshold = energy.iter().copied().fold(0.0f64, f64::max) * 0.5;
            let punch_indices: Vec<usize> = energy
                .iter()
                .enumerate()
                .filter(|(_, &e)| e >= threshold)
                .map(|(i, _)| i + window_size / 2)
                .collect();
            samples
                .iter()
                .enumerate()
                .map(|(i, &x)| {
                    let dist = punch_indices
                        .iter()
                        .map(|&p| (i as isize - p as isize).unsigned_abs())
                        .min()
                        .unwrap_or(usize::MAX);
                    if dist < window_size / 4 {
                        0.0
                    } else {
                        x as f64
                    }
                })
                .collect()
        }
    }
}

pub fn compute_lpc_coefficients(
    samples: &[i64],
    order: usize,
    apod: Apodization,
) -> Option<Vec<f64>> {
    let n = samples.len();
    if n <= order {
        return None;
    }
    let windowed = apply_apodization(samples, apod);
    let r = super::simd::compute_autocorrelation(&windowed, order);
    let mut error = r[0];
    if error == 0.0 {
        return None;
    }
    let mut a = vec![0.0f64; order + 1];
    for i in 1..=order {
        let mut sum = r[i];
        for j in 1..i {
            sum += a[j] * r[i - j];
        }
        let ki = -sum / error;
        if ki.abs() >= 1.0 {
            return None;
        }
        let mut a_new = vec![0.0f64; order + 1];
        for j in 1..i {
            a_new[j] = a[j] + ki * a[i - j];
        }
        a_new[i] = ki;
        a.copy_from_slice(&a_new[..=order]);
        error *= 1.0 - ki * ki;
        if error <= 0.0 {
            return None;
        }
    }
    let coeffs: Vec<f64> = a[1..=order].iter().map(|&x| -x).collect();
    Some(coeffs)
}

pub fn quantize_lpc_coefficients(coeffs: &[f64], precision: usize) -> (Vec<i32>, i8) {
    let mut max_val = 0.0f64;
    for &c in coeffs {
        let abs_c = c.abs();
        if abs_c > max_val {
            max_val = abs_c;
        }
    }
    let mut shift = 0i8;
    if max_val > 0.0 {
        let max_target = (1 << (precision - 1)) as f64 - 1.0;
        let mut s = (max_target / max_val).log2().floor() as i8;
        s = s.clamp(0, 15);
        shift = s;
    }
    let scale = if shift >= 0 {
        (1 << shift) as f64
    } else {
        1.0 / ((1 << (-shift)) as f64)
    };
    let quantized: Vec<i32> = coeffs
        .iter()
        .map(|&c| {
            let val = (c * scale).round() as i32;
            let limit = 1 << (precision - 1);
            val.clamp(-limit, limit - 1)
        })
        .collect();
    (quantized, shift)
}

pub fn compute_lpc_residuals(
    samples: &[i64],
    qlp_coeffs: &[i32],
    qlp_shift: i8,
    order: usize,
) -> Vec<i64> {
    let mut residuals = vec![0i64; samples.len()];
    for i in 0..order {
        residuals[i] = samples[i];
    }
    for i in order..samples.len() {
        let mut sum = 0i64;
        for j in 0..order {
            sum += qlp_coeffs[j] as i64 * samples[i - 1 - j];
        }
        let prediction = if qlp_shift >= 0 {
            sum >> qlp_shift
        } else {
            sum << (-qlp_shift)
        };
        residuals[i] = samples[i] - prediction;
    }
    residuals
}

pub fn reconstruct_lpc(
    residuals: &[i64],
    qlp_coeffs: &[i32],
    qlp_shift: i8,
    order: usize,
    samples: &mut [i64],
) {
    for i in 0..order {
        samples[i] = residuals[i];
    }
    for i in order..residuals.len() {
        let mut sum = 0i64;
        for j in 0..order {
            sum += qlp_coeffs[j] as i64 * samples[i - 1 - j];
        }
        let prediction = if qlp_shift >= 0 {
            sum >> qlp_shift
        } else {
            sum << (-qlp_shift)
        };
        samples[i] = residuals[i] + prediction;
    }
}

pub fn irls_refine(
    samples: &[i64],
    coeffs: &[f64],
    order: usize,
    iterations: u32,
) -> Option<Vec<f64>> {
    let n = samples.len();
    if n <= order {
        return None;
    }
    let mut current = coeffs.to_vec();
    let n_f = n as f64;
    let windowed: Vec<f64> = samples
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            let w = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (n_f - 1.0)).cos());
            x as f64 * w
        })
        .collect();
    for _ in 0..iterations {
        let mut residuals = vec![0.0f64; n];
        for i in order..n {
            let mut pred = 0.0;
            for j in 0..order {
                pred += current[j] * windowed[i - 1 - j];
            }
            residuals[i] = windowed[i] + pred;
        }
        let mut weights = vec![1.0f64; n];
        for i in order..n {
            let r = residuals[i].abs();
            if r > 1e-10 {
                weights[i] = 1.0 / r;
            }
        }
        let mut r = vec![0.0f64; order + 1];
        for lag in 0..=order {
            let mut sum = 0.0;
            for i in order..n {
                sum += windowed[i] * windowed[i - lag] * weights[i];
            }
            r[lag] = sum;
        }
        let mut a = vec![0.0f64; order + 1];
        let mut error = r[0];
        if error == 0.0 {
            return None;
        }
        for i in 1..=order {
            let mut sum = r[i];
            for j in 1..i {
                sum += a[j] * r[i - j];
            }
            let ki = -sum / error;
            if ki.abs() >= 1.0 {
                return None;
            }
            let mut a_new = vec![0.0f64; order + 1];
            for j in 1..i {
                a_new[j] = a[j] + ki * a[i - j];
            }
            a_new[i] = ki;
            a.copy_from_slice(&a_new[..=order]);
            error *= 1.0 - ki * ki;
            if error <= 0.0 {
                return None;
            }
        }
        current = a[1..=order].iter().map(|&x| -x).collect();
    }
    Some(current)
}
