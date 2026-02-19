//! Core data types for AI Agent Wallet
//!
//! This module defines the fundamental data structures used throughout the
//! AI Agent Wallet system, including agent contexts, actions, wallet information,
//! and permission levels.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

use crate::error::Error;

/// Permission levels for agents and operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionLevel {
    /// Can only read balances and transaction history
    ReadOnly,
    /// Can transfer small amounts of SOL
    Basic,
    /// Can perform token operations (transfer, swap)
    Advanced,
    /// Can interact with any protocol
    Full,
    /// Can modify wallet configuration and perform administrative tasks
    Administrator,
}

impl PermissionLevel {
    /// Check if this permission level can perform an action requiring `required` level
    pub fn can_perform(&self, required: PermissionLevel) -> bool {
        *self >= required
    }

    /// Get the display name of the permission level
    pub fn display_name(&self) -> &'static str {
        match self {
            PermissionLevel::ReadOnly => "ReadOnly",
            PermissionLevel::Basic => "Basic",
            PermissionLevel::Advanced => "Advanced",
            PermissionLevel::Full => "Full",
            PermissionLevel::Administrator => "Administrator",
        }
    }
}

impl std::fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Agent action that can be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentAction {
    /// Transfer SOL to another address
    TransferSol {
        /// Destination address
        to: Pubkey,
        /// Amount in lamports
        amount: u64,
        /// Optional memo
        memo: Option<String>,
    },
    /// Transfer SPL token to another address
    TransferToken {
        /// Token mint address
        mint: Pubkey,
        /// Destination address
        to: Pubkey,
        /// Amount in token base units
        amount: u64,
        /// Optional memo
        memo: Option<String>,
    },
    /// Swap tokens using a DEX
    SwapTokens {
        /// Input token mint
        input_mint: Pubkey,
        /// Output token mint
        output_mint: Pubkey,
        /// Amount of input tokens
        amount: u64,
        /// Minimum amount of output tokens expected (slippage protection)
        min_output_amount: u64,
    },
    /// Provide liquidity to a pool
    ProvideLiquidity {
        /// Pool address
        pool: Pubkey,
        /// Token A amount
        token_a_amount: u64,
        /// Token B amount
        token_b_amount: u64,
    },
    /// Remove liquidity from a pool
    RemoveLiquidity {
        /// Pool address
        pool: Pubkey,
        /// LP token amount to remove
        lp_token_amount: u64,
    },
    /// Stake tokens
    StakeTokens {
        /// Staking program or pool
        staking_pool: Pubkey,
        /// Token amount to stake
        amount: u64,
    },
    /// Unstake tokens
    UnstakeTokens {
        /// Staking program or pool
        staking_pool: Pubkey,
        /// Amount to unstake
        amount: u64,
    },
    /// Custom protocol interaction
    ProtocolInteraction {
        /// Protocol identifier
        protocol: String,
        /// Action name
        action: String,
        /// Parameters as JSON string
        parameters: String,
    },
    /// No operation (do nothing)
    NoOp,
}

impl AgentAction {
    /// Get the required permission level for this action
    pub fn required_permission(&self) -> PermissionLevel {
        match self {
            AgentAction::TransferSol { .. } => PermissionLevel::Basic,
            AgentAction::TransferToken { .. } => PermissionLevel::Advanced,
            AgentAction::SwapTokens { .. } => PermissionLevel::Advanced,
            AgentAction::ProvideLiquidity { .. } => PermissionLevel::Advanced,
            AgentAction::RemoveLiquidity { .. } => PermissionLevel::Advanced,
            AgentAction::StakeTokens { .. } => PermissionLevel::Advanced,
            AgentAction::UnstakeTokens { .. } => PermissionLevel::Advanced,
            AgentAction::ProtocolInteraction { .. } => PermissionLevel::Full,
            AgentAction::NoOp => PermissionLevel::ReadOnly,
        }
    }

    /// Get a brief description of the action
    pub fn description(&self) -> String {
        match self {
            AgentAction::TransferSol { to, amount, .. } => {
                format!("Transfer {} lamports to {}", amount, to)
            }
            AgentAction::TransferToken {
                mint, to, amount, ..
            } => {
                format!("Transfer {} of token {} to {}", amount, mint, to)
            }
            AgentAction::SwapTokens {
                input_mint,
                output_mint,
                amount,
                ..
            } => format!(
                "Swap {} of token {} for token {}",
                amount, input_mint, output_mint
            ),
            AgentAction::ProvideLiquidity {
                token_a_amount,
                token_b_amount,
                ..
            } => format!(
                "Provide liquidity: {} token A, {} token B",
                token_a_amount, token_b_amount
            ),
            AgentAction::RemoveLiquidity {
                lp_token_amount, ..
            } => {
                format!("Remove {} LP tokens", lp_token_amount)
            }
            AgentAction::StakeTokens { amount, .. } => format!("Stake {} tokens", amount),
            AgentAction::UnstakeTokens { amount, .. } => format!("Unstake {} tokens", amount),
            AgentAction::ProtocolInteraction {
                protocol, action, ..
            } => format!("Interact with {}: {}", protocol, action),
            AgentAction::NoOp => "No operation".to_string(),
        }
    }
}

