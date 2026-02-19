//! dApp and DeFi Protocol Clients for AI Agent Wallets
//!
//! This library provides clients for interacting with dApps and DeFi protocols
//! on Solana. It enables AI agent wallets to perform complex operations like
//! token swaps, liquidity provision, and custom program interactions.
//!
//! # Features
//!
//! - **Test Program Client**: Simple counter program for testing and development
//! - **Raydium Integration**: Token swaps and liquidity pool operations
//! - **Orca Integration**: Alternative DEX with whirlpool support
//! - **Protocol Abstraction**: Unified interface for multiple DeFi protocols
//! - **Transaction Building**: Helper functions for constructing protocol-specific transactions
//!
//! # Quick Start
//!
//! ```no_run
//! use agent_wallet_dapp::prelude::*;
//! use agent_wallet_core::{Wallet, WalletConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize wallet
//!     let config = WalletConfig::new()
//!         .with_rpc_url("https://api.devnet.solana.com");
//!     let wallet = Wallet::load("wallet.json", config)?;
//!
//!     // Create Raydium client
//!     let raydium = RaydiumClient::new(wallet.rpc_client());
//!
//!     // Perform a swap
//!     let swap_params = SwapParams {
//!         input_token: "SOL".to_string(),
//!         output_token: "USDC".to_string(),
//!         amount: 1.0,
//!         slippage_bps: 50, // 0.5%
//!     };
//!
//!     let transaction = raydium.create_swap_transaction(&swap_params).await?;
//!
//!     // Sign and send with wallet
//!     let signature = wallet.sign_and_send(transaction).await?;
//!     println!("Swap transaction sent: {}", signature);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Supported Protocols
//!
//! ## Test Program
//!
//! A simple counter program for development and testing:
//!
//! ```no_run
//! use agent_wallet_dapp::test_program::CounterClient;
//!
//! let counter = CounterClient::new(program_id, rpc_client);
//! counter.increment(&wallet).await?;
//! let count = counter.get_count().await?;
//! ```
//!
//! ## Raydium
//!
//! Token swaps and liquidity operations:
//!
//! ```no_run
//! use agent_wallet_dapp::raydium::{RaydiumClient, SwapParams, LiquidityParams};
//!
//! // Swap SOL for USDC
//! let swap = SwapParams::new("SOL", "USDC", 1.0)
//!     .with_slippage(0.5); // 0.5% slippage
//!
//! // Add liquidity to pool
//! let liquidity = LiquidityParams::new(pool_address, 1000.0, 2000.0);
//! ```
//!
//! ## Orca
//!
//! Alternative DEX with concentrated liquidity:
//!
//! ```no_run
//! use agent_wallet_dapp::orca::{OrcaClient, WhirlpoolParams};
//!
//! // Interact with whirlpool
//! let whirlpool = WhirlpoolParams::new(pool_address, 100.0, 200.0);
//! ```
//!
//! # Safety Features
//!
//! - **Transaction Simulation**: All transactions are simulated before signing
//! - **Slippage Protection**: Configurable slippage limits for swaps
//! - **Price Validation**: Cross-reference prices from multiple sources
//! - **Gas Estimation**: Accurate fee estimation before transaction submission

#![doc(html_logo_url = "https://raw.githubusercontent.com/yourusername/agent-wallet/main/logo.png")]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/yourusername/agent-wallet/main/favicon.ico"
)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

pub mod common;
pub mod error;
pub mod protocol;

#[cfg(feature = "test-program")]
pub mod test_program;

#[cfg(feature = "raydium")]
pub mod raydium;

#[cfg(feature = "orca")]
pub mod orca;

// Re-exports for convenience
pub use common::{ProtocolClient, TransactionBuilder};
pub use error::{DappError, Result};
pub use protocol::{DexProtocol, ProtocolAction, ProtocolParams};

#[cfg(feature = "test-program")]
pub use test_program::{CounterClient, CounterInstruction};

#[cfg(feature = "raydium")]
pub use raydium::{LiquidityParams, RaydiumClient, SwapParams};

#[cfg(feature = "orca")]
pub use orca::{OrcaClient, WhirlpoolParams};

/// Prelude module for easy importing of common types
pub mod prelude {
    pub use super::{
        DappError, DexProtocol, ProtocolAction, ProtocolClient, ProtocolParams, Result,
        TransactionBuilder,
    };

    #[cfg(feature = "test-program")]
    pub use super::{CounterClient, CounterInstruction};

    #[cfg(feature = "raydium")]
    pub use super::{LiquidityParams, RaydiumClient, SwapParams};

    #[cfg(feature = "orca")]
    pub use super::{OrcaClient, WhirlpoolParams};

    // Re-export commonly used Solana types
    pub use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        transaction::Transaction,
    };
}

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Library name
pub const NAME: &str = "agent-wallet-dapp";
/// Default slippage tolerance in basis points (0.5%)
pub const DEFAULT_SLIPPAGE_BPS: u16 = 50;
/// Default priority fee in micro-lamports
pub const DEFAULT_PRIORITY_FEE_MICRO_LAMPORTS: u64 = 1000;
/// Maximum retry attempts for failed transactions
pub const MAX_TRANSACTION_RETRIES: u8 = 3;
/// Default transaction confirmation timeout in seconds
pub const DEFAULT_CONFIRMATION_TIMEOUT_SECS: u64 = 30;
