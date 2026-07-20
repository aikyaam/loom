pub struct NlmsFilter {
    order: usize,
    mu: f64,
    gamma: f64,
    weights: Vec<f64>,
    history: Vec<f64>,
}

impl NlmsFilter {
    pub fn new(order: usize, mu: f64, gamma: f64) -> Self {
        Self {
            order,
            mu,
            gamma,
            weights: vec![0.0; order],
            history: vec![0.0; order],
        }
    }

    pub fn predict_and_update(&mut self, sample: f64) -> (f64, f64) {
        let mut y = 0.0;
        for i in 0..self.order {
            y += self.weights[i] * self.history[i];
        }
        let e = sample - y;
        let mut norm = self.gamma;
        for i in 0..self.order {
            norm += self.history[i] * self.history[i];
        }
        let step = self.mu * e / norm;
        for i in 0..self.order {
            self.weights[i] += step * self.history[i];
        }
        for i in (1..self.order).rev() {
            self.history[i] = self.history[i - 1];
        }
        if self.order > 0 {
            self.history[0] = sample;
        }
        (y, e)
    }
}

pub fn compute_nlms_residuals(samples: &[i64], order: usize, mu: f64) -> Vec<i64> {
    let mut filter = NlmsFilter::new(order, mu, 1e-4);
    let mut residuals = Vec::with_capacity(samples.len());
    for &s in samples {
        let (_pred, err) = filter.predict_and_update(s as f64);
        residuals.push(err.round() as i64);
    }
    residuals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nlms_filter() {
        let samples: Vec<i64> = (0..500)
            .map(|i| ((i as f64 * 0.1).sin() * 10000.0) as i64)
            .collect();
        let res = compute_nlms_residuals(&samples, 4, 0.5);
        assert_eq!(res.len(), 500);
    }
}
