//! Wallet implementation for AI Agent Wallet
//!
//! This module provides the main `Wallet` struct that integrates all the
//! components of the AI Agent Wallet system. It provides a unified interface
//! for creating, loading, and using wallets with full security features.
//!
//! # Example
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
//!     // Get wallet information
//!     let pubkey = wallet.public_key();
//!     let balance = wallet.get_balance().await?;
//!     println!("Wallet {} has {} SOL", pubkey, balance);
//!
//!     // Save wallet to disk
//!     wallet.save().await?;
//!
//!     // Load wallet later
//!     let loaded_wallet = Wallet::load("my-agent", &passphrase, config).await?;
//!     assert_eq!(loaded_wallet.public_key(), pubkey);
//!
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    transaction::Transaction,
};
use tokio::sync::{Mutex, RwLock};
use zeroize::Zeroizing;

use crate::config::{WalletConfig, WalletSettings};
use crate::encryption::{EncryptedData, EncryptionService};
use crate::error::{Error, Result};
use crate::keypair::{EncryptedKeypair, SecureKeypair};
use crate::rpc::RpcClient;
use crate::storage::{StorageService, WalletData, WalletMetadata, WalletStorage};
use crate::token::{TokenManager, TokenOperationResult};
use crate::transaction::{
    SimulationResult, TransactionBuilder, TransactionOptions, ValidationResult,
};
use crate::types::{AgentContext, PermissionLevel, WalletInfo};

/// Main wallet structure
pub struct Wallet {
    /// Wallet name/identifier
    name: String,
    /// Secure keypair for signing operations
    keypair: Arc<RwLock<SecureKeypair>>,
    /// Encrypted keypair data for storage
    encrypted_keypair: Arc<RwLock<Option<EncryptedKeypair>>>,
    /// RPC client for blockchain operations
    rpc_client: Arc<RwLock<RpcClient>>,
    /// Storage service for wallet persistence
    storage_service: Arc<RwLock<StorageService>>,
    /// Token manager for token operations
    token_manager: Arc<RwLock<TokenManager>>,
    /// Transaction builder for creating transactions
    transaction_builder: Arc<Mutex<TransactionBuilder>>,
    /// Wallet configuration
    config: WalletConfig,
    /// Wallet metadata
    metadata: Arc<RwLock<WalletMetadata>>,
    /// Agent context for decision-making
    agent_context: Arc<RwLock<AgentContext>>,
    /// Whether wallet is loaded and ready
    is_loaded: bool,
}

impl Wallet {
    /// Create a new wallet with a given name and passphrase
    ///
    /// # Arguments
    /// * `name` - Wallet name/identifier
    /// * `passphrase` - Passphrase for encrypting the wallet
    /// * `config` - Wallet configuration
    ///
    /// # Returns
    /// A new `Wallet` instance
    pub async fn create(
        name: impl Into<String>,
        passphrase: &Zeroizing<String>,
        config: WalletConfig,
    ) -> Result<Self> {
        let name = name.into();
        let start_time = std::time::Instant::now();

        // Generate secure keypair
        let keypair = SecureKeypair::generate();
        let public_key = keypair.public_key();

        // Create RPC client
        let rpc_config = crate::rpc::RpcClientConfig::from_settings(&config.rpc);
        let rpc_client = RpcClient::new(rpc_config).await?;

        // Create storage service
        let storage_service = StorageService::new(config.wallet.storage.clone())?;

        // Create token manager
        let token_manager = TokenManager::new_with_commitment(
            rpc_client.clone(),
            config.rpc.commitment.to_solana_commitment(),
        );

        // Create transaction builder
        let transaction_builder = TransactionBuilder::new();

        // Create wallet metadata
        let now = Utc::now();
        let metadata = WalletMetadata {
            name: name.clone(),
            public_key,
            created_at: now,
            last_accessed: now,
            last_modified: now,
            wallet_version: 1,
            description: None,
            tags: Vec::new(),
            custom_data: HashMap::new(),
        };

        // Create agent context
        let agent_context = AgentContext::new(public_key);

        // Encrypt keypair for storage
        let encrypted_keypair = keypair.encrypt(passphrase)?;

        // Create wallet data for storage
        let wallet_data = crate::storage::utils::create_wallet_data_from_key(
            bincode::serialize(&encrypted_keypair)
                .map_err(|e| Error::serialization(format!("Failed to serialize keypair: {}", e)))?,
        );

        // Encrypt wallet data
        let encrypted_data = crate::encryption::utils::encrypt_with_passphrase(
            &crate::storage::utils::serialize_wallet_data(&wallet_data)?,
            passphrase,
            config.wallet.encryption.algorithm.into(),
            config.wallet.encryption.kdf_iterations,
        )?;

        // Save wallet to storage
        storage_service
            .save_wallet(&name, encrypted_data, public_key, None)
            .await?;

        let wallet = Self {
            name: name.clone(),
            keypair: Arc::new(RwLock::new(keypair)),
            encrypted_keypair: Arc::new(RwLock::new(Some(encrypted_keypair))),
            rpc_client: Arc::new(RwLock::new(rpc_client)),
            storage_service: Arc::new(RwLock::new(storage_service)),
            token_manager: Arc::new(RwLock::new(token_manager)),
            transaction_builder: Arc::new(Mutex::new(transaction_builder)),
            config,
            metadata: Arc::new(RwLock::new(metadata)),
            agent_context: Arc::new(RwLock::new(agent_context)),
            is_loaded: true,
        };

        // Update agent context with initial balance
        wallet.update_agent_context().await?;

        let duration = start_time.elapsed();
        log::info!(
            "Wallet '{}' created in {:?}. Public key: {}",
            name,
            duration,
            public_key
        );

        Ok(wallet)
    }

