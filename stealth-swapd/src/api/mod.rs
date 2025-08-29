use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use std::error::Error;
use serde::{Deserialize, Serialize};
use crate::swap_engine::{SwapEngine, QuoteRequest, Direction};
use crate::metrics::MetricsCollector;

use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use hex;

#[derive(Deserialize)]
struct QuoteRequestBody {
    direction: String,
    usdc_amount: u64,
    xmr_amount: u64,
}

#[derive(Deserialize)]
struct AcceptRequestBody {
    quote_id: String,
    counterparty_pubkey: Option<String>,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Serialize)]
struct HealthResponse {
    solana_connected: bool,
    monero_connected: bool,
    last_block_height: u64,
}

#[derive(Serialize)]
struct SwapStatusResponse {
    state: String,
    usdc_amount: u64,
    xmr_amount: u64,
    expiry: String,
    failure_reason: Option<String>,
}

pub struct AppState {
    swap_engine: SwapEngine,
    metrics: Arc<MetricsCollector>,
}

pub fn create_app(swap_engine: SwapEngine, metrics: Arc<MetricsCollector>) -> Router {
    let state = Arc::new(AppState {
        swap_engine,
        metrics,
    });

    Router::new()
        .route("/v1/quote", post(generate_quote))
        .route("/v1/swap/accept", post(accept_swap))
        .route("/v1/swap/:swap_id", get(get_swap_status))
        .route("/health", get(health_check))
        .route("/metrics", get(get_metrics))
        .with_state(state)
}

pub async fn start_server(addr: String, swap_engine: SwapEngine, metrics: Arc<MetricsCollector>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let app = create_app(swap_engine, metrics);
    let addr: std::net::SocketAddr = addr.parse()?;
    axum::Server::bind(&addr).serve(app.into_make_service()).await?;
    Ok(())
}

async fn generate_quote(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QuoteRequestBody>,
) -> Result<Json<ApiResponse<crate::swap_engine::QuoteResponse>>, StatusCode> {
    let direction = match payload.direction.as_str() {
        "usdc_to_xmr" => Direction::UsdcToXmr,
        "xmr_to_usdc" => Direction::XmrToUsdc,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let request = QuoteRequest {
        direction,
        usdc_amount: payload.usdc_amount,
        xmr_amount: payload.xmr_amount,
    };

    match state.swap_engine.generate_quote(request).await {
        Ok(quote) => Ok(Json(ApiResponse {
            success: true,
            data: Some(quote),
            error: None,
        })),
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn accept_swap(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AcceptRequestBody>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let quote_id = match Uuid::parse_str(&payload.quote_id) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    match state.swap_engine.accept_swap(quote_id, payload.counterparty_pubkey).await {
        Ok(swap_id) => Ok(Json(ApiResponse {
            success: true,
            data: Some(hex::encode(swap_id)),
            error: None,
        })),
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn get_swap_status(
    State(state): State<Arc<AppState>>,
    Path(swap_id): Path<String>,
) -> Result<Json<ApiResponse<SwapStatusResponse>>, StatusCode> {
    let swap_id_bytes = match hex::decode(swap_id).ok().and_then(|v| v.try_into().ok()) {
        Some(bytes) => bytes,
        None => return Err(StatusCode::BAD_REQUEST),
    };

    match state.swap_engine.get_swap_status(swap_id_bytes).await {
        Some(swap) => {
            let status = SwapStatusResponse {
                state: format!("{:?}", swap.state).to_lowercase(),
                usdc_amount: swap.usdc_amount,
                xmr_amount: swap.xmr_amount,
                expiry: swap.expires_at.to_rfc3339(),
                failure_reason: swap.failure_reason.clone(),
            };
            
            Ok(Json(ApiResponse {
                success: true,
                data: Some(status),
                error: None,
            }))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn health_check(
    State(_state): State<Arc<AppState>>,
) -> Json<ApiResponse<HealthResponse>> {
    let (solana_connected, monero_connected, height) = (true, true, 1234567);
    
    Json(ApiResponse {
        success: true,
        data: Some(HealthResponse {
            solana_connected,
            monero_connected,
            last_block_height: height,
        }),
        error: None,
    })
}

async fn get_metrics(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<HashMap<String, u64>>> {
    let metrics = state.metrics.get_metrics();
    Json(ApiResponse {
        success: true,
        data: Some(metrics),
        error: None,
    })
}

async fn _check_health(_engine: &SwapEngine) -> (bool, bool, u64) {
    // Mock health check
    (true, true, 1234567)
}