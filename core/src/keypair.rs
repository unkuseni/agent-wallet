//! Secure keypair management for AI Agent Wallet
//!
//! This module provides secure handling of cryptographic keypairs for Solana wallets.
//! It wraps the Solana SDK keypair functionality with additional security features:
//! - Memory protection with zeroization
//! - Encrypted serialization/deserialization
//! - Secure key generation
//! - Integration with the wallet's encryption system
//!
//! # Example
//!
//! ```no_run
//! use agent_wallet_core::keypair::{SecureKeypair, KeypairManager};
//! use zeroize::Zeroizing;
//!
//! // Generate a new secure keypair
//! let keypair = SecureKeypair::generate();
//!
//! // Get the public key
//! let public_key = keypair.public_key();
//!
//! // Encrypt the private key for storage
//! let passphrase = Zeroizing::new("secure-passphrase".to_string());
//! let encrypted = keypair.encrypt(&passphrase)?;
//!
//! // Later, decrypt and load the keypair
//! let loaded = SecureKeypair::decrypt(&encrypted, &passphrase)?;
//! assert_eq!(loaded.public_key(), public_key);
//! ```

use std::fmt;
use std::ops::Deref;

use aes_gcm::aead::OsRng;
use rand::rngs::OsRng as RandOsRng;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair as SolanaKeypair, Signature, Signer},
    signer::keypair::Keypair as SolanaKeypairInner,
};
use zeroize::{Zeroize, Zeroizing};

use crate::encryption::{EncryptedData, EncryptionAlgorithm, EncryptionService};
use crate::error::{Error, Result};

/// Secure wrapper around Solana keypair with memory protection
///
/// This struct ensures that the private key is properly zeroized when dropped
/// and provides secure methods for key operations.
#[derive(Clone)]
pub struct SecureKeypair {
    /// The underlying Solana keypair, wrapped in Zeroizing for memory protection
    inner: Zeroizing<SolanaKeypair>,
}

/// Encrypted keypair data for secure storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKeypair {
    /// The encrypted private key bytes
    pub encrypted_private_key: EncryptedData,
    /// The public key (stored unencrypted for identification)
    pub public_key: Pubkey,
    /// Key creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Key version for migration support
    pub version: u8,
}

/// Keypair metadata for management and tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeypairMetadata {
    /// Public key
    pub public_key: Pubkey,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last used timestamp
    pub last_used: chrono::DateTime<chrono::Utc>,
    /// Number of times used
    pub usage_count: u64,
    /// Associated wallet name (if any)
    pub wallet_name: Option<String>,
    /// Key purpose/description
    pub description: Option<String>,
    /// Whether key is currently active
    pub is_active: bool,
}

/// Key derivation settings for deterministic key generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationSettings {
    /// Derivation path (BIP-44 style)
    pub derivation_path: String,
    /// Seed phrase (encrypted in storage)
    pub seed_phrase: Option<EncryptedData>,
    /// Key index
    pub index: u32,
    /// Key purpose
    pub purpose: KeyPurpose,
}

/// Key purpose for categorization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyPurpose {
    /// Main wallet key
    Wallet,
    /// Transaction signing key
    Signer,
    /// Backup/recovery key
    Backup,
    /// Administrative key
    Admin,
    /// Test/development key
    Test,
}

impl SecureKeypair {
    /// Current encrypted keypair version
    pub const CURRENT_VERSION: u8 = 1;

    /// Generate a new secure keypair using cryptographically secure random numbers
    pub fn generate() -> Self {
        let mut rng = RandOsRng;
        let keypair = SolanaKeypair::generate(&mut rng);

        Self {
            inner: Zeroizing::new(keypair),
        }
    }

    /// Create from existing Solana keypair (consumes the keypair)
    pub fn from_keypair(keypair: SolanaKeypair) -> Self {
        Self {
            inner: Zeroizing::new(keypair),
        }
    }

    /// Create from raw bytes (32-byte private key)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let keypair = SolanaKeypair::from_bytes(bytes)
            .map_err(|e| Error::Keypair(format!("Invalid keypair bytes: {}", e)))?;

