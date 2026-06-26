use std::io::IsTerminal;
use std::sync::OnceLock;
use traz_core::Event;

static COLOR_SUPPORTED: OnceLock<bool> = OnceLock::new();

fn use_color() -> bool {
    *COLOR_SUPPORTED.get_or_init(|| std::io::stdout().is_terminal())
}

fn c(ansi: &'static str) -> &'static str {
    if use_color() { ansi } else { "" }
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

pub fn get_colors() -> (
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

pub fn relative_time(then: &chrono::DateTime<chrono::Utc>) -> String {
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

pub fn type_icon(event_type: &str) -> &'static str {
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
        "  {CYAN}◆{RESET}  {icon}  {BOLD}{}{RESET}  {DIM}[{CYAN}{}{RESET}{DIM} · {}{DIM}]{RESET}",
        event.title, event.tool, rel,
    );
    println!(
        "  {DIM}│{RESET}  {DIM}type:{RESET}  {MAGENTA}{}{RESET}",
        event.event_type
    );

    if let Some(ref summary) = event.summary {
        // Only show the first line in timeline view
        let first_line = summary.lines().next().unwrap_or(summary);
        println!("  {DIM}│{RESET}  {DIM}info:{RESET}  {first_line}");
    }

    if let Some(ref files) = event.files
        && !files.is_empty()
    {
        let file_list = files
            .iter()
            .map(|f| format!("{BLUE}{f}{RESET}"))
            .collect::<Vec<_>>()
            .join(&format!("{DIM}, {RESET}"));
        println!("  {DIM}│{RESET}  {DIM}files:{RESET} {}", file_list);
    }

    if let Some(ref tags) = event.tags
        && !tags.is_empty()
    {
        let tag_str = tags
            .iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!("  {DIM}│{RESET}  {DIM}tags:{RESET}  {YELLOW}{tag_str}{RESET}");
    }

    if let Some(ref diff) = event.diff {
        let line_count = diff.lines().count();
        println!(
            "  {DIM}│{RESET}  {DIM}diff:{RESET}  {GREEN}+{} lines{RESET}",
            line_count
        );
    }
}

pub fn print_events_json(events: &[Event]) {
    let json = serde_json::to_string_pretty(events).unwrap_or_else(|_| "[]".into());
    println!("{}", json);
}

/// Print full details of a single event (for `traz show`)
pub fn print_event_detail(event: &Event) {
    #[allow(non_snake_case, unused_variables)]
    let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = get_colors();
    let icon = type_icon(&event.event_type);
    let rel = relative_time(&event.timestamp);
    let ts = event.timestamp.format("%Y-%m-%d %H:%M:%S UTC");

    println!();
    println!("  {CYAN}◆{RESET}  {BOLD}{icon} {}{RESET}", event.title);
    println!(
        "  {DIM}├──{RESET} {DIM}ID:{RESET}        {CYAN}#{}{RESET}",
        event.id.unwrap_or(0)
    );
    println!(
        "  {DIM}├──{RESET} {DIM}UUID:{RESET}      {DIM}{}{RESET}",
        event.uuid
    );
    println!(
        "  {DIM}├──{RESET} {DIM}Tool:{RESET}      {CYAN}{}{RESET}",
        event.tool
    );
    println!(
        "  {DIM}├──{RESET} {DIM}Type:{RESET}      {MAGENTA}{}{RESET}",
        event.event_type
    );
    println!(
        "  {DIM}├──{RESET} {DIM}When:{RESET}      {} {DIM}({}){RESET}",
        ts, rel
    );

    if let Some(ref session) = event.session_id {
        println!(
            "  {DIM}├──{RESET} {DIM}Session:{RESET}   {YELLOW}{}{RESET}",
            session
        );
    }

    if let Some(ref summary) = event.summary {
        println!("  {DIM}├──{RESET} {BOLD}Summary{RESET}");
        for line in summary.lines() {
            println!("  {DIM}│{RESET}    {}", line);
        }
    }

    if let Some(ref files) = event.files
        && !files.is_empty()
    {
        println!("  {DIM}├──{RESET} {BOLD}Changed Files{RESET}");
        for (idx, f) in files.iter().enumerate() {
            let guide = if idx == files.len() - 1 {
                "  │    └── "
            } else {
                "  │    ├── "
            };
            println!("{}{BLUE}{}{RESET}", guide, f);
        }
    }

    if let Some(ref tags) = event.tags
        && !tags.is_empty()
    {
        let tag_str = tags
            .iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!("  {DIM}├──{RESET} {DIM}Tags:{RESET}     {YELLOW}{tag_str}{RESET}");
    }

    if let Some(ref diff) = event.diff {
        let line_count = diff.lines().count();
        println!(
            "  {DIM}├──{RESET} {DIM}Diff:{RESET}     {GREEN}+{} lines{RESET} {DIM}(use `traz diff {}` to view){RESET}",
            line_count,
            event.id.unwrap_or(0)
        );
    }

    if let Some(ref metadata) = event.metadata {
        println!("  {DIM}└──{RESET} {BOLD}Metadata{RESET}");
        if let Ok(pretty) = serde_json::to_string_pretty(metadata) {
            for line in pretty.lines() {
                println!("       {DIM}{}{RESET}", line);
            }
        }
    } else {
        println!("  {DIM}└──{RESET} {DIM}Metadata: (none){RESET}");
    }
    println!();
}

/// Print context summary with ANSI coloring
pub fn print_context(ctx: &str) {
    #[allow(non_snake_case, unused_variables)]
    let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = get_colors();

    for line in ctx.lines() {
        if line.starts_with("# ") {
            println!("{BOLD}{CYAN}{}{RESET}", line);
        } else if line.starts_with("## ") {
            println!("{BOLD}{YELLOW}{}{RESET}", line);
        } else if line.starts_with("### ") {
            println!("{BOLD}{}{RESET}", line);
        } else if line.starts_with("- **") {
            println!("  {DIM}{}{RESET}", line);
        } else if line.starts_with("**") {
            println!("{BOLD}{}{RESET}", line);
        } else {
            println!("{}", line);
        }
    }
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

pub fn print_warning(msg: &str) {
    #[allow(non_snake_case, unused_variables)]
    let (RESET, BOLD, DIM, CYAN, GREEN, YELLOW, MAGENTA, BLUE) = get_colors();
    println!("  {YELLOW}⚠{RESET} {msg}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use traz_core::Event;

    #[test]
    fn test_relative_time() {
        let now = Utc::now();

        // 30 seconds ago -> "just now"
        let just_now = now - Duration::seconds(30);
        assert_eq!(relative_time(&just_now), "just now");

        // 5 minutes ago -> "5m ago"
        let mins_ago = now - Duration::minutes(5);
        assert_eq!(relative_time(&mins_ago), "5m ago");

        // 3 hours ago -> "3h ago"
        let hours_ago = now - Duration::hours(3);
        assert_eq!(relative_time(&hours_ago), "3h ago");

        // 4 days ago -> "4d ago"
        let days_ago = now - Duration::days(4);
        assert_eq!(relative_time(&days_ago), "4d ago");
    }

    #[test]
    fn test_type_icon() {
        assert_eq!(type_icon("bug_fix"), "🐛");
        assert_eq!(type_icon("feature"), "✨");
        assert_eq!(type_icon("refactor"), "♻️ ");
        assert_eq!(type_icon("decision"), "📌");
        assert_eq!(type_icon("commit"), "📝");
        assert_eq!(type_icon("debug"), "🔍");
        assert_eq!(type_icon("test"), "🧪");
        assert_eq!(type_icon("deploy"), "🚀");
        assert_eq!(type_icon("revert"), "⏪");
        assert_eq!(type_icon("unknown_type"), "•");
    }

    #[test]
    fn test_get_colors() {
        let colors = get_colors();
        // Since we are running in tests (usually not a TTY or stderr/stdout captured),
        // let's verify it returns either the ANSI codes or empty strings.
        // It shouldn't panic.
        assert_eq!(colors.0.contains("\x1b[0m") || colors.0.is_empty(), true);
    }

    #[test]
    fn test_printers_execute_without_panic() {
        let event = Event::new(
            "test_tool".to_string(),
            "bug_fix".to_string(),
            "Fix something".to_string(),
            Some("Detailed line 1\nLine 2".to_string()),
            Some(vec!["file_a.txt".to_string()]),
            None,
        )
        .with_tags(vec!["tag1".to_string()])
        .with_diff("--- a/file_a.txt\n+++ b/file_a.txt\n+added".to_string());

        // Run printers to ensure they don't panic on empty or populated options
        print_event(&event);
        print_events(&[event.clone()]);
        print_events_json(&[event.clone()]);
        print_event_detail(&event);
        print_context("# Header\n## Subheader\n### Small\n- **bullet**\n**bold**\nregular text");
        print_empty("No events found");
        print_success("Done!");
        print_header("SUMMARY");
        print_info("info msg");
        print_warning("warning msg");
    }
}

