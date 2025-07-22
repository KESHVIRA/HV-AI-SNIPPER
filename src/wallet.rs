use log::{error, info};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use std::env;

/// Load a Solana keypair from either a file path or a base58 string in the environment.
/// Priority: 1) ENV var BASE58_KEYPAIR  2) ENV var KEYPAIR_PATH
pub fn load_keypair() -> anyhow::Result<Keypair> {
    // Check for base58-encoded keypair in env
    if let Ok(base58_str) = env::var("BASE58_KEYPAIR") {
        info!(" Loading keypair from BASE58_KEYPAIR env var...");

        let bytes = bs58::decode(base58_str.trim())
            .into_vec()
            .map_err(|e| anyhow::anyhow!("Failed to decode BASE58_KEYPAIR: {:?}", e))?;

        if bytes.len() != 64 {
            return Err(anyhow::anyhow!(
                "Expected 64-byte base58 keypair, got {} bytes",
                bytes.len()
            ));
        }

        let keypair = Keypair::from_bytes(&bytes)
            .map_err(|e| anyhow::anyhow!("Failed to parse keypair from bytes: {:?}", e))?;

        info!(
            " Loaded keypair from base58 env, pubkey: {}",
            keypair.pubkey()
        );
        return Ok(keypair);
    }

    // Else try to load from file
    let path = env::var("KEYPAIR_PATH").unwrap_or_else(|_| "~/.config/solana/id.json".to_string());
    let expanded_path = shellexpand::tilde(&path);

    info!(" Loading keypair from file: {}", expanded_path);

    let keypair = read_keypair_file(expanded_path.as_ref())
        .map_err(|e| anyhow::anyhow!("Failed to load keypair file: {:?}", e))?;

    info!(" Loaded keypair from file, pubkey: {}", keypair.pubkey());
    Ok(keypair)
}

/// Check if the wallet has at least `min_sol` amount of SOL.
pub async fn check_funding(keypair: &Keypair, rpc_url: &str, min_sol: f64) -> anyhow::Result<()> {
    use solana_client::nonblocking::rpc_client::RpcClient;

    let client = RpcClient::new(rpc_url.to_string());

    let balance = client
        .get_balance(&keypair.pubkey())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch wallet balance: {:?}", e))?;

    let sol = balance as f64 / 1_000_000_000.0;

    if sol < min_sol {
        error!(
            " Wallet only has {:.4} SOL (required: {:.4} SOL)",
            sol, min_sol
        );
        return Err(anyhow::anyhow!("Insufficient SOL in wallet"));
    }

    info!(
        " Wallet funded: {:.4} SOL available (min required: {:.4} SOL)",
        sol, min_sol
    );
    Ok(())
}
