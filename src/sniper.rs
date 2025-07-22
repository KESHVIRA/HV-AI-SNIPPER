/// Sells a token on Raydium by swapping it back to SOL/USDC (reverse of snipe_on_raydium)
pub async fn sell_on_raydium(
    rpc: &RpcClient,
    payer: &Keypair,
    pool: RaydiumPoolAccounts,
    raydium_program_id: Pubkey,
    amount_in_token: u64,
) -> anyhow::Result<Signature> {
    use spl_associated_token_account::get_associated_token_address;
    // For sell, user_source is the sniped token, user_destination is wSOL/USDC
    let user_source = get_associated_token_address(&payer.pubkey(), &pool.serum_coin_mint); // token to sell
    let user_destination = get_associated_token_address(&payer.pubkey(), &pool.serum_pc_mint); // receive SOL/USDC

    let ix = build_raydium_swap_instruction(
        user_source,
        user_destination,
        payer.pubkey(),
        &pool,
        SwapInstructionArgs {
            amount_in: amount_in_token,
            min_amount_out: 1, // allow dust for now
        },
        raydium_program_id,
    )?;

    let blockhash = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], blockhash);

    let sig = retry_with_backoff(|| async {
        rpc
            .send_and_confirm_transaction_with_spinner_and_commitment(
                &tx,
                CommitmentConfig::processed(),
            )
            .await
            .map_err(anyhow::Error::from)
    })
    .await?;
    Ok(sig)
}
/// Send alert to Telegram/Slack (stub)
pub async fn send_alert(msg: &str) -> anyhow::Result<()> {
    // TODO: Implement Telegram/Slack webhook
    log::info!("[ALERT] {}", msg);
    Ok(())
}

/// Log event to file (stub)
pub fn log_event_to_file(msg: &str) -> anyhow::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    let mut file = OpenOptions::new().create(true).append(true).open("sniper_events.log")?;
    writeln!(file, "{}", msg)?;
    Ok(())
}
use bincode::serialize;
use solana_sdk::system_instruction;
use log::info;
use serde::Serialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    message::Message,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};

use solana_sdk::{pubkey::Pubkey, account::Account};
use std::time::Duration;
use tokio::time::sleep;


#[derive(Debug)]
pub struct SwapInstructionArgs {
    pub amount_in: u64,
    pub min_amount_out: u64,
}
const MAX_SLIPPAGE_BPS: u64 = 100; // 1% max slippage

/// Raydium SwapBaseIn layout (hardcoded instruction tag = 9)
#[derive(Debug, Clone, Serialize)]
pub struct SwapBaseInLayout {
    pub instruction: u8,     // = 9 for SwapBaseIn
    pub amount_in: u64,      // amount_in (e.g. 0.05 SOL in lamports)
    pub min_amount_out: u64, // minimum out tokens (to avoid slippage loss)
}

/// Pool metadata needed for a Raydium swap (simplified).
/// We'll expand this with real pool and token account info later.
#[derive(Debug, Clone)]
pub struct RaydiumPoolAccounts {
    pub amm_id: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_target_orders: Pubkey,
    pub pool_coin_token_account: Pubkey,
    pub pool_pc_token_account: Pubkey,
    pub serum_market: Pubkey,
    pub serum_bids: Pubkey,
    pub serum_asks: Pubkey,
    pub serum_event_queue: Pubkey,
    pub serum_base_vault: Pubkey,
    pub serum_quote_vault: Pubkey,
    pub serum_open_orders: Pubkey,
    pub serum_request_queue: Pubkey,
    pub serum_coin_mint: Pubkey,
    pub serum_pc_mint: Pubkey,
    pub serum_vault_signer: Pubkey,
}