/// Market conditions for decision-making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConditions {
    /// Volatility index (0-1 scale)
    pub volatility: f64,
    /// Market trend (bullish/bearish/neutral)
    pub trend: MarketTrend,
    /// Liquidity conditions
    pub liquidity: LiquidityConditions,
    /// Overall market sentiment (0-1 scale)
    pub sentiment: f64,
}

/// Market trend direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MarketTrend {
    /// Bullish market
    Bullish,
    /// Bearish market
    Bearish,
    /// Neutral/sideways market
    Neutral,
}

/// Liquidity conditions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LiquidityConditions {
    /// High liquidity
    High,
    /// Medium liquidity
    Medium,
    /// Low liquidity
    Low,
}

/// Oracle data for price feeds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleData {
    /// Source oracle (e.g., "pyth", "chainlink")
    pub source: String,
    /// Price value
    pub price: f64,
    /// Confidence interval
    pub confidence: f64,
    /// Timestamp of the data
    pub timestamp: DateTime<Utc>,
    /// Optional exponential moving average
    pub ema: Option<f64>,
}

/// Spending limits for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingLimits {
    /// Daily spending limit in SOL
    pub daily_limit_sol: f64,
    /// Daily spending limit in USD equivalent
    pub daily_limit_usd: f64,
    /// Per-transaction limit in SOL
    pub per_transaction_limit_sol: f64,
    /// Remaining daily budget in SOL
    pub remaining_daily_budget_sol: f64,
    /// Last reset timestamp
    pub last_reset: DateTime<Utc>,
}

/// Protocol information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Protocol {
    /// Protocol name
    pub name: String,
    /// Protocol address
    pub address: Pubkey,
    /// Protocol version
    pub version: String,
    /// Supported actions
    pub supported_actions: Vec<String>,
    /// Risk level (0-1 scale)
    pub risk_level: f64,
}

/// Transaction record for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    /// Transaction signature
    pub signature: Signature,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Action type
    pub action_type: String,
    /// Amount transferred (if applicable)
    pub amount: Option<u64>,
    /// Token mint (if applicable)
    pub token_mint: Option<Pubkey>,
    /// Destination address (if applicable)
    pub destination: Option<Pubkey>,
    /// Status of the transaction
    pub status: TransactionStatus,
    /// Fee paid in lamports
    pub fee: u64,
    /// Optional memo
    pub memo: Option<String>,
}

/// Transaction status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    /// Successfully confirmed
    Confirmed,
    /// Failed
    Failed,
    /// Pending confirmation
    Pending,
    /// Unknown status
    Unknown,
}

/// Agent error for tracking recent errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentError {
    /// Error message
    pub message: String,
    /// Error type/category
    pub error_type: String,
    /// Timestamp when error occurred
    pub timestamp: DateTime<Utc>,
    /// Context in which error occurred
    pub context: String,
    /// Whether error was recoverable
    pub recoverable: bool,
}

/// Wallet information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    /// Wallet name/identifier
    pub name: String,
    /// Public key
    pub public_key: Pubkey,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,
    /// Current SOL balance in lamports
    pub balance_lamports: u64,
    /// Number of transactions
    pub transaction_count: u64,
    /// Permission level
    pub permission_level: PermissionLevel,
    /// Whether wallet is active
    pub is_active: bool,
}

/// Agent context for decision-making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    // Wallet state
    /// Current SOL balance
    pub wallet_balance: f64,
    /// Token balances (mint -> balance in base units)
    pub token_balances: HashMap<Pubkey, u64>,
    /// Recent transaction history
    pub transaction_history: Vec<TransactionRecord>,

    // Market data
    /// Price feeds (symbol -> price)
    pub price_feeds: HashMap<String, f64>,
    /// Current market conditions
    pub market_conditions: MarketConditions,
    /// Optional oracle data
    pub oracle_data: Option<OracleData>,

    // Temporal data
    /// Current timestamp
    pub timestamp: DateTime<Utc>,
    /// Last action timestamp
    pub last_action_time: Option<DateTime<Utc>>,
    /// Time since last action
    pub time_since_last_action: Option<Duration>,

    // Agent state
    /// Number of decisions made
    pub decision_count: u64,
    /// Success rate (0-1 scale)
    pub success_rate: f64,
    /// Recent errors
    pub recent_errors: Vec<AgentError>,

    // Configuration
    /// Spending limits
    pub spending_limits: SpendingLimits,
    /// Allowed protocols
    pub allowed_protocols: Vec<Protocol>,
    /// Current permission level
    pub permission_level: PermissionLevel,
}

