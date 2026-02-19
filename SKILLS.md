# SKILLS.md - AI Agent Wallet Capabilities

## Overview

The AI Agent Wallet is a Solana wallet designed specifically for autonomous AI agents. It provides secure, programmatic access to blockchain operations while maintaining strict separation between agent decision logic and wallet security. This document describes the wallet's capabilities from an AI agent's perspective.

## Core Capabilities

### 1. Wallet Management
- **Create New Wallet**: Generate a new Solana keypair, encrypt it, and persist it securely
- **Load Existing Wallet**: Decrypt and load a wallet from secure storage using a passphrase
- **Wallet Recovery**: Restore wallet from mnemonic phrase or seed (when supported)
- **Multiple Wallet Support**: Manage multiple wallets simultaneously with distinct identifiers

### 2. Account & Balance Operations
- **SOL Balance Query**: Check SOL balance for any wallet or address
- **SPL Token Balance Query**: Check balances of SPL tokens
- **Account Information**: Retrieve account data, rent exemption status, and program ownership
- **Transaction History**: Get recent transactions and their statuses

### 3. Transaction Execution
- **SOL Transfers**: Send SOL to any Solana address with optional memo
- **SPL Token Transfers**: Send SPL tokens to other addresses
- **Token Account Creation**: Create associated token accounts for SPL tokens
- **Token Minting**: Mint new tokens (requires mint authority)
- **Token Burning**: Burn tokens to reduce supply

### 4. dApp & Protocol Interaction
- **Program Invocation**: Call any Solana program with custom instruction data
- **DeFi Operations**:
  - Swap tokens via Raydium or Orca
  - Add/remove liquidity from pools
  - Stake tokens in liquidity pools
- **Governance Participation**:
  - Vote with governance tokens
  - Create governance proposals
- **Staking Operations**:
  - Delegate SOL to validators
  - Withdraw stakes and rewards

### 5. Advanced Operations
- **Multisig Wallet Management**: Create and interact with multisignature wallets
- **Cross-Program Invocations**: Coordinate multiple program calls in single transaction
- **Priority Fee Calculation**: Automatically adjust fees based on network congestion
- **Transaction Simulation**: Simulate transactions before signing to estimate outcomes

## Agent-Wallet Interface

### Integration Patterns

#### Pattern 1: Direct Library Integration (Recommended for Rust agents)
```rust
use agent_wallet::{Wallet, AgentAction, TransactionBuilder, WalletConfig};

// Initialize wallet
let config = WalletConfig::new()
    .with_encryption_passphrase("secure-passphrase")
    .with_rpc_url("https://api.devnet.solana.com");
    
let mut wallet = Wallet::load_or_create("wallet.json", config)?;

// Agent decision logic
let action = AgentAction::TransferSol {
    recipient: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".parse()?,
    amount_sol: 0.5,
    memo: Some("Payment for services".to_string()),
};

// Execute action
let signature = wallet.execute(action).await?;
println!("Transaction sent: {}", signature);
```

#### Pattern 2: REST API (For non-Rust agents)
```python
import requests
import json

# Initialize session with wallet service
session = requests.Session()
session.headers.update({"Authorization": "Bearer agent-token"})

# Create action
action = {
    "type": "transfer_sol",
    "recipient": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
    "amount": 0.5,
    "memo": "Payment for services"
}

# Execute action
response = session.post(
    "http://localhost:8080/api/v1/execute",
    json={"action": action}
)
result = response.json()
print(f"Transaction: {result['signature']}")
```

#### Pattern 3: WebSocket Stream (For real-time agents)
```javascript
// Connect to wallet WebSocket
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    if (data.type === 'balance_update') {
        console.log(`New balance: ${data.balance} SOL`);
    }
    if (data.type === 'transaction_confirmed') {
        console.log(`Transaction confirmed: ${data.signature}`);
    }
};

// Send action via WebSocket
ws.send(JSON.stringify({
    action: 'transfer_sol',
    params: { recipient: '...', amount: 0.1 }
}));
```

### Action Types

The wallet supports the following `AgentAction` types:

1. **TransferSol**: Transfer SOL to another address
2. **TransferSplToken**: Transfer SPL tokens
3. **CreateTokenAccount**: Create associated token account
4. **MintTokens**: Mint new SPL tokens
5. **SwapTokens**: Execute token swap via DEX
6. **AddLiquidity**: Add liquidity to pool
7. **RemoveLiquidity**: Remove liquidity from pool
8. **StakeTokens**: Stake tokens in protocol
9. **Vote**: Vote in governance proposal
10. **CustomProgramCall**: Call arbitrary Solana program
11. **BatchActions**: Execute multiple actions atomically

