use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub global: GlobalConfig,
    pub session: SessionConfig,
    pub project: ProjectConfig,
    pub retrieval: RetrievalConfig,
    pub embedding: EmbeddingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub knowledge_capacity: usize,
    pub errors_capacity: usize,
    pub eviction_decay_lambda: f64,
    pub consolidation_on_evict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub capacity: usize,
    pub flush_interval_turns: usize,
    pub promotion_threshold: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub auto_summarize_docs: bool,
    pub doc_extensions: Vec<String>,
    pub max_project_md_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    pub errors_top_k: usize,
    pub knowledge_top_k: usize,
    pub project_max_tokens: usize,
    pub session_recent: usize,
    pub session_relevant: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub model: String,
    pub model_path: PathBuf,
    pub dimensions: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        Self {
            global: GlobalConfig {
                knowledge_capacity: 10_000,
                errors_capacity: 5_000,
                eviction_decay_lambda: 0.01,
                consolidation_on_evict: true,
            },
            session: SessionConfig {
                capacity: 1_000,
                flush_interval_turns: 5,
                promotion_threshold: 2,
            },
            project: ProjectConfig {
                auto_summarize_docs: true,
                doc_extensions: vec![
                    "md".into(), "ipynb".into(), "tex".into(),
                    "typ".into(), "rst".into(), "txt".into(),
                ],
                max_project_md_lines: 500,
            },
            retrieval: RetrievalConfig {
                errors_top_k: 3,
                knowledge_top_k: 5,
                project_max_tokens: 500,
                session_recent: 5,
                session_relevant: 3,
            },
            embedding: EmbeddingConfig {
                model: "all-MiniLM-L6-v2".into(),
                model_path: home.join(".arcana/models/all-MiniLM-L6-v2.onnx"),
                dimensions: 384,
            },
        }
    }
}

impl MemoryConfig {
    pub fn load(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn global_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".arcana")
    }

    pub fn config_path() -> PathBuf {
        Self::global_dir().join("memory.toml")
    }
}
