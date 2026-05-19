use crate::database::Db;
use crate::queries::helpers::{collect_events, row_to_event, MAX_RESULTS};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::params;
use traz_core::Event;

impl Db {
    /// Get a single event by its ID.
    pub fn get_event(&self, id: i64) -> Result<Option<Event>> {
        let conn = self.lock_conn();
        let mut stmt = conn.prepare(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
             FROM events WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], row_to_event)?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// Return the `limit` most recent events, newest first.
    pub fn get_recent_events(&self, limit: u32) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let conn = self.lock_conn();
        let mut stmt = conn.prepare(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
             FROM events ORDER BY timestamp DESC LIMIT ?1",
        )?;
        collect_events(stmt.query_map(params![limit], row_to_event)?)
    }

    /// Full-text-ish search with LIKE wildcard escaping, optional tool filter.
    pub fn search_events(&self, query: &str, tool: Option<&str>, limit: u32) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let escaped = query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let like = format!("%{}%", escaped);

        let mut sql = String::from(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
             FROM events
             WHERE (title   LIKE ?1 ESCAPE '\\'
                OR summary LIKE ?1 ESCAPE '\\'
                OR type    LIKE ?1 ESCAPE '\\'
                OR tool    LIKE ?1 ESCAPE '\\'
                OR files   LIKE ?1 ESCAPE '\\'
                OR tags    LIKE ?1 ESCAPE '\\'
             )"
        );

        if tool.is_some() {
            sql.push_str(" AND tool = ?2");
        }
        sql.push_str(" ORDER BY timestamp DESC LIMIT ?3");

        let conn = self.lock_conn();
        let mut stmt = conn.prepare(&sql)?;
        
        if let Some(tool_val) = tool {
            collect_events(stmt.query_map(params![like, tool_val, limit], row_to_event)?)
        } else {
            drop(stmt);
            let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            params.push(Box::new(like));
            let mut sql = String::from(
                "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
                 FROM events
                 WHERE (title   LIKE ?1 ESCAPE '\\'
                    OR summary LIKE ?1 ESCAPE '\\'
                    OR type    LIKE ?1 ESCAPE '\\'
                    OR tool    LIKE ?1 ESCAPE '\\'
                    OR files   LIKE ?1 ESCAPE '\\'
                    OR tags    LIKE ?1 ESCAPE '\\'
                 )"
            );
            if let Some(t) = tool {
                sql.push_str(" AND tool = ?2");
                params.push(Box::new(t.to_string()));
                sql.push_str(" ORDER BY timestamp DESC LIMIT ?3");
                params.push(Box::new(limit));
            } else {
                sql.push_str(" ORDER BY timestamp DESC LIMIT ?2");
                params.push(Box::new(limit));
            }
            let mut stmt = conn.prepare(&sql)?;
            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            collect_events(stmt.query_map(&*param_refs, row_to_event)?)
        }
    }

    /// Filtered query with optional tool, type, and date predicates.
    pub fn get_filtered_events(
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
            conditions.push("timestamp >= ?".into());
        }
        if until.is_some() {
            conditions.push("timestamp <= ?".into());
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }
        sql.push_str(" ORDER BY timestamp DESC LIMIT ?");

        let conn = self.lock_conn();
        let mut stmt = conn.prepare(&sql)?;

        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(ref t) = tool {
            params.push(Box::new(t.clone()));
        }
        if let Some(ref e) = event_type {
            params.push(Box::new(e.clone()));
        }
        if let Some(ref s) = since {
            params.push(Box::new(s.to_rfc3339()));
        }
        if let Some(ref u) = until {
            params.push(Box::new(u.to_rfc3339()));
        }
        params.push(Box::new(limit));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        collect_events(stmt.query_map(&*param_refs, row_to_event)?)
    }

    /// Return events ordered chronologically (oldest first).
    pub fn get_timeline(&self, limit: u32) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let conn = self.lock_conn();
        let mut stmt = conn.prepare(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
             FROM events ORDER BY timestamp ASC LIMIT ?1",
        )?;
        collect_events(stmt.query_map(params![limit], row_to_event)?)
    }

    /// Get a single event by its UUID.
    pub fn get_event_by_uuid(&self, uuid: &str) -> Result<Option<Event>> {
        let conn = self.lock_conn();
        let mut stmt = conn.prepare(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
             FROM events WHERE uuid = ?1",
        )?;
        let mut rows = stmt.query_map(params![uuid], row_to_event)?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// Get events for a specific session.
    pub fn get_session_events(&self, session_id: &str, limit: u32) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let conn = self.lock_conn();
        let mut stmt = conn.prepare(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, diff, timestamp, created_at
             FROM events WHERE session_id = ?1 ORDER BY timestamp DESC LIMIT ?2",
        )?;
        collect_events(stmt.query_map(params![session_id, limit], row_to_event)?)
    }

    /// Generate a structured context summary for AI agents.
    pub fn get_context_summary(&self, limit: u32) -> Result<String> {
        let total = self.count_events()?;
        let stats = self.get_stats()?;
        let recent = self.get_recent_events(limit)?;

        let mut ctx = String::new();
        ctx.push_str("# traz — Engineering Context Summary\n\n");

        // Stats
        ctx.push_str(&format!("**Total events:** {}\n\n", total));

        if !stats.is_empty() {
            ctx.push_str("## Tools Used\n");
            for (tool, count) in &stats {
                ctx.push_str(&format!("- **{}**: {} events\n", tool, count));
            }
            ctx.push('\n');
        }

        // Recent events
        if !recent.is_empty() {
            ctx.push_str(&format!("## Recent Activity (last {})\n\n", recent.len()));
            for event in &recent {
                let ts = event.timestamp.format("%Y-%m-%d %H:%M UTC");
                ctx.push_str(&format!(
                    "### {} [{}] — {}\n",
                    event.title, event.tool, ts
                ));
                ctx.push_str(&format!("- **Type:** {}\n", event.event_type));

                if let Some(ref summary) = event.summary {
                    ctx.push_str(&format!("- **Summary:** {}\n", summary.lines().next().unwrap_or(summary)));
                }
                if let Some(ref files) = event.files {
                    if !files.is_empty() {
                        ctx.push_str(&format!("- **Files:** {}\n", files.join(", ")));
                    }
                }
                if let Some(ref tags) = event.tags {
                    if !tags.is_empty() {
                        ctx.push_str(&format!("- **Tags:** {}\n", tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" ")));
                    }
                }
                if event.diff.is_some() {
                    ctx.push_str("- **Has diff:** yes\n");
                }
                ctx.push('\n');
            }
        }

        Ok(ctx)
    }
}
