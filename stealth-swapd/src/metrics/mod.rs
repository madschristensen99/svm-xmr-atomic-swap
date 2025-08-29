use prometheus::{
    CounterVec, Gauge, GaugeVec, Registry, TextEncoder,
    Opts,
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct MetricsCollector {
    registry: Registry,
    swaps_total: CounterVec,
    swaps_duration_seconds: GaugeVec,
    monero_wallet_balance_xmr: Gauge,
    solana_wallet_balance_usdc: Gauge,
    relayer_fees_earned_usdc: Gauge,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let registry = Registry::new();

        // Swaps total counter
        let swaps_total = CounterVec::new(
            Opts::new("swaps_total", "Total number of swaps by direction and state"),
            &["direction", "state"]
        ).unwrap();
        registry.register(Box::new(swaps_total.clone())).unwrap();

        // Swap duration gauge
        let swaps_duration_seconds = GaugeVec::new(
            Opts::new("swaps_duration_seconds", "Duration of swaps by direction and state"),
            &["direction", "state"]
        ).unwrap();
        registry.register(Box::new(swaps_duration_seconds.clone())).unwrap();

        // Wallet balance gauges
        let monero_wallet_balance_xmr = Gauge::new(
            "monero_wallet_balance_xmr", 
            "Current Monero wallet balance in atomic units"
        ).unwrap();
        registry.register(Box::new(monero_wallet_balance_xmr.clone())).unwrap();

        let solana_wallet_balance_usdc = Gauge::new(
            "solana_wallet_balance_usdc",
            "Current Solana wallet balance in USDC (6 decimal)"
        ).unwrap();
        registry.register(Box::new(solana_wallet_balance_usdc.clone())).unwrap();

        // Relayer fees counter
        let relayer_fees_earned_usdc = Gauge::new(
            "relayer_fees_earned_usdc",
            "Total relayer fees earned in USDC (6 decimal)"
        ).unwrap();
        registry.register(Box::new(relayer_fees_earned_usdc.clone())).unwrap();

        Self {
            registry,
            swaps_total,
            swaps_duration_seconds,
            monero_wallet_balance_xmr,
            solana_wallet_balance_usdc,
            relayer_fees_earned_usdc,
        }
    }

    pub fn increment_quotes_generated(&self) {
        self.swaps_total.with_label_values(&["na", "quoted"]).inc();
    }

    pub fn increment_swaps_accepted(&self, direction: &str) {
        self.swaps_total.with_label_values(&[direction, "accepted"]).inc();
    }

    pub fn increment_swaps_redeemed(&self) {
        self.swaps_total.with_label_values(&["na", "redeemed"]).inc();
    }

    pub fn increment_swaps_refunded(&self) {
        self.swaps_total.with_label_values(&["na", "refunded"]).inc();
    }

    pub fn increment_swaps_failed(&self) {
        self.swaps_total.with_label_values(&["na", "failed"]).inc();
    }

    pub fn set_monero_balance(&self, balance: u64) {
        self.monero_wallet_balance_xmr.set(balance as f64);
    }

    pub fn set_solana_balance(&self, balance: u64) {
        self.solana_wallet_balance_usdc.set(balance as f64);
    }

    pub fn add_relayer_fee(&self, fee: u64) {
        self.relayer_fees_earned_usdc.add(fee as f64);
    }

    pub fn export(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families).unwrap_or_default()
    }

    pub fn get_metrics(&self) -> HashMap<String, u64> {
        let mut metrics = HashMap::new();
        
        // Collect current metrics values
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        
        if let Ok(metrics_text) = encoder.encode_to_string(&metric_families) {
            // Parse back to usable format for API endpoint
            for line in metrics_text.lines() {
                if line.starts_with("swaps_total") || 
                   line.starts_with("swaps_duration_seconds") ||
                   line.starts_with("monero_wallet_balance_xmr") ||
                   line.starts_with("solana_wallet_balance_usdc") ||
                   line.starts_with("relayer_fees_earned_usdc") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(value) = parts[1].parse::<f64>() {
                            metrics.insert(line.split_whitespace().next().unwrap_or("").to_string(), value as u64);
                        }
                    }
                }
            }
        }

        metrics
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}