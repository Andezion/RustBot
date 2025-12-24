pub mod basic;
pub mod store;
pub mod admin;

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use crate::dispatch::Dispatcher;
use crate::types::ReplyMarkup;

type KvStore = Arc<RwLock<HashMap<String, String>>>;
type Users = Arc<RwLock<HashSet<i64>>>;
type Counters = Arc<RwLock<HashMap<String, u64>>>;

pub fn register(
    disp: &mut Dispatcher,
    admin: Option<i64>,
    kv: KvStore,
    users: Users,
    counters: Counters,
    kb_help: ReplyMarkup,
    kb_start: ReplyMarkup,
    kb_keyboard: ReplyMarkup,
) {
    basic::register(disp, admin, users.clone(), kb_help, kb_start, kb_keyboard);
    store::register(disp, kv);
    admin::register(disp, admin, users, counters);
}
