# AI Agent Wallet on Solana

## Overview

The **AI Agent Wallet** is a prototype wallet system designed specifically for autonomous AI agents on the Solana blockchain. It enables AI agents to create wallets programmatically, sign transactions automatically, hold SOL and SPL tokens, and interact with dApps and DeFi protocols—all without human intervention.

This project addresses the growing need for autonomous agent participation in the Solana ecosystem, providing secure, sandboxed wallet operations that separate agent decision logic from wallet security.

## Features

### Core Wallet
- **Programmatic Wallet Creation**: Generate new Solana keypairs with encrypted storage
- **Secure Key Management**: AES-GCM encryption for private keys, never exposed to agent logic
- **Automated Transaction Signing**: Sign and send transactions without manual input
- **SOL & SPL Token Support**: Full token operations (transfer, mint, burn)
- **Multi-Wallet Management**: Handle multiple agent wallets simultaneously

### Agent Integration
- **Clear Separation**: Agent decision logic isolated from wallet operations
- **Flexible Agent Interface**: Support for deterministic, scripted, and LLM-powered agents
- **Transaction Builder**: Convert agent decisions into Solana instructions
- **Sandboxed Execution**: Safe execution environment with permission boundaries

### dApp Interaction
- **Protocol Integration**: Built-in clients for Raydium, Orca, and custom programs
- **Test Program Support**: Example counter program for development and testing
- **Transaction Simulation**: Simulate transactions before signing to prevent failures
- **Error Handling & Retry**: Robust error recovery and transaction retry logic

### Monitoring & Security
- **Command Line Interface**: Full-featured CLI for wallet and agent management
- **Real-time Monitoring**: WebSocket stream for transaction events and balance updates
- **Security Features**: Transaction validation, rate limiting, spending caps
- **Observability**: Structured logging, metrics collection, health checks

## Quick Start

