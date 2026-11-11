use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AffineForm {
    pub center: f64,
    pub noise_terms: Vec<(usize, f64)>,
    pub error: f64,
}

impl AffineForm {
    pub fn new(center: f64) -> Self {
        Self {
            center,
            noise_terms: Vec::new(),
            error: 0.0,
        }
    }

    pub fn with_noise(center: f64, noise_id: usize, half_range: f64) -> Self {
        Self {
            center,
            noise_terms: vec![(noise_id, half_range)],
            error: 0.0,
        }
    }

    pub fn from_interval(lo: f64, hi: f64, noise_id: &mut usize) -> Self {
        let center = (lo + hi) / 2.0;
        let half = (hi - lo) / 2.0;
        let id = *noise_id;
        *noise_id += 1;
        Self {
            center,
            noise_terms: vec![(id, half)],
            error: 0.0,
        }
    }

    pub fn to_interval(&self) -> (f64, f64) {
        let total_dev: f64 =
            self.noise_terms.iter().map(|(_, d)| d.abs()).sum::<f64>() + self.error.abs();
        (self.center - total_dev, self.center + total_dev)
    }

    pub fn add(&self, other: &AffineForm) -> AffineForm {
        let mut combined = self.clone();
        for &(id, val) in &other.noise_terms {
            if let Some(pos) = combined.noise_terms.iter_mut().find(|(i, _)| *i == id) {
                pos.1 += val;
            } else {
                combined.noise_terms.push((id, val));
            }
        }
        combined.center += other.center;
        combined.error += other.error;
        combined
    }

    pub fn sub(&self, other: &AffineForm) -> AffineForm {
        let mut combined = self.clone();
        for &(id, val) in &other.noise_terms {
            if let Some(pos) = combined.noise_terms.iter_mut().find(|(i, _)| *i == id) {
                pos.1 -= val;
            } else {
                combined.noise_terms.push((id, -val));
            }
        }
        combined.center -= other.center;
        combined.error += other.error;
        combined
    }

    pub fn mul(&self, other: &AffineForm) -> AffineForm {
        let c = self.center * other.center;
        let mut terms = Vec::new();
        let mut seen = HashMap::new();

        for &(id, d1) in &self.noise_terms {
            let coeff = d1 * other.center;
            *seen.entry(id).or_insert(0.0) += coeff;
        }
        for &(id, d2) in &other.noise_terms {
            let coeff = d2 * self.center;
            *seen.entry(id).or_insert(0.0) += coeff;
        }
        for (id, val) in seen {
            terms.push((id, val));
        }

        let mut new_err = self.center.abs() * other.error
            + other.center.abs() * self.error
            + self.error * other.error;
        for &(_, d1) in &self.noise_terms {
            for &(_, d2) in &other.noise_terms {
                new_err += (d1 * d2).abs();
            }
        }

        AffineForm {
            center: c,
            noise_terms: terms,
            error: new_err,
        }
    }

    pub fn neg(&self) -> AffineForm {
        AffineForm {
            center: -self.center,
            noise_terms: self.noise_terms.iter().map(|(i, d)| (*i, -d)).collect(),
            error: self.error,
        }
    }

    pub fn sqrt(&self) -> AffineForm {
        let (lo, hi) = self.to_interval();
        if lo < 0.0 {
            return AffineForm::new(0.0);
        }
        let center = self.center.sqrt();
        let deriv = 1.0 / (2.0 * self.center.max(1e-6));
        let mut terms = Vec::new();
        for &(id, d) in &self.noise_terms {
            terms.push((id, d * deriv));
        }
        let range = hi.sqrt() - lo.sqrt();
        AffineForm {
            center,
            noise_terms: terms,
            error: range * 0.5 + self.error * deriv.abs(),
        }
    }
}
