use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    UsdcToXmr,
    XmrToUsdc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapState {
    Quoted,
    LockedUsdc,
    LockedXmr,
    Redeemed,
    Refunded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapTrade {
    pub swap_id: [u8; 32],
    pub quote_id: uuid::Uuid,
    pub direction: Direction,
    pub usdc_amount: u64,
    pub xmr_amount: u64,
    pub secret_hash: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub monero_sub_address: [u8; 64],
    pub alice_solana: Option<String>,
    pub state: SwapState,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub monero_txid: Option<String>,
    pub solana_signature: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QuoteRequest {
    pub direction: Direction,
    pub usdc_amount: u64,
    pub xmr_amount: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuoteResponse {
    pub quote_id: uuid::Uuid,
    pub expires_at: DateTime<Utc>,
    pub usdc_amount: u64,
    pub xmr_amount: u64,
    pub secret_hash: [u8; 32],
    #[serde(with = "serde_bytes")]
    pub monero_sub_address: [u8; 64],
    pub solana_address: String,
}