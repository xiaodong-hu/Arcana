use serde::{Deserialize, Serialize};

/// User's response to a diff review prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffApproval {
    Accept,
    SessionAccept,
    Edit,
    Abort,
}

/// User's response in a long-loop interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopSignal {
    Done,
    Unfinished,
    Abort,
}

/// Interrupt command from the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interrupt {
    Freeze,         // Ctrl+Shift+P
    ModifyPrompt,   // Ctrl+Shift+M
    Cancel,         // Ctrl+C
    EndSession,     // Ctrl+D
}

/// Session status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Frozen,
    Completed,
    Crashed,
}

/// Session metadata entry in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub name: String,
    pub status: SessionStatus,
    pub created_at: String,
    pub last_active: String,
    pub turn_count: usize,
    pub model: String,
}

/// A diff hunk for display.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub file: String,
    pub old_content: String,
    pub new_content: String,
    pub unified_diff: String,
}

/// Context correction message injected after human edit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanCorrection {
    pub file: String,
    pub llm_version: String,
    pub human_version: String,
    pub diff: String,
}
