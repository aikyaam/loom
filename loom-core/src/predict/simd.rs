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

#[cfg(target_arch = "x86_64")]
pub fn compute_autocorrelation(data: &[f64], order: usize) -> Vec<f64> {
    if is_x86_feature_detected!("avx2") {
        unsafe { autocorr_avx2(data, order) }
    } else {
        unsafe { autocorr_sse2(data, order) }
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn compute_autocorrelation(data: &[f64], order: usize) -> Vec<f64> {
    autocorr_scalar(data, order)
}
