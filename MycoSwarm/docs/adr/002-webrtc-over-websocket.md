# ADR-002: WebRTC for P2P Data Channels

## Status
Accepted

## Context
Browser nodes need to exchange messages directly without a central server. Options: WebSocket (requires server relay) or WebRTC (direct browser-to-browser).

## Decision
Use WebRTC with a lightweight signaling server only for initial handshake. Once connected, data flows directly between peers via encrypted data channels.

## Consequences
- True P2P communication with low latency
- Signaling server is stateless and ephemeral
- Requires STUN/TURN for NAT traversal
- Browser support is excellent (all modern browsers)