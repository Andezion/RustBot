use crate::client::Client;
use crate::dispatch::Dispatcher;
use crate::types::Message;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

type Users = Arc<RwLock<std::collections::HashSet<i64>>>;
type Counters = Arc<RwLock<HashMap<String, u64>>>;

pub fn register(disp: &mut Dispatcher, admin: Option<i64>, users: Users, counters: Counters) {
    let users_clone = users.clone();
    disp.add_command("broadcast", move |client: Client, msg: Message| {
        let users = users_clone.clone();
        let admin = admin;
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
                            if let Some(best) = sizes.last().cloned() {
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
}
