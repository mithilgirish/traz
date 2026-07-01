use crate::database::Db;
use crate::queries::helpers::{MAX_FIELD_LEN, MAX_FILES_COUNT, MAX_SUMMARY_LEN, validate_field};
use anyhow::Result;
use traz_core::Event;

impl Db {
    /// Insert an event and return its auto-generated ID.
    pub async fn insert_event(&self, event: &Event) -> Result<i64> {
        // Input validation
        validate_field("tool", &event.tool, MAX_FIELD_LEN)?;
        validate_field("event_type", &event.event_type, MAX_FIELD_LEN)?;
        validate_field("title", &event.title, MAX_FIELD_LEN)?;

        if let Some(ref branch) = event.branch_name {
            validate_field("branch_name", branch, MAX_FIELD_LEN)?;
        }
        if let Some(ref agent) = event.agent_id {
            validate_field("agent_id", agent, MAX_FIELD_LEN)?;
        }

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
            match tokio::task::spawn_blocking(move || traz_embeddings::embed_text(&text))
                .await
                .unwrap_or_else(|e| Err(anyhow::anyhow!("Task failed: {}", e)))
            {
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

        // Auto-assign parent_event_id to build the DAG
        let mut final_parent_event_id = event.parent_event_id;
        if final_parent_event_id.is_none() {
            let branch = event.branch_name.as_deref().unwrap_or("default");

            // Try to find the latest event on the same branch
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id FROM events WHERE branch_name = ?1 ORDER BY timestamp DESC LIMIT 1",
                )
                .await?;
            let mut rows = stmt.query(libsql::params![branch]).await?;

            if let Ok(Some(row)) = rows.next().await {
                final_parent_event_id = row.get::<i64>(0).ok();
            } else if branch != "main" && branch != "master" {
                // Fallback to main branch as ancestor if this is a new branch
                let mut stmt_main = self.conn.prepare("SELECT id FROM events WHERE branch_name IN ('main', 'master') ORDER BY timestamp DESC LIMIT 1").await?;
                let mut rows_main = stmt_main.query(libsql::params![]).await?;
                if let Ok(Some(row)) = rows_main.next().await {
                    final_parent_event_id = row.get::<i64>(0).ok();
                }
            }
        }

        self.conn.execute(
            "INSERT INTO events (uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, branch_name, parent_event_id, is_checkpoint, agent_id, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            libsql::params![
                event.uuid.clone(),
                event.tool.clone(),
                event.event_type.clone(),
                event.title.clone(),
                event.summary.clone(),
                files_json,
                metadata_json,
                tags_json,
                event.session_id.clone(),
                event.diff.clone(),
                event.branch_name.clone(),
                final_parent_event_id,
                event.is_checkpoint.unwrap_or(false),
                event.agent_id.clone(),
                event.timestamp.to_rfc3339(),
            ],
        )
        .await?;

        let event_id = self.conn.last_insert_rowid();

        if let Some(bytes) = embedding_bytes {
            let model_version = "all-MiniLM-L6-v2";
            let created_at = chrono::Utc::now().to_rfc3339();
            if let Err(e) = self
                .conn
                .execute(
                    "INSERT INTO event_embeddings (event_id, vector, model_version, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                    libsql::params![event_id, bytes, model_version, created_at],
                )
                .await
            {
                eprintln!("Warning: Failed to insert event embedding: {}", e);
            }
        }

