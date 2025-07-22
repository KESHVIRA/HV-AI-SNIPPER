use std::{env, fs, path::Path};
use serde::{Deserialize, Serialize};
use url::Url;
use thiserror::Error;

/// Configuration for the Solana Mempool Sniper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// RPC endpoint URL for the Solana node
    pub rpc_url: String,
    
    /// WebSocket endpoint URL for real-time updates
    pub ws_url: String,
    
    /// Maximum number of retries for failed operations
    pub max_retries: u32,
    
    /// Delay between retries in milliseconds
    pub retry_delay_ms: u64,
    
    /// Enable debug mode for more verbose logging
    #[serde(default)]
    pub debug: bool,
    
    /// Maximum number of concurrent tasks
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: usize,
    
    /// Maximum number of transactions to process per second (0 = unlimited)
    #[serde(default)]
    pub max_txs_per_second: u32,
}

/// Default number of max concurrent tasks
const fn default_max_concurrent_tasks() -> usize { 100 }

/// Errors that can occur when loading or validating configuration
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),
    
    #[error("Configuration validation failed: {0}")]
    ValidationError(String),
}

impl From<url::ParseError> for ConfigError {
    fn from(err: url::ParseError) -> Self {
        ConfigError::ValidationError(format!("Invalid URL: {}", err))
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Don't log sensitive information like private keys
        write!(
            f,
            "Config {{ rpc_url: {}, ws_url: {}, max_retries: {}, retry_delay_ms: {} }}",
            self.rpc_url, self.ws_url, self.max_retries, self.retry_delay_ms
        )
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rpc_url: env::var("RPC_URL").unwrap_or_else(|_| "https://mainnet.helius-rpc.com/?api-key=8c371c1f-7c94-4024-9722-076e10e56826".to_string()),
            ws_url: env::var("WS_URL").unwrap_or_else(|_| "wss://mainnet.helius-rpc.com/?api-key=8c371c1f-7c94-4024-9722-076e10e56826".to_string()),
            max_retries: 5,
            retry_delay_ms: 1000,
            debug: false,
            max_concurrent_tasks: default_max_concurrent_tasks(),
            max_txs_per_second: 0, // Unlimited by default
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&content)?;
        
        // Override with environment variables if they exist
        if let Ok(rpc_url) = env::var("RPC_URL") {
            config.rpc_url = rpc_url;
        }
        
        if let Ok(ws_url) = env::var("WS_URL") {
            config.ws_url = ws_url;
        }
        
        if let Ok(debug) = env::var("DEBUG") {
            config.debug = debug == "1" || debug.eq_ignore_ascii_case("true");
        }
        
        if let Ok(max_tasks) = env::var("MAX_CONCURRENT_TASKS") {
            if let Ok(max_tasks) = max_tasks.parse() {
                config.max_concurrent_tasks = max_tasks;
            }
        }
        
        if let Ok(max_txs) = env::var("MAX_TXS_PER_SECOND") {
            if let Ok(max_txs) = max_txs.parse() {
                config.max_txs_per_second = max_txs;
            }
        }
        
        config.validate()?;
        Ok(config)
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate RPC URL
        Url::parse(&self.rpc_url)?;
        
        // Validate WebSocket URL
        if !self.ws_url.starts_with("ws://") && !self.ws_url.starts_with("wss://") {
            return Err(ConfigError::ValidationError(
                "WebSocket URL must start with 'ws://' or 'wss://'".to_string(),
            ));
        }
        
        if self.max_retries > 100 {
            return Err(ConfigError::ValidationError(
                "max_retries cannot exceed 100".to_string(),
            ));
        }
        
        if self.retry_delay_ms > 60_000 {
            return Err(ConfigError::ValidationError(
                "retry_delay_ms cannot exceed 60000 (1 minute)".to_string(),
            ));
        }
        
        if self.max_concurrent_tasks == 0 || self.max_concurrent_tasks > 10_000 {
            return Err(ConfigError::ValidationError(
                "max_concurrent_tasks must be between 1 and 10000".to_string(),
            ));
        }
        
        Ok(())
    }
    
    /// Create a new config with the specified RPC and WebSocket URLs
    pub fn new(rpc_url: impl Into<String>, ws_url: impl Into<String>) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            ws_url: ws_url.into(),
            ..Default::default()
        }
    }
    
    /// Set the debug flag
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }
    
    /// Set the maximum number of concurrent tasks
    pub fn with_max_concurrent_tasks(mut self, max: usize) -> Self {
        self.max_concurrent_tasks = max.clamp(1, 10_000);
        self
    }
    
    /// Set the maximum number of transactions to process per second
    pub fn with_max_txs_per_second(mut self, max: u32) -> Self {
        self.max_txs_per_second = max;
        self
    }
}
