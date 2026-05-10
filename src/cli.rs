use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "traz",
    about = "A shared engineering context layer for AI coding tools",
    long_about = "traz is a local-first developer memory layer that captures debugging history,\n\
                  architectural decisions, and workflow traces — making that context available\n\
                  to every AI tool in your stack.\n\n\
                  No cloud. No auth. Everything stays on your machine.",
    version,
    after_help = "Examples:\n  \
                  traz add --tool cursor --event-type bug_fix --title \"Fixed reconnect\"\n  \
                  traz recent --limit 5\n  \
                  traz search \"memory leak\"\n  \
                  traz capture\n  \
                  traz serve --port 3000\n  \
                  traz mcp"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show recent engineering events (newest first)
    Recent {
        /// Maximum number of events to display
        #[arg(short, long, default_value_t = 10)]
        limit: u32,

        /// Output as raw JSON instead of formatted text
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Show the full chronological timeline (oldest first)
    Timeline {
        /// Output as raw JSON instead of formatted text
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Search events by keyword across titles, summaries, tools, and files
    Search {
        /// Search query
        query: String,

        /// Output as raw JSON instead of formatted text
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Add a new engineering event to the timeline
    Add {
        /// Source tool name (e.g. cursor, aider, claude, gemini)
        #[arg(long)]
        tool: String,

        /// Event category (e.g. bug_fix, refactor, feature, decision)
        #[arg(long)]
        event_type: String,

        /// Short descriptive title
        #[arg(long)]
        title: String,

        /// Longer summary explaining context and reasoning
        #[arg(long)]
        summary: Option<String>,

        /// Comma-separated list of affected files
        #[arg(long)]
        files: Option<String>,
    },

    /// Delete an event by its ID
    Delete {
        /// Event ID to delete
        id: i64,
    },

    /// Show database statistics and storage info
    Stats,

    /// Start the local REST API server for tool integrations
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value_t = 4000)]
        port: u16,
    },

    /// Capture the latest git commit as a traz event
    Capture,

    /// Start the MCP (Model Context Protocol) stdio server
    Mcp,

    /// Export all events as a JSON array to stdout
    Export,
}
