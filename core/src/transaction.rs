//! Transaction management for AI Agent Wallet
//!
//! This module provides comprehensive transaction building, signing, validation,
//! and submission functionality for AI agents on Solana. It converts high-level
//! agent actions into Solana transactions with proper safety checks and optimizations.
//!
//! # Features
//!
//! - **Action Conversion**: Convert `AgentAction` to Solana instructions
//! - **Safety Validation**: Comprehensive transaction validation before signing
//! - **Fee Optimization**: Automatic fee calculation and optimization
//! - **Simulation Support**: Pre-flight transaction simulation
//! - **Multi-Instruction**: Support for complex multi-instruction transactions
//! - **Permission Checking**: Validate actions against agent permissions
//!
//! # Example
//!
//! ```no_run
//! use agent_wallet_core::transaction::{TransactionBuilder, TransactionOptions};
//! use agent_wallet_core::types::{AgentAction, AgentContext};
//! use agent_wallet_core::keypair::SecureKeypair;
//! use agent_wallet_core::rpc::RpcClient;
//! use solana_sdk::pubkey::Pubkey;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create transaction builder
//!     let mut builder = TransactionBuilder::new();
//!
//!     // Create agent action
//!     let action = AgentAction::TransferSol {
//!         to: Pubkey::new_unique(),
//!         amount: 1_000_000, // 0.001 SOL
//!         memo: Some("Test transfer".to_string()),
//!     };
//!
//!     // Build transaction
//!     let agent_context = AgentContext::new(Pubkey::new_unique());
//!     let options = TransactionOptions::default();
//!     let transaction = builder.build_from_action(&action, &agent_context, &options)?;
//!
//!     // Validate transaction
//!     builder.validate_transaction(&transaction, &agent_context)?;
//!
//!     // Simulate (requires RPC client)
//!     // let rpc_client = RpcClient::new(config).await?;
//!     // let simulation = builder.simulate_transaction(&transaction, &rpc_client).await?;
//!
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    message::Message,
    native_token::lamports_to_sol,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    signer::keypair::Keypair,
    system_instruction,
    transaction::{Transaction, VersionedTransaction},
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction as token_instruction;

use crate::error::{Error, Result};
use crate::keypair::SecureKeypair;
use crate::rpc::RpcClient;
use crate::types::{AgentAction, AgentContext, PermissionLevel};

/// Transaction building and validation options
#[derive(Debug, Clone)]
pub struct TransactionOptions {
    /// Priority fee in micro-lamports per compute unit
    pub priority_fee: Option<u64>,
    /// Compute unit limit
    pub compute_unit_limit: Option<u32>,
    /// Compute unit price in micro-lamports
    pub compute_unit_price: Option<u64>,
    /// Skip preflight checks
    pub skip_preflight: bool,
    /// Commitment level for simulation
    pub commitment: CommitmentConfig,
    /// Maximum transaction size in bytes
    pub max_transaction_size: usize,
    /// Maximum signatures per transaction
    pub max_signatures: u8,
    /// Fee payer (if different from wallet)
    pub fee_payer: Option<Pubkey>,
    /// Recent blockhash validity duration (in slots)
    pub blockhash_validity_slots: u64,
    /// Whether to add memo instruction
    pub include_memo: bool,
}

impl Default for TransactionOptions {
    fn default() -> Self {
        Self {
            priority_fee: None,
            compute_unit_limit: Some(200_000), // Default compute unit limit
            compute_unit_price: None,
            skip_preflight: false,
            commitment: CommitmentConfig::confirmed(),
            max_transaction_size: 1232, // Solana max transaction size
            max_signatures: 20,         // Solana max signatures per transaction
            fee_payer: None,
            blockhash_validity_slots: 150, // ~1 minute at 400ms slots
            include_memo: true,
        }
    }
}

/// Transaction validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the transaction is valid
    pub is_valid: bool,
    /// Validation errors (if any)
    pub errors: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
    /// Estimated transaction fee
    pub estimated_fee: u64,
    /// Estimated compute units
    pub estimated_compute_units: u32,
    /// Transaction size in bytes
    pub transaction_size: usize,
}

