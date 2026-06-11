# traz — Deep AI Tool Integration Guide

How to connect `traz` to every major AI coding harness for maximum context continuity
and minimum token usage.

---

## How Deep Integration Works

traz exposes an MCP stdio server. When configured, the AI agent:
1. **On session start** — Automatically calls `traz_recent` or `traz_recap` to retrieve the last checkpoint and resume instantly.
2. **During work** — Calls `traz_add` to log significant events (fixes, decisions, refactors).
3. **On context bloat** — Calls `traz_checkpoint` to snapshot state and prompt you to open a fresh chat.

The `dense` format cuts MCP response payloads by **60–75%**, meaning agents spend fewer tokens reading history and more tokens writing code.

traz also automatically injects a compact `instructions` field during the MCP `initialize` handshake — agents that support server instructions (Claude Code, Codex CLI, Antigravity) will automatically learn how to use traz correctly without any manual system prompt configuration.

---

## Claude Code

### Step 1: Install
```bash
traz setup claude
# traz detects the Claude CLI and offers to auto-run:
# claude mcp add traz -- traz mcp
```

### Step 2 (optional): Add to your CLAUDE.md
```
## Context Layer (traz)
At the start of every session:
1. Call traz_recent with format="dense" and limit=5.
2. If you see a "checkpoint" event, read its summary — it describes where we left off.
3. Call traz_add to log every significant decision or bug fix.
4. If context is filling up, call traz_checkpoint, then ask the user to start a new chat.
```

### Token Budget
- `traz_recent`: `{"limit": 5, "format": "dense"}` → ~65 tokens vs ~200 markdown (**68% savings**)
- `traz_context`: `{"limit": 10, "format": "dense", "max_tokens": 500}`

---

## Antigravity (agy)

### Step 1: Install
```bash
traz setup agy
# traz detects the agy CLI and offers to auto-run:
# agy mcp add traz -- traz mcp
```

### Step 2: No extra config needed
traz's MCP server sends an `instructions` field during initialization.
Antigravity reads this automatically — no AGENTS.md changes required.

### Optional: Add to `.antigravitycli/AGENTS.md`
```
## Memory Layer (traz)
- Call traz_recent(format="dense") at session start.
- Log key decisions with traz_add.
- Call traz_checkpoint when context is bloated.
```

---

## OpenAI Codex CLI

### Step 1: Install
```bash
traz setup codex
# traz detects the codex CLI and offers to auto-run:
# codex mcp add traz -- traz mcp
```

Or add manually to `~/.codex/config.toml`:
```toml
[[mcp_servers]]
name = "traz"
command = "traz"
args = ["mcp"]
```

### Step 2: No extra config needed
Codex CLI reads the server `instructions` field from MCP initialization automatically.
traz sends compact instructions during `initialize` — no extra config needed.

### Optional: Add to `~/.codex/instructions.md`
```markdown
## Engineering Memory (traz)
At the start of every session, call traz_recent with format="dense" to load context.
Log decisions with traz_add. Create checkpoints with traz_checkpoint when context fills up.
```

---

## Cursor

### Step 1: Install
```bash
traz setup cursor
# traz offers to write ~/.cursor/mcp.json automatically
```

Or add manually to `~/.cursor/mcp.json`:
```json
{
  "mcpServers": {
    "traz": {
      "command": "traz",
      "args": ["mcp"]
    }
  }
}
```

### Step 2 (recommended): Add to `.cursorrules`
```
## Engineering Memory (traz)
- Session start: call traz_recent(format="dense", limit=5)
- During work: call traz_add for bug fixes, refactors, decisions
- Context bloat: call traz_checkpoint, then ask user to open new chat
- Daily standup: call traz_recap(hours=24, format="dense")
```

### Cursor-Specific Tips
- Cursor's Composer re-reads context on every turn. Using `dense` format saves ~3,000 tokens per turn on a 10-event history.
- Use `traz_context` with `query="current task"` to pull only relevant history into Composer.

---

## Gemini CLI

### Step 1: Add to `~/.gemini/settings.json`
```json
{
  "mcpServers": {
    "traz": {
      "command": "traz",
      "args": ["mcp"]
    }
  }
}
```

---

## OpenCode

### Step 1: Install
```bash
traz setup opencode
# traz offers to write ~/.config/opencode/opencode.jsonc automatically
```

