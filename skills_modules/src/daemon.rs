use std::path::Path;

use crate::manifest::{self, Skill};
use crate::trigger_db::{TriggerDb, TriggerDbError};
use crate::types::{DaemonRequest, DaemonResponse, TriggeredSkill};

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("trigger db: {0}")]
    TriggerDb(#[from] TriggerDbError),
    #[error("manifest: {0}")]
    Manifest(#[from] crate::manifest::ManifestError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// The skill daemon's core logic (trigger evaluation).
/// In production, this runs in a background process listening on a unix socket.
/// Here we expose it as a library for testability.
pub struct SkillDaemon {
    skills: Vec<Skill>,
    trigger_db: TriggerDb,
}

impl SkillDaemon {
    /// Initialize the daemon: discover skills, rebuild trigger DB.
    pub fn new(
        project_dir: Option<&Path>,
        embed_fn: &dyn Fn(&str) -> Option<Vec<f32>>,
    ) -> Result<Self, DaemonError> {
        let skills = manifest::discover_skills(project_dir);
        let db_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".arcana/skills/skill_trigger.db");
        let trigger_db = TriggerDb::open(&db_path)?;
        trigger_db.rebuild(&skills, embed_fn)?;

        Ok(Self { skills, trigger_db })
    }

    /// Handle a request from the agent.
    pub fn handle(&self, req: &DaemonRequest, embed_fn: &dyn Fn(&str) -> Option<Vec<f32>>) -> Result<DaemonResponse, DaemonError> {
        match req {
            DaemonRequest::Evaluate { prompt, event, files } => {
                self.evaluate(prompt, event, files, embed_fn)
            }
            DaemonRequest::Query { prompt, top_k } => {
                let embedding = embed_fn(prompt).unwrap_or_default();
                let matches = self.trigger_db.query(&embedding, *top_k)?;
                let triggered = matches.into_iter().filter_map(|m| {
                    self.skills.iter().find(|s| s.name == m.skill_name).map(|s| skill_to_triggered(s, &format!("semantic match ({:.2}): {}", 1.0 / (1.0 + m.distance), m.description)))
                }).collect();
                Ok(DaemonResponse { triggered })
            }
            DaemonRequest::Reload | DaemonRequest::Register { .. } => {
                // In a real daemon, this would re-discover skills.
                Ok(DaemonResponse { triggered: Vec::new() })
            }
        }
    }

    fn evaluate(
        &self,
        prompt: &str,
        event: &str,
        files: &[String],
        embed_fn: &dyn Fn(&str) -> Option<Vec<f32>>,
    ) -> Result<DaemonResponse, DaemonError> {
        let mut triggered: Vec<TriggeredSkill> = Vec::new();
        let mut triggered_names: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Tier 1: Rule-based exact triggers
        for skill in &self.skills {
            if self.matches_rules(skill, event, files) {
                triggered_names.insert(skill.name.clone());
                triggered.push(skill_to_triggered(skill, &format!(
                    "rule match: event={}, files={:?}", event, files
                )));
            }
        }

        // Tier 2: Semantic vector triggers
        if let Some(embedding) = embed_fn(prompt) {
            let matches = self.trigger_db.query(&embedding, 10)?;
            for m in matches {
                if !triggered_names.contains(&m.skill_name) {
                    if let Some(skill) = self.skills.iter().find(|s| s.name == m.skill_name) {
                        triggered_names.insert(skill.name.clone());
                        let sim = 1.0 / (1.0 + m.distance);
                        triggered.push(skill_to_triggered(skill, &format!(
                            "semantic match ({:.2}): {}", sim, m.description
                        )));
                    }
                }
            }
        }

        // Sort by priority (descending)
        triggered.sort_by(|a, b| b.priority.cmp(&a.priority));
        Ok(DaemonResponse { triggered })
    }

    fn matches_rules(&self, skill: &Skill, event: &str, files: &[String]) -> bool {
        let event_match = skill.triggers.on_events.is_empty()
            || skill.triggers.on_events.iter().any(|e| e == event);

        let file_match = skill.triggers.file_patterns.is_empty()
            || files.iter().any(|f| {
                skill.triggers.file_patterns.iter().any(|pat| glob_match::glob_match(pat, f))
            });

        // Both must match if both are specified
        let has_events = !skill.triggers.on_events.is_empty();
        let has_patterns = !skill.triggers.file_patterns.is_empty();

        match (has_events, has_patterns) {
            (true, true) => event_match && file_match,
            (true, false) => event_match,
            (false, true) => file_match,
            (false, false) => false, // No rule triggers defined → skip rule tier
        }
    }
}

fn skill_to_triggered(skill: &Skill, reason: &str) -> TriggeredSkill {
    TriggeredSkill {
        skill: skill.name.clone(),
        mode: skill.mode,
        reason: reason.to_string(),
        priority: skill.priority,
        pre_hooks: skill.hooks.pre.clone(),
        post_hooks: skill.hooks.post.clone(),
        prompt_file: skill.context.prompt_file.as_ref().map(|p| p.display().to_string()),
        inject_output: skill.hooks.inject_output,
    }
}
