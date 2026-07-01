use crate::database::Db;
use anyhow::Result;

impl Db {
    /// Return aggregate counts grouped by tool.
    pub async fn get_stats(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tool, COUNT(*) as cnt FROM events GROUP BY tool ORDER BY cnt DESC")
            .await?;
        let mut rows = stmt.query(()).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((row.get::<String>(0)?, row.get::<i64>(1)?));
        }
        Ok(out)
    }

    /// Return the total number of events.
    pub async fn count_events(&self) -> Result<i64> {
        let mut rows = self.conn.query("SELECT COUNT(*) FROM events", ()).await?;
        if let Some(row) = rows.next().await? {
            Ok(row.get::<i64>(0)?)
        } else {
            Ok(0)
        }
    }

    /// Return the ID of the most recently inserted event.
    pub async fn get_last_event_id(&self) -> Result<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM events ORDER BY id DESC LIMIT 1")
            .await?;
        let mut rows = stmt.query(()).await?;
        if let Some(row) = rows.next().await? {
            Ok(Some(row.get::<i64>(0)?))
        } else {
            Ok(None)
        }
    }

    /// Return distinct session IDs with their event counts.
    pub async fn get_sessions(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, COUNT(*) as cnt FROM events WHERE session_id IS NOT NULL GROUP BY session_id ORDER BY MAX(timestamp) DESC",
        ).await?;
        let mut rows = stmt.query(()).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((row.get::<String>(0)?, row.get::<i64>(1)?));
        }
        Ok(out)
    }

    /// Return the number of events that occurred strictly after the given event ID.
    pub async fn count_events_after(&self, id: i64) -> Result<usize> {
        let mut rows = self
            .conn
            .query(
                "SELECT COUNT(*) FROM events WHERE id > ?1",
                libsql::params![id],
            )
            .await?;
        if let Some(row) = rows.next().await? {
            Ok(row.get::<i64>(0)? as usize)
        } else {
            Ok(0)
        }
    }
}
