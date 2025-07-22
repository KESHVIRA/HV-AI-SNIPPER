# Solana Mempool Sniper

A high-performance Rust-based mempool monitoring bot for Solana that listens to transactions in real-time using WebSocket connections. This bot is designed to help you monitor and analyze transactions in the Solana mempool for various DeFi applications, including arbitrage opportunities, MEV (Miner Extractable Value), and other on-chain activities.

## Features

- 🚀 Real-time monitoring of Solana mempool via WebSocket
- 🔍 Configurable RPC and WebSocket endpoints
- 🛠️ Extensible transaction analysis pipeline
- 🔄 Automatic reconnection on connection drops
- 🎯 Filter transactions by program ID, transaction type, and more
- 🪙 Token creation detection - get notified of new token deployments
- 💧 DEX pool creation detection - track new liquidity pools on popular DEXes
- 📊 Built-in metrics and logging
- 🧪 Comprehensive test coverage
- 📦 Easy to integrate into existing Rust projects

## Prerequisites

- Rust 1.70 or later (install via [rustup](https://rustup.rs/))
- Solana CLI tools (optional, for testing)
- A Solana RPC endpoint with WebSocket support (you can use public endpoints or run your own node)

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/solana-mempool-sniper.git
   cd solana-mempool-sniper
   ```

2. Build the project in release mode for optimal performance:
   ```bash
   cargo build --release
   ```

## Quick Start

### Using the Pre-configured Example

1. Copy the example environment file and update it with your RPC endpoints:
   ```bash
   cp .env.example .env
   # Edit .env with your preferred text editor
   ```

2. Run the bot with token and DEX monitoring enabled:
   ```bash
   cargo run --release -- --monitor-tokens --monitor-dex
   ```

   Or run with specific monitoring options:
   ```bash
   # Monitor only token creations
   cargo run --release -- --monitor-tokens
   
   # Monitor only DEX pool creations
   cargo run --release -- --monitor-dex
   ```

### Using the Helper Scripts

For convenience, we provide helper scripts to quickly start the bot with custom RPC endpoints:

#### On Linux/macOS:

```bash
# Make the script executable
chmod +x scripts/run_with_rpc.sh

# Run with your RPC and WebSocket endpoints
./scripts/run_with_rpc.sh https://your-rpc-url.com wss://your-ws-url.com
```

#### On Windows:

```cmd
:: Run with your RPC and WebSocket endpoints
scripts\run_with_rpc.bat https://your-rpc-url.com wss://your-ws-url.com
```

### Testing Your RPC Connection

Before running the full bot, you can test your RPC and WebSocket connection:

```bash
# Test with default RPC URL
cargo run --bin test_rpc_connection

# Or specify a custom RPC URL
cargo run --bin test_rpc_connection -- https://your-rpc-url.com
```

This will verify that both HTTP RPC and WebSocket connections are working correctly.

## Configuration

The bot can be configured using environment variables, command-line arguments, or by modifying the `Config` struct in code.

### Environment Variables

```bash
# Required: Solana RPC and WebSocket endpoints
RPC_URL="https://your-rpc-url.com"
WS_URL="wss://your-ws-url.com"

# Optional: Logging level (default: info)
RUST_LOG=info

# Optional: Maximum number of retries for failed operations (default: 5)
MAX_RETRIES=5

# Optional: Delay between retries in milliseconds (default: 1000)
RETRY_DELAY_MS=1000
```

### Command-Line Arguments

```
USAGE:
    solana-mempool-sniper [OPTIONS] --config <CONFIG>

OPTIONS:
    -c, --config <CONFIG>    Path to the configuration file [default: config.toml]
        --monitor-tokens     Enable token creation monitoring
        --monitor-dex        Enable DEX pool monitoring
    -h, --help               Print help information
    -V, --version            Print version information
```

### Example Configuration

```toml
# config.toml
rpc_url = "https://api.mainnet-beta.solana.com"
ws_url = "wss://api.mainnet-beta.solana.com"
max_retries = 5
retry_delay_ms = 1000
```

### Environment Variables

```bash
# Required: Solana RPC and WebSocket endpoints
RPC_URL="https://your-rpc-url.com"
WS_URL="wss://your-ws-url.com"

# Optional: Logging level (default: info)
RUST_LOG=info

# Optional: Maximum number of retries for failed operations (default: 5)
MAX_RETRIES=5

# Optional: Delay between retries in milliseconds (default: 1000)
RETRY_DELAY_MS=1000
```

### Example: Using a Public RPC Endpoint

```bash
# Using the QuickNode endpoint from the example
RPC_URL="https://multi-distinguished-darkness.solana-mainnet.quiknode.pro/33e08e80b19c76330438eea5d2717a9564f21592/" \
WS_URL="wss://multi-distinguished-darkness.solana-mainnet.quiknode.pro/33e08e80b19c76330438eea5d2717a9564f21592/" \
RUST_LOG=debug \
cargo run --release
```

## Usage as a Library

You can also use this as a library in your own Rust project. Add this to your `Cargo.toml`:

```toml
[dependencies]
solana-mempool-sniper = { git = "https://github.com/yourusername/solana-mempool-sniper.git" }
```

### Example: Monitoring for Token Creations

```rust
use solana_mempool_sniper::{
    config::Config,
    mempool::MempoolMonitor,
    analyzer::{AnalyzerConfig, program_ids},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    env_logger::init();

    // Create configuration
    let config = Config::default();
    
    // Configure analyzer to monitor token creations
    let mut analyzer_config = AnalyzerConfig::default();
    analyzer_config.monitor_token_creations = true;
    
    // Create and initialize the mempool monitor
    let mut monitor = MempoolMonitor::with_analyzer(config, analyzer_config);
    
    // Connect to the Solana node
    monitor.connect().await?;
    
    // Subscribe to transaction logs
    let logs_receiver = monitor.subscribe_logs().await?;
    
    // Start processing transactions
    monitor.process_transactions(logs_receiver).await;
    
    Ok(())
}
```

### Example: Monitoring for DEX Pool Creations

```rust
use solana_mempool_sniper::{
    config::Config,
    mempool::MempoolMonitor,
    analyzer::{AnalyzerConfig, program_ids},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    env_logger::init();

    // Create configuration
    let config = Config::default();
    
    // Configure analyzer to monitor DEX pool creations
    let mut analyzer_config = AnalyzerConfig::default();
    analyzer_config.monitor_dex_listings = true;
    
    // Add DEX programs to monitor
    analyzer_config.dex_programs.insert(program_ids::RAYDIUM_AMM);
    analyzer_config.dex_programs.insert(program_ids::ORCA_WHIRLPOOLS);
    
    // Create and initialize the mempool monitor
    let mut monitor = MempoolMonitor::with_analyzer(config, analyzer_config);
    
    // Rest of the code remains the same as above
    // ...
}
```

Example usage:

```rust
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
    env_logger::init();

    // Create configuration
    let config = Config {
        rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
        ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
        ..Default::default()
    };

    // Create and initialize the mempool monitor
    let mut monitor = MempoolMonitor::new(config);
    
    // Connect to the Solana node
    monitor.connect().await?;
    
    // Subscribe to transaction logs
    let logs_receiver = monitor.subscribe_logs().await?;
    
    // Start processing transactions
    monitor.process_transactions(logs_receiver).await;
    
    Ok(())
}
```

## Monitoring Specific Programs

To monitor specific programs (like DEXs or NFT marketplaces), you can configure the `TransactionAnalyzer` with the program IDs you're interested in:

```rust
let mut analyzer_config = AnalyzerConfig::default();

// Add Raydium AMM program
analyzer_config.monitored_programs.insert(
    pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")
);

// Add Orca Whirlpools program
analyzer_config.monitored_programs.insert(
    pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc")
);

// Create analyzer with configuration
let analyzer = TransactionAnalyzer::new(analyzer_config);
```

## Example Output

When running the bot, you should see output similar to the following:

```
[2023-06-16T05:47:32Z INFO  solana_mempool_sniper] Starting Solana Mempool Sniper...
[2023-06-16T05:47:32Z INFO  solana_mempool_sniper] Using RPC endpoint: https://api.mainnet-beta.solana.com
[2023-06-16T05:47:32Z INFO  solana_mempool_sniper] Using WebSocket endpoint: wss://api.mainnet-beta.solana.com
[2023-06-16T05:47:32Z INFO  solana_mempool_sniper::mempool] Connecting to Solana node...
[2023-06-16T05:47:33Z INFO  solana_mempool_sniper::mempool] Successfully connected to Solana node
[2023-06-16T05:47:33Z INFO  solana_mempool_sniper::mempool] Subscribing to transaction logs...
[2023-06-16T05:47:34Z INFO  solana_mempool_sniper::mempool] Successfully subscribed to transaction logs
[2023-06-16T05:47:34Z INFO  solana_mempool_sniper::mempool] Starting transaction processing loop...
[2023-06-16T05:47:35Z DEBUG solana_mempool_sniper::mempool] Processing transaction: 5n7Wz5R2X3J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J
[2023-06-16T05:47:36Z INFO  solana_mempool_sniper::mempool] Interesting transaction found: 5n7Wz5R2X3J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J
[2023-06-16T05:47:36Z INFO  solana_mempool_sniper::mempool] Transaction 5n7Wz5R2X3J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J7J involves monitored programs or meets criteria
```

## Performance Considerations

1. **RPC Node Selection**: For optimal performance, use a dedicated RPC node or a premium service like QuickNode, Alchemy, or your own node.

2. **Rate Limiting**: Be aware of rate limits on your RPC endpoint. The bot includes basic rate limiting and retry logic, but you may need to adjust based on your provider's limits.

3. **Resource Usage**: The bot is designed to be efficient, but monitoring a high-traffic network like Solana can be resource-intensive. Monitor your system's resource usage, especially memory and network bandwidth.

4. **Connection Management**: The bot automatically handles reconnection on network issues, but you may want to implement additional monitoring for production use.

## Advanced Usage

### Custom Transaction Handlers

You can extend the bot by implementing custom transaction handlers. Here's an example of how to create a custom handler for token swaps:

```rust
use solana_mempool_sniper::{
    analyzer::{TransactionAnalyzer, AnalyzerConfig},
    mempool::MempoolMonitor,
};
use solana_sdk::{
    pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
};

struct SwapHandler {
    // Add any state or configuration needed for your handler
}

impl SwapHandler {
    async fn handle_swap(
        &self,
        tx: &VersionedTransaction,
        signature: &Signature,
    ) -> anyhow::Result<()> {
        // Implement your swap detection and handling logic here
        log::info!("Detected swap in transaction: {}", signature);
        
        // Example: Extract token addresses, amounts, etc.
        // let token_a = /* extract token A */;
        // let token_b = /* extract token B */;
        // let amount_in = /* extract input amount */;
        // let min_amount_out = /* calculate min output */;
        
        // Execute your strategy (e.g., front-run, back-run, arbitrage)
        // self.execute_strategy(token_a, token_b, amount_in, min_amount_out).await?;
        
        Ok(())
    }
}
```

### Monitoring Multiple Programs

To monitor multiple programs simultaneously, you can create multiple instances of `TransactionAnalyzer` with different configurations and combine their outputs.

## Testing

Run the test suite with:

```bash
cargo test -- --nocapture
```

For more verbose test output:

```bash
RUST_LOG=debug cargo test -- --nocapture
```

## Performance Benchmarks

To run benchmarks (requires nightly Rust):

```bash
cargo +nightly bench
```

## Security Considerations

1. **Private Keys**: Never commit private keys or sensitive information to version control. Use environment variables or secure secret management.

2. **RPC Endpoints**: Be cautious when using public RPC endpoints for trading, as they may have rate limits or be monitored.

3. **Transaction Simulation**: Always simulate transactions before sending them to the network to avoid unnecessary fees or failed transactions.

4. **Slippage**: Implement proper slippage protection when executing trades based on mempool data.

## Troubleshooting

### Common Issues

1. **Connection Issues**: If you're having trouble connecting to the Solana node, check:
   - Your internet connection
   - The RPC/WS endpoints are correct and accessible
   - Any firewall or network restrictions

2. **Rate Limiting**: If you encounter rate limiting, try:
   - Using a dedicated RPC endpoint
   - Implementing rate limiting in your code
   - Using multiple RPC endpoints with failover

3. **High Memory Usage**: If the bot is using too much memory:
   - Reduce the number of concurrent tasks
   - Increase the frequency of garbage collection
   - Limit the number of transactions being processed in parallel

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. Here are some areas where you can contribute:

1. **New Features**: Add support for monitoring additional program types or implementing new strategies.

2. **Performance Improvements**: Optimize the code for lower latency and higher throughput.

3. **Bug Fixes**: Report and fix any issues you encounter.

4. **Documentation**: Improve documentation, add examples, or write tutorials.

5. **Testing**: Add more test cases to improve code coverage.

## Support

If you find this project useful, please consider starring the repository or supporting the maintainers. For commercial support or custom development, please contact the maintainers directly.

## Acknowledgments

- [Solana](https://solana.com/) for building an amazing blockchain
- [Solana Program Library](https://spl.solana.com/) for the on-chain programs
- [Solana Labs](https://github.com/solana-labs) for the Rust SDK and client libraries
- The Rust community for building amazing tools and libraries

## Disclaimer

This software is provided "as is" and without warranty of any kind. Use at your own risk. The authors and contributors are not responsible for any financial losses or damages incurred while using this software. Always do your own research and understand the risks involved in cryptocurrency trading and blockchain technology.
