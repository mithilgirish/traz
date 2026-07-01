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

    /// Code diff / patch associated with the event for version control.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,

    /// When the event occurred.
    pub timestamp: DateTime<Utc>,

    /// The git branch or worktree where this event occurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_name: Option<String>,

    /// The parent event ID (for DAG memory model).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_event_id: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_checkpoint: Option<bool>,

    /// The agent or conversation ID that owns this event (for multi-agent scoping).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,

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
            diff: None,
            branch_name: None,
            parent_event_id: None,
            is_checkpoint: None,
            agent_id: None,
            timestamp: timestamp.unwrap_or_else(Utc::now),
            created_at: None,
        }
    }

    /// Builder-style setter for branch_name.
    pub fn with_branch(mut self, branch_name: Option<String>) -> Self {
        self.branch_name = branch_name;
        self
    }

    /// Builder-style setter for parent_event_id.
    pub fn with_parent(mut self, parent_event_id: Option<i64>) -> Self {
        self.parent_event_id = parent_event_id;
        self
    }

    /// Builder-style setter for is_checkpoint.
    pub fn with_checkpoint(mut self, is_checkpoint: bool) -> Self {
        self.is_checkpoint = Some(is_checkpoint);
        self
    }

    /// Builder-style setter for agent_id.
    pub fn with_agent(mut self, agent_id: String) -> Self {
        self.agent_id = Some(agent_id);
        self
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

    /// Builder-style setter for code diff.
    pub fn with_diff(mut self, diff: String) -> Self {
        self.diff = Some(diff);
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_new_defaults() {
        let event = Event::new(
            "test_tool".to_string(),
            "feature".to_string(),
            "Implement feature X".to_string(),
            Some("Detailed description".to_string()),
            Some(vec!["src/lib.rs".to_string()]),
            None,
        );

        assert!(event.id.is_none());
        assert!(!event.uuid.is_empty());
        assert_eq!(event.tool, "test_tool");
        assert_eq!(event.event_type, "feature");
        assert_eq!(event.title, "Implement feature X");
        assert_eq!(event.summary, Some("Detailed description".to_string()));
        assert_eq!(event.files, Some(vec!["src/lib.rs".to_string()]));
        assert!(event.metadata.is_none());
        assert!(event.tags.is_none());
        assert!(event.session_id.is_none());
        assert!(event.diff.is_none());
        assert!(event.created_at.is_none());
        // Verify UUID is valid v4 format (8-4-4-4-12 hex chars)
        assert_eq!(event.uuid.split('-').count(), 5);
    }

    #[test]
    fn test_event_builder_methods() {
        let timestamp = Utc::now();
        let metadata_val = json!({"key": "value"});
        let tags_val = vec!["rust".to_string(), "testing".to_string()];
        let session_val = "session-123".to_string();
        let diff_val = "--- a/src/lib.rs\n+++ b/src/lib.rs".to_string();

        let event = Event::new(
            "test_tool".to_string(),
            "bug_fix".to_string(),
            "Fix bug Y".to_string(),
            None,
            None,
            Some(timestamp),
        )
        .with_metadata(metadata_val.clone())
        .with_tags(tags_val.clone())
        .with_session(session_val.clone())
        .with_diff(diff_val.clone());

        assert_eq!(event.timestamp, timestamp);
        assert_eq!(event.metadata, Some(metadata_val));
        assert_eq!(event.tags, Some(tags_val));
        assert_eq!(event.session_id, Some(session_val));
        assert_eq!(event.diff, Some(diff_val));
    }

    #[test]
    fn test_event_uuid_uniqueness() {
        let event1 = Event::new(
            "tool".to_string(),
            "type".to_string(),
            "title".to_string(),
            None,
            None,
            None,
        );
        let event2 = Event::new(
            "tool".to_string(),
            "type".to_string(),
            "title".to_string(),
            None,
            None,
            None,
        );
        assert_ne!(event1.uuid, event2.uuid);
    }

    #[test]
    fn test_event_display_format() {
        let fixed_time = DateTime::parse_from_rfc3339("2026-06-26T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let event = Event::new(
            "aider".to_string(),
            "refactor".to_string(),
            "Clean codebase".to_string(),
            None,
            None,
            Some(fixed_time),
        );

        let display_str = format!("{}", event);
        assert_eq!(
            display_str,
            "[2026-06-26 12:00:00] aider · refactor — Clean codebase"
        );
    }

    #[test]
    fn test_event_serde_roundtrip() {
        let event = Event::new(
            "mcp".to_string(),
            "decision".to_string(),
            "Use SQLite".to_string(),
            None,
            None,
            None,
        )
        .with_tags(vec!["db".to_string()]);

        let serialized = serde_json::to_string(&event).unwrap();
        // Since id, summary, files, metadata, session_id, diff, created_at are None/skip_serializing_if,
        // they should not appear in the JSON string
        assert!(!serialized.contains("\"id\""));
        assert!(!serialized.contains("\"summary\""));
        assert!(!serialized.contains("\"files\""));
        assert!(serialized.contains("\"tags\""));

        let deserialized: Event = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.uuid, event.uuid);
        assert_eq!(deserialized.tool, event.tool);
        assert_eq!(deserialized.event_type, event.event_type);
        assert_eq!(deserialized.title, event.title);
        assert_eq!(deserialized.tags, Some(vec!["db".to_string()]));
        assert!(deserialized.summary.is_none());
    }
}
