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
- **`traz-api`**: An Axum-based REST API for standard HTTP integrations.
- **`traz-mcp`**: The Model Context Protocol implementation.
- **`traz-tui`**: A `ratatui`-based interactive terminal UI.
- **`traz-cli`**: The `clap` binary that ties everything together.

## Storage Layer

The database lives locally at `~/.traz/traz.db`. 

### The `events` Table
Stores the raw timeline. The `diff` column can store multi-megabyte git diffs, which is why the API/MCP layers have a `10 MB` payload limit.

### Semantic Search & Performance
The `event_embeddings` table stores `f32` vectors. 
`hybrid_search` is highly optimized:
1. It queries vectors independently.
2. Computes Cosine Similarity in-memory.
3. Performs a targeted `IN (...)` SQLite query to fetch only the final structs, completely preventing Out Of Memory (OOM) crashes on large timelines.

## Async Encapsulation

Because SQLite I/O and ONNX CPU calculations are fundamentally blocking, all database interactions within `traz-api` and `traz-mcp` are wrapped in `tokio::task::spawn_blocking`. This prevents the heavy matrix math from stalling the Tokio worker threads that handle incoming network or stdio requests.
