//! RPC client management for AI Agent Wallet
//!
//! This module provides enhanced RPC client functionality with:
//! - Connection pooling for efficient RPC usage
//! - Automatic failover between multiple endpoints
//! - Comprehensive metrics and monitoring
//! - Configurable timeouts and retry policies
//! - Support for different commitment levels
//!
//! # Example
//!
//! ```no_run
//! use agent_wallet_core::rpc::{RpcClient, RpcClientConfig};
//! use agent_wallet_core::config::{RpcSettings, RpcEndpoint, CommitmentLevel};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create RPC configuration
//!     let settings = RpcSettings {
//!         endpoints: vec![
//!             RpcEndpoint::new("https://api.mainnet-beta.solana.com"),
//!             RpcEndpoint::with_priority("https://solana-api.projectserum.com", 2),
//!         ],
//!         timeout_seconds: 30,
//!         commitment: CommitmentLevel::Confirmed,
//!         use_websocket: true,
//!         websocket_url: None,
//!     };
//!
//!     // Create RPC client
//!     let config = RpcClientConfig::from_settings(&settings);
//!     let rpc_client = RpcClient::new(config).await?;
//!
//!     // Use the client
//!     let balance = rpc_client.get_balance(&Pubkey::new_unique()).await?;
//!     println!("Balance: {} lamports", balance);
//!
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::future::BoxFuture;
use prometheus::{Histogram, HistogramOpts, IntCounter, IntCounterVec, Registry};
use solana_client::{
    client_error::ClientError as SolanaClientError,
    nonblocking::rpc_client::RpcClient as SolanaRpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcSendTransactionConfig},
    rpc_request::RpcRequest,
    rpc_response::{RpcAccountInfo, RpcKeyedAccount, RpcLogsResponse, RpcVote},
};
use solana_sdk::{
    account::Account,
    clock::Slot,
    commitment_config::CommitmentConfig,
    epoch_info::EpochInfo,
    hash::Hash,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::Signature,
    signer::Signer,
    transaction::Transaction,
};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, instrument, warn};

use crate::config::{CommitmentLevel, RpcEndpoint, RpcSettings};
use crate::error::{Error, Result};

/// RPC client configuration
#[derive(Debug, Clone)]
pub struct RpcClientConfig {
    /// RPC endpoints with priorities
    pub endpoints: Vec<RpcEndpoint>,
    /// Request timeout
    pub timeout: Duration,
    /// Default commitment level
    pub commitment: CommitmentConfig,
    /// Whether to use websocket for subscriptions
    pub use_websocket: bool,
    /// Maximum connections per endpoint
    pub max_connections_per_endpoint: usize,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl RpcClientConfig {
    /// Create configuration from RpcSettings
    pub fn from_settings(settings: &RpcSettings) -> Self {
        Self {
            endpoints: settings.endpoints.clone(),
            timeout: Duration::from_secs(settings.timeout_seconds),
            commitment: settings.commitment.to_solana_commitment(),
            use_websocket: settings.use_websocket,
            max_connections_per_endpoint: 10,
            max_retries: 3,
            retry_delay_ms: 100,
            enable_metrics: true,
        }
    }

    /// Create configuration with a single endpoint
    pub fn single_endpoint(url: impl Into<String>) -> Self {
        Self {
            endpoints: vec![RpcEndpoint::new(url)],
            timeout: Duration::from_secs(30),
            commitment: CommitmentConfig::confirmed(),
            use_websocket: true,
            max_connections_per_endpoint: 10,
            max_retries: 3,
            retry_delay_ms: 100,
            enable_metrics: true,
        }
    }
}

impl Default for RpcClientConfig {
    fn default() -> Self {
        Self {
            endpoints: vec![RpcEndpoint::new("https://api.devnet.solana.com")],
            timeout: Duration::from_secs(30),
            commitment: CommitmentConfig::confirmed(),
            use_websocket: true,
            max_connections_per_endpoint: 10,
            max_retries: 3,
            retry_delay_ms: 100,
            enable_metrics: true,
        }
    }
}

/// RPC connection pool entry
struct PooledConnection {
    client: SolanaRpcClient,
    in_use: bool,
    last_used: std::time::Instant,
}

/// Connection pool for an endpoint
struct EndpointPool {
    connections: Vec<PooledConnection>,
    endpoint: RpcEndpoint,
    max_connections: usize,
}

impl EndpointPool {
    /// Get an available connection or create a new one
    async fn get_connection(&mut self) -> Result<&mut SolanaRpcClient> {
        // Try to find an available connection
        for connection in &mut self.connections {
            if !connection.in_use {
                connection.in_use = true;
                connection.last_used = std::time::Instant::now();
                return Ok(&mut connection.client);
            }
        }

        // Create new connection if under limit
        if self.connections.len() < self.max_connections {
            let client = SolanaRpcClient::new_with_timeout(
                self.endpoint.url.clone(),
                Duration::from_secs(30),
            );

            let pooled = PooledConnection {
                client,
                in_use: true,
                last_used: std::time::Instant::now(),
            };

            self.connections.push(pooled);
            return Ok(&mut self.connections.last_mut().unwrap().client);
        }

        // All connections are in use
        Err(Error::rpc("All connections are in use"))
    }

