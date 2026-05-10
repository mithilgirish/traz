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
- [ ] Automatic git integration
- [ ] Tool adapters

**Future**
- [ ] Semantic search with local embeddings
- [ ] Context compression for long-running projects
- [ ] Workflow snapshots
- [ ] AI trace visualization
- [ ] Local vector indexing
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
