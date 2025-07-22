//! Transaction analysis module for the Solana Mempool Sniper.

// use bincode::deserialize;
// use bs58::decode;
use log::{debug, info};
// use sha2::{Digest};
// use sha3::Keccak256;
use solana_sdk::{signature::Signature, transaction::VersionedTransaction};
// use solana_transaction_status::{
//     EncodedTransaction, UiInstruction, UiMessage,
//     UiPartiallyDecodedInstruction,
// };
// use spl_associated_token_account::get_associated_token_address;
// use spl_token::{state::Account as TokenAccount, state::Mint};
use std::collections::HashSet;

use solana_sdk::pubkey::Pubkey;
// use crate::analyzer::program_ids;
use spl_token::ID as TOKEN_PROGRAM_ID;



/// Configuration for the transaction analyzer
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    /// Set of program IDs to monitor
    pub monitored_programs: HashSet<Pubkey>,
    /// Minimum SOL transfer amount to consider (in lamports)
    pub min_sol_transfer: Option<u64>,
    /// Whether to monitor new token creations
    pub monitor_token_creations: bool,
    /// Whether to monitor new DEX pool listings
    pub monitor_dex_listings: bool,
    /// Known DEX program IDs to monitor for new listings
    pub dex_programs: HashSet<Pubkey>,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        use crate::analyzer::program_ids::*;
        let mut dex_programs = HashSet::new();
        dex_programs.insert(RAYDIUM_AMM);
        dex_programs.insert(ORCA_WHIRLPOOLS);
        dex_programs.insert(SERUM_DEX);

        Self {
            monitored_programs: HashSet::new(),
            min_sol_transfer: None,
            monitor_token_creations: true,
            monitor_dex_listings: true,
            dex_programs,
        }
    }
}

/// Represents a detected token creation event
#[derive(Clone, Debug)]
pub struct TokenCreationEvent {
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub decimals: u8,
    pub supply: u64,
    pub signature: Signature,
}

/// Represents a detected DEX pool creation event
#[derive(Clone, Debug)]
pub struct DexPoolCreationEvent {
    pub pool: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub dex_program: Pubkey,
    pub signature: Signature,
}

/// Represents the result of transaction analysis
#[derive(Debug)]
pub enum AnalysisResult {
    /// A new token was created
    TokenCreation(TokenCreationEvent),
    /// A new DEX pool was created
    DexPoolCreation(DexPoolCreationEvent),
    /// A monitored program was called
    MonitoredProgramInteraction {
        program_id: Pubkey,
        signature: Signature,
    },
}

/// Analyzes transactions for interesting patterns
#[derive(Clone)]
pub struct TransactionAnalyzer {
    config: AnalyzerConfig,
}

impl TransactionAnalyzer {
    /// Create a new TransactionAnalyzer with the given configuration
    pub fn new(config: AnalyzerConfig) -> Self {
        Self { config }
    }

