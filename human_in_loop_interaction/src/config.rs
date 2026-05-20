use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitlConfig {
    pub editor: EditorConfig,
    pub diff: DiffConfig,
    pub approval: ApprovalConfig,
    pub session: SessionConfig,
    pub freeze: FreezeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub command: String,
    pub diff_command: Option<String>,
    pub wait_for_close: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffConfig {
    pub max_lines: usize,
    pub auto_accept_empty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalConfig {
    pub default_timeout_secs: u64,
    pub session_accept_warning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub max_sessions_kept: usize,
    pub auto_name: bool,
    pub crash_recovery_prompt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeConfig {
    pub auto_freeze_on_disconnect: bool,
    pub checkpoint_conversation: bool,
}

impl Default for HitlConfig {
    fn default() -> Self {
        Self {
            editor: EditorConfig {
                command: "nvim".into(),
                diff_command: None,
                wait_for_close: true,
            },
            diff: DiffConfig {
                max_lines: 20,
                auto_accept_empty: true,
            },
            approval: ApprovalConfig {
                default_timeout_secs: 0,
                session_accept_warning: true,
            },
            session: SessionConfig {
                max_sessions_kept: 50,
                auto_name: true,
                crash_recovery_prompt: true,
            },
            freeze: FreezeConfig {
                auto_freeze_on_disconnect: true,
                checkpoint_conversation: true,
            },
        }
    }
}

impl HitlConfig {
    pub fn load(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn config_path() -> PathBuf {
        dirs::home_dir().unwrap_or_default().join(".arcana/hitl.toml")
    }
}
