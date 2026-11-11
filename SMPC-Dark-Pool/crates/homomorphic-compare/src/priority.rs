use crate::order::Order;
use smpc_engine::compare::secure_compare;
use std::cmp::Ordering;

pub fn compare_price_time(a: &Order, b: &Order) -> Ordering {
    let pa = smpc_engine::types::SecretShare::new(a.price.clone());
    let pb = smpc_engine::types::SecretShare::new(b.price.clone());
    let ta = smpc_engine::types::SecretShare::new(a.timestamp.clone());
    let tb = smpc_engine::types::SecretShare::new(b.timestamp.clone());
    match secure_compare(&pa, &pb) {
        Ordering::Equal => secure_compare(&ta, &tb),
        other => other,
    }
}

pub fn compare_price(a: &Order, b: &Order) -> Ordering {
    let pa = smpc_engine::types::SecretShare::new(a.price.clone());
    let pb = smpc_engine::types::SecretShare::new(b.price.clone());
    secure_compare(&pa, &pb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sss::Share;

    fn make_order(price: u64, ts: u64, id: u64) -> Order {
        Order::new(
            Share::new(1, ark_bn254::Fr::from(price)),
            Share::new(1, ark_bn254::Fr::from(100u64)),
            Share::new(1, ark_bn254::Fr::from(ts)),
            id,
        )
    }

    #[test]
    fn test_price_priority() {
        let high = make_order(100, 1, 1);
        let low = make_order(90, 2, 2);
        assert_eq!(compare_price_time(&high, &low), Ordering::Greater);
        assert_eq!(compare_price_time(&low, &high), Ordering::Less);
    }

    #[test]
    fn test_time_priority_same_price() {
        let early = make_order(100, 1, 1);
        let late = make_order(100, 2, 2);
        assert_eq!(compare_price_time(&early, &late), Ordering::Less);
    }
}