    /// Release a connection back to the pool
    fn release_connection(&mut self, client: &SolanaRpcClient) {
        for connection in &mut self.connections {
            if std::ptr::eq(&connection.client, client) {
                connection.in_use = false;
                break;
            }
        }
    }

    /// Clean up idle connections
    fn cleanup_idle_connections(&mut self, max_idle_time: Duration) {
        let now = std::time::Instant::now();
        self.connections.retain(|conn| {
            if !conn.in_use && now.duration_since(conn.last_used) > max_idle_time {
                false // Remove idle connection
            } else {
                true // Keep connection
            }
        });
    }
}

/// RPC metrics for monitoring
#[derive(Clone)]
struct RpcMetrics {
    request_count: IntCounterVec,
    request_duration: Histogram,
    error_count: IntCounterVec,
    endpoint_switch_count: IntCounter,
}

impl RpcMetrics {
    /// Create new metrics
    fn new(registry: &Registry) -> Result<Self> {
        let request_count = IntCounterVec::new(
            IntCounter::opts(
                "agent_wallet_rpc_requests_total",
                "Total number of RPC requests",
            ),
            &["endpoint", "method", "status"],
        )?;

        let request_duration = Histogram::with_opts(
            HistogramOpts::new(
                "agent_wallet_rpc_request_duration_seconds",
                "RPC request duration in seconds",
            )
        )?;

        let error_count = IntCounterVec::new(
            IntCounter::opts(
                "agent_wallet_rpc_errors_total",
                "Total number of RPC errors",
            ),
            &["endpoint", "error_type"],
        )?;

        let endpoint_switch_count = IntCounter::new(
            "agent_wallet_rpc_endpoint_switches_total",
            "Total number of endpoint switches",
        )?;

        registry.register(Box::new(request_count.clone()))?;
        registry.register(Box::new(request_duration.clone()))?;
        registry.register(Box::new(error_count.clone()))?;
        registry.register(Box::new(endpoint_switch_count.clone()))?;

        Ok(Self {
            request_count,
            request_duration,
            error_count,
            endpoint_switch_count,
        })
    }
}

/// Enhanced RPC client with connection pooling and failover
pub struct RpcClient {
    /// Endpoint connection pools
    endpoint_pools: Arc<RwLock<HashMap<String, EndpointPool>>>,
    /// Current active endpoint
    current_endpoint: Arc<Mutex<RpcEndpoint>>,
    /// Configuration
    config: RpcClientConfig,
    /// Metrics collector
    metrics: Option<RpcMetrics>,
    /// Endpoint health status
    endpoint_health: Arc<RwLock<HashMap<String, EndpointHealth>>>,
}

/// Endpoint health tracking
#[derive(Debug, Clone)]
struct EndpointHealth {
    last_success: Option<std::time::Instant>,
    last_failure: Option<std::time::Instant>,
    consecutive_failures: u32,
    total_requests: u64,
    total_errors: u64,
    success_rate: f64,
}

impl EndpointHealth {
    fn new() -> Self {
        Self {
            last_success: None,
            last_failure: None,
            consecutive_failures: 0,
            total_requests: 0,
            total_errors: 0,
            success_rate: 1.0,
        }
    }

