# traz Architecture

## Workspace Structure

```
traz/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── traz-core/          # Event model, config, errors (shared types)
│   ├── traz-db/            # SQLite storage layer (rusqlite + WAL)
│   ├── traz-cli/           # CLI binary (clap) — the main `traz` command
│   ├── traz-api/           # REST API server (axum)
│   ├── traz-mcp/           # MCP stdio server (JSON-RPC)
│   └── traz-integrations/  # Git, Claude, Cursor adapters
├── docs/                   # Documentation
├── assets/                 # Logo, screenshots, demo GIFs
├── README.md
├── LICENSE
└── .gitignore
```

## Dependency Graph

```
traz-cli (binary)
├── traz-core
├── traz-db ─── traz-core
├── traz-api ─── traz-core + traz-db
├── traz-mcp ─── traz-core + traz-db
└── traz-integrations ─── traz-core + traz-db
```

## Data Flow

```
[AI Tools] ──→ MCP (stdio) ──→ traz-db ──→ SQLite
               REST API ────→ traz-db ──→ SQLite
               CLI ─────────→ traz-db ──→ SQLite
               Git hooks ──→ traz-db ──→ SQLite
```

## Storage

- **Database:** `~/.local/share/traz/traz.db` (overridable via `$TRAZ_DB`)
- **Format:** SQLite with WAL mode
- **Schema:** Single `events` table with UUID, metadata JSON, tags, session grouping

## Security

- Binds only to `127.0.0.1` (never `0.0.0.0`)
- CORS restricted to localhost origins
- All SQL queries use parameterized bindings
- LIKE wildcards escaped to prevent injection
- 64KB request body limit
- Input validation on all fields
- No telemetry, no external API calls
