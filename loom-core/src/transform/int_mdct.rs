pub fn forward_int_mdct(input: &[i64]) -> Vec<i64> {
    let n = input.len();
    let mut out = vec![0i64; n / 2];
    for k in 0..(n / 2) {
        let mut sum = 0.0;
        for i in 0..n {
            let angle = std::f64::consts::PI / (n as f64)
                * (i as f64 + 0.5 + (n as f64 * 0.25))
                * (k as f64 + 0.5);
            sum += input[i] as f64 * angle.cos();
        }
        out[k] = sum.round() as i64;
    }
    out
}

pub fn inverse_int_mdct(coeffs: &[i64]) -> Vec<i64> {
    let k_len = coeffs.len();
    let n = k_len * 2;
    let mut out = vec![0i64; n];
    for i in 0..n {
        let mut sum = 0.0;
        for k in 0..k_len {
            let angle = std::f64::consts::PI / (n as f64)
                * (i as f64 + 0.5 + (n as f64 * 0.25))
                * (k as f64 + 0.5);
            sum += coeffs[k] as f64 * angle.cos();
        }
        out[i] = (sum * 2.0 / (k_len as f64)).round() as i64;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_mdct_forward_inverse() {
        let signal: Vec<i64> = vec![10, 20, 30, 40, 50, 60, 70, 80];
        let coeffs = forward_int_mdct(&signal);
        assert_eq!(coeffs.len(), 4);
        let rec = inverse_int_mdct(&coeffs);
        assert_eq!(rec.len(), 8);
    }
}