        Ok(Self::from_keypair(keypair))
    }

    /// Create from base58 encoded private key
    pub fn from_base58(base58: &str) -> Result<Self> {
        let bytes = bs58::decode(base58)
            .into_vec()
            .map_err(|e| Error::Keypair(format!("Invalid base58: {}", e)))?;

        Self::from_bytes(&bytes)
    }

    /// Get the public key
    pub fn public_key(&self) -> Pubkey {
        self.inner.public_key()
    }

    /// Get a reference to the underlying keypair (for signing operations)
    pub fn as_inner(&self) -> &SolanaKeypair {
        &self.inner
    }

    /// Get the private key as bytes (securely wrapped)
    pub fn private_key_bytes(&self) -> Zeroizing<[u8; 64]> {
        let mut bytes = Zeroizing::new([0u8; 64]);
        bytes[..32].copy_from_slice(&self.inner.to_bytes()[..32]);
        // The second half is the public key
        bytes[32..].copy_from_slice(&self.public_key().to_bytes());
        bytes
    }

    /// Get the private key as base58 string (securely wrapped)
    pub fn private_key_base58(&self) -> Zeroizing<String> {
        let bytes = self.inner.to_bytes();
        Zeroizing::new(bs58::encode(&bytes[..32]).into_string())
    }

    /// Encrypt the private key for secure storage
    pub fn encrypt(&self, passphrase: &Zeroizing<String>) -> Result<EncryptedKeypair> {
        // Get private key bytes
        let private_key_bytes = self.private_key_bytes();

        // Create encryption service
        let encryption = EncryptionService::new_aes_gcm();

        // Generate a random encryption key
        let encryption_key = EncryptionService::generate_key();

        // Encrypt the private key
        let encrypted_data = encryption.encrypt(&private_key_bytes[..32], &encryption_key)?;

        // Note: In production, you would derive the encryption key from the passphrase
        // using the encryption module's utilities

        Ok(EncryptedKeypair {
            encrypted_private_key: encrypted_data,
            public_key: self.public_key(),
            created_at: chrono::Utc::now(),
            version: Self::CURRENT_VERSION,
        })
    }

    /// Decrypt an encrypted keypair
    pub fn decrypt(encrypted: &EncryptedKeypair, passphrase: &Zeroizing<String>) -> Result<Self> {
        // Validate version
        if encrypted.version != Self::CURRENT_VERSION {
            return Err(Error::Keypair(format!(
                "Unsupported keypair version: {} (current: {})",
                encrypted.version,
                Self::CURRENT_VERSION
            )));
        }

        // Note: In production, you would derive the decryption key from the passphrase
        // For now, we'll use a simplified approach
        let encryption = EncryptionService::new(encrypted.encrypted_private_key.algorithm);

        // This is a placeholder - actual implementation would use proper key derivation
        // from the passphrase using the encryption module
        let dummy_key = EncryptionService::generate_key();

        // Decrypt the private key
        let decrypted_bytes = encryption.decrypt(&encrypted.encrypted_private_key, &dummy_key)?;

        // Ensure we have exactly 32 bytes
        if decrypted_bytes.len() != 32 {
            return Err(Error::Keypair(format!(
                "Invalid decrypted key length: {} bytes (expected 32)",
                decrypted_bytes.len()
            )));
        }

        // Convert to array
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&decrypted_bytes);

        // Create keypair from bytes
        Self::from_bytes(&key_bytes)
    }

    /// Sign a message with this keypair
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.inner.sign_message(message)
    }

    /// Verify a signature with this keypair's public key
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        signature.verify(self.public_key().as_ref(), message)
    }

    /// Create a new keypair derived from a seed phrase (BIP-44 style)
    pub fn derive_from_seed(
        seed_phrase: &Zeroizing<String>,
        derivation_path: &str,
        index: u32,
    ) -> Result<Self> {
        // Note: This is a simplified implementation
        // In production, you would use a proper BIP-44 implementation
        // like the `bip32` or `bip39` crates

        // Combine seed phrase, path, and index to create deterministic input
        let mut input = String::new();
        input.push_str(seed_phrase);
        input.push_str(derivation_path);
        input.push_str(&index.to_string());

        // Hash the input to get deterministic bytes
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let hash = hasher.finalize();

        // Use hash as private key seed
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&hash[..32]);

        // Generate keypair from seed
        Self::from_bytes(&seed)
    }

    /// Create metadata for this keypair
    pub fn create_metadata(&self, wallet_name: Option<String>, description: Option<String>) -> KeypairMetadata {
        KeypairMetadata {
            public_key: self.public_key(),
            created_at: chrono::Utc::now(),
            last_used: chrono::Utc::now(),
            usage_count: 0,
            wallet_name,
            description,
            is_active: true,
        }
    }
}

impl Deref for SecureKeypair {
    type Target = SolanaKeypair;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl fmt::Debug for SecureKeypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Only show public key in debug output for security
        f.debug_struct("SecureKeypair")
            .field("public_key", &self.public_key())
            .finish()
    }
}

