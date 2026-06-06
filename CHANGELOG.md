# Changelog

All notable changes to `traz` will be documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

### Added
- `traz recap` command: Summarize recent events from a given timeframe for morning standups and AI context syncing.
- Interactive guided onboarding: First-time users get a friendly setup wizard with `dialoguer` menus.
- Antigravity (agy) MCP integration: `traz setup agy` and documented in `MCP_INTEGRATION.md`.

### Changed
- `traz init` now automatically creates a `.traz/` local directory and adds it to `.gitignore` without needing the `--local` flag.
- Interactive REPL mode: Returning users see a clean prompt instead of a large ASCII banner.
- MCP server command simplified from `traz mcp serve` to `traz mcp` across all docs.

### Fixed
- CI: Fixed `cargo fmt` formatting failures.
- CI: Fixed flaky semantic search snapshot tests in CI environments where embedding generation fails.
- Documentation: Corrected `traz mcp serve` → `traz mcp` references across `README.md` and `MCP_INTEGRATION.md`.

---

## [0.1.0] — 2026-06-06

### Added
- Local-first engineering memory layer backed by SQLite.
- `traz recent`, `traz search`, `traz timeline`, `traz context` — core read commands.
- `traz add`, `traz log`, `traz capture` — event ingestion commands.
- `traz mcp` — MCP stdio server for Claude Code, Cursor, Gemini CLI, Aider, Warp, and Antigravity.
- `traz init` — project setup with optional git hook integration.
- `traz tui` — ratatui-based terminal UI dashboard.
- `traz doctor` — installation diagnostic tool.
- `traz status` — system health overview.
- `traz setup <tool>` — step-by-step integration guides for popular AI tools.
- Semantic search with local ONNX embeddings (all-MiniLM-L6-v2 via fastembed-rs).
- Dense AI-optimized output format (`--dense` flag) reducing MCP token payloads by 60–75%.
- Session checkpointing via `traz checkpoint` for safe context-window resets.
- Git hook integration for automatic event capture on commit/push/checkout.
- Shell failure tracking hooks for zsh and bash.
- Cross-platform release binaries: Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64).
