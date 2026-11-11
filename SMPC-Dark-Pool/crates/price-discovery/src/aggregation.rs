use ark_ff::Field;
use sss::Share;

#[derive(Debug, Clone)]
pub struct VolumePoint {
    pub price: Share,
    pub volume: Share,
}

impl VolumePoint {
    pub fn new(price: Share, volume: Share) -> Self {
        Self { price, volume }
    }
}

pub fn aggregate_volumes_by_price(
    bids: &[Share],
    asks: &[Share],
    granularity: u64,
) -> (Vec<VolumePoint>, Vec<VolumePoint>) {
    let max_price = 10000u64;
    let num_levels = (max_price / granularity) as usize;
    let mut bid_volumes = vec![ark_bn254::Fr::ZERO; num_levels];
    let mut ask_volumes = vec![ark_bn254::Fr::ZERO; num_levels];
    for bid in bids {
        let level = (bid.value.to_string().parse::<u64>().unwrap_or(0) / granularity) as usize;
        if level < num_levels {
            bid_volumes[level] += bid.value;
        }
    }
    for ask in asks {
        let level = (ask.value.to_string().parse::<u64>().unwrap_or(0) / granularity) as usize;
        if level < num_levels {
            ask_volumes[level] += ask.value;
        }
    }
    let bid_points: Vec<VolumePoint> = bid_volumes
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            VolumePoint::new(
                Share::new(1, ark_bn254::Fr::from((i as u64) * granularity)),
                Share::new(1, v),
            )
        })
        .collect();
    let ask_points: Vec<VolumePoint> = ask_volumes
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            VolumePoint::new(
                Share::new(1, ark_bn254::Fr::from((i as u64) * granularity)),
                Share::new(1, v),
            )
        })
        .collect();
    (bid_points, ask_points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use sss::Share;

    #[test]
    fn test_aggregation_structure() {
        let bids = vec![
            Share::new(1, Fr::from(100u64)),
            Share::new(2, Fr::from(101u64)),
        ];
        let asks = vec![Share::new(1, Fr::from(99u64))];
        let (bid_points, ask_points) = aggregate_volumes_by_price(&bids, &asks, 10);
        assert!(!bid_points.is_empty());
        assert!(!ask_points.is_empty());
    }
}