impl AgentContext {
    /// Create a new agent context with default values
    pub fn new(public_key: Pubkey) -> Self {
        let now = Utc::now();
        Self {
            wallet_balance: 0.0,
            token_balances: HashMap::new(),
            transaction_history: Vec::new(),

            price_feeds: HashMap::new(),
            market_conditions: MarketConditions {
                volatility: 0.5,
                trend: MarketTrend::Neutral,
                liquidity: LiquidityConditions::Medium,
                sentiment: 0.5,
            },
            oracle_data: None,

            timestamp: now,
            last_action_time: None,
            time_since_last_action: None,

            decision_count: 0,
            success_rate: 1.0,
            recent_errors: Vec::new(),

            spending_limits: SpendingLimits {
                daily_limit_sol: 10.0,
                daily_limit_usd: 100.0,
                per_transaction_limit_sol: 1.0,
                remaining_daily_budget_sol: 10.0,
                last_reset: now,
            },
            allowed_protocols: Vec::new(),
            permission_level: PermissionLevel::Basic,
        }
    }

    /// Update the timestamp and recalculate time since last action
    pub fn update_timestamp(&mut self) {
        let now = Utc::now();
        self.timestamp = now;

        if let Some(last_action) = self.last_action_time {
            self.time_since_last_action = Some(
                now.signed_duration_since(last_action)
                    .to_std()
                    .unwrap_or_default(),
            );
        }
    }

    /// Record a successful action
    pub fn record_success(&mut self) {
        self.decision_count += 1;
        // Update success rate with exponential moving average
        self.success_rate = (self.success_rate * 0.95) + (1.0 * 0.05);
        self.last_action_time = Some(Utc::now());
        self.update_timestamp();
    }

    /// Record a failed action with error
    pub fn record_failure(&mut self, error: Error, context: String) {
        self.decision_count += 1;
        // Update success rate with exponential moving average
        self.success_rate = (self.success_rate * 0.95) + (0.0 * 0.05);

        let agent_error = AgentError {
            message: error.to_string(),
            error_type: error.to_string(),
            timestamp: Utc::now(),
            context,
            recoverable: error.is_recoverable(),
        };

        self.recent_errors.push(agent_error);

        // Keep only last 10 errors
        if self.recent_errors.len() > 10 {
            self.recent_errors.remove(0);
        }

        self.last_action_time = Some(Utc::now());
        self.update_timestamp();
    }

    /// Check if an action is allowed based on spending limits
    pub fn is_action_allowed(&self, sol_amount: f64) -> Result<(), Error> {
        // Check per-transaction limit
        if sol_amount > self.spending_limits.per_transaction_limit_sol {
            return Err(Error::LimitExceeded(format!(
                "Transaction amount {} SOL exceeds per-transaction limit {} SOL",
                sol_amount, self.spending_limits.per_transaction_limit_sol
            )));
        }

        // Check daily budget
        if sol_amount > self.spending_limits.remaining_daily_budget_sol {
            return Err(Error::InsufficientFunds {
                required: (sol_amount * 1_000_000_000.0) as u64,
                available: (self.spending_limits.remaining_daily_budget_sol * 1_000_000_000.0)
                    as u64,
            });
        }

        Ok(())
    }

    /// Deduct from daily budget
    pub fn deduct_from_budget(&mut self, sol_amount: f64) {
        self.spending_limits.remaining_daily_budget_sol -= sol_amount;

        // Ensure it doesn't go below zero
        if self.spending_limits.remaining_daily_budget_sol < 0.0 {
            self.spending_limits.remaining_daily_budget_sol = 0.0;
        }
    }

    /// Reset daily budget if it's a new day
    pub fn reset_daily_budget_if_needed(&mut self) {
        let now = Utc::now();
        let days_since_reset = now
            .signed_duration_since(self.spending_limits.last_reset)
            .num_days();

        if days_since_reset >= 1 {
            self.spending_limits.remaining_daily_budget_sol = self.spending_limits.daily_limit_sol;
            self.spending_limits.last_reset = now;
        }
    }
}

/// Agent identifier
pub type AgentId = String;

/// Agent status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is running and making decisions
    Active,
    /// Agent is paused (not making decisions)
    Paused,
    /// Agent is stopped
    Stopped,
    /// Agent is in error state
    Error,
}

/// Agent limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLimits {
    /// Maximum SOL per transaction
    pub max_sol_per_transaction: f64,
    /// Maximum tokens per transaction
    pub max_tokens_per_transaction: u64,
    /// Maximum transactions per hour
    pub max_transactions_per_hour: u32,
    /// Maximum transaction size in bytes
    pub max_transaction_size: usize,
    /// Allowed protocols
    pub allowed_protocols: Vec<String>,
}

impl Default for AgentLimits {
    fn default() -> Self {
        Self {
            max_sol_per_transaction: 1.0,
            max_tokens_per_transaction: 1_000_000_000, // 1 token with 9 decimals
            max_transactions_per_hour: 60,
            max_transaction_size: 1232, // Solana transaction size limit
            allowed_protocols: Vec::new(),
        }
    }
}
