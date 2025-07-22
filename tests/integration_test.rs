// tests/integration_tests.rs
use solana_mempool_sniper::{MempoolMonitor, MempoolEvent};
use tokio::time::{timeout, Duration};
use std::sync::Arc;
use test_config::test_config;

#[tokio::test]
async fn test_mempool_monitor() {
    // Setup logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = test_config();
    let mut monitor = MempoolMonitor::new(config);
    
    // Connect to the Solana node
    monitor.connect().await.expect("Failed to connect to Solana node");
    
    // Subscribe to transaction logs
    let logs_rx = monitor.subscribe_logs().await.expect("Failed to subscribe to logs");
    
    // Subscribe to events
    let mut event_rx = monitor.subscribe_events().expect("Failed to subscribe to events");
    
    // Start processing transactions in the background
    let _monitor_handle = tokio::spawn(async move {
        monitor.process_transactions(logs_rx).await;
    });

    // Wait for some events with a timeout
    match timeout(Duration::from_secs(30), async {
        let mut token_created = false;
        let mut dex_pool_created = false;
        
        while let Ok(event) = event_rx.recv().await {
            match event {
                MempoolEvent::TokenCreated(event) => {
                    println!("✅ Detected token creation: {:?}", event);
                    token_created = true;
                },
                MempoolEvent::DexPoolCreated(event) => {
                    println!("✅ Detected DEX pool creation: {:?}", event);
                    dex_pool_created = true;
                },
                MempoolEvent::MonitoredProgramInteraction { program_id, signature } => {
                    println!("ℹ️  Monitored program interaction: {} in {}", program_id, signature);
                },
                MempoolEvent::ProcessingError { signature, error } => {
                    eprintln!("❌ Error processing {}: {}", signature, error);
                }
            }
            
            // Exit if we've seen both types of events
            if token_created && dex_pool_created {
                break;
            }
        }
    }).await {
        Ok(_) => println!("✅ Test completed successfully!"),
        Err(_) => eprintln!("❌ Test timed out before detecting all events"),
    }
}