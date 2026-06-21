# AGENTS.md — event-forecast

## Shared Fleet Standard

Also read and follow the shared fleet-level agent standard at `../AGENTS.md`. Treat this repository as owned product code: protect production stability, keep changes scoped, verify work, and record durable follow-up tasks when something remains incomplete or blocked.

## Project

- **Stack**: Rust, Rocket, SQLx, TimescaleDB, Tokio.
- **Local dev**: `docker compose up -d` (TimescaleDB) · `cargo run` · `cargo test`
- **Product docs**: `docs/product-brief.md` · backlog PRDs in `docs/prd-*.md`
- **Do not** touch production DB credentials or deploy config without explicit approval.
