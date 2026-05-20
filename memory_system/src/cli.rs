//! CLI interface for human access to memory stores.

use crate::manager::{ManagerError, MemoryManager};
use crate::types::{MemoryEntry, StoreKind};

pub fn list(manager: &MemoryManager, kind: StoreKind, limit: Option<usize>) -> Result<Vec<MemoryEntry>, ManagerError> {
    let store = manager.store(kind).ok_or(ManagerError::NotInitialized)?;
    let mut entries = store.list_all()?;
    if let Some(n) = limit { entries.truncate(n); }
    Ok(entries)
}

pub fn search(manager: &MemoryManager, kind: StoreKind, query: &str, top_k: usize) -> Result<Vec<(MemoryEntry, f32)>, ManagerError> {
    let store = manager.store(kind).ok_or(ManagerError::NotInitialized)?;
    let embedding = manager.embedder().embed(query)?;
    let results = store.search_hybrid(query, &embedding, top_k)?;
    Ok(results.into_iter().map(|r| (r.entry, r.score)).collect())
}

pub fn get(manager: &MemoryManager, kind: StoreKind, id: &str) -> Result<MemoryEntry, ManagerError> {
    let store = manager.store(kind).ok_or(ManagerError::NotInitialized)?;
    Ok(store.get(id)?)
}

pub fn delete(manager: &MemoryManager, kind: StoreKind, id: &str) -> Result<(), ManagerError> {
    let store = manager.store(kind).ok_or(ManagerError::NotInitialized)?;
    store.delete(id)?;
    Ok(())
}

pub fn export(manager: &MemoryManager, kind: StoreKind) -> Result<String, ManagerError> {
    let entries = list(manager, kind, None)?;
    Ok(serde_json::to_string_pretty(&entries).unwrap_or_default())
}

pub fn import(manager: &MemoryManager, kind: StoreKind, json: &str) -> Result<usize, ManagerError> {
    let entries: Vec<MemoryEntry> = serde_json::from_str(json)
        .map_err(|e| ManagerError::Store(crate::store::StoreError::Serde(e)))?;
    let count = entries.len();
    let store = manager.store(kind).ok_or(ManagerError::NotInitialized)?;
    for entry in &entries {
        store.insert(entry)?;
    }
    Ok(count)
}

pub fn format_entry(entry: &MemoryEntry) -> String {
    format!(
        "[{}] score={:.4} access={} last={}\n  {}\n  tags: {:?}",
        &entry.id[..8], entry.activation_score, entry.access_count,
        entry.last_accessed.format("%Y-%m-%d %H:%M"),
        entry.text.lines().next().unwrap_or(""), entry.tags,
    )
}
