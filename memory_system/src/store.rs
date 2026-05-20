use rusqlite::{params, Connection};
use sqlite_vec::sqlite3_vec_init;
use std::path::Path;
use zerocopy::IntoBytes;

use crate::types::{MemoryEntry, SearchResult};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("entry not found: {0}")]
    NotFound(String),
}

/// SQLite-backed memory store with vec0 (KNN) + FTS5 (full-text).
pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        std::fs::create_dir_all(path.parent().unwrap_or(Path::new("."))).ok();

        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite3_vec_init as *const (),
            )));
        }

        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                text TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_accessed TEXT NOT NULL,
                access_count INTEGER DEFAULT 0,
                activation_score REAL DEFAULT 0.0,
                source_session TEXT,
                tags TEXT,
                metadata TEXT
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS memories_vec USING vec0(
                id TEXT PRIMARY KEY,
                embedding float[384]
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                id UNINDEXED,
                text,
                tags
            );",
        )?;

        Ok(Self { conn })
    }

    pub fn insert(&self, entry: &MemoryEntry) -> Result<(), StoreError> {
        let tags_json = serde_json::to_string(&entry.tags)?;
        let metadata_json = serde_json::to_string(&entry.metadata)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO memories (id, text, created_at, last_accessed, access_count, activation_score, source_session, tags, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.id,
                entry.text,
                entry.created_at.to_rfc3339(),
                entry.last_accessed.to_rfc3339(),
                entry.access_count,
                entry.activation_score,
                entry.source_session,
                tags_json,
                metadata_json,
            ],
        )?;

        self.conn.execute(
            "INSERT OR REPLACE INTO memories_vec (id, embedding) VALUES (?1, ?2)",
            params![entry.id, entry.embedding.as_bytes()],
        )?;

        self.conn.execute(
            "INSERT OR REPLACE INTO memories_fts (id, text, tags) VALUES (?1, ?2, ?3)",
            params![entry.id, entry.text, entry.tags.join(" ")],
        )?;

        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), StoreError> {
        self.conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        self.conn.execute("DELETE FROM memories_vec WHERE id = ?1", params![id])?;
        self.conn.execute("DELETE FROM memories_fts WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update(&self, entry: &MemoryEntry) -> Result<(), StoreError> {
        self.insert(entry) // INSERT OR REPLACE handles update
    }

    /// KNN vector search via sqlite-vec.
    pub fn search_vector(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<SearchResult>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT v.id, v.distance
             FROM memories_vec v
             WHERE v.embedding MATCH ?1
             ORDER BY v.distance
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![query_embedding.as_bytes(), top_k as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f32>(1)?))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (id, distance) = row?;
            if let Ok(entry) = self.get(&id) {
                // Convert distance to similarity score (lower distance = higher score)
                let score = 1.0 / (1.0 + distance);
                results.push(SearchResult { entry, score });
            }
        }
        Ok(results)
    }

    /// Full-text BM25 search via FTS5.
    pub fn search_text(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, rank FROM memories_fts WHERE memories_fts MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![query, top_k as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (id, rank) = row?;
            if let Ok(entry) = self.get(&id) {
                let score = (-rank) as f32; // FTS5 rank is negative (lower = better)
                results.push(SearchResult { entry, score });
            }
        }
        Ok(results)
    }

    /// Hybrid search: RRF fusion of vector + text results.
    pub fn search_hybrid(&self, query: &str, query_embedding: &[f32], top_k: usize) -> Result<Vec<SearchResult>, StoreError> {
        let vec_results = self.search_vector(query_embedding, top_k * 2)?;
        let text_results = self.search_text(query, top_k * 2)?;

        let k = 60.0f32;
        let mut scores: std::collections::HashMap<String, (f32, MemoryEntry)> = std::collections::HashMap::new();

        for (rank, r) in vec_results.into_iter().enumerate() {
            let rrf = 1.0 / (k + rank as f32 + 1.0);
            scores.entry(r.entry.id.clone()).or_insert((0.0, r.entry)).0 += rrf;
        }
        for (rank, r) in text_results.into_iter().enumerate() {
            let rrf = 1.0 / (k + rank as f32 + 1.0);
            let e = scores.entry(r.entry.id.clone()).or_insert((0.0, r.entry));
            e.0 += rrf;
        }

        let mut results: Vec<SearchResult> = scores.into_values()
            .map(|(score, entry)| SearchResult { entry, score })
            .collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        Ok(results)
    }

    pub fn get(&self, id: &str) -> Result<MemoryEntry, StoreError> {
        self.conn.query_row(
            "SELECT id, text, created_at, last_accessed, access_count, activation_score, source_session, tags, metadata
             FROM memories WHERE id = ?1",
            params![id],
            |row| Ok(row_to_entry(row)),
        )?.ok_or_else(|| StoreError::NotFound(id.to_string()))
    }

    pub fn list_all(&self) -> Result<Vec<MemoryEntry>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, created_at, last_accessed, access_count, activation_score, source_session, tags, metadata
             FROM memories ORDER BY activation_score DESC",
        )?;
        let rows = stmt.query_map([], |row| Ok(row_to_entry(row)))?;
        let mut entries = Vec::new();
        for row in rows {
            if let Some(entry) = row? {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    pub fn count(&self) -> Result<usize, StoreError> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM memories", [], |r| r.get::<_, usize>(0))?)
    }

    pub fn lowest_activation(&self, count: usize) -> Result<Vec<MemoryEntry>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, created_at, last_accessed, access_count, activation_score, source_session, tags, metadata
             FROM memories ORDER BY activation_score ASC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![count as i64], |row| Ok(row_to_entry(row)))?;
        let mut entries = Vec::new();
        for row in rows {
            if let Some(entry) = row? {
                entries.push(entry);
            }
        }
        Ok(entries)
    }
}

fn row_to_entry(row: &rusqlite::Row) -> Option<MemoryEntry> {
    let id: String = row.get(0).ok()?;
    let text: String = row.get(1).ok()?;
    let created_str: String = row.get(2).ok()?;
    let accessed_str: String = row.get(3).ok()?;
    let access_count: u64 = row.get(4).ok()?;
    let activation_score: f64 = row.get(5).ok()?;
    let source_session: Option<String> = row.get(6).ok()?;
    let tags_json: String = row.get::<_, String>(7).unwrap_or_else(|_| "[]".to_string());
    let metadata_json: String = row.get::<_, String>(8).unwrap_or_else(|_| "{}".to_string());

    let created_at = chrono::DateTime::parse_from_rfc3339(&created_str).ok()?.to_utc();
    let last_accessed = chrono::DateTime::parse_from_rfc3339(&accessed_str).ok()?.to_utc();
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let metadata: serde_json::Value = serde_json::from_str(&metadata_json).unwrap_or_default();

    Some(MemoryEntry {
        id,
        text,
        embedding: Vec::new(), // Not loaded from metadata table; loaded on demand from vec table
        created_at,
        last_accessed,
        access_count,
        activation_score,
        source_session,
        tags,
        metadata,
    })
}
