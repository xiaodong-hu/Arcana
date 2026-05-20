use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::types::{SkillMode, SkillPool};

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("missing skill.toml in {0}")]
    Missing(String),
}

/// Parsed skill manifest.
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub priority: i32,
    pub mode: SkillMode,
    pub pool: SkillPool,
    pub dir: PathBuf,
    pub context: SkillContext,
    pub triggers: SkillTriggers,
    pub hooks: SkillHooks,
    pub permissions: SkillPermissions,
}

#[derive(Debug, Clone)]
pub struct SkillContext {
    pub prompt_file: Option<PathBuf>,
    pub memory_access: bool,
    pub memory_write: bool,
}

#[derive(Debug, Clone)]
pub struct SkillTriggers {
    pub on_events: Vec<String>,
    pub file_patterns: Vec<String>,
    pub descriptions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SkillHooks {
    pub pre: Vec<String>,
    pub post: Vec<String>,
    pub inject_output: bool,
}

#[derive(Debug, Clone)]
pub struct SkillPermissions {
    pub read_paths: Vec<String>,
    pub write_paths: Vec<String>,
    pub exec_commands: Vec<String>,
}

// Raw TOML structures for deserialization
#[derive(Deserialize)]
struct RawManifest {
    skill: RawSkill,
    context: Option<RawContext>,
    triggers: Option<RawTriggers>,
    hooks: Option<RawHooks>,
    permissions: Option<RawPermissions>,
}

#[derive(Deserialize)]
struct RawSkill {
    name: String,
    description: Option<String>,
    enabled: Option<bool>,
    priority: Option<i32>,
    mode: Option<String>,
}

#[derive(Deserialize)]
struct RawContext {
    prompt_file: Option<String>,
    memory_access: Option<bool>,
    memory_write: Option<bool>,
}

#[derive(Deserialize)]
struct RawTriggers {
    on_events: Option<Vec<String>>,
    file_patterns: Option<Vec<String>>,
    descriptions: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct RawHooks {
    pre: Option<Vec<String>>,
    post: Option<Vec<String>>,
    inject_output: Option<bool>,
}

#[derive(Deserialize)]
struct RawPermissions {
    read_paths: Option<Vec<String>>,
    write_paths: Option<Vec<String>>,
    exec_commands: Option<Vec<String>>,
}

impl Skill {
    /// Parse a skill from its directory (must contain skill.toml).
    pub fn load(dir: &Path, pool: SkillPool) -> Result<Self, ManifestError> {
        let toml_path = dir.join("skill.toml");
        if !toml_path.exists() {
            return Err(ManifestError::Missing(dir.display().to_string()));
        }
        let content = std::fs::read_to_string(&toml_path)?;
        let raw: RawManifest = toml::from_str(&content)?;

        let mode = match raw.skill.mode.as_deref() {
            Some("context") => SkillMode::Context,
            Some("action") => SkillMode::Action,
            _ => SkillMode::Hybrid,
        };

        let ctx = raw.context.unwrap_or(RawContext { prompt_file: None, memory_access: None, memory_write: None });
        let trg = raw.triggers.unwrap_or(RawTriggers { on_events: None, file_patterns: None, descriptions: None });
        let hks = raw.hooks.unwrap_or(RawHooks { pre: None, post: None, inject_output: None });
        let prm = raw.permissions.unwrap_or(RawPermissions { read_paths: None, write_paths: None, exec_commands: None });

        Ok(Skill {
            name: raw.skill.name,
            description: raw.skill.description.unwrap_or_default(),
            enabled: raw.skill.enabled.unwrap_or(true),
            priority: raw.skill.priority.unwrap_or(0),
            mode,
            pool,
            dir: dir.to_path_buf(),
            context: SkillContext {
                prompt_file: ctx.prompt_file.map(|p| dir.join(p)),
                memory_access: ctx.memory_access.unwrap_or(true),
                memory_write: ctx.memory_write.unwrap_or(false),
            },
            triggers: SkillTriggers {
                on_events: trg.on_events.unwrap_or_default(),
                file_patterns: trg.file_patterns.unwrap_or_default(),
                descriptions: trg.descriptions.unwrap_or_default(),
            },
            hooks: SkillHooks {
                pre: hks.pre.unwrap_or_default(),
                post: hks.post.unwrap_or_default(),
                inject_output: hks.inject_output.unwrap_or(true),
            },
            permissions: SkillPermissions {
                read_paths: prm.read_paths.unwrap_or_default(),
                write_paths: prm.write_paths.unwrap_or_default(),
                exec_commands: prm.exec_commands.unwrap_or_default(),
            },
        })
    }
}

/// Discover all skills from the three pools.
pub fn discover_skills(project_dir: Option<&Path>) -> Vec<Skill> {
    let mut skills = Vec::new();
    let global = dirs::home_dir().unwrap_or_default().join(".arcana/skills");

    load_pool(&global.join("system"), SkillPool::System, &mut skills);
    load_pool(&global.join("user"), SkillPool::User, &mut skills);

    if let Some(proj) = project_dir {
        load_pool(&proj.join(".arcana/skills"), SkillPool::Project, &mut skills);
    }

    skills.sort_by(|a, b| b.priority.cmp(&a.priority));
    skills
}

fn load_pool(dir: &Path, pool: SkillPool, skills: &mut Vec<Skill>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            if let Ok(skill) = Skill::load(&entry.path(), pool) {
                if skill.enabled {
                    skills.push(skill);
                }
            }
        }
    }
}
