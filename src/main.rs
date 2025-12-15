mod client;
mod types;
mod dispatch;

use std::env;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};
use tokio::time::{sleep, Duration};
use client::Client;
use dispatch::Dispatcher;
use types::Message;

type KvStore = Arc<Mutex<HashMap<String, String>>>;
type Users = Arc<Mutex<HashSet<i64>>>;
type Counters = Arc<Mutex<HashMap<String, u64>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let token = env::var("TELEGRAM_BOT_TOKEN")
        .expect("Please set the TELEGRAM_BOT_TOKEN environment variable");

    let admin_id: Option<i64> = env::var("ADMIN_ID").ok().and_then(|s| s.parse().ok());
    let admin = admin_id; 

    let client = Client::new(token);
    let mut offset: i64 = 0;

    let kv: KvStore = Arc::new(Mutex::new(HashMap::new()));
    let users: Users = Arc::new(Mutex::new(HashSet::new()));
    let counters: Counters = Arc::new(Mutex::new(HashMap::new()));

    let mut disp = Dispatcher::new();

    let keyboard_markup = serde_json::json!({"keyboard": [[{"text":"/help"},{"text":"/ping"}], [{"text":"/whoami"}]], "one_time_keyboard": true});
    
    let kb_help = keyboard_markup.clone();
    let kb_start = keyboard_markup.clone();
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
            let _ = client.send_message(msg.chat.id, &help, Some(kb)).await;
        }
    });

    let users_for_start = users.clone();
    disp.add_command("start", move |client: Client, msg: Message| {
        let users = users_for_start.clone();
        let kb = kb_start.clone();
        async move {
            users.lock().unwrap().insert(msg.chat.id);
            let name = msg.from.as_ref().map(|u| u.first_name.clone()).unwrap_or_else(|| "there".to_string());
            let welcome = format!("Hello, {}! Welcome. Type /help to see available commands.", name);
            let _ = client.send_message(msg.chat.id, &welcome, Some(kb)).await;
        }
    });

    disp.add_command("ping", |client: Client, msg: Message| async move {
        let _ = client.send_message(msg.chat.id, "pong", None).await;
    });

    disp.add_command("echo", |client: Client, msg: Message| async move {
        if let Some(text) = msg.text {
            let parts: Vec<&str> = text.splitn(2, ' ').collect();
            let resp = if parts.len() > 1 { parts[1].to_string() } else { "".to_string() };
            let _ = client.send_message(msg.chat.id, &resp, None).await;
        }
    });

    disp.add_command("whoami", |client: Client, msg: Message| async move {
        let user = msg.from;
        if let Some(u) = user {
            let name = u.username.clone().unwrap_or_else(|| u.first_name.clone());
            let resp = format!("id: {}\nusername: {}", u.id, name);
            let _ = client.send_message(msg.chat.id, &resp, None).await;
        }
    });

    disp.add_command("keyboard", move |client: Client, msg: Message| {
        let rm = kb_keyboard.clone();
        async move {
            let _ = client.send_message(msg.chat.id, "Choose:", Some(rm)).await;
        }
    });

    disp.add_command("inline", |client: Client, msg: Message| async move {
        let inline = serde_json::json!({
            "inline_keyboard": [[{"text":"Say hi","callback_data":"echo Hello from button"}]]
        });
        let _ = client.send_message(msg.chat.id, "Inline example:", Some(inline)).await;
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
                        kv.lock().unwrap().insert(k.to_string(), v.to_string());
                    }
                }
            }
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
                    let v = kv.lock().unwrap().get(k).cloned().unwrap_or_else(|| "(not set)".to_string());
                    let _ = client.send_message(msg.chat.id, &v, None).await;
                }
            }
        }
    });

    let users_clone = users.clone();
    disp.add_command("broadcast", move |client: Client, msg: Message| {
        let users = users_clone.clone();
        let admin = admin_id;
        async move {
            if admin.is_none() { let _ = client.send_message(msg.chat.id, "ADMIN_ID not set", None).await; return; }
            let allowed = msg.from.as_ref().map(|u| Some(u.id) == admin).unwrap_or(false);
            if !allowed { let _ = client.send_message(msg.chat.id, "not allowed", None).await; return; }
            if let Some(text) = msg.text {
                let parts: Vec<&str> = text.splitn(2, ' ').collect();
                if parts.len() < 2 { let _ = client.send_message(msg.chat.id, "usage: /broadcast <text>", None).await; return; }
                let body = parts[1];
                let list: Vec<i64> = users.lock().unwrap().iter().cloned().collect();
                for uid in list {
                    let _ = client.send_message(uid, body, None).await;
                }
            }
        }
    });

    disp.add_command("upload", |client: Client, msg: Message| async move {
        let path = "README.md";
        let _ = client.send_document_path(msg.chat.id, path).await;
    });

    let users_c = users.clone();
    let counters_c = counters.clone();
    disp.add_command("stats", move |client: Client, msg: Message| {
        let users = users_c.clone();
        let counters = counters_c.clone();
        async move {
            let u = users.lock().unwrap().len();
            let stats = counters.lock().unwrap().clone();
            let mut s = format!("users: {}\n", u);
            for (k,v) in stats { s.push_str(&format!("{}: {}\n", k, v)); }
            let _ = client.send_message(msg.chat.id, &s, None).await;
        }
    });

    println!("Starting polling bot with dispatcher... (press Ctrl+C to stop)");

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
                                users.lock().unwrap().insert(msg.chat.id);
                            }
                            
                            if let Some(msg) = u.message {
                                
                                if let Some(text) = &msg.text {
                                    if text.chars().next() == Some('/') {
                                        let cmd = text.split_whitespace().next().unwrap_or("").trim_start_matches('/').to_string();
                                        *counters.lock().unwrap().entry(cmd).or_insert(0) += 1;
                                    }
                                }
                                println!("Message from {}: {}", msg.chat.id, msg.text.clone().unwrap_or_default());
                                disp.dispatch(client.clone(), msg).await;
                            }
                            
                            if let Some(cb) = u.callback_query {
                                if let Some(data) = cb.data {
                                    
                                    let synthetic_chat = if let Some(m) = cb.message.clone() { m.chat.clone() } else { types::Chat { id: cb.from.id, kind: None, username: None, first_name: None, last_name: None } };
                                    let synthetic = Message { message_id: 0, text: Some(data), chat: synthetic_chat, from: Some(cb.from) };
                                    disp.dispatch(client.clone(), synthetic).await;
                                }
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
    }

    Ok(())
}
