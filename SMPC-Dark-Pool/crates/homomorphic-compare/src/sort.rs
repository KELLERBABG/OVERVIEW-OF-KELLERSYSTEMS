use crate::order::Order;
use crate::priority::compare_price_time;

pub fn sort_orders(orders: &mut [Order]) {
    orders.sort_by(compare_price_time);
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
    fn test_sort_by_price_then_time() {
        let mut orders = vec![
            make_order(100, 5, 1),
            make_order(90, 1, 2),
            make_order(100, 1, 3),
            make_order(80, 3, 4),
        ];
        sort_orders(&mut orders);
        assert_eq!(orders[0].party_id, 4);
        assert_eq!(orders[1].party_id, 2);
        assert_eq!(orders[2].party_id, 3);
        assert_eq!(orders[3].party_id, 1);
    }
}
