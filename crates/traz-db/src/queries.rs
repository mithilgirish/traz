use crate::database::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{Result as SqliteResult, params};
use traz_core::Event;

/// Maximum allowed length for text fields (tool, type, title).
const MAX_FIELD_LEN: usize = 500;

/// Maximum allowed length for summary text.
const MAX_SUMMARY_LEN: usize = 10_000;

/// Maximum number of file entries per event.
const MAX_FILES_COUNT: usize = 100;

/// Hard upper limit for query results to prevent memory exhaustion.
const MAX_RESULTS: u32 = 1000;

impl Db {
    // ── Write ───────────────────────────────────────────────────────

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

        let conn = self.lock_conn();
        conn.execute(
            "INSERT INTO events (uuid, tool, type, title, summary, files, metadata, tags, session_id, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
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
                event.timestamp.to_rfc3339(),
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Delete an event by its ID. Returns true if a row was deleted.
    pub fn delete_event(&self, id: i64) -> Result<bool> {
        let conn = self.lock_conn();
        let affected = conn.execute("DELETE FROM events WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // ── Read ────────────────────────────────────────────────────────

    /// Return the `limit` most recent events, newest first.
    pub fn get_recent_events(&self, limit: u32) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let conn = self.lock_conn();
        let mut stmt = conn.prepare(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, timestamp, created_at
             FROM events ORDER BY timestamp DESC LIMIT ?1",
        )?;
        collect_events(stmt.query_map(params![limit], row_to_event)?)
    }

    /// Full-text-ish search with LIKE wildcard escaping.
    pub fn search_events(&self, query: &str, limit: u32) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let escaped = query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let like = format!("%{}%", escaped);

        let conn = self.lock_conn();
        let mut stmt = conn.prepare(
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, timestamp, created_at
             FROM events
             WHERE title   LIKE ?1 ESCAPE '\\'
                OR summary LIKE ?1 ESCAPE '\\'
                OR type    LIKE ?1 ESCAPE '\\'
                OR tool    LIKE ?1 ESCAPE '\\'
                OR files   LIKE ?1 ESCAPE '\\'
                OR tags    LIKE ?1 ESCAPE '\\'
             ORDER BY timestamp DESC LIMIT ?2",
        )?;
        collect_events(stmt.query_map(params![like, limit], row_to_event)?)
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
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, timestamp, created_at FROM events",
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
            "SELECT id, uuid, tool, type, title, summary, files, metadata, tags, session_id, timestamp, created_at
             FROM events ORDER BY timestamp ASC LIMIT ?1",
        )?;
        collect_events(stmt.query_map(params![limit], row_to_event)?)
    }

    /// Return aggregate counts grouped by tool.
    pub fn get_stats(&self) -> Result<Vec<(String, i64)>> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT tool, COUNT(*) as cnt FROM events GROUP BY tool ORDER BY cnt DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Return the total number of events.
    pub fn count_events(&self) -> Result<i64> {
        let conn = self.lock_conn();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(count)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn validate_field(name: &str, value: &str, max_len: usize) -> Result<()> {
    anyhow::ensure!(!value.trim().is_empty(), "{} must not be empty", name);
    anyhow::ensure!(
        value.len() <= max_len,
        "{} exceeds maximum length of {} bytes",
        name,
        max_len
    );
    Ok(())
}

fn collect_events(iter: impl Iterator<Item = SqliteResult<Event>>) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    for event in iter {
        events.push(event?);
    }
    Ok(events)
}

fn row_to_event(row: &rusqlite::Row) -> SqliteResult<Event> {
    let files_str: Option<String> = row.get(6)?;
    let files = match files_str {
        Some(s) => serde_json::from_str(&s).unwrap_or(None),
        None => None,
    };

    let metadata_str: Option<String> = row.get(7)?;
    let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

    let tags_str: Option<String> = row.get(8)?;
    let tags: Option<Vec<String>> = tags_str.and_then(|s| serde_json::from_str(&s).ok());

    let session_id: Option<String> = row.get(9)?;

    let timestamp_str: String = row.get(10)?;
    let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let created_at_str: Option<String> = row.get(11)?;
    let created_at = created_at_str.and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .ok()
    });

    Ok(Event {
        id: row.get(0)?,
        uuid: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        tool: row.get(2)?,
        event_type: row.get(3)?,
        title: row.get(4)?,
        summary: row.get(5)?,
        files,
        metadata,
        tags,
        session_id,
        timestamp,
        created_at,
    })
}

// Extension to Db for tests
#[cfg(test)]
impl Db {
    fn migrate_for_test(&self) -> Result<()> {
        let conn = self.lock_conn();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid        TEXT,
                tool        TEXT    NOT NULL,
                type        TEXT    NOT NULL,
                title       TEXT    NOT NULL,
                summary     TEXT,
                files       TEXT,
                metadata    TEXT,
                tags        TEXT,
                session_id  TEXT,
                timestamp   TEXT    NOT NULL,
                created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );",
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::path::PathBuf;
    use std::sync::Mutex;

    #[test]
    fn test_db_basic_operations() {
        let conn = Connection::open_in_memory().unwrap();
        let db = Db {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
        };
        // Use a private method for migrations in tests
        db.migrate_for_test().unwrap();

        let event = Event::new(
            "test_tool".to_string(),
            "feature".to_string(),
            "Test Event".to_string(),
            Some("Testing the db".to_string()),
            None,
            None,
        );

        let id = db.insert_event(&event).expect("Failed to insert event");
        assert!(id > 0);

        let events = db.get_recent_events(10).expect("Failed to get events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Test Event");
    }

    #[test]
    fn test_search() {
        let conn = Connection::open_in_memory().unwrap();
        let db = Db {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
        };
        db.migrate_for_test().unwrap();

        let e1 = Event::new("t1".into(), "f1".into(), "Find me".into(), None, None, None);
        let e2 = Event::new("t2".into(), "f2".into(), "Hide me".into(), None, None, None);

        db.insert_event(&e1).unwrap();
        db.insert_event(&e2).unwrap();

        let results = db.search_events("Find", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Find me");
    }
}