/// Fetch and build a real RaydiumPoolAccounts from pool pubkey
pub async fn fetch_raydium_pool_accounts(
    rpc: &RpcClient,
    pool_id: Pubkey,
) -> anyhow::Result<RaydiumPoolAccounts> {
    let account: Account = rpc
        .get_account(&pool_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch pool account {}: {:?}", pool_id, e))?;

    let data = account.data;

    if data.len() < 324 {
        return Err(anyhow::anyhow!("Raydium pool account data too short"));
    }

    let amm_authority = Pubkey::new(&data[72..104]);
    let amm_open_orders = Pubkey::new(&data[104..136]);
    let amm_target_orders = Pubkey::new(&data[136..168]);
    let pool_coin_token_account = Pubkey::new(&data[168..200]);
    let pool_pc_token_account = Pubkey::new(&data[200..232]);
    let serum_market = Pubkey::new(&data[232..264]);
    let serum_bids = Pubkey::new(&data[264..296]);
    let serum_asks = Pubkey::new(&data[296..328]);
    let serum_event_queue = Pubkey::new(&data[328..360]);
    let serum_base_vault = Pubkey::new(&data[360..392]);
    let serum_quote_vault = Pubkey::new(&data[392..424]);
    let serum_open_orders = Pubkey::new(&data[424..456]);
    let serum_request_queue = Pubkey::new(&data[456..488]);
    let serum_coin_mint = Pubkey::new(&data[488..520]);
    let serum_pc_mint = Pubkey::new(&data[520..552]);
    let serum_vault_signer = Pubkey::new(&data[552..584]);

    Ok(RaydiumPoolAccounts {
        amm_id: pool_id,
        amm_authority,
        amm_open_orders,
        amm_target_orders,
        pool_coin_token_account,
        pool_pc_token_account,
        serum_market,
        serum_bids,
        serum_asks,
        serum_event_queue,
        serum_base_vault,
        serum_quote_vault,
        serum_open_orders,
        serum_request_queue,
        serum_coin_mint,
        serum_pc_mint,
        serum_vault_signer,
    })
}




/// Retries an async closure with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let mut delay = Duration::from_millis(200);
    let max_delay = Duration::from_secs(2);
    let max_retries = 5;

    for attempt in 0..max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(_) if attempt + 1 < max_retries => {
                sleep(delay).await;
                delay = std::cmp::min(delay * 2, max_delay);
            }
            Err(e) => return Err(e),
        }
    }

    // Fallback error, should never reach here if logic is correct
    panic!("retry_with_backoff exhausted retries without returning Ok");
}


/// Sends a Solana transaction using the provided instruction list and keypair.
/// Automatically signs, sends, and confirms the transaction with logs.
pub async fn send_transaction(
    rpc_client: &RpcClient,
    payer: &Keypair,
    instructions: Vec<Instruction>,
) -> anyhow::Result<Signature> {
    // Fetch the latest blockhash
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get latest blockhash: {:?}", e))?;

    // Build the transaction
    let message = Message::new(&instructions, Some(&payer.pubkey()));
    let tx = Transaction::new(&[payer], message, recent_blockhash);

    // Retry sending transaction with backoff
    let sig = retry_with_backoff(|| async {
        rpc_client
            .send_and_confirm_transaction_with_spinner_and_commitment(
                &tx,
                CommitmentConfig::processed(),
            )
            .await
            .map_err(anyhow::Error::from)
    })
    .await?;

    info!("✅ Transaction sent! Signature: {}", sig);
    Ok(sig)
}

// Wrap SOL
pub async fn wrap_sol(
    rpc_client: &RpcClient,
    payer: &Keypair,
    wsol_account: &Pubkey,
    amount: u64,
) -> anyhow::Result<Signature> {
    let blockhash = rpc_client.get_latest_blockhash().await?;
    
    let instructions = vec![
        system_instruction::create_account(
            &payer.pubkey(),
            wsol_account,
            amount,
            165,
            &spl_token::id(),
        ),
        spl_token::instruction::sync_native(&spl_token::id(), wsol_account)?,
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[payer],
        blockhash
    );

    let sig = rpc_client
        .send_and_confirm_transaction_with_spinner_and_commitment(
            &tx,
            CommitmentConfig::processed(),
        )
        .await?;

    info!(" Wrapped {} SOL into {}", amount as f64 / 1e9, wsol_account);
    Ok(sig)
}

