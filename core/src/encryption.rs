//! Encryption services for AI Agent Wallet
//!
//! This module provides secure encryption and decryption services for
//! protecting sensitive wallet data, particularly private keys.
//! It supports multiple encryption backends with a consistent API.
//!
//! # Features
//!
//! - **AES-256-GCM**: Authenticated encryption with AES-GCM
//! - **PBKDF2**: Password-based key derivation with configurable iterations
//! - **Secure Random**: Cryptographically secure random number generation
//! - **Zeroization**: Automatic zeroization of sensitive data in memory
//! - **Multiple Backends**: Support for AES-GCM and Ring encryption
//!
//! # Example
//!
//! ```no_run
//! use agent_wallet_core::encryption::{EncryptionService, KeyDerivation};
//! use zeroize::Zeroizing;
//!
//! // Create encryption service with default settings
//! let encryption = EncryptionService::new_aes_gcm();
//!
//! // Derive encryption key from passphrase
//! let passphrase = Zeroizing::new("secure-passphrase".to_string());
//! let salt = [0u8; 16]; // Should be random in production
//! let key = KeyDerivation::pbkdf2(&passphrase, &salt, 100_000);
//!
//! // Encrypt data
//! let plaintext = b"secret data";
//! let encrypted = encryption.encrypt(plaintext, &key)?;
//!
//! // Decrypt data
//! let decrypted = encryption.decrypt(&encrypted, &key)?;
//! assert_eq!(plaintext, decrypted.as_slice());
//! ```

use std::fmt;

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use pbkdf2::pbkdf2_hmac;
use ring::{
    aead::{self, Aad, LessSafeKey, UnboundKey, NONCE_LEN},
    rand::SystemRandom,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};

use crate::error::{Error, Result};

/// Encryption algorithm variants
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM encryption (default)
    Aes256Gcm,
    /// Ring-based encryption (alternative)
    Ring,
}

impl Default for EncryptionAlgorithm {
    fn default() -> Self {
        Self::Aes256Gcm
    }
}

impl fmt::Display for EncryptionAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncryptionAlgorithm::Aes256Gcm => write!(f, "aes-256-gcm"),
            EncryptionAlgorithm::Ring => write!(f, "ring"),
        }
    }
}

/// Encrypted data structure with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// The encrypted ciphertext
    pub ciphertext: String,
    /// The nonce/IV used for encryption
    pub nonce: String,
    /// The salt used for key derivation
    pub salt: String,
    /// Encryption algorithm used
    pub algorithm: EncryptionAlgorithm,
    /// Key derivation function iterations
    pub kdf_iterations: u32,
    /// Version of the encryption format
    pub version: u8,
}

impl EncryptedData {
    /// Current encryption format version
    pub const CURRENT_VERSION: u8 = 1;

    /// Create new encrypted data
    pub fn new(
        ciphertext: Vec<u8>,
        nonce: Vec<u8>,
        salt: Vec<u8>,
        algorithm: EncryptionAlgorithm,
        kdf_iterations: u32,
    ) -> Self {
        Self {
            ciphertext: STANDARD.encode(ciphertext),
            nonce: STANDARD.encode(nonce),
            salt: STANDARD.encode(salt),
            algorithm,
            kdf_iterations,
            version: Self::CURRENT_VERSION,
        }
    }

    /// Get ciphertext as bytes
    pub fn ciphertext_bytes(&self) -> Result<Vec<u8>> {
        STANDARD
            .decode(&self.ciphertext)
            .map_err(|e| Error::encryption(format!("Failed to decode ciphertext: {}", e)))
    }

    /// Get nonce as bytes
    pub fn nonce_bytes(&self) -> Result<Vec<u8>> {
        STANDARD
            .decode(&self.nonce)
            .map_err(|e| Error::encryption(format!("Failed to decode nonce: {}", e)))
    }

    /// Get salt as bytes
    pub fn salt_bytes(&self) -> Result<Vec<u8>> {
        STANDARD
            .decode(&self.salt)
            .map_err(|e| Error::encryption(format!("Failed to decode salt: {}", e)))
    }

    /// Validate the encrypted data structure
    pub fn validate(&self) -> Result<()> {
        if self.version != Self::CURRENT_VERSION {
            return Err(Error::encryption(format!(
                "Unsupported encryption version: {} (current: {})",
                self.version,
                Self::CURRENT_VERSION
            )));
        }

        // Validate base64 encoding
        self.ciphertext_bytes()?;
        self.nonce_bytes()?;
        self.salt_bytes()?;

        Ok(())
    }
}

/// Key derivation service
pub struct KeyDerivation;

