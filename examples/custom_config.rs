//! Example of using the solana_mempool_sniper library with a custom configuration

use env_logger::Builder;
use log::LevelFilter;
use solana_mempool_sniper::{
    Config,
    MempoolMonitor,
    analyzer::{AnalyzerConfig, TransactionAnalyzer},
};
use solana_sdk::pubkey;
use std::collections::HashSet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    Builder::new()
        .filter_level(LevelFilter::Info)
        .filter_module("solana_mempool_sniper", LevelFilter::Debug)
        .parse_default_env()
        .init();

    // Create a custom configuration
    let config = Config {
        rpc_url: "RPC_URL=https://mainnet.helius-rpc.com/?api-key=8c371c1f-7c94-4024-9722-076e10e56826".to_string(),
        ws_url: "wss://mainnet.helius-rpc.com/?api-key=8c371c1f-7c94-4024-9722-076e10e56826".to_string(),
        max_retries: 10,  // Increase retries for better reliability
        retry_delay_ms: 2000,  // Longer delay between retries
    };

    // Create analyzer configuration
    let mut analyzer_config = AnalyzerConfig::default();
    
    // Monitor popular Solana DEX programs
    let dex_programs = vec![
        // Raydium AMM Program
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
        // Orca Whirlpools
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
        // Serum DEX v3
        "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
    ];

    for program_id in dex_programs {
        if let Ok(pubkey) = program_id.parse() {
            analyzer_config.monitored_programs.insert(pubkey);
        }
    }

    // Set minimum SOL transfer amount to monitor (1 SOL)
    analyzer_config.min_sol_transfer = Some(1_000_000_000);

    // Create analyzer with configuration
    let analyzer = TransactionAnalyzer::new(analyzer_config);

    // Log the configuration
    log::info!("Starting Solana Mempool Sniper with configuration:");
    log::info!("RPC URL: {}", config.rpc_url);
    log::info!("WebSocket URL: {}", config.ws_url);
    log::info!("Monitoring {} programs", analyzer.config.monitored_programs.len());
    log::info!("Minimum SOL transfer: {} lamports", analyzer.config.min_sol_transfer.unwrap_or(0));

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
