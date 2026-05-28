use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "traz",
    about = "A shared engineering context layer for AI coding tools",
    long_about = "traz is a local-first developer memory layer that captures debugging history,\n\
                  architectural decisions, and workflow traces — making that context available\n\
                  to every AI tool in your stack.\n\n\
                  No cloud. No auth. Everything stays on your machine.\n\n\
                  Run `traz` with no arguments to enter interactive mode.",
    version,
    after_help = "Examples:\n  \
                  traz add --tool cursor --event-type bug_fix --title \"Fixed reconnect\"\n  \
                  traz recent --limit 5\n  \
                  traz search \"memory leak\"\n  \
                  traz context\n  \
                  traz show 42\n  \
                  traz capture\n  \
                  traz serve --port 4000\n  \
                  traz setup claude\n  \
                  traz mcp"
)]
pub struct Cli {
    /// Print the shared data directory path and exit
    #[arg(long, default_value_t = false)]
    pub print_share_dir: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize traz for this project (install git hooks, etc.)
    Init {
        /// Also install a git post-commit hook
        #[arg(long, default_value_t = false)]
        hook: bool,

        /// Initialize with semantic search embeddings (fastembed model)
        #[arg(long, default_value_t = false)]
        with_embeddings: bool,

        /// Create a project-local .traz directory for isolation
        #[arg(long, default_value_t = false)]
        local: bool,
    },

    /// Show recent engineering events (newest first)
    Recent {
        /// Maximum number of events to display
        #[arg(short, long, default_value_t = 10)]
        limit: u32,

        /// Filter by tool
        #[arg(long)]
        tool: Option<String>,

        /// Filter by event type
        #[arg(long = "type")]
        event_type: Option<String>,

        /// Output as raw JSON
        #[arg(long, default_value_t = false)]
        json: bool,

        /// Use dense AI-optimized format (~50-70% fewer tokens)
        #[arg(long, default_value_t = false)]
        dense: bool,

        /// Maximum token budget (truncates output to fit)
        #[arg(long)]
        budget: Option<usize>,

        /// Merge near-duplicate events
        #[arg(long, default_value_t = false)]
        deduplicate: bool,
    },

    /// Show the full chronological timeline (oldest first)
    Timeline {
        /// Maximum number of events
        #[arg(short, long, default_value_t = 200)]
        limit: u32,

        /// Output as raw JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Search events by keyword across titles, summaries, tools, and files
    Search {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(short, long, default_value_t = 10)]
        limit: u32,

        /// Filter by tool
        #[arg(long)]
        tool: Option<String>,

        /// Filter by event type
        #[arg(long = "type")]
        event_type: Option<String>,

        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,

        /// Filter since a specific duration string (e.g. "1w" for one week ago, "3d" for 3 days ago)
        #[arg(long)]
        since: Option<String>,

        /// Output as raw JSON
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

        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,

        /// Session identifier to group related events
        #[arg(long)]
        session: Option<String>,

        /// Capture uncommitted git diff automatically
        #[arg(long, default_value_t = false)]
        diff: bool,

        /// Custom JSON metadata string to associate with the event
        #[arg(long)]
        metadata: Option<String>,
    },

    /// Quickly log a manual event shorthand
    Log {
        /// Short descriptive title or message
        message: String,

        /// Event category (defaults to decision)
        #[arg(long = "type", default_value = "decision")]
        event_type: String,

        /// Source tool (defaults to cli)
        #[arg(long, default_value = "cli")]
        tool: String,

        /// Capture uncommitted git diff automatically
        #[arg(long, default_value_t = false)]
        diff: bool,
    },

    /// Delete an event by its ID
    Delete {
        /// Event ID to delete
        id: i64,
    },

    /// Undo the last event (deletes the most recent entry)
    Undo,

    /// Compress older events into a single summary epoch to save context size
    Compress {
        /// Number of days old an event must be to be compressed
        #[arg(short, long, default_value_t = 14)]
        days: u32,

        /// The summary text for the epoch event
        #[arg(short, long)]
        summary: String,
    },

    /// Rewind the timeline to a specific event ID (deletes all events after it)
    Rewind {
        /// The event ID to use as a checkpoint (this event is kept)
        id: i64,
    },

    /// Show full details of a specific event
    Show {
        /// Event ID to display
        id: i64,

        /// Output as raw JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Show the full code diff for a specific event
    Diff {
        /// Event ID to view the diff for
        id: i64,
    },

    /// Generate a context summary for AI agents (markdown)
    Context {
        /// Optional semantic query to fetch only context relevant to your current task
        #[arg(short, long)]
        query: Option<String>,

        /// Number of recent events to include
        #[arg(short, long, default_value_t = 10)]
        limit: u32,

        /// Output as raw JSON instead of markdown
        #[arg(long, default_value_t = false)]
        json: bool,

        /// Use dense AI-optimized format (~50-70% fewer tokens)
        #[arg(long, default_value_t = false)]
        dense: bool,

        /// Maximum token budget (truncates output to fit)
        #[arg(long)]
        budget: Option<usize>,

        /// Merge near-duplicate events
        #[arg(long, default_value_t = false)]
        deduplicate: bool,
    },

    /// Show database statistics and storage info
    Stats {
        /// Output as raw JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Show current traz status and database stats
    Status,

    /// Start the interactive terminal timeline explorer (TUI dashboard)
    Tui,

    /// Import events from JSON on stdin
    Import,

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

    /// Show integration setup instructions for a specific tool
    Setup {
        /// Tool name: claude, cursor, gemini, git
        tool: String,
    },

    /// Generate embeddings for events that are missing them (e.g., from before semantic search was enabled)
    BackfillEmbeddings,

    /// Troubleshoot current traz installation (SQLite FTS5, embedding model files, data path)
    Doctor,
}
