use crate::client::Client;
use crate::dispatch::Dispatcher;
use crate::types::Message;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

type KvStore = Arc<RwLock<HashMap<String, String>>>;

pub fn register(disp: &mut Dispatcher, kv: KvStore) {
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
}