    fn record_success(&mut self) {
        self.last_success = Some(std::time::Instant::now());
        self.consecutive_failures = 0;
        self.total_requests += 1;
        self.success_rate = (self.success_rate * 0.95) + (1.0 * 0.05);
    }

    fn record_failure(&mut self) {
        self.last_failure = Some(std::time::Instant::now());
        self.consecutive_failures += 1;
        self.total_requests += 1;
        self.total_errors += 1;
        self.success_rate = (self.success_rate * 0.95) + (0.0 * 0.05);
    }

    fn is_healthy(&self, max_consecutive_failures: u32) -> bool {
        self.consecutive_failures < max_consecutive_failures
    }
}

impl RpcClient {
    /// Create a new RPC client
    #[instrument(skip(config))]
    pub async fn new(config: RpcClientConfig) -> Result<Self> {
        info!("Creating RPC client with {} endpoints", config.endpoints.len());

        let mut endpoint_pools = HashMap::new();
        let mut endpoint_health = HashMap::new();

        // Initialize pools for each endpoint
        for endpoint in &config.endpoints {
            endpoint_pools.insert(
                endpoint.url.clone(),
                EndpointPool {
                    connections: Vec::new(),
                    endpoint: endpoint.clone(),
                    max_connections: config.max_connections_per_endpoint,
                },
            );

            endpoint_health.insert(endpoint.url.clone(), EndpointHealth::new());
        }

        // Get highest priority endpoint
        let current_endpoint = config.endpoints.iter()
            .min_by_key(|e| e.priority)
            .cloned()
            .ok_or_else(|| Error::config("No RPC endpoints configured"))?;

        // Initialize metrics if enabled
        let metrics = if config.enable_metrics {
            let registry = Registry::new();
            Some(RpcMetrics::new(&registry)?)
        } else {
            None
        };

        Ok(Self {
            endpoint_pools: Arc::new(RwLock::new(endpoint_pools)),
            current_endpoint: Arc::new(Mutex::new(current_endpoint)),
            config,
            metrics,
            endpoint_health: Arc::new(RwLock::new(endpoint_health)),
        })
    }

    /// Execute an RPC request with automatic failover
    #[instrument(skip(self, f))]
    async fn execute_with_failover<T, F>(&self, f: F) -> Result<T>
    where
        F: Fn(&SolanaRpcClient) -> BoxFuture<'_, std::result::Result<T, SolanaClientError>>,
    {
        let mut retries = 0;
        let mut last_error = None;

        while retries <= self.config.max_retries {
            let endpoint = self.current_endpoint.lock().await.clone();
            let endpoint_url = endpoint.url.clone();

            // Get connection from pool
            let connection = {
                let mut pools = self.endpoint_pools.write().await;
                let pool = pools.get_mut(&endpoint_url)
                    .ok_or_else(|| Error::rpc(format!("Endpoint not found: {}", endpoint_url)))?;

                pool.get_connection().await?
            };

            // Record start time for metrics
            let start_time = std::time::Instant::now();

            // Execute request
            match f(connection).await {
                Ok(result) => {
                    // Record success
                    let duration = start_time.elapsed();
                    self.record_success(&endpoint_url, duration).await;

                    // Release connection
                    {
                        let mut pools = self.endpoint_pools.write().await;
                        if let Some(pool) = pools.get_mut(&endpoint_url) {
                            pool.release_connection(connection);
                        }
                    }

                    return Ok(result);
                }
                Err(err) => {
                    // Record failure
                    let duration = start_time.elapsed();
                    self.record_failure(&endpoint_url, &err, duration).await;

                    // Release connection
                    {
                        let mut pools = self.endpoint_pools.write().await;
                        if let Some(pool) = pools.get_mut(&endpoint_url) {
                            pool.release_connection(connection);
                        }
                    }

                    last_error = Some(err);

                    // Check if we should switch endpoints
                    if self.should_switch_endpoint(&endpoint_url).await {
                        if let Err(e) = self.switch_to_next_endpoint().await {
                            warn!("Failed to switch endpoint: {}", e);
                        }
                    }

                    retries += 1;

                    // Delay before retry
                    if retries <= self.config.max_retries {
                        tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                    }
                }
            }
        }

        Err(Error::rpc(format!(
            "All retries failed. Last error: {:?}",
            last_error
        )))
    }

