use solana_mempool_sniper::mempool::RECENT_SNIPES;
use anyhow::Context;
use clap::Parser;
use log::{info};
use solana_mempool_sniper::{
    analyzer::{program_ids, AnalyzerConfig},
    config::Config,
    mempool::{MempoolMonitor,
    SniperResult},
};
use std::env;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Signer;


mod sniper;
mod wallet;

#[derive(Parser, Debug)]
#[clap(
    version,
    about = "Solana Mempool Sniper - Monitor and analyze Solana mempool transactions"
)]
struct Args {
    #[clap(short, long, default_value = "config.toml")]
    config: String,

    #[clap(long)]
    monitor_tokens: bool,

    #[clap(long)]
    monitor_dex: bool,
}

#[tokio::main]
async fn main() -> SniperResult<()> {
    dotenv::dotenv().ok();
    env_logger::init(); // or keep the cfg block if CLI-only logic is required

    tokio::spawn(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            RECENT_SNIPES.lock().unwrap().clear();
            info!(" Cleared recent snipes cache");
        }
    });
    // 1. Parse CLI arguments
    let args = Args::parse();

    info!(" Starting Solana Mempool Sniper...");

    // 2. Load configuration
   let config = crate::load_config().context("Failed to load configuration")?;
    info!(" Configuration loaded");
    let client = RpcClient::new(config.rpc_url.clone());




    // 3. Load keypair from env or file
    let keypair = wallet::load_keypair()?;

    // 4. Check funding (e.g., min 0.05 SOL)
    wallet::check_funding(&keypair, &config.rpc_url, 0.05).await?;

        let balance = client.get_balance(&keypair.pubkey()).await?;
        let sol = balance as f64 / 1_000_000_000.0;
        info!(" Wallet balance: {:.4} SOL ({}) lamports", sol, balance);

    // 5. Set up analyzer config
    let mut analyzer_config = AnalyzerConfig::default();
    analyzer_config.monitor_token_creations = args.monitor_tokens;
    analyzer_config.monitor_dex_listings = args.monitor_dex;
    analyzer_config
        .monitored_programs
        .insert(program_ids::RAYDIUM_AMM);
    analyzer_config
        .monitored_programs
        .insert(program_ids::ORCA_WHIRLPOOLS);
    analyzer_config
        .monitored_programs
        .insert(program_ids::SERUM_DEX);

    // 6. Create monitor and connect
    let mut monitor = MempoolMonitor::with_analyzer(config, analyzer_config);
    monitor
        .connect()
        .await
        .context("Failed to connect to Solana node")?;
    info!(" Connected to Solana node");

    // 7. Subscribe to logs
    let logs_receiver = monitor
        .subscribe_logs()
        .await
        .context("Failed to subscribe to logs")?;
    info!("📡 Subscribed to transaction logs");

    // 8. Begin transaction processing
    info!(" Monitoring mempool... Press Ctrl+C to exit");
    info!(" Monitoring token creations: {}", args.monitor_tokens);
    info!(" Monitoring DEX pool creations: {}", args.monitor_dex);

    monitor.process_transactions(logs_receiver).await;

    Ok(())
}

use anyhow::Result;

/// Load configuration from environment variables
fn load_config() -> Result<Config, anyhow::Error> {
    let mut config = Config::default();

    // Override with environment variables if provided
    if let Ok(rpc_url) = env::var("RPC_URL") {
        config.rpc_url = rpc_url;
    }

    if let Ok(ws_url) = env::var("WS_URL") {
        config.ws_url = ws_url;
    }

    // Set log level from environment if provided
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    // Set other configs from environment
    if let Ok(max_retries) = env::var("MAX_RETRIES") {
        if let Ok(max_retries) = max_retries.parse::<u32>() {
            config.max_retries = max_retries;
        }
    }

    if let Ok(retry_delay_ms) = env::var("RETRY_DELAY_MS") {
        if let Ok(retry_delay_ms) = retry_delay_ms.parse::<u64>() {
            config.retry_delay_ms = retry_delay_ms;
        }
    }

    // Validate configuration
    config
        .validate()
        .context("Configuration validation failed")?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_load_config() {
        // Test default config
        let config = Config::default();
        assert!(!config.rpc_url.is_empty());
        assert!(!config.ws_url.is_empty());

        // Test environment variable override
        env::set_var("RPC_URL", "https://test-rpc-url.com");
        env::set_var("WS_URL", "wss://test-ws-url.com");

        let config = Config::default();
        assert_eq!(config.rpc_url, "https://test-rpc-url.com");
        assert_eq!(config.ws_url, "wss://test-ws-url.com");

        // Cleanup
        env::remove_var("RPC_URL");
        env::remove_var("WS_URL");
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config {
            rpc_url: "not-a-url".to_string(),
            ws_url: "wss://valid-ws-url.com".to_string(),
            ..Default::default()
        };

        // Should fail with invalid RPC URL
        assert!(config.validate().is_err());

        // Fix RPC URL but break WS URL
        config.rpc_url = "https://valid-rpc-url.com".to_string();
        config.ws_url = "invalid-protocol".to_string();

        // Should fail with invalid WebSocket URL
        assert!(config.validate().is_err());

        // Fix WS URL
        config.ws_url = "wss://valid-ws-url.com".to_string();

        // Should pass with valid URLs
        assert!(config.validate().is_ok());
    }
}