    /// Load an existing wallet from storage
    ///
    /// # Arguments
    /// * `name` - Wallet name/identifier
    /// * `passphrase` - Passphrase for decrypting the wallet
    /// * `config` - Wallet configuration
    ///
    /// # Returns
    /// A loaded `Wallet` instance
    pub async fn load(
        name: impl Into<String>,
        passphrase: &Zeroizing<String>,
        config: WalletConfig,
    ) -> Result<Self> {
        let name = name.into();
        let start_time = std::time::Instant::now();

        // Create storage service
        let storage_service = StorageService::new(config.wallet.storage.clone())?;

        // Load wallet from storage
        let (encrypted_data, metadata) = storage_service.load_wallet(&name).await?;

        // Decrypt wallet data
        let decrypted_data =
            crate::encryption::utils::decrypt_with_passphrase(&encrypted_data, passphrase)?;

        // Deserialize wallet data
        let wallet_data = crate::storage::utils::deserialize_wallet_data(&decrypted_data)?;

        // Deserialize encrypted keypair
        let encrypted_keypair: EncryptedKeypair =
            bincode::deserialize(&wallet_data.encrypted_private_key).map_err(|e| {
                Error::serialization(format!("Failed to deserialize keypair: {}", e))
            })?;

        // Decrypt keypair
        let keypair = SecureKeypair::decrypt(&encrypted_keypair, passphrase)?;

        // Create RPC client
        let rpc_config = crate::rpc::RpcClientConfig::from_settings(&config.rpc);
        let rpc_client = RpcClient::new(rpc_config).await?;

        // Create token manager
        let token_manager = TokenManager::new_with_commitment(
            rpc_client.clone(),
            config.rpc.commitment.to_solana_commitment(),
        );

        // Create transaction builder
        let transaction_builder = TransactionBuilder::new();

        // Create agent context
        let mut agent_context = AgentContext::new(metadata.public_key);
        agent_context.permission_level = PermissionLevel::Basic; // Default

        let wallet = Self {
            name: name.clone(),
            keypair: Arc::new(RwLock::new(keypair)),
            encrypted_keypair: Arc::new(RwLock::new(Some(encrypted_keypair))),
            rpc_client: Arc::new(RwLock::new(rpc_client)),
            storage_service: Arc::new(RwLock::new(storage_service)),
            token_manager: Arc::new(RwLock::new(token_manager)),
            transaction_builder: Arc::new(Mutex::new(transaction_builder)),
            config,
            metadata: Arc::new(RwLock::new(metadata)),
            agent_context: Arc::new(RwLock::new(agent_context)),
            is_loaded: true,
        };

        // Update agent context with current state
        wallet.update_agent_context().await?;

        let duration = start_time.elapsed();
        log::info!(
            "Wallet '{}' loaded in {:?}. Public key: {}",
            name,
            duration,
            wallet.public_key()
        );

        Ok(wallet)
    }

