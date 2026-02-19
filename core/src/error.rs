//! Error types for the AI Agent Wallet Core library
//!
//! This module defines a comprehensive error type hierarchy for handling all
//! error scenarios in the wallet operations, from cryptographic failures to
//! network errors and agent-related issues.

use std::fmt;

/// Result type alias for the wallet operations
pub type Result<T> = std::result::Result<T, Error>;

/// Comprehensive error type for AI Agent Wallet operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Cryptographic operation failed
    #[error("Cryptographic error: {0}")]
    Crypto(String),

    /// Wallet encryption/decryption failed
    #[error("Encryption error: {0}")]
    Encryption(String),

    /// Failed to derive key from passphrase
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    /// RPC communication error
    #[error("RPC error: {0}")]
    Rpc(String),

    /// Solana RPC-specific error
    #[error("Solana RPC error: {0}")]
    SolanaRpc(#[from] solana_client::client_error::ClientError),

    /// Transaction-related error
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Transaction simulation failed
    #[error("Transaction simulation failed: {0}")]
    TransactionSimulation(String),

    /// Transaction validation failed
    #[error("Transaction validation failed: {0}")]
    TransactionValidation(String),

    /// Insufficient funds for transaction
    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: u64, available: u64 },

    /// Wallet creation/loading error
    #[error("Wallet error: {0}")]
    Wallet(String),

    /// Wallet storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Wallet not found
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),
    /// Account not found
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    /// Keypair operation failed
    #[error("Keypair error: {0}")]
    Keypair(String),

    /// Invalid key format or corruption
    #[error("Invalid key format: {0}")]
    InvalidKey(String),

    /// Token operation error
    #[error("Token error: {0}")]
    Token(String),

    /// Token account not found
    #[error("Token account not found: {0}")]
    TokenAccountNotFound(String),

    /// Invalid token mint
    #[error("Invalid token mint: {0}")]
    InvalidTokenMint(String),

    /// Agent-related error
    #[error("Agent error: {0}")]
    Agent(String),

    /// Agent sandbox violation
    #[error("Agent sandbox violation: {0}")]
    SandboxViolation(String),

    /// Agent limit exceeded
    #[error("Agent limit exceeded: {0}")]
    LimitExceeded(String),

    /// Agent decision error
    #[error("Agent decision error: {0}")]
    DecisionError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid permissions for operation
    #[error("Invalid permission level: required {required:?}, actual {actual:?}")]
    InvalidPermission {
        required: crate::types::PermissionLevel,
        actual: crate::types::PermissionLevel,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Timeout error
    #[error("Timeout: {0}")]
    Timeout(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Bincode serialization error
    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    /// Invalid address format
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    /// Invalid amount
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    /// Invalid signature
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    /// Invalid public key
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    /// Operation not supported
    #[error("Operation not supported: {0}")]
    NotSupported(String),

    /// State error (e.g., wallet not initialized)
    #[error("State error: {0}")]
    State(String),

    /// External service error
    #[error("External service error: {0}")]
    ExternalService(String),

    /// Unknown error (catch-all)
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl Error {
    /// Create a new cryptographic error
    pub fn crypto(msg: impl Into<String>) -> Self {
        Self::Crypto(msg.into())
    }

    /// Create a new encryption error
    pub fn encryption(msg: impl Into<String>) -> Self {
        Self::Encryption(msg.into())
    }

    /// Create a new RPC error
    pub fn rpc(msg: impl Into<String>) -> Self {
        Self::Rpc(msg.into())
    }

    /// Create a new transaction error
    pub fn transaction(msg: impl Into<String>) -> Self {
        Self::Transaction(msg.into())
    }

    /// Create a new wallet error
    pub fn wallet(msg: impl Into<String>) -> Self {
        Self::Wallet(msg.into())
    }

    /// Create a new storage error
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }

    /// Create a new token error
    pub fn token(msg: impl Into<String>) -> Self {
        Self::Token(msg.into())
    }

    /// Create a new agent error
    pub fn agent(msg: impl Into<String>) -> Self {
        Self::Agent(msg.into())
    }

    /// Create a new configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a new validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a new permission denied error
    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::PermissionDenied(msg.into())
    }

    /// Create a new network error
    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
    }

    /// Create a new serialization error
    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::Serialization(msg.into())
    }

    /// Check if error is due to insufficient funds
    pub fn is_insufficient_funds(&self) -> bool {
        matches!(self, Self::InsufficientFunds { .. })
    }

    /// Check if error is due to permission denial
    pub fn is_permission_denied(&self) -> bool {
        matches!(
            self,
            Self::PermissionDenied(_) | Self::InvalidPermission { .. }
        )
    }

    /// Check if error is due to rate limiting
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, Self::RateLimitExceeded(_))
    }

    /// Check if error is recoverable (can retry)
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Network(_)
            | Self::Timeout(_)
            | Self::Rpc(_)
            | Self::SolanaRpc(_)
            | Self::RateLimitExceeded(_) => true,
            _ => false,
        }
    }
}

impl From<solana_sdk::signature::ParseSignatureError> for Error {
    fn from(err: solana_sdk::signature::ParseSignatureError) -> Self {
        Self::InvalidSignature(format!("Failed to parse signature: {}", err))
    }
}

impl From<solana_sdk::pubkey::ParsePubkeyError> for Error {
    fn from(err: solana_sdk::pubkey::ParsePubkeyError) -> Self {
        Self::InvalidPublicKey(format!("Failed to parse public key: {}", err))
    }
}

impl From<hex::FromHexError> for Error {
    fn from(err: hex::FromHexError) -> Self {
        Self::Serialization(format!("Hex decoding error: {}", err))
    }
}

impl From<base64::DecodeError> for Error {
    fn from(err: base64::DecodeError) -> Self {
        Self::Serialization(format!("Base64 decoding error: {}", err))
    }
}
