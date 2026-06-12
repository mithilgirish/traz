# Architecture & Design

This document is for contributors and power users who want to understand the internals of `traz`, how the various components communicate, and how it delivers a seamless, local-first memory layer for your AI tools.

## System Overview

`traz` is designed as a local-first, zero-cloud architecture. It runs entirely on your machine, ensuring complete privacy and zero latency.

```text
┌─────────────────────────────────────┐
│           Your AI Tools             │
│    (Cursor, Claude Code, Gemini)    │
└──────────────────┬──────────────────┘
                   │ MCP (stdio) / API
                   ▼
         ┌─────────────────┐
         │ traz-mcp / api  │
         └────────┬────────┘
                  │ tokio::spawn_blocking
                  ▼
         ┌─────────────────┐
         │    traz-db      │
         │ (SQLite + ONNX) │
         └─────────────────┘
```

Meet **Cuby**, our beloved mascot!
Cuby represents the underlying context engine of `traz`—a highly optimized, tireless worker that watches, indexes, and retrieves your development history without you ever needing to leave your editor.

## Core Crates

- **`traz-core`**: Contains the primary data models (`Event`, `Config`) and type definitions shared across the workspace.
- **`traz-db`**: The SQLite wrapper. Handles schema migrations, asynchronous queries, and ONNX model execution via `fastembed-rs`.
- **`traz-embeddings`**: Manages the local ONNX embedding pipeline (all-MiniLM-L6-v2 via `fastembed-rs`), including model download, caching, and batched vector generation entirely on your CPU.
- **`traz-api`**: An Axum-based REST API for standard HTTP integrations.
- **`traz-mcp`**: The Model Context Protocol implementation, bridging `traz` to modern AI editors via standard I/O streams.
- **`traz-tui`**: A `ratatui`-based interactive terminal UI.
- **`traz-cli`**: The `clap` binary that acts as the entry point and ties everything together.

## Usage Modalities

`traz` is incredibly versatile and can be interacted with in three distinct ways:

### 1. The Command Line Interface (CLI)
For quick, scriptable actions and CI/CD pipelines, the CLI provides raw access to the database:
- **`traz add "commit message"`**: Inserts a new memory event manually.
- **`traz search "auth bug"`**: Executes a hybrid semantic search directly in your terminal.
- **`traz log`**: Dumps the chronological timeline.

The CLI is perfect for Git hooks. By adding `traz add` to your `post-commit` hook, Cuby will automatically index every commit you make in the background!

### 2. The Terminal User Interface (TUI)
For deep dives into your history, `traz` features a beautifully crafted, keyboard-driven TUI built with `ratatui`.
- Launch it via **`traz ui`**.
- It features an interactive timeline, Vim-style keybindings (j/k to navigate), and a detailed split-pane view showing exact diffs and context for each event.
- It is the best way for a human to visually explore what the AI agents have been tracking.

### 3. The Model Context Protocol (MCP) Server
This is the heart of `traz` for AI agents. By running `traz mcp start`, `traz` runs a persistent daemon communicating via standard I/O.
- AI Agents (like Cursor) query the MCP server autonomously to recall past refactors or architectural decisions without prompting you.

## Storage Layer

The database lives locally at `~/.local/share/traz/traz.db` (XDG Base Directory compliant).

### The `events` Table
Stores the raw timeline. The `diff` column can store multi-megabyte git diffs, which is why the API/MCP layers have payload limits.

### Semantic Search & Performance
The `event_embeddings` table stores `f32` vectors. 
`hybrid_search` is highly optimized:
1. It queries vectors independently.
2. Computes Cosine Similarity in-memory.
3. Performs a targeted `IN (...)` SQLite query to fetch only the final structs, completely preventing Out Of Memory (OOM) crashes on large timelines.

## Dense Output Format

The `--dense` flag (available on most read commands and enabled by default in the MCP server) switches from human-readable text to a compact, token-optimized representation. Key properties:

- Omits whitespace, labels, and redundant punctuation.
- Truncates long `diff` payloads to a configurable character limit.
- Reduces typical MCP response payloads by **60–75%**, which directly lowers AI tool costs and preserves precious context windows.

## Async Encapsulation

Because SQLite I/O and ONNX CPU calculations are fundamentally blocking, all database interactions within `traz-api` and `traz-mcp` are wrapped in `tokio::task::spawn_blocking`. This prevents heavy matrix math from stalling the Tokio worker threads that handle incoming network or stdio requests.
