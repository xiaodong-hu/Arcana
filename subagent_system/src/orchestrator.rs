use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;

use crate::checkpoint;
use crate::types::*;

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("checkpoint: {0}")]
    Checkpoint(#[from] checkpoint::CheckpointError),
    #[error("agent not found: {0}")]
    NotFound(String),
    #[error("agent not in expected state: {0} is {1:?}")]
    BadState(String, AgentStatus),
}

/// In-memory state of a managed sub-agent.
struct ManagedAgent {
    info: AgentInfo,
    scope: AgentScope,
    context: String,
    conversation: Vec<serde_json::Value>,
    stall_counter: usize,
}

/// The sub-agent orchestrator. Manages lifecycle of all sub-agents.
pub struct Orchestrator {
    agents: HashMap<String, ManagedAgent>,
    project_dir: std::path::PathBuf,
}

impl Orchestrator {
    pub fn new(project_dir: &Path) -> Self {
        Self { agents: HashMap::new(), project_dir: project_dir.to_path_buf() }
    }

    /// Handle a request from the main agent.
    pub fn handle(&mut self, req: OrchestratorRequest) -> Result<OrchestratorResponse, OrchestratorError> {
        match req {
            OrchestratorRequest::Spawn(spawn) => self.spawn(spawn),
            OrchestratorRequest::Status => self.status(),
            OrchestratorRequest::Collect { id } => self.collect(&id),
            OrchestratorRequest::Freeze { id } => self.freeze(&id),
            OrchestratorRequest::FreezeAll => self.freeze_all(),
            OrchestratorRequest::Unfreeze { id, new_context } => self.unfreeze(&id, new_context),
            OrchestratorRequest::Kill { id } => self.kill(&id),
        }
    }

    fn spawn(&mut self, req: SpawnRequest) -> Result<OrchestratorResponse, OrchestratorError> {
        let id = format!("subagent-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let info = AgentInfo {
            id: id.clone(),
            task: req.task.clone(),
            status: AgentStatus::Running,
            turn_count: 0,
            max_turns: req.max_turns,
            files_modified: Vec::new(),
            spawned_at: Utc::now(),
        };

        self.agents.insert(id.clone(), ManagedAgent {
            info: info.clone(),
            scope: req.scope,
            context: req.context,
            conversation: Vec::new(),
            stall_counter: 0,
        });

        Ok(OrchestratorResponse::Spawned { id, status: AgentStatus::Running })
    }

    fn status(&self) -> Result<OrchestratorResponse, OrchestratorError> {
        let agents: Vec<AgentInfo> = self.agents.values().map(|a| a.info.clone()).collect();
        Ok(OrchestratorResponse::Status { agents })
    }

    fn collect(&self, id: &str) -> Result<OrchestratorResponse, OrchestratorError> {
        let agent = self.agents.get(id).ok_or_else(|| OrchestratorError::NotFound(id.to_string()))?;
        Ok(OrchestratorResponse::Collected {
            id: id.to_string(),
            output: agent.context.clone(),
            files_modified: agent.info.files_modified.clone(),
            status: agent.info.status,
        })
    }

    fn freeze(&mut self, id: &str) -> Result<OrchestratorResponse, OrchestratorError> {
        let agent = self.agents.get_mut(id).ok_or_else(|| OrchestratorError::NotFound(id.to_string()))?;
        if agent.info.status != AgentStatus::Running {
            return Err(OrchestratorError::BadState(id.to_string(), agent.info.status));
        }
        agent.info.status = AgentStatus::Frozen;

        // Serialize to disk
        let cp = checkpoint::AgentCheckpoint {
            agent_id: id.to_string(),
            status: AgentStatus::Frozen,
            conversation_history: agent.conversation.clone(),
            task: agent.info.task.clone(),
            context_snapshot: agent.context.clone(),
            scope: agent.scope.clone(),
            turn_count: agent.info.turn_count,
            max_turns: agent.info.max_turns,
            files_modified: agent.info.files_modified.clone(),
            active_skills: Vec::new(),
            frozen_at: Utc::now(),
        };
        checkpoint::save_agent(&self.project_dir, &cp)?;

        Ok(OrchestratorResponse::Ok)
    }

