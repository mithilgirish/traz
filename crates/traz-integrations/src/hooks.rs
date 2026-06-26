use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use traz_core::Event;
use traz_db::Db;

#[derive(Debug, Clone, Deserialize)]
pub struct HookInput {
    #[serde(alias = "conversation_id", alias = "generation_id", alias = "session_id")]
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    #[serde(alias = "query", alias = "input", alias = "message")]
    pub prompt: Option<String>,
    pub tool_name: Option<String>,
    pub tool_input: Option<Value>,
    pub tool_response: Option<Value>,
    pub file_path: Option<String>,
    pub edits: Option<Value>,
    pub last_assistant_message: Option<String>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct HookOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hookSpecificOutput: Option<HookSpecificOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub systemMessage: Option<String>,
    #[serde(rename = "continue")]
    pub should_continue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct HookSpecificOutput {
    pub hookEventName: String,
    pub additionalContext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSessionState {
    pub session_id: String,
    pub tool: String,
    pub updated_at: u64,
}

/// Handle a lifecycle hook event from an AI agent platform.
///
/// Parses standard input payload, updates the shared session registry, logs events,
/// and returns the stdout JSON payload for context injection.
pub fn handle_hook(db: &Db, platform: &str, event_type: &str, stdin_data: &str) -> Result<String> {
    let input: HookInput = serde_json::from_str(stdin_data)
        .unwrap_or_else(|_| HookInput {
            session_id: None,
            cwd: None,
            prompt: None,
            tool_name: None,
            tool_input: None,
            tool_response: None,
            file_path: None,
            edits: None,
            last_assistant_message: None,
            exit_code: None,
        });

    let data_dir = db.path().parent().unwrap_or_else(|| Path::new("."));
    let active_session_path = data_dir.join("active_session.json");

    // Shared Memory Layer: Track the most recently active session
    let mut other_session_context = String::new();
    if let Ok(content) = fs::read_to_string(&active_session_path) {
        if let Ok(state) = serde_json::from_str::<ActiveSessionState>(&content) {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            // If another tool was active recently (within past 2 hours), surface that context
            if state.tool != platform && now.saturating_sub(state.updated_at) < 7200 {
                other_session_context = format!(
                    "--- SHARED MEMORY UPDATE ---\n\
                     Note: A session using '{}' was active recently (Session ID: {}).\n\
                     Recent timeline changes below are synchronized across both tools.\n\n",
                    state.tool, state.session_id
                );
            }
        }
    }

    let response = match event_type {
        "session-init" => {
            // Update active session metadata
            if let Some(ref session_id) = input.session_id {
                let state = ActiveSessionState {
                    session_id: session_id.clone(),
                    tool: platform.to_string(),
                    updated_at: SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                };
                if let Ok(serialized) = serde_json::to_string_pretty(&state) {
                    let _ = fs::write(&active_session_path, serialized);
                }
            }

            let mut additional_context = String::new();
            if let Some(ref prompt) = input.prompt {
                if prompt.trim().len() >= 20 {
                    let filters = traz_db::SearchFilters::default();
                    if let Ok(matches) = db.hybrid_search(prompt, &filters, 3) {
                        if !matches.is_empty() {
                            additional_context.push_str("### Relevant Historical Context:\n");
                            for (event, _) in matches {
                                additional_context.push_str(&format!(
                                    "- **[{}] {}** ({}): {}\n",
                                    event.event_type,
                                    event.title,
                                    event.tool,
                                    event.summary.as_deref().unwrap_or_default()
                                ));
                            }
                            additional_context.push_str("\n");
                        }
                    }
                }
            }

            let context_combined = format!("{}{}", other_session_context, additional_context);
            if !context_combined.trim().is_empty() {
                HookOutput {
                    hookSpecificOutput: Some(HookSpecificOutput {
                        hookEventName: "UserPromptSubmit".to_string(),
                        additionalContext: context_combined,
                    }),
                    systemMessage: None,
                    should_continue: true,
                }
            } else {
                HookOutput {
                    hookSpecificOutput: None,
                    systemMessage: None,
                    should_continue: true,
                }
            }
        }

        "context" => {
            let limit = 10;
            let context_summary = db.get_context_summary(None, limit).unwrap_or_default();
            HookOutput {
                hookSpecificOutput: Some(HookSpecificOutput {
                    hookEventName: "SessionStart".to_string(),
                    additionalContext: format!("{}{}", other_session_context, context_summary),
                }),
                systemMessage: Some(format!(
                    "traz: Context successfully synchronized from database: {}",
                    db.path().display()
                )),
                should_continue: true,
            }
        }

        "observation" => {
            if let Some(ref tool_name) = input.tool_name {
                let is_failed = input.exit_code.unwrap_or(0) != 0;
                let mut title = format!("Ran tool: {}", tool_name);
                let mut event_type = "note".to_string();

                if is_failed {
                    title = format!("Command/Tool {} failed (exit status {})", tool_name, input.exit_code.unwrap_or(1));
                    event_type = "config".to_string();
                }

                let summary = match (&input.tool_input, &input.tool_response) {
                    (Some(inp), Some(resp)) => Some(format!(
                        "Input:\n{}\n\nOutput:\n{}",
                        serde_json::to_string_pretty(inp).unwrap_or_default(),
                        serde_json::to_string_pretty(resp).unwrap_or_default()
                    )),
                    (Some(inp), None) => Some(format!("Input:\n{}", serde_json::to_string_pretty(inp).unwrap_or_default())),
                    (None, Some(resp)) => Some(format!("Output:\n{}", serde_json::to_string_pretty(resp).unwrap_or_default())),
                    _ => None,
                };

                let event = Event::new(
                    platform.to_string(),
                    event_type,
                    title,
                    summary,
                    None,
                    None,
                ).with_session(input.session_id.clone().unwrap_or_default());
                let _ = db.insert_event(&event);
            }

            HookOutput {
                hookSpecificOutput: None,
                systemMessage: None,
                should_continue: true,
            }
        }

        "file-edit" => {
            if let Some(ref file_path) = input.file_path {
                let title = format!("Modified file: {}", file_path);
                let summary = input.edits.as_ref().map(|e| serde_json::to_string_pretty(e).unwrap_or_default());
                let event = Event::new(
                    platform.to_string(),
                    "refactor".to_string(),
                    title,
                    summary,
                    Some(vec![file_path.clone()]),
                    None,
                ).with_session(input.session_id.clone().unwrap_or_default());
                let _ = db.insert_event(&event);
            }

            HookOutput {
                hookSpecificOutput: None,
                systemMessage: None,
                should_continue: true,
            }
        }

        "summarize" => {
            if let Some(ref last_msg) = input.last_assistant_message {
                if !last_msg.trim().is_empty() {
                    let event = Event::new(
                        platform.to_string(),
                        "note".to_string(),
                        format!("Session ended: {}", platform),
                        Some(last_msg.clone()),
                        None,
                        None,
                    ).with_session(input.session_id.clone().unwrap_or_default());
                    let _ = db.insert_event(&event);
                }
            }

            HookOutput {
                hookSpecificOutput: None,
                systemMessage: None,
                should_continue: true,
            }
        }

        _ => HookOutput {
            hookSpecificOutput: None,
            systemMessage: None,
            should_continue: true,
        },
    };

    Ok(serde_json::to_string(&response)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_hook_session_init() {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_db_path = std::env::temp_dir().join(format!("traz_test_{}.db", ts));
        
        // Open a temp database
        let db = Db::open(&temp_db_path).unwrap();

        let stdin_payload = r#"{
            "conversation_id": "test-session-123",
            "cwd": "/some/project",
            "prompt": "Hello this is a test prompt to see if hook context injection works properly"
        }"#;

        let res_str = handle_hook(&db, "cursor", "session-init", stdin_payload).unwrap();
        let res: HookOutput = serde_json::from_str(&res_str).unwrap();
        
        assert!(res.should_continue);

        // Clean up
        let _ = std::fs::remove_file(&temp_db_path);
        let _ = std::fs::remove_file(std::env::temp_dir().join("active_session.json"));
    }
}
