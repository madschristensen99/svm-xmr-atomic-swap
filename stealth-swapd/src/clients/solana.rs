use crate::config::SolanaConfig;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct OnchainSwapInfo {
    pub swap_id: [u8; 32],
    pub secret_hash: [u8; 32],
    pub usdc_amount: u64,
    pub is_redeemed: bool,
    pub is_refunded: bool,
}

#[derive(Clone)]
pub struct SolanaClient {
    pub config: SolanaConfig,
    pub rpc_url: String,
    pub keypair_path: String,
    pub usdc_mint: String,
}

impl SolanaClient {
    pub async fn new(config: &SolanaConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            rpc_url: config.rpc_url.clone(),
            keypair_path: config.keypair_path.to_string_lossy().into(),
            usdc_mint: config.usdc_mint.clone(),
        })
    }

    pub fn pubkey(&self) -> String {
        // Return a mock pubkey for demo purposes
        "G1BVSiFojnXFaPG1WUgJAcYaB7aGKLKWtSqhMreKgA82".to_string()
    }

    pub async fn refund_usdc(&self, _swap_id: [u8; 32]) -> Result<String> {
        Ok("refund_tx_placeholder".to_string())
    }

    pub async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }

    pub async fn get_block_height(&self) -> Result<u64> {
        Ok(123456)
    }

    pub async fn create_usdc_to_xmr_swap(&self, _swap_id: [u8; 32], _secret_hash: [u8; 32], _usdc_amount: u64) -> Result<String> {
        Ok("create_swap_tx_placeholder".to_string())
    }

    pub async fn get_swap(&self, _swap_id: [u8; 32]) -> Result<Option<OnchainSwapInfo>, anyhow::Error> {
        // Mock implementation - would fetch from Solana program
        Ok(None)
    }

    pub async fn trigger_onchain_refund(&self, _swap_id: [u8; 32]) -> Result<String> {
        // Mock implementation - would trigger refund on Solana
        Ok("refund_triggered".to_string())
    }
}