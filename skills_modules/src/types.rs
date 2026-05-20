use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillMode {
    Context,
    Action,
    Hybrid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillPool {
    System,
    User,
    Project,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    #[serde(rename = "pre-write")]
    PreWrite,
    #[serde(rename = "post-write")]
    PostWrite,
    #[serde(rename = "pre-exec")]
    PreExec,
    #[serde(rename = "post-exec")]
    PostExec,
    #[serde(rename = "pre-commit")]
    PreCommit,
    #[serde(rename = "session-start")]
    SessionStart,
    #[serde(rename = "session-end")]
    SessionEnd,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreWrite => "pre-write",
            Self::PostWrite => "post-write",
            Self::PreExec => "pre-exec",
            Self::PostExec => "post-exec",
            Self::PreCommit => "pre-commit",
            Self::SessionStart => "session-start",
            Self::SessionEnd => "session-end",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pre-write" => Some(Self::PreWrite),
            "post-write" => Some(Self::PostWrite),
            "pre-exec" => Some(Self::PreExec),
            "post-exec" => Some(Self::PostExec),
            "pre-commit" => Some(Self::PreCommit),
            "session-start" => Some(Self::SessionStart),
            "session-end" => Some(Self::SessionEnd),
            _ => None,
        }
    }
}

/// A triggered skill result returned by the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredSkill {
    pub skill: String,
    pub mode: SkillMode,
    pub reason: String,
    pub priority: i32,
    pub pre_hooks: Vec<String>,
    pub post_hooks: Vec<String>,
    pub prompt_file: Option<String>,
    pub inject_output: bool,
}

/// Request from agent to daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum DaemonRequest {
    #[serde(rename = "evaluate")]
    Evaluate { prompt: String, event: String, files: Vec<String> },
    #[serde(rename = "reload")]
    Reload,
    #[serde(rename = "register")]
    Register { path: String },
    #[serde(rename = "query")]
    Query { prompt: String, top_k: usize },
}

/// Response from daemon to agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonResponse {
    pub triggered: Vec<TriggeredSkill>,
}
