//! Basic example of using the solana_mempool_sniper library

use env_logger::Builder;
use log::LevelFilter;
use solana_mempool_sniper::{
    Config,
    MempoolMonitor,
};
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    Builder::new()
        .filter_level(LevelFilter::Info)
        .filter_module("solana_mempool_sniper", LevelFilter::Debug)
        .parse_default_env()
        .init();

    // Load configuration from environment variables
    let config = Config::default();
    
    // Log the endpoints being used
    log::info!("Using RPC endpoint: {}", config.rpc_url);
    log::info!("Using WebSocket endpoint: {}", config.ws_url);
    
    // Create and initialize the mempool monitor
    let mut monitor = MempoolMonitor::new(config);
    
    // Connect to the Solana node
    log::info!("Connecting to Solana node...");
    monitor.connect().await?;
    log::info!("Successfully connected to Solana node");
    
    // Subscribe to transaction logs
    log::info!("Subscribing to transaction logs...");
    let logs_receiver = monitor.subscribe_logs().await?;
    log::info!("Successfully subscribed to transaction logs");
    
    // Start processing transactions
    log::info!("Starting to monitor the mempool. Press Ctrl+C to exit...");
    monitor.process_transactions(logs_receiver).await;
    
    Ok(())
}
