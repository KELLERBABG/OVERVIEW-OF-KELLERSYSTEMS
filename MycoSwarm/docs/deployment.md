# Deployment Guide

## Signaling Server (Docker)

```bash
# Build and start
docker compose up -d

# Or build manually
docker build -t fungal-signaling-server .
docker run -d -p 8765:8765 fungal-signaling-server
```

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | 8765 | WebSocket server port |
| `MAX_PEERS_PER_ROOM` | 50 | Max nodes per room |
| `LOG_LEVEL` | info | Logging verbosity |
| `TURN_URLS` | (empty) | Comma-separated TURN server URLs |

## Client Deployment

```bash
cd packages/client
pnpm build --outDir dist
# Serve dist/ with any static file server
```

## Smart Contracts

```bash
cd packages/incentive-token
# Install Hardhat and deploy
npx hardhat run scripts/deploy.ts --network arbitrum-sepolia
```

## STUN/TURN Configuration

Default STUN: `stun:stun.l.google.com:19302`

For production, deploy a TURN server:
```bash
docker run -d -p 3478:3478 -p 3478:3478/udp \
  -e TURN_USERNAME=user -e TURN_PASSWORD=pass \
  coturn/coturn