use crate::models::Event;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Maximum allowed length for text fields (tool, type, title).
const MAX_FIELD_LEN: usize = 500;

/// Maximum allowed length for summary text.
const MAX_SUMMARY_LEN: usize = 10_000;

/// Maximum number of file entries per event.
const MAX_FILES_COUNT: usize = 100;

/// Hard upper limit for query results to prevent memory exhaustion.
const MAX_RESULTS: u32 = 1000;

/// Database abstraction over a local SQLite store.
///
/// All event persistence flows through this struct. The inner connection
/// is wrapped in a `Mutex` so the `Db` can be shared safely across the
/// async Axum handlers and the MCP server.
pub struct Db {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Db {
    /// Open (or create) the database at `db_path` and run migrations.
    pub fn new(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        let conn = Connection::open(db_path).context("Failed to open SQLite database")?;

        // Tune SQLite for single-user, local-first workloads
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous  = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )
        .context("Failed to set SQLite pragmas")?;

        let db = Self {
            conn: Mutex::new(conn),
            path: db_path.to_path_buf(),
        };
        db.migrate().context("Failed to run database migrations")?;

        Ok(db)
    }

    /// Returns the filesystem path of the database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    // ── Migrations ──────────────────────────────────────────────────

    fn migrate(&self) -> SqliteResult<()> {
        let conn = self.lock_conn();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                tool       TEXT    NOT NULL,
                type       TEXT    NOT NULL,
                title      TEXT    NOT NULL,
                summary    TEXT,
                files      TEXT,
                tags       TEXT,
                timestamp  TEXT    NOT NULL,
                created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );

