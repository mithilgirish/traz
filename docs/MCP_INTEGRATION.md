# MCP Integration Guide

`traz` utilizes the Model Context Protocol (MCP) to seamlessly integrate with your favorite AI coding assistants.

## Starting the Server

To expose your local `traz` timeline to AI agents, start the MCP stdio listener or TCP server:
```bash
traz mcp serve
```
*(By default, this listens on stdio for direct child-process execution. You can wrap it in `socat` for TCP if required).*

## Available MCP Tools

Once connected, your AI agent has access to the following capabilities:

- **`traz_recent`**: Fetch the most recent engineering events to establish context at the start of a session.
- **`traz_search`**: Search the timeline for specific keywords or concepts (e.g., "how does auth work?").
- **`traz_add`**: Push a new event to the timeline. AI tools use this to document their own refactors or bug fixes.
- **`traz_context`**: Generate a structured markdown summary of the current project state.
- **`traz_stats`**: View analytics on which tools are contributing the most events.

## Client Setup

### Claude Code
Claude Code natively supports MCP via the `mcp add` command:
```bash
claude mcp add traz -- traz mcp serve
```

### Cursor
In Cursor Settings > Features > MCP Servers:
1. Click **Add New MCP Server**.
2. Name: `traz`
3. Type: `command`
4. Command: `traz mcp serve`

### Aider
Start Aider with the MCP server attached:
```bash
aider --mcp "traz mcp serve"
```
