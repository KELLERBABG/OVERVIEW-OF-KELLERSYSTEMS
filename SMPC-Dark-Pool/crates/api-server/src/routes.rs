use crate::state::{AppState, MatchMode};
use crate::types::{
    ApiError, BookSnapshot, OrderRequest, OrderResponse, OrderStatusResponse, TradeHistoryEntry,
};
use ark_bn254::Fr;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use matching_engine::types::{Match, Order, OrderSide};
use sss::Share;
use std::sync::Arc;
use std::sync::atomic::Ordering;

pub async fn submit_order(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OrderRequest>,
) -> Result<Json<OrderResponse>, (StatusCode, Json<ApiError>)> {
    let side = match req.side.to_lowercase().as_str() {
        "bid" | "buy" => OrderSide::Bid,
        "ask" | "sell" => OrderSide::Ask,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    code: 400,
                    message: "invalid side: must be bid or ask".into(),
                }),
            ));
        }
    };
    let price_val: u64 = req.price.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: 400,
                message: "invalid price".into(),
            }),
        )
    })?;
    let qty_val: u64 = req.quantity.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: 400,
                message: "invalid quantity".into(),
            }),
        )
    })?;
    let order = Order::new(
        rand::random::<u64>(),
        req.party_id,
        side,
        Share::new(1, Fr::from(price_val)),
        Share::new(1, Fr::from(qty_val)),
        Share::new(
            1,
            Fr::from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
        ),
    );
    state.orders_total.fetch_add(1, Ordering::Relaxed);
    let mut engine = state.engine.write().await;
    // In periodic (deferred) mode orders accumulate in the book and are matched
    // by the daemon's scheduled cycle. Otherwise match immediately.
    let matches = if matches!(state.match_mode, MatchMode::Periodic { .. }) {
        engine.add_order(order.clone());
        Vec::new()
    } else {
        engine.submit_order(order.clone()).unwrap()
    };
    if !matches.is_empty() {
        persist_matches(&state, &matches).await;
    }
    Ok(Json(OrderResponse {
        order_id: order.id.to_string(),
        status: if matches.is_empty() {
            "open".into()
        } else {
            "matched".into()
        },
        submitted_at: chrono::Utc::now().to_rfc3339(),
    }))
}

/// Persist a batch of matches into trade history, settlement, and WS broadcast.
///
/// Shared by the immediate submit path and the daemon's periodic matcher so both
/// produce identical fills regardless of which execution model is active.
pub async fn persist_matches(state: &Arc<AppState>, matches: &[Match]) {
    if matches.is_empty() {
        return;
    }
    state
        .matches_total
        .fetch_add(matches.len() as u64, Ordering::Relaxed);
    let mut trades = state.trades.write().await;
    let mut settlement = state.settlement.write().await;
    let mut last: Option<TradeHistoryEntry> = None;
    for m in matches {
        let proof = settlement::types::SettlementProof::new(
            m.buy_order_id,
            &[0u8; 32],
            &[0u8; 64],
            m.buy_order_id,
            m.sell_order_id,
            &m.price.value.to_string().into_bytes(),
            &m.quantity.value.to_string().into_bytes(),
        );
        let _ = settlement.submit_proof(proof);
        let entry = TradeHistoryEntry {
            trade_id: format!("trade_{}", m.buy_order_id),
            buy_order_id: m.buy_order_id.to_string(),
            sell_order_id: m.sell_order_id.to_string(),
            price: m.price.value.to_string(),
            quantity: m.quantity.value.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        last = Some(entry.clone());
        trades.push(entry);
    }
    if let Some(msg) = last.map(|t| serde_json::to_string(&t).unwrap()) {
        let clients = state.ws_clients.read().await;
        for tx in clients.iter() {
            let _ = tx.send(msg.clone());
        }
    }
}

pub async fn get_order(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> Result<Json<OrderStatusResponse>, (StatusCode, Json<ApiError>)> {
    let settlement = state.settlement.read().await;
    let status = settlement
        .get_status(id)
        .unwrap_or(settlement::types::SettlementStatus::Pending);
    Ok(Json(OrderStatusResponse {
        order_id: id.to_string(),
        side: "unknown".into(),
        status: format!("{:?}", status),
        matched: status == settlement::types::SettlementStatus::Settled,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn get_book(State(state): State<Arc<AppState>>) -> Json<BookSnapshot> {
    let engine = state.engine.read().await;
    let best_bid = engine.book.best_bid().map(|b| b.price.value.to_string());
    let best_ask = engine.book.best_ask().map(|a| a.price.value.to_string());
    let spread = match (best_bid.clone(), best_ask.clone()) {
        (Some(bid), Some(ask)) => {
            let b: f64 = bid.parse().unwrap_or(0.0);
            let a: f64 = ask.parse().unwrap_or(0.0);
            Some(format!("{:.2}", a - b))
        }
        _ => None,
    };
    Json(BookSnapshot {
        best_bid,
        best_ask,
        spread,
        bid_depth: engine.book.bids.len(),
        ask_depth: engine.book.asks.len(),
    })
}

pub async fn get_trades(State(state): State<Arc<AppState>>) -> Json<Vec<TradeHistoryEntry>> {
    let trades = state.trades.read().await;
    Json(trades.clone())
}

pub async fn metrics(State(state): State<Arc<AppState>>) -> (StatusCode, String) {
    let orders = state.orders_total.load(Ordering::Relaxed);
    let matches = state.matches_total.load(Ordering::Relaxed);
    let connections = state.active_connections.load(Ordering::Relaxed);
    let requests = state.http_requests_total.load(Ordering::Relaxed);
    let body = format!(
        "# HELP orders_total Total orders submitted\n\
         # TYPE orders_total counter\n\
         orders_total {orders}\n\
         # HELP matches_total Total matches found\n\
         # TYPE matches_total counter\n\
         matches_total {matches}\n\
         # HELP active_connections Active WebSocket connections\n\
         # TYPE active_connections gauge\n\
         active_connections {connections}\n\
         # HELP http_requests_total Total HTTP requests\n\
         # TYPE http_requests_total counter\n\
         http_requests_total {requests}\n"
    );
    (StatusCode::OK, body)
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
