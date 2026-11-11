use crate::types::{Scalar, SecretShare};
use ark_ff::{Field, PrimeField};
use std::cmp::Ordering;

/// Compare two secret shares.
///
/// In simulation mode (single node with all shares available), this
/// compares the share values directly. In production, this would be
/// replaced with a multi-party comparison protocol.
pub fn secure_compare(a: &SecretShare, b: &SecretShare) -> Ordering {
    (*a.value()).into_bigint().cmp(&(*b.value()).into_bigint())
}

/// Returns a SecretShare containing 1 if a < b, 0 otherwise.
pub fn secure_less_than(a: &SecretShare, b: &SecretShare) -> SecretShare {
    let result = secure_compare(a, b) == Ordering::Less;
    SecretShare::new(sss::Share::new(
        a.index(),
        if result { Scalar::ONE } else { Scalar::ZERO },
    ))
}

/// Returns a SecretShare containing 1 if a > b, 0 otherwise.
pub fn secure_greater_than(a: &SecretShare, b: &SecretShare) -> SecretShare {
    let result = secure_compare(a, b) == Ordering::Greater;
    SecretShare::new(sss::Share::new(
        a.index(),
        if result { Scalar::ONE } else { Scalar::ZERO },
    ))
}

/// Returns a SecretShare containing 1 if a == b, 0 otherwise.
pub fn secure_equal(a: &SecretShare, b: &SecretShare) -> SecretShare {
    let result = secure_compare(a, b) == Ordering::Equal;
    SecretShare::new(sss::Share::new(
        a.index(),
        if result { Scalar::ONE } else { Scalar::ZERO },
    ))
}

/// Returns the smaller of two secret values as a SecretShare.
///
/// In simulation mode this branches on the comparison result. In production
/// this would use an MPC selection circuit so no party learns which was smaller.
pub fn secure_min(a: &SecretShare, b: &SecretShare) -> SecretShare {
    if secure_compare(a, b) == Ordering::Less {
        a.clone()
    } else {
        b.clone()
    }
}

/// Returns the larger of two secret values as a SecretShare.
pub fn secure_max(a: &SecretShare, b: &SecretShare) -> SecretShare {
    if secure_compare(a, b) == Ordering::Greater {
        a.clone()
    } else {
        b.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Scalar;
    use sss::Share;

    fn ss(v: u64) -> SecretShare {
        SecretShare::new(Share::new(1, Scalar::from(v)))
    }

    #[test]
    fn test_compare_ordering() {
        let small = ss(10);
        let large = ss(20);
        assert_eq!(secure_compare(&small, &large), Ordering::Less);
        assert_eq!(secure_compare(&large, &small), Ordering::Greater);
        assert_eq!(secure_compare(&small, &ss(10)), Ordering::Equal);
    }

    #[test]
    fn test_less_than() {
        let lt = secure_less_than(&ss(5), &ss(10));
        assert_eq!(*lt.value(), Scalar::ONE);
        let lt2 = secure_less_than(&ss(10), &ss(5));
        assert_eq!(*lt2.value(), Scalar::ZERO);
    }

    #[test]
    fn test_min() {
        let min = secure_min(&ss(5), &ss(10));
        assert_eq!(*min.value(), Scalar::from(5u64));
        let min2 = secure_min(&ss(10), &ss(5));
        assert_eq!(*min2.value(), Scalar::from(5u64));
    }

    #[test]
    fn test_max() {
        let max = secure_max(&ss(5), &ss(10));
        assert_eq!(*max.value(), Scalar::from(10u64));
    }
}
