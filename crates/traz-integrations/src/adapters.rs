use anyhow::Result;

/// Provides a highly token-optimized system prompt rule for AI agents.
/// Instructs the agent to only sync when meaningful work is done, keeping summaries short.
pub fn active_sync_prompt() -> &'static str {
    "## Traz — Developer Memory Layer (Active Sync Rules)\n\
    \n\
    This project uses `traz` for persistent AI memory via MCP. The following rules are MANDATORY:\n\
    \n\
    ### READ (Do this proactively)\n\
    - At session start or when asked about project history/context: call `traz_recent` (limit 10).\n\
    - Before fixing a bug or investigating an issue: call `traz_search` with relevant keywords.\n\
    - Before touching core/architectural code: call `traz_context` for a project overview.\n\
    \n\
    ### WRITE (Do this after significant work)\n\
    - After completing a feature, bug fix, refactor, or key decision: call `traz_add` ONCE.\n\
    - Use the appropriate `event_type`: bug_fix | feature | decision | refactor | investigation | performance | security | config | note\n\
    - Keep summaries concise (1–3 sentences). Include what changed and why. Do NOT log chitchat.\n\
    \n\
    ### CHECKPOINT (Do this before risky operations)\n\
    - Before major refactors: call `traz_checkpoint` to mark a safe restore point.\n\
    \n\
    ### Available MCP Tools\n\
    `traz_recent` · `traz_search` · `traz_add` · `traz_context` · `traz_recap` · `traz_checkpoint` · `traz_show` · `traz_diff` · `traz_stats`"
}

/// Returns the content of the traz skill file for AI agents.
pub fn active_sync_skill() -> &'static str {
    include_str!("../../../SKILL.md")
}

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

/// Generate MCP configuration JSON for OpenCode.
///
/// Users can add this to their `opencode.jsonc` configuration file.
pub fn opencode_mcp_config() -> serde_json::Value {
    serde_json::json!({
        "mcp": {
            "traz": {
                "type": "local",
                "command": ["traz", "mcp"],
                "enabled": true
            }
        }
    })
}

