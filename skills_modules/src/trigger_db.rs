use rusqlite::{params, Connection};
use sqlite_vec::sqlite3_vec_init;
use std::path::Path;
use zerocopy::IntoBytes;

use crate::manifest::Skill;
use crate::types::SkillPool;

#[derive(Debug, thiserror::Error)]
pub enum TriggerDbError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// sqlite-vec backed trigger database for semantic skill matching.
pub struct TriggerDb {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct TriggerMatch {
    pub skill_name: String,
    pub description: String,
    pub distance: f32,
}

impl TriggerDb {
    pub fn open(path: &Path) -> Result<Self, TriggerDbError> {
        std::fs::create_dir_all(path.parent().unwrap_or(Path::new("."))).ok();

        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite3_vec_init as *const (),
            )));
        }

        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS triggers (
                id TEXT PRIMARY KEY,
                skill_name TEXT NOT NULL,
                skill_pool TEXT NOT NULL,
                description TEXT NOT NULL,
                threshold REAL DEFAULT 0.75,
                enabled INTEGER DEFAULT 1
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS triggers_vec USING vec0(
                id TEXT PRIMARY KEY,
                embedding float[384]
            );",
        )?;

        Ok(Self { conn })
    }

    /// Rebuild trigger DB from a set of skills (source of truth is skill.toml).
    pub fn rebuild(&self, skills: &[Skill], embed_fn: &dyn Fn(&str) -> Option<Vec<f32>>) -> Result<(), TriggerDbError> {
        self.conn.execute("DELETE FROM triggers", [])?;
        self.conn.execute("DELETE FROM triggers_vec", [])?;

        for skill in skills {
            let pool_str = match skill.pool {
                SkillPool::System => "system",
                SkillPool::User => "user",
                SkillPool::Project => "project",
            };

            for desc in &skill.triggers.descriptions {
                let id = uuid::Uuid::new_v4().to_string();

                self.conn.execute(
                    "INSERT INTO triggers (id, skill_name, skill_pool, description, threshold, enabled)
                     VALUES (?1, ?2, ?3, ?4, ?5, 1)",
                    params![id, skill.name, pool_str, desc, 0.75],
                )?;

                if let Some(embedding) = embed_fn(desc) {
                    self.conn.execute(
                        "INSERT INTO triggers_vec (id, embedding) VALUES (?1, ?2)",
                        params![id, embedding.as_bytes()],
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Query for matching triggers given a prompt embedding.
    pub fn query(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<TriggerMatch>, TriggerDbError> {
        let mut stmt = self.conn.prepare(
            "SELECT v.id, v.distance
             FROM triggers_vec v
             WHERE v.embedding MATCH ?1
             ORDER BY v.distance
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![query_embedding.as_bytes(), top_k as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f32>(1)?))
        })?;

        let mut matches = Vec::new();
        for row in rows {
            let (id, distance) = row?;
            // Look up trigger metadata
            if let Ok((skill_name, description, threshold, enabled)) = self.conn.query_row(
                "SELECT skill_name, description, threshold, enabled FROM triggers WHERE id = ?1",
                params![id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, f64>(2)?, r.get::<_, bool>(3)?)),
            ) {
                if enabled {
                    // sqlite-vec returns L2 distance; convert to similarity
                    let similarity = 1.0 / (1.0 + distance);
                    if similarity >= threshold as f32 {
                        matches.push(TriggerMatch { skill_name, description, distance });
                    }
                }
            }
        }
        Ok(matches)
    }
}
