# Traz User Guide

Welcome to the `traz` user guide! This manual covers everything you need to know to use `traz` effectively on the command line, from basic usage to advanced configurations.

---

## Core Concepts

`traz` acts as a local timeline for your development workflow. It stores **Events**, which represent individual actions, bug fixes, or thoughts.

Every event contains:
- **`tool`**: Which tool created it (e.g., `claude-code`, `cursor`, `aider`, `traz-cli`)
- **`type`**: The category of the event (e.g., `commit`, `bug_fix`, `refactor`, `decision`)
- **`title`**: A short summary of what happened.
- **`summary`**: (Optional) Extended context or reasoning.
- **`files`**: (Optional) A list of files that were modified.
- **`diff`**: (Optional) The raw git diff or patch string of the change.
- **`timestamp`**: Exactly when the event occurred.

---

## Configuration & Timelines

By default, `traz` stores your global engineering timeline at `~/.local/share/traz/traz.db`.

### Project-Local Timelines
You can have isolated `traz` timelines for specific projects! If `traz` detects a `.traz/traz.db` file in your current working directory (or any parent directory), it will automatically use that localized database instead of your global one.

### Environment Variables
You can override default behaviors using environment variables:
- `TRAZ_DB=/path/to/custom/traz.db`: Explicitly sets the database path.
- `TRAZ_PORT=7474`: Overrides the default port (`4000`) used by the MCP and API server.

### The `config.toml` File
When `traz` creates a database, it also creates a `config.toml` next to it (e.g., `~/.local/share/traz/config.toml`). 
```toml
api_port = 4000
embeddings_enabled = true
```

---

## The Command Line Interface (CLI)

### Viewing Recent Activity
To see what happened recently across all your tools:
```bash
traz recent
```
*Optional Flags:*
- `--tool <TOOL_NAME>`: Filter events by a specific tool (e.g., `traz recent --tool cursor`).
- `--type <EVENT_TYPE>`: Filter by event type (e.g., `traz recent --type bug_fix`).
- `--limit <NUMBER>`: Show more or fewer events (default is 10).

### Searching Your History
To find that specific JWT bug you fixed three months ago:
```bash
traz search "JWT token refresh"
```
You can combine search with filters:
```bash
traz search "auth middleware" --tool claude-code --tag security
```

### Semantic Search (Vector Embeddings)
`traz` uses local ONNX embedding models (`all-MiniLM-L6-v2`) to perform semantic searches. If you search for "database connection limits", it will find events labeled "pg_bouncer configured" even if the keywords don't match!

**Note:** If you import an old database, you may need to generate vectors for past events:
```bash
traz backfill-embeddings
```

### Logging Context Manually
While AI tools log events automatically via the MCP integration, you can also leave breadcrumbs for your future self (or your AI):
```bash
traz log "I chose to use Redis for the caching layer instead of Memcached because we need data persistence."
```

### Context Summaries
To quickly generate a markdown summary of the project state to paste into ChatGPT or Claude:
```bash
traz context
```

---

## The Interactive TUI

For a rich, visual timeline experience, use the Terminal UI:
```bash
traz tui
```

### Navigation
- **`k` / `Up Arrow`**: Move up the timeline.
- **`j` / `Down Arrow`**: Move down the timeline.
- **`q` / `Esc`**: Quit the TUI.

### Actions
- **`Enter`**: Open the Detail View to see the full reasoning, summary, and affected files for an event.
- **`d`**: View the full source code `diff` (if the AI tool attached one).
- **`s`**: Enter Semantic Search mode. Type a natural language query and hit enter to instantly filter the timeline.
- **`r` / `u`**: (Experimental) Undo or Rewind workflows if the integrations support it.

---

## Meet Cuby! (The Context Pet)

`traz` isn't just a database—it comes with a built-in virtual context pet named **Cuby**! Because context management shouldn't be boring, you can interact with Cuby directly from the terminal. Cuby will change moods based on your interactions and can even recall your development history.

To interact with Cuby, use the `traz cuby` subcommand:

### Checking Status
```bash
traz cuby status
```
*Checks Cuby's current mood and memory status. If you haven't fed Cuby context recently, he might be asleep or dizzy!*

### Asking for Help
```bash
traz cuby ask "memory leak"
```
*Cuby will search his internal semantic memory bank for the query you provide and respond with the context.*

### Feeding Context
```bash
traz cuby feed "Fixed login database bug"
```
*Manually feed Cuby a memory. He gets very happy when you feed him context, and it is permanently stored in the `traz` database!*

### Fun Commands
Take a break from coding and play with your pet:
- **`traz cuby pet`**: Give Cuby a head pat.
- **`traz cuby sing`**: Let Cuby sing you a developer song.
- **`traz cuby dance`**: Watch Cuby perform a cute terminal dance!
- **`traz cuby play`**: Open the interactive Tamagotchi pet game.
