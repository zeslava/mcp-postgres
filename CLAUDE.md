# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`db-mcp` is a single-binary stdio MCP server exposing read-only SQL tools across multiple database engines. The engine is chosen at runtime by URL scheme (`postgres://`, `sqlite://`); backends live behind a `Database` trait and Cargo features.

See [AGENTS.md](./AGENTS.md) for commands, architecture, conventions, and safety rules. It is the source of truth — keep it updated rather than duplicating its contents here.
