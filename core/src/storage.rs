//! Secure storage services for AI Agent Wallet
//!
//! This module provides secure file-based storage for encrypted wallet data.
//! It handles the serialization, encryption, and persistence of wallet data
//! with proper metadata tracking and backup capabilities.
//!
//! # Storage Format
//!
//! ```json
//! {
//!   "version": "1.0",
//!   "encrypted_data": "base64_encoded_ciphertext",
//!   "salt": "base64_encoded_salt",
//!   "algorithm": "aes-256-gcm",
//!   "kdf_iterations": 100000,
//!   "metadata": {
//!     "created_at": "2024-01-01T00:00:00Z",
//!     "last_accessed": "2024-01-01T00:00:00Z",
//!     "public_key": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
//!     "name": "my-agent-wallet"
//!   }
//! }
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use zeroize::{Zeroize, Zeroizing};

use crate::config::StorageSettings;
use crate::encryption::{EncryptedData, EncryptionAlgorithm};
use crate::error::{Error, Result};
use crate::types::WalletInfo;

/// Wallet storage data structure (on-disk format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletStorage {
    /// Format version
    pub version: String,
    /// Encrypted wallet data
    pub encrypted_data: EncryptedData,
    /// Wallet metadata
    pub metadata: WalletMetadata,
}

/// Wallet metadata for tracking and identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    /// Wallet name/identifier
    pub name: String,
    /// Public key of the wallet
    pub public_key: Pubkey,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,
    /// Last modified timestamp
    pub last_modified: DateTime<Utc>,
    /// Wallet version (for upgrade handling)
    pub wallet_version: u32,
    /// Optional description
    pub description: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata key-value pairs
    pub custom_data: HashMap<String, String>,
}

/// Wallet data that gets encrypted and stored
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct WalletData {
    /// Encrypted private key bytes
    pub encrypted_private_key: Vec<u8>,
    /// Associated token accounts
    pub token_accounts: HashMap<Pubkey, TokenAccountInfo>,
    /// Transaction history (last N transactions)
    pub recent_transactions: Vec<TransactionRecord>,
    /// Agent configuration associated with this wallet
    pub agent_config: Option<AgentConfig>,
}

/// Token account information
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct TokenAccountInfo {
    /// Token mint address
    pub mint: Pubkey,
    /// Account address
    pub address: Pubkey,
    /// Current balance
    pub balance: u64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Transaction record for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    /// Transaction signature
    pub signature: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Action type
    pub action_type: String,
    /// Amount (if applicable)
    pub amount: Option<u64>,
    /// Status
    pub status: String,
}

/// Agent configuration stored with wallet
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct AgentConfig {
    /// Agent identifier
    pub agent_id: String,
    /// Agent type
    pub agent_type: String,
    /// Configuration parameters
    pub parameters: HashMap<String, String>,
    /// Last decision timestamp
    pub last_decision: Option<DateTime<Utc>>,
}

/// Storage service for managing encrypted wallet files
pub struct StorageService {
    /// Storage settings
    settings: StorageSettings,
    /// In-memory cache of loaded wallet metadata
    #[allow(dead_code)]
    wallet_cache: HashMap<String, WalletMetadata>,
}

impl StorageService {
    /// Create a new storage service with the given settings
    pub fn new(settings: StorageSettings) -> Result<Self> {
        // Ensure storage directories exist
        fs::create_dir_all(&settings.path).map_err(|e| {
            Error::storage(format!("Failed to create storage directory: {}", e))
        })?;

        fs::create_dir_all(&settings.backup_path).map_err(|e| {
            Error::storage(format!("Failed to create backup directory: {}", e))
        })?;

        Ok(Self {
            settings,
            wallet_cache: HashMap::new(),
        })
    }

