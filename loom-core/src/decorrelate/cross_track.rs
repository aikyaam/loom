pub fn calculate_cross_coupling(target: &[i64], reference: &[i64]) -> (i8, i64) {
    let n = target.len();
    let mut num = 0.0f64;
    let mut den = 0.0f64;

    for i in 0..n {
        num += target[i] as f64 * reference[i] as f64;
        den += reference[i] as f64 * reference[i] as f64;
    }

    if den == 0.0 {
        return (0, 0);
    }

    let w = num / den;

    let w_q = (w * 128.0).round() as i32;
    let w_q_clamped = w_q.clamp(-128, 127) as i8;

    let mut sum_orig = 0u64;
    let mut sum_diff = 0u64;
    let scale = w_q_clamped as i64;

    for i in 0..n {
        let pred = (scale * reference[i]) >> 7;
        let diff = target[i] - pred;
        sum_orig += target[i].unsigned_abs();
        sum_diff += diff.unsigned_abs();
    }

    let bits_saved = (sum_orig as i64) - (sum_diff as i64);

    (w_q_clamped, bits_saved)
}

pub fn apply_cross_prediction(target: &mut [i64], reference: &[i64], weight: i8) {
    let scale = weight as i64;
    for i in 0..target.len() {
        let pred = (scale * reference[i]) >> 7;
        target[i] -= pred;
    }
}

pub fn reconstruct_cross_prediction(decoded: &mut [i64], reference: &[i64], weight: i8) {
    let scale = weight as i64;
    for i in 0..decoded.len() {
        let pred = (scale * reference[i]) >> 7;
        decoded[i] += pred;
    }
}
