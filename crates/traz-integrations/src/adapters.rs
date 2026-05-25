use anyhow::Result;

/// Generate MCP configuration JSON for Claude Code.
///
/// Users can add this to their `claude_desktop_config.json` to enable
/// traz as an MCP provider.
pub fn claude_mcp_config() -> serde_json::Value {
    serde_json::json!({
        "mcpServers": {
            "traz": {
                "command": "traz",
                "args": ["mcp"],
                "env": {}
            }
        }
    })
}

/// Generate MCP configuration JSON for Cursor.
///
/// Users can add this to their Cursor MCP settings.
pub fn cursor_mcp_config() -> serde_json::Value {
    serde_json::json!({
        "mcpServers": {
            "traz": {
                "command": "traz",
                "args": ["mcp"],
                "env": {}
            }
        }
    })
}

/// Print setup instructions for a specific tool.
pub fn setup_instructions(tool: &str) -> Result<String> {
    match tool {
        "claude" | "claude-code" => Ok(format!(
            "Claude Code MCP Integration\n\
             ───────────────────────────\n\
             Add this to your claude_desktop_config.json:\n\n\
             {}\n\n\
             # Set TRAZ_EXPERIMENTAL=1 to unlock timeline, delete, compress tools\n\n\
             Config location:\n\
             • macOS: ~/Library/Application Support/Claude/claude_desktop_config.json\n\
             • Linux: ~/.config/Claude/claude_desktop_config.json\n\
             • Windows: %APPDATA%/Claude/claude_desktop_config.json",
            serde_json::to_string_pretty(&claude_mcp_config())?
        )),

        "cursor" => Ok(format!(
            "Cursor MCP Integration\n\
             ──────────────────────\n\
             Add this to your Cursor MCP settings:\n\n\
             {}\n\n\
             Go to: Cursor Settings → MCP → Add Server",
            serde_json::to_string_pretty(&cursor_mcp_config())?
        )),

        "gemini" | "gemini-cli" => Ok("Gemini CLI Integration\n\
             ──────────────────────\n\
             Add this to your ~/.gemini/settings.json:\n\n\
             {\n\
               \"mcpServers\": {\n\
                 \"traz\": {\n\
                   \"command\": \"traz\",\n\
                   \"args\": [\"mcp\"]\n\
                 }\n\
               }\n\
             }"
        .to_string()),

        "git" => {
            let post_commit = crate::git::generate_post_commit_hook();
            let post_checkout = crate::git::generate_post_checkout_hook();
            let pre_push = crate::git::generate_pre_push_hook();
            Ok(format!(
                "Git Hooks Integration\n\
                 ─────────────────────\n\
                 Run: traz init --hook\n\n\
                 This installs three git hooks that automatically capture events.\n\n\
                 1. Post-Commit Hook:\n\
                 ───────────────────\n\
                 {}\n\
                 2. Post-Checkout Hook:\n\
                 ─────────────────────\n\
                 {}\n\
                 3. Pre-Push Hook:\n\
                 ────────────────\n\
                 {}\n",
                post_commit, post_checkout, pre_push
            ))
        }

        "shell" => {
            let zsh_script_name = "traz-shell-hook.zsh";
            let bash_script_name = "traz-shell-hook.bash";

            let mut zsh_path = None;
            let mut bash_path = None;

            if let Ok(exe_path) = std::env::current_exe()
                && let Some(exe_dir) = exe_path.parent()
            {
                // Try exe_dir/scripts/
                let d1 = exe_dir.join("scripts");
                if d1.join(zsh_script_name).exists() {
                    zsh_path = Some(d1.join(zsh_script_name));
                    bash_path = Some(d1.join(bash_script_name));
                }

                // Try exe_dir/../scripts/
                if zsh_path.is_none()
                    && let Some(parent) = exe_dir.parent()
                {
                    let d2 = parent.join("scripts");
                    if d2.join(zsh_script_name).exists() {
                        zsh_path = Some(d2.join(zsh_script_name));
                        bash_path = Some(d2.join(bash_script_name));
                    }

                    // Try exe_dir/../../scripts/
                    if zsh_path.is_none()
                        && let Some(grandparent) = parent.parent()
                    {
                        let d3 = grandparent.join("scripts");
                        if d3.join(zsh_script_name).exists() {
                            zsh_path = Some(d3.join(zsh_script_name));
                            bash_path = Some(d3.join(bash_script_name));
                        }
                    }
                }
            }

            let zsh_str = match zsh_path {
                Some(p) => p.to_string_lossy().into_owned(),
                None => "$(traz --print-share-dir)/scripts/traz-shell-hook.zsh".to_string(),
            };

            let bash_str = match bash_path {
                Some(p) => p.to_string_lossy().into_owned(),
                None => "$(traz --print-share-dir)/scripts/traz-shell-hook.bash".to_string(),
            };

            Ok(format!(
                "Shell Failure Tracker Integration\n\
                 ───────────────────────────────\n\
                 Add the following lines to your shell configuration files to enable the failure tracker:\n\n\
                 # Add to ~/.zshrc:\n\
                 source {}\n\n\
                 # Add to ~/.bashrc:\n\
                 source {}\n",
                zsh_str, bash_str
            ))
        }

        _ => Ok("Generic MCP Integration\n\
             ───────────────────────\n\
             traz exposes an MCP stdio server. Run:\n\n\
               traz mcp\n\n\
             Or use the REST API:\n\n\
               traz serve\n\n\
             Then point your tool at http://localhost:4000/events"
            .to_string()),
    }
}
