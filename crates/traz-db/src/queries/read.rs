use crate::database::Db;
use crate::queries::helpers::{MAX_RESULTS, collect_events, row_to_event};
use anyhow::Result;
use chrono::{DateTime, Utc};
use traz_core::Event;

#[derive(Default, Debug, Clone)]
pub struct SearchFilters<'a> {
    pub tool: Option<&'a str>,
    pub event_type: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub since: Option<DateTime<Utc>>,
}

impl Db {
    /// Get a single event by its ID.
    pub async fn get_event(&self, id: i64) -> Result<Option<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
             FROM events WHERE id = ?1",
        ).await?;
        let mut rows = stmt.query(libsql::params![id]).await?;
        if let Some(row) = rows.next().await? {
            Ok(Some(row_to_event(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Return the `limit` most recent events, newest first.
    pub async fn get_recent_events(&self, limit: u32) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let mut stmt = self.conn.prepare(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
             FROM events ORDER BY timestamp DESC LIMIT ?1",
        ).await?;
        let rows = stmt.query(libsql::params![limit]).await?;
        collect_events(rows).await
    }

    /// Full-text-ish search with LIKE wildcard escaping, optional tool filter.
    pub async fn search_events(
        &self,
        query: &str,
        filters: &SearchFilters<'_>,
        limit: u32,
    ) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let terms: Vec<&str> = query.split_whitespace().collect();

        let mut sql = String::from(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
             FROM events
             WHERE 1=1"
        );

        let mut params: Vec<libsql::Value> = Vec::new();
        let mut param_idx = 1;

        if !terms.is_empty() {
            for term in terms {
                let escaped = term
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_");
                let like = format!("%{}%", escaped);

                sql.push_str(&format!(
                    " AND (title LIKE ?{idx} ESCAPE '\\'
                       OR summary LIKE ?{idx} ESCAPE '\\'
                       OR type LIKE ?{idx} ESCAPE '\\'
                       OR tool LIKE ?{idx} ESCAPE '\\'
                       OR files LIKE ?{idx} ESCAPE '\\'
                       OR tags LIKE ?{idx} ESCAPE '\\')",
                    idx = param_idx
                ));
                params.push(libsql::Value::from(like));
                param_idx += 1;
            }
        }

        if let Some(t) = filters.tool {
            sql.push_str(&format!(" AND tool = ?{}", param_idx));
            params.push(libsql::Value::from(t.to_string()));
            param_idx += 1;
        }

        if let Some(et) = filters.event_type {
            sql.push_str(&format!(" AND type = ?{}", param_idx));
            params.push(libsql::Value::from(et.to_string()));
            param_idx += 1;
        }

        if let Some(tag) = filters.tag {
            let escaped = tag
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            let like = format!("%\"{}\"%", escaped);
            sql.push_str(&format!(" AND tags LIKE ?{} ESCAPE '\\'", param_idx));
            params.push(libsql::Value::from(like));
            param_idx += 1;
        }

        if let Some(since) = filters.since {
            sql.push_str(&format!(
                " AND datetime(timestamp) >= datetime(?{})",
                param_idx
            ));
            params.push(libsql::Value::from(since.to_rfc3339()));
            param_idx += 1;
        }

        sql.push_str(&format!(" ORDER BY timestamp DESC LIMIT ?{}", param_idx));
        params.push(libsql::Value::from(limit));

        let mut stmt = self.conn.prepare(&sql).await?;
        let rows = stmt.query(params).await?;
        collect_events(rows).await
    }

    /// Filtered query with optional tool, type, and date predicates.
    pub async fn get_filtered_events(
        &self,
        limit: u32,
        tool: Option<String>,
        event_type: Option<String>,
        since: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
    ) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let mut sql = String::from(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at FROM events",
        );
        let mut conditions: Vec<String> = Vec::new();

        if tool.is_some() {
            conditions.push("tool = ?".into());
        }
        if event_type.is_some() {
            conditions.push("type = ?".into());
        }
        if since.is_some() {
            conditions.push("datetime(timestamp) >= datetime(?)".into());
        }
        if until.is_some() {
            conditions.push("datetime(timestamp) <= datetime(?)".into());
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }
        sql.push_str(" ORDER BY timestamp DESC LIMIT ?");

        let mut params: Vec<libsql::Value> = Vec::new();
        if let Some(ref t) = tool {
            params.push(libsql::Value::from(t.clone()));
        }
        if let Some(ref e) = event_type {
            params.push(libsql::Value::from(e.clone()));
        }
        if let Some(ref s) = since {
            params.push(libsql::Value::from(s.to_rfc3339()));
        }
        if let Some(ref u) = until {
            params.push(libsql::Value::from(u.to_rfc3339()));
        }
        params.push(libsql::Value::from(limit));

        let mut stmt = self.conn.prepare(&sql).await?;
        let rows = stmt.query(params).await?;
        collect_events(rows).await
    }

    /// Return events ordered chronologically (oldest first).
    pub async fn get_timeline(&self, limit: u32) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let mut stmt = self.conn.prepare(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
             FROM events ORDER BY timestamp ASC LIMIT ?1",
        ).await?;
        let rows = stmt.query(libsql::params![limit]).await?;
        collect_events(rows).await
    }

    /// Get a single event by its UUID.
    pub async fn get_event_by_uuid(&self, uuid: &str) -> Result<Option<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
             FROM events WHERE uuid = ?1",
        ).await?;
        let mut rows = stmt.query(libsql::params![uuid]).await?;
        if let Some(row) = rows.next().await? {
            Ok(Some(row_to_event(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Get events for a specific session.
    pub async fn get_session_events(&self, session_id: &str, limit: u32) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let mut stmt = self.conn.prepare(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
             FROM events WHERE session_id = ?1 ORDER BY timestamp DESC LIMIT ?2",
        ).await?;
        let rows = stmt.query(libsql::params![session_id, limit]).await?;
        collect_events(rows).await
    }

    /// Generate a structured context summary for AI agents.
    pub async fn get_context_summary(&self, query: Option<&str>, limit: u32) -> Result<String> {
        // Backward-compatible wrapper: markdown format, unlimited budget, no dedup.
        self.get_context_optimized(query, limit, traz_core::OutputFormat::Markdown, None, false)
            .await
    }

    /// Generate a token-optimized context summary for AI agents.
    ///
    /// Supports:
    /// - `format`: Markdown (human-readable) or Dense (AI-optimized, ~50-70% fewer tokens)
    /// - `max_tokens`: Optional token budget — output is truncated to fit
    /// - `deduplicate`: Merge near-duplicate events to save tokens
    pub async fn get_context_optimized(
        &self,
        query: Option<&str>,
        limit: u32,
        format: traz_core::OutputFormat,
        max_tokens: Option<usize>,
        deduplicate: bool,
    ) -> Result<String> {
        let total = self.count_events().await?;
        let stats = self.get_stats().await?;

        let mut budget = match max_tokens {
            Some(n) => traz_core::TokenBudget::new(n),
            None => traz_core::TokenBudget::unlimited(),
        };

        let mut ctx = String::new();

        // ── Header ──────────────────────────────────────────────
        let header = match format {
            traz_core::OutputFormat::Markdown => {
                format!("# traz — Engineering Context Summary\n\n**Total events:** {total}\n\n")
            }
            traz_core::OutputFormat::Dense => {
                format!("traz|events:{total}\n")
            }
        };
        if budget.would_fit(&header) {
            budget.consume(&header);
            ctx.push_str(&header);
        }

        // ── Tool stats ──────────────────────────────────────────
        if !stats.is_empty() {
            let stats_block = match format {
                traz_core::OutputFormat::Markdown => {
                    let mut s = String::from("## Tools Used\n");
                    for (tool, count) in &stats {
                        s.push_str(&format!("- **{}**: {} events\n", tool, count));
                    }
                    s.push('\n');
                    s
                }
                traz_core::OutputFormat::Dense => {
                    let tools: Vec<String> = stats
                        .iter()
                        .map(|(tool, count)| format!("{tool}:{count}"))
                        .collect();
                    format!("tools|{}\n", tools.join(","))
                }
            };
            if budget.would_fit(&stats_block) {
                budget.consume(&stats_block);
                ctx.push_str(&stats_block);
            }
        }

        // ── Fetch events ────────────────────────────────────────
        let is_rag = query.is_some();
        let events = if let Some(q) = query {
            let search_results = self
                .hybrid_search(q, &SearchFilters::default(), limit)
                .await?;
            search_results.into_iter().map(|(e, _)| e).collect()
        } else {
            self.get_recent_events(limit).await?
        };

        // ── Build context with optimizations ────────────────────
        let section_header = if is_rag {
            Some(format!(
                "## Relevant Context (RAG, {} results)",
                events.len()
            ))
        } else {
            Some(format!("## Recent Activity (last {})", events.len()))
        };

        let optimized = traz_core::build_optimized_context(
            events,
            format,
            &mut budget,
            deduplicate,
            section_header.as_deref(),
        );

        ctx.push_str(&optimized);

        // ── Budget usage footer (dense only) ────────────────────
        if matches!(format, traz_core::OutputFormat::Dense) && !budget.is_unlimited() {
            let footer = format!(
                "---budget|used:{}|max:{}\n",
                budget.max_tokens - budget.remaining(),
                budget.max_tokens
            );
            ctx.push_str(&footer);
        }

        Ok(ctx)
    }

    /// Optimized semantic search using one-pass join scanning and f32 cosine similarities.
    pub async fn semantic_search(&self, query: &str, limit: usize) -> Result<Vec<(Event, f32)>> {
        // TODO v0.2: use sqlite-vec extension for ANN search

        let text = query.to_string();
        let query_vec = match tokio::task::spawn_blocking(move || {
            traz_embeddings::embed_text(&text)
        })
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("Task failed: {}", e)))
        {
            Ok(vec) => vec,
            Err(e) => {
                if !traz_embeddings::is_embedding_model_downloaded() {
                    anyhow::bail!(
                        "Embedding model is not downloaded.\nRun `traz init --with-embeddings` to enable semantic search."
                    );
                } else {
                    anyhow::bail!(
                        "Failed to generate query embedding: {}. If the model files are corrupted, run `traz init --with-embeddings` to download them again.",
                        e
                    );
                }
            }
        };

        let mut top_matches = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT event_id, vector FROM event_embeddings")
                .await?;
            let mut rows = stmt.query(()).await?;

            while let Some(row) = rows.next().await? {
                let event_id: i64 = row.get(0)?;
                let vector_bytes: Vec<u8> = row.get(1)?;
                let event_vec: Vec<f32> = vector_bytes
                    .chunks(4)
                    .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
                    .collect();

                let similarity = traz_embeddings::cosine_similarity(&query_vec, &event_vec);
                let similarity = similarity.clamp(0.0, 1.0);
                top_matches.push((event_id, similarity));
            }
        }

        // Sort by similarity descending
        top_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        // Take top `limit`
        top_matches.truncate(limit);

        if top_matches.is_empty() {
            return Ok(Vec::new());
        }

        let ids: Vec<String> = top_matches.iter().map(|(id, _)| id.to_string()).collect();
        let placeholders = ids.join(",");
        let sql = format!(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at 
             FROM events WHERE id IN ({})",
            placeholders
        );

        let mut stmt = self.conn.prepare(&sql).await?;
        let mut rows = stmt.query(()).await?;
        let mut events_map = std::collections::HashMap::new();

        while let Some(row) = rows.next().await? {
            let event = row_to_event(&row)?;
            events_map.insert(event.id, event);
        }

        let mut results = Vec::new();
        for (id, similarity) in top_matches {
            if let Some(event) = events_map.remove(&Some(id)) {
                results.push((event, similarity));
            }
        }

        Ok(results)
    }

    /// Combines keyword search and semantic search using Reciprocal Rank Fusion (RRF).
    pub async fn hybrid_search(
        &self,
        query: &str,
        filters: &SearchFilters<'_>,
        limit: u32,
    ) -> Result<Vec<(Event, f32)>> {
        let mut results = std::collections::HashMap::new();
        let rrf_k = 60.0;

        // 1. Keyword search (Sparse)
        if let Ok(keyword_events) = self.search_events(query, filters, limit).await {
            for (rank, event) in (1..).zip(keyword_events) {
                let rrf_score = 1.0 / (rrf_k + rank as f32);
                results.insert(event.id, (event, rrf_score));
            }
        }

        // 2. Semantic search (Dense)
        if self.config.embeddings_enabled {
            // Fetch more candidates since we might filter some out
            let fetch_limit = (limit * 3).max(100);
            if let Ok(sem_events) = self.semantic_search(query, fetch_limit as usize).await {
                let mut rank = 1;
                for (event, similarity) in sem_events {
                    if similarity < 0.3 {
                        continue;
                    }

                    if let Some(t) = filters.tool
                        && !event.tool.eq_ignore_ascii_case(t)
                    {
                        continue;
                    }
                    if let Some(et) = filters.event_type
                        && !event.event_type.eq_ignore_ascii_case(et)
                    {
                        continue;
                    }
                    if let Some(tag) = filters.tag {
                        if let Some(tags) = &event.tags {
                            if !tags.iter().any(|t_str| t_str.eq_ignore_ascii_case(tag)) {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                    if let Some(since) = filters.since
                        && event.timestamp < since
                    {
                        continue;
                    }

                    let rrf_score = 1.0 / (rrf_k + rank as f32);
                    if let std::collections::hash_map::Entry::Occupied(mut entry) =
                        results.entry(event.id)
                    {
                        entry.get_mut().1 += rrf_score;
                    } else {
                        results.insert(event.id, (event, rrf_score));
                    }
                    rank += 1;
                }
            }
        }

        let mut all_results: Vec<(Event, f32)> = results.into_values().collect();

        // Sort by blended RRF score descending, then timestamp descending
        all_results.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.0.timestamp.cmp(&a.0.timestamp))
        });

        all_results.truncate(limit as usize);
        Ok(all_results)
    }
}
