use crate::Event;

// ─── Token Budget ────────────────────────────────────────────────────

/// Approximate token count using the ~4 chars/token heuristic.
/// This is intentionally conservative (overestimates) to avoid exceeding budgets.
pub fn estimate_tokens(text: &str) -> usize {
    // GPT-family averages ~4 chars/token for English.
    // We use 3.5 to be slightly conservative (overestimate token count).
    (text.len() as f64 / 3.5).ceil() as usize
}

/// Token budget configuration for context generation.
#[derive(Debug, Clone)]
pub struct TokenBudget {
    /// Maximum tokens for the entire output.
    pub max_tokens: usize,
    /// Tokens already consumed (tracked internally).
    consumed: usize,
}

impl TokenBudget {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            consumed: 0,
        }
    }

    /// Unlimited budget (no truncation).
    pub fn unlimited() -> Self {
        Self {
            max_tokens: usize::MAX,
            consumed: 0,
        }
    }

    pub fn remaining(&self) -> usize {
        self.max_tokens.saturating_sub(self.consumed)
    }

    pub fn consume(&mut self, text: &str) {
        self.consumed += estimate_tokens(text);
    }

    pub fn would_fit(&self, text: &str) -> bool {
        estimate_tokens(text) <= self.remaining()
    }

    pub fn is_exhausted(&self) -> bool {
        self.remaining() == 0
    }

    pub fn is_unlimited(&self) -> bool {
        self.max_tokens == usize::MAX
    }
}

// ─── Output Format ───────────────────────────────────────────────────

/// Output format for context retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Standard markdown (current behavior).
    Markdown,
    /// Dense single-line format optimized for AI token consumption.
    Dense,
    /// Token-Oriented Object Notation, highly optimized for LLMs
    Toon,
}

impl OutputFormat {
    pub fn from_str_opt(s: Option<&str>) -> Self {
        match s {
            Some("dense" | "compact" | "ai") => Self::Dense,
            Some("toon") => Self::Toon,
            _ => Self::Markdown,
        }
    }
}

// ─── Type Abbreviations ──────────────────────────────────────────────

/// Abbreviate event types to save tokens in dense mode.
pub fn abbreviate_type(event_type: &str) -> &str {
    match event_type {
        "bug_fix" => "bf",
        "feature" => "ft",
        "refactor" => "rf",
        "decision" => "dc",
        "commit" => "cm",
        "debug" => "db",
        "test" => "ts",
        "deploy" => "dp",
        "revert" => "rv",
        "epoch" => "ep",
        "checkpoint" => "cp",
        other => other,
    }
}

/// Compact relative time string (saves tokens vs the full format).
pub fn compact_relative_time(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    let delta = chrono::Utc::now().signed_duration_since(*timestamp);

    if delta.num_seconds() < 60 {
        "now".to_string()
    } else if delta.num_minutes() < 60 {
        format!("{}m", delta.num_minutes())
    } else if delta.num_hours() < 24 {
        format!("{}h", delta.num_hours())
    } else if delta.num_days() < 30 {
        format!("{}d", delta.num_days())
    } else if delta.num_days() < 365 {
        format!("{}mo", delta.num_days() / 30)
    } else {
        format!("{}y", delta.num_days() / 365)
    }
}

// ─── Diff Summarization ─────────────────────────────────────────────

/// Summarize a full unified diff into a compact string.
///
/// Example output: `"+42/-15 in 3 files (auth.rs, middleware.rs, config.rs)"`
pub fn summarize_diff(diff: &str) -> String {
    let mut additions: usize = 0;
    let mut deletions: usize = 0;
    let mut files: Vec<String> = Vec::new();

    for line in diff.lines() {
        if line.starts_with("+++ b/") || line.starts_with("+++ a/") {
            let path = line
                .trim_start_matches("+++ b/")
                .trim_start_matches("+++ a/");
            // Extract just the filename
            if let Some(name) = path.rsplit('/').next()
                && !files.contains(&name.to_string())
            {
                files.push(name.to_string());
            }
        } else if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
        }
    }

    if files.is_empty() {
        // Fallback: just count lines
        let total = diff.lines().count();
        format!("+{additions}/-{deletions} ({total} diff lines)")
    } else if files.len() <= 3 {
        format!(
            "+{additions}/-{deletions} in {} ({})",
            pluralize(files.len(), "file"),
            files.join(", ")
        )
    } else {
        let shown: Vec<&str> = files.iter().take(2).map(|s| s.as_str()).collect();
        format!(
            "+{additions}/-{deletions} in {} ({}, +{} more)",
            pluralize(files.len(), "file"),
            shown.join(", "),
            files.len() - 2
        )
    }
}

