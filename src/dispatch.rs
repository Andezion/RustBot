use std::collections::HashMap;
use std::sync::Arc;
use futures::future::BoxFuture;
use futures::FutureExt;
use crate::client::Client;
use crate::types::Message;
use tracing::error;

pub type Handler = Arc<dyn Fn(Client, Message) -> BoxFuture<'static, ()> + Send + Sync>;

pub struct Dispatcher {
    commands: HashMap<String, Vec<Handler>>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self { commands: HashMap::new() }
    }

    pub fn add_command<F, Fut>(&mut self, cmd: &str, f: F)
    where
        F: Fn(Client, Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let h: Handler = Arc::new(move |client: Client, msg: Message| {
            (f)(client, msg).boxed()
        });
        let key = cmd.trim_start_matches('/').to_string();
        self.commands.entry(key).or_default().push(h);
    }

    pub async fn dispatch(&self, client: Client, msg: Message) {
        if let Some(text) = &msg.text {
            if text.starts_with('/') {
                let parts: Vec<&str> = text.split_whitespace().collect();
                let cmd = parts[0].trim_start_matches('/').to_string();
                if let Some(handlers) = self.commands.get(&cmd) {
                    for h in handlers {
                        let c = client.clone();
                        let m = msg.clone();
                        // spawn each handler so one slow handler doesn't block others
                        let fut = h(c, m);
                        tokio::spawn(async move {
                            if let Err(e) = std::panic::AssertUnwindSafe(fut).catch_unwind().await {
                                error!("handler panicked: {:?}", e);
                            }
                        });
                    }
                }
            }
        }
    }
}
