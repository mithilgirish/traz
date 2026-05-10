use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Represents a single engineering event in the traz timeline.
///
/// Events are the fundamental unit of context in traz. Each event captures
/// a discrete engineering action — a bug fix, a refactor, a decision — along
/// with metadata about which tool produced it and which files were involved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Database auto-increment ID (set after insertion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,

    /// Universally unique identifier for cross-tool referencing.
    pub uuid: String,

    /// The tool that created this event (e.g. "cursor", "claude", "aider").
    pub tool: String,

    /// Event category (e.g. "bug_fix", "refactor", "decision", "feature").
    #[serde(rename = "type")]
    pub event_type: String,

    /// Short descriptive title.
    pub title: String,

    /// Longer description with reasoning, context, and decisions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// List of files involved in this event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,

    /// Flexible JSON metadata for tool-specific data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Tags for categorization and filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// Session ID to group related events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// When the event occurred.
    pub timestamp: DateTime<Utc>,

    /// When the event was stored in the database.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

impl Event {
    /// Create a new event with a fresh UUID and current timestamp.
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
            uuid: Uuid::new_v4().to_string(),
            tool,
            event_type,
            title,
            summary,
            files,
            metadata: None,
            tags: None,
            session_id: None,
            timestamp: timestamp.unwrap_or_else(Utc::now),
            created_at: None,
        }
    }

    /// Builder-style setter for metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Builder-style setter for tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Builder-style setter for session ID.
    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ts = self.timestamp.format("%Y-%m-%d %H:%M:%S");
        write!(
            f,
            "[{}] {} · {} — {}",
            ts, self.tool, self.event_type, self.title
        )
    }
}
