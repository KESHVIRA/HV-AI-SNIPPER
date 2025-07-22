//! Mempool monitoring functionality for Solana transactions.
use std::fmt;
use std::collections::VecDeque;
use log::{debug, error, info, warn};
use solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter, RpcTransactionConfig},
    rpc_response::RpcLogsResponse,
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
// use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use crate::error::SniperError;
use crate::error::MempoolResult;

use solana_transaction_status::UiTransactionEncoding;
use tokio::time::{sleep, Duration};


pub type SniperResult<T> = Result<T, SniperError>;
// pub type MempoolResult<T> = std::result::Result<T, SniperError>;


use crate::{
    analyzer::{
        AnalysisResult, AnalyzerConfig, DexPoolCreationEvent, TokenCreationEvent,
        TransactionAnalyzer,
    },
    config::Config,
};
use once_cell::sync::Lazy;
use std::collections::HashSet;
use tokio::sync::Semaphore;

// Allow 1 snipe at a time
pub static SNIPE_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(1));

// Avoid sniping same pool repeatedly
pub static RECENT_SNIPES: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

// Minimum liquidity thresholds
const MIN_SOL_LIQUIDITY: u64 = 1_000_000_000; // 1 SOL
// const MAX_SLIPPAGE_BPS: u16 = 500; // 5%

// Dynamic buy amount config
// Max buy amount in lamports (default: 0.01 SOL)
const DEFAULT_MAX_BUY_LAMPORTS: u64 = 10_000_000; // 0.01 SOL
// Default buy percent of pool liquidity (e.g., 0.5%)
const DEFAULT_BUY_LIQUIDITY_PCT: f64 = 0.005;

// Optional whitelist/blacklist
static WHITELIST: &[&str] = &[]; // e.g. &["So11111111111111111111111111111111111111112"]
static BLACKLIST: &[&str] = &[]; // e.g. scam tokens

/// Event emitted when a transaction is processed
#[derive(Debug, Clone)]
pub enum MempoolEvent {
    /// A new token was created
    TokenCreated(TokenCreationEvent),
    /// A new DEX pool was created
    DexPoolCreated(DexPoolCreationEvent),
    /// A monitored program was called
    MonitoredProgramInteraction {
        /// The program ID that was called
        program_id: Pubkey,
        /// The transaction signature
        signature: Signature,
    },
    /// An error occurred while processing a transaction
    ProcessingError {
        /// The transaction signature
        signature: String,
        /// The error message
        error: String,
    },
}




/// Configuration for the mempool monitor
#[derive( Clone)]
pub struct MempoolConfig {
    /// Maximum number of transactions to keep in the queue
    pub max_queue_size: usize,
    /// Whether to enable event broadcasting
    pub enable_event_broadcast: bool,
    /// Maximum number of event subscribers
    pub max_event_subscribers: usize,
    pub rpc_client: Arc<RpcClient>,
    pub pubsub_client: Option<Arc<PubsubClient>>,
}
impl fmt::Debug for MempoolConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MempoolConfig")
            .field("max_queue_size", &self.max_queue_size)
            .field("enable_event_broadcast", &self.enable_event_broadcast)
            .field("max_event_subscribers", &self.max_event_subscribers)
            .field("rpc_client", &"<RpcClient>") // or skip this field completely
            .finish()
    }
}

impl Default for MempoolConfig {
    fn default() -> Self {
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            std::env::var("RPC_URL").unwrap_or_else(|_| "https://mainnet.helius-rpc.com/?api-key=8c371c1f-7c94-4024-9722-076e10e56826".to_string()),
            CommitmentConfig::confirmed(),
        ));

        Self {
            max_queue_size: 1000,
            enable_event_broadcast: true,
            max_event_subscribers: 100,
            rpc_client,
            pubsub_client: None,
        }
    }
}

// Mempool monitor that processes transactions from the Solana mempool
pub struct MempoolMonitor {
    // Application configuration
    config: Arc<Config>,
    // Transaction analyzer
    analyzer: Arc<TransactionAnalyzer>,
    // PubSub client for subscribing to transaction logs
    pubsub_client: Option<Arc<PubsubClient>>,
    // RPC client for fetching transaction details
    rpc_client: Arc<RpcClient>,
    // Event sender for broadcasting events to subscribers
    event_sender: Option<broadcast::Sender<MempoolEvent>>,
    // Mempool configuration
    mempool_config: MempoolConfig,
    transaction_queue: Arc<Mutex<VecDeque<Signature>>>,
    /// Rate limiter for txs/sec
    tx_rate_limiter: Option<tokio::sync::Semaphore>,
}

