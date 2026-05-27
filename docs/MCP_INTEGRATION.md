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
- **`traz_context`**: Generate a structured, token-optimized markdown summary of the current project state. Includes dense RAG formatting to minimize context consumption.
- **`traz_stats`**: View analytics on which tools are contributing the most events.
- **`traz_checkpoint`**: Snapshot the current conversational state. Used by AI agents to securely save their progress when the context window becomes too bloated, enabling a safe "fresh chat" reset.

## Dealing With Context Bloat

When AI chats go on for too long, context windows fill up, causing AI performance to drop (the "Lost in the Middle" problem). `traz` solves this natively through two mechanisms:
1. **Dense RAG Formatting**: `traz_context` automatically strips tags, caps file lists, and truncates long summaries to provide maximum insight using minimum tokens.
2. **Session Checkpointing**: When an agent detects context bloat, it can call `traz_checkpoint` to summarize its current state, then instruct the user to start a new chat. The new chat reads the checkpoint and resumes instantly.

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