Or add manually to your global configuration file (`~/.config/opencode/opencode.jsonc`) or local project-scoped config file (`opencode.jsonc`):
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

### Step 2: Add to `AGENTS.md` (Project-specific instructions)
```markdown
## Engineering Memory (traz)
- Session start: call traz_recent(format="dense", limit=5)
- During work: call traz_add for bug fixes, refactors, decisions
- Context bloat: call traz_checkpoint, then ask user to open new chat
- Daily standup: call traz_recap(hours=24, format="dense")
```

---

## Token Efficiency Cheat Sheet

| Operation | Default (markdown) | Dense format | Savings |
|---|---|---|---|
| `traz_recent` (5 events) | ~200 tokens | ~65 tokens | **68%** |
| `traz_recent` (10 events) | ~380 tokens | ~122 tokens | **68%** |
| `traz_context` (10 events) | ~500 tokens | ~160 tokens | **68%** |
| `traz_recap` (24h, 20 events) | ~800 tokens | ~250 tokens | **69%** |
| Per-session savings (loop agent, 10 turns) | ~4,000 tokens | ~1,200 tokens | **70%** |

Every LLM call costs tokens, and context window limits restrict how much history you can feed into the agent. traz is designed to be highly token-efficient.

Use this cheat sheet to configure your system prompts and agent rules:

1. **Always use `dense` format for background fetches**:
   - Standard fetch: `traz_recent()` -> ~4,000 tokens
   - Token-optimized: `traz_recent(format="dense")` -> ~950 tokens
   - Saves **75%** of context window capacity.

2. **Differentiate Session Start vs. Interactive Turns**:
   - **Session Start**: Fetch the last 5-10 events in `dense` format to catch the agent up on where you left off.
   - **Interactive Turn**: Do NOT pull history on every turn. Let the agent work. If the agent needs historical context for the current task, it should call `traz_context(query="current task description")` to perform a semantic vector search and pull only the most relevant 2-3 events.

3. **Checkpoints**:
   - Once a task is complete, run `traz_checkpoint` to freeze the memory state.
   - You can then safely clear your agent's chat history/start a new thread to clean up context window usage. The new agent session will start fresh, query `traz_recent`, see the checkpoint, and immediately know the state of the repository.

---

## The `traz_checkpoint` Workflow

When your AI session is running out of context:

```
AI: "Our context is getting long. Let me save a checkpoint..."
→ calls traz_checkpoint with a dense summary of everything done
→ "Checkpoint #42 saved. Please start a new chat to clear context."

[New chat]
AI: → calls traz_recent(format="dense", limit=1)
→ reads checkpoint #42
→ "I see we were fixing the JWT refresh race condition. Resuming..."
```

This is **zero-cost context reset** — no copy-pasting, no re-explaining.

---

## Available MCP Tools

| Tool | Description | Token-Efficient? |
|---|---|---|
| `traz_recent` | Get recent events. Use `format="dense"` | ✅ |
| `traz_recap` | Time-bounded summary (last N hours) | ✅ |
| `traz_context` | RAG-powered context for current task | ✅ |
| `traz_search` | Keyword + semantic search | ✅ |
| `traz_add` | Log a new event | N/A |
| `traz_checkpoint` | Save session state before context reset | N/A |
| `traz_show` | Full details of a specific event | On-demand |
| `traz_diff` | Show code diff for a specific event | On-demand |
| `traz_stats` | Database statistics | ✅ |

---

## Supported Tools Summary

| Tool | Auto-Configure | Server Instructions | System Prompt |
|---|---|---|---|
| Claude Code | ✅ `traz setup claude` | ✅ Auto-injected | Optional (CLAUDE.md) |
| Antigravity (agy) | ✅ `traz setup agy` | ✅ Auto-injected | Optional |
| OpenAI Codex CLI | ✅ `traz setup codex` | ✅ Auto-injected | Optional (AGENTS.md) |
| OpenCode | ✅ `traz setup opencode` | ❌ Not supported | Optional (AGENTS.md) |
| Cursor | ✅ `traz setup cursor` | ❌ Not supported | Recommended (.cursorrules) |
| Gemini CLI | Manual | ❌ Not supported | Optional |
| Aider | Manual | ❌ | Optional |
| Warp | Manual | ❌ | Optional |
