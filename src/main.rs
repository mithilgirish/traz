mod api;
mod cli;
mod db;
mod display;
mod git;
mod mcp;
mod models;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use db::Db;
use display::{print_empty, print_events, print_events_json, print_header, print_success};
use models::Event;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Resolve the database path.
/// Uses `$TRAZ_DB` env var if set, otherwise `~/.local/share/traz/traz.db`.
fn get_db_path() -> PathBuf {
    if let Ok(custom) = std::env::var("TRAZ_DB") {
        return PathBuf::from(custom);
    }
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("traz");
    path.push("traz.db");
    path
}

#[tokio::main]
async fn main() -> Result<()> {
    // Only init tracing for the serve/mcp commands to keep CLI output clean
    let cli = Cli::parse();

    let needs_tracing = matches!(&cli.command, Commands::Serve { .. } | Commands::Mcp);
    if needs_tracing {
        tracing_subscriber::fmt::init();
    }

    let db_path = get_db_path();
    let db = Db::new(&db_path)?;

    match cli.command {
        // ── Read commands ───────────────────────────────────────────

        Commands::Recent { limit, json } => {
            let events = db.get_recent_events(limit)?;
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

        Commands::Timeline { json } => {
            let events = db.get_timeline(1000)?;
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

        Commands::Search { query, json } => {
            let events = db.search_events(&query, 100)?;
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
        } => {
            let files_vec = files.map(|s| {
                s.split(',')
                    .map(|f| f.trim().to_string())
                    .filter(|f| !f.is_empty())
                    .collect::<Vec<String>>()
            });

            let event = Event::new(tool, event_type, title, summary, files_vec, None);
            let id = db.insert_event(&event)?;
            print_success(&format!("Event #{} added.", id));
        }

        Commands::Delete { id } => {
            if db.delete_event(id)? {
                print_success(&format!("Event #{} deleted.", id));
            } else {
                print_empty(&format!("Event #{} not found.", id));
            }
        }

        Commands::Capture => {
            let event = git::capture_latest_commit()?;
            let id = db.insert_event(&event)?;
            print_success(&format!("Captured git commit as event #{}.", id));
        }

        // ── Info commands ───────────────────────────────────────────

        Commands::Stats => {
            let count = db.count_events()?;
            let by_tool = db.get_stats()?;

            print_header("traz stats");
            println!("  Total events: {}", count);
            println!("  Database:     {}", db.path().display());
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

        // ── Server commands ─────────────────────────────────────────

        Commands::Serve { port } => {
            let db_arc = Arc::new(db);
            let app = api::create_router(db_arc);

            let addr = format!("127.0.0.1:{}", port);
            let listener = TcpListener::bind(&addr).await?;

            eprintln!("🚀 traz API server listening on http://{}", addr);
            eprintln!("   Press Ctrl+C to stop.\n");
            axum::serve(listener, app).await?;
        }

        Commands::Mcp => {
            let db_arc = Arc::new(db);
            mcp::run_mcp_server(db_arc).await?;
        }
    }

    Ok(())
}
