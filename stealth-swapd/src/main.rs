mod config;
mod clients;
mod swap_engine;
mod api;
mod metrics;
mod security;

use std::sync::Arc;
use anyhow::Result;
use clap::Parser;
use tracing::{info, error};
use tracing_subscriber;

use config::load_config;
use clients::{SolanaClient, MoneroClient};
use swap_engine::SwapEngine;
use metrics::MetricsCollector;

#[derive(Parser)]
#[command(
    name = "stealth-swapd",
    version = "1.0.0",
    about = "Stealth-Swap backend daemon for Solana-XMR atomic swaps"
)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, env = "STEALTH_SWAP_CONFIG")]
    config: Option<std::path::PathBuf>,

    /// Run database migrations only
    #[arg(long)]
    migrate_only: bool,

    /// Print configuration and exit
    #[arg(long)]
    print_config: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "stealth_swap=info".into())
        )
        .init();

    info!("Starting stealth-swapd");

    let args = Args::parse();
    
    // Load configuration
    let config = load_config().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    if args.print_config {
        println!("{}", serde_yaml::to_string(&config)?);
        return Ok(());
    }

    info!("Configuration loaded successfully");

    // Initialize metrics
    let metrics = MetricsCollector::new();

    // Initialize clients
    info!("Initializing Solana client...");
    let solana_client = SolanaClient::new(&config.solana).await?;
    
    info!("Initializing Monero client...");
    let monero_password = config.get_monero_password()?;
    let monero_client = MoneroClient::new(&config.monero, monero_password).await?;

    // Initialize database
    info!("Initializing database...");
    let _db = init_database(&config.database).await?;

    if args.migrate_only {
        info!("Database migrations completed successfully");
        return Ok(());
    }

    // Initialize swap engine
    info!("Initializing swap engine...");
    let swap_engine = SwapEngine::new(
        config.clone(),
        solana_client,
        monero_client,
        metrics.clone(),
    ).await?;

    info!("Swap engine initialized successfully");

    // Start background tasks
    let swap_engine_handle = {
        let swap_engine = swap_engine.clone();
        tokio::spawn(async move {
            if let Err(e) = swap_engine.run().await {
                error!("Swap engine error: {}", e);
            }
        })
    };

    // Start HTTP server
    let server_handle = {
        let config = config.clone();
        let swap_engine = swap_engine.clone();
        let metrics = Arc::new(metrics.clone());
        
        tokio::spawn(async move {
            info!("Starting HTTP server on {}", config.server.bind_address);
            if let Err(e) = api::start_server(
                config.server.bind_address,
                swap_engine,
                metrics,
            ).await {
                error!("HTTP server error: {}", e);
            }
        })
    };

    // Wait for shutdown signal
    shutdown_signal().await;

    info!("Shutting down stealth-swapd...");

    // Gracefully shutdown
    swap_engine_handle.abort();
    server_handle.abort();

    info!("Gracefully shutdown completed");
    Ok(())
}

async fn init_database(config: &config::DatabaseConfig) -> Result<sqlx::SqlitePool> {
    use sqlx::sqlite::SqlitePoolOptions;

    // Ensure data directory exists
    if let Some(parent) = config.path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Create pool
    let pool = SqlitePoolOptions::new()
        .max_connections(config.max_connections.unwrap_or(10))
        .connect(&format!("sqlite://{}?mode=rwc", config.path.display()))
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C signal"),
        _ = terminate => info!("Received SIGTERM signal"),
    }
}