### Prerequisites
- **Rust**: 1.70+ (install via [rustup](https://rustup.rs/))
- **Solana CLI**: 1.17+ (install via [solana-install](https://docs.solana.com/cli/install-solana-cli-tools))
- **Node.js** (optional, for web dashboard): 18+
- **Docker** (optional): 24+

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/agent-wallet.git
cd agent-wallet

# Install Rust dependencies
cargo build --release

# Install Solana CLI (if not already installed)
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Configure for devnet
solana config set --url https://api.devnet.solana.com
```

### Creating Your First Agent Wallet

```bash
# Build the CLI tool
cargo build --release --bin agent-wallet-cli

# Create a new wallet
./target/release/agent-wallet-cli create --name trading-agent --output wallets/trader.json

# The CLI will prompt for an encryption passphrase
# Wallet created: wallets/trader.json (encrypted)
```

### Running a Simple Agent

Create a basic agent script:

```rust
// examples/simple_agent.rs
use agent_wallet::{Wallet, AgentAction, WalletConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load wallet
    let config = WalletConfig::new()
        .with_rpc_url("https://api.devnet.solana.com");
    
    let mut wallet = Wallet::load("wallets/trader.json", config)?;
    
    // Simple agent logic: transfer 0.1 SOL every hour
    let action = AgentAction::TransferSol {
        recipient: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".parse()?,
        amount_sol: 0.1,
        memo: Some("Hourly transfer".to_string()),
    };
    
    // Execute action
    let signature = wallet.execute(action).await?;
    println!("Transaction sent: {}", signature);
    
    Ok(())
}
```

Run the agent:

```bash
cargo run --example simple_agent
```

## Project Structure

```
agent-wallet/
├── Cargo.toml                    # Workspace configuration
├── README.md                     # This file
├── SKILLS.md                     # Capabilities for AI agents
├── ROADMAP.md                    # Development roadmap
├── core/                         # Wallet core library
│   ├── src/
│   │   ├── wallet.rs            # Wallet struct, encryption, persistence
│   │   ├── transaction_builder.rs
│   │   ├── rpc_client.rs
│   │   └── lib.rs
│   └── tests/
├── agent/                        # Agent framework
│   ├── src/
│   │   ├── agent.rs             # Agent trait and implementations
│   │   ├── llm_agent.rs         # Optional LLM integration
│   │   └── lib.rs
│   └── examples/
├── dapp/                         # dApp interaction clients
│   ├── counter_client.rs        # Test program client
│   └── raydium_client.rs        # Raydium swap client
├── cli/                          # Command-line interface
│   ├── src/
│   └── Cargo.toml
├── web-dashboard/                # Optional React frontend
│   ├── package.json
│   ├── src/
│   └── public/
├── tests/                        # Integration tests
│   ├── integration.rs
│   └── test_utils.rs
└── scripts/                      # Deployment scripts
    ├── deploy-test-program.sh
    └── start-devnet.sh
```

## Usage Examples

### Basic SOL Transfer

```rust
use agent_wallet::{Wallet, AgentAction};

let mut wallet = Wallet::load("path/to/wallet.json", config)?;

let action = AgentAction::TransferSol {
    recipient: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".parse()?,
    amount_sol: 1.5,
    memo: Some("Payment for services".to_string()),
};

let signature = wallet.execute(action).await?;
```

### SPL Token Operations

```rust
use agent_wallet::{Wallet, AgentAction, TokenMint};

let action = AgentAction::TransferSplToken {
    mint: TokenMint::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?, // USDC
    recipient: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".parse()?,
    amount: 100_000_000, // 100 USDC (6 decimals)
};
```

### DeFi Swap via Raydium

```rust
use agent_wallet::{Wallet, AgentAction, SwapParams};

let action = AgentAction::SwapTokens {
    params: SwapParams {
        input_token: "SOL".to_string(),
        output_token: "USDC".to_string(),
        amount: 0.5,
        slippage_bps: 50, // 0.5%
    },
};
```

### Custom Program Interaction

```rust
use agent_wallet::{Wallet, AgentAction, ProgramCall};
use solana_sdk::instruction::Instruction;

let instruction = Instruction::new_with_bytes(
    program_id,
    &[1, 2, 3, 4], // Instruction data
    vec![], // Accounts
);

let action = AgentAction::CustomProgramCall {
    instruction,
};
```

## CLI Reference

### Wallet Management
```bash
# Create a new wallet
agent-wallet-cli create --name my-agent --output wallet.json

# List all wallets
agent-wallet-cli list

# Show wallet balance
agent-wallet-cli balance --wallet wallet.json

# Import wallet from private key
agent-wallet-cli import --private-key [key] --output wallet.json
```

### Agent Control
```bash
# Run a deterministic agent
agent-wallet-cli agent run --type periodic --wallet wallet.json --interval 3600

# Run LLM agent (requires OpenAI API key)
agent-wallet-cli agent run --type llm --wallet wallet.json --prompt "Trade based on market conditions"

# List running agents
agent-wallet-cli agent list

# Stop an agent
agent-wallet-cli agent stop --id agent-123
```

### Transaction Operations
```bash
# Send SOL
agent-wallet-cli transfer --wallet wallet.json --to <address> --amount 1.0

# Check transaction status
agent-wallet-cli transaction status --signature <signature>

# Get transaction history
agent-wallet-cli transaction history --wallet wallet.json --limit 10
```

## Configuration

### Environment Variables
```bash
# Required for production
export SOLANA_RPC_URL="https://api.mainnet-beta.solana.com"
export WALLET_ENCRYPTION_PASSPHRASE="your-secure-passphrase"

# Optional for LLM agents
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."

# Optional for monitoring
export PROMETHEUS_ENDPOINT="http://localhost:9090"
export LOKI_ENDPOINT="http://localhost:3100"
```

### Configuration File
Create `config.yaml`:
```yaml
wallet:
  rpc_url: "https://api.devnet.solana.com"
  commitment_level: "confirmed"
  timeout_seconds: 30

agent:
  permission_level: "advanced"
  daily_spend_limit_sol: 10.0
  allowed_protocols: ["raydium", "orca"]
  
security:
  require_transaction_simulation: true
  max_slippage_percent: 1.0
  blacklist_check: true
  
logging:
  level: "info"
  format: "json"
  output: "stdout"
```

## Security Considerations

### Key Management
- Private keys are encrypted using AES-GCM with a passphrase-derived key
- Keys are never stored in plaintext or logged
- Memory is zeroized after use to prevent leaks
- Optional hardware security module (HSM) support

### Agent Sandboxing
- Agent code runs in isolated execution environment
- Limited filesystem and network access
- Transaction validation before signing
- Rate limiting and spending caps

### Best Practices
1. **Use unique passphrases** for each wallet
2. **Regularly rotate encryption keys** for long-lived agents
3. **Implement agent-specific spending limits**
4. **Monitor agent activity** for unusual patterns
5. **Keep wallet software updated** with security patches

## Testing

### Unit Tests
```bash
cargo test --lib
```

### Integration Tests
```bash
# Start local test validator
solana-test-validator

# Run integration tests
cargo test --test integration
```

### End-to-End Tests
```bash
# Deploy test program to devnet
./scripts/deploy-test-program.sh

# Run e2e tests
cargo test --test e2e -- --test-threads=1
```

## Deployment

### Docker
```bash
# Build Docker image
docker build -t agent-wallet .

# Run with Docker Compose
docker-compose up -d
```

### Kubernetes
```bash
# Deploy to Kubernetes
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
kubectl apply -f k8s/configmap.yaml
```

### Cloud Providers
- **AWS**: ECS/EKS with Parameter Store for secrets
- **GCP**: Cloud Run/GKE with Secret Manager
- **Azure**: AKS with Key Vault

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

1. **Fork the repository**
2. **Create a feature branch**
3. **Make your changes**
4. **Add tests** for new functionality
5. **Run the test suite** to ensure nothing breaks
6. **Submit a pull request**

### Development Setup
```bash
# Install development dependencies
cargo install cargo-watch
cargo install cargo-tarpaulin  # For coverage
cargo install cargo-audit       # For security audit

# Run tests with coverage
cargo tarpaulin --ignore-tests

# Run security audit
cargo audit
```

## Roadmap

See [ROADMAP.md](ROADMAP.md) for detailed development plans and future features.

### Upcoming Features
- Multi-chain support (Ethereum, Polygon via Wormhole)
- Advanced order types (limit orders, stop losses)
- Portfolio management and rebalancing
- Cross-agent communication protocols
- Formal verification of transaction safety

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **Solana Foundation** for the bounty program
- **Solana Labs** for the excellent Rust SDK
- **Pyth Network** for price feed integration
- **Raydium & Orca** for DeFi protocol inspiration

## Support

- **Documentation**: [docs.agent-wallet.org](https://docs.agent-wallet.org)
- **Discord**: [Join our community](https://discord.gg/agent-wallet)
- **GitHub Issues**: [Report bugs & request features](https://github.com/yourusername/agent-wallet/issues)
- **Security Issues**: security@agent-wallet.org

---

*Built with ❤️ for the Solana AI agent ecosystem*