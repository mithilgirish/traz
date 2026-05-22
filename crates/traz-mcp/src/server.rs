use anyhow::Result;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use traz_core::Event;
use traz_db::Db;

#[allow(dead_code)]
const STABLE_TOOLS: &[&str] = &[
    "traz_recent",
    "traz_search",
    "traz_add",
    "traz_context",
    "traz_stats",
];

const EXPERIMENTAL_TOOLS: &[&str] = &[
    "traz_timeline",
    "traz_delete",
    "traz_compress",
];

/// Maximum line length accepted from stdin (1 MB).
const MAX_LINE_LEN: usize = 1_024 * 1_024;

/// Run the MCP (Model Context Protocol) stdio server.
pub async fn run_mcp_server(db: Arc<Db>) -> Result<()> {
    let experimental = std::env::var("TRAZ_EXPERIMENTAL").unwrap_or_default() == "1";
    if experimental {
        eprintln!("[traz] experimental MCP tools enabled (traz_timeline, traz_delete, traz_compress). These may change in v0.2.");
    }

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
                "result": { "tools": build_tool_definitions(experimental) }
            }),

            "tools/call" => {
                let result = handle_tool_call(&db, &req, experimental);
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

fn build_tool_definitions(experimental: bool) -> Value {
    let mut tools = vec![
        json!({
            "name": "traz_recent",
            "description": "Get recent engineering events from the traz local timeline. Use this to understand what was recently worked on, debugged, or decided.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "number", "description": "Number of events to retrieve (default 10, max 100)" }
                }
            }
        }),
        json!({
            "name": "traz_search",
            "description": "Search the traz engineering timeline for events matching a keyword. Searches across titles, summaries, tools, types, and filenames.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The search term" }
                },
                "required": ["query"]
            }
        }),
        json!({
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
        }),
        json!({
            "name": "traz_context",
            "description": "Get a structured markdown summary of recent engineering activity, perfect for establishing context before starting a task.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "number", "description": "Number of recent events to include (default 10)" }
                }
            }
        }),
        json!({
            "name": "traz_stats",
            "description": "Get database statistics including total events and event counts per tool.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
    ];

    if experimental {
        tools.push(json!({
            "name": "traz_timeline",
            "description": "Get the full chronological timeline of engineering events, oldest first.",
            "inputSchema": { "type": "object", "properties": {} }
        }));
        tools.push(json!({
            "name": "traz_delete",
            "description": "Delete a specific engineering event by its ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number", "description": "The ID of the event to delete" }
                },
                "required": ["id"]
            }
        }));
        tools.push(json!({
            "name": "traz_compress",
            "description": "Compress older events into a single 'epoch' summary event to save context space. AI agents can use this to keep the timeline manageable. You should fetch events first, summarize them, and then pass that summary here.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "days": { "type": "number", "description": "Number of days old an event must be to be compressed" },
                    "summary": { "type": "string", "description": "The high-level summary of the events being compressed" }
                },
                "required": ["days", "summary"]
            }
        }));
    }

    Value::Array(tools)
}

fn handle_tool_call(db: &Db, req: &Value, experimental: bool) -> Value {
    let default_params = json!({});
    let params = req.get("params").unwrap_or(&default_params);
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let default_args = json!({});
    let args = params.get("arguments").unwrap_or(&default_args);

    if !experimental && EXPERIMENTAL_TOOLS.contains(&name) {
        return tool_err(&format!("Unknown tool: {}", name));
    }

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
            if db.config.embeddings_enabled {
                match db.semantic_search(query, 100) {
                    Ok(results) if results.is_empty() => {
                        tool_ok(&format!("[semantic search] No events found matching \"{}\"", query))
                    }
                    Ok(results) => {
                        let mut output = String::new();
                        output.push_str("[semantic search]\n");
                        for (idx, (event, score)) in results.iter().enumerate() {
                            output.push_str(&format!("{}. [{:.0}%] {} - {} (tool: {}, type: {})\n", 
                                idx + 1, 
                                score * 100.0, 
                                event.title, 
                                event.summary.as_deref().unwrap_or_default(), 
                                event.tool, 
                                event.event_type
                            ));
                        }
                        tool_ok(&output)
                    }
                    Err(e) => tool_err(&e.to_string()),
                }
            } else {
                match db.search_events(query, None, 100) {
                    Ok(events) if events.is_empty() => {
                        tool_ok(&format!("[keyword search] No events found matching \"{}\"", query))
                    }
                    Ok(events) => {
                        let mut output = String::new();
                        output.push_str("[keyword search]\n");
                        for (idx, event) in events.iter().enumerate() {
                            output.push_str(&format!("{}. {} - {} (tool: {}, type: {})\n", 
                                idx + 1, 
                                event.title, 
                                event.summary.as_deref().unwrap_or_default(), 
                                event.tool, 
                                event.event_type
                            ));
                        }
                        tool_ok(&output)
                    }
                    Err(e) => tool_err(&e.to_string()),
                }
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
        "traz_context" => {
            let limit = args
                .get("limit")
                .and_then(|l| l.as_u64())
                .unwrap_or(10)
                .min(100) as u32;
            match db.get_context_summary(limit) {
                Ok(ctx) => tool_ok(&ctx),
                Err(e) => tool_err(&e.to_string()),
            }
        }
        "traz_stats" => {
            let count = db.count_events().unwrap_or(0);
            let stats = db.get_stats().unwrap_or_default();
            let mut summary = format!("Total Events: {}\n\nBy Tool:\n", count);
            for (tool, c) in stats {
                summary.push_str(&format!("- {}: {}\n", tool, c));
            }
            tool_ok(&summary)
        }
        "traz_delete" => {
            if let Some(id) = args.get("id").and_then(|i| i.as_i64()) {
                match db.delete_event(id) {
                    Ok(true) => tool_ok(&format!("Event #{} deleted.", id)),
                    Ok(false) => tool_err(&format!("Event #{} not found.", id)),
                    Err(e) => tool_err(&e.to_string()),
                }
            } else {
                tool_err("Missing required argument: id")
            }
        }
        "traz_compress" => {
            let days = args.get("days").and_then(|d| d.as_u64()).map(|d| d as u32);
            let summary = args.get("summary").and_then(|s| s.as_str());

            if let (Some(d), Some(s)) = (days, summary) {
                match db.compress_events(d, s.to_string()) {
                    Ok((count, new_id)) => {
                        if count > 0 {
                            tool_ok(&format!("Compressed {} events older than {} days into new Epoch event #{}.", count, d, new_id))
                        } else {
                            tool_ok(&format!("No events older than {} days found. Nothing was compressed.", d))
                        }
                    }
                    Err(e) => tool_err(&e.to_string()),
                }
            } else {
                tool_err("Missing required arguments: days, summary")
            }
        }
        _ => tool_err(&format!("Unknown tool: {}", name)),
    }
}

fn tool_ok(text: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": text }] })
}

fn tool_err(text: &str) -> Value {
    json!({ "isError": true, "content": [{ "type": "text", "text": text }] })
}