impl KeyDerivation {
    /// Derive an encryption key using PBKDF2 with HMAC-SHA256
    ///
    /// # Arguments
    /// * `passphrase` - The passphrase to derive key from
    /// * `salt` - The salt (should be random for each key)
    /// * `iterations` - Number of PBKDF2 iterations
    ///
    /// # Returns
    /// A 32-byte encryption key wrapped in Zeroizing for secure memory handling
    pub fn pbkdf2(
        passphrase: &Zeroizing<String>,
        salt: &[u8],
        iterations: u32,
    ) -> Zeroizing<[u8; 32]> {
        let mut key = Zeroizing::new([0u8; 32]);
        pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), salt, iterations, &mut *key);
        key
    }

    /// Generate a random salt
    pub fn generate_salt() -> Zeroizing<[u8; 16]> {
        let mut salt = Zeroizing::new([0u8; 16]);
        let mut rng = rand::rngs::OsRng;
        rand::RngCore::fill_bytes(&mut rng, &mut *salt);
        salt
    }
}

/// Main encryption service
pub struct EncryptionService {
    algorithm: EncryptionAlgorithm,
}

impl EncryptionService {
    /// Create a new encryption service with AES-256-GCM
    pub fn new_aes_gcm() -> Self {
        Self {
            algorithm: EncryptionAlgorithm::Aes256Gcm,
        }
    }

    /// Create a new encryption service with Ring
    pub fn new_ring() -> Self {
        Self {
            algorithm: EncryptionAlgorithm::Ring,
        }
    }

    /// Create a new encryption service with specified algorithm
    pub fn new(algorithm: EncryptionAlgorithm) -> Self {
        Self { algorithm }
    }

    /// Get the encryption algorithm
    pub fn algorithm(&self) -> EncryptionAlgorithm {
        self.algorithm
    }

    /// Encrypt plaintext data
    ///
    /// # Arguments
    /// * `plaintext` - The data to encrypt
    /// * `key` - The encryption key (must be appropriate length for algorithm)
    ///
    /// # Returns
    /// EncryptedData containing the ciphertext and metadata
    pub fn encrypt(&self, plaintext: &[u8], key: &Zeroizing<[u8; 32]>) -> Result<EncryptedData> {
        match self.algorithm {
            EncryptionAlgorithm::Aes256Gcm => self.encrypt_aes_gcm(plaintext, key),
            EncryptionAlgorithm::Ring => self.encrypt_ring(plaintext, key),
        }
    }

    /// Decrypt encrypted data
    ///
    /// # Arguments
    /// * `encrypted` - The encrypted data structure
    /// * `key` - The encryption key used for encryption
    ///
    /// # Returns
    /// Decrypted plaintext data
    pub fn decrypt(
        &self,
        encrypted: &EncryptedData,
        key: &Zeroizing<[u8; 32]>,
    ) -> Result<Zeroizing<Vec<u8>>> {
        match encrypted.algorithm {
            EncryptionAlgorithm::Aes256Gcm => self.decrypt_aes_gcm(encrypted, key),
            EncryptionAlgorithm::Ring => self.decrypt_ring(encrypted, key),
        }
    }

    /// Generate a new random encryption key
    pub fn generate_key() -> Zeroizing<[u8; 32]> {
        let mut key = Zeroizing::new([0u8; 32]);
        let mut rng = rand::rngs::OsRng;
        rand::RngCore::fill_bytes(&mut rng, &mut *key);
        key
    }

    // AES-GCM implementation
    fn encrypt_aes_gcm(
        &self,
        plaintext: &[u8],
        key: &Zeroizing<[u8; 32]>,
    ) -> Result<EncryptedData> {
        // Generate random salt
        let salt = KeyDerivation::generate_salt();

        // Generate random nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Create cipher
        let cipher_key = Key::<Aes256Gcm>::from_slice(&**key);
        let cipher = Aes256Gcm::new(cipher_key);

        // Encrypt data
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| Error::encryption(format!("AES-GCM encryption failed: {}", e)))?;

