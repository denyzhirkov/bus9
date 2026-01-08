use crate::engine::Engine;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use crate::config::AppConfig;

pub struct AppState {
    pub engine: Arc<Engine>,
    pub metrics: MetricsStore,
    pub config: AppConfig,
    pub auth_token: Option<String>,
    pub version: String,
}

pub struct MetricsStore {
    requests: RwLock<HashMap<String, u64>>,
}

impl MetricsStore {
    pub fn new() -> Self {
        Self {
            requests: RwLock::new(HashMap::new()),
        }
    }

    pub fn increment(&self, key: &str) {
        let mut requests = self.requests.write();
        *requests.entry(key.to_string()).or_insert(0) += 1;
    }

    pub fn snapshot(&self) -> HashMap<String, u64> {
        self.requests.read().clone()
    }
}
