// tests/test_config.rs
use solana_mempool_sniper::config::Config;

pub fn test_config() -> Config {
    Config {
        rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
        ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
        ..Default::default()
    }
}