// Ref: FT-SSF-026
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub struct CacheEntry {
    pub value: String,
    pub expires_at: u64,
}

pub struct TTLCache {
    store: HashMap<String, CacheEntry>,
    default_ttl_secs: u64,
}

impl TTLCache {
    pub fn new(default_ttl: u64) -> Self {
        Self {
            store: HashMap::new(),
            default_ttl_secs: default_ttl,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<&str> {
        let now = now_secs();
        if let Some(entry) = self.store.get(key) {
            if entry.expires_at <= now {
                self.store.remove(key);
                return None;
            }
        }
        self.store.get(key).map(|e| e.value.as_str())
    }

    pub fn set(&mut self, key: &str, value: String, ttl: Option<u64>) {
        let ttl = ttl.unwrap_or(self.default_ttl_secs);
        self.store.insert(
            key.to_string(),
            CacheEntry {
                value,
                expires_at: now_secs() + ttl,
            },
        );
    }

    pub fn delete(&mut self, key: &str) -> bool {
        self.store.remove(key).is_some()
    }

    pub fn cleanup(&mut self) {
        let now = now_secs();
        self.store.retain(|_, entry| entry.expires_at > now);
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn keys(&self) -> Vec<&str> {
        self.store.keys().map(|k| k.as_str()).collect()
    }
}
