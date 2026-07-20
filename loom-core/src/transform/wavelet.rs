pub fn forward_cdwt_53(signal: &[i64]) -> (Vec<i64>, Vec<i64>) {
    let n = signal.len();
    if n == 0 {
        return (Vec::new(), Vec::new());
    }
    let half = (n + 1) / 2;
    let mut low = vec![0i64; half];
    let mut high = vec![0i64; n / 2];

    for i in 0..(n / 2) {
        let x0 = signal[2 * i];
        let x1 = signal[2 * i + 1];
        let x2 = if 2 * i + 2 < n { signal[2 * i + 2] } else { x0 };
        let d = x1 - ((x0 + x2) >> 1);
        high[i] = d;
    }

    for i in 0..half {
        let x0 = signal[2 * i];
        let d_prev = if i > 0 { high[i - 1] } else { 0 };
        let d_curr = if i < high.len() { high[i] } else { 0 };
        let s = x0 + ((d_prev + d_curr + 2) >> 2);
        low[i] = s;
    }

    (low, high)
}

pub fn inverse_cdwt_53(low: &[i64], high: &[i64]) -> Vec<i64> {
    let total_len = low.len() + high.len();
    if total_len == 0 {
        return Vec::new();
    }
    let mut reconstructed = vec![0i64; total_len];

    for i in 0..low.len() {
        let s = low[i];
        let d_prev = if i > 0 { high[i - 1] } else { 0 };
        let d_curr = if i < high.len() { high[i] } else { 0 };
        let x0 = s - ((d_prev + d_curr + 2) >> 2);
        reconstructed[2 * i] = x0;
    }

    for i in 0..high.len() {
        let d = high[i];
        let x0 = reconstructed[2 * i];
        let x2 = if 2 * i + 2 < total_len {
            reconstructed[2 * i + 2]
        } else {
            x0
        };
        let x1 = d + ((x0 + x2) >> 1);
        reconstructed[2 * i + 1] = x1;
    }

    reconstructed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cdwt_53_roundtrip() {
        let original: Vec<i64> = vec![12, 45, 98, 120, 300, 450, 500, 610];
        let (low, high) = forward_cdwt_53(&original);
        let reconstructed = inverse_cdwt_53(&low, &high);
        assert_eq!(original, reconstructed);
    }
}
