//! Solana Mempool Sniper
//!
//! A high-performance mempool monitoring bot for Solana that can be used for various DeFi applications.
//! This crate provides functionality to monitor the Solana mempool, analyze transactions in real-time,
//! and take actions based on predefined criteria.
//!
//! # Features
//! - Real-time monitoring of Solana mempool via WebSocket
//! - Token creation detection
//! - DEX pool creation detection
//! - Customizable transaction filtering and analysis
//! - Extensible architecture for adding new analysis modules

#![warn(rustdoc::missing_crate_level_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

pub mod analyzer;
pub mod config;
pub mod error;
pub mod mempool;
pub mod sniper;
pub mod wallet;

/// Re-export commonly used types
pub use analyzer::{
    program_ids, AnalysisResult, AnalyzerConfig, DexPoolCreationEvent, TokenCreationEvent,
    TransactionAnalyzer,
};
pub use config::Config;
pub use error::{Result, SniperError};
pub use mempool::MempoolMonitor;