impl ValidationResult {
    /// Create a valid validation result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            estimated_fee: 0,
            estimated_compute_units: 0,
            transaction_size: 0,
        }
    }

    /// Create an invalid validation result with errors
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
            estimated_fee: 0,
            estimated_compute_units: 0,
            transaction_size: 0,
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/// Transaction simulation result
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Whether simulation succeeded
    pub success: bool,
    /// Simulation logs
    pub logs: Vec<String>,
    /// Compute units consumed
    pub compute_units_consumed: Option<u64>,
    /// Return data (if any)
    pub return_data: Option<Vec<u8>>,
    /// Error message (if simulation failed)
    pub error: Option<String>,
    /// Accounts modified during simulation
    pub accounts_modified: Vec<Pubkey>,
    /// Transaction fee
    pub fee: u64,
}

/// Transaction builder for converting agent actions to Solana transactions
pub struct TransactionBuilder {
    /// Recent blockhash cache
    blockhash_cache: Option<(Hash, u64)>,
    /// Cache timestamp
    cache_timestamp: Option<std::time::Instant>,
    /// Maximum cache age
    max_cache_age: Duration,
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub fn new() -> Self {
        Self {
            blockhash_cache: None,
            cache_timestamp: None,
            max_cache_age: Duration::from_secs(30), // Cache for 30 seconds
        }
    }

    /// Build a transaction from an agent action
    pub fn build_from_action(
        &mut self,
        action: &AgentAction,
        context: &AgentContext,
        options: &TransactionOptions,
    ) -> Result<Transaction> {
        // Check permission
        self.validate_permission(action, context)?;

        // Check spending limits
        self.validate_spending_limits(action, context)?;

        // Convert action to instructions
        let instructions = self.action_to_instructions(action, context)?;

        // Check instruction count
        if instructions.len() > 20 {
            // Arbitrary limit for safety
            return Err(Error::validation(
                "Too many instructions in transaction".to_string(),
            ));
        }

        // Get fee payer
        let fee_payer = options
            .fee_payer
            .unwrap_or(context.permission_level.get_default_payer()?)
            .unwrap_or(context.get_wallet_pubkey());

        // Build message
        let message = Message::new_with_blockhash(
            &instructions,
            Some(&fee_payer),
            &self.get_cached_blockhash()?, // Will be updated with real blockhash before signing
        );

        // Create transaction
        let transaction = Transaction::new_unsigned(message);

        // Validate transaction size
        self.validate_transaction_size(&transaction, options)?;

        Ok(transaction)
    }

    /// Validate a transaction against agent context and options
    pub fn validate_transaction(
        &self,
        transaction: &Transaction,
        context: &AgentContext,
        options: &TransactionOptions,
    ) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Check transaction size
        let transaction_size = bincode::serialized_size(transaction).unwrap_or(0) as usize;
        result.transaction_size = transaction_size;

        if transaction_size > options.max_transaction_size {
            result.add_error(format!(
                "Transaction size {} bytes exceeds maximum {} bytes",
                transaction_size, options.max_transaction_size
            ));
        }

        // Check signature count
        let signature_count = transaction.signatures.len();
        if signature_count > options.max_signatures as usize {
            result.add_error(format!(
                "Transaction has {} signatures, maximum is {}",
                signature_count, options.max_signatures
            ));
        }

        // Check fee payer permission
        if let Some(fee_payer) = transaction.message.fee_payer() {
            if fee_payer != &context.get_wallet_pubkey() {
                // Only allow fee payer changes for administrators
                if !context
                    .permission_level
                    .can_perform(PermissionLevel::Administrator)
                {
                    result
                        .add_error("Only administrators can specify custom fee payers".to_string());
                }
            }
        }

        // Estimate fee (simplified)
        result.estimated_fee = self.estimate_transaction_fee(transaction, options);
        result.estimated_compute_units = self.estimate_compute_units(transaction);

