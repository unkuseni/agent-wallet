//! Configuration management for AI Agent Wallet
//!
//! This module provides configuration structures and loading utilities
//! for the AI Agent Wallet system. Configuration can be loaded from
//! multiple sources with the following priority order:
//! 1. Command line arguments (highest priority)
//! 2. Environment variables
//! 3. Configuration files (YAML/JSON)
//! 4. Default values
//!
//! # Example
//!
//! ```no_run
//! use agent_wallet_core::config::{WalletConfig, RpcEndpoint};
//!
//! // Create configuration with builder pattern
//! let config = WalletConfig::builder()
//!     .with_rpc_endpoint(RpcEndpoint::new("https://api.devnet.solana.com"))
//!     .with_daily_spend_limit(10.0)
//!     .build();
//!
//! // Load configuration from file
//! let config = WalletConfig::from_file("config.yaml")?;
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{Error, Result};
use crate::types::PermissionLevel;

/// Main configuration structure for the wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WalletConfig {
    /// Wallet-specific configuration
    pub wallet: WalletSettings,
    /// Agent-specific configuration
    pub agent: AgentSettings,
    /// RPC client configuration
    pub rpc: RpcSettings,
    /// Monitoring and observability configuration
    pub monitoring: MonitoringSettings,
}

/// Wallet-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WalletSettings {
    /// Encryption configuration
    pub encryption: EncryptionSettings,
    /// Storage configuration
    pub storage: StorageSettings,
}

/// Encryption algorithm configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM encryption (default)
    Aes256Gcm,
    /// Ring-based encryption (alternative)
    Ring,
}

/// Encryption settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EncryptionSettings {
    /// Encryption algorithm to use
    pub algorithm: EncryptionAlgorithm,
    /// Number of PBKDF2 iterations for key derivation
    pub kdf_iterations: u32,
}

/// Storage settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageSettings {
    /// Primary wallet storage directory
    pub path: PathBuf,
    /// Backup wallet storage directory
    pub backup_path: PathBuf,
    /// Maximum number of wallet versions to keep
    pub max_versions: usize,
}

/// Agent-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentSettings {
    /// Sandbox configuration for agent execution
    pub sandbox: SandboxSettings,
    /// Operational limits for agents
    pub limits: AgentLimits,
    /// Default permission level for new agents
    pub default_permission_level: PermissionLevel,
}

/// Sandbox execution settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxSettings {
    /// Whether sandbox execution is enabled
    pub enabled: bool,
    /// Memory limit in megabytes
    pub memory_limit_mb: u64,
    /// CPU limit as percentage (0-100)
    pub cpu_limit_percent: u8,
    /// Timeout for agent decisions
    pub decision_timeout_seconds: u64,
}

/// Agent operational limits
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentLimits {
    /// Daily spending limit in SOL
    pub daily_spend_limit_sol: f64,
    /// Maximum transactions per minute
    pub max_transactions_per_minute: u32,
    /// Maximum transaction size in bytes
    pub max_transaction_size: usize,
    /// Maximum number of signatures per transaction
    pub max_signatures: u8,
}

/// RPC client settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RpcSettings {
    /// List of RPC endpoints with priorities
    pub endpoints: Vec<RpcEndpoint>,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Default commitment level
    pub commitment: CommitmentLevel,
    /// Whether to use websocket for subscriptions
    pub use_websocket: bool,
    /// Websocket endpoint (if different from HTTP)
    pub websocket_url: Option<String>,
}

/// RPC endpoint with priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEndpoint {
    /// RPC URL
    pub url: String,
    /// Priority (lower number = higher priority)
    pub priority: u32,
    /// Optional authentication token
    pub auth_token: Option<String>,
}

/// Solana commitment level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommitmentLevel {
    /// Processed commitment (least secure, fastest)
    Processed,
    /// Confirmed commitment (balanced)
    Confirmed,
    /// Finalized commitment (most secure, slowest)
    Finalized,
}

/// Monitoring and observability settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MonitoringSettings {
    /// Metrics collection settings
    pub metrics: MetricsSettings,
    /// Logging configuration
    pub logging: LoggingSettings,
}

