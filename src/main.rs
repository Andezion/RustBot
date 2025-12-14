mod client;
mod types;
mod dispatch;

use std::env;
use tokio::time::{sleep, Duration};
use client::Client;
use dispatch::Dispatcher;
use types::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("TELEGRAM_BOT_TOKEN")
        .expect("Please set the TELEGRAM_BOT_TOKEN environment variable");

    let client = Client::new(token);
    let mut offset: i64 = 0;

    let mut disp = Dispatcher::new();

    disp.add_command("start", |client: Client, msg: Message| async move {
        let params = serde_json::json!({"chat_id": msg.chat.id, "text": "Привет! Я диспетчерный бот."});
        let _ : Result<serde_json::Value, _> = client.send("sendMessage", &params).await;
    });

    println!("Starting polling bot with dispatcher...");

    loop {
        match client.get_updates(offset, 30).await {
            Ok(updates) => {
                for u in updates {
                    offset = u.update_id + 1;
                    if let Some(msg) = u.message {
                        let text_preview = msg.text.clone().unwrap_or_default();
                        println!("Message from {}: {}", msg.chat.id, text_preview);
                        disp.dispatch(client.clone(), msg).await;
                    }
                }
            }
            Err(e) => {
                eprintln!("poll error: {}", e);
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
}