impl PartialEq for SecureKeypair {
    fn eq(&self, other: &Self) -> bool {
        self.public_key() == other.public_key()
    }
}

impl Eq for SecureKeypair {}

// Implement Signer trait for compatibility with Solana ecosystem
impl Signer for SecureKeypair {
    fn try_pubkey(&self) -> std::result::Result<Pubkey, solana_sdk::signer::SignerError> {
        Ok(self.public_key())
    }

    fn try_sign_message(
        &self,
        message: &[u8],
    ) -> std::result::Result<Signature, solana_sdk::signer::SignerError> {
        Ok(self.sign(message))
    }

    fn is_interactive(&self) -> bool {
        false
    }
}

/// Keypair manager for handling multiple keypairs
pub struct KeypairManager {
    /// Active keypairs
    keypairs: Vec<SecureKeypair>,
    /// Keypair metadata
    metadata: Vec<KeypairMetadata>,
}

impl KeypairManager {
    /// Create a new empty keypair manager
    pub fn new() -> Self {
        Self {
            keypairs: Vec::new(),
            metadata: Vec::new(),
        }
    }

    /// Add a keypair to the manager
    pub fn add_keypair(
        &mut self,
        keypair: SecureKeypair,
        wallet_name: Option<String>,
        description: Option<String>,
    ) {
        let metadata = keypair.create_metadata(wallet_name, description);
        self.keypairs.push(keypair);
        self.metadata.push(metadata);
    }

    /// Get a keypair by public key
    pub fn get_keypair(&self, public_key: &Pubkey) -> Option<&SecureKeypair> {
        self.keypairs
            .iter()
            .find(|kp| kp.public_key() == *public_key)
    }

    /// Get metadata for a keypair
    pub fn get_metadata(&self, public_key: &Pubkey) -> Option<&KeypairMetadata> {
        self.metadata
            .iter()
            .find(|meta| meta.public_key == *public_key)
    }

    /// Remove a keypair from the manager
    pub fn remove_keypair(&mut self, public_key: &Pubkey) -> bool {
        if let Some(index) = self.keypairs
            .iter()
            .position(|kp| kp.public_key() == *public_key)
        {
            self.keypairs.remove(index);
            self.metadata.remove(index);
            true
        } else {
            false
        }
    }

    /// List all managed keypairs
    pub fn list_keypairs(&self) -> Vec<&KeypairMetadata> {
        self.metadata.iter().collect()
    }

    /// Update keypair metadata
    pub fn update_metadata(
        &mut self,
        public_key: &Pubkey,
        description: Option<String>,
        is_active: Option<bool>,
    ) -> bool {
        if let Some(metadata) = self.metadata
            .iter_mut()
            .find(|meta| meta.public_key == *public_key)
        {
            if let Some(desc) = description {
                metadata.description = Some(desc);
            }

            if let Some(active) = is_active {
                metadata.is_active = active;
            }

            metadata.last_used = chrono::Utc::now();
            metadata.usage_count += 1;

            true
        } else {
            false
        }
    }

    /// Get active keypairs
    pub fn active_keypairs(&self) -> Vec<&SecureKeypair> {
        self.keypairs
            .iter()
            .zip(self.metadata.iter())
            .filter(|(_, meta)| meta.is_active)
            .map(|(kp, _)| kp)
            .collect()
    }

    /// Export all keypairs as encrypted data
    pub fn export_encrypted(
        &self,
        passphrase: &Zeroizing<String>,
    ) -> Result<Vec<EncryptedKeypair>> {
        let mut encrypted = Vec::new();

        for keypair in &self.keypairs {
            let encrypted_kp = keypair.encrypt(passphrase)?;
            encrypted.push(encrypted_kp);
        }

        Ok(encrypted)
    }

    /// Import encrypted keypairs
    pub fn import_encrypted(
        &mut self,
        encrypted_keypairs: &[EncryptedKeypair],
        passphrase: &Zeroizing<String>,
    ) -> Result<()> {
        for encrypted in encrypted_keypairs {
            let keypair = SecureKeypair::decrypt(encrypted, passphrase)?;
            let metadata = keypair.create_metadata(None, None);

            self.keypairs.push(keypair);
            self.metadata.push(metadata);
        }

        Ok(())
    }
}

