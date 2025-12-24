use crate::client::Client;
use crate::dispatch::Dispatcher;
use crate::types::{Message, ReplyMarkup};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};

pub fn register(
    disp: &mut Dispatcher,
    admin: Option<i64>,
    _users: Arc<RwLock<HashSet<i64>>>,
    kb_help: ReplyMarkup,
    kb_start: ReplyMarkup,
    kb_keyboard: ReplyMarkup,
) {
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

    let users_for_start = _users.clone();
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

    disp.add_command("keyboard", move |client: Client, msg: Message| {
        let rm = kb_keyboard.clone();
        async move {
            let rmv = serde_json::to_value(&rm).ok();
            client.send_message(msg.chat.id, "Choose:", rmv).await?;
            Ok(())
        }
    });

    disp.add_command("inline", |client: Client, msg: Message| async move {
        let inline = ReplyMarkup::InlineKeyboard(crate::types::InlineKeyboardMarkup {
            inline_keyboard: vec![vec![crate::types::InlineKeyboardButton { text: "Say hi".to_string(), callback_data: Some("echo Hello from button".to_string()), url: None }]]
        });
        let rm = serde_json::to_value(&inline).ok();
        client.send_message(msg.chat.id, "Inline example:", rm).await?;
        Ok(())
    });

    disp.add_callback(|client: Client, cb: crate::types::CallbackQuery| async move {
        let _ = client.answer_callback_query(&cb.id, Some("Received"), Some(false), None, None).await;
        if let Some(msg) = cb.message {
            let chat_id = msg.chat.id;
            let d = cb.data.unwrap_or_else(|| "(no data)".to_string());
            client.send_message(chat_id, &format!("Button pressed: {}", d), None).await?;
        }
        Ok(())
    });
}
