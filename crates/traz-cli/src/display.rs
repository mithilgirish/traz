use traz_core::Event;
use std::io::IsTerminal;
use std::sync::OnceLock;

static COLOR_SUPPORTED: OnceLock<bool> = OnceLock::new();

fn use_color() -> bool {
    *COLOR_SUPPORTED.get_or_init(|| std::io::stdout().is_terminal())
}

fn c(ansi: &'static str) -> &'static str {
    if use_color() {
        ansi
    } else {
        ""
    }
}

/// ANSI color constants (raw).
const RESET_RAW: &str = "\x1b[0m";
const BOLD_RAW: &str = "\x1b[1m";
const DIM_RAW: &str = "\x1b[2m";
const CYAN_RAW: &str = "\x1b[36m";
const GREEN_RAW: &str = "\x1b[32m";
const YELLOW_RAW: &str = "\x1b[33m";
const MAGENTA_RAW: &str = "\x1b[35m";
const BLUE_RAW: &str = "\x1b[34m";

fn get_colors() -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    (
        c(RESET_RAW),
        c(BOLD_RAW),
        c(DIM_RAW),
        c(CYAN_RAW),
        c(GREEN_RAW),
        c(YELLOW_RAW),
        c(MAGENTA_RAW),
        c(BLUE_RAW),
    )
}

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
    #[allow(non_snake_case, unused_variables)]
    let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = get_colors();
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

    if let Some(ref diff) = event.diff {
        let line_count = diff.lines().count();
        println!("    {DIM}diff:{RESET} {GREEN}+{} lines{RESET}", line_count);
    }
}

pub fn print_events_json(events: &[Event]) {
    let json = serde_json::to_string_pretty(events).unwrap_or_else(|_| "[]".into());
    println!("{}", json);
}

pub fn print_empty(msg: &str) {
    #[allow(non_snake_case, unused_variables)]
    let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = get_colors();
    println!("  {DIM}{msg}{RESET}");
}

pub fn print_success(msg: &str) {
    #[allow(non_snake_case, unused_variables)]
    let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = get_colors();
    println!("  {GREEN}✓{RESET} {msg}");
}

pub fn print_header(label: &str) {
    #[allow(non_snake_case, unused_variables)]
    let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = get_colors();
    println!();
    println!("  {BOLD}{YELLOW}{label}{RESET}");
    println!("  {DIM}{}{RESET}", "─".repeat(label.len() + 2));
}

pub fn print_info(msg: &str) {
    #[allow(non_snake_case, unused_variables)]
    let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = get_colors();
    println!("  {CYAN}ℹ{RESET} {msg}");
}