    /// Record successful request
    async fn record_success(&self, endpoint_url: &str, duration: Duration) {
        // Update health
        {
            let mut health_map = self.endpoint_health.write().await;
            if let Some(health) = health_map.get_mut(endpoint_url) {
                health.record_success();
            }
        }

        // Update metrics
        if let Some(metrics) = &self.metrics {
            metrics.request_duration.observe(duration.as_secs_f64());
            // Note: We'd need the method name to properly label the metric
            // This would require passing additional context
        }
    }

    /// Record failed request
    async fn record_failure(&self, endpoint_url: &str, error: &SolanaClientError, duration: Duration) {
        // Update health
        {
            let mut health_map = self.endpoint_health.write().await;
            if let Some(health) = health_map.get_mut(endpoint_url) {
                health.record_failure();
            }
        }

        // Update metrics
        if let Some(metrics) = &self.metrics {
            let error_type = match error {
                SolanaClientError::Io(_) => "io",
                SolanaClientError::Reqwest(_) => "reqwest",
                SolanaClientError::RpcError(_) => "rpc",
                SolanaClientError::SerdeJson(_) => "serde_json",
                SolanaClientError::SerdeYaml(_) => "serde_yaml",
                SolanaClientError::TransactionError(_) => "transaction",
                SolanaClientError::InvalidParams(_) => "invalid_params",
                SolanaClientError::DecodeError(_) => "decode",
                SolanaClientError::TooManyRequests => "too_many_requests",
                SolanaClientError::EmptyResponse => "empty_response",
                SolanaClientError::Custom(_) => "custom",
            };

            metrics.error_count
                .with_label_values(&[endpoint_url, error_type])
                .inc();
        }
    }

    /// Check if we should switch to another endpoint
    async fn should_switch_endpoint(&self, endpoint_url: &str) -> bool {
        let health_map = self.endpoint_health.read().await;
        if let Some(health) = health_map.get(endpoint_url) {
            // Switch if we have too many consecutive failures
            if health.consecutive_failures >= 3 {
                return true;
            }

            // Switch if success rate is too low
            if health.success_rate < 0.5 && health.total_requests > 10 {
                return true;
            }
        }

        false
    }

    /// Switch to the next healthy endpoint
    async fn switch_to_next_endpoint(&self) -> Result<()> {
        let health_map = self.endpoint_health.read().await;
        let pools = self.endpoint_pools.read().await;

        // Get all endpoints sorted by priority
        let mut endpoints: Vec<_> = pools.keys().collect();
        endpoints.sort_by_key(|url| {
            pools.get(*url).map(|p| p.endpoint.priority).unwrap_or(u32::MAX)
        });

        // Find first healthy endpoint
        for endpoint_url in endpoints {
            if let Some(health) = health_map.get(endpoint_url) {
                if health.is_healthy(3) {
                    if let Some(pool) = pools.get(endpoint_url) {
                        let new_endpoint = pool.endpoint.clone();

                        // Update current endpoint
                        let mut current = self.current_endpoint.lock().await;
                        *current = new_endpoint;

                        // Record switch in metrics
                        if let Some(metrics) = &self.metrics {
                            metrics.endpoint_switch_count.inc();
                        }

                        info!("Switched to endpoint: {}", endpoint_url);
                        return Ok(());
                    }
                }
            }
        }

        Err(Error::rpc("No healthy endpoints available"))
    }

