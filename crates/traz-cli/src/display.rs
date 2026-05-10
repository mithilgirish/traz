use traz_core::Event;

/// ANSI color helpers.
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const BLUE: &str = "\x1b[34m";

fn relative_time(then: &chrono::DateTime<chrono::Utc>) -> String {
    let delta = chrono::Utc::now().signed_duration_since(*then);

    if delta.num_seconds() < 60 {
        "just now".to_string()
    } else if delta.num_minutes() < 60 {
        format!("{}m ago", delta.num_minutes())
    } else if delta.num_hours() < 24 {
        format!("{}h ago", delta.num_hours())
    } else {
        format!("{}d ago", delta.num_days())
    }
}

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

pub fn print_events(events: &[Event]) {
    for (i, event) in events.iter().enumerate() {
        if i > 0 {
            println!();
        }
        print_event(event);
    }
}

pub fn print_event(event: &Event) {
    let icon = type_icon(&event.event_type);
    let rel = relative_time(&event.timestamp);

    println!(
        "  {icon} {BOLD}{}{RESET}  {DIM}[{CYAN}{}{RESET}{DIM} · {}{DIM}]{RESET}",
        event.title, event.tool, rel,
    );
    println!("    {MAGENTA}{}{RESET}", event.event_type);

    if let Some(ref summary) = event.summary {
        // Only show the first line in timeline view
        let first_line = summary.lines().next().unwrap_or(summary);
        println!("    {DIM}{}{RESET}", first_line);
    }

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

    if let Some(ref tags) = event.tags {
        if !tags.is_empty() {
            let tag_str = tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(" ");
            println!("    {DIM}{tag_str}{RESET}");
        }
    }
}

pub fn print_events_json(events: &[Event]) {
    let json = serde_json::to_string_pretty(events).unwrap_or_else(|_| "[]".into());
    println!("{}", json);
}

pub fn print_empty(msg: &str) {
    println!("  {DIM}{msg}{RESET}");
}

pub fn print_success(msg: &str) {
    println!("  {GREEN}✓{RESET} {msg}");
}

pub fn print_header(label: &str) {
    println!();
    println!("  {BOLD}{YELLOW}{label}{RESET}");
    println!("  {DIM}{}{RESET}", "─".repeat(label.len() + 2));
}

pub fn print_info(msg: &str) {
    println!("  {CYAN}ℹ{RESET} {msg}");
}
