use crate::database::Db;
use crate::queries::helpers::{MAX_FIELD_LEN, MAX_FILES_COUNT, MAX_SUMMARY_LEN, validate_field};
use anyhow::Result;
use rusqlite::params;
use traz_core::Event;

impl Db {
    /// Insert an event and return its auto-generated ID.
    pub fn insert_event(&self, event: &Event) -> Result<i64> {
        // Input validation
        validate_field("tool", &event.tool, MAX_FIELD_LEN)?;
        validate_field("event_type", &event.event_type, MAX_FIELD_LEN)?;
        validate_field("title", &event.title, MAX_FIELD_LEN)?;

        if let Some(ref summary) = event.summary {
            validate_field("summary", summary, MAX_SUMMARY_LEN)?;
        }
        if let Some(ref files) = event.files {
            anyhow::ensure!(
                files.len() <= MAX_FILES_COUNT,
                "Too many files (max {})",
                MAX_FILES_COUNT
            );
            for f in files {
                validate_field("file path", f, MAX_FIELD_LEN)?;
            }
        }

        let files_json = event
            .files
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let metadata_json = event
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let tags_json = event.tags.as_ref().map(serde_json::to_string).transpose()?;

        let embedding_bytes = if self.config.embeddings_enabled {
            let text = format!(
                "{} {}",
                event.title,
                event.summary.as_deref().unwrap_or_default()
            );
            match traz_embeddings::embed_text(&text) {
                Ok(vec) => {
                    let bytes: Vec<u8> = vec.iter().flat_map(|f| f.to_le_bytes()).collect();
                    Some(bytes)
                }
                Err(e) => {
                    if !traz_embeddings::is_embedding_model_downloaded() {
                        eprintln!(
                            "Warning: Embedding model is not downloaded. Run `traz init --with-embeddings` to generate semantic vectors."
                        );
                    } else {
                        eprintln!(
                            "Warning: Failed to generate event embedding: {}. Run `traz init --with-embeddings` to re-download if corrupted.",
                            e
                        );
                    }
                    None
                }
            }
        } else {
            None
        };

        let conn = self.lock_conn();
        conn.execute(
            "INSERT INTO events (uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                event.uuid,
                event.tool,
                event.event_type,
                event.title,
                event.summary,
                files_json,
                metadata_json,
                tags_json,
                event.session_id,
                event.diff,
                event.timestamp.to_rfc3339(),
            ],
        )?;

        let event_id = conn.last_insert_rowid();

        if let Some(bytes) = embedding_bytes {
            let model_version = "all-MiniLM-L6-v2";
            let created_at = chrono::Utc::now().to_rfc3339();
            if let Err(e) = conn.execute(
                "INSERT INTO event_embeddings (event_id, vector, model_version, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![event_id, bytes, model_version, created_at],
            ) {
                eprintln!("Warning: Failed to insert event embedding: {}", e);
            }
        }

