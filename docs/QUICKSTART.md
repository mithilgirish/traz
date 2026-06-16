# traz — 60-Second Quickstart

This guide gets you from zero to a working AI memory layer in under a minute.

---

## Step 1: Install

Choose your preferred installation method:

::: code-group

```bash [NPM]
# Install globally via NPM
npm install -g @traz-dev/traz
```

```bash [Homebrew]
# Tap the repository and install
brew tap mithilgirish/traz
brew install traz
```

```bash [Shell (macOS & Linux)]
# Install via standalone shell script
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/mithilgirish/traz/releases/latest/download/traz-installer.sh | sh
```

```powershell [PowerShell (Windows)]
# Install via standalone PowerShell script
irm https://github.com/mithilgirish/traz/releases/latest/download/traz-installer.ps1 | iex
```

```bash [Cargo]
# Install from source via Cargo (requires Rust toolchain)
cargo install --git https://github.com/mithilgirish/traz.git traz
```

:::

---

## Step 2: Initialize your project

Navigate to your project folder and run:

```bash
cd your-project
traz init
```

This creates a `.traz/` directory (already git-ignored) that stores your local timeline.

---

## Step 3: Connect your AI tool

Run the setup wizard for your AI coding tool:

```bash
# For Claude Code
traz setup claude

# For Cursor
traz setup cursor

# For OpenCode
traz setup opencode

# For Gemini CLI
traz setup gemini

# For Antigravity (agy)
traz setup agy

# For OpenAI Codex CLI
traz setup codex
```

This prints the exact config snippet and commands to connect `traz` as an MCP server.

---

## Step 4: You're Ready

Now every time your AI tool starts a new session, it will automatically:
1. Call `traz_recent` to retrieve your latest events.
2. Resume right where you left off — no copy-pasting context.

### Verify it works
```bash
# See your recent activity
traz recent

# Log a manual note
traz log "Started migrating auth to JWT tokens"

# See what happened in the last 24 hours
traz recap
```

---

## Troubleshooting

Run the health check at any time:
```bash
traz doctor
```

This checks your SQLite setup, embedding model, and configuration — and tells you exactly what to fix.

---

## Going Deeper

- [User Guide](./USER_GUIDE.md) — full command reference
- [MCP Integration](./MCP_INTEGRATION.md) — connecting every AI tool
- [Architecture](./ARCHITECTURE.md) — how it works under the hood
