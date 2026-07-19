pub fn compute_lpc_coefficients(samples: &[i64], order: usize) -> Option<Vec<f64>> {
    let n = samples.len();
    if n <= order {
        return None;
    }

    let n_f = n as f64;
    let windowed: Vec<f64> = samples
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            let w = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (n_f - 1.0)).cos());
            x as f64 * w
        })
        .collect();

    let mut r = vec![0.0f64; order + 1];
    for lag in 0..=order {
        let mut sum = 0.0f64;
        for i in 0..(n - lag) {
            sum += windowed[i] * windowed[i + lag];
        }
        r[lag] = sum;
    }

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

        for j in 1..=i {
            a[j] = a_new[j];
        }

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
