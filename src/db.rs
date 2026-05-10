use crate::models::Event;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
        let conn = self.conn.lock().unwrap();

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
    pub fn insert_event(&self, event: &Event) -> Result<i64> {
        let files_json = event
            .files
            .as_ref()
            .map(|f| serde_json::to_string(f))
            .transpose()?;

        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM events WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // ── Read ────────────────────────────────────────────────────────

    /// Return the `limit` most recent events, newest first.
    pub fn get_recent_events(&self, limit: u32) -> Result<Vec<Event>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, tool, type, title, summary, files, timestamp, created_at
             FROM events
             ORDER BY timestamp DESC
             LIMIT ?1",
        )?;

        Self::collect_events(stmt.query_map(params![limit], |row| Self::row_to_event(row))?)
    }

    /// Full-text-ish search across title, summary, type, tool, and files.
    pub fn search_events(&self, query: &str) -> Result<Vec<Event>> {
        let like = format!("%{}%", query);
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, tool, type, title, summary, files, timestamp, created_at
             FROM events
             WHERE title   LIKE ?1
                OR summary LIKE ?1
                OR type    LIKE ?1
                OR tool    LIKE ?1
                OR files   LIKE ?1
             ORDER BY timestamp DESC",
        )?;

        Self::collect_events(stmt.query_map(params![like], |row| Self::row_to_event(row))?)
    }

    /// Filtered query supporting optional tool / event_type predicates.
    pub fn get_filtered_events(
        &self,
        limit: u32,
        tool: Option<String>,
        event_type: Option<String>,
    ) -> Result<Vec<Event>> {
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

        let conn = self.conn.lock().unwrap();
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

    /// Return every event ordered chronologically (oldest first).
    pub fn get_timeline(&self) -> Result<Vec<Event>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, tool, type, title, summary, files, timestamp, created_at
             FROM events
             ORDER BY timestamp ASC",
        )?;

        Self::collect_events(stmt.query_map([], |row| Self::row_to_event(row))?)
    }

    /// Return aggregate counts grouped by tool.
    pub fn get_stats(&self) -> Result<Vec<(String, i64)>> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(count)
    }

    // ── Helpers ─────────────────────────────────────────────────────

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
