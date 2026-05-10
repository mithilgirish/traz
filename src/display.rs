use crate::models::Event;

/// ANSI color helpers — no external crate needed.
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const BLUE: &str = "\x1b[34m";

/// Format a duration between `then` and now as a human-friendly string.
fn relative_time(then: &chrono::DateTime<chrono::Utc>) -> String {
    let delta = chrono::Utc::now().signed_duration_since(*then);

    if delta.num_seconds() < 60 {
        "just now".to_string()
    } else if delta.num_minutes() < 60 {
        let m = delta.num_minutes();
        format!("{}m ago", m)
    } else if delta.num_hours() < 24 {
        let h = delta.num_hours();
        format!("{}h ago", h)
    } else {
        let d = delta.num_days();
        format!("{}d ago", d)
    }
}

/// Map common event types to a symbol for quick scanning.
fn type_icon(event_type: &str) -> &'static str {
    match event_type {
        "bug_fix" => "🐛",
        "feature" => "✨",
        "refactor" => "♻️ ",
        "decision" => "📌",
        "commit" => "📝",
        "debug" => "🔍",
        "test" => "🧪",
        "deploy" => "🚀",
        "revert" => "⏪",
        _ => "•",
    }
}

// ── Public render functions ─────────────────────────────────────────

/// Print a list of events with rich formatting.
pub fn print_events(events: &[Event]) {
    for (i, event) in events.iter().enumerate() {
        if i > 0 {
            println!();
        }
        print_event(event);
    }
}

/// Print a single event with colors, icons, and relative timestamps.
pub fn print_event(event: &Event) {
    let icon = type_icon(&event.event_type);
    let rel = relative_time(&event.timestamp);

    // Line 1: icon + title + [tool · relative time]
    println!(
        "  {icon} {BOLD}{}{RESET}  {DIM}[{CYAN}{}{RESET}{DIM} · {}{DIM}]{RESET}",
        event.title, event.tool, rel,
    );

    // Line 2: event type tag
    println!(
        "    {MAGENTA}{}{RESET}",
        event.event_type,
    );

    // Line 3: summary (if present)
    if let Some(ref summary) = event.summary {
        println!("    {DIM}{}{RESET}", summary);
    }

    // Line 4: files (if present)
    if let Some(ref files) = event.files {
        if !files.is_empty() {
            let file_list = files
                .iter()
                .map(|f| format!("{BLUE}{f}{RESET}"))
                .collect::<Vec<_>>()
                .join(&format!("{DIM}, {RESET}"));
            println!("    {DIM}files:{RESET} {}", file_list);
        }
    }
}

/// Print events as a compact JSON array to stdout.
pub fn print_events_json(events: &[Event]) {
    let json = serde_json::to_string_pretty(events).unwrap_or_else(|_| "[]".into());
    println!("{}", json);
}

/// Print a "no results" message with consistent styling.
pub fn print_empty(msg: &str) {
    println!("  {DIM}{msg}{RESET}");
}

/// Print a success message.
pub fn print_success(msg: &str) {
    println!("  {GREEN}✓{RESET} {msg}");
}

/// Print a header / section label.
pub fn print_header(label: &str) {
    println!();
    println!("  {BOLD}{YELLOW}{label}{RESET}");
    println!("  {DIM}{}{RESET}", "─".repeat(label.len() + 2));
}
