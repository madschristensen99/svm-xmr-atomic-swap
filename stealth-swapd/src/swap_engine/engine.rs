use crate::config::AppConfig;
use crate::clients::{SolanaClient, MoneroClient};
use crate::metrics::MetricsCollector;
use crate::security::KeyDerivation;
use crate::swap_engine::{SwapTrade, SwapState, Direction, QuoteRequest, QuoteResponse};

use std::collections::HashMap;
use std::sync::Arc;
use chrono::{Utc, Duration};
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Clone)]
pub struct SwapEngine {
    config: AppConfig,
    solana_client: SolanaClient,
    monero_client: std::sync::Arc<MoneroClient>,
    metrics: Arc<MetricsCollector>,
    active_swaps: Arc<RwLock<HashMap<[u8; 32], SwapTrade>>>,
    quotes: Arc<RwLock<HashMap<uuid::Uuid, SwapTrade>>>,
}

impl SwapEngine {
    pub async fn new(
        config: AppConfig,
        solana_client: SolanaClient,
        monero_client: MoneroClient,
        metrics: MetricsCollector,
    ) -> Result<Self> {
        let client = Self {
            config,
            solana_client,
            monero_client: std::sync::Arc::new(monero_client),
            metrics: Arc::new(metrics),
            active_swaps: Arc::new(RwLock::new(HashMap::new())),
            quotes: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load saved swaps from database if they exist
        client.load_persisted_swaps().await?;
        
        Ok(client)
    }

    pub async fn generate_quote(&self, request: QuoteRequest) -> Result<QuoteResponse> {
        self.validate_trade_parameters(request.direction, request.usdc_amount, request.xmr_amount)?;
        
        let quote_id = uuid::Uuid::new_v4();
        let secret_hash = KeyDerivation::derive_secret_hash(&KeyDerivation::generate_adaptor_secret());
        
        let (_monero_address, monero_sub_address) = self.monero_client
            .create_subaddress(&format!("swap_{}", quote_id))
            .await?;
        
        let expires_at = Utc::now() + Duration::minutes(30);
        
        let quote = SwapTrade {
            swap_id: KeyDerivation::generate_swap_id(),
            quote_id,
            direction: request.direction,
            usdc_amount: request.usdc_amount,
            xmr_amount: request.xmr_amount,
            secret_hash,
            monero_sub_address,
            alice_solana: None,
            state: SwapState::Quoted,
            created_at: Utc::now(),
            expires_at,
            monero_txid: None,
            solana_signature: None,
            failure_reason: None,
        };

        {
            let mut quotes = self.quotes.write().await;
            quotes.insert(quote_id, quote.clone());
        }

        self.metrics.increment_quotes_generated();

        Ok(QuoteResponse {
            quote_id,
            expires_at,
            usdc_amount: quote.usdc_amount,
            xmr_amount: quote.xmr_amount,
            secret_hash,
            monero_sub_address: quote.monero_sub_address,
            solana_address: self.solana_client.pubkey().to_string(),
        })
    }

    pub async fn accept_swap(&self, quote_id: uuid::Uuid, alice_solana: Option<String>) -> Result<[u8; 32]> {
        let mut quote = {
            let mut quotes = self.quotes.write().await;
            quotes.remove(&quote_id)
                .ok_or_else(|| anyhow::anyhow!("Quote not found"))?
        };

        if Utc::now() > quote.expires_at {
            return Err(anyhow::anyhow!("Quote expired"));
        }

        quote.alice_solana = alice_solana;
        quote.state = match quote.direction {
            Direction::UsdcToXmr => SwapState::LockedUsdc,
            Direction::XmrToUsdc => SwapState::LockedXmr,
        };

        {
            let mut active_swaps = self.active_swaps.write().await;
            active_swaps.insert(quote.swap_id, quote.clone());
        }

        Ok(quote.swap_id)
    }

    pub async fn get_swap_status(&self, swap_id: [u8; 32]) -> Option<SwapTrade> {
        let active_swaps = self.active_swaps.read().await;
        active_swaps.get(&swap_id).cloned()
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            self.process_expired_swaps().await?;
            self.process_pending_swaps().await?;
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    }

    async fn process_expired_swaps(&self) -> Result<()> {
        let now = Utc::now();
        let mut expired_swaps = Vec::new();

        {
            let active_swaps = self.active_swaps.read().await;
            for (swap_id, swap) in active_swaps.iter() {
                if now > swap.expires_at 
                    && (swap.state == SwapState::Quoted || swap.state == SwapState::LockedUsdc || swap.state == SwapState::LockedXmr) {
                    expired_swaps.push(*swap_id);
                }
            }
        }

        for swap_id in expired_swaps {
            self.refund_swap(swap_id).await?;
            // Also trigger refund on blockchain if necessary
            self.trigger_onchain_refund(swap_id).await?;
        }

        Ok(())
    }

    async fn process_pending_swaps(&self) -> Result<()> {
        let pending_swaps: Vec<SwapTrade> = {
            let active_swaps = self.active_swaps.read().await;
            active_swaps
                .values()
                .filter(|swap| 
                    swap.state == SwapState::LockedUsdc || swap.state == SwapState::LockedXmr)
                .cloned()
                .collect()
        };

        for swap in pending_swaps {
            self.process_swap_completion(&swap).await?;
        }

        Ok(())
    }

    async fn process_swap_completion(&self, swap: &SwapTrade) -> Result<()> {
        match swap.direction {
            Direction::UsdcToXmr => self.process_usdc_to_xmr_completion(swap).await,
            Direction::XmrToUsdc => self.process_xmr_to_usdc_completion(swap).await,
        }
    }

    async fn process_usdc_to_xmr_completion(&self, swap: &SwapTrade) -> Result<()> {
        match swap.state {
            SwapState::LockedUsdc => {
                // Monitor Monero blockchain for XMR lock
                if let Some(monero_txid) = &swap.monero_txid {
                    if let Some(confirmed) = self.check_monero_deposit(monero_txid, swap.xmr_amount).await? {
                        if confirmed {
                            // Update state to LockedXmr
                            let mut active_swaps = self.active_swaps.write().await;
                            if let Some(swap) = active_swaps.get_mut(&swap.swap_id) {
                                swap.state = SwapState::LockedXmr;
                                self.persist_swap(swap).await?;
                            }
                        }
                    }
                }
            },
            SwapState::LockedXmr => {
                // Monitor for Alice's USDC redemption or timeout
                let now = Utc::now();
                if now > swap.expires_at {
                    // Alice can now claim USDC with adaptor signature
                    self.check_adaptor_redeemption(swap).await?;
                }
            },
            _ => {},
        }
        Ok(())
    }

    async fn process_xmr_to_usdc_completion(&self, swap: &SwapTrade) -> Result<()> {
        match swap.state {
            SwapState::LockedXmr => {
                // Monitor Solana for Bob's USDC lock
                if let Ok(Some(onchain_swap)) = self.solana_client.get_swap(swap.swap_id).await {
                    if onchain_swap.usdc_amount == swap.usdc_amount {
                        // Update state to LockedUsdc
                        let mut active_swaps = self.active_swaps.write().await;
                        if let Some(swap) = active_swaps.get_mut(&swap.swap_id) {
                            swap.state = SwapState::LockedUsdc;
                            self.persist_swap(swap).await?;
                        }
                    }
                }
            },
            SwapState::LockedUsdc => {
                // Monitor for redemption or timeout
                let _now = Utc::now();
                if let Some(_alice_pubkey) = &swap.alice_solana {
                    // Check if Alice has redeemed with adaptor signature
                    if self.check_adaptor_redeemption(swap).await? {
                        // Alice has revealed the secret, Bob can unlock XMR
                        self.unlock_xmr(swap).await?;
                    }
                }
            },
            _ => {},
        }
        Ok(())
    }

    async fn refund_swap(&self, swap_id: [u8; 32]) -> Result<()> {
        let mut swap = {
            let active_swaps = self.active_swaps.read().await;
            match active_swaps.get(&swap_id) {
                Some(swap) => swap.clone(),
                None => return Ok(()),
            }
        };

        swap.state = SwapState::Refunded;
        swap.failure_reason = Some("Swap expired".to_string());

        {
            let mut active_swaps = self.active_swaps.write().await;
            active_swaps.insert(swap_id, swap.clone());
        }

        self.metrics.increment_swaps_failed();
        let _ = self.emit_failed_event(&swap).await;

        Ok(())
    }

    fn validate_trade_parameters(&self, _direction: Direction, usdc_amount: u64, _xmr_amount: u64) -> Result<()> {
        if usdc_amount < self.config.quoting.min_usdc || usdc_amount > self.config.quoting.max_usdc {
            return Err(anyhow::anyhow!("USDC amount out of allowed range"));
        }
        Ok(())
    }

    async fn check_monero_deposit(&self, txid: &str, amount: u64) -> Result<Option<bool>> {
        if let Some(transfer) = self.monero_client.get_transfers(txid).await? {
            let received_amount = transfer["amount"].as_u64().unwrap_or(0);
            let confirmations = transfer["confirmations"].as_u64().unwrap_or(0);
            
            if confirmations >= 10 && received_amount >= amount {
                return Ok(Some(true));
            }
        }
        
        Ok(None)
    }

    async fn trigger_onchain_refund(&self, swap_id: [u8; 32]) -> Result<()> {
        self.solana_client.refund_usdc(swap_id).await?;
        Ok(())
    }

    async fn load_persisted_swaps(&self) -> Result<()> {
        // Load saved swaps from database
        // This would query SQLite to restore state after restart
        tracing::info!("Loading persisted swaps from database...");
        Ok(())
    }

    async fn persist_swap(&self, swap: &SwapTrade) -> Result<()> {
        // Persist swap state to database
        // This would insert/update into SQLite
        tracing::debug!("Persisting swap: {} ", hex::encode(&swap.swap_id));
        Ok(())
    }

    async fn emit_failed_event(&self, swap: &SwapTrade) -> Result<()> {
        if let Ok(webhook_url) = std::env::var("FAIL_WEBHOOK_URL") {
            let payload = serde_json::json!({
                "swap_id": hex::encode(&swap.swap_id),
                "state": format!("{:?}", swap.state),
                "failure_reason": swap.failure_reason,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            let client = reqwest::Client::new();
            if let Err(e) = client.post(&webhook_url).json(&payload).send().await {
                tracing::error!("Failed to emit webhook: {}", e);
            }
        }
        Ok(())
    }

    async fn unlock_xmr(&self, swap: &SwapTrade) -> Result<()> {
        let monero_address = Self::bytes_to_address_str(&swap.monero_sub_address);
        let tx_hash = self.monero_client.send_transfer(
            &monero_address,
            swap.xmr_amount
        ).await?;
        
        let mut active_swaps = self.active_swaps.write().await;
        if let Some(swap) = active_swaps.get_mut(&swap.swap_id) {
            swap.state = SwapState::Redeemed;
            swap.solana_signature = Some(tx_hash);
        }
        
        self.metrics.increment_swaps_refunded();
        
        Ok(())
    }

    fn bytes_to_address_str(bytes: &[u8; 64]) -> String {
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end]).into_owned()
    }

    async fn check_adaptor_redeemption(&self, swap: &SwapTrade) -> Result<bool> {
        if let Ok(Some(onchain_swap)) = self.solana_client.get_swap(swap.swap_id).await {
            if onchain_swap.is_redeemed {
                return Ok(true);
            }
            if onchain_swap.is_refunded {
                let mut active_swaps = self.active_swaps.write().await;
                if let Some(swap) = active_swaps.get_mut(&swap.swap_id) {
                    swap.state = SwapState::Refunded;
                    swap.failure_reason = Some("Refunded".to_string());
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