impl MempoolMonitor {
    /// Create a new Mempool Monitor with the given configuration and default analyzer
    pub fn new(config: Config) -> Self {
        // Create a default analyzer config
        let analyzer_config = AnalyzerConfig {
            min_sol_transfer: Some(1_000_000_000), // 1 SOL
            ..Default::default()
        };

        Self::with_analyzer_and_config(config, analyzer_config, MempoolConfig::default())
    }

    /// Create a new Mempool Monitor with a custom analyzer configuration and mempool config
    pub fn with_analyzer_and_config(
        config: Config,
        analyzer_config: AnalyzerConfig,
        mempool_config: MempoolConfig,
    ) -> Self {
        let (event_sender, _) = if mempool_config.enable_event_broadcast {
            let (tx, _) = broadcast::channel(mempool_config.max_event_subscribers);
            (Some(tx), None::<()>) // Explicitly specify `Option<()>` for the second element
        } else {
            (None, None::<()>) // Same here, specify `Option<()>`
        };

        let rpc_client = Arc::new(RpcClient::new_with_commitment(config.rpc_url.clone(), CommitmentConfig::confirmed()));

        Self {
            config: Arc::new(config),
            analyzer: Arc::new(TransactionAnalyzer::new(analyzer_config)),
            pubsub_client: None,
            rpc_client,
            event_sender,
            mempool_config: mempool_config.clone(),
            transaction_queue: Arc::new(Mutex::new(VecDeque::with_capacity(
                mempool_config.max_queue_size,
            ))),
            tx_rate_limiter: None,
        }
    }

    /// Create a new Mempool Monitor with a custom analyzer configuration
    pub fn with_analyzer(config: Config, analyzer_config: AnalyzerConfig) -> Self {
        let max_txs_per_sec = std::env::var("MAX_TXS_PER_SECOND")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(0);
        let tx_rate_limiter = if max_txs_per_sec > 0 {
            Some(tokio::sync::Semaphore::new(max_txs_per_sec))
        } else {
            None
        };
        let mut monitor = Self::with_analyzer_and_config(config, analyzer_config, MempoolConfig::default());
        monitor.tx_rate_limiter = tx_rate_limiter;
        monitor
    }