        Ok(event_id)
    }

    /// Delete an event by its ID. Returns true if a row was deleted.
    pub fn delete_event(&self, id: i64) -> Result<bool> {
        let conn = self.lock_conn();
        let affected = conn.execute("DELETE FROM events WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    /// Delete all events that occurred strictly after the given event ID.
    /// Returns the number of events deleted.
    pub fn delete_events_after(&self, id: i64) -> Result<usize> {
        let conn = self.lock_conn();
        let affected = conn.execute("DELETE FROM events WHERE id > ?1", params![id])?;
        Ok(affected)
    }

    /// Compress older events into a single "epoch" event to save context.
    /// Returns a tuple of (number of events compressed, new epoch event ID).
    pub fn compress_events(&self, older_than_days: u32, summary: String) -> Result<(usize, i64)> {
        let mut guard = match self.conn.lock() {
            Ok(g) => g,
            Err(poisoned) => {
                tracing::warn!("Recovered from poisoned mutex lock in compress_events");
                poisoned.into_inner()
            }
        };

        let tx = guard.transaction()?;

        // 1. Find how many events we are compressing.
        let count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM events WHERE datetime(timestamp) < datetime('now', '-' || ?1 || ' days') AND type != 'epoch'",
            params![older_than_days],
            |row| row.get(0)
        )?;

        if count == 0 {
            return Ok((0, 0)); // Nothing to compress
        }

        // 2. Delete the old events.
        tx.execute(
            "DELETE FROM events WHERE datetime(timestamp) < datetime('now', '-' || ?1 || ' days') AND type != 'epoch'",
            params![older_than_days]
        )?;

        // 3. Insert the epoch event.
        let now = chrono::Utc::now();
        let title = format!(
            "Compressed {} events (older than {} days)",
            count, older_than_days
        );

        // We use traz_core::Event just to generate a valid UUID without adding a direct dependency
        let dummy = traz_core::Event::new(
            "traz".into(),
            "epoch".into(),
            title.clone(),
            None,
            None,
            None,
        );
        let uuid = dummy.uuid;

        tx.execute(
            "INSERT INTO events (uuid, tool, type, title, summary, timestamp) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![uuid, "traz", "epoch", title, summary, now.to_rfc3339()],
        )?;

        let epoch_id = tx.last_insert_rowid();

        tx.commit()?;

        Ok((count as usize, epoch_id))
    }

    pub fn backfill_missing_embeddings(&self) -> Result<usize> {
        if !self.config.embeddings_enabled {
            anyhow::bail!("Embeddings are not enabled in the configuration");
        }

        let mut missing_ids = Vec::new();
        {
            let conn = self.lock_conn();
            let mut stmt = conn.prepare(
                "SELECT id FROM events 
                 WHERE id NOT IN (SELECT event_id FROM event_embeddings)",
            )?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                missing_ids.push(id);
            }
        }

        let mut count = 0;
        let model_version = "all-MiniLM-L6-v2";
        let chunk_size = 100;

        for chunk in missing_ids.chunks(chunk_size) {
            let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!(
                "SELECT id, title, summary FROM events WHERE id IN ({})",
                placeholders
            );

            let mut batch = Vec::new();
            {
                let conn = self.lock_conn();
                let mut stmt = conn.prepare(&sql)?;
                let mut rows = stmt.query(rusqlite::params_from_iter(chunk))?;
                while let Some(row) = rows.next()? {
                    let id: i64 = row.get(0)?;
                    let title: String = row.get(1)?;
                    let summary: Option<String> = row.get(2)?;
                    batch.push((id, title, summary));
                }
            }

            let mut embeddings_batch = Vec::new();
            for (id, title, summary) in batch {
                let text = format!("{} {}", title, summary.as_deref().unwrap_or_default());
                match traz_embeddings::embed_text(&text) {
                    Ok(vec) => {
                        let bytes: Vec<u8> = vec.iter().flat_map(|f| f.to_le_bytes()).collect();
                        embeddings_batch.push((id, bytes));
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to generate event embedding for id {}: {}",
                            id, e
                        );
                    }
                }
            }

            if !embeddings_batch.is_empty() {
                let mut guard = match self.conn.lock() {
                    Ok(g) => g,
                    Err(poisoned) => poisoned.into_inner(),
                };
                let tx = guard.transaction()?;
                let created_at = chrono::Utc::now().to_rfc3339();
                for (id, bytes) in embeddings_batch {
                    if let Err(e) = tx.execute(
                        "INSERT INTO event_embeddings (event_id, vector, model_version, created_at) VALUES (?1, ?2, ?3, ?4)",
                        params![id, bytes, model_version, &created_at],
                    ) {
                        eprintln!("Warning: Failed to insert event embedding for id {}: {}", id, e);
                    } else {
                        count += 1;
                    }
                }
                tx.commit()?;
            }
        }

        Ok(count)
    }
}