        Ok(EncryptedData::new(
            ciphertext,
            nonce.to_vec(),
            salt.to_vec(),
            EncryptionAlgorithm::Aes256Gcm,
            100_000, // Default iterations
        ))
    }

    fn decrypt_aes_gcm(
        &self,
        encrypted: &EncryptedData,
        key: &Zeroizing<[u8; 32]>,
    ) -> Result<Zeroizing<Vec<u8>>> {
        // Get ciphertext and nonce
        let ciphertext = encrypted.ciphertext_bytes()?;
        let nonce_bytes = encrypted.nonce_bytes()?;

        // Create nonce
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Create cipher
        let cipher_key = Key::<Aes256Gcm>::from_slice(&**key);
        let cipher = Aes256Gcm::new(cipher_key);

        // Decrypt data
        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| Error::encryption(format!("AES-GCM decryption failed: {}", e)))?;

        Ok(Zeroizing::new(plaintext))
    }

    // Ring implementation
    fn encrypt_ring(&self, plaintext: &[u8], key: &Zeroizing<[u8; 32]>) -> Result<EncryptedData> {
        // Generate random salt
        let salt = KeyDerivation::generate_salt();

        // Generate random nonce
        let rng = SystemRandom::new();
        let mut nonce = [0u8; NONCE_LEN];
        rng.fill(&mut nonce)
            .map_err(|e| Error::encryption(format!("Failed to generate nonce: {}", e)))?;

        // Create sealing key
        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &**key)
            .map_err(|e| Error::encryption(format!("Failed to create key: {}", e)))?;
        let sealing_key = LessSafeKey::new(unbound_key);

        // Encrypt data
        let mut in_out = plaintext.to_vec();
        let tag = sealing_key
            .seal_in_place_separate_tag(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut in_out)
            .map_err(|e| Error::encryption(format!("Ring encryption failed: {}", e)))?;

        // Combine ciphertext and tag
        let mut ciphertext = in_out;
        ciphertext.extend_from_slice(tag.as_ref());

        Ok(EncryptedData::new(
            ciphertext,
            nonce.to_vec(),
            salt.to_vec(),
            EncryptionAlgorithm::Ring,
            100_000, // Default iterations
        ))
    }

    fn decrypt_ring(
        &self,
        encrypted: &EncryptedData,
        key: &Zeroizing<[u8; 32]>,
    ) -> Result<Zeroizing<Vec<u8>>> {
        // Get ciphertext and nonce
        let mut ciphertext = encrypted.ciphertext_bytes()?;
        let nonce_bytes = encrypted.nonce_bytes()?;

        if nonce_bytes.len() != NONCE_LEN {
            return Err(Error::encryption(format!(
                "Invalid nonce length: expected {}, got {}",
                NONCE_LEN,
                nonce_bytes.len()
            )));
        }

        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&nonce_bytes);

        // Create opening key
        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &**key)
            .map_err(|e| Error::encryption(format!("Failed to create key: {}", e)))?;
        let opening_key = LessSafeKey::new(unbound_key);

        // Decrypt data
        opening_key
            .open_in_place(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut ciphertext)
            .map_err(|e| Error::encryption(format!("Ring decryption failed: {}", e)))?;

        // Remove tag from the end
        let tag_len = aead::AES_256_GCM.tag_len();
        ciphertext.truncate(ciphertext.len() - tag_len);

        Ok(Zeroizing::new(ciphertext))
    }
}

/// Utility functions for common encryption operations
pub mod utils {
    use super::*;

    /// Encrypt data with a passphrase (derives key automatically)
    pub fn encrypt_with_passphrase(
        plaintext: &[u8],
        passphrase: &Zeroizing<String>,
        algorithm: EncryptionAlgorithm,
        kdf_iterations: u32,
    ) -> Result<EncryptedData> {
        // Generate random salt
        let salt = KeyDerivation::generate_salt();

        // Derive key from passphrase
        let key = KeyDerivation::pbkdf2(passphrase, &*salt, kdf_iterations);

        // Create encryption service and encrypt
        let encryption = EncryptionService::new(algorithm);
        let mut encrypted = encryption.encrypt(plaintext, &key)?;

        // Update metadata
        encrypted.kdf_iterations = kdf_iterations;

        Ok(encrypted)
    }

    /// Decrypt data with a passphrase (derives key automatically)
    pub fn decrypt_with_passphrase(
        encrypted: &EncryptedData,
        passphrase: &Zeroizing<String>,
    ) -> Result<Zeroizing<Vec<u8>>> {
        // Validate encrypted data
        encrypted.validate()?;

        // Get salt
        let salt = encrypted.salt_bytes()?;

        // Derive key from passphrase
        let key = KeyDerivation::pbkdf2(passphrase, &salt, encrypted.kdf_iterations);

        // Create encryption service and decrypt
        let encryption = EncryptionService::new(encrypted.algorithm);
        encryption.decrypt(encrypted, &key)
    }

