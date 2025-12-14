use reqwest::Client as HttpClient;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::types::{ApiResponse, Update};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("api error: {0}")]
    Api(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone)]
pub struct Client {
    pub base: String,
    pub http: HttpClient,
}

impl Client {
    pub fn new(token: impl Into<String>) -> Self {
        let token = token.into();
        let base = format!("https://api.telegram.org/bot{}", token);
        Self { base, http: HttpClient::new() }
    }

    pub async fn send<R: DeserializeOwned, P: Serialize>(&self, method: &str, params: &P) -> Result<R, BotError> {
        let url = format!("{}/{}", self.base, method);
        let resp = self.http.post(&url).json(params).send().await?;
        let text = resp.text().await?;
        let api: ApiResponse<R> = serde_json::from_str(&text)?;
        if api.ok {
            Ok(api.result)
        } else {
            Err(BotError::Api(api.description.unwrap_or_else(|| "telegram api error".into())))
        }
    }

    pub async fn get_updates(&self, offset: i64, timeout: u64) -> Result<Vec<Update>, BotError> {
        let params = serde_json::json!({"offset": offset, "timeout": timeout});
        let updates: Vec<Update> = self.send("getUpdates", &params).await?;
        Ok(updates)
    }
}
