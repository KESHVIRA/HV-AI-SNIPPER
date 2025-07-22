// examples/monitor.rs
use solana_mempool_sniper::{MempoolMonitor, MempoolEvent};
use std::error::Error;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Setup logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Create configuration
    let config = solana_mempool_sniper::Config {
        rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
        ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
        ..Default::default()
    };

    println!(" Starting Solana Mempool Monitor...");
    
    // Create and configure the monitor
    let mut monitor = MempoolMonitor::new(config);
    
    // Connect to the Solana node
    monitor.connect().await?;
    println!("✅ Connected to Solana node");
    
    // Subscribe to transaction logs
    let logs_rx = monitor.subscribe_logs().await?;
    println!("✅ Subscribed to transaction logs");
    
    // Subscribe to events
    let mut event_rx = monitor.subscribe_events().expect("Failed to subscribe to events");
    println!("✅ Subscribed to mempool events");
    
    // Start processing transactions in the background
    let _monitor_handle = tokio::spawn(async move {
        monitor.process_transactions(logs_rx).await;
    });

    println!(" Listening for transactions (Ctrl+C to exit)...\n");
    
    // Process events
    while let Ok(event) = event_rx.recv().await {
        match event {
            MempoolEvent::TokenCreated(event) => {
                println!("\n New Token Created!");
                println!("   Mint: {}", event.mint);
                println!("   Creator: {}", event.creator);
                println!("   Decimals: {}", event.decimals);
                println!("   Signature: {}", event.signature);
            },
            MempoolEvent::DexPoolCreated(event) => {
                println!("\n New DEX Pool Created!");
                println!("   Pool: {}", event.pool);
                println!("   Token A: {}", event.token_a);
                println!("   Token B: {}", event.token_b);
                println!("   DEX: {}", event.dex_program);
                println!("   Signature: {}", event.signature);
            },
            MempoolEvent::MonitoredProgramInteraction { program_id, signature } => {
                println!("\n Monitored Program Interaction");
                println!("   Program: {}", program_id);
                println!("   Signature: {}", signature);
            },
            MempoolEvent::ProcessingError { signature, error } => {
                eprintln!("\n Error Processing Transaction");
                eprintln!("   Signature: {}", signature);
                eprintln!("   Error: {}", error);
            }
        }
    }

    Ok(())
}