//! HTTP RPC client for integration tests

use reqwest::Client;
use serde::de::DeserializeOwned;

pub struct RpcClient {
    client: Client,
    base_url: String,
}

impl RpcClient {
    pub fn new(port: u16) -> Self {
        Self {
            client: Client::new(),
            base_url: format!("http://127.0.0.1:{}", port),
        }
    }

    pub async fn health(&self) -> Result<HealthResponse, String> {
        self.get("/health").await
    }

    pub async fn height(&self) -> Result<u64, String> {
        let resp: HeightResponse = self.get("/height").await?;
        Ok(resp.height)
    }

    pub async fn block(&self, height: u64) -> Result<BlockResponse, String> {
        self.get(&format!("/block/{}", height)).await
    }

    pub async fn latest(&self) -> Result<BlockResponse, String> {
        self.get("/latest").await
    }

    pub async fn submit_message(
        &self,
        sender: &str,
        content: &str,
    ) -> Result<String, String> {
        let req = SubmitMessageRequest {
            sender: sender.to_string(),
            content: content.to_string(),
        };
        let resp: SubmitMessageResponse = self.post("/message", &req).await?;
        Ok(resp.message_id)
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}{}", self.base_url, path);
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?
            .json::<T>()
            .await
            .map_err(|e| format!("JSON error: {}", e))
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let url = format!("{}{}", self.base_url, path);
        self.client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?
            .json::<T>()
            .await
            .map_err(|e| format!("JSON error: {}", e))
    }
}

// Response types mirror RPC server types
#[derive(serde::Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub height: u64,
}

#[derive(serde::Deserialize)]
pub struct HeightResponse {
    pub height: u64,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockResponse {
    pub height: u64,
    pub timestamp: u64,
    pub prev_hash: String,
    pub message_count: usize,
    pub hash: String,
}

#[derive(serde::Serialize)]
pub struct SubmitMessageRequest {
    pub sender: String,
    pub content: String,
}

#[derive(serde::Deserialize)]
pub struct SubmitMessageResponse {
    pub message_id: String,
}