/// Metrics collection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MetricsSettings {
    /// Whether metrics collection is enabled
    pub enabled: bool,
    /// Metrics server port
    pub port: u16,
    /// Metrics collection interval in seconds
    pub interval_seconds: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingSettings {
    /// Log level (error, warn, info, debug, trace)
    pub level: LogLevel,
    /// Log format (text, json)
    pub format: LogFormat,
    /// Whether to log to file
    pub log_to_file: bool,
    /// Log file path (if logging to file)
    pub log_file: Option<PathBuf>,
}

/// Log level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Error level
    Error,
    /// Warning level
    Warn,
    /// Info level (default)
    Info,
    /// Debug level
    Debug,
    /// Trace level
    Trace,
}

/// Log format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Plain text format
    Text,
    /// JSON format
    Json,
}

// Default implementations

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            wallet: WalletSettings::default(),
            agent: AgentSettings::default(),
            rpc: RpcSettings::default(),
            monitoring: MonitoringSettings::default(),
        }
    }
}

impl Default for WalletSettings {
    fn default() -> Self {
        Self {
            encryption: EncryptionSettings::default(),
            storage: StorageSettings::default(),
        }
    }
}

impl Default for EncryptionSettings {
    fn default() -> Self {
        Self {
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            kdf_iterations: 100_000,
        }
    }
}

impl Default for StorageSettings {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

        Self {
            path: home_dir.join(".agent-wallet/wallets"),
            backup_path: home_dir.join(".agent-wallet/backups"),
            max_versions: 10,
        }
    }
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            sandbox: SandboxSettings::default(),
            limits: AgentLimits::default(),
            default_permission_level: PermissionLevel::Basic,
        }
    }
}

impl Default for SandboxSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            memory_limit_mb: 512,
            cpu_limit_percent: 50,
            decision_timeout_seconds: 30,
        }
    }
}

impl Default for AgentLimits {
    fn default() -> Self {
        Self {
            daily_spend_limit_sol: 10.0,
            max_transactions_per_minute: 10,
            max_transaction_size: 1232, // Solana transaction size limit
            max_signatures: 20,         // Solana max signatures per transaction
        }
    }
}

impl Default for RpcSettings {
    fn default() -> Self {
        Self {
            endpoints: vec![RpcEndpoint::new("https://api.devnet.solana.com")],
            timeout_seconds: 30,
            commitment: CommitmentLevel::Confirmed,
            use_websocket: true,
            websocket_url: None,
        }
    }
}

impl Default for MonitoringSettings {
    fn default() -> Self {
        Self {
            metrics: MetricsSettings::default(),
            logging: LoggingSettings::default(),
        }
    }
}

impl Default for MetricsSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9090,
            interval_seconds: 15,
        }
    }
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Text,
            log_to_file: false,
            log_file: None,
        }
    }
}

// Implementations

impl WalletConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration builder
    pub fn builder() -> WalletConfigBuilder {
        WalletConfigBuilder::new()
    }

    /// Load configuration from a YAML file
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| Error::config(format!("Failed to read config file: {}", e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| Error::config(format!("Failed to parse YAML config: {}", e)))
    }

    /// Load configuration from a JSON file
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| Error::config(format!("Failed to read config file: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| Error::config(format!("Failed to parse JSON config: {}", e)))
    }

    /// Load configuration from a file (auto-detects format by extension)
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "yaml" | "yml" => Self::from_yaml_file(path),
            "json" => Self::from_json_file(path),
            _ => Err(Error::config(format!(
                "Unsupported config file format: {}. Supported: .yaml, .yml, .json",
                extension
            ))),
        }
    }

    /// Save configuration to a YAML file
    pub fn save_to_yaml_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_yaml::to_string(self)
            .map_err(|e| Error::config(format!("Failed to serialize config to YAML: {}", e)))?;

        std::fs::write(path.as_ref(), content)
            .map_err(|e| Error::config(format!("Failed to write config file: {}", e)))
    }

    /// Save configuration to a JSON file
    pub fn save_to_json_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| Error::config(format!("Failed to serialize config to JSON: {}", e)))?;

        std::fs::write(path.as_ref(), content)
            .map_err(|e| Error::config(format!("Failed to write config file: {}", e)))
    }

    /// Get the request timeout as Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.rpc.timeout_seconds)
    }

    /// Get the highest priority RPC endpoint URL
    pub fn primary_rpc_url(&self) -> Option<&str> {
        self.rpc
            .endpoints
            .iter()
            .min_by_key(|endpoint| endpoint.priority)
            .map(|endpoint| endpoint.url.as_str())
    }

    /// Get websocket URL (falls back to primary RPC URL if not specified)
    pub fn websocket_url(&self) -> Option<&str> {
        if let Some(url) = &self.rpc.websocket_url {
            Some(url.as_str())
        } else if self.rpc.use_websocket {
            self.primary_rpc_url()
        } else {
            None
        }
    }
}

