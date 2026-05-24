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
    if let Some(Commands::Init { local: true, .. }) = &cli.command {
        let local_dir = std::path::Path::new(".traz");
        if !local_dir.exists() {
            std::fs::create_dir_all(local_dir)?;
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
            let db = Arc::new(Db::open(&config.db_path)?);
            run_command(command, &config, db, db_existed).await?;
        }
        None => {
            run_interactive().await?;
        }
    }

    Ok(())
}

async fn run_command(command: Commands, config: &TrazConfig, db: Arc<Db>, db_existed: bool) -> Result<()> {
    match command {
        // ── Project setup ───────────────────────────────────────────
        Commands::Init { hook, with_embeddings, local: _ } => {
            #[allow(non_snake_case, unused_variables)]
            let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();

            if db_existed {
                let count = db.count_events()?;
                println!(
                    "  {GREEN}✓{RESET} {BOLD}Traz already initialized{RESET} (DB: {}, {} events)",
                    config.db_path.display(),
                    count
                );
            } else {
                println!("  {GREEN}✓{RESET} {BOLD}Traz initialized{RESET}");
                println!("    {DIM}DB:{RESET}   {}", config.db_path.display());
                println!("    {BOLD}Next:{RESET} run {CYAN}`traz setup claude`{RESET} to connect Claude Code");
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
                        print_success(&format!("Installed post-commit hook: {}", git_dir_path.join("hooks/post-commit").display()));
                        print_success(&format!("Installed post-checkout hook: {}", git_dir_path.join("hooks/post-checkout").display()));
                        print_success(&format!("Installed pre-push hook: {}", git_dir_path.join("hooks/pre-push").display()));
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

            let results = db.hybrid_search(&query, &filters, limit)?;

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
                    let tags_str = event.tags.as_ref()
                        .map(|t| t.iter().map(|s| format!("#{s}")).collect::<Vec<_>>().join(" "))
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
            let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();
            println!("{BOLD}Backfilling embeddings for missing events...{RESET}");
            match db.backfill_missing_embeddings() {
                Ok(count) => println!("  {GREEN}✓{RESET} Generated embeddings for {BOLD}{count}{RESET} events."),
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
            let message_trimmed = message.trim();
            if message_trimmed.is_empty() {
                anyhow::bail!("Event title/message cannot be empty.");
            }

            let mut event = Event::new(tool.clone(), event_type.clone(), message_trimmed.to_string(), None, None, None);
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
            let id = db.insert_event(&event)?;

            #[allow(non_snake_case, unused_variables)]
            let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();
            println!(
                "  {GREEN}✓{RESET} Logged [{MAGENTA}{event_type}{RESET}] \"{BOLD}{message_trimmed}{RESET}\" (id: {CYAN}#{id}{RESET}, just now)"
            );
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

        Commands::Status => {
            #[allow(non_snake_case, unused_variables)]
            let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = display::get_colors();

            print_header("Traz Status");

            // 1. DB path
            println!("  {BOLD}Database:{RESET}     {}", config.db_path.display());

            // 2. Total count of events
            let count = db.count_events()?;
            println!("  {BOLD}Total events:{RESET} {}", count);

            // 3. Last event details (handling empty DB safely)
            println!();
            println!("  {BOLD}Last Event:{RESET}");
            if let Some(last_id) = db.get_last_event_id()? {
                if let Some(event) = db.get_event(last_id)? {
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
            let by_tool = db.get_stats()?;
            println!();
            println!("  {BOLD}Events by Tool:{RESET}");
            if by_tool.is_empty() {
                println!("    {DIM}(None){RESET}");
            } else {
                for (tool, cnt) in &by_tool {
                    println!("    {:<16} {}", tool, cnt);
                }
            }

            // 5. REST API status
            let api_running = if let Ok(addr) = "127.0.0.1:4000".parse::<std::net::SocketAddr>() {
                if let Ok(res) = tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    tokio::net::TcpStream::connect(&addr),
                )
                .await {
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
                    "    {GREEN}✓{RESET} {BOLD}REST API{RESET}      {GREEN}Running{RESET} on port 4000"
                );
            } else {
                println!(
                    "    {DIM}✗{RESET} {BOLD}REST API{RESET}      {DIM}Not running{RESET} (start with `traz serve`)"
                );
            }

            // 6. MCP server instructions
            println!(
                "    {CYAN}ℹ{RESET} {BOLD}MCP Server{RESET}    Start with {CYAN}`traz mcp`{RESET} to connect to Cursor/Claude"
            );
            println!();
        }

        Commands::Tui => {
            let db_path = config.db_path.clone();
            traz_tui::run(db_path)?;
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

async fn run_interactive() -> Result<()> {
    banner::print_banner();
    banner::print_interactive_welcome();

    let config = TrazConfig::resolve();
    let db = Arc::new(Db::open(&config.db_path)?);

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
            "tui" => {
                let db_path = config.db_path.clone();
                if let Err(e) = traz_tui::run(db_path) {
                    eprintln!("  \x1b[31m✗\x1b[0m Failed to launch TUI: {}", e);
                }
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
                    if let Err(e) = run_command(cmd, &config, db.clone(), true).await {
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
    if s.is_empty() { return None; }
    
    let chars: String = s.chars().filter(|c| c.is_alphabetic()).collect();
    let num_str: String = s.chars().filter(|c| c.is_numeric()).collect();
    let num: i64 = num_str.parse().ok()?;

    match chars.as_str() {
        "d" | "day" | "days" => Some(now - chrono::Duration::try_days(num)?),
        "w" | "week" | "weeks" => Some(now - chrono::Duration::try_weeks(num)?),
        "m" | "month" | "months" => Some(now - chrono::Duration::try_days(num * 30)?),
        "y" | "year" | "years" => Some(now - chrono::Duration::try_days(num * 365)?),
        _ => None
    }
}