## Security Model

### Key Principles
1. **Private Key Isolation**: Agent logic never has direct access to private keys
2. **Transaction Validation**: All transactions are validated for safety before signing
3. **Sandboxed Execution**: Agent code runs in isolated environment with limited permissions
4. **Rate Limiting**: Automatic rate limiting to prevent abuse
5. **Spending Limits**: Configurable daily/weekly spending limits per agent

### Safety Checks
Before signing any transaction, the wallet performs:
- **Balance Verification**: Sufficient funds for transaction + fees
- **Destination Validation**: Recipient addresses are valid Solana addresses
- **Amount Sanity Checks**: Transfer amounts within reasonable bounds
- **Simulation**: Transaction simulation to catch potential failures
- **Blacklist Check**: Recipient not on known scam/malicious address list

### Permission Levels
Agents can be configured with different permission levels:

| Level | Capabilities | Typical Use Case |
|-------|-------------|------------------|
| **Read-Only** | Balance queries, transaction history | Monitoring agents |
| **Basic** | SOL transfers up to limit, token transfers | Simple payment agents |
| **Advanced** | All transfers, token operations | Trading agents |
| **Full** | All capabilities including program calls | DeFi interaction agents |

## Environment Setup

### Requirements
1. **Solana RPC Endpoint**: Devnet or mainnet RPC URL
2. **Wallet Service**: Running wallet service (local or remote)
3. **Authentication Token**: API token for agent authentication
4. **Network Access**: Internet connectivity to Solana network

### Quick Start for AI Agents

#### Step 1: Installation
```bash
# For Rust agents
cargo add agent-wallet

# For Python agents  
pip install agent-wallet-client

# For JavaScript agents
npm install @agent-wallet/client
```

#### Step 2: Configuration
```yaml
# agent_config.yaml
wallet:
  rpc_url: "https://api.devnet.solana.com"
  wallet_path: "./wallets/agent_wallet.json"
  encryption_passphrase_env: "WALLET_PASSPHRASE"
  
agent:
  permission_level: "advanced"
  daily_spend_limit_sol: 10.0
  allowed_protocols: ["raydium", "orca"]
  
security:
  require_transaction_simulation: true
  max_slippage_percent: 1.0
  blacklist_check: true
```

#### Step 3: Initialize Agent
```python
from agent_wallet import AgentWallet, AgentConfig

config = AgentConfig.from_yaml("agent_config.yaml")
wallet = AgentWallet(config)

# Check initial state
balance = wallet.get_balance()
print(f"Initial balance: {balance} SOL")

# Subscribe to updates
wallet.subscribe_to_updates(lambda event: print(f"Event: {event}"))
```

#### Step 4: Implement Decision Logic
```python
class TradingAgent:
    def __init__(self, wallet):
        self.wallet = wallet
        self.last_action_time = None
        
    def decide_action(self, market_data):
        # Agent-specific decision logic
        if market_data["sol_price"] < 100:
            return {
                "type": "transfer_sol",
                "params": {
                    "recipient": "buy_address",
                    "amount": 1.0
                }
            }
        elif market_data["sol_price"] > 110:
            return {
                "type": "swap_tokens",
                "params": {
                    "input_token": "SOL",
                    "output_token": "USDC",
                    "amount": 2.0,
                    "slippage": 0.5
                }
            }
        return None
        
    def run(self):
        while True:
            market_data = self.fetch_market_data()
            action = self.decide_action(market_data)
            if action:
                result = self.wallet.execute(action)
                self.log_action(action, result)
            time.sleep(60)  # Check every minute
```

## Performance Characteristics

### Transaction Latency
- **Local Signing**: < 10ms
- **RPC Round-trip**: 100-500ms (depends on RPC provider)
- **Network Confirmation**: 1-10 seconds (Solana block time)

### Throughput
- **Maximum Transactions**: ~100 TPS per agent (Solana network limit)
- **Concurrent Operations**: Multiple asynchronous operations supported
- **Batch Transactions**: Up to 10 actions per atomic transaction

### Resource Usage
- **Memory**: ~50MB baseline
- **CPU**: Minimal for transaction signing
- **Network**: Variable based on transaction volume

## Limitations & Constraints

### Current Limitations
1. **Network**: Currently optimized for Solana devnet; mainnet support experimental
2. **Token Support**: All SPL tokens supported, but some DeFi protocols require specific integration
3. **Cross-Chain**: No native cross-chain capabilities (requires bridging solutions)
4. **Privacy**: Transactions are public on Solana blockchain
5. **Oracle Integration**: Limited built-in oracle support (Pyth integrated, others require custom setup)