impl RpcEndpoint {
    /// Create a new RPC endpoint with default priority (1)
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            priority: 1,
            auth_token: None,
        }
    }

    /// Create a new RPC endpoint with custom priority
    pub fn with_priority(url: impl Into<String>, priority: u32) -> Self {
        Self {
            url: url.into(),
            priority,
            auth_token: None,
        }
    }

    /// Set authentication token for the endpoint
    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }
}

impl CommitmentLevel {
    /// Convert to Solana SDK commitment config
    pub fn to_solana_commitment(&self) -> solana_sdk::commitment_config::CommitmentConfig {
        match self {
            CommitmentLevel::Processed => {
                solana_sdk::commitment_config::CommitmentConfig::processed()
            }
            CommitmentLevel::Confirmed => {
                solana_sdk::commitment_config::CommitmentConfig::confirmed()
            }
            CommitmentLevel::Finalized => {
                solana_sdk::commitment_config::CommitmentConfig::finalized()
            }
        }
    }
}

impl LogLevel {
    /// Convert to tracing level
    pub fn to_tracing_level(&self) -> tracing::Level {
        match self {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }
}

/// Builder for WalletConfig with fluent interface
pub struct WalletConfigBuilder {
    config: WalletConfig,
}

impl WalletConfigBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: WalletConfig::default(),
        }
    }

    /// Set the wallet storage path
    pub fn with_storage_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.wallet.storage.path = path.into();
        self
    }

    /// Set the backup storage path
    pub fn with_backup_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.wallet.storage.backup_path = path.into();
        self
    }

    /// Set the encryption algorithm
    pub fn with_encryption_algorithm(mut self, algorithm: EncryptionAlgorithm) -> Self {
        self.config.wallet.encryption.algorithm = algorithm;
        self
    }

    /// Set the KDF iterations
    pub fn with_kdf_iterations(mut self, iterations: u32) -> Self {
        self.config.wallet.encryption.kdf_iterations = iterations;
        self
    }

    /// Enable or disable sandbox
    pub fn with_sandbox_enabled(mut self, enabled: bool) -> Self {
        self.config.agent.sandbox.enabled = enabled;
        self
    }

    /// Set sandbox memory limit in MB
    pub fn with_sandbox_memory_limit(mut self, mb: u64) -> Self {
        self.config.agent.sandbox.memory_limit_mb = mb;
        self
    }

    /// Set daily spend limit in SOL
    pub fn with_daily_spend_limit(mut self, limit: f64) -> Self {
        self.config.agent.limits.daily_spend_limit_sol = limit;
        self
    }

    /// Set maximum transactions per minute
    pub fn with_max_transactions_per_minute(mut self, max: u32) -> Self {
        self.config.agent.limits.max_transactions_per_minute = max;
        self
    }

    /// Add an RPC endpoint
    pub fn with_rpc_endpoint(mut self, endpoint: RpcEndpoint) -> Self {
        self.config.rpc.endpoints.push(endpoint);
        self
    }

    /// Set RPC timeout in seconds
    pub fn with_rpc_timeout(mut self, seconds: u64) -> Self {
        self.config.rpc.timeout_seconds = seconds;
        self
    }

    /// Set commitment level
    pub fn with_commitment(mut self, commitment: CommitmentLevel) -> Self {
        self.config.rpc.commitment = commitment;
        self
    }

    /// Set metrics enabled/disabled
    pub fn with_metrics_enabled(mut self, enabled: bool) -> Self {
        self.config.monitoring.metrics.enabled = enabled;
        self
    }

    /// Set metrics port
    pub fn with_metrics_port(mut self, port: u16) -> Self {
        self.config.monitoring.metrics.port = port;
        self
    }

    /// Set log level
    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        self.config.monitoring.logging.level = level;
        self
    }

    /// Build the final configuration
    pub fn build(self) -> WalletConfig {
        self.config
    }
}

