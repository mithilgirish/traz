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

        "git" => Ok("Git Post-Commit Hook\n\
             ────────────────────\n\
             Run: traz init --hook\n\n\
             This installs a post-commit hook that automatically\n\
             captures every commit as a traz event."
            .to_string()),

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