    fn freeze_all(&mut self) -> Result<OrchestratorResponse, OrchestratorError> {
        let ids: Vec<String> = self.agents.keys().cloned().collect();
        for id in &ids {
            if self.agents[id].info.status == AgentStatus::Running {
                self.freeze(id)?;
            }
        }
        // Save orchestrator state
        let state = checkpoint::OrchestratorState {
            agents: self.agents.values().map(|a| a.info.clone()).collect(),
            frozen_at: Utc::now(),
        };
        checkpoint::save_orchestrator(&self.project_dir, &state)?;
        Ok(OrchestratorResponse::Ok)
    }

    fn unfreeze(&mut self, id: &str, new_context: Option<String>) -> Result<OrchestratorResponse, OrchestratorError> {
        let agent = self.agents.get_mut(id).ok_or_else(|| OrchestratorError::NotFound(id.to_string()))?;
        if agent.info.status != AgentStatus::Frozen {
            return Err(OrchestratorError::BadState(id.to_string(), agent.info.status));
        }
        agent.info.status = AgentStatus::Running;
        if let Some(ctx) = new_context {
            agent.context = ctx;
        }
        // Remove checkpoint file
        checkpoint::remove_checkpoint(&self.project_dir, id)?;
        Ok(OrchestratorResponse::Ok)
    }

    fn kill(&mut self, id: &str) -> Result<OrchestratorResponse, OrchestratorError> {
        self.agents.remove(id).ok_or_else(|| OrchestratorError::NotFound(id.to_string()))?;
        checkpoint::remove_checkpoint(&self.project_dir, id).ok();
        Ok(OrchestratorResponse::Ok)
    }

    /// Called by the daemon each turn a sub-agent takes. Enforces limits.
    pub fn record_turn(&mut self, id: &str, files_modified: &[String]) -> Result<Option<AgentStatus>, OrchestratorError> {
        let agent = self.agents.get_mut(id).ok_or_else(|| OrchestratorError::NotFound(id.to_string()))?;
        agent.info.turn_count += 1;

        if files_modified.is_empty() {
            agent.stall_counter += 1;
        } else {
            agent.stall_counter = 0;
            for f in files_modified {
                if !agent.info.files_modified.contains(f) {
                    agent.info.files_modified.push(f.clone());
                }
            }
        }

        // Enforce completion criteria
        if agent.info.turn_count >= agent.info.max_turns {
            agent.info.status = AgentStatus::Stalled;
            return Ok(Some(AgentStatus::Stalled));
        }
        if agent.stall_counter >= 5 {
            agent.info.status = AgentStatus::Stalled;
            return Ok(Some(AgentStatus::Stalled));
        }

        Ok(None)
    }

    /// Mark a sub-agent as completed (called when `done` tool passes post-hooks).
    pub fn mark_completed(&mut self, id: &str) -> Result<(), OrchestratorError> {
        let agent = self.agents.get_mut(id).ok_or_else(|| OrchestratorError::NotFound(id.to_string()))?;
        agent.info.status = AgentStatus::Completed;
        Ok(())
    }

    /// Restore from checkpoints on disk.
    pub fn restore(project_dir: &Path) -> Result<Self, OrchestratorError> {
        let mut orch = Self::new(project_dir);
        if let Ok(state) = checkpoint::load_orchestrator(project_dir) {
            for info in state.agents {
                if let Ok(cp) = checkpoint::load_agent(project_dir, &info.id) {
                    orch.agents.insert(info.id.clone(), ManagedAgent {
                        info,
                        scope: cp.scope,
                        context: cp.context_snapshot,
                        conversation: cp.conversation_history,
                        stall_counter: 0,
                    });
                }
            }
        }
        Ok(orch)
    }
}
