//! AdapterRegistry
use crate::adapters::{Adapter, AdapterResult};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
pub struct AdapterRegistry { adapters: HashMap<String, Arc<dyn Adapter>> }
impl AdapterRegistry {
    pub fn new() -> Self { Self { adapters: HashMap::new() } }
    pub fn register(mut self, adapter: Arc<dyn Adapter>) -> Self {
        self.adapters.insert(adapter.name().to_string(), adapter); self
    }
    pub fn get(&self, name: &str) -> Option<Arc<dyn Adapter>> {
        self.adapters.get(name).cloned()
    }
    pub fn inspect(&self, name: &str, tx: &Value) -> Option<AdapterResult> {
        self.get(name).map(|a| a.inspect(tx))
    }
}
impl Default for AdapterRegistry { fn default() -> Self { Self::new() } }
