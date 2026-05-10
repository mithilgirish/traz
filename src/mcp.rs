use crate::db::Db;
use crate::models::Event;
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::Arc;

/// Run the MCP (Model Context Protocol) stdio server.
///
/// This implements the MCP JSON-RPC protocol over stdin/stdout so that
/// any MCP-compatible AI tool (Claude Code, Cursor, Gemini CLI, etc.)
/// can connect to traz and read/write engineering context natively.
pub async fn run_mcp_server(db: Arc<Db>) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("Parse error: {}", e) }
                });
                writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        // Handle notifications (no id) — just acknowledge silently
        let id = match req.get("id") {
            Some(id) => id.clone(),
            None => continue,
        };

        let method = req
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");

        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "traz",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            }),

            "notifications/initialized" => continue,

            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": build_tool_definitions()
                }
            }),

            "tools/call" => {
                let result = handle_tool_call(&db, &req);
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": result
                })
            }

            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                }
            }),
        };

        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

// ── Tool definitions ────────────────────────────────────────────────

fn build_tool_definitions() -> Value {
    json!([
        {
            "name": "traz_recent",
            "description": "Get recent engineering events from the traz local timeline. Use this to understand what was recently worked on, debugged, or decided.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "number",
                        "description": "Number of events to retrieve (default 10, max 100)"
                    }
                }
            }
        },
        {
            "name": "traz_search",
            "description": "Search the traz engineering timeline for events matching a keyword. Searches across titles, summaries, tools, event types, and file names.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search term to look for"
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "traz_add",
            "description": "Add a new engineering event to the traz timeline. Use this to record bug fixes, refactors, architectural decisions, or any engineering context worth preserving for future sessions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tool":    { "type": "string", "description": "Name of your AI tool (e.g. claude, cursor, gemini)" },
                    "type":    { "type": "string", "description": "Event category: bug_fix, refactor, feature, decision, debug, test, deploy, revert" },
                    "title":   { "type": "string", "description": "A short, descriptive title" },
                    "summary": { "type": "string", "description": "Longer explanation of reasoning, context, and decisions made" },
                    "files":   { "type": "array", "items": { "type": "string" }, "description": "List of files involved" }
                },
                "required": ["tool", "type", "title"]
            }
        },
        {
            "name": "traz_timeline",
            "description": "Get the full chronological timeline of engineering events, oldest first. Useful for understanding the evolution of a project.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }
    ])
}

// ── Tool dispatch ───────────────────────────────────────────────────

fn handle_tool_call(db: &Db, req: &Value) -> Value {
    let default_params = json!({});
    let params = req.get("params").unwrap_or(&default_params);
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("");
    let default_args = json!({});
    let args = params.get("arguments").unwrap_or(&default_args);

    match name {
        "traz_recent" => {
            let limit = args
                .get("limit")
                .and_then(|l| l.as_u64())
                .unwrap_or(10)
                .min(100) as u32;

            match db.get_recent_events(limit) {
                Ok(events) => tool_ok(&serde_json::to_string_pretty(&events).unwrap_or_default()),
                Err(e) => tool_err(&e.to_string()),
            }
        }
        "traz_search" => {
            let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
            if query.is_empty() {
                return tool_err("Missing required argument: query");
            }
            match db.search_events(query) {
                Ok(events) if events.is_empty() => {
                    tool_ok(&format!("No events found matching \"{}\"", query))
                }
                Ok(events) => tool_ok(&serde_json::to_string_pretty(&events).unwrap_or_default()),
                Err(e) => tool_err(&e.to_string()),
            }
        }
        "traz_add" => {
            let tool = args
                .get("tool")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();
            let event_type = args
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("misc")
                .to_string();
            let title = args
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("Untitled")
                .to_string();
            let summary = args
                .get("summary")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());
            let files = args.get("files").and_then(|f| f.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            });

            let event = Event::new(tool, event_type, title, summary, files, None);
            match db.insert_event(&event) {
                Ok(id) => tool_ok(&format!("Event created with ID {}", id)),
                Err(e) => tool_err(&e.to_string()),
            }
        }
        "traz_timeline" => match db.get_timeline() {
            Ok(events) => tool_ok(&serde_json::to_string_pretty(&events).unwrap_or_default()),
            Err(e) => tool_err(&e.to_string()),
        },
        _ => tool_err(&format!("Unknown tool: {}", name)),
    }
}

fn tool_ok(text: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": text }] })
}

fn tool_err(text: &str) -> Value {
    json!({ "isError": true, "content": [{ "type": "text", "text": text }] })
}