        result
    }

    /// Sign a transaction with a secure keypair
    pub fn sign_transaction(
        &self,
        transaction: &mut Transaction,
        keypair: &SecureKeypair,
        recent_blockhash: Hash,
    ) -> Result<Signature> {
        // Update transaction with recent blockhash
        let mut message = transaction.message.clone();
        message.recent_blockhash = recent_blockhash;
        *transaction = Transaction::new_unsigned(message);

        // Sign transaction
        let signature = keypair.sign_transaction(transaction);
        transaction.signatures = vec![signature];

        Ok(signature)
    }

    /// Prepare transaction for sending (update blockhash, sign)
    pub async fn prepare_transaction(
        &mut self,
        transaction: &mut Transaction,
        keypair: &SecureKeypair,
        rpc_client: &RpcClient,
    ) -> Result<Signature> {
        // Get fresh blockhash
        let recent_blockhash = rpc_client.get_latest_blockhash().await?;

        // Update blockhash cache
        self.blockhash_cache = Some((recent_blockhash, 0)); // Slot 0 placeholder
        self.cache_timestamp = Some(std::time::Instant::now());

        // Sign transaction
        self.sign_transaction(transaction, keypair, recent_blockhash)
    }

    /// Simulate a transaction using RPC
    pub async fn simulate_transaction(
        &self,
        transaction: &Transaction,
        rpc_client: &RpcClient,
        options: &TransactionOptions,
    ) -> Result<SimulationResult> {
        let simulation = rpc_client.simulate_transaction(transaction).await?;

        let mut result = SimulationResult {
            success: simulation.value.err.is_none(),
            logs: simulation.value.logs.unwrap_or_default(),
            compute_units_consumed: simulation.value.units_consumed,
            return_data: simulation.value.return_data.map(|rd| rd.data),
            error: simulation.value.err.map(|e| format!("{:?}", e)),
            accounts_modified: Vec::new(),
            fee: simulation.value.fee.unwrap_or(0),
        };

        // Extract accounts modified from logs (simplified)
        for log in &result.logs {
            if log.contains("Program") && log.contains("succeeded") {
                // Parse program log to extract modified accounts
                // This is simplified - actual implementation would parse more carefully
            }
        }

        Ok(result)
    }

    /// Convert agent action to Solana instructions
    fn action_to_instructions(
        &self,
        action: &AgentAction,
        context: &AgentContext,
    ) -> Result<Vec<Instruction>> {
        match action {
            AgentAction::TransferSol { to, amount, memo } => {
                self.build_transfer_sol_instructions(context.get_wallet_pubkey(), to, *amount, memo)
            }
            AgentAction::TransferToken {
                mint,
                to,
                amount,
                memo,
            } => self.build_transfer_token_instructions(
                context.get_wallet_pubkey(),
                mint,
                to,
                *amount,
                memo,
            ),
            AgentAction::NoOp => Ok(Vec::new()),
            _ => Err(Error::NotSupported(
                "Action type not yet implemented".to_string(),
            )),
        }
    }

    /// Build SOL transfer instructions
    fn build_transfer_sol_instructions(
        &self,
        from: &Pubkey,
        to: &Pubkey,
        amount: u64,
        memo: &Option<String>,
    ) -> Result<Vec<Instruction>> {
        let mut instructions = Vec::new();

        // Add memo instruction if provided
        if let Some(memo_text) = memo {
            if !memo_text.is_empty() {
                instructions.push(spl_memo::build_memo(
                    memo_text.as_bytes(),
                    &[from], // Signer for memo
                ));
            }
        }

        // Add transfer instruction
        instructions.push(system_instruction::transfer(from, to, amount));

        Ok(instructions)
    }

    /// Build token transfer instructions
    fn build_transfer_token_instructions(
        &self,
        owner: &Pubkey,
        mint: &Pubkey,
        to: &Pubkey,
        amount: u64,
        memo: &Option<String>,
    ) -> Result<Vec<Instruction>> {
        let mut instructions = Vec::new();

        // Get associated token accounts
        let source_token_account = get_associated_token_address(owner, mint);
        let destination_token_account = get_associated_token_address(to, mint);

        // Add memo instruction if provided
        if let Some(memo_text) = memo {
            if !memo_text.is_empty() {
                instructions.push(spl_memo::build_memo(
                    memo_text.as_bytes(),
                    &[owner], // Signer for memo
                ));
            }
        }

        // Create destination token account if it doesn't exist
        instructions.push(
            spl_associated_token_account::create_associated_token_account(
                owner, // payer
                to,    // owner
                mint,  // mint
            ),
        );

        // Add transfer instruction
        instructions.push(token_instruction::transfer(
            &spl_token::id(),
            &source_token_account,
            &destination_token_account,
            owner,
            &[], // signers
            amount,
        )?);

        Ok(instructions)
    }

    /// Validate agent permission for action
    fn validate_permission(&self, action: &AgentAction, context: &AgentContext) -> Result<()> {
        let required_permission = action.required_permission();

        if !context.permission_level.can_perform(required_permission) {
            return Err(Error::InvalidPermission {
                required: required_permission,
                actual: context.permission_level,
            });
        }

        Ok(())
    }

    /// Validate spending limits for action
    fn validate_spending_limits(&self, action: &AgentAction, context: &AgentContext) -> Result<()> {
        match action {
            AgentAction::TransferSol { amount, .. } => {
                let sol_amount = lamports_to_sol(*amount);
                context.is_action_allowed(sol_amount)
            }
            AgentAction::TransferToken { amount, .. } => {
                // For tokens, we need to check value in SOL equivalent
                // This is simplified - would need price feed integration
                let estimated_sol_value = (*amount as f64) / 1_000_000_000.0; // Assume 1:1 for now
                context.is_action_allowed(estimated_sol_value)
            }
            AgentAction::NoOp => Ok(()),
            _ => {
                // For other actions, check a default minimum
                context.is_action_allowed(0.1) // 0.1 SOL default check
            }
        }
    }

    /// Get cached blockhash or generate placeholder
    fn get_cached_blockhash(&self) -> Result<Hash> {
        if let Some((blockhash, _)) = self.blockhash_cache {
            if let Some(timestamp) = self.cache_timestamp {
                if timestamp.elapsed() < self.max_cache_age {
                    return Ok(blockhash);
                }
            }
        }

        // Return placeholder - will be updated with real blockhash before signing
        Ok(Hash::new_from_array([0u8; 32]))
    }

    /// Validate transaction size
    fn validate_transaction_size(
        &self,
        transaction: &Transaction,
        options: &TransactionOptions,
    ) -> Result<()> {
        let size = bincode::serialized_size(transaction).unwrap_or(0) as usize;

        if size > options.max_transaction_size {
            return Err(Error::validation(format!(
                "Transaction size {} bytes exceeds maximum {} bytes",
                size, options.max_transaction_size
            )));
        }

        Ok(())
    }

    /// Estimate transaction fee
    fn estimate_transaction_fee(
        &self,
        transaction: &Transaction,
        options: &TransactionOptions,
    ) -> u64 {
        // Base fee: 5000 lamports per signature
        let base_fee = 5000 * transaction.signatures.len() as u64;

        // Priority fee if specified
        let priority_fee = options.priority_fee.unwrap_or(0);

        // Compute unit fee if specified
        let compute_fee = if let (Some(compute_unit_price), Some(compute_unit_limit)) =
            (options.compute_unit_price, options.compute_unit_limit)
        {
            (compute_unit_price * compute_unit_limit as u64) / 1_000_000 // Convert from micro-lamports
        } else {
            0
        };

        base_fee + priority_fee + compute_fee
    }

    /// Estimate compute units (simplified)
    fn estimate_compute_units(&self, transaction: &Transaction) -> u32 {
        // Simple estimation based on instruction count
        // Real implementation would analyze instructions
        transaction.message.instructions.len() as u32 * 10_000
    }

    /// Add priority fee instructions if needed
    pub fn add_priority_fee_instructions(
        &self,
        instructions: &mut Vec<Instruction>,
        options: &TransactionOptions,
    ) {
        if let (Some(compute_unit_limit), Some(compute_unit_price)) =
            (options.compute_unit_limit, options.compute_unit_price)
        {
            // Add compute budget instructions
            instructions.insert(
                0,
                ComputeBudgetInstruction::set_compute_unit_limit(compute_unit_limit),
            );
            instructions.insert(
                1,
                ComputeBudgetInstruction::set_compute_unit_price(compute_unit_price),
            );
        } else if let Some(compute_unit_limit) = options.compute_unit_limit {
            instructions.insert(
                0,
                ComputeBudgetInstruction::set_compute_unit_limit(compute_unit_limit),
            );
        }
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for AgentContext to support transaction operations
pub trait AgentContextExt {
    /// Get wallet public key (simplified - would come from actual wallet)
    fn get_wallet_pubkey(&self) -> Pubkey;
}

impl AgentContextExt for AgentContext {
    fn get_wallet_pubkey(&self) -> Pubkey {
        // In real implementation, this would come from the wallet
        // For now, return a placeholder
        Pubkey::new_from_array([0u8; 32])
    }
}

/// Extension trait for PermissionLevel to support transaction operations
pub trait PermissionLevelExt {
    /// Get default fee payer for permission level
    fn get_default_payer(&self) -> Result<Option<Pubkey>>;
}

impl PermissionLevelExt for PermissionLevel {
    fn get_default_payer(&self) -> Result<Option<Pubkey>> {
        match self {
            PermissionLevel::ReadOnly | PermissionLevel::Basic | PermissionLevel::Advanced => {
                Ok(None) // Use wallet as fee payer
            }
            PermissionLevel::Full | PermissionLevel::Administrator => {
                // Can specify custom fee payer
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Keypair;

    #[test]
    fn test_transaction_builder_creation() {
        let builder = TransactionBuilder::new();
        assert!(builder.blockhash_cache.is_none());
        assert!(builder.cache_timestamp.is_none());
    }

    #[test]
    fn test_validation_result() {
        let valid_result = ValidationResult::valid();
        assert!(valid_result.is_valid);
        assert!(valid_result.errors.is_empty());

        let errors = vec!["Error 1".to_string(), "Error 2".to_string()];
        let invalid_result = ValidationResult::invalid(errors.clone());
        assert!(!invalid_result.is_valid);
        assert_eq!(invalid_result.errors, errors);
    }

    #[test]
    fn test_permission_validation() -> Result<()> {
        let builder = TransactionBuilder::new();
        let mut context = AgentContext::new(Pubkey::new_unique());

        // Test Basic permission trying to perform Advanced action
        context.permission_level = PermissionLevel::Basic;
        let action = AgentAction::TransferToken {
            mint: Pubkey::new_unique(),
            to: Pubkey::new_unique(),
            amount: 1000,
            memo: None,
        };

        let result = builder.validate_permission(&action, &context);
        assert!(result.is_err());

        // Test Advanced permission trying to perform Basic action (should succeed)
        context.permission_level = PermissionLevel::Advanced;
        let action = AgentAction::TransferSol {
            to: Pubkey::new_unique(),
            amount: 1000,
            memo: None,
        };

        let result = builder.validate_permission(&action, &context);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_transaction_options_default() {
        let options = TransactionOptions::default();
        assert_eq!(options.max_transaction_size, 1232);
        assert_eq!(options.max_signatures, 20);
        assert!(!options.skip_preflight);
        assert_eq!(
            options.commitment.commitment,
            solana_sdk::commitment_config::CommitmentLevel::Confirmed
        );
    }

    #[test]
    fn test_action_to_instructions() -> Result<()> {
        let builder = TransactionBuilder::new();
        let context = AgentContext::new(Pubkey::new_unique());

        // Test SOL transfer
        let action = AgentAction::TransferSol {
            to: Pubkey::new_unique(),
            amount: 1_000_000,
            memo: Some("Test".to_string()),
        };

        let instructions = builder.action_to_instructions(&action, &context)?;
        assert!(!instructions.is_empty());

        // Test NoOp
        let action = AgentAction::NoOp;
        let instructions = builder.action_to_instructions(&action, &context)?;
        assert!(instructions.is_empty());

        Ok(())
    }

    #[test]
    fn test_fee_estimation() {
        let builder = TransactionBuilder::new();
        let keypair = Keypair::new();
        let to = Pubkey::new_unique();

        let transaction = Transaction::new_signed_with_payer(
            &[system_instruction::transfer(&keypair.pubkey(), &to, 1000)],
            Some(&keypair.pubkey()),
            &[&keypair],
            Hash::new_unique(),
        );

        let options = TransactionOptions::default();
        let fee = builder.estimate_transaction_fee(&transaction, &options);

        // Base fee for 1 signature = 5000 lamports
        assert_eq!(fee, 5000);
    }
}
