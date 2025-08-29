# Stealth Swap Backend

This repository contains the Rust implementation of the Stealth Swap backend daemon (`stealth-swapd`) that acts as **Bob (liquidity maker)** for Solana-XMR atomic swaps, with optional relayer functionality.

## Quick Start

### Prerequisites
- Rust 1.72+ 
- Docker & Docker Compose (optional)

### Development Setup

```bash
# Clone the repository
git clone <repository-url>
cd stealh-swap-backend

# Copy configuration
cp config.example.yaml config.yaml

# For local development against devnet
export MONERO_WALLET_PASSWORD="your-monero-wallet-password"

# Build and run
cargo run --bin stealth-swapd
```

### Docker Setup (Recommended)

```bash
# Start with Docker Compose
docker-compose up --build

# This will start:
# - Monerod daemon
# - Monero wallet RPC
# - Stealth Swap backend
# - Prometheus + Grafana for monitoring
```

### API Endpoints

- **POST /v1/quote** - Generate a swap quote
- **POST /v1/swap/accept** - Accept a quote and lock funds
- **GET /v1/swap/:swap_id** - Get swap status
- **GET /health** - Health check endpoint
- **GET /metrics** - Prometheus metrics

## Environment Configuration

Required environment variables:
- `MONERO_WALLET_PASSWORD` - Monero wallet password
- `STEALTH_SWAP_CONFIG` - Path to configuration file (default: ./config.yaml)

## Project Structure

```
├── stealth-swapd/              # Main backend binary
│   ├── src/
│   │   ├── config.rs           # Configuration management
│   │   ├── clients/            # Blockchain clients
│   │   │   ├── solana.rs       # Solana RPC client
│   │   │   └── monero.rs       # Monero wallet client
│   │   ├── swap_engine.rs      # Core swap logic with state machine
│   │   ├── api.rs              # HTTP API endpoints
│   │   ├── metrics.rs          # Prometheus metrics
│   │   └── security.rs         # Cryptography and secrets management
│   └── Cargo.toml
├── solana-program/             # On-chain Solana program
├── config.example.yaml         # Configuration template
├── docker-compose.yml          # Docker services
├── prometheus.yml             # Metrics configuration
└── alerts.yml                 # Prometheus alerting rules
```

## Architecture

The backend implements a state machine with the following states:
- **Quoted** - Generated quote awaiting acceptance
- **LockedUSDC** - USDC locked on Solana
- **LockedXMR** - XMR locked on Monero  
- **Redeemed** - Swap completed successfully
- **Refunded** - Swap expired/refunded
- **Failed** - Swap failed with error

## Security Features

- Private keys never leave memory unencrypted (using `secrecy` crate)
- Encrypted secrets storage at rest
- Rate limiting on RPC calls with exponential backoff  
- SQLite database with encryption via SQLCipher
- Non-root UID 1000 in Docker container
- Signed container images with Sigstore

## Metrics & Monitoring

The daemon exports Prometheus metrics including:
- `stealth_swap_swaps_total` - Total swaps by direction/state
- `stealth_swap_swaps_duration_seconds` - Swap execution time
- `stealth_swap_monero_wallet_balance_xmr` - Monero wallet balance
- `stealth_swap_solana_wallet_balance_usdc` - Solana USDC balance
- `stealth_swap_relayer_fees_earned_usdc` - Relayer earnings

Access Grafana at http://localhost:3000 (admin/admin) for dashboards.

## Testing

```bash
# Run unit tests
cargo test --workspace

# Run integration with local instances
docker-compose exec stealth-swapd cargo test

# Check for security issues
cargo audit
cargo clippy -- -D warnings
```

## Production Checklist

- [ ] Generate and secure Monero wallet
- [ ] Obtain Solana signer keypair with funds
- [ ] Configure production RPC endpoints  
- [ ] Set up SSL/TLS certificates
- [ ] Configure firewall rules
- [ ] Set up monitoring alerts
- [ ] Test disaster recovery procedures

## License

Apache 2.0 - see LICENSE file for details.