/// Print setup instructions for a specific tool.
pub fn setup_instructions(tool: &str) -> Result<String> {
    match tool {
        "opencode" => Ok(format!(
            "OpenCode MCP Integration\n\
             ────────────────────────\n\
             Add this to your global OpenCode configuration file (~/.config/opencode/opencode.jsonc)\n\
             or to your project-scoped config file (opencode.jsonc) in the project root:\n\n\
             {}\n\n\
             Note: OpenCode reads `AGENTS.md` for project-specific agent instructions.\n\
             Traz injects active sync rules into AGENTS.md so the agent always reads/writes memory correctly.\n\
             Run `traz setup opencode` in your project directory to configure this automatically.",
            serde_json::to_string_pretty(&opencode_mcp_config())?
        )),

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

        "codex" | "openai-codex" => Ok("OpenAI Codex CLI Integration\n\
             ─────────────────────────────\n\
             Run this command to connect traz:\n\n\
               codex mcp add traz -- traz mcp\n\n\
             Alternatively, add this to ~/.codex/config.toml manually:\n\n\
             [[mcp_servers]]\n\
             name = \"traz\"\n\
             command = \"traz\"\n\
             args = [\"mcp\"]\n\n\
             Project instructions are read from AGENTS.md in the project root.\n\
             Traz injects active sync rules into AGENTS.md automatically on setup.\n\
             Codex reads the MCP server's 'instructions' field at initialization — traz uses\n\
             this to immediately surface your project context."
            .to_string()),

        "agy" | "antigravity" => Ok("Antigravity (agy) MCP Integration\n\
             ──────────────────────────────────\n\
             Add this to your workspace config file (.agents/mcp_config.json) under your project root:\n\n\
             {\n\
               \"mcpServers\": {\n\
                 \"traz\": {\n\
                   \"command\": \"traz\",\n\
                   \"args\": [\"mcp\"]\n\
                 }\n\
               }\n\
             }\n\n\
             Alternatively, you can add it globally to ~/.gemini/config/mcp_config.json.\n\n\
             Tip: Once connected, agy will automatically retrieve your traz checkpoint at\n\
             the start of every chat session!\n\
             Project rules will be read from .agents/rules/traz.md and skills from .agents/skills/traz-memory/."
            .to_string()),

        "copilot" | "github-copilot" | "vscode" => Ok(
            "GitHub Copilot / VS Code MCP Integration\n\
             ─────────────────────────────────────────\n\
             Add this to your .vscode/mcp.json (workspace) or User settings.json (global):\n\n\
             {\n\
               \"servers\": {\n\
                 \"traz\": {\n\
                   \"type\": \"stdio\",\n\
                   \"command\": \"traz\",\n\
                   \"args\": [\"mcp\"]\n\
                 }\n\
               }\n\
             }\n\n\
             Project instructions are read from .github/copilot-instructions.md.\n\
             Traz injects active sync rules into this file automatically on setup.\n\n\
             To enable in VS Code:\n\
             • Install the GitHub Copilot extension (v1.300+)\n\
             • Enable agent mode: enable MCP support in Copilot settings\n\
             • Restart VS Code to activate the traz MCP server\n\
             • In Copilot Chat, use @workspace or Agent mode to access traz tools"
                .to_string()
        ),

        "aider" => Ok(
            "Aider Integration\n\
             ─────────────────\n\
             Aider reads project conventions from CONVENTIONS.md in the project root.\n\
             Traz injects active sync rules into CONVENTIONS.md automatically on setup.\n\n\
             To use traz context with Aider, add the --read flag:\n\n\
               aider --read CONVENTIONS.md\n\n\
             Or add it to your .aider.conf.yml:\n\n\
               read:\n\
                 - CONVENTIONS.md\n\n\
             Note: Aider does not natively support MCP. Traz memory is surfaced via\n\
             CONVENTIONS.md context injection only. For full MCP support, use Claude Code,\n\
             OpenCode, Codex, or Cursor instead."
                .to_string()
        ),

        _ => Ok("Generic MCP Integration\n\
             ───────────────────────\n\
             traz exposes an MCP stdio server. Run:\n\n\
               traz mcp\n\n\
             Or use the REST API:\n\n\
               traz serve\n\n\
             Then point your tool at http://localhost:4000/events\n\n\
             Supported tools with dedicated setup:\n\
               traz setup claude    → Claude Code\n\
               traz setup cursor    → Cursor IDE\n\
               traz setup codex     → OpenAI Codex CLI\n\
               traz setup opencode  → OpenCode\n\
               traz setup vscode    → VS Code + GitHub Copilot\n\
               traz setup agy       → Antigravity (agy) / Gemini CLI\n\
               traz setup aider     → Aider\n\
               traz setup gemini    → Gemini CLI"
            .to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opencode_mcp_config() {
        let config = opencode_mcp_config();
        assert!(config.get("mcp").is_some());
        assert!(config["mcp"].get("traz").is_some());
        assert_eq!(config["mcp"]["traz"]["type"], "local");
        assert_eq!(
            config["mcp"]["traz"]["command"],
            serde_json::json!(["traz", "mcp"])
        );
        assert_eq!(config["mcp"]["traz"]["enabled"], true);
    }

    #[test]
    fn test_claude_mcp_config() {
        let config = claude_mcp_config();
        assert!(config.get("mcpServers").is_some());
        assert!(config["mcpServers"].get("traz").is_some());
        assert_eq!(config["mcpServers"]["traz"]["command"], "traz");
        assert_eq!(config["mcpServers"]["traz"]["args"], serde_json::json!(["mcp"]));
    }

    #[test]
    fn test_cursor_mcp_config() {
        let config = cursor_mcp_config();
        assert!(config.get("mcpServers").is_some());
        assert!(config["mcpServers"].get("traz").is_some());
        assert_eq!(config["mcpServers"]["traz"]["command"], "traz");
        assert_eq!(config["mcpServers"]["traz"]["args"], serde_json::json!(["mcp"]));
    }

    #[test]
    fn test_setup_instructions_all_platforms() {
        let platforms = vec![
            "opencode",
            "claude",
            "claude-code",
            "cursor",
            "vscode",
            "aider",
            "agy",
            "antigravity",
            "gemini",
            "gemini-cli",
            "unknown",
        ];

        for platform in platforms {
            let res = setup_instructions(platform);
            assert!(res.is_ok());
            let instructions = res.unwrap();
            assert!(!instructions.trim().is_empty());
            
            // Check key phrases based on target platform
            match platform {
                "opencode" => {
                    assert!(instructions.contains("OpenCode"));
                    assert!(instructions.contains("opencode.jsonc"));
                }
                "claude" | "claude-code" => {
                    assert!(instructions.contains("Claude Code"));
                    assert!(instructions.contains("claude_desktop_config.json"));
                }
                "cursor" => {
                    assert!(instructions.contains("Cursor"));
                    assert!(instructions.contains("Settings"));
                }
                "vscode" => {
                    assert!(instructions.contains("VS Code"));
                    assert!(instructions.contains("Copilot"));
                }
                "aider" => {
                    assert!(instructions.contains("Aider"));
                    assert!(instructions.contains("CONVENTIONS.md"));
                }
                "agy" | "antigravity" => {
                    assert!(instructions.contains("Antigravity"));
                    assert!(instructions.contains(".agents"));
                }
                "gemini" | "gemini-cli" => {
                    assert!(instructions.contains("Gemini"));
                    assert!(instructions.contains("settings.json"));
                }
                _ => {
                    assert!(instructions.contains("Generic MCP Integration"));
                }
            }
        }
    }

    #[test]
    fn test_active_sync_prompt() {
        let prompt = active_sync_prompt();
        assert!(prompt.contains("Traz — Developer Memory Layer"));
        assert!(prompt.contains("traz_recent"));
        assert!(prompt.contains("traz_add"));
    }
}
