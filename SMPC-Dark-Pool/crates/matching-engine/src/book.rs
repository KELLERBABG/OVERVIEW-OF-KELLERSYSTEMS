use crate::types::{Order, OrderSide};
use homomorphic_compare::order::Order as HcOrder;
use homomorphic_compare::priority::compare_price_time;
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct OrderBook {
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    pub fn add_order(&mut self, order: Order) {
        match order.side {
            OrderSide::Bid => {
                self.bids.push(order);
                self.bids.sort_by(|a, b| {
                    let ha = HcOrder::new(
                        a.price.clone(),
                        a.quantity.clone(),
                        a.timestamp.clone(),
                        a.party_id,
                    );
                    let hb = HcOrder::new(
                        b.price.clone(),
                        b.quantity.clone(),
                        b.timestamp.clone(),
                        b.party_id,
                    );
                    compare_price_time(&hb, &ha)
                });
            }
            OrderSide::Ask => {
                self.asks.push(order);
                self.asks.sort_by(|a, b| {
                    let ha = HcOrder::new(
                        a.price.clone(),
                        a.quantity.clone(),
                        a.timestamp.clone(),
                        a.party_id,
                    );
                    let hb = HcOrder::new(
                        b.price.clone(),
                        b.quantity.clone(),
                        b.timestamp.clone(),
                        b.party_id,
                    );
                    compare_price_time(&ha, &hb)
                });
            }
        }
    }

    pub fn remove_order(&mut self, order_id: u64) -> Option<Order> {
        if let Some(pos) = self.bids.iter().position(|o| o.id == order_id) {
            Some(self.bids.remove(pos))
        } else if let Some(pos) = self.asks.iter().position(|o| o.id == order_id) {
            Some(self.asks.remove(pos))
        } else {
            None
        }
    }

    pub fn best_bid(&self) -> Option<&Order> {
        self.bids.first()
    }

    pub fn best_ask(&self) -> Option<&Order> {
        self.asks.first()
    }

    pub fn is_crossed(&self) -> bool {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => {
                let bid_hc = HcOrder::new(
                    bid.price.clone(),
                    bid.quantity.clone(),
                    bid.timestamp.clone(),
                    bid.party_id,
                );
                let ask_hc = HcOrder::new(
                    ask.price.clone(),
                    ask.quantity.clone(),
                    ask.timestamp.clone(),
                    ask.party_id,
                );
                compare_price_time(&bid_hc, &ask_hc) != Ordering::Less
            }
            _ => false,
        }
    }

    pub fn len(&self) -> usize {
        self.bids.len() + self.asks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bids.is_empty() && self.asks.is_empty()
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use sss::Share;

    fn make_bid(id: u64, price: u64, qty: u64, ts: u64) -> Order {
        Order::new(
            id,
            id,
            OrderSide::Bid,
            Share::new(1, Fr::from(price)),
            Share::new(1, Fr::from(qty)),
            Share::new(1, Fr::from(ts)),
        )
    }

    fn make_ask(id: u64, price: u64, qty: u64, ts: u64) -> Order {
        Order::new(
            id,
            id,
            OrderSide::Ask,
            Share::new(1, Fr::from(price)),
            Share::new(1, Fr::from(qty)),
            Share::new(1, Fr::from(ts)),
        )
    }

    #[test]
    fn test_add_and_best() {
        let mut book = OrderBook::new();
        book.add_order(make_bid(1, 100, 10, 1));
        book.add_order(make_bid(2, 110, 5, 2));
        assert_eq!(book.best_bid().unwrap().id, 2);
    }

    #[test]
    fn test_remove_order() {
        let mut book = OrderBook::new();
        book.add_order(make_bid(1, 100, 10, 1));
        assert!(book.remove_order(1).is_some());
        assert!(book.best_bid().is_none());
    }

    #[test]
    fn test_is_crossed() {
        let mut book = OrderBook::new();
        book.add_order(make_bid(1, 110, 10, 1));
        book.add_order(make_ask(2, 100, 10, 2));
        assert!(book.is_crossed());
    }

    #[test]
    fn test_not_crossed() {
        let mut book = OrderBook::new();
        book.add_order(make_bid(1, 90, 10, 1));
        book.add_order(make_ask(2, 100, 10, 2));
        assert!(!book.is_crossed());
    }
}
