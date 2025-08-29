use crate::config::MoneroConfig;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::time::Duration;
use secrecy::{SecretString, ExposeSecret};

#[derive(Clone)]
pub struct MoneroClient {
    rpc_url: String,
    wallet_name: String,
    password: SecretString,
    http_client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoneroBalance {
    pub unlocked: u64,
    pub locked: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcResponse<T> {
    id: String,
    result: Option<T>,
    error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

impl MoneroClient {
    pub async fn new(
        config: &MoneroConfig,
        password: SecretString,
    ) -> Result<Self> {
        let client = Self {
            rpc_url: config.wallet_rpc_url.clone(),
            wallet_name: config.wallet_file.clone(),
            password,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?
        };
        
        // Test connection
        client.health_check().await?;
        
        Ok(client)
    }

    pub async fn health_check(&self) -> Result<bool> {
        let response: serde_json::Value = 
            self.call_rpc("get_version", serde_json::json!([])).await?;
        
        match response.get("error") {
            Some(_) => Ok(false),
            None => Ok(true),
        }
    }

    pub async fn get_height(&self) -> Result<u64> {
        let response: serde_json::Value = 
            self.call_rpc("get_height", serde_json::json!([])).await?;
            
        let result = response["result"]["height"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Failed to get height"))?;
            
        Ok(result)
    }

    pub async fn get_balance(&self) -> Result<MoneroBalance> {
        let response: serde_json::Value = 
            self.call_rpc("get_balance", serde_json::json!({"account_index": 0})).await?;
            
        let balance = &response["result"];
        
        Ok(MoneroBalance {
            unlocked: balance["unlocked_balance"]
                .as_str()
                .unwrap_or("0")
                .parse::<u64>()
                .unwrap_or(0),
            locked: balance["locked_balance"]
                .as_str()
                .unwrap_or("0")
                .parse::<u64>()
                .unwrap_or(0),
            total: balance["balance"]
                .as_str()
                .unwrap_or("0")
                .parse::<u64>()
                .unwrap_or(0),
        })
    }

    pub async fn create_subaddress(&self, label: &str) -> Result<(String, [u8; 64])> {
        let params = serde_json::json!({
            "account_index": 0,
            "label": label
        });
        
        let response: serde_json::Value = 
            self.call_rpc("create_address", params).await?;
            
        let address = response["result"]["address"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to create subaddress"))?.to_string();
        
        // Convert address to 64-byte array
        let mut bytes = [0u8; 64];
        let address_bytes = address.as_bytes();
        let len = std::cmp::min(address_bytes.len(), 64);
        bytes[..len].copy_from_slice(&address_bytes[..len]);
        
        Ok((address, bytes))
    }

    pub async fn send_transfer(
        &self,
        destination: &str,
        amount: u64,
    ) -> Result<String> {
        let params = serde_json::json!({
            "destinations": [{
                "address": destination,
                "amount": amount.to_string()
            }],
            "account_index": 0,
            "priority": 1, // Normal priority
            "get_tx_key": true,
            "unlock_time": 0
        });
        
        let response: serde_json::Value = 
            self.call_rpc("transfer", params).await?;
            
        let tx_hash = response["result"]["tx_hash"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to get transaction hash"))?.to_string();
            
        Ok(tx_hash)
    }

    pub async fn validate_address(&self, address: &str) -> Result<bool> {
        let params = serde_json::json!({
            "address": address
        });

        let response: serde_json::Value = 
            self.call_rpc("validate_address", params).await?;

        let valid = response["result"]["valid"].as_bool().unwrap_or(false);
        Ok(valid)
    }

    pub async fn get_transfers(&self, txid: &str) -> Result<Option<serde_json::Value>> {
        let params = serde_json::json!({
            "txid": txid
        });
        
        let response: serde_json::Value = 
            self.call_rpc("get_transfer_by_txid", params).await?;
        
        if response["result"].is_null() {
            Ok(None)
        } else {
            Ok(Some(response["result"].clone()))
        }
    }

    pub async fn open_wallet(&self) -> Result<()> {
        let params = serde_json::json!({
            "filename": self.wallet_name,
            "password": self.password.expose_secret()
        });
        
        self.call_rpc("open_wallet", params).await?;
        Ok(())
    }

    pub async fn close_wallet(&self) -> Result<()> {
        self.call_rpc("close_wallet", serde_json::json!([])).await?;
        Ok(())
    }

    async fn call_rpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            method: method.to_string(),
            params,
        };

        let response = self.http_client
            .post(&self.rpc_url)
            .json(&request)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let response_json: serde_json::Value = response.json().await?;
        
        if let Some(error) = response_json.get("error") {
            let code = error["code"].as_i64().unwrap_or(-1);
            let message = error["message"].as_str().unwrap_or("Unknown error");
            return Err(anyhow::anyhow!("Monero RPC error {}: {}", code, message));
        }

        Ok(response_json)
    }
}