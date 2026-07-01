mod banner;
mod cli;
mod cuby;
mod display;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use display::{
    print_context, print_empty, print_event_detail, print_events, print_events_json, print_header,
    print_info, print_success, print_warning,
};
use std::io::IsTerminal;
use std::sync::Arc;
use tokio::net::TcpListener;
use traz_core::{Event, TrazConfig};
use traz_db::Db;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.print_share_dir {
        let config = TrazConfig::resolve();
        println!("{}", config.data_dir().display());
        return Ok(());
    }

    let mut config = TrazConfig::resolve();

    // Handle project-local initialization before opening the database
    if let Some(Commands::Init { .. }) = &cli.command {
        let local_dir = std::path::Path::new(".traz");
        if !local_dir.exists() {
            std::fs::create_dir_all(local_dir)?;
            let gitignore_path = std::path::Path::new(".gitignore");
            let contents = std::fs::read_to_string(gitignore_path).unwrap_or_default();
            if !contents.contains(".traz/") {
                let to_append = if !contents.ends_with('\n') && !contents.is_empty() {
                    "\n.traz/\n"
                } else {
                    ".traz/\n"
                };
                let _ = safe_write(gitignore_path, to_append, true);
            }
            // Re-resolve so we pick up the new local directory
            config = TrazConfig::resolve();
        }
    }

    match cli.command {
        Some(command) => {
            let needs_tracing = matches!(&command, Commands::Serve { .. } | Commands::Mcp);
            if needs_tracing {
                tracing_subscriber::fmt::init();
            }

            let db_existed = config.db_path.exists();
            let db = Arc::new(Db::open(&config.db_path).await?);
            run_command(command, &config, db, db_existed).await?;
        }
        None => {
            run_interactive().await?;
        }
    }

    Ok(())
}

