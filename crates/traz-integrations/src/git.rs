use anyhow::{Context, Result};
use std::process::Command;
use traz_core::Event;

/// Capture the latest git commit from the current working directory
/// and return it as a traz `Event`.
///
/// Extracts the commit subject, body, author, branch, and list of modified files.
pub fn capture_latest_commit() -> Result<Event> {
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

    // Get current branch
    let branch = get_current_branch().unwrap_or_else(|_| "unknown".to_string());

    // Build summary from body + author + branch
    let summary = match body {
        Some(b) => Some(format!("{}\n\nauthor: {}\nbranch: {}", b, author, branch)),
        None => Some(format!("author: {}\nbranch: {}", author, branch)),
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

    // Get actual diff patch content
    let patch_output = Command::new("git")
        .args(["show", "--pretty=format:", "-U3", "HEAD"])
        .output()
        .ok();
    let diff_patch = patch_output
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let mut event = Event::new(
        "git".to_string(),
        "commit".to_string(),
        title,
        summary,
        files_opt,
        None,
    )
    .with_metadata(serde_json::json!({
        "commit_hash": hash,
        "branch": branch,
        "author": author,
    }));

    if let Some(patch) = diff_patch {
        event = event.with_diff(patch);
    }

    Ok(event)
}

/// Get uncommitted changes (staged and unstaged) in the repository as a diff patch.
pub fn get_uncommitted_diff() -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["diff", "HEAD", "--no-color"])
        .output()
        .context("Failed to run git diff")?;

    if !output.status.success() {
        // Head might not exist yet if completely empty repo
        let init_output = Command::new("git")
            .args(["diff", "--no-color"])
            .output()
            .context("Failed to run git diff fallback")?;
        if init_output.status.success() {
            let s = String::from_utf8_lossy(&init_output.stdout).trim().to_string();
            if s.is_empty() { return Ok(None); }
            return Ok(Some(s));
        }
        return Ok(None);
    }

    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}

/// Get the name of the current git branch.
pub fn get_current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .context("Failed to get current branch")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Generate a post-commit hook script that auto-captures commits.
pub fn generate_post_commit_hook() -> String {
    r#"#!/bin/sh
# traz: auto-capture git commits as engineering events
# Install: cp this to .git/hooks/post-commit && chmod +x .git/hooks/post-commit
traz capture 2>/dev/null || true
"#
    .to_string()
}

/// Install the post-commit hook in the current git repo.
/// Fails if a hook already exists to avoid overwriting user scripts.
pub fn install_post_commit_hook() -> Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .context("Not a git repository")?;

    let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let hook_path = std::path::Path::new(&git_dir)
        .join("hooks")
        .join("post-commit");

    if hook_path.exists() {
        let content = std::fs::read_to_string(&hook_path)?;
        if content.contains("traz capture") {
            return Ok(()); // Already installed
        }
        anyhow::bail!(
            "A post-commit hook already exists at {}. Please add 'traz capture' to it manually.",
            hook_path.display()
        );
    }

    std::fs::create_dir_all(hook_path.parent().unwrap())?;
    std::fs::write(&hook_path, generate_post_commit_hook())?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}
