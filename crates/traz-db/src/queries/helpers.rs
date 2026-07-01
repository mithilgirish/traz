use anyhow::Result;
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

pub(crate) async fn collect_events(mut rows: libsql::Rows) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    while let Some(row) = rows.next().await? {
        events.push(row_to_event(&row)?);
    }
    Ok(events)
}

pub(crate) fn row_to_event(row: &libsql::Row) -> Result<Event> {
    let id: Option<i64> = row.get(0)?;
    let uuid: String = row.get::<Option<String>>(1)?.unwrap_or_default();
    let tool: String = row.get(2)?;
    let event_type: String = row.get(3)?;
    let title: String = row.get(4)?;
    let summary: Option<String> = row.get(5)?;

    let files_str: Option<String> = row.get(6)?;
    let files: Option<Vec<String>> = files_str.map(|s| serde_json::from_str(&s)).transpose()?;

    let metadata_str: Option<String> = row.get(7)?;
    let metadata: Option<serde_json::Value> =
        metadata_str.map(|s| serde_json::from_str(&s)).transpose()?;

    let tags_str: Option<String> = row.get(8)?;
    let tags: Option<Vec<String>> = tags_str.map(|s| serde_json::from_str(&s)).transpose()?;

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
        id,
        uuid,
        tool,
        event_type,
        title,
        summary,
        files,
        metadata,
        tags,
        session_id,
        diff,
        timestamp,
        created_at,
    })
}
