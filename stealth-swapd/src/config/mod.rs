use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use secrecy::SecretString;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub solana: SolanaConfig,
    pub monero: MoneroConfig,
    pub quoting: QuotingConfig,
    pub relayer: RelayerConfig,
    pub logging: LoggingConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub keypair_path: PathBuf,
    pub usdc_mint: String,
    pub commitment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoneroConfig {
    pub wallet_rpc_url: String,
    pub wallet_file: String,
    pub password_env: String,
    pub daemon_url: Option<String>,
    pub daemon_username: Option<String>,
    pub daemon_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotingConfig {
    pub min_usdc: u64,
    pub max_usdc: u64,
    pub spread_bps: u64,
    pub expiry_minutes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerConfig {
    pub enabled: bool,
    pub fee_bps: u64,
    pub max_gas_lamports: u64,
    pub retry_attempts: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub timeout_seconds: Option<u64>,
    pub max_connections: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub max_connections: Option<u32>,
    pub checkpoint_interval: Option<u64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            solana: SolanaConfig {
                rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
                keypair_path: PathBuf::from("/secrets/bob.json"),
                usdc_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                commitment: Some("confirmed".to_string()),
            },
            monero: MoneroConfig {
                wallet_rpc_url: "http://127.0.0.1:18083".to_string(),
                wallet_file: "bob_swap".to_string(),
                password_env: "MONERO_WALLET_PASSWORD".to_string(),
                daemon_url: None,
                daemon_username: None,
                daemon_password: None,
            },
            quoting: QuotingConfig {
                min_usdc: 100_000_000,  // 100 USDC
                max_usdc: 10_000_000_000,  // 10,000 USDC
                spread_bps: 50,
                expiry_minutes: Some(30),
            },
            relayer: RelayerConfig {
                enabled: true,
                fee_bps: 10,
                max_gas_lamports: 30_000,
                retry_attempts: Some(3),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                output: Some("stdout".to_string()),
            },
            server: ServerConfig {
                bind_address: "0.0.0.0:3000".to_string(),
                timeout_seconds: Some(30),
                max_connections: Some(100),
            },
            database: DatabaseConfig {
                path: PathBuf::from("./data/stealth-swap.db"),
                backup_path: Some(PathBuf::from("./data/backup")),
                max_connections: Some(10),
                checkpoint_interval: Some(300),
            },
        }
    }
}

impl AppConfig {
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.to_path_buf()));
        }

        let config_str = std::fs::read_to_string(path)?;
        let config: AppConfig = serde_yaml::from_str(&config_str)?;
        
        // Validate configuration
        config.validate()?;
        
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate Solana config
        if !self.solana.keypair_path.exists() {
            tracing::warn!("Solana keypair file not found at: {:?}", self.solana.keypair_path);
        }

        // Validate Monero config
        let password_env = std::env::var(&self.monero.password_env)
            .map_err(|_| ConfigError::MissingPasswordEnv(self.monero.password_env.clone()))?;
        if password_env.is_empty() {
            return Err(ConfigError::EmptyPasswordEnv(self.monero.password_env.clone()));
        }

        // Validate quoting parameters
        if self.quoting.min_usdc >= self.quoting.max_usdc {
            return Err(ConfigError::InvalidQuotingRange);
        }
        
        if self.quoting.spread_bps > 10000 {
            return Err(ConfigError::InvalidSpread(self.quoting.spread_bps));
        }

        // Validate relayer config
        if self.relayer.fee_bps > 10000 {
            return Err(ConfigError::InvalidFeeBps(self.relayer.fee_bps));
        }

        Ok(())
    }

    pub fn get_monero_password(&self) -> Result<SecretString, ConfigError> {
        let password = std::env::var(&self.monero.password_env)
            .map_err(|_| ConfigError::MissingPasswordEnv(self.monero.password_env.clone()))?;
        
        Ok(SecretString::new(password))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {0:?}")]
    FileNotFound(PathBuf),
    
    #[error("Failed to read configuration file: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Failed to parse YAML configuration: {0}")]
    YamlError(#[from] serde_yaml::Error),
    
    #[error("Missing environment variable: {0}")]
    MissingPasswordEnv(String),
    
    #[error("Empty password environment variable: {0}")]
    EmptyPasswordEnv(String),
    
    #[error("Invalid quoting range: min must be less than max")]
    InvalidQuotingRange,
    
    #[error("Invalid spread basis points: {0}")]
    InvalidSpread(u64),
    
    #[error("Invalid fee basis points: {0}")]
    InvalidFeeBps(u64),
}

pub fn load_config() -> Result<AppConfig, ConfigError> {
    let config_path = std::env::var("STEALTH_SWAP_CONFIG")
        .unwrap_or_else(|_| "./config.yaml".to_string());
    
    let path = std::path::Path::new(&config_path);
    
    if path.exists() {
        AppConfig::load_from_file(path)
    } else {
        tracing::warn!("Configuration file not found, using defaults");
        Ok(AppConfig::default())
    }
}