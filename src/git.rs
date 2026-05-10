use crate::models::Event;
use anyhow::{Context, Result};
use std::process::Command;

/// Capture the latest git commit from the current working directory
/// and return it as a traz `Event`.
///
/// Extracts the commit subject, body, author, and list of modified files.
pub fn capture_latest_commit() -> Result<Event> {
    // Get commit hash, subject, body, and author
    let log_output = Command::new("git")
        .args(["log", "-1", "--pretty=format:%h|||%s|||%b|||%an"])
        .output()
        .context("Failed to execute `git log`. Is this a git repository?")?;

    if !log_output.status.success() {
        let stderr = String::from_utf8_lossy(&log_output.stderr);
        anyhow::bail!("git log failed: {}", stderr.trim());
    }

    let log_str = String::from_utf8_lossy(&log_output.stdout);
    let parts: Vec<&str> = log_str.splitn(4, "|||").collect();

    let hash = parts.first().unwrap_or(&"").trim();
    let subject = parts.get(1).unwrap_or(&"").trim().to_string();
    let body = parts
        .get(2)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let author = parts.get(3).map(|s| s.trim()).unwrap_or("unknown");

    let title = if hash.is_empty() {
        subject
    } else {
        format!("{} — {}", hash, subject)
    };

    // Build summary from body + author
    let summary = match body {
        Some(b) => Some(format!("{}\n\nauthor: {}", b, author)),
        None => Some(format!("author: {}", author)),
    };

    // Get modified files
    let diff_output = Command::new("git")
        .args(["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"])
        .output()
        .context("Failed to execute `git diff-tree`")?;

    let files_str = String::from_utf8_lossy(&diff_output.stdout);
    let files: Vec<String> = files_str
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let files_opt = if files.is_empty() { None } else { Some(files) };

    Ok(Event::new(
        "git".to_string(),
        "commit".to_string(),
        title,
        summary,
        files_opt,
        None,
    ))
}