    /// Save wallet to storage
    pub async fn save(&self) -> Result<()> {
        if !self.is_loaded {
            return Err(Error::State("Wallet is not loaded".to_string()));
        }

        let encrypted_keypair = self.encrypted_keypair.read().await;
        let encrypted_keypair = encrypted_keypair.as_ref().ok_or_else(|| {
            Error::State("Encrypted keypair not available for saving".to_string())
        })?;

        // Create wallet data
        let wallet_data = crate::storage::utils::create_wallet_data_from_key(
            bincode::serialize(encrypted_keypair)
                .map_err(|e| Error::serialization(format!("Failed to serialize keypair: {}", e)))?,
        );

        // For saving, we need the passphrase which we don't store
        // This is a limitation - we need to either:
        // 1. Store the passphrase (insecure)
        // 2. Ask for it again when saving
        // 3. Use a derived key that we can re-derive
        // For now, we'll return an error and document this limitation
        Err(Error::NotSupported(
            "Saving already-encrypted wallets requires re-encryption with passphrase. \
             Use Wallet::create() for new wallets or implement re-encryption logic."
                .to_string(),
        ))
    }

    /// Get wallet public key
    pub fn public_key(&self) -> Pubkey {
        // This is a bit awkward because we need to lock, but public key should be immutable
        // In practice, we might want to store public key separately
        let keypair = self.keypair.blocking_read();
        keypair.public_key()
    }

    /// Get wallet name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get wallet balance in SOL
    pub async fn get_balance(&self) -> Result<f64> {
        let rpc_client = self.rpc_client.read().await;
        let pubkey = self.public_key();

        let balance_lamports: u64 = rpc_client.get_balance(&pubkey).await?;
        Ok(balance_lamports as f64 / 1_000_000_000.0) // Convert lamports to SOL
    }

    /// Get token balance for a specific mint
    pub async fn get_token_balance(&self, mint: &Pubkey) -> Result<u64> {
        let token_manager = self.token_manager.read().await;
        let pubkey = self.public_key();

        token_manager.get_balance(mint, &pubkey).await
    }

    /// Transfer SOL to another address
    pub async fn transfer_sol(
        &self,
        to: &Pubkey,
        amount: f64, // Amount in SOL
        memo: Option<String>,
    ) -> Result<Signature> {
        // Convert SOL to lamports
        let amount_lamports = (amount * 1_000_000_000.0).round() as u64;

        if amount_lamports == 0 {
            return Err(Error::InvalidAmount(
                "Transfer amount must be greater than zero".to_string(),
            ));
        }

        // Check balance
        let balance = self.get_balance().await?;
        let balance_lamports = (balance * 1_000_000_000.0).round() as u64;

        if amount_lamports > balance_lamports {
            return Err(Error::InsufficientFunds {
                required: amount_lamports,
                available: balance_lamports,
            });
        }

        // Create agent action for validation
        let action = crate::types::AgentAction::TransferSol {
            to: *to,
            amount: amount_lamports,
            memo,
        };

        // Validate against agent context
        let agent_context = self.agent_context.read().await;
        agent_context.is_action_allowed(amount)?;

        // Build transaction
        let mut transaction_builder = self.transaction_builder.lock().await;
        let options = TransactionOptions::default();
        let mut transaction =
            transaction_builder.build_from_action(&action, &agent_context, &options)?;

        // Validate transaction
        let validation =
            transaction_builder.validate_transaction(&transaction, &agent_context, &options);
        if !validation.is_valid {
            return Err(Error::TransactionValidation(format!(
                "Transaction validation failed: {:?}",
                validation.errors
            )));
        }

        // Prepare and sign transaction
        let keypair = self.keypair.read().await;
        let rpc_client = self.rpc_client.read().await;
        let signature = transaction_builder
            .prepare_transaction(&mut transaction, &keypair, &rpc_client)
            .await?;

        // Send transaction
        rpc_client.send_transaction(&transaction).await?;

        // Update agent context
        let mut agent_context = self.agent_context.write().await;
        agent_context.deduct_from_budget(amount);
        agent_context.record_success();

        Ok(signature)
    }

