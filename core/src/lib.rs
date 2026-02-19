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
//! use zeroize::Zeroizing;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a new wallet
//!     let config = WalletConfig::new();
//!     let passphrase = Zeroizing::new("secure-passphrase".to_string());
//!     let mut wallet = Wallet::create("my-agent", &passphrase, config).await?;
//!
//!     // Get balance
//!     let balance = wallet.get_balance().await?;
//!     println!("Balance: {} SOL", balance);
//!
//!     // Transfer SOL
//!     let signature = wallet.transfer_sol(
//!         &Pubkey::new_unique(),
//!         0.1,
//!         Some("Test transfer".to_string()),
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
pub use encryption::{EncryptedData, EncryptionService};
pub use error::{Error, Result};
pub use keypair::{EncryptedKeypair, KeypairManager, SecureKeypair};
pub use rpc::{RpcClient, RpcClientConfig};
pub use storage::{StorageService, WalletStorage};
pub use token::{TokenAccountInfo, TokenInfo, TokenManager, TokenMetadataInfo};
pub use transaction::{SimulationResult, TransactionBuilder, TransactionOptions, ValidationResult};
pub use types::{AgentAction, AgentContext, PermissionLevel, WalletInfo};
pub use wallet::{Wallet, WalletBuilder};

// Type aliases for compatibility with architecture documentation
/// Secure keypair type
pub type Keypair = SecureKeypair;
/// Public key type
pub type PublicKey = solana_sdk::pubkey::Pubkey;
/// Token information type
pub type Token = TokenInfo;
/// Token account information type
pub type TokenAccount = TokenAccountInfo;
/// Token mint address type
pub type TokenMint = Pubkey;

/// Prelude module for easy importing of common types
pub mod prelude {
    pub use super::{
        AgentAction, AgentContext, EncryptedData, EncryptedKeypair, EncryptionService, Error,
        Keypair, KeypairManager, PermissionLevel, PublicKey, Result, RpcClient, RpcClientConfig,
        SecureKeypair, SimulationResult, StorageService, Token, TokenAccount, TokenAccountInfo,
        TokenInfo, TokenManager, TokenMetadataInfo, TokenMint, TransactionBuilder,
        TransactionOptions, ValidationResult, Wallet, WalletBuilder, WalletConfig, WalletInfo,
        WalletStorage,
    };

    // Re-export commonly used Solana types
    pub use solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{Signature, Signer},
        transaction::Transaction,
    };

    // Re-export zeroize for secure memory handling
    pub use zeroize::Zeroizing;
}

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Library name
pub const NAME: &str = "agent-wallet-core";
/// Minimum supported Solana RPC version
pub const MIN_SOLANA_RPC_VERSION: &str = "1.17.0";
