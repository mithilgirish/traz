use anyhow::{Context, Result};
use std::path::Path;
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
            let s = String::from_utf8_lossy(&init_output.stdout)
                .trim()
                .to_string();
            if s.is_empty() {
                return Ok(None);
            }
            return Ok(Some(s));
        }
        return Ok(None);
    }

    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() { Ok(None) } else { Ok(Some(s)) }
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
traz capture 2>/dev/null || true
"#
    .to_string()
}

/// Generate a post-checkout hook script that auto-captures branch switches.
pub fn generate_post_checkout_hook() -> String {
    r#"#!/bin/sh
# traz: auto-capture branch switches as engineering events
prev_ref="$1"
new_ref="$2"
flag="$3"

current_branch=$(git branch --show-current)
traz add --tool git --event-type branch_switch \
  --title "Switched to $current_branch" \
  --summary "From $prev_ref to $new_ref" \
  --metadata "{\"prev_ref\":\"$prev_ref\",\"new_ref\":\"$new_ref\",\"checkout_type\":\"$flag\"}" 2>/dev/null || true
"#
    .to_string()
}

/// Generate a pre-push hook script that auto-captures push events.
pub fn generate_pre_push_hook() -> String {
    r#"#!/bin/sh
# traz: auto-capture git pushes as engineering events
current_branch=$(git branch --show-current)
remote_url=$(cat .git/config 2>/dev/null | grep url | head -1 | awk '{print $3}')
commits=$(git log "origin/$current_branch..HEAD" --oneline 2>/dev/null | head -10)

traz add --tool git --event-type pre_push \
  --title "Pre-push: $current_branch → ${remote_url:-unknown}" \
  --summary "${commits:-No upstream branch or no new commits.}" 2>/dev/null || true
"#
    .to_string()
}

/// Helper function to safely write or append a git hook, ensuring it's executable.
fn write_or_append_hook(hook_path: &Path, content: &str, marker: &str) -> Result<()> {
    if hook_path.exists() {
        let existing = std::fs::read_to_string(hook_path)?;
        if existing.contains(marker) {
            return Ok(()); // Already installed
        }

        let mut new_content = existing;
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }

        // Remove the shebang if appending to an existing hook
        let lines: Vec<&str> = content.lines().collect();
        let content_to_append = if lines.first().map(|l| l.starts_with("#!")).unwrap_or(false) {
            lines[1..].join("\n")
        } else {
            content.to_string()
        };

        new_content.push_str(&content_to_append);
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        std::fs::write(hook_path, new_content)?;
    } else {
        if let Some(parent) = hook_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(hook_path, content)?;
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(hook_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(hook_path, perms)?;
    }

    Ok(())
}

/// Install the post-commit hook in the current git repo.
pub fn install_post_commit_hook(repo_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(repo_path)
        .output()
        .context("Not a git repository")?;

    let git_dir_raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let git_dir = Path::new(&git_dir_raw);
    let git_dir = if git_dir.is_absolute() {
        git_dir.to_path_buf()
    } else {
        repo_path.join(git_dir)
    };

    let hook_path = git_dir.join("hooks").join("post-commit");
    write_or_append_hook(&hook_path, &generate_post_commit_hook(), "traz capture")
}

/// Install the post-checkout hook in the current git repo.
pub fn install_post_checkout_hook(repo_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(repo_path)
        .output()
        .context("Not a git repository")?;

    let git_dir_raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let git_dir = Path::new(&git_dir_raw);
    let git_dir = if git_dir.is_absolute() {
        git_dir.to_path_buf()
    } else {
        repo_path.join(git_dir)
    };

    let hook_path = git_dir.join("hooks").join("post-checkout");
    write_or_append_hook(
        &hook_path,
        &generate_post_checkout_hook(),
        "traz add --tool git --event-type branch_switch",
    )
}

/// Install the pre-push hook in the current git repo.
pub fn install_pre_push_hook(repo_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(repo_path)
        .output()
        .context("Not a git repository")?;

    let git_dir_raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let git_dir = Path::new(&git_dir_raw);
    let git_dir = if git_dir.is_absolute() {
        git_dir.to_path_buf()
    } else {
        repo_path.join(git_dir)
    };

    let hook_path = git_dir.join("hooks").join("pre-push");
    write_or_append_hook(
        &hook_path,
        &generate_pre_push_hook(),
        "traz add --tool git --event-type pre_push",
    )
}

/// Install all three hooks in the current git repo.
pub fn install_hooks(repo_path: &Path) -> Result<()> {
    install_post_commit_hook(repo_path)?;
    install_post_checkout_hook(repo_path)?;
    install_pre_push_hook(repo_path)?;
    Ok(())
}