    /// Get the current endpoint URL
    pub async fn current_endpoint_url(&self) -> String {
        let endpoint = self.current_endpoint.lock().await;
        endpoint.url.clone()
    }

    /// Get endpoint health information
    pub async fn get_endpoint_health(&self) -> HashMap<String, EndpointHealth> {
        self.endpoint_health.read().await.clone()
    }

    /// Clean up idle connections
    pub async fn cleanup_idle_connections(&self, max_idle_time: Duration) {
        let mut pools = self.endpoint_pools.write().await;
        for pool in pools.values_mut() {
            pool.cleanup_idle_connections(max_idle_time);
        }
    }
}

// Implement common RPC methods
impl RpcClient {
    /// Get account balance
    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        self.execute_with_failover(|client| {
            Box::pin(client.get_balance_with_commitment(pubkey, self.config.commitment))
        })
        .await
        .map_err(|e| Error::SolanaRpc(e))
    }

    /// Get account information
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Account> {
        self.execute_with_failover(|client| {
            Box::pin(client.get_account_with_commitment(pubkey, self.config.commitment))
        })
        .await
        .map_err(|e| Error::SolanaRpc(e))
    }

    /// Get multiple accounts
    pub async fn get_multiple_accounts(&self, pubkeys: &[Pubkey]) -> Result<Vec<Option<Account>>> {
        self.execute_with_failover(|client| {
            Box::pin(client.get_multiple_accounts_with_commitment(pubkeys, self.config.commitment))
        })
        .await
        .map_err(|e| Error::SolanaRpc(e))
    }

    /// Get latest blockhash
    pub async fn get_latest_blockhash(&self) -> Result<Hash> {
        self.execute_with_failover(|client| {
            Box::pin(client.get_latest_blockhash_with_commitment(self.config.commitment))
        })
        .await
        .map_err(|e| Error::SolanaRpc(e))
    }

    /// Send transaction
    pub async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature> {
        let config = RpcSendTransactionConfig {
            skip_preflight: false,
            preflight_commitment: Some(self.config.commitment.commitment),
            encoding: None,
            max_retries: None,
            min_context_slot: None,
        };

        self.execute_with_failover(|client| {
            Box::pin(client.send_transaction_with_config(transaction, config))
        })
        .await
        .map_err(|e| Error::SolanaRpc(e))
    }

    /// Simulate transaction
    pub async fn simulate_transaction(&self, transaction: &Transaction) -> Result<solana_client::rpc_response::RpcSimulateTransactionResult> {
        self.execute_with_failover(|client| {
            Box::pin(client.simulate_transaction(transaction))
        })
        .await
        .map_err(|e| Error::SolanaRpc(e))
    }

    /// Get transaction
    pub async fn get_transaction(&self, signature: &Signature) -> Result<solana_client::rpc_response::RpcTransactionInfo> {
        self.execute_with_failover(|client| {
            Box::pin(client.get_transaction(signature, solana_transaction_status::UiTransactionEncoding::Json))
        })
        .await
        .map_err(|e| Error::SolanaRpc(e))
    }

    /// Get slot
    pub async fn get_slot(&self) -> Result<Slot> {
        self.execute_with_failover(|client| {
            Box::pin(client.get_slot_with_commitment(self.config.commitment))
        })
        .await
        .map_err(|e| Error::SolanaRpc(e))
    }

    /// Get epoch information
    pub async fn get_epoch_info(&self) -> Result<EpochInfo> {
        self.execute_with_failover(|client| {
            Box::pin(client.get_epoch_info_with_commitment(self.config.commitment))
        })
        .await
        .map_err(|e| Error::SolanaRpc(e))
    }

    /// Get minimum balance for rent exemption
    pub async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> Result<u64> {
        self.execute_with_failover(|client| {
            Box::pin(client.get_minimum_balance_for_rent_exemption(data_len))
        })
        .await
        .map_err(|e| Error::SolanaRpc(e))
    }

