# Traz Documentation

Welcome to the official documentation for **traz**, the local-first developer memory layer that gives AI coding tools a shared brain.

## Table of Contents

- [Quickstart](./QUICKSTART.md) - Get from zero to a working AI memory layer in 60 seconds.
- [User Guide](./USER_GUIDE.md) - Learn how to install and use the CLI, interactive TUI, semantic search, and timeline visualization.
- [MCP Integration](./MCP_INTEGRATION.md) - Learn how to connect your AI tools (Claude Code, Cursor, Aider, etc.) to the `traz` local server so they can read and write to your engineering context.
- [Architecture](./ARCHITECTURE.md) - A deep dive into how `traz` works under the hood, including its SQLite schema, embeddings engine, and async Tokio runtime.
- [Changelog](../CHANGELOG.md) - A history of all notable changes to `traz`.

## What is traz?

AI coding workflows are fragmented. When you switch from Claude Code to Cursor to Warp, the context is lost. `traz` fixes this by providing a single, local SQLite timeline that every MCP-compatible tool can read from and write to. 

It acts as an automated engineering diary, capturing decisions, refactors, and traces so your AI agents are always up to speed.
