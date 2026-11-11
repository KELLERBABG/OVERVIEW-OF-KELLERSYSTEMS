use crate::book::OrderBook;
use crate::types::{Match, MatchError, Order};
use smpc_engine::compare::secure_min;
use smpc_engine::types::SecretShare;

#[derive(Debug, Clone)]
pub struct MatchingEngine {
    pub book: OrderBook,
    pub matches: Vec<Match>,
}

impl MatchingEngine {
    pub fn new() -> Self {
        Self {
            book: OrderBook::new(),
            matches: Vec::new(),
        }
    }

    pub fn submit_order(&mut self, order: Order) -> Result<Vec<Match>, MatchError> {
        self.book.add_order(order);
        self.process_matches()
    }

    /// Defer an order into the book without running the match loop.
    ///
    /// Used by periodic (batched) matching mode: orders accumulate here and are
    /// run through [`MatchingEngine::process_matches`] on the scheduled cycle.
    pub fn add_order(&mut self, order: Order) {
        self.book.add_order(order);
    }

    pub fn cancel_order(&mut self, order_id: u64) -> Result<(), MatchError> {
        self.book
            .remove_order(order_id)
            .ok_or(MatchError::OrderNotFound(order_id))?;
        Ok(())
    }

    pub fn process_matches(&mut self) -> Result<Vec<Match>, MatchError> {
        let mut new_matches = Vec::new();
        while self.book.is_crossed() {
            let bid = self.book.bids.first().unwrap().clone();
            let ask = self.book.asks.first().unwrap().clone();
            let bid_qty = SecretShare::new(bid.quantity.clone());
            let ask_qty = SecretShare::new(ask.quantity.clone());
            let match_qty = secure_min(&bid_qty, &ask_qty).share.clone();
            let m = Match::new(
                bid.id,
                ask.id,
                bid.price.clone(),
                match_qty,
                bid.timestamp.clone(),
            );
            new_matches.push(m.clone());
            self.matches.push(m);
            self.book.remove_order(bid.id);
            self.book.remove_order(ask.id);
        }
        Ok(new_matches)
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Consume the accumulated match buffer after a periodic match cycle has
    /// persisted its results.
    ///
    /// In the default immediate model the buffer is drained as matches occur, so
    /// this makes periodic matching idempotent: after `persist_matches` runs on a
    /// batch, `clear_pending` resets the accumulator so a later cycle will not
    /// re-emit already-persisted fills.
    pub fn clear_pending(&mut self) {
        self.matches.clear();
    }
}

impl Default for MatchingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OrderSide;
    use ark_bn254::Fr;
    use sss::Share;

    fn make_order(id: u64, side: OrderSide, price: u64, qty: u64, ts: u64) -> Order {
        Order::new(
            id,
            id,
            side,
            Share::new(1, Fr::from(price)),
            Share::new(1, Fr::from(qty)),
            Share::new(1, Fr::from(ts)),
        )
    }

    #[test]
    fn test_simple_match() {
        let mut engine = MatchingEngine::new();
        let bid = make_order(1, OrderSide::Bid, 100, 10, 1);
        let ask = make_order(2, OrderSide::Ask, 90, 10, 2);
        let matches = engine.submit_order(bid).unwrap();
        assert!(matches.is_empty());
        let matches = engine.submit_order(ask).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].buy_order_id, 1);
        assert_eq!(matches[0].sell_order_id, 2);
    }

    #[test]
    fn test_no_match_low_bid() {
        let mut engine = MatchingEngine::new();
        let bid = make_order(1, OrderSide::Bid, 80, 10, 1);
        let ask = make_order(2, OrderSide::Ask, 100, 10, 2);
        let _ = engine.submit_order(bid).unwrap();
        let matches = engine.submit_order(ask).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_multiple_matches() {
        let mut engine = MatchingEngine::new();
        engine
            .submit_order(make_order(1, OrderSide::Bid, 100, 5, 1))
            .unwrap();
        engine
            .submit_order(make_order(2, OrderSide::Bid, 110, 10, 2))
            .unwrap();
        engine
            .submit_order(make_order(3, OrderSide::Ask, 90, 8, 3))
            .unwrap();
        engine
            .submit_order(make_order(4, OrderSide::Ask, 95, 7, 4))
            .unwrap();
        assert_eq!(engine.match_count(), 2);
    }
}