    /// Get token account balance
    pub async fn get_token_account_balance(&self, token_account: &Pubkey) -> Result<solana_account_decoder::UiTokenAmount> {
        self.execute_with_failover(|client| {
            Box::pin(client.get_token_account_balance_with_commitment(token_account, self.config.commitment))
        })
        .await
        .map_err(|e| Error::SolanaRpc(e))
    }

    /// Get program accounts
    pub async fn get_program_accounts(&self, program_id: &Pubkey, config: Option<RpcProgramAccountsConfig>) -> Result<Vec<RpcKeyedAccount>> {
        self.execute_with_failover(|client| {
            Box::pin(client.get_program_accounts_with_config(program_id, config.unwrap_or_default()))
        })
        .await
        .map_err(|e| Error::SolanaRpc(e))
    }
}

// Provide a compatibility layer for existing code expecting a SolanaRpcClient
#[async_trait]
pub trait RpcClientExt {
    /// Get a reference to the underlying Solana RPC client for the current endpoint
    async fn get_inner_client(&self) -> Result<impl std::ops::Deref<Target = SolanaRpcClient>>;

    /// Get commitment configuration
    fn get_commitment_config(&self) -> CommitmentConfig;

    /// Get request timeout
    fn get_timeout(&self) -> Duration;
}

#[async_trait]
impl RpcClientExt for RpcClient {
    async fn get_inner_client(&self) -> Result<impl std::ops::Deref<Target = SolanaRpcClient>> {
        let endpoint_url = self.current_endpoint_url().await;
        let mut pools = self.endpoint_pools.write().await;

        let pool = pools.get_mut(&endpoint_url)
            .ok_or_else(|| Error::rpc(format!("Endpoint not found: {}", endpoint_url)))?;

        let connection = pool.get_connection().await?;

        // We need to return something that holds the lock and can release it
        // This is a simplified approach - in production you'd want a guard type
        Ok(connection)
    }

    fn get_commitment_config(&self) -> CommitmentConfig {
        self.config.commitment
    }

    fn get_timeout(&self) -> Duration {
        self.config.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rpc_client_creation() -> Result<()> {
        let config = RpcClientConfig::single_endpoint("https://api.devnet.solana.com");
        let client = RpcClient::new(config).await?;

        let current_endpoint = client.current_endpoint_url().await;
        assert_eq!(current_endpoint, "https://api.devnet.solana.com");

        Ok(())
    }

    #[tokio::test]
    async fn test_endpoint_health_tracking() -> Result<()> {
        let config = RpcClientConfig {
            endpoints: vec![
                RpcEndpoint::new("https://endpoint1.com"),
                RpcEndpoint::new("https://endpoint2.com"),
            ],
            ..Default::default()
        };

        let client = RpcClient::new(config).await?;
        let health = client.get_endpoint_health().await;

        assert_eq!(health.len(), 2);
        assert!(health.contains_key("https://endpoint1.com"));
        assert!(health.contains_key("https://endpoint2.com"));

        Ok(())
    }

    #[test]
    fn test_endpoint_health_calculation() {
        let mut health = EndpointHealth::new();

        // Initial state
        assert_eq!(health.success_rate, 1.0);
        assert_eq!(health.consecutive_failures, 0);

        // Record success
        health.record_success();
        assert_eq!(health.consecutive_failures, 0);
        assert!(health.last_success.is_some());

        // Record failure
        health.record_failure();
        assert_eq!(health.consecutive_failures, 1);
        assert!(health.last_failure.is_some());
        assert!(health.success_rate < 1.0);

        // Check health status
        assert!(health.is_healthy(3));
        assert!(!health.is_healthy(1));
    }

    #[test]
    fn test_config_from_settings() {
        let settings = RpcSettings {
            endpoints: vec![RpcEndpoint::new("https://test.com")],
            timeout_seconds: 60,
            commitment: CommitmentLevel::Finalized,
            use_websocket: false,
            websocket_url: None,
        };

        let config = RpcClientConfig::from_settings(&settings);

        assert_eq!(config.endpoints.len(), 1);
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.commitment.commitment, solana_sdk::commitment_config::CommitmentLevel::Finalized);
        assert!(!config.use_websocket);
    }
}
