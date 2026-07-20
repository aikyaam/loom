#[inline]
pub fn predict_fixed(samples: &[i64], n: usize, order: usize) -> i64 {
    match order {
        0 => 0,
        1 => samples[n - 1],
        2 => 2 * samples[n - 1] - samples[n - 2],
        3 => 3 * samples[n - 1] - 3 * samples[n - 2] + samples[n - 3],
        4 => 4 * samples[n - 1] - 6 * samples[n - 2] + 4 * samples[n - 3] - samples[n - 4],
        _ => panic!("Invalid fixed predictor order: {}", order),
    }
}

pub fn compute_fixed_residuals(samples: &[i64], order: usize) -> Vec<i64> {
    let mut residuals = vec![0i64; samples.len()];
    for i in 0..order {
        residuals[i] = samples[i];
    }
    for i in order..samples.len() {
        let prediction = predict_fixed(samples, i, order);
        residuals[i] = samples[i] - prediction;
    }
    residuals
}

pub fn reconstruct_fixed(residuals: &[i64], samples: &mut [i64], order: usize) {
    for i in 0..order {
        samples[i] = residuals[i];
    }
    for i in order..residuals.len() {
        let prediction = predict_fixed(samples, i, order);
        samples[i] = residuals[i] + prediction;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_predictors_roundtrip() {
        let samples: Vec<i64> = vec![10, 25, 42, 65, 90, 120, 155, 195];
        for order in 0..=4 {
            let res = compute_fixed_residuals(&samples, order);
            let mut reconstructed = vec![0i64; samples.len()];
            reconstruct_fixed(&res, &mut reconstructed, order);
            assert_eq!(samples, reconstructed);
        }
    }
}
