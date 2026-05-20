use std::path::Path;

use crate::config::MemoryConfig;
use crate::embedding::Embedder;
use crate::store::{MemoryStore, StoreError};
use crate::types::{MemoryEntry, SearchResult, StoreKind};

#[derive(Debug, thiserror::Error)]
pub enum ManagerError {
    #[error("store: {0}")]
    Store(#[from] StoreError),
    #[error("embedding: {0}")]
    Embedding(#[from] crate::embedding::EmbeddingError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("not initialized — run first-startup flow")]
    NotInitialized,
}

/// Orchestrates the multi-tier memory system.
pub struct MemoryManager {
    config: MemoryConfig,
    embedder: Embedder,
    knowledge: MemoryStore,
    errors: MemoryStore,
    session: Option<MemoryStore>,
    session_buffer: Vec<MemoryEntry>,
    turn_count: usize,
}

impl MemoryManager {
    pub fn open(project_dir: Option<&Path>, session_id: Option<&str>) -> Result<Self, ManagerError> {
        let global_dir = MemoryConfig::global_dir();
        if !global_dir.exists() {
            return Err(ManagerError::NotInitialized);
        }

        let config = MemoryConfig::load(&MemoryConfig::config_path()).unwrap_or_default();
        let embedder = Embedder::new(&config.embedding.model_path)?;
        let knowledge = MemoryStore::open(&global_dir.join("knowledge.db"))?;
        let errors = MemoryStore::open(&global_dir.join("errors.db"))?;

        let session = match (project_dir, session_id) {
            (Some(proj), Some(sid)) => {
                let path = proj.join(".arcana/memory/sessions").join(format!("{sid}.db"));
                Some(MemoryStore::open(&path)?)
            }
            _ => None,
        };

        Ok(Self { config, embedder, knowledge, errors, session, session_buffer: Vec::new(), turn_count: 0 })
    }

    /// First-time initialization of ~/.arcana.
    pub fn initialize() -> Result<(), ManagerError> {
        let global_dir = MemoryConfig::global_dir();
        std::fs::create_dir_all(&global_dir)?;
        std::fs::create_dir_all(global_dir.join("models"))?;

        let config = MemoryConfig::default();
        std::fs::write(MemoryConfig::config_path(), toml::to_string_pretty(&config).unwrap_or_default())?;

        if !global_dir.join("SOUL.md").exists() {
            std::fs::write(global_dir.join("SOUL.md"), DEFAULT_SOUL)?;
        }
        if !global_dir.join("USER.md").exists() {
            std::fs::write(global_dir.join("USER.md"), DEFAULT_USER)?;
        }
        Ok(())
    }

    /// Retrieve relevant memories for a query.
    pub fn retrieve(&self, query: &str) -> Result<RetrievalResult, ManagerError> {
        let embedding = self.embedder.embed(query)?;

        let knowledge = self.knowledge.search_hybrid(query, &embedding, self.config.retrieval.knowledge_top_k)?;
        let errors = self.errors.search_hybrid(query, &embedding, self.config.retrieval.errors_top_k)?;
        let session = match &self.session {
            Some(s) => s.search_hybrid(query, &embedding, self.config.retrieval.session_relevant)?,
            None => Vec::new(),
        };

        Ok(RetrievalResult { knowledge, errors, session })
    }

    /// Record a new memory entry.
    pub fn record(&mut self, text: &str, kind: StoreKind, tags: Vec<String>, session_id: Option<&str>) -> Result<(), ManagerError> {
        let embedding = self.embedder.embed(text)?;
        let entry = MemoryEntry::new(text.to_string(), embedding, session_id.map(|s| s.to_string()), tags);

        match kind {
            StoreKind::Knowledge => { self.knowledge.insert(&entry)?; }
            StoreKind::Errors => { self.errors.insert(&entry)?; }
            StoreKind::Session => { self.session_buffer.push(entry); }
        }
        Ok(())
    }

    /// Called each turn to potentially flush session buffer.
    pub fn on_turn(&mut self) -> Result<(), ManagerError> {
        self.turn_count += 1;
        if self.turn_count % self.config.session.flush_interval_turns == 0 {
            self.flush_session()?;
        }
        Ok(())
    }

    pub fn flush_session(&mut self) -> Result<(), ManagerError> {
        if let Some(ref session) = self.session {
            for entry in self.session_buffer.drain(..) {
                session.insert(&entry)?;
            }
        }
        Ok(())
    }

    pub fn evict(&mut self, kind: StoreKind) -> Result<Vec<MemoryEntry>, ManagerError> {
        let (store, capacity) = match kind {
            StoreKind::Knowledge => (&self.knowledge, self.config.global.knowledge_capacity),
            StoreKind::Errors => (&self.errors, self.config.global.errors_capacity),
            StoreKind::Session => {
                if let Some(ref s) = self.session {
                    return Self::evict_store(s, self.config.session.capacity);
                }
                return Ok(Vec::new());
            }
        };
        Self::evict_store(store, capacity)
    }

    fn evict_store(store: &MemoryStore, capacity: usize) -> Result<Vec<MemoryEntry>, ManagerError> {
        let count = store.count()?;
        if count <= capacity {
            return Ok(Vec::new());
        }
        let victims = store.lowest_activation(count - capacity)?;
        for entry in &victims {
            store.delete(&entry.id)?;
        }
        Ok(victims)
    }

    pub fn record_access(&self, ids: &[String], kind: StoreKind) -> Result<(), ManagerError> {
        let store = match kind {
            StoreKind::Knowledge => &self.knowledge,
            StoreKind::Errors => &self.errors,
            StoreKind::Session => {
                if let Some(ref s) = self.session { s } else { return Ok(()); }
            }
        };
        for id in ids {
            if let Ok(mut entry) = store.get(id) {
                entry.record_access();
                entry.update_activation(self.config.global.eviction_decay_lambda);
                store.update(&entry)?;
            }
        }
        Ok(())
    }

    pub fn end_session(&mut self) -> Result<(), ManagerError> {
        self.flush_session()?;
        self.evict(StoreKind::Session)?;
        self.evict(StoreKind::Knowledge)?;
        self.evict(StoreKind::Errors)?;
        Ok(())
    }

    pub fn store(&self, kind: StoreKind) -> Option<&MemoryStore> {
        match kind {
            StoreKind::Knowledge => Some(&self.knowledge),
            StoreKind::Errors => Some(&self.errors),
            StoreKind::Session => self.session.as_ref(),
        }
    }

    pub fn embedder(&self) -> &Embedder { &self.embedder }
}

#[derive(Debug)]
pub struct RetrievalResult {
    pub knowledge: Vec<SearchResult>,
    pub errors: Vec<SearchResult>,
    pub session: Vec<SearchResult>,
}

const DEFAULT_SOUL: &str = "# SOUL.md — Arcana Agent Personality\n\n## Tone\n- Direct, concise, no filler\n- Match user's technical level\n\n## Preferences\n- Show reasoning before conclusions\n- Use code examples over prose explanations\n\n## Constraints\n- Never apologize unnecessarily\n- Do not repeat information already stated\n";

const DEFAULT_USER: &str = "# USER.md — User Portrait\n\n(To be populated from interactions)\n";