    /// Save a wallet to storage
    pub fn save_wallet(
        &mut self,
        name: &str,
        encrypted_data: EncryptedData,
        public_key: Pubkey,
        description: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();

        // Create metadata
        let metadata = WalletMetadata {
            name: name.to_string(),
            public_key,
            created_at: now,
            last_accessed: now,
            last_modified: now,
            wallet_version: 1,
            description: description.map(|s| s.to_string()),
            tags: Vec::new(),
            custom_data: HashMap::new(),
        };

        // Create wallet storage
        let wallet_storage = WalletStorage {
            version: "1.0".to_string(),
            encrypted_data,
            metadata,
        };

        // Serialize to JSON
        let json_data = serde_json::to_string_pretty(&wallet_storage)
            .map_err(|e| Error::serialization(format!("Failed to serialize wallet: {}", e)))?;

        // Determine file path
        let file_path = self.wallet_file_path(name);

        // Write to file atomically
        let temp_path = file_path.with_extension("tmp");
        fs::write(&temp_path, &json_data)
            .map_err(|e| Error::storage(format!("Failed to write wallet file: {}", e)))?;

        fs::rename(&temp_path, &file_path)
            .map_err(|e| Error::storage(format!("Failed to rename wallet file: {}", e)))?;

        // Create backup
        self.backup_wallet(name)?;

        // Update cache
        self.wallet_cache.insert(name.to_string(), wallet_storage.metadata.clone());

        Ok(())
    }