    pub async fn connect(&mut self) -> MempoolResult<()> {
        info!(" Connecting to Solana node at {}", self.config.ws_url);

        let mut retry_attempts = 0;

        loop {
            match PubsubClient::new(&self.config.ws_url).await {
                Ok(client) => {
                    self.pubsub_client = Some(Arc::new(client));
                    info!(" WebSocket connected");
                    return Ok(());
                }
                Err(e) => {
                    retry_attempts += 1;
                    error!(
                        " WebSocket connection failed: {} (attempt #{})",
                        e, retry_attempts
                    );

                    let delay = std::cmp::min(5 * retry_attempts, 30); // cap delay at 30s
                    warn!(" Retrying WebSocket in {}s...", delay);
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
            }
        }
    }

pub async fn subscribe_logs(&self) -> MempoolResult<mpsc::Receiver<String>> {
    let pubsub_client = self
        .pubsub_client
        .as_ref()
       .ok_or(SniperError::ConnectionError("Not connected to Solana node. Call connect() first.".to_string()))?
        .clone(); // clone the Arc

    let (tx, rx) = mpsc::channel(100);

    tokio::spawn(async move {
        use tokio_stream::StreamExt;

        let filter = RpcTransactionLogsFilter::All;

        let (mut logs_stream, _unsub) = pubsub_client
            .logs_subscribe(
                filter,
                RpcTransactionLogsConfig {
                    commitment: Some(CommitmentConfig::confirmed()),
                },
            )
            .await
            .expect("Failed to subscribe to logs");

        while let Some(result) = logs_stream.next().await {
            match result.value {
                RpcLogsResponse { signature, .. } => {
                    if let Err(e) = tx.send(signature.to_string()).await {
                        error!("Failed to send log: {}", e);
                    }
                }
            }
        }
    });

    info!("Subscribed to transaction logs");
    Ok(rx)
}

    /// Process transactions from the mempool
        pub async fn process_transactions(&self, mut rx: mpsc::Receiver<String>) {
        info!("Starting transaction processing loop");

    // Clone shared state once outside loop
    let rpc_client = Arc::clone(&self.rpc_client);
    let analyzer = Arc::clone(&self.analyzer);
    let event_sender = self.event_sender.clone(); // This is an Option<Sender<_>>
    

    let mut last_tick = tokio::time::Instant::now();
    let mut txs_this_sec = 0;
    let max_txs_per_sec = std::env::var("MAX_TXS_PER_SECOND")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    while let Some(signature) = rx.recv().await {
        // Rate limiting: throttle txs/sec
        if max_txs_per_sec > 0 {
            let now = tokio::time::Instant::now();
            if now.duration_since(last_tick).as_secs() >= 1 {
                last_tick = now;
                txs_this_sec = 0;
            }
            if txs_this_sec >= max_txs_per_sec {
                let sleep_time = 1_000 - now.duration_since(last_tick).as_millis() as u64;
                if sleep_time > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(sleep_time)).await;
                }
                last_tick = tokio::time::Instant::now();
                txs_this_sec = 0;
            }
            txs_this_sec += 1;
        }
        debug!("Processing transaction: {}", signature);
        // Clone inside loop for use in task
        let rpc_client = Arc::clone(&rpc_client);
        let analyzer = Arc::clone(&analyzer);
        let event_sender = event_sender.clone();
        let signature = signature.clone();
        // Fault tolerance: wrap in retry_with_backoff
        tokio::spawn(async move {
            let tx_result = Self::retry_with_backoff(|| {
                let rpc_client = &rpc_client;
                let analyzer = &analyzer;
                let signature = &signature;
                async move {
                    Self::process_single_transaction(rpc_client, analyzer, signature)
                        .await
                        .map_err(anyhow::Error::from)
                }
            }).await;
            match tx_result {
                Ok(events) => {
                    if let Some(sender) = &event_sender {
                        for event in events {
                            if let Err(e) = sender.send(event) {
                                error!("Failed to send event: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("🔁 [PROCESS LOOP] Failed transaction: {} — {}. It will not be retried.", signature, e);
                    if let Some(sender) = &event_sender {
                        let _ = sender.send(MempoolEvent::ProcessingError {
                            signature: signature.clone(),
                            error: format!("{:?}", e),
                        });
                    }
                    // Optional: send alert
                    crate::sniper::send_alert(&format!("Tx failed: {} — {}", signature, e)).await.ok();
                }
            }
        });
    }
}


    /// Process a single transaction
    async fn process_single_transaction(
        rpc_client: &RpcClient,
        analyzer: &TransactionAnalyzer,
        signature_str: &str,
    ) -> MempoolResult<Vec<MempoolEvent>> {
     let mut events = Vec::new();
        // Parse the signature
        let signature = signature_str
        .parse::<Signature>()
       .map_err(|e| SniperError::RpcError(e.to_string()))?;

        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Json),
           commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };

        let tx = Self::retry_with_backoff(|| {
            Box::pin(async {
                tokio::time::sleep(Duration::from_millis(3000)).await;
                rpc_client
                    .get_transaction_with_config(&signature, config.clone())
                    .await
                    .map_err(anyhow::Error::from)
            })
        })
        .await
        .map_err(|e| anyhow::anyhow!(" Failed to fetch tx after retries: {:?}", e))?;



    
        let transaction = &tx.transaction;
        let Some(versioned_tx) = transaction.transaction.decode() else {
            warn!("No valid transaction data for signature {} (possibly unconfirmed or null)", signature);
            return Err(SniperError::RpcError("No valid transaction data found".to_string()));
        };

    // Convert to a VersionedTransaction for analysis

    if let Some(versioned_tx) = transaction.transaction.decode() {
        let results = analyzer
            .analyze_transaction(&versioned_tx, &signature)
            .await?;

                // Process each analysis result
                for result in results {
                    let event = match result {
                        AnalysisResult::TokenCreation(event) => {
                            info!(
                                "New token created: mint={}, creator={}, decimals={}, signature={}",
                                event.mint, event.creator, event.decimals, signature
                            );
                            MempoolEvent::TokenCreated(event)
                        }
                        AnalysisResult::DexPoolCreation(event) => {
                            info!(
                            "🎯 Detected DEX pool: {} with tokens {} & {} — signature: {}",
                            event.pool, event.token_a, event.token_b, signature
                        );
                            info!(
                                "New DEX pool created: pool={}, token_a={}, token_b={}, dex={}, signature={}",
                                event.pool, event.token_a, event.token_b, event.dex_program, signature
                            );
                            if event.dex_program == crate::analyzer::program_ids::RAYDIUM_AMM {
                                // === NEW FILTERS: Blacklist/Whitelist before price/volume enforcement ===
                                // Blacklist filter
                                if BLACKLIST.contains(&event.token_a.to_string().as_str())
                                    || BLACKLIST.contains(&event.token_b.to_string().as_str())
                                {
                                    warn!(" Skipping blacklisted token");
                                    return Ok(vec![]);
                                }
                                // Whitelist filter
                                if !WHITELIST.is_empty()
                                    && !WHITELIST.contains(&event.token_a.to_string().as_str())
                                    && !WHITELIST.contains(&event.token_b.to_string().as_str())
                                {
                                    warn!(" Token not in whitelist, skipping...");
                                    return Ok(vec![]);
                                }
                                // Advanced filter: price/volume enforcement
                                let min_price = std::env::var("MIN_TOKEN_PRICE").ok().and_then(|v| v.parse::<f64>().ok());
                                let min_vol = std::env::var("MIN_POOL_VOLUME").ok().and_then(|v| v.parse::<u64>().ok());

                                // Fetch pool info (price/volume) using a helper (stubbed for now)
                                let (pool_price, pool_volume) = match crate::sniper::fetch_pool_price_and_volume(rpc_client, event.pool).await {
                                    Ok((price, volume)) => (price, volume),
                                    Err(e) => {
                                        warn!("Failed to fetch pool price/volume: {:?}", e);
                                        return Ok(vec![]);
                                    }
                                };
                                if let Some(min_price) = min_price {
                                    if pool_price < min_price {
                                        warn!("Pool price {} below min {}, skipping", pool_price, min_price);
                                        return Ok(vec![]);
                                    }
                                }
                                if let Some(min_vol) = min_vol {
                                    if pool_volume < min_vol {
                                        warn!("Pool volume {} below min {}, skipping", pool_volume, min_vol);
                                        return Ok(vec![]);
                                    }
                                }
                                // Optional: log event to file
                                crate::sniper::log_event_to_file(&format!("Sniping pool: {}", event.pool)).ok();

                                let pool_id = event.pool.to_string();
                                // Skip if already sniped
                                {
                                    let mut seen = RECENT_SNIPES.lock().unwrap();
                                    if seen.contains(&pool_id) {
                                        warn!(" Already sniped pool {}, skipping...", pool_id);
                                        return Ok(vec![]);
                                    } else {
                                        seen.insert(pool_id.clone());
                                    }
                                }
                                // Ensure only one snipe at a time
                                let permit = match SNIPE_SEMAPHORE.try_acquire() {
                                    Ok(p) => p,
                                    Err(_) => {
                                        warn!(" Another snipe already in progress. Skipping pool {}", pool_id);
                                        return Ok(vec![]);
                                    }
                                };
                                // Liquidity check
                                let coin_liq = rpc_client
                                    .get_token_account_balance(&event.token_a)
                                    .await
                                    .map_err(anyhow::Error::from)?
                                    .amount
                                    .parse::<u64>()
                                    .unwrap_or(0);
                                let pc_liq = rpc_client
                                    .get_token_account_balance(&event.token_b)
                                    .await
                                    .map_err(anyhow::Error::from)?
                                    .amount
                                    .parse::<u64>()
                                    .unwrap_or(0);
                                if coin_liq < MIN_SOL_LIQUIDITY && pc_liq < MIN_SOL_LIQUIDITY {
                                    warn!("💧 Pool too small — coin: {}, pc: {}", coin_liq, pc_liq);
                                    return Ok(vec![]);
                                }
                                // Slippage and simulation config (from env or default)
                                let max_slippage_bps = std::env::var("MAX_SLIPPAGE_BPS")
                                    .ok()
                                    .and_then(|v| v.trim().parse::<u64>().ok())
                                    .unwrap_or(100); // 1%
                                let simulate = std::env::var("SIMULATE_SNIPES")
                                    .ok()
                                    .map(|v| v == "1" || v.to_lowercase() == "true")
                                    .unwrap_or(false);
                                // === Dynamic buy amount calculation ===
                                let max_buy_lamports = std::env::var("MAX_BUY_LAMPORTS")
                                    .ok()
                                    .and_then(|v| v.trim().parse::<u64>().ok())
                                    .unwrap_or(DEFAULT_MAX_BUY_LAMPORTS);
                                let buy_liquidity_pct = std::env::var("BUY_LIQUIDITY_PCT")
                                    .ok()
                                    .and_then(|v| v.trim().parse::<f64>().ok())
                                    .unwrap_or(DEFAULT_BUY_LIQUIDITY_PCT);
                                let pool_liq = coin_liq.max(pc_liq);
                                let mut buy_amount = ((pool_liq as f64) * buy_liquidity_pct) as u64;
                                if buy_amount > max_buy_lamports {
                                    buy_amount = max_buy_lamports;
                                }
                                if buy_amount == 0 {
                                    warn!("Calculated buy amount is zero, skipping snipe.");
                                    drop(permit);
                                    return Ok(vec![]);
                                }
                                info!("Dynamic buy amount: {} lamports (pool_liq: {}, pct: {}, max: {})", buy_amount, pool_liq, buy_liquidity_pct, max_buy_lamports);
                                // Passed all filters — execute snipe
                                let payer = crate::wallet::load_keypair()?;
                                let payer = std::sync::Arc::new(crate::wallet::load_keypair()?);
                                let pool = match crate::sniper::fetch_raydium_pool_accounts(rpc_client, event.pool).await {
                                    Ok(p) => p,
                                    Err(e) => {
                                        error!("❌ Failed to fetch Raydium pool accounts: {:?}", e);
                                        drop(permit);
                                        return Ok(vec![]);
                                    }
                                };
                                // Pre-trade simulation (optional)
                                if simulate {
                                    info!("[SIMULATION] Would snipe pool {} with {} lamports, slippage {}bps", pool_id, buy_amount, max_slippage_bps);
                                    drop(permit);
                                    return Ok(vec![]);
                                }
                                // Build and send transaction using send_and_confirm_transaction
                                let result = crate::sniper::snipe_on_raydium(
                                    rpc_client,
                                    &*payer,
                                    pool.clone(),
                                    event.dex_program,
                                    buy_amount,
                                )
                                .await;
                                drop(permit); // Release lock
                                match result {
                                    Ok(sig) => {
                                        info!("🎯 Sniped! Signature: {}", sig);
                                        // === Auto-sell logic: sell after a delay ===
                                        let rpc_client = rpc_client.clone();
                                        let payer = std::sync::Arc::clone(&payer);
                                        let pool = pool.clone();
                                        let dex_program = event.dex_program;
                                        let sell_amount = buy_amount; // sell the same amount as bought
                                        tokio::spawn(async move {
                                            let delay_secs = std::env::var("AUTO_SELL_DELAY_SECS")
                                                .ok()
                                                .and_then(|v| v.trim().parse::<u64>().ok())
                                                .unwrap_or(30);
                                            info!("⏳ Waiting {}s before auto-sell...", delay_secs);
                                            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                                            match crate::sniper::sell_on_raydium(
                                                &rpc_client,
                                                &*payer,
                                                pool,
                                                dex_program,
                                                sell_amount,
                                            ).await {
                                                Ok(sell_sig) => info!("💸 Auto-sold! Signature: {}", sell_sig),
                                                Err(e) => error!("❌ Auto-sell failed: {:?}", e),
                                            }
                                        });
                                    },
                                    Err(e) => error!("❌ Snipe failed: {:?}", e),
                                }
                            }

                            MempoolEvent::DexPoolCreated(event)
                        }
                        AnalysisResult::MonitoredProgramInteraction {
                            program_id,
                            signature: monitored_signature,
                        } => {
                            info!(
                                "Transaction interacted with monitored program: program={}, signature={}",
                                program_id, monitored_signature
                            );
                            MempoolEvent::MonitoredProgramInteraction {
                                program_id,
                                signature: monitored_signature,
                            }
                        }
                    };
                    events.push(event);
                }
            }
            else {
            error!(" Failed to decode transaction= {}", signature_str);
            return Ok(vec![]);
        }
    Ok(events)
}

    pub async fn retry_with_backoff<F, Fut, T>(mut task: F) -> Result<T, anyhow::Error>
        where
            F: FnMut() -> Fut,
            Fut: std::future::Future<Output = Result<T, anyhow::Error>>,
        {
            // Read from env or use defaults
            let max_retries = std::env::var("MAX_RETRIES")
                .ok()
                .and_then(|v| v.trim().parse::<u32>().ok())
                .unwrap_or(5);
            let mut delay = std::env::var("RETRY_DELAY_MS")
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
                .unwrap_or(1000);
            for attempt in 1..=max_retries {
                let fut = task();
                match fut.await {
                    Ok(res) => return Ok(res),
                    Err(e) => {
                        warn!(
                            "Retry {}/{} failed: {}. Retrying in {}ms...",
                            attempt, max_retries, e, delay
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        delay *= 2;
                    }
                }
            }
            Err(anyhow::anyhow!("All retries failed"))
        }

    /// Subscribe to mempool events
    pub fn subscribe_events(&self) -> Option<broadcast::Receiver<MempoolEvent>> {
        self.event_sender.as_ref().map(|sender| sender.subscribe())
    }
}