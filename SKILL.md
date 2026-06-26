---
name: traz
description: Use the traz developer memory MCP server to retrieve project history, past bug fixes, architectural decisions, and session context, and to record new engineering events as work happens. Trigger this skill at the start of any coding session, whenever the user asks about prior fixes, "why was this built this way," past decisions, or recent project activity, and immediately after completing any significant feature, bug fix, refactor, or architectural decision so it gets captured in the timeline. Applies identically across Claude Code, Cursor, Codex, OpenCode, Aider, GitHub Copilot, and Gemini CLI / Antigravity.
---

# Traz — Persistent Developer Memory Layer

`traz` is a local-first, AI-native memory system for a codebase. It stores engineering history — bug fixes, architectural decisions, workflow traces, commit summaries — as a queryable timeline, and exposes that timeline to any AI coding agent through an MCP (Model Context Protocol) server.

If a project has traz initialized, you have access to a living record of everything significant that has happened in it. Treat that record as part of the codebase, not as optional extra context. An agent that skips it will re-litigate decisions that were already made, re-introduce bugs that were already fixed, and ask the user questions whose answers are already in the timeline.

---

## Core directives

These apply to every agent that connects to traz, regardless of which tool is hosting the session.

1. **Orient before acting.** At the start of a session, or whenever the user asks about project state or history, call `traz_recent` or `traz_recap` before assuming you understand the current state. Don't guess at context that's one tool call away.
2. **Search before fixing.** If the user reports a bug, a regression, or something that "used to work," call `traz_search` with relevant keywords before touching code. The fix or the reason it's hard may already be documented.
3. **Understand before restructuring.** Before modifying core logic or doing anything cross-cutting, call `traz_context` to get a synthesized view of the architecture and the decisions behind it. Acting on a partial mental model is how regressions happen.
4. **Checkpoint before risk.** Before a large refactor, a migration, or any change that's hard to undo, call `traz_checkpoint` to mark a restore point in the timeline.
5. **Record after substantive work.** After completing a feature, bug fix, refactor, or notable decision, call `traz_add` exactly once. Write 1–3 sentences. State what changed and why — not just what was done. Skip this for trivial or exploratory work; see "What not to log" below.

---

## Quick decision guide

Use this to figure out which tool applies to the moment you're in:

| Situation | Tool to call |
|---|---|
| Just started this session, unsure of current state | `traz_recent` or `traz_recap` |
| User reports a bug, especially a recurring or "didn't this happen before" one | `traz_search` |
| About to work in an unfamiliar or core part of the codebase | `traz_context` |
| About to do something risky or hard to reverse | `traz_checkpoint` |
| Just finished meaningful work | `traz_add` |
| Found a relevant past event and need full detail | `traz_show` |
| Need to see the actual code change behind a past event | `traz_diff` |
| Memory layer behaving oddly, or want a sanity check | `traz_stats` |

All tools except `traz_add` and `traz_checkpoint` are read-only — call them freely; there's no cost to checking first.

---

## MCP tools reference

| Tool | Purpose | When to use |
|---|---|---|
| `traz_recent` | Returns the N most recent timeline events. | Session start; catching up after time away. |
| `traz_recap` | Summarizes today's or the current session's events. | End-of-session handoff; quick re-orientation. |
| `traz_search` | Hybrid keyword + semantic search over all events. | Before fixing a bug; looking for prior art on a problem. |
| `traz_context` | Synthesized high-level summary of architecture and key decisions. | Before touching core logic or unfamiliar areas. |
| `traz_checkpoint` | Marks a stable restore point/checkpoint. Requires `summary` summarizing current achievements and next steps so you can restart chat cleanly. | Before a major refactor or risky operation. |
| `traz_add` | Records a new event. You MUST call this after completing significant work. Requires `tool` (your AI tool name), `type` (event category), and `title` (short description). Optionally accepts `summary`, `files` (array), and `diff`. | After completing significant work. |
| `traz_show` | Full detail of a single event by ID. | After locating an event via `traz_search` or `traz_recent`. |
| `traz_diff` | Git diff associated with a past event. | Reviewing how a past bug was actually fixed in code. |
| `traz_stats` | Database statistics: event count, date range, type breakdown. | Diagnosing the health of the memory layer. |

---

## Event types for traz_add

Pick the type that best matches what happened — this is what makes `traz_search` and `traz_recap` useful later, so don't default to `note` out of laziness:

