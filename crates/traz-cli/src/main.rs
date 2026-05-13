mod cli;
mod display;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use display::{
    print_empty, print_events, print_events_json, print_header, print_info, print_success,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use traz_core::{Event, TrazConfig};
use traz_db::Db;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let needs_tracing = matches!(&cli.command, Commands::Serve { .. } | Commands::Mcp);
    if needs_tracing {
        tracing_subscriber::fmt::init();
    }

    let config = TrazConfig::resolve();
    let db = Db::open(&config.db_path)?;

    match cli.command {
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
        Commands::Recent { limit, tool, event_type, json } => {
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
                print_header(&format!("Search: \"{}\" ({} results)", query, events.len()));
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

        Commands::Log { message, event_type, tool, diff } => {
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

        Commands::Diff { id } => {
            match db.get_event(id)? {
                Some(event) => {
                    if let Some(diff) = event.diff {
                        use std::io::IsTerminal;
                        let use_color = std::io::stdout().is_terminal();
                        
                        for line in diff.lines() {
                            if use_color && line.starts_with('+') && !line.starts_with("+++") {
                                println!("\x1b[32m{}\x1b[0m", line); // green
                            } else if use_color && line.starts_with('-') && !line.starts_with("---") {
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

        Commands::Capture => {
            let event = traz_integrations::git::capture_latest_commit()?;
            let id = db.insert_event(&event)?;
            print_success(&format!("Captured git commit as event #{}.", id));
        }

        // ── Info commands ───────────────────────────────────────────
        Commands::Stats => {
            let count = db.count_events()?;
            let by_tool = db.get_stats()?;

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

        Commands::Export => {
            let events = db.get_timeline(u32::MAX)?;
            let json = serde_json::to_string_pretty(&events)?;
            println!("{}", json);
        }

        Commands::Setup { tool } => {
            let instructions = traz_integrations::adapters::setup_instructions(&tool)?;
            println!("\n{}\n", instructions);
        }

        // ── Server commands ─────────────────────────────────────────
        Commands::Serve { port } => {
            let db_arc = Arc::new(db);
            let app = traz_api::create_router(db_arc);

            let addr = format!("127.0.0.1:{}", port);
            let listener = TcpListener::bind(&addr).await?;

            eprintln!("🚀 traz API server listening on http://{}", addr);
            eprintln!("   Press Ctrl+C to stop.\n");
            axum::serve(listener, app).await?;
        }

        Commands::Mcp => {
            let db_arc = Arc::new(db);
            traz_mcp::run_mcp_server(db_arc).await?;
        }
    }

    Ok(())
}