impl Default for WalletConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = WalletConfig::default();

        assert_eq!(
            config.wallet.encryption.algorithm,
            EncryptionAlgorithm::Aes256Gcm
        );
        assert_eq!(config.wallet.encryption.kdf_iterations, 100_000);
        assert_eq!(config.agent.limits.daily_spend_limit_sol, 10.0);
        assert_eq!(config.agent.limits.max_transactions_per_minute, 10);
        assert_eq!(config.rpc.timeout_seconds, 30);
        assert_eq!(config.rpc.commitment, CommitmentLevel::Confirmed);
        assert!(config.rpc.endpoints.len() > 0);
    }

    #[test]
    fn test_builder_pattern() {
        let config = WalletConfig::builder()
            .with_daily_spend_limit(5.0)
            .with_max_transactions_per_minute(5)
            .with_rpc_endpoint(RpcEndpoint::new("https://custom.rpc.url"))
            .with_commitment(CommitmentLevel::Finalized)
            .build();

        assert_eq!(config.agent.limits.daily_spend_limit_sol, 5.0);
        assert_eq!(config.agent.limits.max_transactions_per_minute, 5);
        assert_eq!(config.rpc.commitment, CommitmentLevel::Finalized);
        assert!(config
            .rpc
            .endpoints
            .iter()
            .any(|e| e.url == "https://custom.rpc.url"));
    }

    #[test]
    fn test_config_serialization() {
        let config = WalletConfig::default();

        // Test YAML serialization
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("wallet:"));
        assert!(yaml.contains("agent:"));
        assert!(yaml.contains("rpc:"));

        // Test JSON serialization
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"wallet\""));
        assert!(json.contains("\"agent\""));
        assert!(json.contains("\"rpc\""));
    }

    #[test]
    fn test_config_file_io() -> Result<()> {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let original_config = WalletConfig::builder().with_daily_spend_limit(15.0).build();

        // Save to file
        original_config.save_to_yaml_file(&config_path)?;

        // Load from file
        let loaded_config = WalletConfig::from_file(&config_path)?;

        assert_eq!(loaded_config.agent.limits.daily_spend_limit_sol, 15.0);

        Ok(())
    }

    #[test]
    fn test_rpc_endpoint_priority() {
        let mut config = WalletConfig::default();

        // Add multiple endpoints with different priorities
        config.rpc.endpoints = vec![
            RpcEndpoint::with_priority("https://low.priority.url", 10),
            RpcEndpoint::with_priority("https://high.priority.url", 1),
            RpcEndpoint::with_priority("https://medium.priority.url", 5),
        ];

        let primary_url = config.primary_rpc_url();
        assert_eq!(primary_url, Some("https://high.priority.url"));
    }

    #[test]
    fn test_commitment_conversion() {
        let processed = CommitmentLevel::Processed.to_solana_commitment();
        let confirmed = CommitmentLevel::Confirmed.to_solana_commitment();
        let finalized = CommitmentLevel::Finalized.to_solana_commitment();

        assert_eq!(
            processed.commitment,
            solana_sdk::commitment_config::CommitmentLevel::Processed
        );
        assert_eq!(
            confirmed.commitment,
            solana_sdk::commitment_config::CommitmentLevel::Confirmed
        );
        assert_eq!(
            finalized.commitment,
            solana_sdk::commitment_config::CommitmentLevel::Finalized
        );
    }
}
