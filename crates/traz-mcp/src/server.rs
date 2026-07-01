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
    "traz_rollback",
    "traz_rollup",
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
    let mut reader = io::BufReader::new(stdin.lock());

    loop {
        let mut line_buf = Vec::new();
        let mut current_len = 0;
        let mut eof = false;

        loop {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                eof = true;
                break;
            }

            let (done, used) = match available.iter().position(|&b| b == b'\n') {
                Some(i) => {
                    line_buf.extend_from_slice(&available[..=i]);
                    (true, i + 1)
                }
                None => {
                    line_buf.extend_from_slice(available);
                    (false, available.len())
                }
            };
            reader.consume(used);
            current_len += used;

            if current_len > MAX_LINE_LEN {
                break;
            }
            if done {
                break;
            }
        }

        if eof && line_buf.is_empty() {
            break;
        }

        if current_len > MAX_LINE_LEN {
            // Discard the rest of the malicious line until newline
            loop {
                let available = reader.fill_buf()?;
                if available.is_empty() {
                    break;
                }
                let (done, used) = match available.iter().position(|&b| b == b'\n') {
                    Some(i) => (true, i + 1),
                    None => (false, available.len()),
                };
                reader.consume(used);
                if done {
                    break;
                }
            }
            let err_resp = json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32600, "message": "Request too large" }
            });
            writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
            stdout.flush()?;
            continue;
        }

        let line = String::from_utf8_lossy(&line_buf);
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
                let result = handle_tool_call(&db, &req, experimental).await;
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
                    "diff":    { "type": "string", "description": "Unified diff or patch content for this change" },
                    "agent_id": { "type": "string", "description": "Optional identifier for the subagent making the change" }
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
            "name": "traz_rollback",
            "description": "Roll back the agent's memory to the last checkpoint on the current branch. Use this if you realize you've gone down a wrong path and want to 'forget' recent memory clutter.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "traz_rollup",
            "description": "Roll up a subagent's memory into a single summary event, deleting their granular episodic memory to keep the main context window clean.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {
                        "type": "string",
                        "description": "The ID of the subagent to rollup."
                    },
                    "summary": {
                        "type": "string",
                        "description": "A high-level summary of what the subagent accomplished."
                    }
                },
                "required": ["agent_id", "summary"]
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