fn calculate_price_impact(_pool: &RaydiumPoolAccounts, _amount_in: u64) -> anyhow::Result<u64> {
    // TODO: Implement price impact calculation using pool reserves
    // This is a placeholder - you should implement actual calculation
    Ok(50) // Returns 0.5% impact
}

pub fn build_raydium_swap_instruction(
    user_source: Pubkey,
    user_destination: Pubkey,
    user_owner: Pubkey,
    pool: &RaydiumPoolAccounts,
    args: SwapInstructionArgs,
    raydium_program_id: Pubkey,
) -> anyhow::Result<Instruction> {
    let ix_data = serialize(&SwapBaseInLayout {
        instruction: 9,
        amount_in: args.amount_in,
        min_amount_out: args.min_amount_out,
    })
    .map_err(|e| anyhow::anyhow!("Failed to serialize swap instruction: {:?}", e))?;

    let price_impact = calculate_price_impact(pool, args.amount_in)?;
    if price_impact > MAX_SLIPPAGE_BPS {
        return Err(anyhow::anyhow!("Price impact too high: {}%", price_impact/100));
    }

    Ok(Instruction {
        program_id: raydium_program_id,
        accounts: vec![
            AccountMeta::new(pool.amm_id, false),
            AccountMeta::new_readonly(pool.amm_authority, false),
            AccountMeta::new(pool.amm_open_orders, false),
            AccountMeta::new(pool.amm_target_orders, false),
            AccountMeta::new(pool.pool_coin_token_account, false),
            AccountMeta::new(pool.pool_pc_token_account, false),
            AccountMeta::new(pool.serum_market, false),
            AccountMeta::new(pool.serum_bids, false),
            AccountMeta::new(pool.serum_asks, false),
            AccountMeta::new(pool.serum_event_queue, false),
            AccountMeta::new(pool.serum_base_vault, false),
            AccountMeta::new(pool.serum_quote_vault, false),
            AccountMeta::new(pool.serum_open_orders, false),
            AccountMeta::new(pool.serum_request_queue, false),
            AccountMeta::new(pool.serum_coin_mint, false),
            AccountMeta::new(pool.serum_pc_mint, false),
            AccountMeta::new_readonly(pool.serum_vault_signer, false),
            AccountMeta::new(user_source, false),
            AccountMeta::new(user_destination, false),
            AccountMeta::new_readonly(user_owner, true),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: ix_data,
    })
}

pub async fn snipe_on_raydium(
    rpc: &RpcClient,
    payer: &Keypair,
    pool: RaydiumPoolAccounts,
    raydium_program_id: Pubkey,
    amount_in_lamports: u64,
) -> anyhow::Result<Signature> {
    // Step 1: resolve ATAs
    use spl_associated_token_account::get_associated_token_address;
    let user_source = get_associated_token_address(&payer.pubkey(), &pool.serum_pc_mint); // usually wSOL
    let user_destination = get_associated_token_address(&payer.pubkey(), &pool.serum_coin_mint); // token to buy

    // Step 2: Optional - check if ATA exists (not done for speed)

    // Step 3: Create Raydium instruction
    let ix = build_raydium_swap_instruction(
        user_source,
        user_destination,
        payer.pubkey(),
        &pool,
        SwapInstructionArgs {
            amount_in: amount_in_lamports,
            min_amount_out: 1, // allow dust for now, upgrade later
        },
        raydium_program_id,
    )?;

    // Step 4: Send TX
    let blockhash = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], blockhash);

let sig = retry_with_backoff(|| async {
    rpc
        .send_and_confirm_transaction_with_spinner_and_commitment(
            &tx,
            CommitmentConfig::processed(),
        )
        .await
        .map_err(anyhow::Error::from)
})
.await?;
 Ok(sig)
}
