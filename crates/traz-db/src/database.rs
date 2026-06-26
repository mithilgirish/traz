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
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = std::fs::metadata(parent) {
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o700);
                    let _ = std::fs::set_permissions(parent, perms);
                }
            }
        }

        let conn = Connection::open(db_path).context("Failed to open SQLite database")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(db_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o600);
                let _ = std::fs::set_permissions(db_path, perms);
            }
        }

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

    /// Check if the compiled SQLite version has FTS5 support enabled.
    pub fn check_fts5_support(&self) -> bool {
        let conn = self.lock_conn();
        conn.execute_batch(
            "CREATE VIRTUAL TABLE temp.temp_fts USING fts5(dummy);
             DROP TABLE temp.temp_fts;",
        )
        .is_ok()
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
            CREATE INDEX IF NOT EXISTS idx_embeddings_event_id ON event_embeddings(event_id);",
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::SystemTime;

    fn get_temp_db_path() -> (PathBuf, PathBuf) {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("traz_database_test_{}", ts));
        fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("traz.db");
        (db_path, temp_dir)
    }

    #[test]
    fn test_db_open_and_migrations() {
        let (db_path, temp_dir) = get_temp_db_path();

        // 1. Open the DB (which runs migrations)
        let db = Db::open(&db_path).expect("Failed to open database");
        assert_eq!(db.path(), db_path);

        // Verify the database file exists
        assert!(db_path.exists());

        // On Unix, check that the file and parent directory have the correct permissions (0o600 and 0o700)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let file_meta = fs::metadata(&db_path).unwrap();
            let parent_meta = fs::metadata(&temp_dir).unwrap();
            assert_eq!(file_meta.permissions().mode() & 0o777, 0o600);
            assert_eq!(parent_meta.permissions().mode() & 0o777, 0o700);
        }

        // Verify tables are created by running simple query
        {
            let conn = db.lock_conn();
            let table_count: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('events', 'event_embeddings')",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(table_count, 2);
        }

        // 2. Test FTS5 support call
        let _has_fts5 = db.check_fts5_support();

        // 3. Clean up
        drop(db);
        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_db_migrate_idempotent() {
        let (db_path, temp_dir) = get_temp_db_path();
        
        let db = Db::open(&db_path).unwrap();
        
        // Running migrate again shouldn't fail or corrupt data
        let res = db.migrate();
        assert!(res.is_ok());

        // Test add_column_if_missing logic
        {
            let conn = db.lock_conn();
            // Try to add an existing column - should not fail
            Db::add_column_if_missing(&conn, "title");
            
            // Add a new column that's not there
            Db::add_column_if_missing(&conn, "some_new_test_field");
            
            // Verify new column exists
            let has_col: bool = conn
                .prepare("SELECT COUNT(*) FROM pragma_table_info('events') WHERE name='some_new_test_field'")
                .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
                .map(|n| n > 0)
                .unwrap_or(false);
            assert!(has_col);
        }

        drop(db);
        fs::remove_dir_all(temp_dir).unwrap();
    }
}

