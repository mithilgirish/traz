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
    #[serde(
        alias = "conversation_id",
        alias = "generation_id",
        alias = "session_id"
    )]
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
/// Derives a safe session key for a branch name using hex encoding.
pub fn session_key_for_branch(branch_name: &str) -> String {
    branch_name
        .as_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

pub async fn handle_hook(
    db: &Db,
    platform: &str,
    event_type: &str,
    stdin_data: &str,
) -> Result<String> {
    let input: HookInput = serde_json::from_str(stdin_data).unwrap_or(HookInput {
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

    // Phase 2: Worktree Session Isolation
    let branch_name =
        crate::git::get_current_branch_normalized().unwrap_or_else(|| "default".to_string());
    let safe_branch_name = session_key_for_branch(&branch_name);

    let sessions_dir = data_dir.join("sessions");
    let _ = fs::create_dir_all(&sessions_dir);
    let active_session_path =
        sessions_dir.join(format!("active_session_{}.json", safe_branch_name));

    // Shared Memory Layer: Track the most recently active session
    let mut other_session_context = String::new();
    if let Some(state) = fs::read_to_string(&active_session_path)
        .ok()
        .and_then(|content| serde_json::from_str::<ActiveSessionState>(&content).ok())
    {
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
            if let Some(prompt) = input.prompt.as_deref().filter(|p| p.trim().len() >= 20) {
                let filters = traz_db::SearchFilters {
                    branch_names: Some(vec![branch_name.as_str()]),
                    ..Default::default()
                };
                let matches = db
                    .hybrid_search(prompt, &filters, 3)
                    .await
                    .unwrap_or_default();
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
                    additional_context.push('\n');
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
            let branch_filter = Some(vec![branch_name.as_str()]);
            match db
                .get_context_optimized(
                    None,
                    limit,
                    traz_core::OutputFormat::Markdown,
                    None,
                    false,
                    branch_filter,
                )
                .await
            {
                Ok(context_summary) => HookOutput {
                    hookSpecificOutput: Some(HookSpecificOutput {
                        hookEventName: "SessionStart".to_string(),
                        additionalContext: format!("{}{}", other_session_context, context_summary),
                    }),
                    systemMessage: Some(format!(
                        "traz: Context successfully synchronized from database: {}",
                        db.path().display()
                    )),
                    should_continue: true,
                },
                Err(e) => HookOutput {
                    hookSpecificOutput: Some(HookSpecificOutput {
                        hookEventName: "SessionStart".to_string(),
                        additionalContext: other_session_context,
                    }),
                    systemMessage: Some(format!(
                        "traz: Warning: Failed to fetch context summary from database: {}",
                        e
                    )),
                    should_continue: true,
                },
            }
        }

        "observation" => {
            if let Some(ref tool_name) = input.tool_name {
                let is_failed = input.exit_code.unwrap_or(0) != 0;
                let mut title = format!("Ran tool: {}", tool_name);
                let mut event_type = "note".to_string();

                if is_failed {
                    title = format!(
                        "Command/Tool {} failed (exit status {})",
                        tool_name,
                        input.exit_code.unwrap_or(1)
                    );
                    event_type = "config".to_string();
                }

                let summary = match (&input.tool_input, &input.tool_response) {
                    (Some(inp), Some(resp)) => Some(format!(
                        "Input:\n{}\n\nOutput:\n{}",
                        serde_json::to_string_pretty(inp).unwrap_or_default(),
                        serde_json::to_string_pretty(resp).unwrap_or_default()
                    )),
                    (Some(inp), None) => Some(format!(
                        "Input:\n{}",
                        serde_json::to_string_pretty(inp).unwrap_or_default()
                    )),
                    (None, Some(resp)) => Some(format!(
                        "Output:\n{}",
                        serde_json::to_string_pretty(resp).unwrap_or_default()
                    )),
                    _ => None,
                };

                let branch = Some(branch_name.clone());
                let event =
                    Event::new(platform.to_string(), event_type, title, summary, None, None)
                        .with_session(input.session_id.clone().unwrap_or_default())
                        .with_branch(branch);
                let _ = db.insert_event(&event).await;
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
                let summary = input
                    .edits
                    .as_ref()
                    .map(|e| serde_json::to_string_pretty(e).unwrap_or_default());
                let branch = Some(branch_name.clone());
                let event = Event::new(
                    platform.to_string(),
                    "refactor".to_string(),
                    title,
                    summary,
                    Some(vec![file_path.clone()]),
                    None,
                )
                .with_session(input.session_id.clone().unwrap_or_default())
                .with_branch(branch);
                let _ = db.insert_event(&event).await;
            }

            HookOutput {
                hookSpecificOutput: None,
                systemMessage: None,
                should_continue: true,
            }
        }

        "summarize" => {
            if let Some(last_msg) = input
                .last_assistant_message
                .as_deref()
                .filter(|m| !m.trim().is_empty())
            {
                let event = Event::new(
                    platform.to_string(),
                    "note".to_string(),
                    format!("Session ended: {}", platform),
                    Some(last_msg.to_string()),
                    None,
                    None,
                )
                .with_session(input.session_id.clone().unwrap_or_default());
                let _ = db.insert_event(&event).await;
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

    async fn setup_test_env(test_name: &str) -> (Db, std::path::PathBuf) {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let unique_dir = std::env::temp_dir().join(format!("traz_test_{}_{}", test_name, ts));
        let _ = std::fs::create_dir_all(&unique_dir);
        let db_path = unique_dir.join("traz.db");
        let db = Db::open(&db_path).await.unwrap();
        (db, unique_dir)
    }

    fn cleanup_test_env(unique_dir: std::path::PathBuf) {
        let _ = std::fs::remove_dir_all(unique_dir);
    }

    #[tokio::test]
    async fn test_handle_hook_session_init_with_shared_memory() {
        let (db, test_dir) = setup_test_env("session_init_shared").await;

        // Pre-populate active_session_default.json with another tool active recently
        let sessions_dir = test_dir.join("sessions");
        let _ = std::fs::create_dir_all(&sessions_dir);
        let branch_name =
            crate::git::get_current_branch_normalized().unwrap_or_else(|| "default".to_string());
        let safe_branch_name = session_key_for_branch(&branch_name);
        let active_session_path =
            sessions_dir.join(format!("active_session_{}.json", safe_branch_name));
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let state = ActiveSessionState {
            session_id: "other-session-456".to_string(),
            tool: "claude-code".to_string(),
            updated_at: now - 600, // 10 minutes ago
        };
        let serialized = serde_json::to_string_pretty(&state).unwrap();
        fs::write(&active_session_path, serialized).unwrap();

        let stdin_payload = r#"{
            "conversation_id": "test-session-123",
            "cwd": "/some/project",
            "prompt": "Hello this is a test prompt"
        }"#;

        let res_str = handle_hook(&db, "cursor", "session-init", stdin_payload)
            .await
            .unwrap();
        let res: HookOutput = serde_json::from_str(&res_str).unwrap();

        assert!(res.should_continue);
        let specific_output = res.hookSpecificOutput.unwrap();
        assert_eq!(specific_output.hookEventName, "UserPromptSubmit");
        assert!(
            specific_output
                .additionalContext
                .contains("SHARED MEMORY UPDATE")
        );
        assert!(specific_output.additionalContext.contains("claude-code"));
        assert!(
            specific_output
                .additionalContext
                .contains("other-session-456")
        );

        // Verify active session state has updated to cursor
        let updated_content = fs::read_to_string(&active_session_path).unwrap();
        let updated_state: ActiveSessionState = serde_json::from_str(&updated_content).unwrap();
        assert_eq!(updated_state.tool, "cursor");
        assert_eq!(updated_state.session_id, "test-session-123");

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_hook_session_init_expired_shared_memory() {
        let (db, test_dir) = setup_test_env("session_init_expired").await;

        // Pre-populate active_session_default.json with another tool active long ago
        let sessions_dir = test_dir.join("sessions");
        let _ = std::fs::create_dir_all(&sessions_dir);
        let branch_name =
            crate::git::get_current_branch_normalized().unwrap_or_else(|| "default".to_string());
        let safe_branch_name = session_key_for_branch(&branch_name);
        let active_session_path =
            sessions_dir.join(format!("active_session_{}.json", safe_branch_name));
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let state = ActiveSessionState {
            session_id: "other-session-456".to_string(),
            tool: "claude-code".to_string(),
            updated_at: now - 8000, // > 2 hours ago
        };
        let serialized = serde_json::to_string_pretty(&state).unwrap();
        fs::write(&active_session_path, serialized).unwrap();

        let stdin_payload = r#"{
            "conversation_id": "test-session-123",
            "cwd": "/some/project",
            "prompt": "Hello this is a test prompt"
        }"#;

        let res_str = handle_hook(&db, "cursor", "session-init", stdin_payload)
            .await
            .unwrap();
        let res: HookOutput = serde_json::from_str(&res_str).unwrap();

        assert!(res.should_continue);
        // Should not have output or additional context because prompt is < 20 chars
        // and shared memory was expired.
        assert!(res.hookSpecificOutput.is_none());

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_hook_context() {
        let (db, test_dir) = setup_test_env("context").await;

        let res_str = handle_hook(&db, "cursor", "context", "{}").await.unwrap();
        let res: HookOutput = serde_json::from_str(&res_str).unwrap();

        assert!(res.should_continue);
        let spec = res.hookSpecificOutput.unwrap();
        assert_eq!(spec.hookEventName, "SessionStart");
        assert!(
            res.systemMessage
                .unwrap()
                .contains("traz: Context successfully synchronized")
        );

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_hook_observation() {
        let (db, test_dir) = setup_test_env("observation").await;

        let stdin_payload = r#"{
            "conversation_id": "session-obs-789",
            "tool_name": "cargo test",
            "tool_input": { "args": ["--verbose"] },
            "tool_response": "tests passed",
            "exit_code": 0
        }"#;

        let res_str = handle_hook(&db, "cursor", "observation", stdin_payload)
            .await
            .unwrap();
        let res: HookOutput = serde_json::from_str(&res_str).unwrap();
        assert!(res.should_continue);

        // Verify database entry
        let events = db.get_recent_events(10).await.unwrap();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.tool, "cursor");
        assert_eq!(event.event_type, "note");
        assert_eq!(event.title, "Ran tool: cargo test");
        assert_eq!(event.session_id.as_deref(), Some("session-obs-789"));
        assert!(event.summary.as_ref().unwrap().contains("Input"));
        assert!(event.summary.as_ref().unwrap().contains("Output"));

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_hook_file_edit() {
        let (db, test_dir) = setup_test_env("file_edit").await;

        let stdin_payload = r#"{
            "conversation_id": "session-edit-101",
            "file_path": "src/lib.rs",
            "edits": { "insertions": 10 }
        }"#;

        let res_str = handle_hook(&db, "cursor", "file-edit", stdin_payload)
            .await
            .unwrap();
        let res: HookOutput = serde_json::from_str(&res_str).unwrap();
        assert!(res.should_continue);

        // Verify database entry
        let events = db.get_recent_events(10).await.unwrap();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.tool, "cursor");
        assert_eq!(event.event_type, "refactor");
        assert_eq!(event.title, "Modified file: src/lib.rs");
        assert_eq!(event.session_id.as_deref(), Some("session-edit-101"));
        assert_eq!(
            event.files.as_ref().unwrap(),
            &vec!["src/lib.rs".to_string()]
        );

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_hook_summarize() {
        let (db, test_dir) = setup_test_env("summarize").await;

        let stdin_payload = r#"{
            "conversation_id": "session-sum-202",
            "last_assistant_message": "Completed implementing tests."
        }"#;

        let res_str = handle_hook(&db, "cursor", "summarize", stdin_payload)
            .await
            .unwrap();
        let res: HookOutput = serde_json::from_str(&res_str).unwrap();
        assert!(res.should_continue);

        // Verify database entry
        let events = db.get_recent_events(10).await.unwrap();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.tool, "cursor");
        assert_eq!(event.event_type, "note");
        assert_eq!(event.title, "Session ended: cursor");
        assert_eq!(
            event.summary.as_ref().unwrap(),
            "Completed implementing tests."
        );
        assert_eq!(event.session_id.as_deref(), Some("session-sum-202"));

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_hook_invalid_stdin() {
        let (db, test_dir) = setup_test_env("invalid_stdin").await;

        let res_str = handle_hook(&db, "cursor", "session-init", "{invalid-json}")
            .await
            .unwrap();
        let res: HookOutput = serde_json::from_str(&res_str).unwrap();
        assert!(res.should_continue);
        assert!(res.hookSpecificOutput.is_none());

        cleanup_test_env(test_dir);
    }
}
