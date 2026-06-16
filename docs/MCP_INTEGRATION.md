# MCP Integration Guide

`traz` utilizes the Model Context Protocol (MCP) to seamlessly integrate with your favorite AI coding assistants.

## Starting the Server

To expose your local `traz` timeline to AI agents, start the MCP stdio listener or TCP server:
```bash
traz mcp
```
*(By default, this listens on stdio for direct child-process execution. You can wrap it in `socat` for TCP if required).*

## Available MCP Tools

Once connected, your AI agent has access to the following capabilities:

- **`traz_recent`**: Fetch the most recent engineering events to establish context at the start of a session.
- **`traz_search`**: Search the timeline for specific keywords or concepts (e.g., "how does auth work?").
- **`traz_add`**: Push a new event to the timeline. AI tools use this to document their own refactors or bug fixes.
- **`traz_context`**: Generate a structured, token-optimized markdown summary of the current project state. Includes dense RAG formatting to minimize context consumption.
- **`traz_stats`**: View analytics on which tools are contributing the most events.
- **`traz_checkpoint`**: Snapshot the current conversational state. Used by AI agents to securely save their progress when the context window becomes too bloated, enabling a safe "fresh chat" reset.
- **`traz_show`**: Retrieve the full detail of a specific event by its ID, including the complete diff and metadata.
- **`traz_diff`**: Compare two events or time ranges to understand what changed between two points in your engineering timeline.

## Dealing With Context Bloat

When AI chats go on for too long, context windows fill up, causing AI performance to drop (the "Lost in the Middle" problem). `traz` solves this natively through two mechanisms:

1. **Token Optimization (`dense` format)**: By passing `{"format": "dense"}` to MCP tools like `traz_recent`, agents receive a highly compressed string (e.g., `[7m]|ai-agent|cp|Session Checkpoint|...`). 
   - **Massive Savings**: This strips UUIDs, JSON punctuation, and whitespace, reducing the token payload by **60% to 75%**. 
   - **Compounding Benefits**: For loop-based agents (Claude Code, Antigravity, Cursor) that re-read context on every turn, saving 3,000 tokens on a history fetch means saving those tokens *on every single turn*. This preserves the context window for actual code and slashes API costs.
2. **Session Checkpointing**: When an agent detects context bloat, it can call `traz_checkpoint` to summarize its current state, then instruct the user to start a new chat. The new chat reads the checkpoint and resumes instantly.

## Client Setup

### Antigravity (agy)
You can add `traz` as an MCP server to Antigravity using the CLI:
```bash
agy mcp add traz -- traz mcp
```
*(Alternatively, you can configure it via the interactive workspace setup menu).*

### Claude Code
Claude Code natively supports MCP via the `mcp add` command:
```bash
claude mcp add traz -- traz mcp
```

### Cursor
In Cursor Settings > Features > MCP Servers:
1. Click **Add New MCP Server**.
2. Name: `traz`
3. Type: `command`
4. Command: `traz mcp`

### OpenCode
Add this to your global configuration file (`~/.config/opencode/opencode.jsonc`) or local project config file (`opencode.jsonc`):
```json
{
  "mcp": {
    "traz": {
      "type": "local",
      "command": ["traz", "mcp"],
      "enabled": true
    }
  }
}
```

### Aider
Start Aider with the MCP server attached:
```bash
aider --mcp "traz mcp"
```
