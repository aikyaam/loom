pub struct KltTransform {
    pub _channels: usize,
    pub matrix: Vec<Vec<f64>>,
}

impl KltTransform {
    pub fn compute_covariance(channels_data: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let ch_count = channels_data.len();
        if ch_count == 0 || channels_data[0].is_empty() {
            return Vec::new();
        }
        let n = channels_data[0].len() as f64;
        let mut cov = vec![vec![0.0; ch_count]; ch_count];
        for i in 0..ch_count {
            for j in i..ch_count {
                let mut sum = 0.0;
                for k in 0..channels_data[0].len() {
                    sum += channels_data[i][k] * channels_data[j][k];
                }
                let val = sum / n;
                cov[i][j] = val;
                cov[j][i] = val;
            }
        }
        cov
    }

    pub fn new_2channel_identity() -> Self {
        Self {
            _channels: 2,
            matrix: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        }
    }

    pub fn transform_2channel(ch0: &[i64], ch1: &[i64]) -> (Vec<i64>, Vec<i64>) {
        let len = ch0.len().min(ch1.len());
        let mut out0 = Vec::with_capacity(len);
        let mut out1 = Vec::with_capacity(len);
        for i in 0..len {
            let m = (ch0[i] + ch1[i]) >> 1;
            let s = ch0[i] - ch1[i];
            out0.push(m);
            out1.push(s);
        }
        (out0, out1)
    }

    pub fn inverse_transform_2channel(out0: &[i64], out1: &[i64]) -> (Vec<i64>, Vec<i64>) {
        let len = out0.len().min(out1.len());
        let mut ch0 = Vec::with_capacity(len);
        let mut ch1 = Vec::with_capacity(len);
        for i in 0..len {
            let m = out0[i];
            let s = out1[i];
            let c0 = m + ((s + 1) >> 1);
            let c1 = m - (s >> 1);
            ch0.push(c0);
            ch1.push(c1);
        }
        (ch0, ch1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_klt_covariance_and_transform() {
        let ch0: Vec<i64> = vec![100, 200, 300, 400];
        let ch1: Vec<i64> = vec![90, 210, 290, 410];
        let (out0, out1) = KltTransform::transform_2channel(&ch0, &ch1);
        let (rec0, rec1) = KltTransform::inverse_transform_2channel(&out0, &out1);
        assert_eq!(ch0, rec0);
        assert_eq!(ch1, rec1);
    }
}
