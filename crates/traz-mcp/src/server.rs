use anyhow::Result;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use traz_core::Event;
use traz_db::Db;

/// Maximum line length accepted from stdin (1 MB).
const MAX_LINE_LEN: usize = 1_024 * 1_024;

/// Run the MCP (Model Context Protocol) stdio server.
pub async fn run_mcp_server(db: Arc<Db>) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = io::BufReader::new(stdin.lock());

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if line.len() > MAX_LINE_LEN {
            let err_resp = json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32600, "message": "Request too large" }
            });
            writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
            stdout.flush()?;
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

        let id = match req.get("id") {
            Some(id) => id.clone(),
            None => continue,
        };

        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
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
                "result": { "tools": build_tool_definitions() }
            }),

            "tools/call" => {
                let result = handle_tool_call(&db, &req);
                json!({ "jsonrpc": "2.0", "id": id, "result": result })
            }

            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {}", method) }
            }),
        };

        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

fn build_tool_definitions() -> Value {
    json!([
        {
            "name": "traz_recent",
            "description": "Get recent engineering events from the traz local timeline. Use this to understand what was recently worked on, debugged, or decided.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "number", "description": "Number of events to retrieve (default 10, max 100)" }
                }
            }
        },
        {
            "name": "traz_search",
            "description": "Search the traz engineering timeline for events matching a keyword. Searches across titles, summaries, tools, types, and filenames.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The search term" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "traz_add",
            "description": "Add a new engineering event to the traz timeline. Use this to record bug fixes, refactors, decisions, or any context worth preserving.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tool":    { "type": "string", "description": "Name of your AI tool" },
                    "type":    { "type": "string", "description": "Event category: bug_fix, refactor, feature, decision, debug, test, deploy, revert" },
                    "title":   { "type": "string", "description": "Short, descriptive title" },
                    "summary": { "type": "string", "description": "Longer explanation of reasoning and context" },
                    "files":   { "type": "array", "items": { "type": "string" }, "description": "List of files involved" },
                    "diff":    { "type": "string", "description": "Unified diff or patch content for this change" }
                },
                "required": ["tool", "type", "title"]
            }
        },
        {
            "name": "traz_timeline",
            "description": "Get the full chronological timeline of engineering events, oldest first.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}

fn handle_tool_call(db: &Db, req: &Value) -> Value {
    let default_params = json!({});
    let params = req.get("params").unwrap_or(&default_params);
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
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
            let query = &query[..query.len().min(500)];
            match db.search_events(query, None, 100) {
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
                    .collect()
            });

            let diff = args.get("diff").and_then(|d| d.as_str()).map(|s| s.to_string());

            let mut event = Event::new(tool, event_type, title, summary, files, None);
            if let Some(d) = diff {
                event = event.with_diff(d);
            }
            match db.insert_event(&event) {
                Ok(id) => tool_ok(&format!("Event created with ID {}", id)),
                Err(e) => tool_err(&e.to_string()),
            }
        }
        "traz_timeline" => match db.get_timeline(500) {
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
