<p align="center">
  <img width="1774" alt="traz" src="https://github.com/user-attachments/assets/f4b969a0-b23e-400b-a012-38f05e20973b" />
</p>

<p align="center">
  <strong>trace. context. continuity.</strong><br/>
  <sub>A local-first developer memory layer that gives AI coding tools a shared brain.</sub>
</p>

<p align="center">
  <a href="https://github.com/mithilgirish/traz/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License" /></a>
  <img src="https://img.shields.io/badge/MCP-compatible-green" alt="MCP compatible" />
</p>

---

```bash
$ traz recent

✓ fixed websocket reconnect issue        [claude-code · 2h ago]
✓ updated auth middleware                [cursor · 5h ago]
✓ traced memory leak in queue worker     [warp · 1d ago]
✓ reverted broken cache optimization     [aider · 2d ago]
```

---

## The Problem

AI coding workflows are fragmented by design.

You debug a gnarly issue in Claude Code, switch to Cursor to refactor, open Warp to run some tests — and at every step, you're starting from scratch. The context is gone. The reasoning is lost. The debugging history never existed.

Every AI session is an island.

**`traz` builds the bridge.**

---

## What It Does

`traz` is a local-first engineering memory layer. It captures debugging history, architectural decisions, and workflow traces as you code — and makes that context available to every AI tool in your stack.

No cloud. No vendor lock-in. No sync accounts. Just a lightweight SQLite store living on your machine, accessible to any MCP-compatible tool.

---

## Features

- **Local-first** — everything stays on your machine, always
- **Shared engineering memory** — one context layer across all your AI tools
- **Searchable debugging history** — find that fix you made three days ago in two seconds
- **Timeline-based workflow tracking** — see how your thinking evolved across sessions
- **MCP-compatible** — plug into any MCP-supporting tool without extra config
- **AI Context Checkpointing** — native escape hatches for long-running AI sessions to prevent context window bloat
- **Token-Optimized RAG** — dense formatting for AI context retrieval to save tokens and improve inference
- **SQLite-powered** — zero-dependency, zero-overhead storage
- **CLI-first** — fast, scriptable, composable
- **Zero cloud dependency** — your context never leaves your machine

---

## Installation

```bash
cargo install traz
```

<details>
<summary>Build from source</summary>

```bash
git clone https://github.com/mithilgirish/traz
cd traz
cargo build --release
./target/release/traz --version
```

</details>

---

## Documentation

For a comprehensive guide on using `traz` and integrating it with your AI tools, please see the [Documentation](./docs/index.md):

- 📖 [**User Guide**](./docs/USER_GUIDE.md) - CLI commands, semantic search, interactive TUI, and advanced filtering.
- 🔌 [**MCP Integration**](./docs/MCP_INTEGRATION.md) - How to connect Claude Code, Cursor, Aider, and Warp to the `traz` context server.
- 🏗️ [**Architecture**](./docs/ARCHITECTURE.md) - A deep dive into the SQLite vector storage, RRF search logic, and zero-cloud design.

---

## Usage

### View recent activity across all tools

```bash
$ traz recent

✓ fixed websocket reconnect issue        [claude-code · 2h ago]
✓ updated auth middleware                [cursor · 5h ago]
✓ traced memory leak in queue worker     [warp · 1d ago]
✓ reverted broken cache optimization     [aider · 2d ago]
```

### Search your engineering history

```bash
$ traz search auth

[2d ago] claude-code
Fixed JWT refresh race condition

[5h ago] cursor
Updated auth middleware retry logic
```

### Semantic Search

`traz` automatically generates local embeddings for your events using ONNX and `fastembed-rs`. 
This allows you to find contextually relevant history even if you don't use the exact keywords.

```bash
$ traz search "database connection pooling"

[semantic search] Search: "database connection pooling" (2 results)
─────────────────────────────────────────────
 1. Added pg_bouncer for connections (72%)
    Tool: cursor       Type: commit     Age: 3d ago     Tags: #db

 2. Re-architected connection lifecycle (64%)
    Tool: gemini       Type: refactor   Age: 1w ago     Tags: #db #performance
```

### Backfill Missing Embeddings

If you imported old events or just added the embeddings feature, you can generate missing vectors in bulk:

```bash
$ traz backfill-embeddings
```

### View a workflow timeline

```bash
$ traz timeline

• created websocket handler
• debugged reconnect issue
• added retry backoff
• verified with local tests
```

### Log context manually

```bash
$ traz log "traced root cause of memory leak to unbounded queue growth"
```

### Filter by tool

```bash
$ traz recent --tool cursor
$ traz recent --tool claude-code
```

---

## How It Works

See [ARCHITECTURE.md](./docs/ARCHITECTURE.md) for a deep dive into the system design.

```
┌─────────────────────────────────────┐
│           Your AI Tools             │
│  Claude Code · Cursor · Gemini CLI  │
│  Warp · Aider · Ollama · Agents     │
└──────────────────┬──────────────────┘
                   │  MCP / CLI
                   ▼
         ┌─────────────────┐
         │      traz       │
         │  context layer  │
         └────────┬────────┘
                  │
                  ▼
     ┌────────────────────────┐
     │   Local Timeline Engine │
     │   SQLite / Context Store│
     └────────────────────────┘
```

Each AI tool writes context to `traz` as you work. When you switch tools, the new session inherits that context — understanding what was already tried, what was fixed, and why decisions were made.

---

## MCP Integration

`traz` runs a local MCP server that any compatible tool can connect to:

```bash
$ traz mcp serve
# Listening on localhost:7474
```

Point your AI tool at `localhost:7474` and it gains access to your full engineering timeline automatically.

---

## Supported Tools

| Tool | Status |
|---|---|
| Claude Code | ✅ Supported |
| Cursor | ✅ Supported |
| Gemini CLI | ✅ Supported |
| Warp | ✅ Supported |
| Aider | ✅ Supported |
| Ollama | ✅ Supported |
| Local MCP agents | ✅ Supported |

---

## Roadmap

**v0.1**
- [x] Local timeline storage
- [x] CLI commands — `recent`, `search`, `timeline`, `log`
- [x] Search and history system
- [x] MCP server
- [x] Automatic git integration
- [x] Tool adapters
- [x] Semantic search with local embeddings
- [x] Local vector indexing
- [x] AI trace visualization (TUI)
- [x] Context compression for long-running projects
- [x] Workflow snapshots (Checkpoints)

**Future**
- [ ] VSCode extension

---

## Philosophy

Engineering context should persist across AI tools — not uploaded to a vendor, not locked into one product, not lost when you close a tab.

`traz` treats your debugging history, architectural decisions, and workflow reasoning as first-class artifacts: stored locally, searchable instantly, available to every tool in your stack.

It's the missing memory layer for AI-native development.

---

## Contributing

Contributions are welcome. Please open an issue before submitting a large PR so we can align on direction first.

```bash
git clone https://github.com/mithilgirish/traz
cd traz
cargo test
```

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

---

## License

MIT — see [LICENSE](./LICENSE) for details.

---

<p align="center">
  <sub>Built for developers who switch tools, not context.</sub>
</p>
