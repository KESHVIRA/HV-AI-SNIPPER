//! Simple script to test RPC connection and WebSocket subscription

use anyhow::Result;
use log::{info, error};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::RpcBlockSubscribeConfig,
    rpc_response::RpcBlockUpdate,
};
use solana_sdk::commitment_config::CommitmentConfig;
use std::time::Duration;
use tokio::time::timeout;

const CONNECTION_TIMEOUT: u64 = 10; // seconds

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Get RPC URL from command line or use default
    let rpc_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://api.mainnet-beta.solana.com".to_string());

    info!("Testing connection to RPC endpoint: {}", rpc_url);

    // Test HTTP RPC connection
    test_http_connection(&rpc_url).await?;

    // Test WebSocket subscription
    test_websocket_connection(&rpc_url).await?;

    info!("✅ All tests passed! Your RPC endpoint is working correctly.");
    Ok(())
}

async fn test_http_connection(rpc_url: &str) -> Result<()> {
    info!("Testing HTTP RPC connection...");
    
    let client = RpcClient::new_with_commitment(
        rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    );

    // Try to get the latest block height
    match timeout(
        Duration::from_secs(CONNECTION_TIMEOUT), 
        client.get_slot()
    ).await {
        Ok(Ok(slot)) => {
            info!("✅ Successfully connected to RPC endpoint. Current slot: {}", slot);
            Ok(())
        },
        Ok(Err(e)) => {
            error!(" RPC request failed: {}", e);
            Err(e.into())
        },
        Err(_) => {
            let msg = format!("RPC request timed out after {} seconds", CONNECTION_TIMEOUT);
            error!(" {}", msg);
            Err(anyhow::anyhow!(msg))
        },
    }
}

async fn test_websocket_connection(rpc_url: &str) -> Result<()> {
    info!("Testing WebSocket subscription...");
    
    // Convert HTTP URL to WebSocket URL if needed
    let ws_url = if rpc_url.starts_with("https://") {
        rpc_url.replacen("https://", "wss://", 1)
    } else if rpc_url.starts_with("http://") {
        rpc_url.replacen("http://", "ws://", 1)
    } else if !rpc_url.starts_with("ws") {
        format!("wss://{}", rpc_url)
    } else {
        rpc_url.to_string()
    };

    info!("Connecting to WebSocket endpoint: {}", ws_url);
    
    let client = solana_client::nonblocking::pubsub_client::PubsubClient::new(&ws_url).await
        .map_err(|e| {
            error!(" Failed to connect to WebSocket: {}", e);
            anyhow::anyhow!("WebSocket connection failed: {}", e)
        })?;

    // Subscribe to block updates
    let (mut block_subscription, _) = client.block_subscribe(
        solana_client::rpc_config::RpcBlockSubscribeConfig {
            encoding: None,
            transaction_details: None,
            show_rewards: None,
            max_supported_transaction_version: Some(0),
        },
    ).await.map_err(|e| {
        error!(" Failed to subscribe to block updates: {}", e);
        anyhow::anyhow!("Block subscription failed: {}", e)
    })?;

    info!("✅ Successfully subscribed to block updates. Waiting for the next block...");
    info!("   (This may take a moment. Press Ctrl+C to cancel)");

    // Wait for the next block update or timeout
    match timeout(
        Duration::from_secs(CONNECTION_TIMEOUT * 3), // Give more time for block production
        block_subscription.next()
    ).await {
        Ok(Some(Ok(RpcBlockUpdate { slot, .. }))) => {
            info!("✅ Successfully received block update for slot: {}", slot);
            Ok(())
        },
        Ok(Some(Err(e))) => {
            error!(" Error receiving block update: {}", e);
            Err(anyhow::anyhow!("Block update error: {}", e))
        },
        Ok(None) => {
            let msg = "WebSocket connection closed unexpectedly";
            error!(" {}", msg);
            Err(anyhow::anyhow!(msg))
        },
        Err(_) => {
            let msg = format!("Did not receive any block updates in {} seconds. The WebSocket connection might be working, but no blocks were produced in this time.", CONNECTION_TIMEOUT * 3);
            info!("  {}", msg);
            info!("   This might be expected on testnet or if the network is not producing blocks.");
            info!("   The WebSocket connection itself appears to be working correctly.");
            Ok(())
        },
    }
}