async fn handle_tool_call(db: &Db, req: &Value, experimental: bool) -> Value {
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

            match db.get_recent_events(limit).await {
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

            let search_branch = traz_integrations::git::get_current_branch()
                .ok()
                .filter(|b| !b.trim().is_empty());
            let branch_names_owned: Vec<String> = if let Some(ref bn) = search_branch {
                db.get_ancestor_branches(bn)
                    .await
                    .unwrap_or_else(|_| vec!["main".to_string()])
            } else {
                Vec::new()
            };
            let branch_refs: Vec<&str> = branch_names_owned.iter().map(|s| s.as_str()).collect();

            let filters = traz_db::SearchFilters {
                tool: tool_filter,
                event_type: type_filter,
                tag: tag_filter,
                since: None,
                branch_names: if branch_refs.is_empty() {
                    None
                } else {
                    Some(branch_refs)
                },
            };

            match db.hybrid_search(query, &filters, 100).await {
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

            let agent_id = args
                .get("agent_id")
                .and_then(|a| a.as_str())
                .map(|s| s.to_string());

            let mut event = Event::new(tool, event_type, title, summary, files, None);
            if let Some(d) = diff {
                event = event.with_diff(d);
            }
            if let Some(a) = agent_id {
                event = event.with_agent(a);
            }
            let branch = traz_integrations::git::get_current_branch().ok();
            event = event.with_branch(branch);

            match db.insert_event(&event).await {
                Ok(id) => tool_ok(&format!("Event created with ID {}", id)),
                Err(e) => tool_err(&e.to_string()),
            }
        }
        "traz_timeline" => match db.get_timeline(500).await {
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

            let branch_name =
                traz_integrations::git::get_current_branch().unwrap_or_else(|_| "main".to_string());
            let branch_names = db
                .get_ancestor_branches(&branch_name)
                .await
                .unwrap_or_else(|_| vec!["main".to_string()]);
            let branch_refs: Vec<&str> = branch_names.iter().map(|s| s.as_str()).collect();

            match db
                .get_context_optimized(
                    query,
                    limit,
                    format,
                    max_tokens,
                    deduplicate,
                    Some(branch_refs),
                )
                .await
            {
                Ok(ctx) => tool_ok(&ctx),
                Err(e) => tool_err(&e.to_string()),
            }
        }
        "traz_stats" => {
            let count = match db.count_events().await {
                Ok(c) => c,
                Err(e) => return tool_err(&format!("Failed to count events: {}", e)),
            };
            let stats = match db.get_stats().await {
                Ok(s) => s,
                Err(e) => return tool_err(&format!("Failed to get stats: {}", e)),
            };
            let mut summary = format!("Total Events: {}\n\nBy Tool:\n", count);
            for (tool, c) in stats {
                summary.push_str(&format!("- {}: {}\n", tool, c));
            }
            tool_ok(&summary)
        }
        "traz_delete" => {
            if let Some(id) = args.get("id").and_then(|i| i.as_i64()) {
                match db.delete_event(id).await {
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
                match db.compress_events(d, s.to_string()).await {
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
                    match db.get_event(id).await {
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
                    match db.get_event(id).await {
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
                .unwrap_or("No summary provided.");

            let branch =
                traz_integrations::git::get_current_branch().unwrap_or_else(|_| "main".to_string());
            match db.mark_checkpoint(&branch, Some(summary.to_string())).await {
                Ok(id) => tool_ok(&format!("Checkpoint created at event #{}", id)),
                Err(e) => tool_err(&e.to_string()),
            }
        }
        "traz_rollback" => {
            let branch =
                traz_integrations::git::get_current_branch().unwrap_or_else(|_| "main".to_string());
            match db.rollback_to_checkpoint(&branch).await {
                Ok(deleted) => {
                    if deleted > 0 {
                        tool_ok(&format!("Rolled back memory. Forgot {} events.", deleted))
                    } else {
                        tool_ok("Already at the latest checkpoint.")
                    }
                }
                Err(e) => tool_err(&e.to_string()),
            }
        }
        "traz_rollup" => {
            let agent_id = args.get("agent_id").and_then(|s| s.as_str());
            let summary = args.get("summary").and_then(|s| s.as_str());

            if let (Some(a), Some(s)) = (agent_id, summary) {
                let branch = traz_integrations::git::get_current_branch().ok();
                match db.rollup_agent_memory(a, s.to_string(), branch).await {
                    Ok(id) => tool_ok(&format!(
                        "Rolled up subagent memory into summary event #{}",
                        id
                    )),
                    Err(e) => tool_err(&e.to_string()),
                }
            } else {
                tool_err("Missing required arguments: agent_id, summary")
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

            match db
                .get_filtered_events(100, None, None, Some(since), None)
                .await
            {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    async fn setup_test_env(test_name: &str) -> (Db, std::path::PathBuf) {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let unique_dir = std::env::temp_dir().join(format!("traz_mcp_test_{}_{}", test_name, ts));
        let _ = std::fs::create_dir_all(&unique_dir);
        let db_path = unique_dir.join("traz.db");
        let db = Db::open(&db_path).await.unwrap();
        (db, unique_dir)
    }

    fn cleanup_test_env(unique_dir: std::path::PathBuf) {
        let _ = std::fs::remove_dir_all(unique_dir);
    }

    #[test]
    fn test_mcp_build_tool_definitions() {
        let stable_tools = build_tool_definitions(false);
        let stable_arr = stable_tools.as_array().unwrap();

        // Assert that stable tools are returned, but no experimental ones
        assert!(stable_arr.iter().any(|t| t["name"] == "traz_recent"));
        assert!(stable_arr.iter().any(|t| t["name"] == "traz_add"));
        assert!(stable_arr.iter().any(|t| t["name"] == "traz_search"));
        assert!(!stable_arr.iter().any(|t| t["name"] == "traz_timeline"));

        let experimental_tools = build_tool_definitions(true);
        let experimental_arr = experimental_tools.as_array().unwrap();
        assert!(
            experimental_arr
                .iter()
                .any(|t| t["name"] == "traz_timeline")
        );
    }

    #[tokio::test]
    async fn test_handle_tool_call_traz_add_and_recent() {
        let (db, test_dir) = setup_test_env("add_recent").await;

        // 1. Call traz_add
        let add_payload = json!({
            "params": {
                "name": "traz_add",
                "arguments": {
                    "tool": "cursor",
                    "type": "feature",
                    "title": "Implement active synchronization",
                    "summary": "Added active timeline hooks",
                    "files": ["crates/traz-cli/src/main.rs"]
                }
            }
        });

        let res = handle_tool_call(&db, &add_payload, false).await;
        assert!(res.get("isError").is_none());
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Event created with ID"));

        // 2. Call traz_recent (format = dense)
        let recent_payload = json!({
            "params": {
                "name": "traz_recent",
                "arguments": {
                    "limit": 10,
                    "format": "dense"
                }
            }
        });
        let res_recent = handle_tool_call(&db, &recent_payload, false).await;
        let text_recent = res_recent["content"][0]["text"].as_str().unwrap();
        assert!(text_recent.contains("cursor"));
        assert!(text_recent.contains("ft")); // abbreviated type for feature
        assert!(text_recent.contains("Implement active synchronization"));

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_tool_call_traz_search() {
        let (db, test_dir) = setup_test_env("search").await;

        // Seed with event on "main" so it's visible under branch-scoped search
        let event = Event::new(
            "aider".to_string(),
            "bug_fix".to_string(),
            "Resolved panic in parser".to_string(),
            Some("Fixed a crash when input was empty".to_string()),
            None,
            None,
        )
        .with_branch(Some("main".to_string()));
        db.insert_event(&event).await.unwrap();

        let search_payload = json!({
            "params": {
                "name": "traz_search",
                "arguments": {
                    "query": "panic"
                }
            }
        });

        let res = handle_tool_call(&db, &search_payload, false).await;
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Resolved panic in parser"));
        assert!(text.contains("aider"));

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_tool_call_traz_stats() {
        let (db, test_dir) = setup_test_env("stats").await;

        let event = Event::new(
            "cursor".to_string(),
            "feature".to_string(),
            "T1".to_string(),
            None,
            None,
            None,
        );
        db.insert_event(&event).await.unwrap();

        let stats_payload = json!({
            "params": {
                "name": "traz_stats",
                "arguments": {}
            }
        });

        let res = handle_tool_call(&db, &stats_payload, false).await;
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Total Events: 1"));

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_tool_call_traz_checkpoint() {
        let (db, test_dir) = setup_test_env("checkpoint").await;

        let checkpoint_payload = json!({
            "params": {
                "name": "traz_checkpoint",
                "arguments": {
                    "summary": "Completed auth feature. All tests pass."
                }
            }
        });

        let res = handle_tool_call(&db, &checkpoint_payload, false).await;
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Checkpoint created at event #"));

        // Verify the database has the checkpoint event
        let events = db.get_recent_events(5).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "checkpoint");
        assert_eq!(events[0].title, "Completed auth feature. All tests pass.");
        assert_eq!(events[0].summary.as_deref(), None);

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_tool_call_unknown_and_experimental() {
        let (db, test_dir) = setup_test_env("unknown_experimental").await;

        // 1. Unknown tool
        let unknown_payload = json!({
            "params": {
                "name": "traz_unknown_tool",
                "arguments": {}
            }
        });
        let res = handle_tool_call(&db, &unknown_payload, false).await;
        assert_eq!(res["isError"], true);

        // 2. Experimental tool with experimental = false
        let exp_payload = json!({
            "params": {
                "name": "traz_timeline",
                "arguments": {}
            }
        });
        let res_exp_false = handle_tool_call(&db, &exp_payload, false).await;
        assert_eq!(res_exp_false["isError"], true);

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_tool_call_traz_context() {
        let (db, test_dir) = setup_test_env("context").await;

        let event = Event::new(
            "cursor".to_string(),
            "feature".to_string(),
            "MCP context testing".to_string(),
            None,
            None,
            None,
        )
        .with_branch(Some("main".to_string()));
        db.insert_event(&event).await.unwrap();

        let context_payload = json!({
            "params": {
                "name": "traz_context",
                "arguments": {
                    "limit": 5,
                    "format": "markdown"
                }
            }
        });

        let res = handle_tool_call(&db, &context_payload, false).await;
        assert!(res.get("isError").is_none());
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("MCP context testing"));

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_tool_call_traz_show_and_diff() {
        let (db, test_dir) = setup_test_env("show_diff").await;

        let event = Event::new(
            "cursor".to_string(),
            "feature".to_string(),
            "Testing show and diff".to_string(),
            None,
            None,
            None,
        )
        .with_diff("--- a/f\n+++ b/f\n+hello".to_string());

        let id = db.insert_event(&event).await.unwrap();

        // 1. traz_show
        let show_payload = json!({
            "params": {
                "name": "traz_show",
                "arguments": {
                    "id": id
                }
            }
        });
        let res_show = handle_tool_call(&db, &show_payload, false).await;
        assert!(res_show.get("isError").is_none());
        let text_show = res_show["content"][0]["text"].as_str().unwrap();
        assert!(text_show.contains("Testing show and diff"));

        // 2. traz_diff
        let diff_payload = json!({
            "params": {
                "name": "traz_diff",
                "arguments": {
                    "id": id
                }
            }
        });
        let res_diff = handle_tool_call(&db, &diff_payload, false).await;
        assert!(res_diff.get("isError").is_none());
        let text_diff = res_diff["content"][0]["text"].as_str().unwrap();
        assert!(text_diff.contains("+hello"));

        // 3. traz_show missing event -> isError=true
        let show_missing = json!({
            "params": {
                "name": "traz_show",
                "arguments": {
                    "id": 99999
                }
            }
        });
        let res_show_missing = handle_tool_call(&db, &show_missing, false).await;
        assert_eq!(res_show_missing["isError"], true);

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_tool_call_traz_recap() {
        let (db, test_dir) = setup_test_env("recap").await;

        let event = Event::new(
            "cursor".to_string(),
            "bug_fix".to_string(),
            "Recap item test".to_string(),
            None,
            None,
            None,
        );
        db.insert_event(&event).await.unwrap();

        let recap_payload = json!({
            "params": {
                "name": "traz_recap",
                "arguments": {
                    "hours": 12,
                    "format": "markdown"
                }
            }
        });

        let res = handle_tool_call(&db, &recap_payload, false).await;
        assert!(res.get("isError").is_none());
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Recap"));
        assert!(text.contains("Recap item test"));

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_tool_call_experimental_enabled() {
        let (db, test_dir) = setup_test_env("experimental").await;

        let event = Event::new(
            "cursor".to_string(),
            "feature".to_string(),
            "Exp event".to_string(),
            None,
            None,
            None,
        );
        let id = db.insert_event(&event).await.unwrap();

        // 1. traz_timeline
        let timeline_payload = json!({
            "params": {
                "name": "traz_timeline",
                "arguments": {}
            }
        });
        let res_timeline = handle_tool_call(&db, &timeline_payload, true).await;
        assert!(res_timeline.get("isError").is_none());
        assert!(
            res_timeline["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("Exp event")
        );

        // 2. traz_delete
        let delete_payload = json!({
            "params": {
                "name": "traz_delete",
                "arguments": {
                    "id": id
                }
            }
        });
        let res_delete = handle_tool_call(&db, &delete_payload, true).await;
        assert!(res_delete.get("isError").is_none());
        assert!(
            res_delete["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("deleted")
        );

        // Verify it was deleted
        assert!(db.get_event(id).await.unwrap().is_none());

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_handle_tool_call_bad_arguments() {
        let (db, test_dir) = setup_test_env("bad_arguments").await;

        // 1. traz_search missing query
        let bad_search = json!({
            "params": {
                "name": "traz_search",
                "arguments": {}
            }
        });
        let res_search = handle_tool_call(&db, &bad_search, false).await;
        assert_eq!(res_search["isError"], true);
        assert!(
            res_search["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("Missing required argument")
        );

        // 2. traz_show missing id
        let bad_show = json!({
            "params": {
                "name": "traz_show",
                "arguments": {}
            }
        });
        let res_show = handle_tool_call(&db, &bad_show, false).await;
        assert_eq!(res_show["isError"], true);

        // 3. traz_diff bad id format
        let bad_diff = json!({
            "params": {
                "name": "traz_diff",
                "arguments": {
                    "id": "not_an_integer"
                }
            }
        });
        let res_diff = handle_tool_call(&db, &bad_diff, false).await;
        assert_eq!(res_diff["isError"], true);

        cleanup_test_env(test_dir);
    }
}
