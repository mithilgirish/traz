use anyhow::Result;
use rusqlite::Result as SqliteResult;
use traz_core::Event;

pub(crate) const MAX_FIELD_LEN: usize = 500;
pub(crate) const MAX_SUMMARY_LEN: usize = 10_000;
pub(crate) const MAX_FILES_COUNT: usize = 100;
pub(crate) const MAX_RESULTS: u32 = 1000;

pub(crate) fn validate_field(name: &str, value: &str, max_len: usize) -> Result<()> {
    anyhow::ensure!(!value.trim().is_empty(), "{} must not be empty", name);
    anyhow::ensure!(
        value.len() <= max_len,
        "{} exceeds maximum length of {} bytes",
        name,
        max_len
    );
    Ok(())
}

pub(crate) fn collect_events(iter: impl Iterator<Item = SqliteResult<Event>>) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    for event in iter {
        events.push(event?);
    }
    Ok(events)
}

pub(crate) fn row_to_event(row: &rusqlite::Row) -> SqliteResult<Event> {
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
    let diff: Option<String> = row.get(10)?;

    let timestamp_str: String = row.get(11)?;
    let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let created_at_str: Option<String> = row.get(12)?;
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
        diff,
        timestamp,
        created_at,
    })
}
