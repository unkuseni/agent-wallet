//! AI Agent Wallet Core Library
//!
//! This library provides the core functionality for AI agent wallets on Solana.
//! It enables autonomous agents to create wallets, sign transactions, hold SOL/SPL tokens,
//! and interact with dApps without human intervention.
//!
//! # Features
//!
//! - **Secure Key Management**: AES-GCM encryption for private keys
//! - **Programmatic Wallet Creation**: Generate new wallets with encrypted storage
//! - **Automated Transaction Signing**: Sign and send transactions without manual input
//! - **SOL & SPL Token Support**: Full token operations (transfer, mint, burn)
//! - **Multi-Wallet Management**: Handle multiple agent wallets simultaneously
//! - **Sandboxed Execution**: Safe environment for agent decision logic
//!
//! # Quick Start
//!
//! ```no_run
//! use agent_wallet_core::{Wallet, WalletConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a new wallet
//!     let config = WalletConfig::new()
//!         .with_rpc_url("https://api.devnet.solana.com");
//!
//!     let mut wallet = Wallet::create("my-agent", "secure-passphrase", config)?;
//!
//!     // Get balance
//!     let balance = wallet.get_balance().await?;
//!     println!("Balance: {} SOL", balance);
//!
//!     // Transfer SOL
//!     let signature = wallet.transfer_sol(
//!         "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
//!         0.1,
//!         Some("Test transfer"),
//!     ).await?;
//!
//!     println!("Transaction sent: {}", signature);
//!     Ok(())
//! }
//! ```

#![doc(html_logo_url = "https://raw.githubusercontent.com/yourusername/agent-wallet/main/logo.png")]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/yourusername/agent-wallet/main/favicon.ico"
)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

pub mod config;
pub mod encryption;
pub mod error;
pub mod keypair;
pub mod rpc;
pub mod storage;
pub mod token;
pub mod transaction;
pub mod types;
pub mod wallet;

// Re-exports for convenience
pub use config::WalletConfig;
pub use error::{Error, Result};
pub use keypair::{Keypair, PublicKey};
pub use token::{Token, TokenAccount, TokenMint};
pub use transaction::{TransactionBuilder, TransactionOptions};
pub use types::{AgentAction, AgentContext, WalletInfo};
pub use wallet::Wallet;

/// Prelude module for easy importing of common types
pub mod prelude {
    pub use super::{
        AgentAction, AgentContext, Error, Keypair, PublicKey, Result, Token, TokenAccount,
        TokenMint, TransactionBuilder, Wallet, WalletConfig, WalletInfo,
    };

    // Re-export commonly used Solana types
    pub use solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::{Signature, Signer},
        transaction::Transaction,
    };
}

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Library name
pub const NAME: &str = "agent-wallet-core";
/// Minimum supported Solana RPC version
pub const MIN_SOLANA_RPC_VERSION: &str = "1.17.0";