- `bug_fix` — A bug was identified and resolved.
- `feature` — A new capability was implemented.
- `decision` — An architectural or design choice was made, especially one that closes off alternatives.
- `refactor` — Code was restructured with no behavior change.
- `investigation` — Research into a problem, including ones left unresolved. Worth logging precisely because it saves the next person from repeating the dead end.
- `performance` — A measured performance improvement or profiling finding.
- `security` — A security fix or audit finding.
- `config` — Infrastructure, environment, or tooling change.
- `note` — Context worth keeping that doesn't fit the above. Use sparingly.

---

## What not to log

A memory layer is only useful if signal isn't buried in noise. Do not call `traz_add` for:

- Conversational exchanges, clarifying questions, or anything that didn't change the codebase or a decision.
- Work that was started and then abandoned with no resulting insight (if there's a real takeaway, that's an `investigation`, not nothing).
- Formatting-only or whitespace changes with no logical content.
- Anything you've already logged in this session — `traz_add` is one entry per completed unit of work, not a running log of every file edit.

If you're unsure whether something is worth recording, ask: "would a future agent or the user benefit from knowing this happened?" If no, skip it.

---

## Workflow examples

**Starting a session**
```
→ traz_recent (limit: 10) to see what's changed recently.
→ If the user names a specific area, traz_search on it.
→ Proceed with work informed by that context.
```

**Fixing a recurring bug**
```
User: "The auth token is expiring too early again."
→ traz_search("auth token expiry") before opening any code.
→ Review past attempts and their outcomes; apply or build on the most relevant one.
→ After fixing: traz_add(tool="Gemini", type="bug_fix", title="Fix auth token expiry", summary="Adjusted token expiry duration to match spec").
```

**Starting a refactor**
```
User: "Let's refactor the payment module."
→ traz_context to understand the architecture and prior decisions around it.
→ traz_checkpoint(summary="Refactoring payment module; stable base state marked.").
→ traz_search("payment module") for related history.
→ Do the refactor.
→ traz_add(tool="Gemini", type="refactor", title="Refactor payment module", summary="Restructured classes...").
```

**Ending a session**
```
→ traz_recap to confirm what got captured.
→ If something significant is missing, traz_add it before closing — don't rely on the next session to remember unlogged work.
```

---

## Tool-specific integration notes

The MCP tools above are identical across agents. Only the configuration surface differs:

| Tool | Rule file | MCP config | Setup command |
|---|---|---|---|
| Claude Code | `CLAUDE.md` | `~/.claude/claude_desktop_config.json` (`~/.config/Claude/claude_desktop_config.json` on Linux) | `traz setup claude` |
| OpenAI Codex | `AGENTS.md` | `~/.codex/config.toml` | `traz setup codex` |
| OpenCode | `AGENTS.md` | `~/.config/opencode/opencode.jsonc` | `traz setup opencode` |
| Cursor | `.cursorrules` | `~/.cursor/mcp.json` | `traz setup cursor` |
| GitHub Copilot / VS Code | `.github/copilot-instructions.md` | VS Code MCP extension settings or `settings.json` | `traz setup copilot` |
| Antigravity (agy) / Gemini CLI | `.agents/rules/traz.md` | `~/.gemini/settings.json` | `traz setup agy` |
| Aider | `CONVENTIONS.md` | — | `traz setup aider` |

Note for Codex: it reads MCP server instructions at initialization and receives the traz context summary automatically, without needing an explicit first call.

---

## CLI quick reference

Run `traz help` for the full command list.

```bash
traz init                    # Initialize traz in the current project
traz init --hook             # Install git hooks for automatic capture
traz init --with-embeddings  # Download the semantic search model
traz setup <tool>            # Configure traz for a specific AI agent
traz recent                  # Show recent events
traz search "<query>"        # Search event history
traz add --event-type <type> --title "<title>" --summary "<text>"
traz recap                   # Summarize today's events
traz context                 # High-level project context
traz doctor                  # Check health of the local database
traz serve                   # Start the REST API on :4000
traz mcp                     # Start the MCP stdio server (used by agents)
```

---

## If traz is unavailable or a call fails

Don't let the memory layer become a blocker. If `traz_mcp` isn't connected, a tool call errors, or the database is missing, proceed with the user's actual task using your own judgment, and mention briefly that traz context wasn't available for this step. If `traz doctor` reveals a corrupted or missing database, surface that to the user rather than silently skipping all memory operations for the rest of the session — they may want to re-run `traz init`.

---

## Quality rules

- Log substantive engineering events only — never conversational exchanges.
- Keep summaries to three sentences or fewer; optimize for token efficiency over completeness.
- State what changed and why, not just what was done — "why" is what makes an entry useful six months later.
- Reference concrete identifiers where relevant: filenames, module names, error codes, or commit hashes.
- One `traz_add` call per completed unit of work. If you're tempted to log the same thing twice in a session, you probably already logged it once — check `traz_recap` first.