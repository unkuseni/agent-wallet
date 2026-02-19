//! AI Agent Framework Library
//!
//! This library provides the agent framework for AI agent wallets on Solana.
//! It enables the creation of autonomous agents that can make decisions and
//! interact with wallets to execute transactions on the blockchain.
//!
//! # Features
//!
//! - **Agent Trait**: Unified interface for all agent types
//! - **Deterministic Agents**: Rule-based agents for predictable behavior
//! - **LLM Agents**: AI-powered agents using language models (optional feature)
//! - **Context Management**: Structured context for agent decision-making
//! - **Decision Framework**: Types for agent decisions and actions
//! - **Sandboxed Execution**: Safe environment for agent logic
//!
//! # Quick Start
//!
//! ```no_run
//! use agent_wallet_agent::prelude::*;
//! use agent_wallet_core::Wallet;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a deterministic agent
//!     let agent = DeterministicAgent::new(
//!         DeterministicStrategy::PeriodicTransfer {
//!             interval_seconds: 3600,
//!             recipient: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".parse()?,
//!             amount_sol: 0.1,
//!         }
//!     );
//!
//!     // Create agent context
//!     let context = AgentContext {
//!         wallet_balance: 5.0,
//!         market_data: None,
//!         timestamp: chrono::Utc::now(),
//!     };
//!
//!     // Get agent decision
//!     let decision = agent.decide(&context).await?;
//!
//!     match decision {
//!         Some(action) => {
//!             println!("Agent decided: {:?}", action);
//!             // Execute action through wallet
//!         }
//!         None => println!("No action required"),
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Agent Types
//!
//! ## Deterministic Agents
//!
//! Rule-based agents that follow predefined logic:
//!
//! - `PeriodicTransferAgent`: Transfers SOL at regular intervals
//! - `PriceThresholdAgent`: Executes trades based on price thresholds
//! - `ScriptedAgent`: Follows a sequence of predefined actions
//!
//! ## LLM Agents (Optional)
//!
//! AI-powered agents using language models:
//!
//! - `OpenAIAgent`: Uses OpenAI's GPT models for decision-making
//! - `AnthropicAgent`: Uses Claude models (if available)
//! - `LocalLLMAgent`: Uses local LLMs via ollama or similar
//!
//! # Safety Features
//!
//! - **Input Validation**: All agent decisions are validated before execution
//! - **Rate Limiting**: Agents have configurable rate limits
//! - **Spending Caps**: Maximum spending limits per time period
//! - **Sandboxing**: Agent logic runs in isolated environment

#![doc(html_logo_url = "https://raw.githubusercontent.com/yourusername/agent-wallet/main/logo.png")]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/yourusername/agent-wallet/main/favicon.ico"
)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

pub mod agent;
pub mod context;
pub mod decision;
pub mod deterministic;
pub mod error;
pub mod limits;
pub mod sandbox;

#[cfg(feature = "llm")]
pub mod llm;

// Re-exports for convenience
pub use agent::{Agent, AgentId, AgentStatus};
pub use context::AgentContext;
pub use decision::{AgentAction, AgentDecision, DecisionOutcome};
pub use deterministic::{DeterministicAgent, DeterministicStrategy};
pub use error::{AgentError, Result};

#[cfg(feature = "llm")]
pub use llm::{LlmAgent, LlmConfig, LlmProvider};

pub use limits::{AgentLimits, RateLimit, SpendingLimit};
pub use sandbox::{Sandbox, SandboxConfig};

/// Prelude module for easy importing of common types
pub mod prelude {
    pub use super::{
        Agent, AgentAction, AgentContext, AgentDecision, AgentError, AgentId, AgentLimits,
        AgentStatus, DecisionOutcome, DeterministicAgent, DeterministicStrategy, RateLimit, Result,
        Sandbox, SandboxConfig, SpendingLimit,
    };

    #[cfg(feature = "llm")]
    pub use super::{LlmAgent, LlmConfig, LlmProvider};
}

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Library name
pub const NAME: &str = "agent-wallet-agent";
/// Default agent decision timeout in seconds
pub const DEFAULT_DECISION_TIMEOUT_SECS: u64 = 30;
/// Default rate limit: decisions per minute
pub const DEFAULT_RATE_LIMIT_DECISIONS_PER_MINUTE: u32 = 60;
/// Default spending limit per day in SOL
pub const DEFAULT_SPENDING_LIMIT_SOL_PER_DAY: f64 = 10.0;