        Ok(event_id)
    }

    /// Delete an event by its ID. Returns true if a row was deleted.
    pub async fn delete_event(&self, id: i64) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM events WHERE id = ?1", libsql::params![id])
            .await?;
        Ok(affected > 0)
    }

    /// Delete all events that occurred strictly after the given event ID.
    /// Returns the number of events deleted.
    pub async fn delete_events_after(&self, id: i64) -> Result<usize> {
        let affected = self
            .conn
            .execute("DELETE FROM events WHERE id > ?1", libsql::params![id])
            .await?;
        Ok(affected as usize)
    }

    /// Mark the current state as a checkpoint by inserting a new checkpoint event.
    pub async fn mark_checkpoint(&self, branch: &str, message: Option<String>) -> Result<i64> {
        let title = message.unwrap_or_else(|| "Memory Checkpoint".to_string());

        let mut event =
            traz_core::Event::new("traz".into(), "checkpoint".into(), title, None, None, None)
                .with_branch(Some(branch.to_string()));
        event.is_checkpoint = Some(true);

        self.insert_event(&event).await
    }

    /// Roll back the branch to its most recent checkpoint.
    pub async fn rollback_to_checkpoint(&self, branch: &str) -> Result<usize> {
        // Find the most recent checkpoint on this branch, ordered by id (consistent with delete boundary)
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id FROM events WHERE branch_name = ?1 AND is_checkpoint = 1 ORDER BY id DESC LIMIT 1",
            )
            .await?;
        let mut rows = stmt.query(libsql::params![branch]).await?;

        if let Some(row) = rows.next().await? {
            let checkpoint_id: i64 = row.get(0)?;

            // Delete all events on this branch that have an id > checkpoint_id
            let affected = self
                .conn
                .execute(
                    "DELETE FROM events WHERE branch_name = ?1 AND id > ?2",
                    libsql::params![branch, checkpoint_id],
                )
                .await?;

            Ok(affected as usize)
        } else {
            anyhow::bail!("No checkpoint found on branch '{}'", branch);
        }
    }

    /// Compress older events into a single "epoch" event to save context.
    /// Returns a tuple of (number of events compressed, new epoch event ID).
    pub async fn compress_events(
        &self,
        older_than_days: u32,
        summary: String,
    ) -> Result<(usize, i64)> {
        let tx = self.conn.transaction().await?;

        // 1. Find how many events we are compressing.
        let count = {
            let mut rows = tx.query(
                "SELECT COUNT(*) FROM events WHERE datetime(timestamp) < datetime('now', '-' || ?1 || ' days') AND type != 'epoch'",
                libsql::params![older_than_days],
            ).await?;
            if let Some(row) = rows.next().await? {
                row.get::<i64>(0)?
            } else {
                0
            }
        };

        if count == 0 {
            return Ok((0, 0)); // Nothing to compress
        }

        // 2. Delete the old events.
        tx.execute(
            "DELETE FROM events WHERE datetime(timestamp) < datetime('now', '-' || ?1 || ' days') AND type != 'epoch'",
            libsql::params![older_than_days]
        ).await?;

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
            "INSERT INTO events (uuid, tool, type, title, summary, branch_name, parent_event_id, is_checkpoint, agent_id, timestamp) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            libsql::params![uuid, "traz", "epoch", title, summary, None::<String>, None::<i64>, false, None::<String>, now.to_rfc3339()],
        )
        .await?;

        let epoch_id = {
            let mut rows = tx.query("SELECT last_insert_rowid()", ()).await?;
            if let Some(row) = rows.next().await? {
                row.get::<i64>(0)?
            } else {
                0
            }
        };

        tx.commit().await?;

        Ok((count as usize, epoch_id))
    }

    /// Rollup a subagent's memory into a single Semantic Summary Event in the parent agent's timeline.
    /// Deletion is scoped to the given branch (or all branches if None).
    pub async fn rollup_agent_memory(
        &self,
        agent_id: &str,
        summary: String,
        branch_name: Option<String>,
    ) -> Result<i64> {
        validate_field("agent_id", agent_id, MAX_FIELD_LEN)?;
        validate_field("summary", &summary, MAX_SUMMARY_LEN)?;
        if let Some(ref b) = branch_name {
            validate_field("branch_name", b, MAX_FIELD_LEN)?;
        }

        let tx = self.conn.transaction().await?;

        // 1. Delete episodic events for the subagent, scoped by branch when provided.
        let count = if let Some(ref branch) = branch_name {
            tx.execute(
                "DELETE FROM events WHERE agent_id = ?1 AND branch_name = ?2",
                libsql::params![agent_id, branch.clone()],
            )
            .await?
        } else {
            tx.execute(
                "DELETE FROM events WHERE agent_id = ?1",
                libsql::params![agent_id],
            )
            .await?
        };

        if count == 0 {
            anyhow::bail!(
                "No events found for agent_id '{}' on the requested branch",
                agent_id
            );
        }

        // 2. Resolve the latest remaining event on this branch as parent.
        let parent_id: Option<i64> = if let Some(ref branch) = branch_name {
            let mut rows = tx
                .query(
                    "SELECT id FROM events WHERE branch_name = ?1 ORDER BY id DESC LIMIT 1",
                    libsql::params![branch.clone()],
                )
                .await?;
            if let Some(row) = rows.next().await? {
                row.get::<i64>(0).ok()
            } else {
                None
            }
        } else {
            None
        };

        // 3. Insert the semantic summary event.
        let title = format!("Subagent Rollup (deleted {} events)", count);
        let now = chrono::Utc::now();
        let dummy = traz_core::Event::new(
            "traz".into(),
            "summary".into(),
            title.clone(),
            None,
            None,
            None,
        );
        let uuid = dummy.uuid;

        let full_text = format!("{} {}", title, summary);

        tx.execute(
            "INSERT INTO events (uuid, tool, type, title, summary, branch_name, parent_event_id, is_checkpoint, agent_id, timestamp) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            libsql::params![uuid, "traz", "summary", title, summary, branch_name, parent_id, false, None::<String>, now.to_rfc3339()],
        )
        .await?;

        let rollup_id = {
            let mut rows = tx.query("SELECT last_insert_rowid()", ()).await?;
            if let Some(row) = rows.next().await? {
                row.get::<i64>(0)?
            } else {
                0
            }
        };

        let embedding_bytes = if self.config.embeddings_enabled {
            match tokio::task::spawn_blocking(move || traz_embeddings::embed_text(&full_text))
                .await
                .unwrap_or_else(|e| Err(anyhow::anyhow!("Task failed: {}", e)))
            {
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

        if let Some(bytes) = embedding_bytes {
            let model_version = "all-MiniLM-L6-v2";
            let created_at = chrono::Utc::now().to_rfc3339();
            if let Err(e) = tx
                .execute(
                    "INSERT INTO event_embeddings (event_id, vector, model_version, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                    libsql::params![rollup_id, bytes, model_version, created_at],
                )
                .await
            {
                eprintln!("Warning: Failed to insert event embedding: {}", e);
            }
        }

        tx.commit().await?;

        Ok(rollup_id)
    }

    pub async fn backfill_missing_embeddings(&self) -> Result<usize> {
        if !self.config.embeddings_enabled {
            anyhow::bail!("Embeddings are not enabled in the configuration");
        }

        let mut missing_ids = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id FROM events 
                 WHERE id NOT IN (SELECT event_id FROM event_embeddings)",
                )
                .await?;
            let mut rows = stmt.query(()).await?;
            while let Some(row) = rows.next().await? {
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
                let mut stmt = self.conn.prepare(&sql).await?;
                let params: Vec<libsql::Value> =
                    chunk.iter().map(|&id| libsql::Value::from(id)).collect();
                let mut rows = stmt.query(params).await?;
                while let Some(row) = rows.next().await? {
                    let id: i64 = row.get(0)?;
                    let title: String = row.get(1)?;
                    let summary: Option<String> = row.get(2)?;
                    batch.push((id, title, summary));
                }
            }

            let mut embeddings_batch = Vec::new();
            for (id, title, summary) in batch {
                let text = format!("{} {}", title, summary.as_deref().unwrap_or_default());
                match tokio::task::spawn_blocking(move || traz_embeddings::embed_text(&text))
                    .await
                    .unwrap_or_else(|e| Err(anyhow::anyhow!("Task failed: {}", e)))
                {
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
                let tx = self.conn.transaction().await?;
                let created_at = chrono::Utc::now().to_rfc3339();
                for (id, bytes) in embeddings_batch {
                    if let Err(e) = tx.execute(
                        "INSERT INTO event_embeddings (event_id, vector, model_version, created_at) VALUES (?1, ?2, ?3, ?4)",
                        libsql::params![id, bytes, model_version, created_at.clone()],
                    ).await {
                        eprintln!("Warning: Failed to insert event embedding for id {}: {}", id, e);
                    } else {
                        count += 1;
                    }
                }
                tx.commit().await?;
            }
        }

        Ok(count)
    }
}
