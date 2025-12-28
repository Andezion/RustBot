use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use tokio::time::{sleep, Duration};
use crate::client::Client;
use crate::dispatch::Dispatcher;
use std::path::PathBuf;
use tokio::fs as tokio_fs;
use chrono::Utc;

type KvStore = Arc<RwLock<HashMap<String, String>>>;
type Users = Arc<RwLock<HashSet<i64>>>;
type Counters = Arc<RwLock<HashMap<String, u64>>>;

const DATA_DIR: &str = "data";
const KV_FILE: &str = "data/kv.json";
const USERS_FILE: &str = "data/users.json";
const AUTOSAVE_INTERVAL_SECS: u64 = 30;
const COOLDOWN_SECONDS: u64 = 2;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();
        let token = match env::var("TELEGRAM_BOT_TOKEN") {
            Ok(t) if t.contains(':') && t.len() > 10 => t,
            Ok(_) => {
                eprintln!("TELEGRAM_BOT_TOKEN looks invalid (expected token like 123:ABC). Please check env.");
                return Err("invalid TELEGRAM_BOT_TOKEN".into());
            }
            Err(_) => {
                eprintln!("Missing TELEGRAM_BOT_TOKEN environment variable. Set it and restart.");
                return Err("missing TELEGRAM_BOT_TOKEN".into());
            }
        };

    let admin_id: Option<i64> = env::var("ADMIN_ID").ok().and_then(|s| s.parse().ok());
    let admin = admin_id;

    let cooldown_seconds: u64 = env::var("COOLDOWN_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(COOLDOWN_SECONDS);

    let client = Client::builder(token).build();
    let mut offset: i64 = 0;

    let kv_map = if let Ok(b) = tokio_fs::read(KV_FILE).await {
        match serde_json::from_slice::<HashMap<String, String>>(&b) {
            Ok(m) => m,
            Err(_) => HashMap::new(),
        }
    } else { HashMap::new() };
    let users_set = if let Ok(b) = tokio_fs::read(USERS_FILE).await {
        match serde_json::from_slice::<HashSet<i64>>(&b) {
            Ok(s) => s,
            Err(_) => HashSet::new(),
        }
    } else { HashSet::new() };

    let kv: KvStore = Arc::new(RwLock::new(kv_map));
    let users: Users = Arc::new(RwLock::new(users_set));
    let counters: Counters = Arc::new(RwLock::new(HashMap::new()));

    let cooldowns: Arc<RwLock<HashMap<i64, u64>>> = Arc::new(RwLock::new(HashMap::new()));

    let mut disp = Dispatcher::new();

    let keyboard_markup = crate::types::ReplyMarkup::ReplyKeyboard(crate::types::ReplyKeyboardMarkup {
        keyboard: vec![
            vec![crate::types::KeyboardButton { text: "/help".to_string(), request_contact: None, request_location: None }, crate::types::KeyboardButton { text: "/ping".to_string(), request_contact: None, request_location: None }],
            vec![crate::types::KeyboardButton { text: "/whoami".to_string(), request_contact: None, request_location: None }],
        ],
        one_time_keyboard: Some(true),
        resize_keyboard: None,
    });

    let kb_help = keyboard_markup.clone();
    let kb_start = crate::types::ReplyMarkup::ReplyKeyboard(crate::types::ReplyKeyboardMarkup {
        keyboard: vec![
            vec![crate::types::KeyboardButton { text: "Share contact".to_string(), request_contact: Some(true), request_location: None }, crate::types::KeyboardButton { text: "Share location".to_string(), request_contact: None, request_location: Some(true) }]
        ],
        one_time_keyboard: Some(true),
        resize_keyboard: None,
    });
    let kb_keyboard = keyboard_markup.clone();

    crate::commands::register(&mut disp, admin, kv.clone(), users.clone(), counters.clone(), kb_help, kb_start, kb_keyboard);

    let _ = tokio_fs::create_dir_all(DATA_DIR).await;

    {
        let kv_s = kv.clone();
        let users_s = users.clone();
            let autosave_interval_secs: u64 = env::var("AUTOSAVE_INTERVAL_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(AUTOSAVE_INTERVAL_SECS);
            tokio::spawn(async move {
                loop {
                    sleep(Duration::from_secs(autosave_interval_secs)).await;
                    let kv_json = serde_json::to_vec(&*kv_s.read().await).unwrap_or_default();
                    let _ = tokio_fs::write(KV_FILE, kv_json).await;
                    let users_json = serde_json::to_vec(&*users_s.read().await).unwrap_or_default();
                    let _ = tokio_fs::write(USERS_FILE, users_json).await;
                }
            });
    }

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

                                        let uid = msg.chat.id;
                                        let now = chrono::Utc::now().timestamp() as u64;
                                        let mut cds = cooldowns.write().await;
                                            if let Some(last) = cds.get(&uid) {
                                                if now.saturating_sub(*last) < cooldown_seconds {
                                                    let _ = client.send_message(uid, "Please wait a moment before sending another command.", None).await;
                                                    continue;
                                                }
                                            }
                                            cds.insert(uid, now);
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