    /// Transfer tokens to another address
    pub async fn transfer_token(
        &self,
        mint: &Pubkey,
        to: &Pubkey,
        amount: u64,
        memo: Option<String>,
    ) -> Result<Signature> {
        if amount == 0 {
            return Err(Error::InvalidAmount(
                "Transfer amount must be greater than zero".to_string(),
            ));
        }

        // Check token balance
        let balance = self.get_token_balance(mint).await?;
        if amount > balance {
            return Err(Error::InsufficientFunds {
                required: amount,
                available: balance,
            });
        }

        // Create agent action for validation
        let action = crate::types::AgentAction::TransferToken {
            mint: *mint,
            to: *to,
            amount,
            memo,
        };

        // Validate against agent context
        let agent_context = self.agent_context.read().await;

        // For tokens, we need to estimate SOL value
        // This is simplified - would need price feed integration
        let estimated_sol_value = amount as f64 / 1_000_000_000.0; // Assume 9 decimals
        agent_context.is_action_allowed(estimated_sol_value)?;

        // Build transaction
        let mut transaction_builder = self.transaction_builder.lock().await;
        let options = TransactionOptions::default();
        let mut transaction =
            transaction_builder.build_from_action(&action, &agent_context, &options)?;

        // Validate transaction
        let validation =
            transaction_builder.validate_transaction(&transaction, &agent_context, &options);
        if !validation.is_valid {
            return Err(Error::TransactionValidation(format!(
                "Transaction validation failed: {:?}",
                validation.errors
            )));
        }

        // Prepare and sign transaction
        let keypair = self.keypair.read().await;
        let rpc_client = self.rpc_client.read().await;
        let signature = transaction_builder
            .prepare_transaction(&mut transaction, &keypair, &rpc_client)
            .await?;

        // Send transaction
        rpc_client.send_transaction(&transaction).await?;

        // Update agent context
        let mut agent_context = self.agent_context.write().await;
        agent_context.deduct_from_budget(estimated_sol_value);
        agent_context.record_success();

        Ok(signature)
    }

    /// Sign a transaction (does not send it)
    pub async fn sign_transaction(&self, transaction: &mut Transaction) -> Result<Signature> {
        let keypair = self.keypair.read().await;
        let rpc_client = self.rpc_client.read().await;
        let mut transaction_builder = self.transaction_builder.lock().await;

        // Get recent blockhash
        let recent_blockhash = rpc_client.get_latest_blockhash().await?;

        // Sign transaction
        transaction_builder.sign_transaction(transaction, &keypair, recent_blockhash)
    }

    /// Sign and send a transaction
    pub async fn sign_and_send(&self, transaction: &mut Transaction) -> Result<Signature> {
        let signature = self.sign_transaction(transaction).await?;
        let rpc_client = self.rpc_client.read().await;

        // Send transaction
        rpc_client.send_transaction(transaction).await?;

        // Update agent context
        let mut agent_context = self.agent_context.write().await;
        agent_context.record_success();

        Ok(signature)
    }

    /// Simulate a transaction
    pub async fn simulate_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<SimulationResult> {
        let rpc_client = self.rpc_client.read().await;
        let transaction_builder = self.transaction_builder.lock().await;
        let options = TransactionOptions::default();

        transaction_builder
            .simulate_transaction(transaction, &rpc_client, &options)
            .await
    }

    /// Validate a transaction
    pub async fn validate_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<ValidationResult> {
        let transaction_builder = self.transaction_builder.lock().await;
        let agent_context = self.agent_context.read().await;
        let options = TransactionOptions::default();

        Ok(transaction_builder.validate_transaction(transaction, &agent_context, &options))
    }

    /// Get wallet information
    pub async fn get_info(&self) -> Result<WalletInfo> {
        let metadata = self.metadata.read().await;
        let balance = self.get_balance().await?;
        let balance_lamports = (balance * 1_000_000_000.0).round() as u64;

        // Get transaction count (simplified - would need to query history)
        let transaction_count = 0;

        Ok(WalletInfo {
            name: metadata.name.clone(),
            public_key: metadata.public_key,
            created_at: metadata.created_at,
            last_accessed: metadata.last_accessed,
            balance_lamports,
            transaction_count,
            permission_level: PermissionLevel::Basic, // Default
            is_active: self.is_loaded,
        })
    }

    /// Get agent context
    pub async fn get_agent_context(&self) -> Result<AgentContext> {
        let agent_context = self.agent_context.read().await;
        Ok(agent_context.clone())
    }

