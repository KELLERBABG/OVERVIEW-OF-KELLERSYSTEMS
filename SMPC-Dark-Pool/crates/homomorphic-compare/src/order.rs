use sss::Share;

#[derive(Debug, Clone)]
pub struct Order {
    pub price: Share,
    pub quantity: Share,
    pub timestamp: Share,
    pub party_id: u64,
}

impl Order {
    pub fn new(price: Share, quantity: Share, timestamp: Share, party_id: u64) -> Self {
        Self {
            price,
            quantity,
            timestamp,
            party_id,
        }
    }
}
