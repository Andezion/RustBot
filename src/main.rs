mod client;
mod types;
mod dispatch;
mod commands;

use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use tokio::time::{sleep, Duration};
use client::{Client};
use dispatch::Dispatcher;

type KvStore = Arc<RwLock<HashMap<String, String>>>;
type Users = Arc<RwLock<HashSet<i64>>>;
type Counters = Arc<RwLock<HashMap<String, u64>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();
    let token = env::var("TELEGRAM_BOT_TOKEN")
        .expect("Please set the TELEGRAM_BOT_TOKEN environment variable");

    let admin_id: Option<i64> = env::var("ADMIN_ID").ok().and_then(|s| s.parse().ok());
    let admin = admin_id; 

    let client = Client::builder(token).build();
    let mut offset: i64 = 0;

    let kv: KvStore = Arc::new(RwLock::new(HashMap::new()));
    let users: Users = Arc::new(RwLock::new(HashSet::new()));
    let counters: Counters = Arc::new(RwLock::new(HashMap::new()));

    let mut disp = Dispatcher::new();

    let keyboard_markup = types::ReplyMarkup::ReplyKeyboard(types::ReplyKeyboardMarkup {
        keyboard: vec![
            vec![types::KeyboardButton { text: "/help".to_string(), request_contact: None, request_location: None }, types::KeyboardButton { text: "/ping".to_string(), request_contact: None, request_location: None }],
            vec![types::KeyboardButton { text: "/whoami".to_string(), request_contact: None, request_location: None }],
        ],
        one_time_keyboard: Some(true),
        resize_keyboard: None,
    });

    let kb_help = keyboard_markup.clone();
    let kb_start = types::ReplyMarkup::ReplyKeyboard(types::ReplyKeyboardMarkup {
        keyboard: vec![
            vec![types::KeyboardButton { text: "Share contact".to_string(), request_contact: Some(true), request_location: None }, types::KeyboardButton { text: "Share location".to_string(), request_contact: None, request_location: Some(true) }]
        ],
        one_time_keyboard: Some(true),
        resize_keyboard: None,
    });
    let kb_keyboard = keyboard_markup.clone();

    commands::register(&mut disp, admin, kv.clone(), users.clone(), counters.clone(), kb_help, kb_start, kb_keyboard);

    tracing::info!("Starting polling bot with dispatcher... (press Ctrl+C to stop)");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("Shutdown signal received, stopping polling...");
                if let Some(aid) = admin {
                    let _ = client.send_message(aid, "Bot is shutting down", None).await;
                }
                break;
            }
            res = client.get_updates(offset, 30) => {
                match res {
                    Ok(updates) => {
                        for u in updates {
                            offset = u.update_id + 1;
                            
                            if let Some(msg) = &u.message {
                                let mut us = users.write().await;
                                us.insert(msg.chat.id);
                            }
                            
                            if let Some(msg) = u.message {
                                
                                if let Some(text) = &msg.text {
                                    if text.starts_with('/') {
                                        let cmd = text.split_whitespace().next().unwrap_or("").trim_start_matches('/').to_string();
                                        let mut ctr = counters.write().await;
                                        *ctr.entry(cmd).or_insert(0) += 1;
                                    }
                                }
                                tracing::info!("Message from {}: {}", msg.chat.id, msg.text.clone().unwrap_or_default());

                                if let Some(aid) = admin {
                                    if let Some(contact) = &msg.contact {
                                        let mut body = format!("Contact from chat {}:\nphone: {}\nfirst_name: {}\n", msg.chat.id, contact.phone_number, contact.first_name);
                                        if let Some(last) = &contact.last_name { body.push_str(&format!("last_name: {}\n", last)); }
                                        if let Some(uid) = contact.user_id { body.push_str(&format!("user_id: {}\n", uid)); }
                                        let _ = client.send_message(aid, &body, None).await;
                                    }
                                    if let Some(loc) = &msg.location {
                                        let body = format!("Location from chat {}:\nlat: {}\nlon: {}\n", msg.chat.id, loc.latitude, loc.longitude);
                                        let _ = client.send_message(aid, &body, None).await;
                                    }
                                }

                                disp.dispatch(client.clone(), msg).await;
                            }
                            
                            if let Some(cb) = u.callback_query {
                                disp.dispatch_callback(client.clone(), cb).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("poll error: {}", e);
                        sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        }
    }

    Ok(())
}
