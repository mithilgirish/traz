use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Database abstraction over a local libSQL store.
///
/// All event persistence flows through this struct.
pub struct Db {
    pub(crate) conn: libsql::Connection,
    pub(crate) path: PathBuf,
    pub config: traz_core::TrazConfig,
}

impl Db {
    /// Open (or create) the database at `db_path` and run migrations.
    pub async fn open(db_path: &Path) -> Result<Self> {
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

        let db_path_str = db_path
            .to_str()
            .context("Database path is not valid UTF-8")?;
        let db = libsql::Builder::new_local(db_path_str)
            .build()
            .await
            .context("Failed to build local libSQL database")?;
        let conn = db
            .connect()
            .context("Failed to connect to local libSQL database")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(db_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o600);
                let _ = std::fs::set_permissions(db_path, perms);
            }
        }

        // Tune SQLite/libSQL for single-user, local-first workloads
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous  = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )
        .await
        .context("Failed to set SQLite/libSQL pragmas")?;

        let config = traz_core::TrazConfig::resolve();
        let db_instance = Self {
            conn,
            path: db_path.to_path_buf(),
            config,
        };
        db_instance
            .migrate()
            .await
            .context("Failed to run database migrations")?;

        Ok(db_instance)
    }

    /// Returns the filesystem path of the database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if the compiled SQLite version has FTS5 support enabled.
    pub async fn check_fts5_support(&self) -> bool {
        self.conn
            .execute_batch(
                "CREATE VIRTUAL TABLE temp.temp_fts USING fts5(dummy);
             DROP TABLE temp.temp_fts;",
            )
            .await
            .is_ok()
    }

    pub async fn migrate(&self) -> Result<()> {
        // Step 1: Create table if completely new
        self.conn
            .execute_batch(
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
                branch_name TEXT,
                parent_event_id INTEGER,
                is_checkpoint BOOLEAN DEFAULT FALSE,
                agent_id    TEXT,
                timestamp   TEXT    NOT NULL,
                created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );",
            )
            .await?;

        // Step 2: Add columns that may be missing from older schemas
        self.add_column_if_missing("uuid", "TEXT").await?;
        self.add_column_if_missing("metadata", "TEXT").await?;
        self.add_column_if_missing("tags", "TEXT").await?;
        self.add_column_if_missing("session_id", "TEXT").await?;
        self.add_column_if_missing("diff", "TEXT").await?;
        self.add_column_if_missing("branch_name", "TEXT").await?;
        self.add_column_if_missing("parent_event_id", "INTEGER")
            .await?;
        self.add_column_if_missing("is_checkpoint", "BOOLEAN DEFAULT FALSE")
            .await?;
        self.add_column_if_missing("agent_id", "TEXT").await?;

        // Step 3: Create indexes (safe now that all columns exist)
        self.conn
            .execute_batch(
                "CREATE INDEX IF NOT EXISTS idx_events_tool      ON events(tool);
             CREATE INDEX IF NOT EXISTS idx_events_type      ON events(type);
             CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);",
            )
            .await?;

        // Step 4: Create event_embeddings table
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS event_embeddings (
                id INTEGER PRIMARY KEY,
                event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
                vector BLOB NOT NULL,
                model_version TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_embeddings_event_id ON event_embeddings(event_id);",
            )
            .await?;

        Ok(())
    }

    async fn add_column_if_missing(&self, column: &str, definition: &str) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('events') WHERE name=?1")
            .await?;

        let mut rows = stmt.query([column]).await?;
        let has_col = if let Some(row) = rows.next().await? {
            row.get::<i64>(0).unwrap_or(0) > 0
        } else {
            false
        };

        if !has_col {
            let sql = format!("ALTER TABLE events ADD COLUMN {} {}", column, definition);
            self.conn.execute(&sql, ()).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn get_temp_db_path() -> (PathBuf, PathBuf) {
        let ts = uuid::Uuid::new_v4().to_string();
        let temp_dir = std::env::temp_dir().join(format!("traz_database_test_{}", ts));
        fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("traz.db");
        (db_path, temp_dir)
    }

    #[tokio::test]
    async fn test_db_open_and_migrations() {
        let (db_path, temp_dir) = get_temp_db_path();

        // 1. Open the DB (which runs migrations)
        let db = Db::open(&db_path).await.expect("Failed to open database");
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
            let mut rows = db.conn
                .query(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('events', 'event_embeddings')",
                    (),
                )
                .await
                .unwrap();
            let row = rows.next().await.unwrap().unwrap();
            let table_count: i64 = row.get(0).unwrap();
            assert_eq!(table_count, 2);
        }

        // 2. Test FTS5 support call
        let _has_fts5 = db.check_fts5_support().await;

        // 3. Clean up
        drop(db);
        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[tokio::test]
    async fn test_db_migrate_idempotent() {
        let (db_path, temp_dir) = get_temp_db_path();

        let db = Db::open(&db_path).await.unwrap();

        // Running migrate again shouldn't fail or corrupt data
        let res = db.migrate().await;
        assert!(res.is_ok());

        // Test add_column_if_missing logic
        {
            // Try to add an existing column - should not fail
            db.add_column_if_missing("title", "TEXT").await.unwrap();

            // Add a new column that's not there
            db.add_column_if_missing("some_new_test_field", "TEXT")
                .await
                .unwrap();

            // Verify new column exists
            let mut stmt = db.conn.prepare("SELECT COUNT(*) FROM pragma_table_info('events') WHERE name='some_new_test_field'").await.unwrap();
            let mut rows = stmt.query(()).await.unwrap();
            let row = rows.next().await.unwrap().unwrap();
            let count: i64 = row.get(0).unwrap();
            assert!(count > 0);
        }

        drop(db);
        fs::remove_dir_all(temp_dir).unwrap();
    }
}
