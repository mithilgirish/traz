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
    "traz_checkpoint",
    "traz_recap",
    "traz_show",
    "traz_diff",
];

const EXPERIMENTAL_TOOLS: &[&str] = &["traz_timeline", "traz_delete", "traz_compress"];

/// Maximum line length accepted from stdin (10 MB).
const MAX_LINE_LEN: usize = 10 * 1024 * 1024;

/// Run the MCP (Model Context Protocol) stdio server.
pub async fn run_mcp_server(db: Arc<Db>) -> Result<()> {
    eprintln!("🚀 traz MCP server is running and listening on stdio.");
    eprintln!("   (Note: this mode is for AI clients. Press Ctrl+C to exit.)");

    let experimental = std::env::var("TRAZ_EXPERIMENTAL").unwrap_or_default() == "1";
    if experimental {
        eprintln!(
            "[traz] experimental MCP tools enabled (traz_timeline, traz_delete, traz_compress). These may change in v0.2."
        );
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
                    },
                    "instructions": "You have access to traz, a local-first engineering memory layer. \
            ALWAYS call traz_recent at the start of every new chat to retrieve the latest checkpoint and resume context. \
            Use format='dense' for all traz tool calls to save 60-75% of tokens. \
            Use traz_add to log significant decisions, bug fixes, and architectural changes during the session. \
            Use traz_checkpoint before the context window fills up to save progress and enable a clean restart. \
            Use traz_recap for a quick summary of what happened in the last 24h instead of reading the full timeline."
                }
            }),

            "notifications/initialized" => continue,

            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": build_tool_definitions(experimental) }
            }),

            "tools/call" => {
                let db_clone = db.clone();
                let req_clone = req.clone();
                let result = tokio::task::spawn_blocking(move || {
                    handle_tool_call(&db_clone, &req_clone, experimental)
                })
                .await
                .unwrap_or_else(|e| tool_err(&format!("Task panicked: {}", e)));
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
            "description": "Get recent engineering events from the traz local timeline. If you are starting a fresh chat to reset context bloat, use this to read the latest 'checkpoint' event. Supports token-optimized output: pass format='dense' to get ~50-70% fewer tokens.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "number", "description": "Number of events to retrieve (default 10, max 100)" },
                    "format": { "type": "string", "description": "Output format: 'markdown' (default, human-readable) or 'dense' (AI-optimized, ~50-70% fewer tokens)" },
                    "max_tokens": { "type": "number", "description": "Optional token budget. Output is truncated to fit within this many tokens." },
                    "deduplicate": { "type": "boolean", "description": "Merge near-duplicate events to save tokens (default false)" }
                }
            }
        }),
        json!({
            "name": "traz_search",
            "description": "Search the traz engineering timeline for events matching a keyword. Searches across titles, summaries, tools, types, and filenames.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The search term" },
                    "tool": { "type": "string", "description": "Filter by tool" },
                    "type": { "type": "string", "description": "Filter by event type" },
                    "tag": { "type": "string", "description": "Filter by tag" }
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
            "description": "Get a structured summary of recent activity, optimized for AI agent consumption. Provide a 'query' to retrieve only relevant context via RAG. Use format='dense' for ~50-70% token savings, and max_tokens to enforce a budget. If you are starting a fresh chat to reset context bloat, use this to instantly regain your bearing without reading old conversation history.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Optional search query to fetch only context relevant to your current task." },
                    "limit": { "type": "number", "description": "Number of recent events to include (default 10)" },
                    "format": { "type": "string", "description": "Output format: 'markdown' (default) or 'dense' (AI-optimized, uses pipe-delimited single-line format, type abbreviations, diff summaries — ~50-70% fewer tokens)" },
                    "max_tokens": { "type": "number", "description": "Optional token budget. Output is automatically truncated to fit, using progressive detail reduction." },
                    "deduplicate": { "type": "boolean", "description": "Merge near-duplicate events to save tokens (default false). Uses Jaccard similarity on titles." }
                }
            }
        }),
        json!({
            "name": "traz_stats",
            "description": "Get database statistics including total events and event counts per tool.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "traz_checkpoint",
            "description": "When your AI chat gets too long, context bloats and performance drops. Call this tool to generate a 'Checkpoint' event summarizing all progress and current state. Then ask the user to start a fresh chat and read this checkpoint. This resets your context window.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "summary": { "type": "string", "description": "A dense summary of what was accomplished, what failed, and exact next steps." }
                },
                "required": ["summary"]
            }
        }),
        json!({
            "name": "traz_show",
            "description": "Show the full, unabridged details of a specific engineering event by its ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number", "description": "The ID of the event to view" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "traz_diff",
            "description": "Show the full code diff (patch) associated with a specific engineering event by its ID, if any.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number", "description": "The ID of the event" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "traz_recap",
            "description": "Get a time-bounded summary of recent engineering activity. Perfect for morning standups or quick 'what happened recently?' context. Returns events from the last N hours in a clean, human-readable format. Token-efficient by design.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hours": { "type": "number", "description": "Number of hours to look back (default 24, max 168=1week)" },
                    "format": { "type": "string", "description": "Output format: 'markdown' (default) or 'dense' (AI-optimized, ~50-70% fewer tokens)" }
                }
            }
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
            let format =
                traz_core::OutputFormat::from_str_opt(args.get("format").and_then(|f| f.as_str()));
            let max_tokens = args
                .get("max_tokens")
                .and_then(|m| m.as_u64())
                .map(|m| m as usize);
            let deduplicate = args
                .get("deduplicate")
                .and_then(|d| d.as_bool())
                .unwrap_or(false);

            match db.get_recent_events(limit) {
                Ok(events) => match format {
                    traz_core::OutputFormat::Dense => {
                        let mut budget = match max_tokens {
                            Some(n) => traz_core::TokenBudget::new(n),
                            None => traz_core::TokenBudget::unlimited(),
                        };
                        let output = traz_core::build_optimized_context(
                            events,
                            format,
                            &mut budget,
                            deduplicate,
                            Some("Recent Events"),
                        );
                        tool_ok(&output)
                    }
                    traz_core::OutputFormat::Markdown => {
                        tool_ok(&serde_json::to_string_pretty(&events).unwrap_or_default())
                    }
                },
                Err(e) => tool_err(&e.to_string()),
            }
        }
        "traz_search" => {
            let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
            if query.is_empty() {
                return tool_err("Missing required argument: query");
            }
            let query = &query[..query.len().min(500)];

            let tool_filter = args.get("tool").and_then(|v| v.as_str());
            let type_filter = args.get("type").and_then(|v| v.as_str());
            let tag_filter = args.get("tag").and_then(|v| v.as_str());

            let filters = traz_db::SearchFilters {
                tool: tool_filter,
                event_type: type_filter,
                tag: tag_filter,
                since: None,
            };

            match db.hybrid_search(query, &filters, 100) {
                Ok(results) if results.is_empty() => {
                    tool_ok(&format!("[search] No events found matching \"{}\"", query))
                }
                Ok(results) => {
                    let mut output = String::new();
                    output.push_str("[search]\n");
                    for (idx, (event, score)) in results.iter().enumerate() {
                        let score_str = if *score >= 0.99 {
                            "exact match".to_string()
                        } else {
                            format!("{:.0}%", score * 100.0)
                        };
                        output.push_str(&format!(
                            "{}. [{}] {} - {} (tool: {}, type: {})\n",
                            idx + 1,
                            score_str,
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

            let diff = args
                .get("diff")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());

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
            let query = args.get("query").and_then(|q| q.as_str());
            let format =
                traz_core::OutputFormat::from_str_opt(args.get("format").and_then(|f| f.as_str()));
            let max_tokens = args
                .get("max_tokens")
                .and_then(|m| m.as_u64())
                .map(|m| m as usize);
            let deduplicate = args
                .get("deduplicate")
                .and_then(|d| d.as_bool())
                .unwrap_or(false);

            match db.get_context_optimized(query, limit, format, max_tokens, deduplicate) {
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
                            tool_ok(&format!(
                                "Compressed {} events older than {} days into new Epoch event #{}.",
                                count, d, new_id
                            ))
                        } else {
                            tool_ok(&format!(
                                "No events older than {} days found. Nothing was compressed.",
                                d
                            ))
                        }
                    }
                    Err(e) => tool_err(&e.to_string()),
                }
            } else {
                tool_err("Missing required arguments: days, summary")
            }
        }
        "traz_show" => {
            if let Some(id_val) = args.get("id") {
                if let Some(id) = id_val.as_i64() {
                    match db.get_event(id) {
                        Ok(Some(event)) => {
                            tool_ok(&serde_json::to_string_pretty(&event).unwrap_or_default())
                        }
                        Ok(None) => tool_err(&format!("Event {} not found.", id)),
                        Err(e) => tool_err(&e.to_string()),
                    }
                } else {
                    tool_err("Argument 'id' must be an integer.")
                }
            } else {
                tool_err("Missing required argument: id")
            }
        }
        "traz_diff" => {
            if let Some(id_val) = args.get("id") {
                if let Some(id) = id_val.as_i64() {
                    match db.get_event(id) {
                        Ok(Some(event)) => {
                            if let Some(diff) = event.diff {
                                tool_ok(&diff)
                            } else {
                                tool_ok("This event has no associated code diff.")
                            }
                        }
                        Ok(None) => tool_err(&format!("Event {} not found.", id)),
                        Err(e) => tool_err(&e.to_string()),
                    }
                } else {
                    tool_err("Argument 'id' must be an integer.")
                }
            } else {
                tool_err("Missing required argument: id")
            }
        }
        "traz_checkpoint" => {
            let summary = args
                .get("summary")
                .and_then(|s| s.as_str())
                .unwrap_or("No summary provided.")
                .to_string();

            let event = Event::new(
                "ai-agent".to_string(),
                "checkpoint".to_string(),
                "Session Checkpoint".to_string(),
                Some(summary),
                None,
                None,
            );

            match db.insert_event(&event) {
                Ok(id) => tool_ok(&format!(
                    "Checkpoint created with ID {}. Please instruct the user to start a new chat session to clear context bloat.",
                    id
                )),
                Err(e) => tool_err(&e.to_string()),
            }
        }
        "traz_recap" => {
            let hours = args
                .get("hours")
                .and_then(|h| h.as_u64())
                .unwrap_or(24)
                .min(168) as i64;
            let format =
                traz_core::OutputFormat::from_str_opt(args.get("format").and_then(|f| f.as_str()));

            let since = chrono::Utc::now()
                - chrono::Duration::try_hours(hours).unwrap_or(chrono::Duration::zero());

            match db.get_filtered_events(100, None, None, Some(since), None) {
                Ok(events) if events.is_empty() => tool_ok(&format!(
                    "No events in the last {} hours. You're starting fresh!",
                    hours
                )),
                Ok(events) => {
                    let mut output =
                        format!("Recap — Last {} hours ({} events)\n", hours, events.len());
                    output.push_str("─────────────────────────────────────\n");
                    match format {
                        traz_core::OutputFormat::Dense => {
                            let mut budget = traz_core::TokenBudget::unlimited();
                            let dense = traz_core::build_optimized_context(
                                events,
                                traz_core::OutputFormat::Dense,
                                &mut budget,
                                false,
                                None,
                            );
                            output.push_str(&dense);
                        }
                        traz_core::OutputFormat::Markdown => {
                            for e in &events {
                                output.push_str(&format!(
                                    "• {} [{}·{}]{}\n",
                                    e.title,
                                    e.tool,
                                    e.event_type,
                                    e.summary
                                        .as_deref()
                                        .map(|s| format!(" — {}", &s[..s.len().min(80)]))
                                        .unwrap_or_default()
                                ));
                            }
                        }
                    }
                    tool_ok(&output)
                }
                Err(e) => tool_err(&e.to_string()),
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
