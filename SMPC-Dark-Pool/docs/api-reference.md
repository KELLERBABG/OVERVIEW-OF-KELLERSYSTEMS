# SMPC Dark Pool — API Reference

**Base URL**: `http://<host>:8080` (or `https://<host>:8080` with TLS)

---

## Authentication

If the `API_KEY` environment variable is set, all POST and DELETE endpoints require the header:

```
X-API-Key: <your-api-key>
```

The `GET /api/health` endpoint is always unauthenticated.

---

## Rate Limiting

- **Limit**: 100 requests per minute per IP
- **Response on exceed**: `429 Too Many Requests`
- **Header**: `Retry-After: <seconds>`

---

## Endpoints

### GET /api/health

Health check for load balancers and monitoring.

**Response** `200 OK`:
```json
{
    "status": "ok",
    "version": "0.1.0",
    "timestamp": "2026-07-30T23:00:00Z"
}
```

---

### GET /api/metrics

Prometheus-formatted metrics for scraping.

**Response** `200 OK`:
```
# HELP orders_total Total orders submitted
# TYPE orders_total counter
orders_total 42
# HELP matches_total Total matches found
# TYPE matches_total counter
matches_total 17
# HELP active_connections Active WebSocket connections
# TYPE active_connections gauge
active_connections 5
# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",path="/api/health"} 100
http_requests_total{method="POST",path="/api/orders"} 42
```

---

### POST /api/orders

Submit a new order to the dark pool.

**Auth required**: Yes (if `API_KEY` is set)

**Request Body**:
```json
{
    "party_id": 12345,
    "side": "bid",
    "price": "1000000000000000000",
    "quantity": "500000000000000000"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `party_id` | integer | Trader identifier |
| `side` | string | `"bid"` or `"ask"` (case-insensitive) |
| `price` | string | Price in wei/base units (decimal string) |
| `quantity` | string | Quantity in base units (decimal string) |

**Response** `200 OK`:
```json
{
    "order_id": "42",
    "status": "open",
    "submitted_at": "2026-07-30T23:01:00Z"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `order_id` | string | Unique order identifier |
| `status` | string | `"open"` (no match) or `"matched"` (matched immediately) |
| `submitted_at` | string | RFC 3339 timestamp |

**Errors**:

| Status | Condition |
|--------|-----------|
| 400 | Invalid side, price, or quantity |
| 401 | Missing or invalid API key |
| 429 | Rate limit exceeded |

---

### GET /api/orders/{id}

Get the status of a specific order.

**Auth required**: No

**Response** `200 OK`:
```json
{
    "order_id": "42",
    "side": "bid",
    "status": "Settled",
    "matched": true,
    "timestamp": "2026-07-30T23:01:00Z"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `order_id` | string | Order identifier |
| `side` | string | `"bid"` or `"ask"` |
| `status` | string | `"Pending"`, `"Settled"`, or `"Failed(reason)"` |
| `matched` | boolean | Whether the order was matched |
| `timestamp` | string | RFC 3339 timestamp |

**Errors**:

| Status | Condition |
|--------|-----------|
| 404 | Order not found |

---

### GET /api/book

Get a blinded snapshot of the order book. Prices and quantities are shown as their secret-shared values (field elements), not as plaintext.

**Auth required**: No

**Response** `200 OK`:
```json
{
    "best_bid": "1000000000000000000",
    "best_ask": "1005000000000000000",
    "spread": "5000000000000000.00",
    "bid_depth": 12,
    "ask_depth": 8
}
```

| Field | Type | Description |
|-------|------|-------------|
| `best_bid` | string/null | Highest bid price (reconstructed, for display only) |
| `best_ask` | string/null | Lowest ask price |
| `spread` | string/null | Difference between best ask and best bid |
| `bid_depth` | integer | Number of bids on the book |
| `ask_depth` | integer | Number of asks on the book |

---

### GET /api/trades

Get the trade history.

**Auth required**: No

**Response** `200 OK`:
```json
[
    {
        "trade_id": "trade_1",
        "buy_order_id": "1",
        "sell_order_id": "2",
        "price": "1000000000000000000",
        "quantity": "500000000000000000",
        "timestamp": "2026-07-30T23:01:30Z"
    }
]
```

---

### WS /api/ws

WebSocket endpoint for real-time trade updates.

**Auth required**: No (WebSocket upgrade)

**Protocol**:
1. Client connects via WebSocket upgrade
2. Server sends JSON-encoded `TradeHistoryEntry` objects as trades occur
3. No client-to-server messages are processed (publish-only feed)

**Example message**:
```json
{
    "trade_id": "trade_1",
    "buy_order_id": "1",
    "sell_order_id": "2",
    "price": "1000000000000000000",
    "quantity": "500000000000000000",
    "timestamp": "2026-07-30T23:01:30Z"
}
```

---

## Error Responses

All errors return a JSON body:

```json
{
    "code": 400,
    "message": "invalid side: must be bid or ask"
}
```

| Status | Code | Description |
|--------|------|-------------|
| 400 | 400 | Bad request (invalid input) |
| 401 | 401 | Unauthorized (missing/invalid API key) |
| 404 | 404 | Not found |
| 429 | 429 | Rate limit exceeded |
| 500 | 500 | Internal server error |
