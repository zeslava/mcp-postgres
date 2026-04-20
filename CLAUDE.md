# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build          # compile
cargo run -- --database-url postgres://user:pass@host/db
DATABASE_URL=postgres://user:pass@host/db cargo run
cargo clippy         # lint
cargo fmt            # format
```

## Architecture

Single-binary stdio MCP server (`src/main.rs`). No modules — everything lives in one file.

**Stack:** `rmcp` 1.5 (MCP SDK) + `tokio-postgres` + `clap` for CLI/env config.

**Pattern:** `PgServer` struct holds `Arc<tokio_postgres::Client>` and a `ToolRouter<PgServer>`. Tools are defined with `#[tool]` on `impl PgServer` block annotated `#[tool_router]`. The `ServerHandler` impl uses `#[tool_handler]` to wire routing automatically.

**Tools exposed:**
- `query` — SELECT only (rejected otherwise), returns JSON array of row objects
- `list_tables` — queries `information_schema.tables`, excludes system schemas
- `describe_table` — queries `information_schema.columns`, accepts `table` + `schema` (default `public`)

**Type mapping:** `pg_value_to_json()` converts postgres column types to `serde_json::Value` by matching on `tokio_postgres::types::Type`. Unknown types fall back to `&str`.

**Transport:** stdio (`rmcp::transport::stdio`). Tracing writes to stderr to avoid polluting the MCP JSON-RPC stream on stdout.
