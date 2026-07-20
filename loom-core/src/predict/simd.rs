#[allow(dead_code)]
fn autocorr_scalar(data: &[f64], order: usize) -> Vec<f64> {
    let n = data.len();
    let mut r = vec![0.0f64; order + 1];
    for lag in 0..=order {
        let mut sum = 0.0f64;
        for i in 0..(n - lag) {
            sum += data[i] * data[i + lag];
        }
        r[lag] = sum;
    }
    r
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn autocorr_sse2(data: &[f64], order: usize) -> Vec<f64> {
    use core::arch::x86_64::*;
    let n = data.len();
    let mut r = vec![0.0f64; order + 1];
    for lag in 0..=order {
        let mut sum = _mm_setzero_pd();
        let limit = n - lag;
        let mut i = 0;
        while i + 1 < limit {
            let a = _mm_loadu_pd(data.as_ptr().add(i));
            let b = _mm_loadu_pd(data.as_ptr().add(i + lag));
            sum = _mm_add_pd(sum, _mm_mul_pd(a, b));
            i += 2;
        }
        let mut total = _mm_cvtsd_f64(sum) + _mm_cvtsd_f64(_mm_unpackhi_pd(sum, sum));
        if i < limit {
            total += data[i] * data[i + lag];
        }
        r[lag] = total;
    }
    r
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn autocorr_avx2(data: &[f64], order: usize) -> Vec<f64> {
    use core::arch::x86_64::*;
    let n = data.len();
    let mut r = vec![0.0f64; order + 1];
    for lag in 0..=order {
        let mut sum = _mm256_setzero_pd();
        let limit = n - lag;
        let mut i = 0;
        while i + 3 < limit {
            let a = _mm256_loadu_pd(data.as_ptr().add(i));
            let b = _mm256_loadu_pd(data.as_ptr().add(i + lag));
            sum = _mm256_add_pd(sum, _mm256_mul_pd(a, b));
            i += 4;
        }
        let hi = _mm256_extractf128_pd(sum, 1);
        let lo = _mm256_castpd256_pd128(sum);
        let sum128 = _mm_add_pd(lo, hi);
        let mut total = _mm_cvtsd_f64(sum128) + _mm_cvtsd_f64(_mm_unpackhi_pd(sum128, sum128));
        while i < limit {
            total += data[i] * data[i + lag];
            i += 1;
        }
        r[lag] = total;
    }
    r
}

#[cfg(target_arch = "aarch64")]
unsafe fn autocorr_neon(data: &[f64], order: usize) -> Vec<f64> {
    use core::arch::aarch64::*;
    let n = data.len();
    let mut r = vec![0.0f64; order + 1];
    for lag in 0..=order {
        let mut sum_vec = vdupq_n_f64(0.0);
        let limit = n - lag;
        let mut i = 0;
        while i + 1 < limit {
            let a = vld1q_f64(data.as_ptr().add(i));
            let b = vld1q_f64(data.as_ptr().add(i + lag));
            sum_vec = vaddq_f64(sum_vec, vmulq_f64(a, b));
            i += 2;
        }
        let mut total = vaddvq_f64(sum_vec);
        if i < limit {
            total += data[i] * data[i + lag];
        }
        r[lag] = total;
    }
    r
}

#[cfg(target_arch = "x86_64")]
pub fn compute_autocorrelation(data: &[f64], order: usize) -> Vec<f64> {
    if is_x86_feature_detected!("avx2") {
        unsafe { autocorr_avx2(data, order) }
    } else {
        unsafe { autocorr_sse2(data, order) }
    }
}

#[cfg(target_arch = "aarch64")]
pub fn compute_autocorrelation(data: &[f64], order: usize) -> Vec<f64> {
    unsafe { autocorr_neon(data, order) }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub fn compute_autocorrelation(data: &[f64], order: usize) -> Vec<f64> {
    autocorr_scalar(data, order)
}

pub fn vector_sub_i64(a: &[i64], b: &[i64], out: &mut [i64]) {
    let len = a.len().min(b.len()).min(out.len());
    for i in 0..len {
        out[i] = a[i] - b[i];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_autocorrelation() {
        let data: Vec<f64> = (0..1024).map(|i| (i as f64 * 0.1).sin()).collect();
        let r = compute_autocorrelation(&data, 8);
        assert_eq!(r.len(), 9);
        assert!(r[0] > 0.0);
    }
}