    /// Update agent context with current wallet state
    async fn update_agent_context(&self) -> Result<()> {
        let mut agent_context = self.agent_context.write().await;

        // Update wallet balance
        let balance = self.get_balance().await?;
        agent_context.wallet_balance = balance;

        // Update token balances
        // This is simplified - would need to fetch all token accounts
        agent_context.token_balances.clear();

        // Update timestamp
        agent_context.update_timestamp();

        // Reset daily budget if needed
        agent_context.reset_daily_budget_if_needed();

        Ok(())
    }

    /// List all wallets in storage
    pub async fn list_wallets(config: &WalletConfig) -> Result<Vec<WalletInfo>> {
        let storage_service = StorageService::new(config.wallet.storage.clone())?;
        storage_service.list_wallets().await
    }

    /// Delete wallet from storage
    pub async fn delete(name: impl Into<String>, config: &WalletConfig) -> Result<()> {
        let name = name.into();
        let storage_service = StorageService::new(config.wallet.storage.clone())?;
        storage_service.delete_wallet(&name).await
    }

    /// Check if wallet exists in storage
    pub async fn exists(name: impl Into<String>, config: &WalletConfig) -> Result<bool> {
        let name = name.into();
        let storage_service = StorageService::new(config.wallet.storage.clone())?;
        Ok(storage_service.wallet_exists(&name))
    }

    /// Get RPC client for direct access (advanced usage)
    pub fn rpc_client(&self) -> Arc<RwLock<RpcClient>> {
        self.rpc_client.clone()
    }

    /// Get token manager for direct access (advanced usage)
    pub fn token_manager(&self) -> Arc<RwLock<TokenManager>> {
        self.token_manager.clone()
    }

    /// Get transaction builder for direct access (advanced usage)
    pub fn transaction_builder(&self) -> Arc<Mutex<TransactionBuilder>> {
        self.transaction_builder.clone()
    }

    /// Get wallet configuration
    pub fn config(&self) -> &WalletConfig {
        &self.config
    }

    /// Check if wallet is loaded
    pub fn is_loaded(&self) -> bool {
        self.is_loaded
    }
}

impl Drop for Wallet {
    fn drop(&mut self) {
        if self.is_loaded {
            log::debug!("Wallet '{}' dropped", self.name);
        }
    }
}

// Implement Signer trait for compatibility with Solana ecosystem
impl Signer for Wallet {
    fn try_pubkey(&self) -> std::result::Result<Pubkey, solana_sdk::signer::SignerError> {
        Ok(self.public_key())
    }

    fn try_sign_message(
        &self,
        message: &[u8],
    ) -> std::result::Result<Signature, solana_sdk::signer::SignerError> {
        let keypair = self.keypair.blocking_read();
        Ok(keypair.sign(message))
    }

    fn is_interactive(&self) -> bool {
        false
    }
}

/// Wallet builder for creating wallets with custom configuration
pub struct WalletBuilder {
    name: Option<String>,
    config: WalletConfig,
}

impl WalletBuilder {
    /// Create a new wallet builder
    pub fn new() -> Self {
        Self {
            name: None,
            config: WalletConfig::default(),
        }
    }

    /// Set wallet name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set wallet configuration
    pub fn config(mut self, config: WalletConfig) -> Self {
        self.config = config;
        self
    }

    /// Build and create the wallet
    pub async fn create(self, passphrase: &Zeroizing<String>) -> Result<Wallet> {
        let name = self
            .name
            .ok_or_else(|| Error::config("Wallet name is required"))?;
        Wallet::create(name, passphrase, self.config).await
    }

    /// Build and load an existing wallet
    pub async fn load(self, passphrase: &Zeroizing<String>) -> Result<Wallet> {
        let name = self
            .name
            .ok_or_else(|| Error::config("Wallet name is required"))?;
        Wallet::load(name, passphrase, self.config).await
    }
}

impl Default for WalletBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // Note: These tests are mostly compile-time checks
    // Real tests would require a running Solana RPC endpoint

    #[test]
    fn test_wallet_builder_creation() {
        let builder = WalletBuilder::new();
        assert!(builder.name.is_none());
    }

    #[test]
    fn test_wallet_builder_with_name() {
        let builder = WalletBuilder::new().name("test-wallet");
        assert_eq!(builder.name, Some("test-wallet".to_string()));
    }
}
