use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::types::{AgentInfo, AgentScope, AgentStatus};

#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Serialized state of a single agent (main or sub).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCheckpoint {
    pub agent_id: String,
    pub status: AgentStatus,
    pub conversation_history: Vec<serde_json::Value>,
    pub task: String,
    pub context_snapshot: String,
    pub scope: AgentScope,
    pub turn_count: usize,
    pub max_turns: usize,
    pub files_modified: Vec<String>,
    pub active_skills: Vec<String>,
    pub frozen_at: DateTime<Utc>,
}

/// Serialized state of the orchestrator (the full agent tree).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorState {
    pub agents: Vec<AgentInfo>,
    pub frozen_at: DateTime<Utc>,
}

fn checkpoints_dir(project_dir: &Path) -> PathBuf {
    project_dir.join(".arcana/checkpoints")
}

/// Save an agent checkpoint to disk.
pub fn save_agent(project_dir: &Path, checkpoint: &AgentCheckpoint) -> Result<(), CheckpointError> {
    let dir = checkpoints_dir(project_dir);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", checkpoint.agent_id));
    let json = serde_json::to_string_pretty(checkpoint)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load an agent checkpoint from disk.
pub fn load_agent(project_dir: &Path, agent_id: &str) -> Result<AgentCheckpoint, CheckpointError> {
    let path = checkpoints_dir(project_dir).join(format!("{agent_id}.json"));
    let json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

/// Save orchestrator state.
pub fn save_orchestrator(project_dir: &Path, state: &OrchestratorState) -> Result<(), CheckpointError> {
    let dir = checkpoints_dir(project_dir);
    std::fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(dir.join("orchestrator_state.json"), json)?;
    Ok(())
}

/// Load orchestrator state.
pub fn load_orchestrator(project_dir: &Path) -> Result<OrchestratorState, CheckpointError> {
    let path = checkpoints_dir(project_dir).join("orchestrator_state.json");
    let json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

/// List all checkpoint files in the project.
pub fn list_checkpoints(project_dir: &Path) -> Vec<String> {
    let dir = checkpoints_dir(project_dir);
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".json") && name != "orchestrator_state.json" {
                Some(name.trim_end_matches(".json").to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Remove a checkpoint.
pub fn remove_checkpoint(project_dir: &Path, agent_id: &str) -> Result<(), CheckpointError> {
    let path = checkpoints_dir(project_dir).join(format!("{agent_id}.json"));
    if path.exists() { std::fs::remove_file(path)?; }
    Ok(())
}
