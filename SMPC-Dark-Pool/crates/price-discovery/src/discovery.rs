use crate::aggregation::VolumePoint;
use ark_ff::Field;
use sss::Share;

#[derive(Debug, Clone)]
pub struct ClearingPrice {
    pub price: Share,
    pub volume: Share,
}

pub fn compute_clearing_price(
    bid_volumes: &[VolumePoint],
    ask_volumes: &[VolumePoint],
) -> Option<ClearingPrice> {
    let mut cum_bid = ark_bn254::Fr::ZERO;
    let mut cum_ask = ark_bn254::Fr::ZERO;
    let max_len = bid_volumes.len().max(ask_volumes.len());
    for i in 0..max_len {
        if i < bid_volumes.len() {
            cum_bid += bid_volumes[i].volume.value;
        }
        if i < ask_volumes.len() {
            cum_ask += ask_volumes[i].volume.value;
        }
        if cum_bid >= cum_ask && cum_ask > ark_bn254::Fr::ZERO {
            let price = if i < bid_volumes.len() {
                bid_volumes[i].price.clone()
            } else {
                ask_volumes[i].price.clone()
            };
            let volume = Share::new(1, cum_bid.min(cum_ask));
            return Some(ClearingPrice { price, volume });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregation::VolumePoint;
    use ark_bn254::Fr;
    use sss::Share;

    #[test]
    fn test_clearing_price_none_for_empty() {
        let result = compute_clearing_price(&[], &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_clearing_price_basic() {
        let bids = vec![VolumePoint::new(
            Share::new(1, Fr::from(100u64)),
            Share::new(1, Fr::from(50u64)),
        )];
        let asks = vec![VolumePoint::new(
            Share::new(1, Fr::from(100u64)),
            Share::new(1, Fr::from(30u64)),
        )];
        let cp = compute_clearing_price(&bids, &asks);
        assert!(cp.is_some());
    }
}
