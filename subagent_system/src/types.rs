use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Running,
    Frozen,
    Completed,
    Failed,
    Stalled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentScope {
    pub read_paths: Vec<String>,
    pub write_paths: Vec<String>,
    pub exec_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    pub task: String,
    pub context: String,
    pub scope: AgentScope,
    pub model: Option<String>,
    pub max_turns: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub task: String,
    pub status: AgentStatus,
    pub turn_count: usize,
    pub max_turns: usize,
    pub files_modified: Vec<String>,
    pub spawned_at: DateTime<Utc>,
}

/// Request from main agent to orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum OrchestratorRequest {
    #[serde(rename = "spawn")]
    Spawn(SpawnRequest),
    #[serde(rename = "status")]
    Status,
    #[serde(rename = "collect")]
    Collect { id: String },
    #[serde(rename = "freeze")]
    Freeze { id: String },
    #[serde(rename = "freeze_all")]
    FreezeAll,
    #[serde(rename = "unfreeze")]
    Unfreeze { id: String, new_context: Option<String> },
    #[serde(rename = "kill")]
    Kill { id: String },
}

/// Response from orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestratorResponse {
    #[serde(rename = "spawned")]
    Spawned { id: String, status: AgentStatus },
    #[serde(rename = "status")]
    Status { agents: Vec<AgentInfo> },
    #[serde(rename = "collected")]
    Collected { id: String, output: String, files_modified: Vec<String>, status: AgentStatus },
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "error")]
    Error { message: String },
}
