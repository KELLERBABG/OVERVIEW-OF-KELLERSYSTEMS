use crate::types::SecretShare;
use ark_ff::Zero;

pub fn secure_equality(a: &SecretShare, b: &SecretShare) -> bool {
    a.value() == b.value()
}

pub fn secure_equality_zero(a: &SecretShare) -> bool {
    a.value().is_zero()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Scalar;
    use ark_ff::Field;
    use sss::Share;

    #[test]
    fn test_equality() {
        let a = Share::new(1, Scalar::from(42u64));
        let b = Share::new(1, Scalar::from(42u64));
        let c = Share::new(1, Scalar::from(43u64));
        assert!(secure_equality(
            &SecretShare::new(a),
            &SecretShare::new(b.clone())
        ));
        assert!(!secure_equality(&SecretShare::new(b), &SecretShare::new(c)));
    }

    #[test]
    fn test_equality_zero() {
        let z = Share::new(1, Scalar::ZERO);
        let nz = Share::new(1, Scalar::from(1u64));
        assert!(secure_equality_zero(&SecretShare::new(z)));
        assert!(!secure_equality_zero(&SecretShare::new(nz)));
    }
}