### Agent Constraints
1. **Decision Frequency**: Agents limited to 1 decision per second by default
2. **Spending Caps**: Configurable daily/weekly limits enforced
3. **Protocol Restrictions**: Can restrict which protocols agents can interact with
4. **Time Windows**: Can limit agent operation to specific time periods

### Security Constraints
1. **No Private Key Export**: Agents cannot export private keys
2. **Transaction Review**: Large transactions may require manual review if configured
3. **Geographic Restrictions**: Can block transactions to/from certain jurisdictions
4. **Compliance Checks**: Optional AML/KYC checks for large transactions

## Error Handling & Recovery

### Common Errors
- **InsufficientBalance**: Agent tried to spend more than available
- **TransactionFailed**: Transaction rejected by network
- **RateLimitExceeded**: Too many requests in time period
- **PermissionDenied**: Agent lacks permission for requested action
- **NetworkError**: Cannot connect to Solana network

### Recovery Strategies
```python
def execute_with_retry(wallet, action, max_retries=3):
    for attempt in range(max_retries):
        try:
            return wallet.execute(action)
        except InsufficientBalanceError:
            # Wait for balance update or reduce amount
            time.sleep(5)
            continue
        except TransactionFailedError as e:
            # Log failure and retry with higher fee
            action["params"]["priority_fee"] = action["params"].get("priority_fee", 0) + 1000
            continue
        except NetworkError:
            # Switch RPC endpoint
            wallet.switch_rpc_endpoint()
            time.sleep(2 ** attempt)  # Exponential backoff
            continue
    raise ExecutionError(f"Failed after {max_retries} attempts")
```

### State Recovery
- **Wallet State**: Automatically recovered from persistent storage
- **Transaction State**: Tracked with confirmations and finality status
- **Agent State**: Agents responsible for maintaining their own decision state

## Monitoring & Observability

### Built-in Monitoring
- **Balance Tracking**: Real-time balance updates
- **Transaction Logging**: All transactions logged with metadata
- **Performance Metrics**: Latency, success rate, error rates
- **Agent Activity**: Decision frequency, action types, spending patterns

### Integration Points
- **Prometheus Metrics**: Export metrics for dashboarding
- **Structured Logging**: JSON logs for log aggregation systems
- **WebSocket Events**: Real-time events for dashboard updates
- **Webhook Notifications**: Notifications for important events

### Health Checks
```bash
# Check wallet service health
curl http://localhost:8080/health

# Check RPC connectivity  
curl http://localhost:8080/health/rpc

# Check wallet balance
curl http://localhost:8080/health/balance
```

## Future Capabilities (Planned)

### Short-term (Next 3 months)
- **Multi-chain Support**: Ethereum, Polygon via wormhole
- **Advanced Order Types**: Limit orders, stop losses
- **Portfolio Management**: Automated portfolio rebalancing
- **Gas Optimization**: Automatic gas price optimization

### Medium-term (3-6 months)
- **Privacy Features**: Confidential transactions where possible
- **Cross-Agent Communication**: Agent-to-agent transaction protocols
- **Predictive Fees**: Machine learning for fee prediction
- **Insurance Integration**: DeFi insurance for large transactions

### Long-term (6+ months)
- **Federated Learning**: Collaborative agent learning without sharing data
- **Formal Verification**: Mathematically verified transaction safety
- **Quantum Resistance**: Post-quantum cryptography support
- **Decentralized Agent Network**: Fully decentralized agent coordination

## Support & Resources

### Documentation
- **API Reference**: Complete API documentation
- **Tutorials**: Step-by-step guides for common use cases
- **Examples**: Example agents for different domains (trading, DeFi, payments)
- **Troubleshooting**: Common issues and solutions

### Community
- **Discord Channel**: Real-time support and discussion
- **GitHub Issues**: Bug reports and feature requests
- **Agent Marketplace**: Share and discover agent strategies
- **Contributor Guide**: How to contribute to the project

### Security Reporting
- **Responsible Disclosure**: security@agent-wallet.org
- **Bug Bounty Program**: Rewards for security vulnerabilities
- **Security Advisories**: Regular security updates and patches

---

## Version Information

- **Current Version**: 1.0.0-alpha
- **Solana Network**: Devnet (mainnet support: experimental)
- **Last Updated**: [Current Date]
- **Compatibility**: Solana v1.17+

*Note: This document is automatically generated from the wallet's capability registry. For the most up-to-date information, agents should query the wallet's `/api/v1/capabilities` endpoint.*