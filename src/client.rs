use reqwest::Client as HttpClient;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::types::{ApiResponse, Update, File, UserProfilePhotos};
use thiserror::Error;
use reqwest::multipart::{Form, Part};
use tokio::fs;
use std::path::Path;
use tokio::time::{sleep, Duration};
use tracing::{warn, info};

#[derive(Error, Debug)]
pub enum BotError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("api error: {0}")]
    Api(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
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

    pub async fn send_raw<P: Serialize>(&self, method: &str, params: &P) -> Result<serde_json::Value, BotError> {
        let url = format!("{}/{}", self.base, method);
        let mut attempt: u32 = 0;
        let max_attempts: u32 = 5;
        let mut backoff = Duration::from_millis(500);

        loop {
            attempt += 1;
            let resp = self.http.post(&url).json(params).send().await?;
            let status = resp.status();
            let text = resp.text().await?;

            let api: Result<ApiResponse<serde_json::Value>, serde_json::Error> = serde_json::from_str(&text);

            if status.as_u16() == 429 {
                let retry_after = resp.headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok());
                if let Some(secs) = retry_after {
                    warn!("received 429, retry after {}s (header)", secs);
                    sleep(Duration::from_secs(secs)).await;
                } else {
                    warn!("received 429, backing off {}ms", backoff.as_millis());
                    sleep(backoff).await;
                    backoff = backoff.checked_mul(2).unwrap_or(backoff);
                }
                if attempt >= max_attempts { return Err(BotError::Api("too many requests (429)".into())); }
                continue;
            }

            if status.is_server_error() {
                warn!("server error status {} on attempt {}", status, attempt);
                if attempt >= max_attempts { return Err(BotError::Api(format!("server error: {}", status))); }
                sleep(backoff).await;
                backoff = backoff.checked_mul(2).unwrap_or(backoff);
                continue;
            }

            let api = match api {
                Ok(a) => a,
                Err(e) => return Err(BotError::Json(e)),
            };

            if api.ok {
                return Ok(api.result);
            }

            if let Some(desc) = api.description {
                if let Some(secs) = parse_retry_after_from_description(&desc) {
                    warn!("telegram responded with retry_after={}s in description", secs);
                    sleep(Duration::from_secs(secs)).await;
                    if attempt >= max_attempts { return Err(BotError::Api(desc)); }
                    continue;
                }
                return Err(BotError::Api(desc));
            } else {
                return Err(BotError::Api("telegram api error".into()));
            }
        }
    }

    pub async fn send_message(&self, chat_id: i64, text: &str, reply_markup: Option<serde_json::Value>) -> Result<serde_json::Value, BotError> {
        let mut params = serde_json::json!({"chat_id": chat_id, "text": text});
        if let Some(rm) = reply_markup {
            params["reply_markup"] = rm;
        }
        self.send_raw("sendMessage", &params).await
    }

    pub async fn send_document_path(&self, chat_id: i64, path: &str) -> Result<serde_json::Value, BotError> {
        let url = format!("{}/sendDocument", self.base);
        let data = fs::read(path).await?;
        let filename = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();
        let part = Part::bytes(data).file_name(filename);
        let form = Form::new().text("chat_id", chat_id.to_string()).part("document", part);
        let resp = self.http.post(&url).multipart(form).send().await?;
        let text = resp.text().await?;
        let api: ApiResponse<serde_json::Value> = serde_json::from_str(&text)?;
        if api.ok { Ok(api.result) } else { Err(BotError::Api(api.description.unwrap_or_else(|| "telegram api error".into()))) }
    }

    pub async fn send<R: DeserializeOwned, P: Serialize>(&self, method: &str, params: &P) -> Result<R, BotError> {
        let url = format!("{}/{}", self.base, method);
        let mut attempt: u32 = 0;
        let max_attempts: u32 = 5;
        let mut backoff = Duration::from_millis(500);

        loop {
            attempt += 1;
            let resp = self.http.post(&url).json(params).send().await?;
            let status = resp.status();
            let text = resp.text().await?;

            if status.as_u16() == 429 {
                let retry_after = resp.headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok());
                if let Some(secs) = retry_after {
                    warn!("received 429, retry after {}s (header)", secs);
                    sleep(Duration::from_secs(secs)).await;
                } else {
                    warn!("received 429, backing off {}ms", backoff.as_millis());
                    sleep(backoff).await;
                    backoff = backoff.checked_mul(2).unwrap_or(backoff);
                }
                if attempt >= max_attempts { return Err(BotError::Api("too many requests (429)".into())); }
                continue;
            }

            if status.is_server_error() {
                warn!("server error status {} on attempt {}", status, attempt);
                if attempt >= max_attempts { return Err(BotError::Api(format!("server error: {}", status))); }
                sleep(backoff).await;
                backoff = backoff.checked_mul(2).unwrap_or(backoff);
                continue;
            }

            let api: ApiResponse<R> = serde_json::from_str(&text)?;
            if api.ok { return Ok(api.result); }
            if let Some(desc) = api.description {
                if let Some(secs) = parse_retry_after_from_description(&desc) {
                    warn!("telegram responded with retry_after={}s in description", secs);
                    sleep(Duration::from_secs(secs)).await;
                    if attempt >= max_attempts { return Err(BotError::Api(desc)); }
                    continue;
                }
                return Err(BotError::Api(desc));
            }
            return Err(BotError::Api("telegram api error".into()));
        }
    }

    pub async fn get_updates(&self, offset: i64, timeout: u64) -> Result<Vec<Update>, BotError> {
        let params = serde_json::json!({"offset": offset, "timeout": timeout});
        let updates: Vec<Update> = self.send("getUpdates", &params).await?;
        Ok(updates)
    }

    pub async fn get_chat(&self, chat_id: i64) -> Result<serde_json::Value, BotError> {
        let params = serde_json::json!({"chat_id": chat_id});
        let v: serde_json::Value = self.send("getChat", &params).await?;
        Ok(v)
    }

    pub async fn get_user_profile_photos(&self, user_id: i64) -> Result<UserProfilePhotos, BotError> {
        let params = serde_json::json!({"user_id": user_id});
        let uph: UserProfilePhotos = self.send("getUserProfilePhotos", &params).await?;
        Ok(uph)
    }

    pub async fn get_file(&self, file_id: &str) -> Result<File, BotError> {
        let params = serde_json::json!({"file_id": file_id});
        let f: File = self.send("getFile", &params).await?;
        Ok(f)
    }

    pub async fn download_file_bytes(&self, file_path: &str) -> Result<Vec<u8>, BotError> {
        let file_base = self.base.replacen("/bot", "/file/bot", 1);
        let url = format!("{}/{}", file_base, file_path);
        let resp = self.http.get(&url).send().await?;
        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }
}

fn parse_retry_after_from_description(desc: &str) -> Option<u64> {
    let s = desc.to_lowercase();
    if let Some(pos) = s.find("retry after") {
        let tail = &s[pos + "retry after".len()..];
        for tok in tail.split(|c: char| !c.is_digit(10)) {
            if tok.is_empty() { continue; }
            if let Ok(n) = tok.parse::<u64>() {
                return Some(n);
            }
        }
    }
    None
}
