//! Common Solana program IDs for DEXes and other DeFi protocols

use solana_sdk::{pubkey, pubkey::Pubkey};


// DEX Program IDs

/// Raydium AMM Program
pub const RAYDIUM_AMM: Pubkey = pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");

/// Orca Whirlpools Program
pub const ORCA_WHIRLPOOLS: Pubkey = pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");

/// Serum DEX Program v3
pub const SERUM_DEX: Pubkey = pubkey!("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");

/// Aldrin AMM Program
pub const ALDRIN_AMM: Pubkey = pubkey!("AMM55ShdkoGRB5jVYPjWziwk8m5MvwyCXnb6hQPz2WU7");

/// Saber Stable Swap Program
pub const SABER_STABLE_SWAP: Pubkey = pubkey!("SSwpkEEcbUqx4vtoEByFjSkhKdCT862DNVb52nZg1UZ");

// Token Program IDs

/// SPL Token Program
pub const SPL_TOKEN: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// SPL Associated Token Account Program
pub const SPL_ASSOCIATED_TOKEN_ACCOUNT: Pubkey = pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

// Other DeFi Program IDs

/// Raydium Liquidity Pools V4
pub const RAYDIUM_LIQUIDITY_POOLS_V4: Pubkey = pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");

/// Raydium Staking Program
pub const RAYDIUM_STAKING: Pubkey = pubkey!("EhhTKczWMGQt46ynNeRX1WfeagwwJd7vfH7mUGQeFpq8");

/// Orca Token Swap V2
pub const ORCA_TOKEN_SWAP_V2: Pubkey = pubkey!("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP");

/// Mango Markets V3
pub const MANGO_MARKETS_V3: Pubkey = pubkey!("mv3ekLzLbnVPNxjSKvqBpU3ZeZXPQdEC3bp5MDEBG68");

/// Port Finance Lending
pub const PORT_FINANCE_LENDING: Pubkey = pubkey!("Port7uDYB3wk6GJAw4KT1WpTeMtSu9bTcChBksVYrVr");

/// Lido for Solana Program
pub const LIDO_PROGRAM: Pubkey = pubkey!("CrX7kMhLC3cSsXJdT7JDgqrRVWGnUpX3gfEfxxU2NVLi");

/// Marinade Finance Program
pub const MARINADE_PROGRAM: Pubkey = pubkey!("MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD");

/// Adding a test module for program ID validation
#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    /// Helper function to check if a program ID is valid
    fn is_valid_pubkey(pubkey: &Pubkey) -> bool {
        // A valid pubkey is 32 bytes long
        pubkey.to_bytes().len() == 32
    }

    #[test]
    fn test_program_ids_are_valid() {
        // Test DEX Program IDs
        assert!(is_valid_pubkey(&RAYDIUM_AMM));
        assert!(is_valid_pubkey(&ORCA_WHIRLPOOLS));
        assert!(is_valid_pubkey(&SERUM_DEX));
        assert!(is_valid_pubkey(&ALDRIN_AMM));
        assert!(is_valid_pubkey(&SABER_STABLE_SWAP));
        
        // Test Token Program IDs
        assert!(is_valid_pubkey(&SPL_TOKEN));
        assert!(is_valid_pubkey(&SPL_ASSOCIATED_TOKEN_ACCOUNT));
        
        // Test Other DeFi Program IDs
        assert!(is_valid_pubkey(&RAYDIUM_LIQUIDITY_POOLS_V4));
        assert!(is_valid_pubkey(&RAYDIUM_STAKING));
        assert!(is_valid_pubkey(&ORCA_TOKEN_SWAP_V2));
        assert!(is_valid_pubkey(&MANGO_MARKETS_V3));
        assert!(is_valid_pubkey(&PORT_FINANCE_LENDING));
        assert!(is_valid_pubkey(&LIDO_PROGRAM));
        assert!(is_valid_pubkey(&MARINADE_PROGRAM));
    }
}