fn pluralize(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

// ─── Dense Event Formatter ──────────────────────────────────────────

/// Format a single event in dense single-line format.
///
/// Format: `[{age}] {tool}|{type}|{title}|{files}`
/// Example: `[2d] cursor|bf|Fixed JWT refresh|auth.rs,middleware.rs`
pub fn format_event_dense(event: &Event) -> String {
    let age = compact_relative_time(&event.timestamp);
    let typ = abbreviate_type(&event.event_type);

    let mut parts = vec![
        format!("[{age}]"),
        event.tool.clone(),
        typ.to_string(),
        event.title.clone(),
    ];

    // Add summary (first sentence only, max 80 chars)
    if let Some(ref summary) = event.summary {
        let first_sentence = summary
            .lines()
            .next()
            .unwrap_or(summary)
            .chars()
            .take(80)
            .collect::<String>();
        if !first_sentence.is_empty() {
            parts.push(first_sentence);
        }
    }

    // Add files (basename only, max 3)
    if let Some(ref files) = event.files
        && !files.is_empty()
    {
        let basenames: Vec<&str> = files
            .iter()
            .take(3)
            .map(|f| f.rsplit('/').next().unwrap_or(f))
            .collect();
        let files_str = if files.len() > 3 {
            format!("{}+{}", basenames.join(","), files.len() - 3)
        } else {
            basenames.join(",")
        };
        parts.push(files_str);
    }

    // Add diff summary
    if let Some(ref diff) = event.diff {
        parts.push(summarize_diff(diff));
    }

    parts.join("|")
}

// ─── Semantic Deduplication ─────────────────────────────────────────

/// Given a list of events with optional similarity scores, remove near-duplicates.
/// Returns deduplicated events with `[+N similar]` annotations in the title.
///
/// Events are compared pairwise by title similarity (Jaccard on word sets).
/// For full semantic dedup, use embedding-based comparison via `traz-db`.
pub fn deduplicate_events(events: Vec<Event>, similarity_threshold: f64) -> Vec<Event> {
    if events.len() <= 1 {
        return events;
    }

    let mut result: Vec<Event> = Vec::new();
    let mut merged_count: Vec<usize> = Vec::new();
    let mut used = vec![false; events.len()];

    // Pre-compute word sets to avoid O(N^2) string allocations
    let parsed_titles: Vec<std::collections::HashSet<&str>> = events
        .iter()
        .map(|e| e.title.split_whitespace().collect())
        .collect();

    for i in 0..events.len() {
        if used[i] {
            continue;
        }

        let mut count = 0;
        for j in (i + 1)..events.len() {
            if used[j] {
                continue;
            }

            let sim = jaccard_similarity_sets(&parsed_titles[i], &parsed_titles[j]);
            if sim >= similarity_threshold {
                used[j] = true;
                count += 1;
            }
        }

        result.push(events[i].clone());
        merged_count.push(count);
        used[i] = true;
    }

    // Annotate merged events
    for (event, count) in result.iter_mut().zip(merged_count.iter()) {
        if *count > 0 {
            event.title = format!("{} [+{} similar]", event.title, count);
        }
    }

    result
}

/// Jaccard similarity on pre-computed word sets.
fn jaccard_similarity_sets(
    set_a: &std::collections::HashSet<&str>,
    set_b: &std::collections::HashSet<&str>,
) -> f64 {
    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }

    let intersection = set_a.intersection(set_b).count();
    let union = set_a.union(set_b).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Helper for tests to keep existing test API intact.
#[cfg(test)]
fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let set_b: std::collections::HashSet<&str> = b.split_whitespace().collect();
    jaccard_similarity_sets(&set_a, &set_b)
}

// ─── Detail Level (Progressive) ─────────────────────────────────────

/// Progressive detail levels for adaptive context generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DetailLevel {
    /// Titles + tool + type only (~15 tokens/event).
    Minimal,
    /// + first-line summaries (~30 tokens/event).
    Standard,
    /// + files, tags, diff summary (~50 tokens/event).
    Full,
}

