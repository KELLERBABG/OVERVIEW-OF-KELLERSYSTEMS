use crate::field::{ScalarField, random_scalar};
use ark_ff::Field;

pub struct Polynomial {
    coefficients: Vec<ScalarField>,
}

impl Polynomial {
    pub fn random(degree: usize, secret: ScalarField, rng: &mut impl rand::RngCore) -> Self {
        let mut coeffs = Vec::with_capacity(degree + 1);
        coeffs.push(secret);
        for _ in 0..degree {
            coeffs.push(random_scalar(rng));
        }
        Self {
            coefficients: coeffs,
        }
    }

    pub fn evaluate(&self, x: &ScalarField) -> ScalarField {
        self.coefficients
            .iter()
            .rev()
            .fold(ScalarField::ZERO, |acc, c| acc * x + c)
    }

    pub fn degree(&self) -> usize {
        self.coefficients.len() - 1
    }
}

pub fn lagrange_interpolate(shares: &[(ScalarField, ScalarField)]) -> ScalarField {
    let mut result = ScalarField::ZERO;
    for (i, (x_i, y_i)) in shares.iter().enumerate() {
        let mut num = ScalarField::ONE;
        let mut den = ScalarField::ONE;
        for (j, (x_j, _)) in shares.iter().enumerate() {
            if i != j {
                num *= x_j;
                den *= x_j - x_i;
            }
        }
        result += *y_i * num * den.inverse().unwrap();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::random_scalar;
    use ark_ff::Field;
    use rand::thread_rng;

    #[test]
    fn test_evaluate_constant() {
        let mut rng = thread_rng();
        let secret = random_scalar(&mut rng);
        let poly = Polynomial::random(0, secret, &mut rng);
        assert_eq!(poly.evaluate(&ScalarField::ZERO), secret);
        assert_eq!(poly.evaluate(&ScalarField::ONE), secret);
    }

    #[test]
    fn test_evaluate_linear() {
        let mut rng = thread_rng();
        let secret = random_scalar(&mut rng);
        let poly = Polynomial::random(1, secret, &mut rng);
        let x = random_scalar(&mut rng);
        let y = poly.evaluate(&x);
        let expected = poly.coefficients[0] + poly.coefficients[1] * x;
        assert_eq!(y, expected);
    }

    #[test]
    fn test_lagrange_interpolation() {
        let mut rng = thread_rng();
        let secret = random_scalar(&mut rng);
        let poly = Polynomial::random(2, secret, &mut rng);
        let points: Vec<(ScalarField, ScalarField)> = (0..3)
            .map(|i| {
                let x = ScalarField::from(i as u64);
                (x, poly.evaluate(&x))
            })
            .collect();
        let recovered = lagrange_interpolate(&points);
        assert_eq!(recovered, secret);
    }

    #[test]
    fn test_lagrange_any_t_of_n() {
        let mut rng = thread_rng();
        let secret = random_scalar(&mut rng);
        let poly = Polynomial::random(2, secret, &mut rng);
        let all_points: Vec<(ScalarField, ScalarField)> = (1..=5)
            .map(|i| {
                let x = ScalarField::from(i as u64);
                (x, poly.evaluate(&x))
            })
            .collect();
        let recovered = lagrange_interpolate(&all_points[..3]);
        assert_eq!(recovered, secret);
        let recovered_2 = lagrange_interpolate(&all_points[2..5]);
        assert_eq!(recovered_2, secret);
    }
}
