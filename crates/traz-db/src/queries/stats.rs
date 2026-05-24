use crate::database::Db;
use anyhow::Result;

impl Db {
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

    /// Return the ID of the most recently inserted event.
    pub fn get_last_event_id(&self) -> Result<Option<i64>> {
        let conn = self.lock_conn();
        let mut stmt = conn.prepare("SELECT id FROM events ORDER BY id DESC LIMIT 1")?;
        let mut rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// Return distinct session IDs with their event counts.
    pub fn get_sessions(&self) -> Result<Vec<(String, i64)>> {
        let conn = self.lock_conn();
        let mut stmt = conn.prepare(
            "SELECT session_id, COUNT(*) as cnt FROM events WHERE session_id IS NOT NULL GROUP BY session_id ORDER BY MAX(timestamp) DESC",
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

    /// Return the number of events that occurred strictly after the given event ID.
    pub fn count_events_after(&self, id: i64) -> Result<usize> {
        let conn = self.lock_conn();
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM events WHERE id > ?1", [id], |row| {
                row.get(0)
            })?;
        Ok(count as usize)
    }
}
