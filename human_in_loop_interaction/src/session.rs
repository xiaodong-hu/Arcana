use chrono::Utc;
use std::path::{Path, PathBuf};

use crate::types::{SessionMeta, SessionStatus};

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("session not found: {0}")]
    NotFound(String),
}

fn sessions_dir(project_dir: &Path) -> PathBuf {
    project_dir.join(".arcana/sessions")
}

fn index_path(project_dir: &Path) -> PathBuf {
    sessions_dir(project_dir).join("index.json")
}

/// Load the session index.
pub fn load_index(project_dir: &Path) -> Result<Vec<SessionMeta>, SessionError> {
    let path = index_path(project_dir);
    if !path.exists() { return Ok(Vec::new()); }
    let json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

/// Save the session index.
pub fn save_index(project_dir: &Path, index: &[SessionMeta]) -> Result<(), SessionError> {
    let dir = sessions_dir(project_dir);
    std::fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(index)?;
    std::fs::write(index_path(project_dir), json)?;
    Ok(())
}

/// Create a new session, returning its ID.
pub fn create_session(project_dir: &Path, model: &str) -> Result<String, SessionError> {
    let id = format!("sess-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let now = Utc::now().to_rfc3339();

    let session_dir = sessions_dir(project_dir).join(&id);
    std::fs::create_dir_all(&session_dir)?;

    // Write lock file (for crash detection)
    std::fs::write(session_dir.join("lock"), "")?;

    // Write meta
    let meta = SessionMeta {
        id: id.clone(),
        name: "Untitled session".into(),
        status: SessionStatus::Active,
        created_at: now.clone(),
        last_active: now,
        turn_count: 0,
        model: model.to_string(),
    };
    let meta_json = serde_json::to_string_pretty(&meta)?;
    std::fs::write(session_dir.join("meta.json"), meta_json)?;

    // Update index
    let mut index = load_index(project_dir)?;
    index.push(meta);
    save_index(project_dir, &index)?;

    Ok(id)
}

/// Update session metadata (name, status, turn_count, last_active).
pub fn update_session(project_dir: &Path, id: &str, name: Option<&str>, status: Option<SessionStatus>, turn_count: Option<usize>) -> Result<(), SessionError> {
    let mut index = load_index(project_dir)?;
    let entry = index.iter_mut().find(|s| s.id == id).ok_or_else(|| SessionError::NotFound(id.to_string()))?;

    if let Some(n) = name { entry.name = n.to_string(); }
    if let Some(s) = status { entry.status = s; }
    if let Some(t) = turn_count { entry.turn_count = t; }
    entry.last_active = Utc::now().to_rfc3339();

    // Clone for writing meta.json before releasing borrow
    let entry_clone = entry.clone();
    save_index(project_dir, &index)?;

    let meta_path = sessions_dir(project_dir).join(id).join("meta.json");
    let meta_json = serde_json::to_string_pretty(&entry_clone)?;
    std::fs::write(meta_path, meta_json)?;

    Ok(())
}

/// Mark session as frozen and remove lock file.
pub fn freeze_session(project_dir: &Path, id: &str) -> Result<(), SessionError> {
    update_session(project_dir, id, None, Some(SessionStatus::Frozen), None)?;
    let lock = sessions_dir(project_dir).join(id).join("lock");
    if lock.exists() { std::fs::remove_file(lock)?; }
    Ok(())
}

/// Mark session as completed and remove lock file.
pub fn complete_session(project_dir: &Path, id: &str) -> Result<(), SessionError> {
    update_session(project_dir, id, None, Some(SessionStatus::Completed), None)?;
    let lock = sessions_dir(project_dir).join(id).join("lock");
    if lock.exists() { std::fs::remove_file(lock)?; }
    Ok(())
}

/// Detect crashed sessions (lock file exists but status is not Active).
pub fn detect_crashed(project_dir: &Path) -> Result<Vec<SessionMeta>, SessionError> {
    let index = load_index(project_dir)?;
    let mut crashed = Vec::new();
    for session in &index {
        if session.status == SessionStatus::Active {
            let lock = sessions_dir(project_dir).join(&session.id).join("lock");
            // If lock exists and status is Active, it might be running OR crashed.
            // We can't tell without checking if the process is alive.
            // For now, report all Active sessions with lock files as potentially crashed.
            if lock.exists() {
                crashed.push(session.clone());
            }
        }
    }
    Ok(crashed)
}

/// Delete a session and all its data.
pub fn delete_session(project_dir: &Path, id: &str) -> Result<(), SessionError> {
    let session_dir = sessions_dir(project_dir).join(id);
    if session_dir.exists() {
        std::fs::remove_dir_all(session_dir)?;
    }
    let mut index = load_index(project_dir)?;
    index.retain(|s| s.id != id);
    save_index(project_dir, &index)?;
    Ok(())
}

/// Prune old sessions beyond max_kept (oldest first).
pub fn prune_sessions(project_dir: &Path, max_kept: usize) -> Result<Vec<String>, SessionError> {
    let mut index = load_index(project_dir)?;
    let mut pruned = Vec::new();

    // Only prune completed sessions
    let completed: Vec<_> = index.iter()
        .filter(|s| s.status == SessionStatus::Completed)
        .cloned()
        .collect();

    if completed.len() > max_kept {
        let to_remove = completed.len() - max_kept;
        for session in completed.iter().take(to_remove) {
            let session_dir = sessions_dir(project_dir).join(&session.id);
            if session_dir.exists() { std::fs::remove_dir_all(session_dir).ok(); }
            pruned.push(session.id.clone());
        }
        index.retain(|s| !pruned.contains(&s.id));
        save_index(project_dir, &index)?;
    }

    Ok(pruned)
}

/// Save conversation history for a session.
pub fn save_conversation(project_dir: &Path, id: &str, messages: &[serde_json::Value]) -> Result<(), SessionError> {
    let path = sessions_dir(project_dir).join(id).join("conversation.json");
    let json = serde_json::to_string_pretty(messages)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load conversation history for a session.
pub fn load_conversation(project_dir: &Path, id: &str) -> Result<Vec<serde_json::Value>, SessionError> {
    let path = sessions_dir(project_dir).join(id).join("conversation.json");
    if !path.exists() { return Ok(Vec::new()); }
    let json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}
