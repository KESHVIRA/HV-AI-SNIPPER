use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum SniperError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Subscription error: {0}")]
    SubscriptionError(String),
    
    #[error("RPC error: {0}")]
    RpcError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("URL parse error: {0}")]
    UrlError(#[from] url::ParseError),
}

// Implement From for Solana client errors
impl From<solana_client::client_error::ClientError> for SniperError {
    fn from(err: solana_client::client_error::ClientError) -> Self {
        SniperError::RpcError(err.to_string())
    }
}

impl From<anyhow::Error> for SniperError {
    fn from(err: anyhow::Error) -> Self {
        SniperError::RpcError(err.to_string())
    }
}

// Implement From for tokio mpsc errors
impl<T> From<mpsc::error::SendError<T>> for SniperError {
    fn from(err: mpsc::error::SendError<T>) -> Self {
        SniperError::SubscriptionError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SniperError>;
pub type MempoolResult<T> = std::result::Result<T, SniperError>;