    /// Analyzes a transaction for interesting events
    pub async fn analyze_transaction(
        &self,
        tx: &VersionedTransaction,
        signature: &Signature,
    ) -> anyhow::Result<Vec<AnalysisResult>> {
        // use program_ids::TOKEN_PROGRAM_ID;

        let message = &tx.message; // Accessing the 'message' field directly

        let account_keys: Vec<_> = message.static_account_keys().iter().collect();
        let mut results = Vec::new();

        // Check for token creations if enabled
        if self.config.monitor_token_creations {
            if let Some(event) = self
                .detect_token_creation(tx, signature, &account_keys)
                .await?
            {
                results.push(AnalysisResult::TokenCreation(event));
            }
        }

        // Check for DEX pool listings if enabled
        if self.config.monitor_dex_listings {
            if let Some(event) = self
                .detect_dex_pool_creation(tx, signature, &account_keys)
                .await?
            {
                results.push(AnalysisResult::DexPoolCreation(event));
            }
        }

        // Check for monitored program interactions
        for instruction in message.instructions() {
            if let Some(program_id) = account_keys.get(instruction.program_id_index as usize) {
                if self.config.monitored_programs.contains(program_id) {
                    debug!(
                        "Found transaction calling monitored program: {}",
                        program_id
                    );
                    results.push(AnalysisResult::MonitoredProgramInteraction {
                        program_id: **program_id, // Dereference only once if needed
                        signature: *signature,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Detects token creation events
    async fn detect_token_creation(
        &self,
        tx: &VersionedTransaction,
        signature: &Signature,
        account_keys: &[&Pubkey],
    ) -> anyhow::Result<Option<TokenCreationEvent>> {


        // Assuming tx is of type `VersionedTransaction`
        let message = &tx.message; // Directly access the field
        for (_i, instruction) in message.instructions().iter().enumerate() {
            // Check if this is a call to the SPL Token program
            if let Some(program_id) = account_keys.get(instruction.program_id_index as usize) {
                if *program_id != &TOKEN_PROGRAM_ID {
                    continue;
                }

                // The first byte of the instruction data is the instruction discriminator
                // For InitializeMint, it's 0
                if !instruction.data.is_empty() && instruction.data[0] == 0 {
                    // Try to parse the mint account from the accounts list
                    // The first account is typically the mint account being initialized
                    if let Some(&&mint_account) = account_keys.get(0) {
                        // The second account is typically the mint authority
                        if let Some(&&mint_authority) = account_keys.get(1) {
                            // The mint data is in the instruction data
                            // Format: [0, decimals, [mint_authority], [freeze_authority]]
                            if instruction.data.len() >= 1 + 1 + 1 + 32 + 1 + 32 {
                                let decimals = instruction.data[1];
                                // Skip the freeze authority check for simplicity

                                info!(
                                    "Detected new token creation: mint={}, creator={}, decimals={}",
                                    mint_account, mint_authority, decimals
                                );

                                return Ok(Some(TokenCreationEvent {
                                    mint: mint_account,
                                    creator: mint_authority, // Use mint_authority directly if it's already a Pubkey
                                    decimals,
                                    supply: 0, // Will need to be fetched separately
                                    signature: *signature,
                                }));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Detects DEX pool creation events
    async fn detect_dex_pool_creation(
        &self,
        tx: &VersionedTransaction,
        signature: &Signature,
        account_keys: &[&Pubkey],
    ) -> anyhow::Result<Option<DexPoolCreationEvent>> {
        let message = tx.message.clone();

        for instruction in message.instructions() {
            if let Some(program_id) = account_keys.get(instruction.program_id_index as usize) {
                // Check if this is a known DEX program
                if !self.config.dex_programs.contains(program_id) {
                    continue;
                }

                // For Raydium AMM, look for Initialize instruction
               if **program_id == crate::analyzer::program_ids::RAYDIUM_AMM {
                    // Raydium's Initialize instruction has discriminator 1
                    if !instruction.data.is_empty() && instruction.data[0] == 1 {
                        // First few accounts should be the pool, token A, token B, etc.
                        if account_keys.len() >= 3 {
                            let pool_account = account_keys[0];
                            let token_a = account_keys[1];
                            let token_b = account_keys[2];

                            info!(
                                "Detected new Raydium pool: pool={}, token_a={}, token_b={}",
                                pool_account, token_a, token_b
                            );

                            return Ok(Some(DexPoolCreationEvent {
                                pool: *pool_account,
                                token_a: *token_a,
                                token_b: *token_b,
                                dex_program: **program_id,
                                signature: *signature,
                            }));
                        }
                    }
                }
                // Add detection for other DEXes here
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signer::Signer;
    use solana_sdk::{signature::Keypair, system_instruction, transaction::Transaction};

    #[tokio::test]
    #[tokio::test]
async fn test_analyze_transaction_with_monitored_program() {
    let program_id = Keypair::new().pubkey();
    let mut config = AnalyzerConfig::default();
    config.monitored_programs.insert(program_id);
    let analyzer = TransactionAnalyzer::new(config);

    let from_keypair = Keypair::new();
    let to_pubkey = Keypair::new().pubkey();
    let instruction = system_instruction::transfer(&from_keypair.pubkey(), &to_pubkey, 1000);
    let recent_blockhash = Pubkey::new_unique(); // Mock blockhash
    let mut tx = Transaction::new_with_payer(&[instruction], Some(&from_keypair.pubkey()));
    tx.sign(&[&from_keypair], recent_blockhash);
    let versioned_tx = VersionedTransaction::try_from(tx).unwrap();
    let signature = *tx.signatures.first().unwrap();

    let results = analyzer.analyze_transaction(&versioned_tx, &signature).await.unwrap();
    assert_eq!(results.len(), 0); // No monitored program match
}
}