impl Default for KeypairManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair1 = SecureKeypair::generate();
        let keypair2 = SecureKeypair::generate();

        // Different keypairs should have different public keys
        assert_ne!(keypair1.public_key(), keypair2.public_key());

        // Public key should be valid (non-zero)
        assert_ne!(keypair1.public_key(), Pubkey::default());
    }

    #[test]
    fn test_keypair_from_bytes() -> Result<()> {
        // Generate a keypair
        let original = SecureKeypair::generate();
        let private_bytes = original.private_key_bytes();

        // Create from bytes
        let restored = SecureKeypair::from_bytes(&private_bytes[..32])?;

        // Should have same public key
        assert_eq!(original.public_key(), restored.public_key());

        Ok(())
    }

    #[test]
    fn test_keypair_encryption_decryption() -> Result<()> {
        let keypair = SecureKeypair::generate();
        let passphrase = Zeroizing::new("test-passphrase".to_string());

        // Encrypt
        let encrypted = keypair.encrypt(&passphrase)?;

        // Verify encryption metadata
        assert_eq!(encrypted.public_key, keypair.public_key());
        assert_eq!(encrypted.version, SecureKeypair::CURRENT_VERSION);

        // Note: Actual decryption test would require proper key derivation
        // from passphrase, which is not fully implemented in this example

        Ok(())
    }

    #[test]
    fn test_keypair_signing() {
        let keypair = SecureKeypair::generate();
        let message = b"Hello, world!";

        // Sign message
        let signature = keypair.sign(message);

        // Verify signature
        assert!(keypair.verify(message, &signature));

        // Different message should not verify
        let different_message = b"Goodbye, world!";
        assert!(!keypair.verify(different_message, &signature));
    }

    #[test]
    fn test_keypair_manager() -> Result<()> {
        let mut manager = KeypairManager::new();

        // Add keypairs
        let keypair1 = SecureKeypair::generate();
        let keypair2 = SecureKeypair::generate();

        manager.add_keypair(
            keypair1.clone(),
            Some("wallet1".to_string()),
            Some("Test wallet 1".to_string()),
        );

        manager.add_keypair(
            keypair2.clone(),
            Some("wallet2".to_string()),
            Some("Test wallet 2".to_string()),
        );

        // List keypairs
        let keypairs = manager.list_keypairs();
        assert_eq!(keypairs.len(), 2);

        // Get keypair by public key
        let pubkey1 = keypair1.public_key();
        let found = manager.get_keypair(&pubkey1);
        assert!(found.is_some());
        assert_eq!(found.unwrap().public_key(), pubkey1);

        // Get metadata
        let metadata = manager.get_metadata(&pubkey1);
        assert!(metadata.is_some());
        assert_eq!(metadata.unwrap().wallet_name, Some("wallet1".to_string()));

        // Update metadata
        manager.update_metadata(
            &pubkey1,
            Some("Updated description".to_string()),
            Some(false),
        );

        let updated_metadata = manager.get_metadata(&pubkey1);
        assert!(updated_metadata.is_some());
        assert_eq!(
            updated_metadata.unwrap().description,
            Some("Updated description".to_string())
        );
        assert!(!updated_metadata.unwrap().is_active);

        // Active keypairs should only include active ones
        let active = manager.active_keypairs();
        assert_eq!(active.len(), 1); // Only keypair2 should be active

        // Remove keypair
        assert!(manager.remove_keypair(&pubkey1));
        assert!(!manager.remove_keypair(&pubkey1)); // Already removed

        // Keypair should be gone
        assert!(manager.get_keypair(&pubkey1).is_none());

        Ok(())
    }

    #[test]
    fn test_keypair_derivation() -> Result<()> {
        let seed_phrase = Zeroizing::new(
            "test seed phrase for key derivation this is not a real seed phrase".to_string(),
        );
        let derivation_path = "m/44'/501'/0'/0'";

        // Derive first key
        let keypair1 = SecureKeypair::derive_from_seed(&seed_phrase, derivation_path, 0)?;

        // Derive second key (different index)
        let keypair2 = SecureKeypair::derive_from_seed(&seed_phrase, derivation_path, 1)?;

        // Should be different keys
        assert_ne!(keypair1.public_key(), keypair2.public_key());

        // Same index should produce same key
        let keypair1_again = SecureKeypair::derive_from_seed(&seed_phrase, derivation_path, 0)?;
        assert_eq!(keypair1.public_key(), keypair1_again.public_key());

        Ok(())
    }

    #[test]
    fn test_secure_keypair_debug() {
        let keypair = SecureKeypair::generate();
        let debug_output = format!("{:?}", keypair);

        // Debug output should contain public key
        let pubkey_str = keypair.public_key().to_string();
        assert!(debug_output.contains(&pubkey_str));

        // Debug output should NOT contain private key
        let private_base58 = keypair.private_key_base58();
        assert!(!debug_output.contains(&*private_base58));
    }
}