            CREATE INDEX IF NOT EXISTS idx_events_tool      ON events(tool);
            CREATE INDEX IF NOT EXISTS idx_events_type       ON events(type);
            CREATE INDEX IF NOT EXISTS idx_events_timestamp  ON events(timestamp);",
        )?;

        // Add tags column if upgrading from an older schema
        let has_tags: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('events') WHERE name='tags'")?
            .query_row([], |row| row.get::<_, i64>(0))
            .map(|n| n > 0)
            .unwrap_or(false);

        if !has_tags {
            let _ = conn.execute("ALTER TABLE events ADD COLUMN tags TEXT", []);
        }

        Ok(())
    }

    // ── Write ───────────────────────────────────────────────────────

    /// Insert an event and return its auto-generated ID.
    ///
    /// Validates all field lengths before writing.
    pub fn insert_event(&self, event: &Event) -> Result<i64> {
        // Input validation
        Self::validate_field("tool", &event.tool, MAX_FIELD_LEN)?;
        Self::validate_field("event_type", &event.event_type, MAX_FIELD_LEN)?;
        Self::validate_field("title", &event.title, MAX_FIELD_LEN)?;

        if let Some(ref summary) = event.summary {
            Self::validate_field("summary", summary, MAX_SUMMARY_LEN)?;
        }
        if let Some(ref files) = event.files {
            anyhow::ensure!(
                files.len() <= MAX_FILES_COUNT,
                "Too many files (max {})",
                MAX_FILES_COUNT
            );
            for f in files {
                Self::validate_field("file path", f, MAX_FIELD_LEN)?;
            }
        }

        let files_json = event
            .files
            .as_ref()
            .map(|f| serde_json::to_string(f))
            .transpose()?;

        let conn = self.lock_conn();
        conn.execute(
            "INSERT INTO events (tool, type, title, summary, files, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.tool,
                event.event_type,
                event.title,
                event.summary,
                files_json,
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
            "SELECT id, tool, type, title, summary, files, timestamp, created_at
             FROM events
             ORDER BY timestamp DESC
             LIMIT ?1",
        )?;

        Self::collect_events(stmt.query_map(params![limit], |row| Self::row_to_event(row))?)
    }

    /// Full-text-ish search across title, summary, type, tool, and files.
    ///
    /// Escapes LIKE wildcards in the user query so `%` and `_` are treated
    /// as literal characters.
    pub fn search_events(&self, query: &str, limit: u32) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        // Escape LIKE meta-characters to prevent wildcard injection
        let escaped = query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let like = format!("%{}%", escaped);
        let conn = self.lock_conn();
        let mut stmt = conn.prepare(
            "SELECT id, tool, type, title, summary, files, timestamp, created_at
             FROM events
             WHERE title   LIKE ?1 ESCAPE '\\'
                OR summary LIKE ?1 ESCAPE '\\'
                OR type    LIKE ?1 ESCAPE '\\'
                OR tool    LIKE ?1 ESCAPE '\\'
                OR files   LIKE ?1 ESCAPE '\\'
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        Self::collect_events(stmt.query_map(params![like, limit], |row| Self::row_to_event(row))?)
    }

    /// Filtered query supporting optional tool / event_type predicates.
    pub fn get_filtered_events(
        &self,
        limit: u32,
        tool: Option<String>,
        event_type: Option<String>,
    ) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let mut sql = String::from(
            "SELECT id, tool, type, title, summary, files, timestamp, created_at FROM events",
        );
        let mut conditions = Vec::new();

        if tool.is_some() {
            conditions.push("tool = ?");
        }
        if event_type.is_some() {
            conditions.push("type = ?");
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY timestamp DESC LIMIT ?");

        let conn = self.lock_conn();
        let mut stmt = conn.prepare(&sql)?;

        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        if let Some(ref t) = tool {
            params.push(t);
        }
        if let Some(ref e) = event_type {
            params.push(e);
        }
        params.push(&limit);

        Self::collect_events(stmt.query_map(&*params, |row| Self::row_to_event(row))?)
    }

    /// Return events ordered chronologically (oldest first), capped at MAX_RESULTS.
    pub fn get_timeline(&self, limit: u32) -> Result<Vec<Event>> {
        let limit = limit.min(MAX_RESULTS);
        let conn = self.lock_conn();
        let mut stmt = conn.prepare(
            "SELECT id, tool, type, title, summary, files, timestamp, created_at
             FROM events
             ORDER BY timestamp ASC
             LIMIT ?1",
        )?;

        Self::collect_events(stmt.query_map(params![limit], |row| Self::row_to_event(row))?)
    }

    /// Return aggregate counts grouped by tool.
    pub fn get_stats(&self) -> Result<Vec<(String, i64)>> {
        let conn = self.lock_conn();
        let mut stmt = conn.prepare(
            "SELECT tool, COUNT(*) as cnt FROM events GROUP BY tool ORDER BY cnt DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Return the total number of events in the database.
    pub fn count_events(&self) -> Result<i64> {
        let conn = self.lock_conn();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(count)
    }

    // ── Helpers ─────────────────────────────────────────────────────

    /// Acquire the connection lock, recovering from a poisoned mutex
    /// instead of panicking the server.
    fn lock_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        match self.conn.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Recovered from poisoned mutex lock");
                poisoned.into_inner()
            }
        }
    }

    /// Validate that a string field is non-empty and within length limits.
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

    fn collect_events(
        iter: impl Iterator<Item = SqliteResult<Event>>,
    ) -> Result<Vec<Event>> {
        let mut events = Vec::new();
        for event in iter {
            events.push(event?);
        }
        Ok(events)
    }

    fn row_to_event(row: &rusqlite::Row) -> SqliteResult<Event> {
        let files_str: Option<String> = row.get(5)?;
        let files = match files_str {
            Some(s) => serde_json::from_str(&s).unwrap_or(None),
            None => None,
        };

        let timestamp_str: String = row.get(6)?;
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let created_at_str: Option<String> = row.get(7)?;
        let created_at = created_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        });

        Ok(Event {
            id: row.get(0)?,
            tool: row.get(1)?,
            event_type: row.get(2)?,
            title: row.get(3)?,
            summary: row.get(4)?,
            files,
            timestamp,
            created_at,
        })
    }
}
