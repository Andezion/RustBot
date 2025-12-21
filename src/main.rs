mod client;
mod types;
mod dispatch;

use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use tokio::time::{sleep, Duration};
use client::{Client};
use dispatch::Dispatcher;
use types::Message;

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

    let client = Client::new(token);
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

    disp.add_command("help", move |client: Client, msg: Message| {
        let admin = admin;
        let kb = kb_help.clone();
        async move {
            let mut help = String::from("Available commands:\n");
            help.push_str("/start - start and register\n");
            help.push_str("/help - this message\n");
            help.push_str("/ping - pong\n");
            help.push_str("/echo <text> - echo back text\n");
            help.push_str("/whoami - show your id and username\n");
            help.push_str("/keyboard - show custom keyboard\n");
            help.push_str("/inline - show inline buttons example\n");
            help.push_str("/set <k> <v> - save key/value (in-memory)\n");
            help.push_str("/get <k> - get saved value\n");
            help.push_str("/broadcast <text> - send to all users (admin only)\n");
            help.push_str("/upload - upload README.md\n");
            help.push_str("/stats - show simple stats\n");
            if admin.is_some() {
                help.push_str("\nAdmin commands are enabled.\n");
            } else {
                help.push_str("\nNote: ADMIN_ID not set. Some commands require ADMIN_ID.\n");
            }
            let rm = serde_json::to_value(&kb).ok();
            client.send_message(msg.chat.id, &help, rm).await?;
            Ok(())
        }
    });

    let users_for_start = users.clone();
    let admin_for_start = admin; 
    disp.add_command("start", move |client: Client, msg: Message| {
        let users = users_for_start.clone();
        let kb = kb_start.clone();
        let admin = admin_for_start;
        async move {
            let mut us = users.write().await;
            us.insert(msg.chat.id);
            let name = msg.from.as_ref().map(|u| u.first_name.clone()).unwrap_or_else(|| "there".to_string());
            let welcome = format!("Hello, {}! Welcome. Type /help to see available commands.", name);
            let rm = serde_json::to_value(&kb).ok();
            client.send_message(msg.chat.id, &welcome, rm).await?;

            let mut info = String::new();
            info.push_str("New /start received:\n\n");
            info.push_str(&format!("chat: {:?}\n", msg.chat));
            info.push_str(&format!("from: {:?}\n", msg.from));
            info.push_str(&format!("message_id: {}\n", msg.message_id));
            info.push_str(&format!("text: {}\n\n", msg.text.clone().unwrap_or_default()));
            if let Ok(js) = serde_json::to_string_pretty(&msg) {
                info.push_str("raw_json:\n");
                info.push_str(&js);
                info.push_str("\n");
            }

            if let Some(aid) = admin {
                let _ = if info.len() < 3500 {
                    client.send_message(aid, &info, None).await
                } else {
                    let mut path = std::env::temp_dir();
                    let fname = format!("start_info_{}.json", msg.chat.id);
                    path.push(fname);
                    let path_str = path.to_string_lossy().to_string();
                    let _ = tokio::fs::write(&path_str, info.as_bytes()).await;
                    client.send_document_path(aid, &path_str).await
                };
            }
            Ok(())
        }
    });

    disp.add_command("ping", |client: Client, msg: Message| async move {
        client.send_message(msg.chat.id, "pong", None).await?;
        Ok(())
    });

    disp.add_command("echo", |client: Client, msg: Message| async move {
        if let Some(text) = msg.text {
            let parts: Vec<&str> = text.splitn(2, ' ').collect();
            let resp = if parts.len() > 1 { parts[1].to_string() } else { "".to_string() };
            client.send_message(msg.chat.id, &resp, None).await?;
        }
        Ok(())
    });

    disp.add_command("whoami", |client: Client, msg: Message| async move {
        let user = msg.from;
        if let Some(u) = user {
            let name = u.username.clone().unwrap_or_else(|| u.first_name.clone());
            let resp = format!("id: {}\nusername: {}", u.id, name);
            client.send_message(msg.chat.id, &resp, None).await?;
        }
        Ok(())
    });

    let admin_for_inspect = admin;
    disp.add_command("inspect", move |client: Client, msg: Message| {
        let admin = admin_for_inspect;
        async move {
            let allowed = msg.from.as_ref().map(|u| Some(u.id) == admin).unwrap_or(false);
            if !allowed {
                client.send_message(msg.chat.id, "not allowed", None).await?;
                return Ok(());
            }

            let target_id_opt = if let Some(text) = &msg.text {
                let parts: Vec<&str> = text.split_whitespace().collect();
                if parts.len() > 1 {
                    parts[1].parse::<i64>().ok()
                } else { None }
            } else { None };

            let target_id = target_id_opt
                .or_else(|| msg.from.as_ref().map(|u| u.id))
                .unwrap_or(msg.chat.id);

            let mut report = String::new();
            report.push_str(&format!("target_user_id: {}\n", target_id));

            if let Ok(chat_js) = client.get_chat(target_id).await {
                report.push_str(&format!("getChat: {}\n", serde_json::to_string_pretty(&chat_js).unwrap_or_default()));
            } else {
                report.push_str("getChat failed\n");
            }

            match client.get_user_profile_photos(target_id).await {
                Ok(uph) => {
                    report.push_str(&format!("profile_photos_total: {}\n", uph.total_count));
                    if uph.total_count > 0 {
                        if let Some(sizes) = uph.photos.get(0) {
                            if let Some(best) = sizes.last() {
                                report.push_str(&format!("chosen_photo_file_id: {}\n", best.file_id));
                                if let Ok(finfo) = client.get_file(&best.file_id).await {
                                    report.push_str(&format!("file_info: {:?}\n", finfo));
                                    if let Some(fp) = finfo.file_path {
                                        if let Ok(bytes) = client.download_file_bytes(&fp).await {
                                            report.push_str(&format!("downloaded_bytes: {}\n", bytes.len()));
                                            report.push_str("EXIF parsing disabled in this build.\n");
                                        } else {
                                            report.push_str("failed to download file bytes\n");
                                        }
                                    } else {
                                        report.push_str("file has no file_path (maybe not downloadable)\n");
                                    }
                                } else {
                                    report.push_str("getFile failed\n");
                                }
                            }
                        }
                    }
                }
                Err(_) => { report.push_str("failed to get profile photos\n"); }
            }

            client.send_message(msg.chat.id, &report, None).await?;
            Ok(())
        }
    });

    disp.add_command("keyboard", move |client: Client, msg: Message| {
        let rm = kb_keyboard.clone();
        async move {
            let rmv = serde_json::to_value(&rm).ok();
            client.send_message(msg.chat.id, "Choose:", rmv).await?;
            Ok(())
        }
    });

    disp.add_command("inline", |client: Client, msg: Message| async move {
        let inline = types::ReplyMarkup::InlineKeyboard(types::InlineKeyboardMarkup {
            inline_keyboard: vec![vec![types::InlineKeyboardButton { text: "Say hi".to_string(), callback_data: Some("echo Hello from button".to_string()), url: None }]]
        });
        let rm = serde_json::to_value(&inline).ok();
        client.send_message(msg.chat.id, "Inline example:", rm).await?;
        Ok(())
    });

    disp.add_callback(|client: Client, cb: types::CallbackQuery| async move {
        let _ = client.answer_callback_query(&cb.id, Some("Received"), Some(false), None, None).await;
        if let Some(msg) = cb.message {
            let chat_id = msg.chat.id;
            let d = cb.data.unwrap_or_else(|| "(no data)".to_string());
            client.send_message(chat_id, &format!("Button pressed: {}", d), None).await?;
        }
        Ok(())
    });

    let kv_set = kv.clone();
    disp.add_command("set", move |_client: Client, msg: Message| {
        let kv = kv_set.clone();
        async move {
            if let Some(text) = msg.text {
                let mut parts = text.splitn(3, ' ');
                parts.next(); 
                if let Some(k) = parts.next() {
                    if let Some(v) = parts.next() {
                        let mut map = kv.write().await;
                        map.insert(k.to_string(), v.to_string());
                    }
                }
            }
            Ok(())
        }
    });

    let kv_get = kv.clone();
    disp.add_command("get", move |client: Client, msg: Message| {
        let kv = kv_get.clone();
        async move {
            if let Some(text) = msg.text {
                let mut parts = text.splitn(2, ' ');
                parts.next();
                if let Some(k) = parts.next() {
                    let v = kv.read().await.get(k).cloned().unwrap_or_else(|| "(not set)".to_string());
                    client.send_message(msg.chat.id, &v, None).await?;
                }
            }
            Ok(())
        }
    });

    let users_clone = users.clone();
    disp.add_command("broadcast", move |client: Client, msg: Message| {
        let users = users_clone.clone();
        let admin = admin_id;
        async move {
            if admin.is_none() { let _ = client.send_message(msg.chat.id, "ADMIN_ID not set", None).await; return Ok(()); }
            let allowed = msg.from.as_ref().map(|u| Some(u.id) == admin).unwrap_or(false);
            if !allowed { let _ = client.send_message(msg.chat.id, "not allowed", None).await; return Ok(()); }
            if let Some(text) = msg.text {
                let parts: Vec<&str> = text.splitn(2, ' ').collect();
                if parts.len() < 2 { let _ = client.send_message(msg.chat.id, "usage: /broadcast <text>", None).await; return Ok(()); }
                let body = parts[1];
                let list: Vec<i64> = users.read().await.iter().cloned().collect();
                for uid in list {
                    let _ = client.send_message(uid, body, None).await;
                }
            }
            Ok(())
        }
    });

    disp.add_command("upload", |client: Client, msg: Message| async move {
        let path = "README.md";
        client.send_document_path(msg.chat.id, path).await?;
        Ok(())
    });

    let users_c = users.clone();
    let counters_c = counters.clone();
    disp.add_command("stats", move |client: Client, msg: Message| {
        let users = users_c.clone();
        let counters = counters_c.clone();
        async move {
            let u = users.read().await.len();
            let stats = counters.read().await.clone();
            let mut s = format!("users: {}\n", u);
            for (k,v) in stats { s.push_str(&format!("{}: {}\n", k, v)); }
            client.send_message(msg.chat.id, &s, None).await?;
            Ok(())
        }
    });

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
                                    if text.chars().next() == Some('/') {
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
