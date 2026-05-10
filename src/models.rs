use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a single engineering event in the traz timeline.
///
/// Events are the fundamental unit of context in traz. Each event captures
/// a discrete engineering action — a bug fix, a refactor, a decision — along
/// with metadata about which tool produced it and which files were involved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub tool: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

impl Event {
    pub fn new(
        tool: String,
        event_type: String,
        title: String,
        summary: Option<String>,
        files: Option<Vec<String>>,
        timestamp: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: None,
            tool,
            event_type,
            title,
            summary,
            files,
            timestamp: timestamp.unwrap_or_else(Utc::now),
            created_at: None,
        }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ts = self.timestamp.format("%Y-%m-%d %H:%M:%S");
        write!(f, "[{}] {} · {} — {}", ts, self.tool, self.event_type, self.title)
    }
}
