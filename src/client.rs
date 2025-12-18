use reqwest::Client as HttpClient;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::types::{ApiResponse, Update, File, UserProfilePhotos};
use thiserror::Error;
use reqwest::multipart::{Form, Part};
use tokio::fs;
use std::path::Path;

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
        let resp = self.http.post(&url).json(params).send().await?;
        let text = resp.text().await?;
        let api: ApiResponse<serde_json::Value> = serde_json::from_str(&text)?;
        if api.ok {
            Ok(api.result)
        } else {
            Err(BotError::Api(api.description.unwrap_or_else(|| "telegram api error".into())))
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