impl DetailLevel {
    /// Estimate the approximate token cost per event at this detail level.
    pub fn tokens_per_event(&self) -> usize {
        match self {
            Self::Minimal => 15,
            Self::Standard => 30,
            Self::Full => 50,
        }
    }

    /// Choose the best detail level that fits N events within a token budget.
    pub fn fit_budget(num_events: usize, available_tokens: usize) -> Self {
        if num_events == 0 {
            return Self::Full;
        }

        let budget_per_event = available_tokens / num_events;

        if budget_per_event >= Self::Full.tokens_per_event() {
            Self::Full
        } else if budget_per_event >= Self::Standard.tokens_per_event() {
            Self::Standard
        } else {
            Self::Minimal
        }
    }
}

// ─── Optimized Context Builder ──────────────────────────────────────

/// Build a token-optimized context string from events.
///
/// This is the main entry point for the optimization layer.
/// It combines all techniques: budgeting, dense formatting, dedup, diff summarization.
pub fn build_optimized_context(
    events: Vec<Event>,
    format: OutputFormat,
    budget: &mut TokenBudget,
    deduplicate: bool,
    header: Option<&str>,
) -> String {
    let events = if deduplicate {
        deduplicate_events(events, 0.85)
    } else {
        events
    };

    let mut output = String::new();

    // Header
    if let Some(h) = header {
        let header_line = match format {
            OutputFormat::Markdown => format!("{h}\n\n"),
            OutputFormat::Dense | OutputFormat::Toon => format!("# {h}\n"),
        };
        if budget.would_fit(&header_line) {
            budget.consume(&header_line);
            output.push_str(&header_line);
        }
    }

    if events.is_empty() {
        let empty_msg = "(no events)\n";
        budget.consume(empty_msg);
        output.push_str(empty_msg);
        return output;
    }

    match format {
        OutputFormat::Dense => {
            // Determine detail level based on budget
            let detail = if budget.is_unlimited() {
                DetailLevel::Full
            } else {
                DetailLevel::fit_budget(events.len(), budget.remaining())
            };

            for event in &events {
                if budget.is_exhausted() {
                    output.push_str("...(truncated)\n");
                    break;
                }

                let line = match detail {
                    DetailLevel::Minimal => {
                        let age = compact_relative_time(&event.timestamp);
                        let typ = abbreviate_type(&event.event_type);
                        format!("[{age}] {typ}|{}|{}\n", event.tool, event.title)
                    }
                    DetailLevel::Standard => {
                        let age = compact_relative_time(&event.timestamp);
                        let typ = abbreviate_type(&event.event_type);
                        let summary_bit = event
                            .summary
                            .as_deref()
                            .and_then(|s| s.lines().next())
                            .map(|s| {
                                let truncated: String = s.chars().take(60).collect();
                                format!("|{truncated}")
                            })
                            .unwrap_or_default();
                        format!(
                            "[{age}] {typ}|{}|{}{summary_bit}\n",
                            event.tool, event.title
                        )
                    }
                    DetailLevel::Full => {
                        format!("{}\n", format_event_dense(event))
                    }
                };

                if budget.would_fit(&line) {
                    budget.consume(&line);
                    output.push_str(&line);
                } else {
                    output.push_str("...(truncated)\n");
                    break;
                }
            }
        }

        OutputFormat::Markdown => {
            for event in &events {
                if budget.is_exhausted() {
                    output.push_str("\n> ⚡ *Context truncated to fit token budget.*\n");
                    break;
                }

                let ts = event.timestamp.format("%Y-%m-%d %H:%M UTC");
                let title_line = format!("### {} [{}] — {}\n", event.title, event.tool, ts);

                if !budget.would_fit(&title_line) {
                    output.push_str("\n> ⚡ *Context truncated to fit token budget.*\n");
                    break;
                }

                budget.consume(&title_line);
                output.push_str(&title_line);

                // Type
                let type_line = format!("- **Type:** {}\n", event.event_type);
                if budget.would_fit(&type_line) {
                    budget.consume(&type_line);
                    output.push_str(&type_line);
                }

                // Summary (truncate to budget)
                if let Some(ref summary) = event.summary {
                    let sum_text = summary.trim();
                    let sum_line = if !budget.is_unlimited() && sum_text.len() > 200 {
                        let truncated: String = sum_text.chars().take(200).collect();
                        format!("- **Summary:** {}…\n", truncated)
                    } else {
                        let dense_sum = sum_text.replace('\n', " ");
                        format!("- **Summary:** {}\n", dense_sum)
                    };
                    if budget.would_fit(&sum_line) {
                        budget.consume(&sum_line);
                        output.push_str(&sum_line);
                    }
                }

                // Files (limit to 3 with budget)
                if let Some(ref files) = event.files
                    && !files.is_empty()
                {
                    let files_line = if !budget.is_unlimited() && files.len() > 3 {
                        let top: Vec<&str> = files.iter().take(3).map(|s| s.as_str()).collect();
                        format!(
                            "- **Files:** {}, +{} more\n",
                            top.join(", "),
                            files.len() - 3
                        )
                    } else {
                        format!("- **Files:** {}\n", files.join(", "))
                    };
                    if budget.would_fit(&files_line) {
                        budget.consume(&files_line);
                        output.push_str(&files_line);
                    }
                }

                // Diff summary (never include full diff in context)
                if let Some(ref diff) = event.diff {
                    let diff_line = format!("- **Changes:** {}\n", summarize_diff(diff));
                    if budget.would_fit(&diff_line) {
                        budget.consume(&diff_line);
                        output.push_str(&diff_line);
                    }
                }

                // Tags (included in markdown, omitted in dense for savings)
                if let Some(ref tags) = event.tags
                    && !tags.is_empty()
                {
                    let tags_line = format!(
                        "- **Tags:** {}\n",
                        tags.iter()
                            .map(|t| format!("#{}", t))
                            .collect::<Vec<_>>()
                            .join(" ")
                    );
                    if budget.would_fit(&tags_line) {
                        budget.consume(&tags_line);
                        output.push_str(&tags_line);
                    }
                }

                output.push('\n');
            }
        }
        OutputFormat::Toon => {
            #[derive(serde::Serialize)]
            struct ToonEvent<'a> {
                tool: &'a str,
                #[serde(rename = "type")]
                event_type: &'a str,
                title: &'a str,
                age: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                summary: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                files: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                changes: Option<String>,
            }

            let mut projected_events = Vec::with_capacity(events.len());
            let mut event_costs = Vec::with_capacity(events.len());

            for event in &events {
                let summary = event
                    .summary
                    .as_ref()
                    .map(|s| s.chars().take(200).collect::<String>());
                let files = event.files.as_ref().filter(|f| !f.is_empty()).map(|f| {
                    if f.len() > 3 {
                        let top: Vec<&str> = f.iter().take(3).map(|s| s.as_str()).collect();
                        format!("{}, +{} more", top.join(", "), f.len() - 3)
                    } else {
                        f.join(", ")
                    }
                });
                let changes = event.diff.as_ref().map(|d| summarize_diff(d));

                let toon_event = ToonEvent {
                    tool: &event.tool,
                    event_type: abbreviate_type(&event.event_type),
                    title: &event.title,
                    age: compact_relative_time(&event.timestamp),
                    summary,
                    files,
                    changes,
                };

                let cost = estimate_tokens(&serde_json::to_string(&toon_event).unwrap_or_default());
                projected_events.push(toon_event);
                event_costs.push(cost);
            }

            let mut valid_count = projected_events.len();
            while valid_count > 0 {
                let current_cost: usize = event_costs[..valid_count].iter().sum();
                if budget.is_unlimited() || current_cost <= budget.remaining() {
                    break;
                }
                valid_count -= 1;
            }
            projected_events.truncate(valid_count);

            if projected_events.is_empty() {
                let empty_msg = "(no events fit in budget)\n";
                budget.consume(empty_msg);
                output.push_str(empty_msg);
            } else if let Ok(json_bytes) = serde_json::to_vec(&projected_events)
                && let Ok(toon_str) = _etoon::toon::encode(&json_bytes)
            {
                let formatted = format!("{}\n", toon_str);
                if budget.would_fit(&formatted) {
                    budget.consume(&formatted);
                    output.push_str(&formatted);
                }
            }
        }
    }

    output
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        assert!(estimate_tokens("hello world") > 0);
        assert!(estimate_tokens("") == 0);
        // ~4 chars per token average
        let long_text = "a".repeat(100);
        let tokens = estimate_tokens(&long_text);
        assert!((25..=35).contains(&tokens));
    }

    #[test]
    fn test_token_budget() {
        let mut b = TokenBudget::new(100);
        assert!(!b.is_exhausted());
        assert_eq!(b.remaining(), 100);

        b.consume("hello world test"); // ~5 tokens
        assert!(b.remaining() < 100);
        assert!(!b.is_exhausted());
    }

    #[test]
    fn test_abbreviate_type() {
        assert_eq!(abbreviate_type("bug_fix"), "bf");
        assert_eq!(abbreviate_type("feature"), "ft");
        assert_eq!(abbreviate_type("custom_type"), "custom_type");
    }

    #[test]
    fn test_summarize_diff() {
        let diff = "\
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,7 @@
+use std::io;
 fn main() {
-    println!(\"hello\");
+    println!(\"hello world\");
+    io::stdout().flush().unwrap();
 }";
        let summary = summarize_diff(diff);
        assert!(summary.contains("+3/-1"));
        assert!(summary.contains("main.rs"));
    }

    #[test]
    fn test_jaccard_similarity() {
        assert_eq!(jaccard_similarity("hello world", "hello world"), 1.0);
        assert_eq!(jaccard_similarity("hello", "world"), 0.0);
        assert!(jaccard_similarity("fixed auth bug", "fixed auth issue") >= 0.5);
    }

    #[test]
    fn test_deduplicate_events() {
        let e1 = Event::new(
            "cursor".into(),
            "bug_fix".into(),
            "Fixed auth token refresh".into(),
            None,
            None,
            None,
        );
        let e2 = Event::new(
            "claude".into(),
            "bug_fix".into(),
            "Fixed auth token refresh issue".into(),
            None,
            None,
            None,
        );
        let e3 = Event::new(
            "cursor".into(),
            "feature".into(),
            "Added websocket support".into(),
            None,
            None,
            None,
        );

        let result = deduplicate_events(vec![e1, e2, e3], 0.6);
        // e1 and e2 should be merged, e3 stays
        assert_eq!(result.len(), 2);
        assert!(result[0].title.contains("[+1 similar]"));
    }

    #[test]
    fn test_detail_level_budget() {
        // 10 events, 500 tokens → Full
        assert_eq!(DetailLevel::fit_budget(10, 500), DetailLevel::Full);
        // 100 events, 500 tokens → Minimal
        assert_eq!(DetailLevel::fit_budget(100, 500), DetailLevel::Minimal);
        // 20 events, 500 tokens → 25 tokens/event → Minimal (threshold for Standard is 30)
        assert_eq!(DetailLevel::fit_budget(20, 500), DetailLevel::Minimal);
    }

    #[test]
    fn test_format_event_dense() {
        let event = Event::new(
            "cursor".into(),
            "bug_fix".into(),
            "Fixed JWT refresh".into(),
            Some("Resolved race condition in token refresh".into()),
            Some(vec!["src/auth.rs".into(), "src/middleware.rs".into()]),
            None,
        );
        let dense = format_event_dense(&event);
        assert!(dense.contains("cursor"));
        assert!(dense.contains("bf"));
        assert!(dense.contains("Fixed JWT refresh"));
        assert!(dense.contains("auth.rs"));
    }

    #[test]
    fn test_build_optimized_context_dense() {
        let events = vec![
            Event::new(
                "cursor".into(),
                "bug_fix".into(),
                "Fixed auth".into(),
                None,
                None,
                None,
            ),
            Event::new(
                "claude".into(),
                "feature".into(),
                "Added search".into(),
                None,
                None,
                None,
            ),
        ];

        let mut budget = TokenBudget::unlimited();
        let output = build_optimized_context(
            events,
            OutputFormat::Dense,
            &mut budget,
            false,
            Some("Context"),
        );
        assert!(output.contains("bf"));
        assert!(output.contains("ft"));
    }

    #[test]
    fn test_budget_truncation() {
        let events: Vec<Event> = (0..100)
            .map(|i| {
                Event::new(
                    "tool".into(),
                    "feature".into(),
                    format!("Event number {i} with a moderately long title for testing"),
                    None,
                    None,
                    None,
                )
            })
            .collect();

        let mut budget = TokenBudget::new(200);
        let output = build_optimized_context(events, OutputFormat::Dense, &mut budget, false, None);
        assert!(output.contains("truncated"));
    }
}
