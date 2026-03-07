use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

/// Deduplicates concurrent fetches for the same key.
///
/// When two requests arrive for the same key simultaneously:
/// - The first caller becomes the "leader" and receives an `InFlightGuard`
/// - Subsequent callers receive the `Notify` they should wait on
///
/// When the guard is dropped (on success or failure), all waiting callers are
/// unblocked and should re-check the cache or retry.
#[derive(Clone, Default)]
pub struct InFlightTracker {
    map: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
}

impl InFlightTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempts to claim the leader role for the given key.
    pub fn try_acquire(&self, key: &str) -> AcquireResult {
        let mut map = self.map.lock().unwrap();
        if let Some(notify) = map.get(key) {
            AcquireResult::Wait(Arc::clone(notify))
        } else {
            let notify = Arc::new(Notify::new());
            map.insert(key.to_string(), Arc::clone(&notify));
            AcquireResult::Lead(InFlightGuard {
                map: Arc::clone(&self.map),
                key: key.to_string(),
                notify,
            })
        }
    }
}

pub enum AcquireResult {
    /// This caller is the leader and should perform the fetch.
    Lead(InFlightGuard),
    /// Another caller is already fetching; wait on this `Notify`, then re-check cache.
    Wait(Arc<Notify>),
}

/// Held by the leader for a given in-flight key. On drop (whether the fetch
/// succeeded, failed, or the future was cancelled), all waiters are unblocked.
pub struct InFlightGuard {
    map: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
    key: String,
    notify: Arc<Notify>,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.map.lock().unwrap().remove(&self.key);
        self.notify.notify_waiters();
    }
}
