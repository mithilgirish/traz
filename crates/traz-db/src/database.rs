use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Database abstraction over a local SQLite store.
///
/// All event persistence flows through this struct. The inner connection
/// is wrapped in a `Mutex` so the `Db` can be shared safely across the
/// async Axum handlers and the MCP server.
pub struct Db {
    pub(crate) conn: Mutex<Connection>,
    pub(crate) path: PathBuf,
    pub config: traz_core::TrazConfig,
}

impl Db {
    /// Open (or create) the database at `db_path` and run migrations.
    pub fn open(db_path: &Path) -> Result<Self> {
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

        let config = traz_core::TrazConfig::resolve();
        let db = Self {
            conn: Mutex::new(conn),
            path: db_path.to_path_buf(),
            config,
        };
        db.migrate().context("Failed to run database migrations")?;

        Ok(db)
    }

    /// Returns the filesystem path of the database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Acquire the connection lock, recovering from a poisoned mutex
    /// instead of panicking the server.
    pub(crate) fn lock_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        match self.conn.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Recovered from poisoned mutex lock");
                poisoned.into_inner()
            }
        }
    }

    pub fn migrate(&self) -> Result<()> {
        let conn = self.lock_conn();

        // Step 1: Create table if completely new
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
                diff        TEXT,
                timestamp   TEXT    NOT NULL,
                created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );",
        )?;

        // Step 2: Add columns that may be missing from older schemas
        Self::add_column_if_missing(&conn, "uuid");
        Self::add_column_if_missing(&conn, "metadata");
        Self::add_column_if_missing(&conn, "tags");
        Self::add_column_if_missing(&conn, "session_id");
        Self::add_column_if_missing(&conn, "diff");

        // Step 3: Create indexes (safe now that all columns exist)
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_events_tool      ON events(tool);
             CREATE INDEX IF NOT EXISTS idx_events_type      ON events(type);
             CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);",
        )?;

        // Step 4: Create event_embeddings table
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS event_embeddings (
                id INTEGER PRIMARY KEY,
                event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
                vector BLOB NOT NULL,
                model_version TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_embeddings_event_id ON event_embeddings(event_id);"
        )?;

        Ok(())
    }

    fn add_column_if_missing(conn: &Connection, column: &str) {
        let has_col: bool = conn
            .prepare(&format!(
                "SELECT COUNT(*) FROM pragma_table_info('events') WHERE name='{}'",
                column
            ))
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|n| n > 0)
            .unwrap_or(false);

        if !has_col {
            let _ = conn.execute(
                &format!("ALTER TABLE events ADD COLUMN {} TEXT", column),
                [],
            );
        }
    }
}
