//! One synchronized, immutable view of each registry per fetch invocation.
use super::cache::{RegistryCache, RegistrySnapshot};
use anyhow::Result;
use std::{collections::HashMap, path::PathBuf};

#[derive(Default)]
pub struct RegistrySession {
    snapshots: HashMap<(PathBuf, String), RegistrySnapshot>,
}

impl RegistrySession {
    pub fn synced(&mut self, cache: &RegistryCache) -> Result<&RegistrySnapshot> {
        let key = (cache.local_path(), cache.url().to_owned());
        if !self.snapshots.contains_key(&key) {
            // A failed synchronization must remain retryable.
            let snapshot = cache.snapshot(false)?;
            self.snapshots.insert(key.clone(), snapshot);
        }
        Ok(self.snapshots.get(&key).expect("snapshot was inserted"))
    }
}
