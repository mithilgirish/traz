mod banner;
mod cli;
mod display;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use display::{
    print_context, print_empty, print_event_detail, print_events, print_events_json, print_header,
    print_info, print_success, print_warning,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use traz_core::{Event, TrazConfig};
use traz_db::Db;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(command) => {
            let needs_tracing = matches!(&command, Commands::Serve { .. } | Commands::Mcp);
            if needs_tracing {
                tracing_subscriber::fmt::init();
            }

            let config = TrazConfig::resolve();
            let db = Arc::new(Db::open(&config.db_path)?);
            run_command(command, &config, db).await?;
        }
        None => {
            // Interactive REPL mode
            run_interactive().await?;
        }
    }

    Ok(())
}

/// Execute a single traz command
async fn run_command(command: Commands, config: &TrazConfig, db: Arc<Db>) -> Result<()> {
    match command {
        // ── Project setup ───────────────────────────────────────────
        Commands::Init { hook } => {
            print_success(&format!(
                "traz initialized. Database at: {}",
                config.db_path.display()
            ));

            if hook {
                match traz_integrations::git::install_post_commit_hook() {
                    Ok(_) => print_success("Git post-commit hook installed."),
                    Err(e) => print_empty(&format!("Failed to install hook: {}", e)),
                }
            }

            print_info("Run `traz setup <tool>` to configure integrations.");
        }

        // ── Read commands ───────────────────────────────────────────
        Commands::Recent {
            limit,
            tool,
            event_type,
            json,
        } => {
            let events = if tool.is_some() || event_type.is_some() {
                db.get_filtered_events(limit, tool, event_type, None, None)?
            } else {
                db.get_recent_events(limit)?
            };
            if events.is_empty() {
                print_empty("No events yet. Add one with `traz add`.");
            } else if json {
                print_events_json(&events);
            } else {
                print_header(&format!("Recent Events ({})", events.len()));
                print_events(&events);
                println!();
            }
        }

        Commands::Timeline { limit, json } => {
            let events = db.get_timeline(limit)?;
            if events.is_empty() {
                print_empty("Timeline is empty.");
            } else if json {
                print_events_json(&events);
            } else {
                print_header(&format!("Timeline ({} events)", events.len()));
                print_events(&events);
                println!();
            }
        }

        Commands::Search {
            query,
            limit,
            tool,
            json,
        } => {
            let events = db.search_events(&query, tool.as_deref(), limit)?;
            if events.is_empty() {
                print_empty(&format!("No events matching \"{}\".", query));
            } else if json {
                print_events_json(&events);
            } else {
                print_header(&format!(
                    "Search: \"{}\" ({} results)",
                    query,
                    events.len()
                ));
                print_events(&events);
                println!();
            }
        }

        // ── Write commands ──────────────────────────────────────────
        Commands::Add {
            tool,
            event_type,
            title,
            summary,
            files,
            tags,
            session,
            diff,
        } => {
            let files_vec = files.map(|s| {
                s.split(',')
                    .map(|f| f.trim().to_string())
                    .filter(|f| !f.is_empty())
                    .collect::<Vec<String>>()
            });

            let tags_vec = tags.map(|s| {
                s.split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect::<Vec<String>>()
            });

            let mut event = Event::new(tool, event_type, title, summary, files_vec, None);
            if let Some(t) = tags_vec {
                event = event.with_tags(t);
            }
            if let Some(s) = session {
                event = event.with_session(s);
            }
            if diff {
                if let Ok(Some(d)) = traz_integrations::git::get_uncommitted_diff() {
                    event = event.with_diff(d);
                }
            }

            let id = db.insert_event(&event)?;
            print_success(&format!("Event #{} added.", id));
        }

        Commands::Log {
            message,
            event_type,
            tool,
            diff,
        } => {
            let mut event = Event::new(tool, event_type, message, None, None, None);
            if diff {
                if let Ok(Some(d)) = traz_integrations::git::get_uncommitted_diff() {
                    event = event.with_diff(d);
                }
            }
            let id = db.insert_event(&event)?;
            print_success(&format!("Logged event #{} shorthand.", id));
        }

        Commands::Delete { id } => {
            if db.delete_event(id)? {
                print_success(&format!("Event #{} deleted.", id));
            } else {
                print_empty(&format!("Event #{} not found.", id));
            }
        }

        Commands::Undo => {
            if let Some(id) = db.get_last_event_id()? {
                let event = db.get_event(id)?;
                if db.delete_event(id)? {
                    if let Some(e) = event {
                        print_success(&format!(
                            "Undone event #{}: \"{}\" [{}]",
                            id, e.title, e.tool
                        ));
                    } else {
                        print_success(&format!("Undone event #{}.", id));
                    }
                }
            } else {
                print_empty("No events to undo.");
            }
        }

        Commands::Compress { days, summary } => {
            let (count, new_id) = db.compress_events(days, summary)?;
            if count > 0 {
                print_success(&format!(
                    "Compressed {} events older than {} days into new Epoch event #{}.",
                    count, days, new_id
                ));
            } else {
                print_info(&format!("No events older than {} days found.", days));
            }
        }

        Commands::Rewind { id } => {
            if db.get_event(id)?.is_none() {
                print_empty(&format!("Checkpoint event #{} not found.", id));
            } else {
                let deleted = db.delete_events_after(id)?;
                if deleted > 0 {
                    print_success(&format!(
                        "Rewound to event #{}. Deleted {} subsequent events.",
                        id, deleted
                    ));
                } else {
                    print_info(&format!("Already at event #{}. No events to delete.", id));
                }
            }
        }

        Commands::Show { id, json } => match db.get_event(id)? {
            Some(event) => {
                if json {
                    let j =
                        serde_json::to_string_pretty(&event).unwrap_or_else(|_| "{}".into());
                    println!("{}", j);
                } else {
                    print_event_detail(&event);
                }
            }
            None => {
                print_empty(&format!("Event #{} not found.", id));
            }
        },

        Commands::Diff { id } => {
            match db.get_event(id)? {
                Some(event) => {
                    if let Some(diff) = event.diff {
                        use std::io::IsTerminal;
                        let use_color = std::io::stdout().is_terminal();

                        for line in diff.lines() {
                            if use_color && line.starts_with('+') && !line.starts_with("+++") {
                                println!("\x1b[32m{}\x1b[0m", line); // green
                            } else if use_color
                                && line.starts_with('-')
                                && !line.starts_with("---")
                            {
                                println!("\x1b[31m{}\x1b[0m", line); // red
                            } else if use_color && line.starts_with("@@") {
                                println!("\x1b[36m{}\x1b[0m", line); // cyan
                            } else {
                                println!("{}", line);
                            }
                        }
                    } else {
                        print_empty(&format!("Event #{} has no associated code diff.", id));
                    }
                }
                None => {
                    print_empty(&format!("Event #{} not found.", id));
                }
            }
        }

        Commands::Context { limit, json } => {
            let ctx = db.get_context_summary(limit)?;
            if json {
                let data = serde_json::json!({
                    "context": ctx,
                    "format": "markdown",
                    "version": env!("CARGO_PKG_VERSION"),
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).unwrap_or_default()
                );
            } else {
                print_context(&ctx);
            }
        }

        Commands::Capture => {
            let event = traz_integrations::git::capture_latest_commit()?;
            let id = db.insert_event(&event)?;
            print_success(&format!("Captured git commit as event #{}.", id));
        }

        // ── Info commands ───────────────────────────────────────────
        Commands::Stats { json } => {
            let count = db.count_events()?;
            let by_tool = db.get_stats()?;

            if json {
                let tools: serde_json::Value = by_tool
                    .iter()
                    .map(|(tool, cnt)| {
                        serde_json::json!({ "tool": tool, "count": cnt })
                    })
                    .collect();
                let data = serde_json::json!({
                    "total_events": count,
                    "db_path": config.db_path.display().to_string(),
                    "by_tool": tools,
                    "version": env!("CARGO_PKG_VERSION"),
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).unwrap_or_default()
                );
            } else {
                print_header("traz stats");
                println!("  Total events: {}", count);
                println!("  Database:     {}", config.db_path.display());
                if !by_tool.is_empty() {
                    println!();
                    println!("  Events by tool:");
                    for (tool, cnt) in &by_tool {
                        println!("    {:<16} {}", tool, cnt);
                    }
                }
                println!();
            }
        }

        Commands::Export => {
            let events = db.get_timeline(u32::MAX)?;
            let json = serde_json::to_string_pretty(&events)?;
            println!("{}", json);
        }

        Commands::Import => {
            use std::io::Read;
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;

            let events: Vec<Event> = serde_json::from_str(&input)?;
            let mut imported = 0;
            let mut skipped = 0;

            for event in &events {
                // Skip if UUID already exists
                if let Ok(Some(_)) = db.get_event_by_uuid(&event.uuid) {
                    skipped += 1;
                    continue;
                }
                match db.insert_event(event) {
                    Ok(_) => imported += 1,
                    Err(e) => {
                        print_warning(&format!(
                            "Skipped event \"{}\": {}",
                            event.title, e
                        ));
                        skipped += 1;
                    }
                }
            }

            print_success(&format!(
                "Imported {} events ({} skipped).",
                imported, skipped
            ));
        }

        Commands::Setup { tool } => {
            let instructions = traz_integrations::adapters::setup_instructions(&tool)?;
            println!("\n{}\n", instructions);
        }

        // ── Server commands ─────────────────────────────────────────
        Commands::Serve { port } => {
            let app = traz_api::create_router(db);

            let addr = format!("127.0.0.1:{}", port);
            let listener = TcpListener::bind(&addr).await?;

            eprintln!("🚀 traz API server listening on http://{}", addr);
            eprintln!("   Press Ctrl+C to stop.\n");
            axum::serve(listener, app).await?;
        }

        Commands::Mcp => {
            traz_mcp::run_mcp_server(db).await?;
        }
    }

    Ok(())
}

