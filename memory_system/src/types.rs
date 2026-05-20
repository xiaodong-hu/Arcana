use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const EMBEDDING_DIM: usize = 384;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub text: String,
    pub embedding: Vec<f32>,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u64,
    pub activation_score: f64,
    pub source_session: Option<String>,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
}

impl MemoryEntry {
    pub fn new(text: String, embedding: Vec<f32>, source_session: Option<String>, tags: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            text,
            embedding,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            activation_score: 0.0,
            source_session,
            tags,
            metadata: serde_json::Value::Object(Default::default()),
        }
    }

    /// Recompute activation score with exponential decay.
    pub fn update_activation(&mut self, decay_lambda: f64) {
        let age_secs = (Utc::now() - self.last_accessed).num_seconds().max(0) as f64;
        let recency = (-decay_lambda * age_secs).exp();
        self.activation_score = self.access_count as f64 * recency;
    }

    pub fn record_access(&mut self) {
        self.last_accessed = Utc::now();
        self.access_count += 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreKind {
    Knowledge,
    Errors,
    Session,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entry: MemoryEntry,
    pub score: f32,
}