async fn run_command(
    command: Commands,
    config: &TrazConfig,
    db: Arc<Db>,
    db_existed: bool,
) -> Result<()> {
    match command {
        // ── Project setup ───────────────────────────────────────────
        Commands::Init {
            hook,
            with_embeddings,
        } => {
            #[allow(non_snake_case, unused_variables)]
            let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();

            if db_existed {
                let count = db.count_events().await?;
                println!(
                    "  {GREEN}✓{RESET} {BOLD}Traz already initialized{RESET} (DB: {}, {} events)",
                    config.db_path.display(),
                    count
                );
            } else {
                println!("  {GREEN}✓{RESET} {BOLD}Traz initialized{RESET}");
                println!("    {DIM}DB:{RESET}   {}", config.db_path.display());

                // Auto-inject rules into common files during init for easy use
                let prompt = traz_integrations::adapters::active_sync_prompt();
                let common_files = vec![".cursorrules", "CLAUDE.md", "AGENTS.md"];

                let mut injected_count = 0;
                let cwd = std::env::current_dir().unwrap_or_default();

                if confirm_prompt(
                    "  Auto-inject Active Sync rules into current project (.cursorrules, CLAUDE.md, etc)?",
                ) {
                    for filename in common_files {
                        let path = cwd.join(filename);
                        let existing = std::fs::read_to_string(&path).unwrap_or_default();
                        if !existing.contains("traz_add") {
                            let new_content = if existing.is_empty() {
                                format!("{}\n", prompt)
                            } else {
                                format!("{}\n\n{}\n", existing, prompt)
                            };
                            if safe_write(&path, &new_content, false).is_ok() {
                                injected_count += 1;
                            }
                        }
                    }

                    // Special case for Antigravity/Gemini which needs nested directory
                    if std::fs::create_dir_all(".agents/rules").is_ok() {
                        let path = cwd.join(".agents/rules/traz.md");
                        let existing = std::fs::read_to_string(&path).unwrap_or_default();
                        if !existing.contains("traz_add") {
                            let new_content = if existing.is_empty() {
                                format!("{}\n", prompt)
                            } else {
                                format!("{}\n\n{}\n", existing, prompt)
                            };
                            if safe_write(&path, &new_content, false).is_ok() {
                                injected_count += 1;
                            }
                        }
                    }

                    // Install traz skill for Antigravity/Gemini
                    if std::fs::create_dir_all(".agents/skills/traz-memory").is_ok() {
                        let path = cwd.join(".agents/skills/traz-memory/SKILL.md");
                        let skill_content = traz_integrations::adapters::active_sync_skill();
                        if safe_write(&path, skill_content, false).is_ok() {
                            injected_count += 1;
                        }
                    }
                }

                if injected_count > 0 {
                    println!(
                        "  {GREEN}✓{RESET} {BOLD}Auto-configured Active Sync rules for Cursor, Claude, OpenCode, and agy!{RESET}"
                    );
                }

                println!(
                    "    {BOLD}Next:{RESET} run {CYAN}`traz setup claude`{RESET} (or cursor/opencode/agy) to connect the MCP server"
                );
                println!("          run {CYAN}`traz init --hook`{RESET} to install git hooks");
                println!("          run {CYAN}`traz serve`{RESET} to start the REST API on :4000");
            }

            if with_embeddings {
                println!("  Downloading embedding model (all-MiniLM-L6-v2, ~25MB)...");
                let spinner_running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
                let spinner_running_clone = spinner_running.clone();
                let spinner_handle = std::thread::spawn(move || {
                    while spinner_running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        print!(".");
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                });

                match traz_embeddings::get_embedder() {
                    Ok(_) => {
                        spinner_running.store(false, std::sync::atomic::Ordering::Relaxed);
                        let _ = spinner_handle.join();
                        println!();

                        let mut new_config = config.clone();
                        new_config.embeddings_enabled = true;
                        if let Err(e) = new_config.save() {
                            eprintln!("Error saving configuration: {}", e);
                            std::process::exit(1);
                        }

                        println!(
                            "  {GREEN}✓{RESET} {BOLD}Semantic search enabled{RESET}. Re-run `traz search` to use it."
                        );
                        println!(
                            "  {YELLOW}[EXPERIMENTAL]{RESET} Semantic search is experimental in v0.1.\n\
                            \x20\x20First search may be slow (model load ~2s). Report issues at github.com/mithilgirish/traz"
                        );
                    }
                    Err(e) => {
                        spinner_running.store(false, std::sync::atomic::Ordering::Relaxed);
                        let _ = spinner_handle.join();
                        println!();

                        let mut new_config = config.clone();
                        new_config.embeddings_enabled = false;
                        let _ = new_config.save();

                        eprintln!("Error initializing embedding model: {}", e);
                        std::process::exit(1);
                    }
                }
            }

            if hook {
                match traz_integrations::git::install_hooks(std::path::Path::new(".")) {
                    Ok(_) => {
                        let git_dir = std::process::Command::new("git")
                            .args(["rev-parse", "--git-dir"])
                            .output()
                            .ok()
                            .and_then(|out| {
                                if out.status.success() {
                                    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| ".git".to_string());

                        let git_dir_path = std::path::Path::new(&git_dir);
                        print_success(&format!(
                            "Installed post-commit hook: {}",
                            git_dir_path.join("hooks/post-commit").display()
                        ));
                        print_success(&format!(
                            "Installed post-checkout hook: {}",
                            git_dir_path.join("hooks/post-checkout").display()
                        ));
                        print_success(&format!(
                            "Installed pre-push hook: {}",
                            git_dir_path.join("hooks/pre-push").display()
                        ));
                    }
                    Err(e) => print_empty(&format!("Failed to install hooks: {}", e)),
                }
            }

            if !db_existed && !hook {
                println!();
            }
        }

        // ── Read commands ───────────────────────────────────────────
        Commands::Recent {
            limit,
            tool,
            event_type,
            json,
            dense,
            budget,
            deduplicate,
        } => {
            let events = if tool.is_some() || event_type.is_some() {
                db.get_filtered_events(limit, tool, event_type, None, None)
                    .await?
            } else {
                db.get_recent_events(limit).await?
            };
            if events.is_empty() {
                print_empty("No events yet. Add one with `traz add`.");
            } else if json {
                print_events_json(&events);
            } else if dense {
                let format = traz_core::OutputFormat::Dense;
                let mut token_budget = match budget {
                    Some(n) => traz_core::TokenBudget::new(n),
                    None => traz_core::TokenBudget::unlimited(),
                };
                let output = traz_core::build_optimized_context(
                    events,
                    format,
                    &mut token_budget,
                    deduplicate,
                    Some("Recent Events"),
                );
                println!("{}", output);
            } else {
                print_header(&format!("Recent Events ({})", events.len()));
                print_events(&events);
                println!();
            }
        }

        Commands::Recap { hours } => {
            #[allow(non_snake_case, unused_variables)]
            let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();
            let limit = 100;
            let since = chrono::Utc::now()
                - chrono::Duration::try_hours(hours as i64).unwrap_or(chrono::Duration::zero());
            let events = db
                .get_filtered_events(limit, None, None, Some(since), None)
                .await?;
            if events.is_empty() {
                print_empty(&format!(
                    "No events found in the last {} hours. You're starting fresh!",
                    hours
                ));
            } else {
                print_header(&format!("Morning Recap (Last {} hours)", hours));
                print_events(&events);
                println!();
                println!(
                    "  {DIM}Tip: Feed this into Claude or Cursor to quickly catch them up.{RESET}"
                );
                println!();
            }
        }

        Commands::Timeline { limit, json } => {
            let events = db.get_timeline(limit).await?;
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
            event_type,
            tag,
            since,
            json,
        } => {
            let limit = limit.min(50);
            #[allow(non_snake_case, unused_variables)]
            let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();

            let since_dt = since.as_deref().and_then(parse_duration);
            let filters = traz_db::SearchFilters {
                tool: tool.as_deref(),
                event_type: event_type.as_deref(),
                tag: tag.as_deref(),
                since: since_dt,
            };

            let results = db.hybrid_search(&query, &filters, limit).await?;

            if results.is_empty() {
                print_empty(&format!("No events matching \"{}\".", query));
            } else if json {
                let events: Vec<_> = results.into_iter().map(|(e, _)| e).collect();
                print_events_json(&events);
            } else {
                print_header(&format!(
                    "[search] Search: \"{}\" ({} results)",
                    query,
                    results.len()
                ));

                for (idx, (event, score)) in results.iter().enumerate() {
                    let num = idx + 1;
                    let age = display::relative_time(&event.timestamp);
                    let tags_str = event
                        .tags
                        .as_ref()
                        .map(|t| {
                            t.iter()
                                .map(|s| format!("#{s}"))
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_default();

                    let highlighted_title = highlight_term(&event.title, &query);

                    let score_str = if *score >= 0.99 {
                        format!("{GREEN}(exact match){RESET}")
                    } else {
                        format!("{GREEN}({:.0}%){RESET}", score * 100.0)
                    };

                    println!("  {:>2}. {} {}", num, highlighted_title, score_str);
                    println!(
                        "      {DIM}Tool:{RESET} {:<12} {DIM}Type:{RESET} {:<12} {DIM}Age:{RESET} {:<10} {DIM}Tags:{RESET} {}",
                        event.tool, event.event_type, age, tags_str
                    );
                    println!();
                }
            }
        }

        Commands::BackfillEmbeddings => {
            #[allow(non_snake_case, unused_variables)]
            let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();
            println!("{BOLD}Backfilling embeddings for missing events...{RESET}");
            match db.backfill_missing_embeddings().await {
                Ok(count) => println!(
                    "  {GREEN}✓{RESET} Generated embeddings for {BOLD}{count}{RESET} events."
                ),
                Err(e) => eprintln!("  {MAGENTA}✗{RESET} Error: {}", e),
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
            metadata,
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
            if let Some(m) = metadata {
                let parsed: serde_json::Value = serde_json::from_str(&m)
                    .map_err(|e| anyhow::anyhow!("Invalid metadata JSON: {}", e))?;
                event = event.with_metadata(parsed);
            }
            if diff && let Ok(Some(d)) = traz_integrations::git::get_uncommitted_diff() {
                event = event.with_diff(d);
            }

            let id = db.insert_event(&event).await?;
            print_success(&format!("Event #{} added.", id));
        }

        Commands::Log {
            message,
            event_type,
            tool,
            diff,
        } => {
            let message_trimmed = message.trim();
            if message_trimmed.is_empty() {
                anyhow::bail!("Event title/message cannot be empty.");
            }

            let mut event = Event::new(
                tool.clone(),
                event_type.clone(),
                message_trimmed.to_string(),
                None,
                None,
                None,
            );
            if diff {
                match traz_integrations::git::get_uncommitted_diff() {
                    Ok(Some(d)) => {
                        event = event.with_diff(d);
                    }
                    Ok(None) => {}
                    Err(_) => {
                        print_warning("No git repo found, logging without diff");
                    }
                }
            }
            let id = db.insert_event(&event).await?;

            #[allow(non_snake_case, unused_variables)]
            let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();
            println!(
                "  {GREEN}✓{RESET} Logged [{MAGENTA}{event_type}{RESET}] \"{BOLD}{message_trimmed}{RESET}\" (id: {CYAN}#{id}{RESET}, just now)"
            );
        }

        Commands::Delete { id } => {
            if db.delete_event(id).await? {
                print_success(&format!("Event #{} deleted.", id));
            } else {
                print_empty(&format!("Event #{} not found.", id));
            }
        }

        Commands::Undo => {
            if let Some(id) = db.get_last_event_id().await? {
                let event = db.get_event(id).await?;
                if db.delete_event(id).await? {
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
            let (count, new_id) = db.compress_events(days, summary).await?;
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
            if db.get_event(id).await?.is_none() {
                print_empty(&format!("Checkpoint event #{} not found.", id));
            } else {
                let deleted = db.delete_events_after(id).await?;
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

        Commands::Show { id, json } => match db.get_event(id).await? {
            Some(event) => {
                if json {
                    let j = serde_json::to_string_pretty(&event).unwrap_or_else(|_| "{}".into());
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
            match db.get_event(id).await? {
                Some(event) => {
                    if let Some(diff) = event.diff {
                        use std::io::IsTerminal;
                        let use_color = std::io::stdout().is_terminal();

                        for line in diff.lines() {
                            if use_color && line.starts_with('+') && !line.starts_with("+++") {
                                println!("\x1b[32m{}\x1b[0m", line); // green
                            } else if use_color && line.starts_with('-') && !line.starts_with("---")
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

        Commands::Context {
            query,
            limit,
            json,
            dense,
            budget,
            deduplicate,
        } => {
            let format = if dense {
                traz_core::OutputFormat::Dense
            } else {
                traz_core::OutputFormat::Markdown
            };
            let ctx = db
                .get_context_optimized(query.as_deref(), limit, format, budget, deduplicate)
                .await?;
            if json {
                let data = serde_json::json!({
                    "context": ctx,
                    "format": if dense { "dense" } else { "markdown" },
                    "version": env!("CARGO_PKG_VERSION"),
                    "token_estimate": traz_core::estimate_tokens(&ctx),
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
            let id = db.insert_event(&event).await?;
            print_success(&format!("Captured git commit as event #{}.", id));
        }

        // ── Info commands ───────────────────────────────────────────
        Commands::Stats { json } => {
            let count = db.count_events().await?;
            let by_tool = db.get_stats().await?;

            if json {
                let tools: serde_json::Value = by_tool
                    .iter()
                    .map(|(tool, cnt)| serde_json::json!({ "tool": tool, "count": cnt }))
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

        Commands::Status => {
            #[allow(non_snake_case, unused_variables)]
            let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();

            print_header("Traz Status");

            // 1. DB path
            println!("  {BOLD}Database:{RESET}     {}", config.db_path.display());

            // 2. Total count of events
            let count = db.count_events().await?;
            println!("  {BOLD}Total events:{RESET} {}", count);

            // 3. Last event details (handling empty DB safely)
            println!();
            println!("  {BOLD}Last Event:{RESET}");
            if let Some(last_id) = db.get_last_event_id().await? {
                if let Some(event) = db.get_event(last_id).await? {
                    let icon = display::type_icon(&event.event_type);
                    let rel = display::relative_time(&event.timestamp);
                    println!(
                        "    {} {BOLD}{}{RESET} {DIM}({} · {}){RESET}",
                        icon, event.title, event.tool, rel
                    );
                } else {
                    println!("    {DIM}(None){RESET}");
                }
            } else {
                println!("    {DIM}(No events recorded yet){RESET}");
            }

            // 4. Tool stats breakdown
            let by_tool = db.get_stats().await?;
            println!();
            println!("  {BOLD}Events by Tool:{RESET}");
            if by_tool.is_empty() {
                println!("    {DIM}(None){RESET}");
            } else {
                for (tool, cnt) in &by_tool {
                    println!("    {:<16} {}", tool, cnt);
                }
            }

            // 5. REST API status (use configured port, not hardcoded 4000)
            let api_port = config.api_port;
            let api_running = if let Ok(addr) =
                format!("127.0.0.1:{}", api_port).parse::<std::net::SocketAddr>()
            {
                if let Ok(res) = tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    tokio::net::TcpStream::connect(&addr),
                )
                .await
                {
                    res.is_ok()
                } else {
                    false
                }
            } else {
                false
            };

            println!();
            println!("  {BOLD}Services:{RESET}");
            if api_running {
                println!(
                    "    {GREEN}✓{RESET} {BOLD}REST API{RESET}      {GREEN}Running{RESET} on port {}",
                    api_port
                );
            } else {
                println!(
                    "    {DIM}✗{RESET} {BOLD}REST API{RESET}      {DIM}Not running{RESET} (start with `traz serve`)"
                );
            }

            // 6. MCP server instructions
            println!(
                "    {CYAN}ℹ{RESET} {BOLD}MCP Server{RESET}    Start with {CYAN}`traz mcp`{RESET} to connect to Cursor/Claude/OpenCode"
            );
            println!();
        }

        Commands::Tui => {
            let db_path = config.db_path.clone();
            traz_tui::run(db_path).await?;
        }

        Commands::Export => {
            let events = db.get_timeline(u32::MAX).await?;
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
                if let Ok(Some(_)) = db.get_event_by_uuid(&event.uuid).await {
                    skipped += 1;
                    continue;
                }
                match db.insert_event(event).await {
                    Ok(_) => imported += 1,
                    Err(e) => {
                        print_warning(&format!("Skipped event \"{}\": {}", event.title, e));
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
            let tool_lower = tool.to_lowercase();
            if tool_lower == "model" || tool_lower == "onnx" || tool_lower == "embeddings" {
                #[allow(non_snake_case, unused_variables)]
                let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();
                if traz_embeddings::is_embedding_model_downloaded() {
                    println!(
                        "\n  {GREEN}✓{RESET} {BOLD}Embedding Model:{RESET} Already downloaded and active."
                    );
                } else {
                    println!(
                        "\n  {MAGENTA}✗{RESET} {BOLD}Embedding Model:{RESET} NOT downloaded.\n    Run {CYAN}`traz init --with-embeddings`{RESET} to download it."
                    );
                }
            } else {
                let instructions = traz_integrations::adapters::setup_instructions(&tool)?;
                println!("\n{}\n", instructions);

                #[allow(non_snake_case, unused_variables)]
                let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();

                // Auto-configure if the tool's CLI is available
                let auto_cmd: Option<(&str, Vec<&str>)> = match tool_lower.as_str() {
                    "claude" | "claude-code" => {
                        Some(("claude", vec!["mcp", "add", "traz", "--", "traz", "mcp"]))
                    }
                    "codex" | "openai-codex" => {
                        Some(("codex", vec!["mcp", "add", "traz", "--", "traz", "mcp"]))
                    }
                    _ => None,
                };

                if let Some((cli, args)) = auto_cmd {
                    let cli_available = std::process::Command::new(cli)
                        .arg("--version")
                        .output()
                        .map(|o| o.status.success())
                        .unwrap_or(false);

                    if cli_available {
                        let cmd_str = format!("{} {}", cli, args.join(" "));
                        println!("  {GREEN}✓{RESET} {BOLD}{} CLI detected!{RESET}", cli);
                        if confirm_prompt(&format!("  Run `{CYAN}{cmd_str}{RESET}` automatically?"))
                        {
                            match std::process::Command::new(cli).args(&args).status() {
                                Ok(s) if s.success() => {
                                    println!(
                                        "  {GREEN}✓{RESET} {BOLD}traz successfully added to {}!{RESET} Restart {} to activate.",
                                        cli, cli
                                    );
                                }
                                Ok(_) => println!(
                                    "  {YELLOW}⚠{RESET} Command ran but returned a non-zero exit code. Try manually."
                                ),
                                Err(e) => println!("  {MAGENTA}✗{RESET} Failed to run: {}", e),
                            }
                        }
                        println!();
                    }
                }

                // Inject token-optimized active sync rule into local project
                let prompt = traz_integrations::adapters::active_sync_prompt();
                let rule_file = match tool_lower.as_str() {
                    "cursor" => Some(".cursorrules"),
                    "claude" | "claude-code" => Some("CLAUDE.md"),
                    "agy" | "antigravity" | "gemini" | "gemini-cli" => {
                        if let Err(e) = std::fs::create_dir_all(".agents/rules") {
                            eprintln!("Failed to create .agents/rules directory: {}", e);
                        }

                        // Install traz skill for Antigravity/Gemini
                        if std::fs::create_dir_all(".agents/skills/traz-memory").is_ok() {
                            let cwd = std::env::current_dir().unwrap_or_default();
                            let path = cwd.join(".agents/skills/traz-memory/SKILL.md");
                            let skill_content = traz_integrations::adapters::active_sync_skill();
                            let _ = safe_write(&path, skill_content, false);
                        }

                        Some(".agents/rules/traz.md")
                    }
                    "aider" => Some("CONVENTIONS.md"),
                    "copilot" | "github-copilot" | "vscode" => {
                        // GitHub Copilot uses this standard path for project-level instructions
                        if let Err(e) = std::fs::create_dir_all(".github") {
                            eprintln!("Failed to create .github directory: {}", e);
                        }
                        Some(".github/copilot-instructions.md")
                    }
                    "codex" | "openai-codex" | "opencode" => Some("AGENTS.md"),
                    "warp" => Some("Warp.md"),
                    // Fallback for any other generic AI tool
                    _ => Some("AI_INSTRUCTIONS.md"),
                };

                if let Some(filename) = rule_file
                    && let Ok(cwd) = std::env::current_dir()
                {
                    let path = cwd.join(filename);
                    let existing = std::fs::read_to_string(&path).unwrap_or_default();
                    if !existing.contains("traz_add") {
                        let new_content = if existing.is_empty() {
                            format!("{}\n", prompt)
                        } else {
                            format!("{}\n\n{}\n", existing, prompt)
                        };
                        if safe_write(&path, &new_content, false).is_ok() {
                            #[allow(non_snake_case, unused_variables)]
                            let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) =
                                display::get_colors();
                            println!(
                                "  {GREEN}✓{RESET} {BOLD}Active Sync Rule injected into {}!{RESET} (Token optimized)",
                                filename
                            );
                        }
                    }
                }

                // Special: Cursor doesn't have a CLI, write config file directly
                if tool_lower == "cursor" {
                    #[allow(non_snake_case, unused_variables)]
                    let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) =
                        display::get_colors();
                    if let Some(home) = dirs::home_dir() {
                        let cursor_dir = home.join(".cursor");
                        let mcp_path = cursor_dir.join("mcp.json");

                        // Read existing config or create new
                        let existing: serde_json::Value = if mcp_path.exists() {
                            std::fs::read_to_string(&mcp_path)
                                .ok()
                                .and_then(|s| serde_json::from_str(&s).ok())
                                .unwrap_or_else(|| serde_json::json!({"mcpServers": {}}))
                        } else {
                            serde_json::json!({"mcpServers": {}})
                        };

                        let mut cursor_config = existing;
                        cursor_config["mcpServers"]["traz"] = serde_json::json!({
                            "command": "traz",
                            "args": ["mcp"]
                        });

                        let json_str =
                            serde_json::to_string_pretty(&cursor_config).unwrap_or_default();

                        if confirm_prompt(&format!(
                            "  Auto-configure Cursor by writing to {}?",
                            mcp_path.display()
                        )) {
                            if let Err(e) = std::fs::create_dir_all(&cursor_dir) {
                                println!("  {MAGENTA}✗{RESET} Could not create ~/.cursor/: {}", e);
                            } else if let Err(e) = safe_write(&mcp_path, &json_str, false) {
                                println!(
                                    "  {MAGENTA}✗{RESET} Could not write {}: {}",
                                    mcp_path.display(),
                                    e
                                );
                            } else {
                                println!(
                                    "  {GREEN}✓{RESET} {BOLD}traz added to Cursor!{RESET} Restart Cursor to activate."
                                );
                            }
                        }
                        println!();
                    }
                }

                // Special: OpenCode doesn't have a CLI, write config file directly
                if tool_lower == "opencode" {
                    #[allow(non_snake_case, unused_variables)]
                    let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) =
                        display::get_colors();
                    if let Some(home) = dirs::home_dir() {
                        let opencode_dir = if cfg!(windows) {
                            std::env::var("APPDATA")
                                .map(std::path::PathBuf::from)
                                .unwrap_or_else(|_| home.join(".config").join("opencode"))
                        } else {
                            home.join(".config").join("opencode")
                        };
                        let mcp_path = opencode_dir.join("opencode.jsonc");

                        // Helper: strip single-line // comments from JSONC before parsing
                        // to avoid silently destroying user config that contains comments
                        let strip_jsonc_comments = |s: &str| -> String {
                            s.lines()
                                .map(|line| {
                                    // Simple comment stripping: remove // comments outside strings
                                    // This handles the common case; full JSONC parsing would require a dedicated crate
                                    let trimmed = line.trim_start();
                                    if trimmed.starts_with("//") { "" } else { line }
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        };

                        // Read existing config or create new
                        let existing: serde_json::Value = if mcp_path.exists() {
                            match std::fs::read_to_string(&mcp_path) {
                                Ok(contents) => {
                                    let stripped = strip_jsonc_comments(&contents);
                                    match serde_json::from_str(&stripped) {
                                        Ok(val) => val,
                                        Err(e) => {
                                            println!(
                                                "  {YELLOW}⚠{RESET} Could not parse {}: {}. Will only add traz MCP config.",
                                                mcp_path.display(),
                                                e
                                            );
                                            serde_json::json!({"mcp": {}})
                                        }
                                    }
                                }
                                Err(_) => serde_json::json!({"mcp": {}}),
                            }
                        } else {
                            let json_fallback = opencode_dir.join("opencode.json");
                            if json_fallback.exists() {
                                std::fs::read_to_string(&json_fallback)
                                    .ok()
                                    .and_then(|s| serde_json::from_str(&s).ok())
                                    .unwrap_or_else(|| serde_json::json!({"mcp": {}}))
                            } else {
                                serde_json::json!({"mcp": {}})
                            }
                        };

                        let mut opencode_config = existing;
                        if !opencode_config.is_object() {
                            opencode_config = serde_json::json!({"mcp": {}});
                        } else if opencode_config.get("mcp").is_none() {
                            opencode_config["mcp"] = serde_json::json!({});
                        }
                        opencode_config["mcp"]["traz"] = serde_json::json!({
                            "type": "local",
                            "command": ["traz", "mcp"],
                            "enabled": true
                        });

                        let json_str =
                            serde_json::to_string_pretty(&opencode_config).unwrap_or_default();

                        if confirm_prompt(&format!(
                            "  Auto-configure OpenCode by writing to {}?",
                            mcp_path.display()
                        )) {
                            if let Err(e) = std::fs::create_dir_all(&opencode_dir) {
                                println!(
                                    "  {MAGENTA}✗{RESET} Could not create config directory: {}",
                                    e
                                );
                            } else if let Err(e) = safe_write(&mcp_path, &json_str, false) {
                                println!(
                                    "  {MAGENTA}✗{RESET} Could not write {}: {}",
                                    mcp_path.display(),
                                    e
                                );
                            } else {
                                println!(
                                    "  {GREEN}✓{RESET} {BOLD}traz added to OpenCode!{RESET} Restart OpenCode to activate."
                                );
                            }
                        }
                        println!();
                    }
                }

                // Special: Antigravity/Gemini workspace MCP configuration
                if tool_lower == "agy"
                    || tool_lower == "antigravity"
                    || tool_lower == "gemini"
                    || tool_lower == "gemini-cli"
                {
                    #[allow(non_snake_case, unused_variables)]
                    let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) =
                        display::get_colors();
                    if let Ok(cwd) = std::env::current_dir() {
                        let agents_dir = cwd.join(".agents");
                        let mcp_path = agents_dir.join("mcp_config.json");

                        // Read existing config or create new
                        let existing: serde_json::Value = if mcp_path.exists() {
                            std::fs::read_to_string(&mcp_path)
                                .ok()
                                .and_then(|s| serde_json::from_str(&s).ok())
                                .unwrap_or_else(|| serde_json::json!({"mcpServers": {}}))
                        } else {
                            serde_json::json!({"mcpServers": {}})
                        };

                        let mut agy_config = existing;
                        if !agy_config.is_object() {
                            agy_config = serde_json::json!({"mcpServers": {}});
                        } else if agy_config.get("mcpServers").is_none() {
                            agy_config["mcpServers"] = serde_json::json!({});
                        }

                        agy_config["mcpServers"]["traz"] = serde_json::json!({
                            "command": "traz",
                            "args": ["mcp"]
                        });

                        let json_str =
                            serde_json::to_string_pretty(&agy_config).unwrap_or_default();

                        if confirm_prompt(&format!(
                            "  Auto-configure Antigravity workspace by writing to {}?",
                            mcp_path.display()
                        )) {
                            if let Err(e) = std::fs::create_dir_all(&agents_dir) {
                                println!(
                                    "  {MAGENTA}✗{RESET} Could not create .agents/ directory: {}",
                                    e
                                );
                            } else if let Err(e) = safe_write(&mcp_path, &json_str, false) {
                                println!(
                                    "  {MAGENTA}✗{RESET} Could not write {}: {}",
                                    mcp_path.display(),
                                    e
                                );
                            } else {
                                println!(
                                    "  {GREEN}✓{RESET} {BOLD}traz added to Antigravity workspace config!{RESET} Restart your agy session to activate."
                                );
                            }
                        }
                        println!();
                    }
                }

                if (tool_lower == "claude"
                    || tool_lower == "claude-code"
                    || tool_lower == "cursor"
                    || tool_lower == "gemini"
                    || tool_lower == "agy"
                    || tool_lower == "antigravity"
                    || tool_lower == "codex"
                    || tool_lower == "openai-codex"
                    || tool_lower == "opencode")
                    && !traz_embeddings::is_embedding_model_downloaded()
                {
                    println!(
                        "  {YELLOW}⚠️  [WARNING]{RESET} {BOLD}Embedding model is missing.{RESET} Semantic search will not work.\n   Run {CYAN}`traz init --with-embeddings`{RESET} to download the model.\n"
                    );
                }
            }
        }

        Commands::Doctor => {
            #[allow(non_snake_case, unused_variables)]
            let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();

            print_header("traz doctor");
            println!("  Diagnosing local-first engineering memory layer installation...\n");

            // 1. Check Data Directory & DB Path
            println!("  {BOLD}1. Directory & Storage Checks:{RESET}");
            println!("     - Shared Data Dir:  {}", config.data_dir().display());
            println!("     - Database Path:    {}", config.db_path.display());

            let db_status = if config.db_path.exists() {
                format!("{GREEN}✓ Exists & Active{RESET}")
            } else {
                format!("{YELLOW}⚠ Missing (will be created on first write){RESET}")
            };
            println!("     - SQLite Database:  {}", db_status);

            // 2. Check SQLite Connection and FTS5 Support
            println!("\n  {BOLD}2. Database Engine Checks:{RESET}");

            let mut db_healthy = false;
            let mut fts5_supported = false;

            match Db::open(&config.db_path).await {
                Ok(database) => {
                    db_healthy = true;
                    println!("     - Connection:       {GREEN}✓ Connection Succeeded{RESET}");

                    // Run real FTS5 support check in-memory or on database connection
                    fts5_supported = database.check_fts5_support().await;

                    if fts5_supported {
                        println!("     - SQLite FTS5:      {GREEN}✓ Enabled & Compiled{RESET}");
                    } else {
                        println!(
                            "     - SQLite FTS5:      {MAGENTA}✗ Disabled/Missing{RESET}\n       {YELLOW}⚠️  [CRITICAL] Keyword search requires FTS5 support in your SQLite compilation.{RESET}"
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "     - Connection:       {MAGENTA}✗ Failed to Open DB: {}{RESET}",
                        e
                    );
                    println!("     - SQLite FTS5:      {DIM}Skipped (DB failed to open){RESET}");
                }
            }

            // 3. Check Embedding Model (ONNX)
            println!("\n  {BOLD}3. Local Machine Learning Checks:{RESET}");
            let embeddings_enabled = config.embeddings_enabled;
            println!(
                "     - Config Status:    {}",
                if embeddings_enabled {
                    format!("{GREEN}Enabled{RESET}")
                } else {
                    format!("{DIM}Disabled{RESET}")
                }
            );

            let model_ok = traz_embeddings::is_embedding_model_downloaded();
            if model_ok {
                println!(
                    "     - ONNX Model:       {GREEN}✓ Downloaded & Ready{RESET} (all-MiniLM-L6-v2)"
                );
            } else {
                let msg = if embeddings_enabled {
                    format!(
                        "{MAGENTA}✗ Missing (Enabled in config but files missing){RESET}\n       {YELLOW}⚠️  [WARNING] Run `traz init --with-embeddings` to download the model files.{RESET}"
                    )
                } else {
                    format!(
                        "{DIM}Not Downloaded{RESET} (Optional, run `traz init --with-embeddings` to setup)"
                    )
                };
                println!("     - ONNX Model:       {}", msg);
            }

            // Summary
            println!("\n  ──────────────────────────────────────────────────────────");

            if db_healthy && fts5_supported && (!embeddings_enabled || model_ok) {
                println!(
                    "  {GREEN}🎉 Everything looks solid! traz is fully healthy on this machine.{RESET}"
                );
            } else if !fts5_supported {
                println!(
                    "  {MAGENTA}❌ Installation issues found. SQLite FTS5 support is missing on this machine.{RESET}"
                );
            } else {
                println!(
                    "  {YELLOW}⚠ Setup incomplete. Run `traz init --with-embeddings` to enable semantic capabilities.{RESET}"
                );
            }
            println!();
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

        Commands::Hook { platform, event } => {
            use std::io::Read;
            let mut stdin_data = String::new();
            let _ = std::io::stdin().read_to_string(&mut stdin_data);

            match traz_integrations::hooks::handle_hook(&db, &platform, &event, &stdin_data).await {
                Ok(response) => {
                    println!("{}", response);
                }
                Err(e) => {
                    eprintln!("Hook execution failed: {}", e);
                    let err_output = serde_json::json!({
                        "continue": true,
                        "systemMessage": format!("traz hook execution failed: {}", e)
                    });
                    println!("{}", err_output);
                }
            }
        }

        Commands::Cuby { subcommand, args } => {
            cuby::handle_cuby_command(&subcommand, &args, db).await?;
        }
    }

    Ok(())
}

async fn run_interactive() -> Result<()> {
    let config = TrazConfig::resolve();
    let db = Arc::new(Db::open(&config.db_path).await?);

    let is_first_run = db.count_events().await.unwrap_or(0) == 0;

    if is_first_run {
        banner::print_banner();
        println!("  Welcome to traz! The shared memory layer for AI coding tools.\n");
        use dialoguer::{Input, Select, theme::ColorfulTheme};
        let selections = &[
            "🔌 Setup an AI tool (Claude, Cursor, etc)",
            "📝 Log my first manual event",
            "📖 Skip to interactive REPL",
            "🚪 Exit",
        ];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("It looks like this is your first time here. What would you like to do?")
            .default(0)
            .items(&selections[..])
            .interact()?;

        println!();
        match selection {
            0 => {
                let tool: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Which tool? (claude, cursor, gemini, opencode, aider, warp)")
                    .interact_text()?;
                run_command(
                    Commands::Setup {
                        tool: tool.to_lowercase(),
                    },
                    &config,
                    db.clone(),
                    true,
                )
                .await?;
            }
            1 => {
                let message: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("What are you working on right now?")
                    .interact_text()?;
                run_command(
                    Commands::Log {
                        message,
                        event_type: "decision".to_string(),
                        tool: "cli".to_string(),
                        diff: false,
                    },
                    &config,
                    db.clone(),
                    true,
                )
                .await?;
            }
            3 => {
                banner::print_farewell();
                return Ok(());
            }
            _ => {}
        }
    } else {
        println!();
        println!("  \x1b[38;5;240m─────────────────────────────────────────────────────\x1b[0m");
        println!();
        banner::print_interactive_welcome();
    }

    // Set up Ctrl+C handler for graceful exit
    ctrlc::set_handler(move || {
        println!(); // newline after ^C
        banner::print_farewell();
        std::process::exit(0);
    })?;

    let stdin = std::io::stdin();
    let mut line_buf = String::new();

    loop {
        banner::print_prompt();

        line_buf.clear();
        let bytes = stdin.read_line(&mut line_buf)?;
        if bytes == 0 {
            banner::print_farewell();
            break;
        }

        let input = line_buf.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "/exit" | "/quit" | "/q" => {
                banner::print_farewell();
                break;
            }
            "help" | "h" | "?" => {
                banner::print_interactive_help();
                continue;
            }
            "clear" | "cls" => {
                print!("\x1b[2J\x1b[H");
                if is_first_run {
                    banner::print_banner();
                }
                continue;
            }
            "banner" => {
                banner::print_banner();
                continue;
            }
            "tui" => {
                let db_path = config.db_path.clone();
                if let Err(e) = traz_tui::run(db_path).await {
                    eprintln!("  \x1b[31m✗\x1b[0m Failed to launch TUI: {}", e);
                }
                continue;
            }
            _ => {}
        }

        let tokens = shell_split(input);
        let argv: Vec<&str> = std::iter::once("traz")
            .chain(tokens.iter().map(|s| s.as_str()))
            .collect();

        match Cli::try_parse_from(&argv) {
            Ok(parsed) => {
                if let Some(cmd) = parsed.command {
                    match &cmd {
                        Commands::Serve { .. } => {
                            eprintln!(
                                "  \x1b[33m⚠\x1b[0m Cannot run server inside interactive mode. Run `traz serve` directly."
                            );
                        }
                        Commands::Mcp => {
                            eprintln!(
                                "  \x1b[33m⚠\x1b[0m Cannot run MCP server inside interactive mode. Run `traz mcp` directly."
                            );
                        }
                        _ => {
                            if let Err(e) = run_command(cmd, &config, db.clone(), true).await {
                                eprintln!("  \x1b[31m✗\x1b[0m Error: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let msg = e.to_string();
                if let Some(first_line) = msg.lines().find(|l| !l.trim().is_empty()) {
                    eprintln!("  \x1b[31m✗\x1b[0m {}", first_line.trim());
                } else {
                    eprintln!("  \x1b[31m✗\x1b[0m Invalid command. Type `help` for usage.");
                }
            }
        }
    }

    Ok(())
}

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

fn should_use_color() -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    if std::env::var("TERM").unwrap_or_default() == "dumb" {
        return false;
    }
    std::io::stdout().is_terminal()
}

fn highlight_term(title: &str, query: &str) -> String {
    if query.is_empty() || !should_use_color() {
        return title.to_string();
    }

    let query_lower = query.to_lowercase();
    let title_lower = title.to_lowercase();

    let mut result = String::new();
    let mut last_idx = 0;

    while let Some(start_idx) = title_lower[last_idx..].find(&query_lower) {
        let match_start = last_idx + start_idx;
        let match_end = match_start + query.len();

        result.push_str(&title[last_idx..match_start]);
        result.push_str("\x1b[1m");
        result.push_str(&title[match_start..match_end]);
        result.push_str("\x1b[0m");

        last_idx = match_end;
    }

    result.push_str(&title[last_idx..]);
    result
}

fn parse_duration(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let now = chrono::Utc::now();
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let chars: String = s.chars().filter(|c| c.is_alphabetic()).collect();
    let num_str: String = s.chars().filter(|c| c.is_numeric()).collect();
    let num: i64 = num_str.parse().ok()?;

    match chars.as_str() {
        "d" | "day" | "days" => Some(now - chrono::Duration::try_days(num)?),
        "w" | "week" | "weeks" => Some(now - chrono::Duration::try_weeks(num)?),
        "m" | "month" | "months" => Some(now - chrono::Duration::try_days(num * 30)?),
        "y" | "year" | "years" => Some(now - chrono::Duration::try_days(num * 365)?),
        _ => None,
    }
}

/// Safely write or append to a file, aborting if the target is a symlink.
/// This mitigates symlink-overwrite attacks in cloned malicious repositories.
fn safe_write(path: &std::path::Path, content: &str, append: bool) -> std::io::Result<()> {
    if std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(std::io::Error::other(
            "Refusing to write to a symlink for security reasons",
        ));
    }

    if append {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        write!(file, "{}", content)
    } else {
        std::fs::write(path, content)
    }
}

/// Helper to prompt the user for a Yes/No confirmation interactively.
fn confirm_prompt(prompt_msg: &str) -> bool {
    use std::io::Write;
    print!("{} [Y/n] ", prompt_msg);
    std::io::stdout().flush().ok();
    let mut response = String::new();
    std::io::stdin().read_line(&mut response).ok();
    let trimmed = response.trim().to_lowercase();
    trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
}
