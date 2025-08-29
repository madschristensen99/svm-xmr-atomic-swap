# Stealth-Swap Backend

A headless daemon that acts as **Bob (liquidity maker)** for the Solana-XMR atomic swap protocol.

## Features

- **Solana-XMR Atomic Swaps**: Implement both directions (USDC → XMR and XMR → USDC)
- **Monero Integration**: Connect to Monero wallet RPC for XMR operations
- **Solana Program Integration**: Use Anchor client for program interactions
- **REST API**: Minimal HTTP+JSON API for quotes, swaps, and status
- **Monitoring**: Prometheus metrics and health checks
- **Security**: Encrypted key storage, audit logging, rate limiting
- **Relayer**: Optional transaction relaying with fee recovery

## Quick Start

### Prerequisites

- Current stable Rust toolchain
- Monero wallet RPC endpoint
- Solana with anchor CLI tools

### Installation

```bash
# Clone the repository
git clone https://github.com/org/stealth-swap-backend.git
cd stealth-swap-backend

# Copy configuration
cp config.example.yaml config.yaml

# Install dependencies
cargo build --release
```

### Configuration

1. **Copy configuration**: 
   ```bash
   cp config.example.yaml config.yaml
   ```

2. **Set environment variables**:
   ```bash
   export MONERO_WALLET_PASSWORD="your-secure-password"
   export RUST_LOG="stealth_swap=info"
   ```

3. **Configure services**: Edit `config.yaml` for your setup

### Running Locally

```bash
# Development mode
cargo run

# Run with specific config
cargo run -- --config /path/to/config.yaml

# Only run migrations
cargo run -- --migrate-only

# Print configuration and exit
cargo run -- --print-config
```

### Docker

```bash
# Build
docker build -t stealth-swapd .

# Run
docker run -e MONERO_WALLET_PASSWORD=yourpassword -p 3000:3000 stealth-swapd
```

## API Endpoints

### Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/quote` | POST | Generate a swap quote |
| `/v1/swap/accept` | POST | Accept a quote and create a swap |
| `/v1/swap/:id` | GET | Get swap status |
| `/health` | GET | Health check |
| `/metrics` | GET | Prometheus metrics |

### Example Usage

```bash
# Generate quote
curl -X POST http://localhost:3000/v1/quote \
  -H "Content-Type: application/json" \
  -d '{"direction":"usdc_to_xmr","usdc_amount":100000000,"xmr_amount":1000000000000}'

# Accept quote
curl -X POST http://localhost:3000/v1/swap/accept \
  -H "Content-Type: application/json" \
  -d '{"quote_id":"your-quote-id","counterparty_pubkey":"alice_pubkey"}'

# Check status
curl http://localhost:3000/v1/swap/YOUR_SWAP_ID_HEX
```

## Configuration

### Key Settings

```yaml
solana:
  rpc_url: "https://api.mainnet-beta.solana.com"
  keypair_path: "/secrets/bob.json"
  usdc_mint: "_EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"

monero:
  wallet_rpc_url: "http://localhost:18083"
  wallet_file: "bob_swap"
  password_env: "MONERO_WALLET_PASSWORD"

quoting:
  min_usdc: 100_000_000  # 100 USDC
  max_usdc: 10_000_000_000  # 10,000 USDC

relayer:
  enabled: true
  fee_bps: 10  # 0.1%
```

## Security

- **Private Keys**: Never leave memory unencrypted
- **Adaptor Secrets**: Encrypted with age format
- **Rate Limiting**: RPC calls with exponential backoff
- **Audit Logging**: All significant events logged
- **Container Security**: Runs as non-root in minimal image

## Development

### Build Requirements

```bash
# Install dependencies and build
cargo build --release --target x86_64-unknown-linux-musl

# Run tests
cargo test --workspace

# Security audit
cargo-audit audit
cargo-deny check
cargo-geiger
```

### Environment Setup

1. **Local Monero wallet**:
   ```bash
   # Install monero wallet RPC
   monero-wallet-rpc --testnet --rpc-bind-port 18083 --password ''
   ```

2. **Local Solana devnet**:
   ```bash
   # Setup devnet
   solana config set --url localhost
   solana keygen new --outfile /secrets/bob.json
   ```

### Contributing

1. Fork the repository
2. Create feature branch
3. Add tests for new functionality
4. Run full test suite
5. Submit pull request

## Monitoring

### Prometheus Metrics

- `swaps_total{direction,state}`: Total swaps
- `swaps_duration_seconds{direction,state}`: Swap duration
- `monero_wallet_balance_xmr`: XMR balance
- `solana_wallet_balance_usdc`: USDC balance
- `relayer_fees_earned_usdc`: Cumulative fees

### Health Monitoring

```bash
# Check health
curl http://localhost:3000/health

# Get metrics
curl http://localhost:3000/metrics
```

## Troubleshooting

### Common Issues

1. **Monero RPC connectivity**:
   ```bash
   # Test Monero connection
   curl -X POST http://localhost:18083/json_rpc \
     -d '{"jsonrpc":"2.0","id":"1","method":"get_version"}'
   ```

2. **Solana keypair issues**:
   ```bash
   # Check keypair
   solana-keygen pubkey /secrets/bob.json
   ```

3. **Database permissions**:
   ```bash
   # Fix database permissions
   chown -R 1000:1000 ./data
   ```

## License

MIT License