    /// Generate a new random passphrase
    pub fn generate_passphrase(length: usize) -> Zeroizing<String> {
        use rand::distributions::{Alphanumeric, DistString};

        let passphrase = Alphanumeric.sample_string(&mut rand::rngs::OsRng, length);
        Zeroizing::new(passphrase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_derivation() {
        let passphrase = Zeroizing::new("test-passphrase".to_string());
        let salt = [0u8; 16];
        let iterations = 1000;

        let key1 = KeyDerivation::pbkdf2(&passphrase, &salt, iterations);
        let key2 = KeyDerivation::pbkdf2(&passphrase, &salt, iterations);

        // Same inputs should produce same key
        assert_eq!(&**key1, &**key2);

        // Different salt should produce different key
        let different_salt = [1u8; 16];
        let key3 = KeyDerivation::pbkdf2(&passphrase, &different_salt, iterations);
        assert_ne!(&**key1, &**key3);

        // Different iterations should produce different key
        let key4 = KeyDerivation::pbkdf2(&passphrase, &salt, iterations + 1);
        assert_ne!(&**key1, &**key4);
    }

    #[test]
    fn test_aes_gcm_encryption_decryption() -> Result<()> {
        let encryption = EncryptionService::new_aes_gcm();
        let key = EncryptionService::generate_key();

        let plaintext = b"Hello, world! This is a secret message.";

        // Encrypt
        let encrypted = encryption.encrypt(plaintext, &key)?;
        assert_eq!(encrypted.algorithm, EncryptionAlgorithm::Aes256Gcm);
        assert!(encrypted.ciphertext.len() > 0);
        assert!(encrypted.nonce.len() > 0);
        assert!(encrypted.salt.len() > 0);

        // Decrypt
        let decrypted = encryption.decrypt(&encrypted, &key)?;
        assert_eq!(plaintext, decrypted.as_slice());

        // Different key should fail
        let different_key = EncryptionService::generate_key();
        assert!(encryption.decrypt(&encrypted, &different_key).is_err());

        Ok(())
    }

    #[test]
    fn test_ring_encryption_decryption() -> Result<()> {
        let encryption = EncryptionService::new_ring();
        let key = EncryptionService::generate_key();

        let plaintext = b"Hello, world! This is a secret message.";

        // Encrypt
        let encrypted = encryption.encrypt(plaintext, &key)?;
        assert_eq!(encrypted.algorithm, EncryptionAlgorithm::Ring);
        assert!(encrypted.ciphertext.len() > 0);
        assert!(encrypted.nonce.len() > 0);
        assert!(encrypted.salt.len() > 0);

        // Decrypt
        let decrypted = encryption.decrypt(&encrypted, &key)?;
        assert_eq!(plaintext, decrypted.as_slice());

        // Different key should fail
        let different_key = EncryptionService::generate_key();
        assert!(encryption.decrypt(&encrypted, &different_key).is_err());

        Ok(())
    }

    #[test]
    fn test_passphrase_encryption() -> Result<()> {
        let passphrase = Zeroizing::new("super-secret-passphrase".to_string());
        let plaintext = b"Very sensitive data";

        // Encrypt with passphrase
        let encrypted = utils::encrypt_with_passphrase(
            plaintext,
            &passphrase,
            EncryptionAlgorithm::Aes256Gcm,
            100_000,
        )?;

        // Decrypt with same passphrase
        let decrypted = utils::decrypt_with_passphrase(&encrypted, &passphrase)?;
        assert_eq!(plaintext, decrypted.as_slice());

        // Wrong passphrase should fail
        let wrong_passphrase = Zeroizing::new("wrong-passphrase".to_string());
        assert!(utils::decrypt_with_passphrase(&encrypted, &wrong_passphrase).is_err());

        Ok(())
    }

    #[test]
    fn test_encrypted_data_validation() {
        let valid_data = EncryptedData {
            ciphertext: STANDARD.encode(b"ciphertext"),
            nonce: STANDARD.encode(b"nonce"),
            salt: STANDARD.encode(b"salt"),
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            kdf_iterations: 100_000,
            version: EncryptedData::CURRENT_VERSION,
        };

        assert!(valid_data.validate().is_ok());

        // Invalid base64 should fail
        let invalid_base64 = EncryptedData {
            ciphertext: "not-base64".to_string(),
            nonce: STANDARD.encode(b"nonce"),
            salt: STANDARD.encode(b"salt"),
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            kdf_iterations: 100_000,
            version: EncryptedData::CURRENT_VERSION,
        };

        assert!(invalid_base64.validate().is_err());

        // Wrong version should fail
        let wrong_version = EncryptedData {
            ciphertext: STANDARD.encode(b"ciphertext"),
            nonce: STANDARD.encode(b"nonce"),
            salt: STANDARD.encode(b"salt"),
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            kdf_iterations: 100_000,
            version: 255,
        };

        assert!(wrong_version.validate().is_err());
    }

    #[test]
    fn test_generate_passphrase() {
        let passphrase = utils::generate_passphrase(32);
        assert_eq!(passphrase.len(), 32);
        assert!(passphrase.chars().all(|c| c.is_ascii_alphanumeric()));
    }
}
