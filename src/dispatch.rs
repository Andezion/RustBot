use std::collections::HashMap;
use std::sync::Arc;
use futures::future::BoxFuture;
use futures::FutureExt;
use crate::client::{Client, BotError};
use crate::types::{Message, CallbackQuery};
use tracing::error;
use tokio::sync::Semaphore;

pub type Handler = Arc<dyn Fn(Client, Message) -> BoxFuture<'static, Result<(), BotError>> + Send + Sync>;
pub type CallbackHandler = Arc<dyn Fn(Client, CallbackQuery) -> BoxFuture<'static, Result<(), BotError>> + Send + Sync>;

pub struct Dispatcher {
    commands: HashMap<String, Vec<Handler>>,
    callbacks: Vec<CallbackHandler>,
    handler_sem: Option<Arc<Semaphore>>,
    admin: Option<i64>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self { commands: HashMap::new(), callbacks: Vec::new(), handler_sem: None, admin: None }
    }

    pub fn set_concurrency_limit(&mut self, sem: Arc<Semaphore>) {
        self.handler_sem = Some(sem);
    }

    pub fn set_admin(&mut self, admin: Option<i64>) {
        self.admin = admin;
    }

    pub fn add_command<F, Fut>(&mut self, cmd: &str, f: F)
    where
        F: Fn(Client, Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), BotError>> + Send + 'static,
    {
        let h: Handler = Arc::new(move |client: Client, msg: Message| {
            (f)(client, msg).boxed()
        });
        let key = cmd.trim_start_matches('/').to_string();
        self.commands.entry(key).or_default().push(h);
    }

    pub fn add_callback<F, Fut>(&mut self, f: F)
    where
        F: Fn(Client, CallbackQuery) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), BotError>> + Send + 'static,
    {
        let h: CallbackHandler = Arc::new(move |client: Client, cb: CallbackQuery| {
            (f)(client, cb).boxed()
        });
        self.callbacks.push(h);
    }

    pub async fn dispatch(&self, client: Client, msg: Message) {
        if let Some(text) = &msg.text {
            if text.starts_with('/') {
                let parts: Vec<&str> = text.split_whitespace().collect();
                let cmd = parts[0].trim_start_matches('/').to_string();
                if let Some(handlers) = self.commands.get(&cmd) {
                    for h in handlers {
                        let c_for_call = client.clone();
                        let c_for_notify = c_for_call.clone();
                        let m = msg.clone();
                        let cmd_clone = cmd.clone();

                        let sem = self.handler_sem.clone();
                        let admin = self.admin;
                        let fut = h(c_for_call, m);
                        tokio::spawn(async move {
                            let _permit = if let Some(s) = sem {
                                match s.clone().acquire_owned().await {
                                    Ok(p) => Some(p),
                                    Err(_) => None,
                                }
                            } else { None };
                            match std::panic::AssertUnwindSafe(fut).catch_unwind().await {
                                Ok(Ok(())) => {}
                                Ok(Err(e)) => {
                                    error!("handler error: {}", e);
                                    if let Some(aid) = admin {
                                        let _ = c_for_notify.send_message(aid, &format!("Handler error for command '/{}': {}", cmd_clone, e), None).await;
                                    }
                                }
                                Err(p) => {
                                    error!("handler panicked: {:?}", p);
                                    if let Some(aid) = admin {
                                        let _ = c_for_notify.send_message(aid, &format!("Handler panicked for command '/{}': {:?}", cmd_clone, p), None).await;
                                    }
                                }
                            }
                        });
                    }
                }
            }
        }
    }

    pub async fn dispatch_callback(&self, client: Client, cb: CallbackQuery) {
        for h in &self.callbacks {
            let c_for_call = client.clone();
            let c_for_notify = c_for_call.clone();
            let cb_clone = cb.clone();
            let sem = self.handler_sem.clone();
            let admin = self.admin;
            let fut = h(c_for_call, cb_clone);
            tokio::spawn(async move {
                let _permit = if let Some(s) = sem {
                    match s.clone().acquire_owned().await {
                        Ok(p) => Some(p),
                        Err(_) => None,
                    }
                } else { None };
                match std::panic::AssertUnwindSafe(fut).catch_unwind().await {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        error!("callback handler error: {}", e);
                        if let Some(aid) = admin {
                            let _ = c_for_notify.send_message(aid, &format!("Callback handler error: {}", e), None).await;
                        }
                    }
                    Err(p) => {
                        error!("callback handler panicked: {:?}", p);
                        if let Some(aid) = admin {
                            let _ = c_for_notify.send_message(aid, &format!("Callback handler panicked: {:?}", p), None).await;
                        }
                    }
                }
            });
        }
    }
}
