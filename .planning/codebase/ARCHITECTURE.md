<!-- refreshed: 2026-06-09 -->
# Architecture

**Analysis Date:** 2026-06-09

## System Overview

```text
┌─────────────────────────────────────────────────────────────┐
│                          main.rs                             │
│  Loads .env, inits logger, builds DB pool, dispatches mode   │
│  `src/main.rs`  (RUN_MODE: "direct" | "consumer")            │
├──────────────────────────────┬──────────────────────────────┤
│      run_direct_mode()       │      run_consumer_mode()      │
│  scrape → enrich → publish   │   RabbitMQConsumer loop       │
└──────────────┬───────────────┴───────────────┬──────────────┘
               │                                │
               ▼                                ▼
┌─────────────────────────────────────────────────────────────┐
│                       controllers/                           │
│  Orchestration (thin): sequences service calls               │
│  `src/controllers/bonds_crawler.rs`                          │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                         services/                            │
│  BondsCrawler (WebDriver)   RabbitMQProducer  RabbitMQConsumer│
│  opencode_service (AI)                                       │
│  `src/services/*.rs`                                         │
└───────┬────────────────┬───────────────┬───────────────┬────┘
        │                │               │               │
        ▼                ▼               ▼               ▼
┌──────────────┐ ┌──────────────┐ ┌──────────┐ ┌──────────────┐
│   models/    │ │  repository/ │ │   api/   │ │   shared/    │
│ Bond,        │ │ BondsReposit.│ │ DTO      │ │ time helpers │
│ BondListItem │ │ (sqlx)       │ │ shapes   │ │              │
│ `src/models` │ │ `src/repo...`│ │`src/api` │ │`src/shared`  │
└──────┬───────┘ └──────┬───────┘ └──────────┘ └──────────────┘
       │                │
       ▼                ▼
┌──────────────┐ ┌──────────────────────────────────────────┐
│ CSV output   │ │ PostgreSQL (via database.rs PgPool)       │
│ ./output/*.csv│ │ obligation_crawler_runs / _bonds          │
└──────────────┘ └──────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| Entry / mode dispatch | Load env, build DB pool, route by `RUN_MODE` | `src/main.rs` |
| Config | `CrawlerConfig` built from env vars with defaults | `src/config.rs` |
| Errors | `CrawlerError` enum + `Result<T>` alias | `src/error.rs` |
| DB pool factory | `create_connection_pool() -> PgPool` | `src/database.rs` |
| Scraper service | Owns `WebDriver`, scrapes T-Bank bond rows + detail pages | `src/services/bonds_crawler.rs` |
| AI enrichment | Calls `opencode` CLI subprocess, builds risk prompt | `src/services/opencode_service.rs` |
| MQ producer | Owns lapin `Connection`/`Channel`, publishes JSON | `src/services/rabbitmq_producer.rs` |
| MQ consumer | Reconnecting consume loop with handler callback | `src/services/rabbitmq_consumer.rs` |
| Controllers | Thin orchestration over `BondsCrawler` | `src/controllers/bonds_crawler.rs` |
| Data models | `Bond`, `BondListItem`, CSV I/O methods | `src/models/bonds.rs` |
| MQ message models | `CrawlerTask`, `BondCrawlerTask` | `src/models/rabbitmq.rs` |
| Persistence | `BondsRepository` insert/update via sqlx | `src/repository/bonds_repository.rs` |
| API DTOs | `BondsResponse`, `BondsApiResponse` | `src/api/bonds.rs` |
| Utilities | Date/time helpers | `src/shared/utils.rs` |

## Pattern Overview

**Overall:** Layered, service-oriented modular monolith with strict downward dependency direction.

**Key Characteristics:**
- Service objects own external resources (`WebDriver`, lapin `Connection`/`Channel`) and expose an async lifecycle.
- Configuration is injected, never read from env inside services (env reads live in `main.rs`, `config.rs`, `database.rs`).
- A single `CrawlerError` enum unifies all failure modes; `crate::error::Result<T>` is the standard return type.
- Graceful degradation: DB and RabbitMQ are optional; the crawler continues if either is unavailable (`db_pool: Option<PgPool>`, `producer: Option<...>`).
- Two run modes dispatched from one binary: `direct` (scrape pipeline) and `consumer` (MQ worker).

## Layers

**main.rs (entry/dispatch):**
- Purpose: Bootstrap and route to a run mode.
- Location: `src/main.rs`
- Contains: `main()`, `run_direct_mode()`, `run_consumer_mode()`; env reads; user-facing `println!`.
- Depends on: `config`, `database`, `services`, `error`.
- Used by: nothing (binary root).

**config / error / database (foundation):**
- Purpose: Cross-cutting primitives.
- Location: `src/config.rs`, `src/error.rs`, `src/database.rs`
- Contains: `CrawlerConfig` + `ConfigError`; `CrawlerError` + `Result<T>`; `PgPool` factory.
- Depends on: external crates only (`error` also depends on `config::ConfigError`).
- Used by: every higher layer.

**controllers (orchestration):**
- Purpose: Sequence service calls; no scraping/business logic of its own.
- Location: `src/controllers/bonds_crawler.rs`
- Contains: `run_bonds_crawler()`, `collect_bonds_once()`.
- Depends on: `services`, `models`, `config`, `error`.
- Used by: `main.rs` (note: current `main.rs` calls `BondsCrawler` directly; controllers are the intended seam).

**services (business logic):**
- Purpose: Own external resources and implement scraping, AI, MQ I/O.
- Location: `src/services/`
- Contains: `BondsCrawler`, `RabbitMQProducer`, `RabbitMQConsumer`, `analyze_bond`.
- Depends on: `models`, `api`, `config`, `error`, `shared`, `repository`.
- Used by: `controllers`, `main.rs`.

**models / api / repository / shared (data + edges):**
- Purpose: Data structs, DTOs, persistence, pure helpers.
- Location: `src/models/`, `src/api/`, `src/repository/`, `src/shared/`
- Contains: structs, sqlx queries, time utilities.
- Depends on: `models` ← `api`/`repository`; `shared` depends on external crates only.
- Used by: `services`, `controllers`, `main.rs`.

## Data Flow

### Primary Request Path (direct mode)

1. `main()` loads `.env`, builds optional `PgPool`, reads `RUN_MODE=direct` (`src/main.rs:21`).
2. `run_direct_mode()` builds `CrawlerConfig::from_env()` and an optional `RabbitMQProducer` (`src/main.rs:55`).
3. `BondsCrawler::new(config, db_pool)` creates the CSV file and a DB run record (`src/services/bonds_crawler.rs:453`).
4. `run_crawl_loop()` calls `initialize()` → `navigate_to_bonds()` → `wait_for_login()` → polling loop of `collect_bonds()` (`src/services/bonds_crawler.rs:642`).
5. Per row, `parse_bond_row_inner()` scrapes list + detail page, then conditionally calls `analyze_bond()` (opencode) (`src/services/bonds_crawler.rs:12`, `src/services/opencode_service.rs:6`).
6. Each `BondListItem` is appended to CSV and saved via `BondsRepository::save_bond()` if a pool exists (`src/services/bonds_crawler.rs:219`, `:602`).
7. After collection, run is marked completed in DB and the JSON batch is published to RabbitMQ if enabled (`src/services/bonds_crawler.rs:694`, `src/main.rs:166`).
8. `crawler.close()` quits the WebDriver (`src/services/bonds_crawler.rs:709`).

### Consumer flow

1. `run_consumer_mode()` reads `RABBITMQ_URL` / `RABBITMQ_QUEUE` (`src/main.rs:178`).
2. `RabbitMQConsumer::new(...).start_consuming(handler)` enters a reconnecting loop, declares queue, sets QoS=1, consumes and acks messages (`src/services/rabbitmq_consumer.rs:19`).

**State Management:**
- Mutable scraping state lives inside the `BondsCrawler` struct (`driver`, `csv_filename`, `db_pool`, `run_id`); accessed via `&mut self`.
- No global mutable state; resources are owned per-instance.

## Key Abstractions

**CrawlerConfig (dependency injection):**
- Purpose: Single typed config object built once and passed into services.
- Examples: `src/config.rs`, injected at `src/services/bonds_crawler.rs:453`.
- Pattern: `from_env()` reads vars with defaults; services receive a value, never read env.

**CrawlerError / Result<T> alias:**
- Purpose: Unified error type with `#[from]` conversions for sqlx, lapin, reqwest, io, WebDriver, serde_json.
- Examples: `src/error.rs:5`, alias at `src/error.rs:44`.
- Pattern: functions return `crate::error::Result<T>`; `?` propagates with automatic conversion.

**Async service lifecycle (new / initialize / run / close):**
- Purpose: Consistent resource ownership and teardown.
- Examples: `BondsCrawler::new` / `initialize` / `run_crawl_loop` / `close` (`src/services/bonds_crawler.rs:453`–`714`); also a `Drop` impl spawning `quit()` (`src/services/bonds_crawler.rs:756`).
- Pattern: `new()` is sync setup, `initialize()` acquires the external resource, run methods do work, `close()` releases it.

**Optional dependencies:**
- Purpose: Degrade gracefully when DB/MQ unavailable.
- Examples: `db_pool: Option<PgPool>` (`src/services/bonds_crawler.rs:448`), `producer: Option<RabbitMQProducer>` (`src/main.rs:67`).

## Entry Points

**Binary main:**
- Location: `src/main.rs:20` (`#[tokio::main] async fn main`).
- Triggers: `cargo run` / container start (`start.sh`).
- Responsibilities: env load, DB pool, `RUN_MODE` dispatch.

**RUN_MODE=direct:**
- Location: `src/main.rs:55`.
- Triggers: default mode.
- Responsibilities: full scrape → enrich → CSV/DB/MQ pipeline.

**RUN_MODE=consumer:**
- Location: `src/main.rs:178`.
- Triggers: `RUN_MODE=consumer`.
- Responsibilities: long-running RabbitMQ worker.

## Architectural Constraints

- **Module layer rule:** Lower layers must NOT import higher layers. `services` cannot import `controllers`; `models` cannot import `services`. Full matrix in `ai/docs/module-architecture.md`.
- **No env reads in services:** All `env::var` lives in `main.rs`, `config.rs`, `database.rs`. Services receive config via `CrawlerConfig`.
- **Threading:** Single Tokio multi-threaded runtime (`tokio = { features = ["full"] }`). WebDriver work is sequential within `BondsCrawler`; the `Drop` impl uses `tokio::spawn` for async cleanup.
- **Global state:** None. All resources are instance-owned.
- **Optional infra:** DB and RabbitMQ are optional; code must handle `None` pools/producers.
- **Numeric mapping:** Rust `f64` ↔ Postgres `DOUBLE PRECISION`; date fields stored as text `DD.MM.YYYY` strings (`migrations/001_create_crawler_schema.sql`).

## Anti-Patterns

### Business logic / orchestration in `main.rs`

**What happens:** `run_direct_mode()` instantiates `BondsCrawler` and `RabbitMQProducer` directly and contains the publish logic, duplicating what `controllers/bonds_crawler.rs` already provides.
**Why it's wrong:** Bypasses the controller orchestration seam, making the pipeline harder to test and reuse.
**Do this instead:** Route scraping through `controllers::run_bonds_crawler()` (`src/controllers/bonds_crawler.rs:6`) and keep `main.rs` to dispatch + presentation.

### Cross-layer imports

**What happens:** A service importing from `controllers`, or `models` importing from `services`.
**Why it's wrong:** Violates the documented dependency direction and creates cycles.
**Do this instead:** Keep imports downward only — see the layer matrix in `ai/docs/module-architecture.md:19`.

### Business logic in models

**What happens:** Putting scraping/async fetch methods on `BondListItem`.
**Why it's wrong:** Models must stay pure data; scraping belongs in services.
**Do this instead:** Scraping stays in `BondsCrawler::collect_bonds()` (`src/services/bonds_crawler.rs:535`). CSV I/O on the struct is the one accepted exception (tightly coupled to data shape).

### Unguarded access to uninitialized resources

**What happens:** `self.driver.as_ref().unwrap()` before `initialize()`.
**Why it's wrong:** Panics in production; the project bans `unwrap`/`expect`.
**Do this instead:** Guard with `.ok_or_else(|| CrawlerError::SeleniumError(...))?` as in `src/services/bonds_crawler.rs:507`.

## Error Handling

**Strategy:** Single `CrawlerError` enum with `?`-based propagation and `From` conversions; non-fatal failures (DB save, AI analysis, CSV append) are logged/warned and the loop continues.

**Patterns:**
- `#[from]` conversions for sqlx, lapin, reqwest, io, WebDriver; manual `From<serde_json::Error>` (`src/error.rs:38`).
- Degrade-don't-crash: failed DB/MQ operations `warn!` and proceed (`src/services/bonds_crawler.rs:602`).
- `opencode_service` is the exception — it returns `Box<dyn Error + Send + Sync>` rather than `CrawlerError` (`src/services/opencode_service.rs:6`).

## Cross-Cutting Concerns

**Logging:** `log` macros (`info!`, `warn!`, `error!`) via `env_logger`; `main.rs` and scraper also use raw `println!`/`eprintln!` for user-facing output (project rule: `log` macros only outside `main.rs`).
**Validation:** Config validation in `CrawlerConfig::from_env()`; scraped values parsed with `.ok()` fallbacks to `None`.
**Authentication:** Manual browser login during a `wait_after_login_seconds` window (`src/services/bonds_crawler.rs:499`); no stored credentials. RabbitMQ/Postgres creds come from URLs in env.

---

*Architecture analysis: 2026-06-09*