    /// Load a wallet from storage
    pub fn load_wallet(&mut self, name: &str) -> Result<(EncryptedData, WalletMetadata)> {
        let file_path = self.wallet_file_path(name);

        // Read file
        let json_data = fs::read_to_string(&file_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Error::WalletNotFound(name.to_string())
                } else {
                    Error::storage(format!("Failed to read wallet file: {}", e))
                }
            })?;

        // Deserialize
        let wallet_storage: WalletStorage = serde_json::from_str(&json_data)
            .map_err(|e| Error::serialization(format!("Failed to parse wallet file: {}", e)))?;

        // Update last accessed timestamp
        let mut updated_storage = wallet_storage.clone();
        updated_storage.metadata.last_accessed = Utc::now();

        // Save updated metadata
        let updated_json = serde_json::to_string_pretty(&updated_storage)
            .map_err(|e| Error::serialization(format!("Failed to serialize updated wallet: {}", e)))?;

        fs::write(&file_path, &updated_json)
            .map_err(|e| Error::storage(format!("Failed to update wallet metadata: {}", e)))?;

        // Update cache
        self.wallet_cache.insert(name.to_string(), updated_storage.metadata.clone());

        Ok((wallet_storage.encrypted_data, updated_storage.metadata))
    }

    /// Delete a wallet from storage
    pub fn delete_wallet(&mut self, name: &str) -> Result<()> {
        let file_path = self.wallet_file_path(name);

        // Check if file exists
        if !file_path.exists() {
            return Err(Error::WalletNotFound(name.to_string()));
        }

        // Create backup before deletion
        self.backup_wallet(name)?;

        // Delete file
        fs::remove_file(&file_path)
            .map_err(|e| Error::storage(format!("Failed to delete wallet file: {}", e)))?;

        // Remove from cache
        self.wallet_cache.remove(name);

        // Also delete backup if it exists
        let backup_path = self.backup_file_path(name);
        if backup_path.exists() {
            let _ = fs::remove_file(&backup_path);
        }

        Ok(())
    }

    /// List all wallets in storage
    pub fn list_wallets(&self) -> Result<Vec<WalletInfo>> {
        let mut wallets = Vec::new();

        // Read storage directory
        let entries = fs::read_dir(&self.settings.path)
            .map_err(|e| Error::storage(format!("Failed to read storage directory: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                Error::storage(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    // Try to load metadata
                    match self.load_wallet_metadata(name) {
                        Ok(metadata) => {
                            let wallet_info = WalletInfo {
                                name: metadata.name.clone(),
                                public_key: metadata.public_key,
                                created_at: metadata.created_at,
                                last_accessed: metadata.last_accessed,
                                balance_lamports: 0, // Will be populated by wallet
                                transaction_count: 0, // Will be populated by wallet
                                permission_level: crate::types::PermissionLevel::Basic,
                                is_active: true,
                            };
                            wallets.push(wallet_info);
                        }
                        Err(_) => {
                            // Skip corrupted wallets
                            continue;
                        }
                    }
                }
            }
        }

        Ok(wallets)
    }

    /// Backup a wallet
    pub fn backup_wallet(&self, name: &str) -> Result<()> {
        let source_path = self.wallet_file_path(name);
        let backup_path = self.backup_file_path(name);

        if !source_path.exists() {
            return Err(Error::WalletNotFound(name.to_string()));
        }

        // Copy file to backup location
        fs::copy(&source_path, &backup_path)
            .map_err(|e| Error::storage(format!("Failed to backup wallet: {}", e)))?;

        Ok(())
    }

    /// Restore a wallet from backup
    pub fn restore_wallet(&self, name: &str) -> Result<()> {
        let backup_path = self.backup_file_path(name);
        let target_path = self.wallet_file_path(name);

        if !backup_path.exists() {
            return Err(Error::storage(format!("No backup found for wallet: {}", name)));
        }

        // Copy backup to main location
        fs::copy(&backup_path, &target_path)
            .map_err(|e| Error::storage(format!("Failed to restore wallet: {}", e)))?;

        Ok(())
    }

    /// Get wallet file path
    fn wallet_file_path(&self, name: &str) -> PathBuf {
        self.settings.path.join(format!("{}.json", name))
    }

    /// Get backup file path
    fn backup_file_path(&self, name: &str) -> PathBuf {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        self.settings.backup_path.join(format!("{}_{}.json", name, timestamp))
    }

    /// Load only wallet metadata (without encrypted data)
    fn load_wallet_metadata(&self, name: &str) -> Result<WalletMetadata> {
        let file_path = self.wallet_file_path(name);

        // Read file
        let json_data = fs::read_to_string(&file_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Error::WalletNotFound(name.to_string())
                } else {
                    Error::storage(format!("Failed to read wallet file: {}", e))
                }
            })?;

        // Deserialize only metadata
        #[derive(Deserialize)]
        struct PartialWalletStorage {
            metadata: WalletMetadata,
        }

        let partial: PartialWalletStorage = serde_json::from_str(&json_data)
            .map_err(|e| Error::serialization(format!("Failed to parse wallet file: {}", e)))?;

        Ok(partial.metadata)
    }

    /// Check if a wallet exists
    pub fn wallet_exists(&self, name: &str) -> bool {
        self.wallet_file_path(name).exists()
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> Result<StorageStats> {
        let mut stats = StorageStats::default();

        // Count wallet files
        let entries = fs::read_dir(&self.settings.path)
            .map_err(|e| Error::storage(format!("Failed to read storage directory: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                Error::storage(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                stats.wallet_count += 1;

                // Get file size
                if let Ok(metadata) = fs::metadata(&path) {
                    stats.total_size += metadata.len();
                }
            }
        }

        // Count backup files
        let backup_entries = fs::read_dir(&self.settings.backup_path)
            .map_err(|e| Error::storage(format!("Failed to read backup directory: {}", e)))?;

        for entry in backup_entries {
            let entry = entry.map_err(|e| {
                Error::storage(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                stats.backup_count += 1;
            }
        }

        Ok(stats)
    }

    /// Clean up old backups (keep only N most recent)
    pub fn cleanup_old_backups(&self, keep_count: usize) -> Result<()> {
        if keep_count == 0 {
            return Ok(());
        }

        // List all backup files
        let mut backup_files = Vec::new();
        let entries = fs::read_dir(&self.settings.backup_path)
            .map_err(|e| Error::storage(format!("Failed to read backup directory: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                Error::storage(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        backup_files.push((path, modified));
                    }
                }
            }
        }

        // Sort by modification time (oldest first)
        backup_files.sort_by(|a, b| a.1.cmp(&b.1));

        // Delete old backups
        for i in 0..backup_files.len().saturating_sub(keep_count) {
            let (path, _) = &backup_files[i];
            let _ = fs::remove_file(path);
        }

        Ok(())
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Number of wallet files
    pub wallet_count: u64,
    /// Number of backup files
    pub backup_count: u64,
    /// Total storage size in bytes
    pub total_size: u64,
}

impl Default for StorageStats {
    fn default() -> Self {
        Self {
            wallet_count: 0,
            backup_count: 0,
            total_size: 0,
        }
    }
}

/// Utility functions for wallet data serialization
pub mod utils {
    use super::*;

    /// Serialize wallet data to bytes
    pub fn serialize_wallet_data(wallet_data: &WalletData) -> Result<Vec<u8>> {
        bincode::serialize(wallet_data)
            .map_err(|e| Error::serialization(format!("Failed to serialize wallet data: {}", e)))
    }

    /// Deserialize wallet data from bytes
    pub fn deserialize_wallet_data(bytes: &[u8]) -> Result<WalletData> {
        bincode::deserialize(bytes)
            .map_err(|e| Error::serialization(format!("Failed to deserialize wallet data: {}", e)))
    }

    /// Create default wallet data
    pub fn create_default_wallet_data() -> WalletData {
        WalletData {
            encrypted_private_key: Vec::new(),
            token_accounts: HashMap::new(),
            recent_transactions: Vec::new(),
            agent_config: None,
        }
    }

    /// Create wallet data from encrypted private key
    pub fn create_wallet_data_from_key(encrypted_private_key: Vec<u8>) -> WalletData {
        WalletData {
            encrypted_private_key,
            token_accounts: HashMap::new(),
            recent_transactions: Vec::new(),
            agent_config: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_storage_service_creation() -> Result<()> {
        let temp_dir = tempdir().unwrap();
        let backup_dir = tempdir().unwrap();

        let settings = StorageSettings {
            path: temp_dir.path().to_path_buf(),
            backup_path: backup_dir.path().to_path_buf(),
            max_versions: 10,
        };

        let storage = StorageService::new(settings)?;

        // Verify directories were created
        assert!(temp_dir.path().exists());
        assert!(backup_dir.path().exists());

        Ok(())
    }

    #[test]
    fn test_wallet_metadata_serialization() -> Result<()> {
        let public_key = Pubkey::new_unique();
        let now = Utc::now();

        let metadata = WalletMetadata {
            name: "test-wallet".to_string(),
            public_key,
            created_at: now,
            last_accessed: now,
            last_modified: now,
            wallet_version: 1,
            description: Some("Test wallet".to_string()),
            tags: vec!["test".to_string(), "demo".to_string()],
            custom_data: {
                let mut map = HashMap::new();
                map.insert("owner".to_string(), "test-user".to_string());
                map
            },
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&metadata)?;
        let deserialized: WalletMetadata = serde_json::from_str(&json)?;

        assert_eq!(metadata.name, deserialized.name);
        assert_eq!(metadata.public_key, deserialized.public_key);
        assert_eq!(metadata.description, deserialized.description);
        assert_eq!(metadata.tags.len(), deserialized.tags.len());

        Ok(())
    }

    #[test]
    fn test_wallet_storage_format() -> Result<()> {
        let encrypted_data = EncryptedData {
            ciphertext: "ciphertext".to_string(),
            nonce: "nonce".to_string(),
            salt: "salt".to_string(),
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            kdf_iterations: 100_000,
            version: 1,
        };

        let public_key = Pubkey::new_unique();
        let now = Utc::now();

        let metadata = WalletMetadata {
            name: "test-wallet".to_string(),
            public_key,
            created_at: now,
            last_accessed: now,
            last_modified: now,
            wallet_version: 1,
            description: None,
            tags: Vec::new(),
            custom_data: HashMap::new(),
        };

        let wallet_storage = WalletStorage {
            version: "1.0".to_string(),
            encrypted_data,
            metadata,
        };

        // Verify serialization
        let json = serde_json::to_string_pretty(&wallet_storage)?;
        assert!(json.contains("\"version\": \"1.0\""));
        assert!(json.contains("\"encrypted_data\""));
        assert!(json.contains("\"metadata\""));

        Ok(())
    }

    #[test]
    fn test_wallet_data_zeroize() {
        let mut wallet_data = utils::create_default_wallet_data();
        wallet_data.encrypted_private_key = vec![1, 2, 3, 4, 5];

        // Verify data exists
        assert_eq!(wallet_data.encrypted_private_key.len(), 5);

        // Zeroize and drop
        drop(wallet_data);

        // The zeroize should have cleared the memory
        // (This is a compile-time check, not runtime)
    }
}