/// Interactive REPL: `❯ traz` prompt
async fn run_interactive() -> Result<()> {
    banner::print_banner();

    let config = TrazConfig::resolve();
    let db = Arc::new(Db::open(&config.db_path)?);

    banner::print_interactive_welcome();

    // Set up Ctrl+C handler for graceful exit
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
        println!(); // newline after ^C
    })?;

    let stdin = std::io::stdin();
    let mut line_buf = String::new();

    loop {
        if !running.load(std::sync::atomic::Ordering::SeqCst) {
            banner::print_farewell();
            break;
        }

        banner::print_prompt();

        line_buf.clear();
        let bytes = stdin.read_line(&mut line_buf)?;
        if bytes == 0 {
            // EOF (e.g. piped input ended)
            banner::print_farewell();
            break;
        }

        let input = line_buf.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "exit" | "quit" | "q" => {
                banner::print_farewell();
                break;
            }
            "help" | "h" | "?" => {
                banner::print_interactive_help();
                continue;
            }
            "clear" | "cls" => {
                // ANSI clear screen
                print!("\x1b[2J\x1b[H");
                banner::print_banner();
                continue;
            }
            "banner" => {
                banner::print_banner();
                continue;
            }
            _ => {}
        }

        // Parse the input as a traz subcommand
        // Build argv: ["traz", ...tokens]
        let tokens = shell_split(input);
        let argv: Vec<&str> = std::iter::once("traz")
            .chain(tokens.iter().map(|s| s.as_str()))
            .collect();

        // Try to parse as a Cli
        match Cli::try_parse_from(&argv) {
            Ok(parsed) => {
                if let Some(cmd) = parsed.command {
                    // Prevent launching sub-servers inside REPL
                    match &cmd {
                        Commands::Serve { .. } => {
                            print_info("Use `traz serve` directly from your shell (not inside interactive mode).");
                            continue;
                        }
                        Commands::Mcp => {
                            print_info("Use `traz mcp` directly from your shell (not inside interactive mode).");
                            continue;
                        }
                        _ => {}
                    }
                    if let Err(e) = run_command(cmd, &config, db.clone()).await {
                        eprintln!("  \x1b[31m✗\x1b[0m {}", e);
                    }
                }
            }
            Err(e) => {
                // clap error — show a friendlier message
                let msg = e.to_string();
                // Only show the first meaningful line
                if let Some(first_line) = msg.lines().find(|l| !l.trim().is_empty()) {
                    eprintln!("  \x1b[31m✗\x1b[0m {}", first_line.trim());
                } else {
                    eprintln!(
                        "  \x1b[31m✗\x1b[0m Invalid command. Type `help` for usage."
                    );
                }
            }
        }
    }

    Ok(())
}

/// Simple shell-like splitting that respects double quotes
fn shell_split(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in input.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}
