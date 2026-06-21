# Project Recommendation Context

Generated: 2026-06-06T21:14:19.546Z

This file is a CodeVetter Repo Unpacked-inspired audit written for Starboard recommendations. It is intentionally local, evidence-oriented, and safe to commit: it records product context, feature areas, stack inventory, and recommendation guidance without secrets or environment values.

## Project Identity

- Slug: `event-forecast`
- Registry description: Time-series forecasting product that ingests event, order, and operations streams to predict future demand, requirements, or anomalies.
- Product grouping: `internal-first`
- Source path: `event-forecast`

## Product Context

Time-series forecasting product that ingests event, order, and operations streams to predict future demand, requirements, or anomalies.

Event Forecast is a time-series forecasting product for event, order, and operations streams. It is now a Rust + Rocket web service backed by TimescaleDB. It takes historical timestamped data and predicts future demand, requirements, anomalies, and likely categorical properties such as location, service type, product type, food orders, or factory needs. The first implementation is intentionally a transparent baseline. It should prove the data shape and evaluation loop before adding heavier models.

Event Forecast Event Forecast predicts the next events in a stream and the likely properties on those events: location, service type, product type, or any other categorical field supplied in properties . The first version is a Rust + Rocket web service backed by TimescaleDB. The prediction model is still a transparent baseline, not a black-box ML model. It learns from historical event sequences using: - event-type transitions - median time between events - property transitions between neighboring events - property distributions conditioned on event-type transitions This makes the output debuggable before the project adds heavier forecasting or spatiotemporal models. Current Input Shape Top-l

## Feature Map

- **AI agents**: Agents, tool use, workflows, orchestration, RAG, evals, and model integration. Keywords: ai, agent, agents, llm, rag, embedding, eval, model.
- **Testing and quality**: Unit tests, browser tests, evals, CI quality gates, and regression checks. Keywords: test, testing, quality, vitest, playwright, ci, eval, benchmark.
- **Analytics and intelligence**: Signal analysis, forecasting, monitoring, trends, metrics, and decision support. Keywords: analytics, intelligence, signal, forecast, monitoring, metric, trend, insight.
- **UI workflows**: Dashboards, tables, forms, component systems, charts, and user workflows. Keywords: ui, ux, dashboard, table, component, react, next, tailwind.
- **Database and storage**: SQL, document storage, migrations, cache, queues, vectors, and persistence. Keywords: database, db, sql, sqlite, postgres, turso, libsql, drizzle.
- **Ingestion and sync**: External API ingestion, sync jobs, scraping, enrichment, and scheduled updates. Keywords: sync, ingest, ingestion, scrape, scraping, enrich, crawler, etl.
- **Content and media**: Content production, video, reels, documents, markdown, and publishing workflows. Keywords: content, media, video, reel, markdown, document, publish, editor.

## Runtime Surfaces and Entrypoints

- `src/main.rs`

## Current Stack

- Languages: `Rust`
- Frameworks/tools: `Cargo`
- Config files:
- `Cargo.toml`

## OSS Already In Use

Direct dependencies:
- Not detected in this pass.

Development dependencies:
- Not detected in this pass.

Package scripts:
- Not detected in this pass.

## Testing and Quality Signals

- `tests/action_report_fixture.rs`
- `tests/evaluate_cli.rs`
- `tests/fixtures/sample-stream.json`

## Recommendation Guidance

Good matches:
- Repos that strengthen ai agents without replacing already-installed libraries.
- Repos that strengthen testing and quality without replacing already-installed libraries.
- Repos that strengthen analytics and intelligence without replacing already-installed libraries.
- Repos that strengthen ui workflows without replacing already-installed libraries.
- Repos that strengthen database and storage without replacing already-installed libraries.
- Repos that strengthen ingestion and sync without replacing already-installed libraries.
- Repos that strengthen content and media without replacing already-installed libraries.
- Tools with concrete support for events, event, json, bash, product, stream, report, curl.
- Implementation repos, SDKs, CLIs, testing utilities, adapters, and focused libraries are higher value than generic awesome lists.

Avoid recommending:
- Do not recommend packages already listed under direct or development dependencies unless the task is migration research.
- Do not recommend broad framework replacements unless the project context explicitly calls for a rewrite.
- Downrank curated lists, archived repos, stale demos, and generic UI kits that do not map to the feature catalog.

## Evidence Read

Primary docs and handoff files:
- `PROJECT_STATUS.md`
- `README.md`
- `docs/product-brief.md`

Package manifests:
- Not detected in this pass.

Inventory notes:
- Files scanned: 21
- This pass uses deterministic repo inventory plus local documentation/source-path evidence. It does not claim a full manual line-by-line review of every source file.

## Confidence

Confidence: **medium**

Why:
- PROJECT_STATUS.md present
- README.md present
- 3 test/quality files identified

Refresh command:

```bash
cd /Users/sarthak/Desktop/fleet/starboard
pnpm fleet:audit-recommendation-context
pnpm fleet:extract-projects
```
