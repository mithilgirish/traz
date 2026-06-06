# Traz Architecture

This document is for contributors and power users who want to understand the internals of `traz`.

## System Overview

`traz` is designed as a local-first, zero-cloud architecture.

```text
┌─────────────────────────────────────┐
│           Your AI Tools             │
└──────────────────┬──────────────────┘
                   │ MCP (stdio)
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

## Core Crates

- **`traz-core`**: Contains the primary data models (`Event`, `Config`) and type definitions.
- **`traz-db`**: The SQLite wrapper. Handles schema migrations, queries, and ONNX model execution via `fastembed-rs`.
- **`traz-embeddings`**: Manages the local ONNX embedding pipeline (all-MiniLM-L6-v2 via `fastembed-rs`), including model download, caching, and batched vector generation.
- **`traz-api`**: An Axum-based REST API for standard HTTP integrations.
- **`traz-mcp`**: The Model Context Protocol implementation.
- **`traz-tui`**: A `ratatui`-based interactive terminal UI.
- **`traz-cli`**: The `clap` binary that ties everything together.

## Storage Layer

The database lives locally at `~/.local/share/traz/traz.db` (XDG Base Directory compliant).

### The `events` Table
Stores the raw timeline. The `diff` column can store multi-megabyte git diffs, which is why the API/MCP layers have a `10 MB` payload limit.

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
- Reduces typical MCP response payloads by **60–75%**, which directly lowers AI tool costs and latency.

The MCP server always uses dense output; the human-facing CLI defaults to normal output unless `--dense` is passed.

---

## Async Encapsulation

Because SQLite I/O and ONNX CPU calculations are fundamentally blocking, all database interactions within `traz-api` and `traz-mcp` are wrapped in `tokio::task::spawn_blocking`. This prevents the heavy matrix math from stalling the Tokio worker threads that handle incoming network or stdio requests.
