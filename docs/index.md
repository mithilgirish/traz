---
layout: home

features:
  - title: Shared AI Memory
    details: Stop losing context between Claude Code, Cursor, and Aider. Traz acts as a persistent memory plane they can all read and write to natively.
  - title: Local-First SQLite
    details: Built in pure Rust. Zero cloud dependencies, zero latency, and absolute privacy. Your codebase architecture never leaves your machine.
  - title: Native MCP Server
    details: Built entirely on the Model Context Protocol (MCP). It hooks into your workflow with one command, delivering dense, compressed context timelines.
  - title: Semantic Vector Search
    details: Features hybrid search powered by local ONNX-accelerated MiniLM embeddings and FTS5, merging keyword fidelity with semantic understanding.
  - title: Meet Cuby (The Context Pet)
    details: Take a break from coding! traz includes a built-in virtual Tamagotchi pet named Cuby. You can feed him context, ask him questions, or watch him dance right in your terminal.
  - title: Token Compressed
    details: By utilizing Dense Output Formatting, traz compresses context payloads by up to 75%, preserving your LLM's precious context window limit.
---

<br>

<p align="center">
  <sub>Built for developers who switch tools, not context.</sub>
</p>

## The Context Fragmentation Problem

Modern software development increasingly relies on multiple, specialized AI agents. A standard workflow might involve:
1. Triaging a stack trace in Claude Code.
2. Refactoring a dense component within Cursor IDE.
3. Generating unit tests via a terminal agent like Aider or Antigravity.

At every tool boundary, **context is lost**. The AI in the IDE does not inherently know what the AI in the terminal just fixed. Every new session starts as an isolated environment, forcing developers to manually rebuild the context window.

## The Solution

`traz` resolves this by acting as an automated engineering diary. It establishes a highly searchable, persistent timeline. This timeline is securely stored locally and exposed via the Model Context Protocol, ensuring that subsequent AI sessions inherit the context of previous decisions.



