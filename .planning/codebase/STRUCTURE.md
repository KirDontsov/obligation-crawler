# Codebase Structure

**Analysis Date:** 2026-06-09

## Directory Layout

```
obligation-crawler/
├── Cargo.toml                  # Crate manifest + dependencies
├── Cargo.lock                  # Pinned dependency versions
├── rustfmt.toml                # Formatting config (tabs, per fmt rules)
├── Dockerfile                  # Container build
├── docker-compose.yml          # Local stack (Postgres, RabbitMQ, etc.)
├── start.sh                    # Container/app entrypoint script
├── start_driver.sh             # Launches chromedriver on :9515
├── CLAUDE.md                   # Project rules for Claude
├── migrations/
│   └── 001_create_crawler_schema.sql   # Postgres schema (runs, bonds, views)
├── src/
│   ├── main.rs                 # Entry, RUN_MODE dispatch
│   ├── config.rs               # CrawlerConfig + ConfigError
│   ├── error.rs                # CrawlerError + Result<T> alias
│   ├── database.rs             # PgPool factory
│   ├── controllers/            # Orchestration layer
│   │   ├── mod.rs
│   │   └── bonds_crawler.rs
│   ├── services/               # Business logic (own external resources)
│   │   ├── mod.rs
│   │   ├── bonds_crawler.rs     # WebDriver scraping
│   │   ├── opencode_service.rs  # AI enrichment via opencode CLI
│   │   ├── rabbitmq_producer.rs
│   │   └── rabbitmq_consumer.rs
│   ├── models/                 # Pure data structs + CSV I/O
│   │   ├── mod.rs
│   │   ├── bonds.rs             # Bond, BondListItem
│   │   └── rabbitmq.rs          # CrawlerTask, BondCrawlerTask
│   ├── repository/             # Persistence (sqlx)
│   │   ├── mod.rs
│   │   └── bonds_repository.rs
│   ├── api/                    # DTO / response shapes
│   │   ├── mod.rs
│   │   └── bonds.rs
│   └── shared/                 # Pure utilities (no I/O, no async)
│       ├── mod.rs
│       └── utils.rs
├── ai/                         # Project knowledge base (not compiled)
│   ├── context.md
│   ├── workflow.md
│   ├── docs/                   # code-style, error-handling, async, testing, module-arch
│   ├── plans/                  # Dated implementation plans
│   └── research/               # Research notes
├── .claude/
│   └── skills/                 # Slash-command skills (research, plan, service, model, test, fix, review)
├── output/                     # Generated CSV files (gitignored)
└── target/                     # Cargo build artifacts (gitignored)
```

## Directory Purposes

**`src/` (crate root):**
- Purpose: All compiled Rust source.
- Contains: foundation files + one subdirectory per layer.
- Key files: `main.rs`, `config.rs`, `error.rs`, `database.rs`.

**`src/controllers/`:**
- Purpose: Thin orchestration that sequences service calls.
- Contains: per-feature orchestrators.
- Key files: `src/controllers/bonds_crawler.rs` (`run_bonds_crawler`, `collect_bonds_once`).

**`src/services/`:**
- Purpose: Business logic; each service struct owns an external resource and exposes an async lifecycle.
- Contains: scraper, AI subprocess wrapper, MQ producer/consumer.
- Key files: `src/services/bonds_crawler.rs`, `src/services/opencode_service.rs`, `src/services/rabbitmq_producer.rs`, `src/services/rabbitmq_consumer.rs`.

**`src/models/`:**
- Purpose: Pure data structs; CSV I/O methods allowed on structs.
- Contains: domain models + MQ message shapes.
- Key files: `src/models/bonds.rs`, `src/models/rabbitmq.rs`.

**`src/repository/`:**
- Purpose: Database access via sqlx (insert run, save bond, finish run).
- Contains: `BondsRepository` plus `sqlx::FromRow` record structs.
- Key files: `src/repository/bonds_repository.rs`.

**`src/api/`:**
- Purpose: DTO/response envelopes for JSON serialization.
- Contains: `BondsResponse`, `BondsApiResponse`.
- Key files: `src/api/bonds.rs`.

**`src/shared/`:**
- Purpose: Pure helpers with no `crate::` deps beyond `error`.
- Contains: date/time utilities.
- Key files: `src/shared/utils.rs`.

**`ai/`:**
- Purpose: Documentation and workflow knowledge base; never compiled.
- Contains: `docs/` (style, error handling, async patterns, testing, module architecture), `plans/`, `research/`, `context.md`, `workflow.md`.

**`.claude/skills/`:**
- Purpose: Slash-command definitions (`/research`, `/plan`, `/service`, `/model`, `/test`, `/fix`, `/review`).
- Contains: one `SKILL.md` per skill subdirectory.

## Key File Locations

**Entry Points:**
- `src/main.rs`: `main()` + `run_direct_mode()` / `run_consumer_mode()`.

**Configuration:**
- `src/config.rs`: `CrawlerConfig::from_env()`.
- `.env` / `.env.example`: env variables (gitignored; never read contents).
- `rustfmt.toml`: formatting rules.

**Core Logic:**
- `src/services/bonds_crawler.rs`: scraping pipeline.
- `src/services/opencode_service.rs`: AI prompt + subprocess.
- `src/repository/bonds_repository.rs`: persistence.

**Schema:**
- `migrations/001_create_crawler_schema.sql`: tables `obligation_crawler_runs`, `obligation_crawler_bonds`, views.

**Testing:**
- No dedicated test files present yet; `time = "0.3.36"` is the only dev-dependency. Tests follow patterns in `ai/docs/testing-guidelines.md`.

## Naming Conventions

**Files:**
- snake_case module files: `bonds_crawler.rs`, `rabbitmq_producer.rs`, `opencode_service.rs`.
- Each layer directory has a `mod.rs` that declares `pub mod x;` and re-exports `pub use x::*;`.

**Directories:**
- One lowercase noun per layer: `services/`, `models/`, `controllers/`, `repository/`, `api/`, `shared/`.

**Types:** PascalCase structs/enums (`BondsCrawler`, `CrawlerConfig`, `CrawlerError`, `BondListItem`).
**Functions:** snake_case (`from_env`, `run_crawl_loop`, `analyze_bond`, `save_bond`).
**DB objects:** `obligation_crawler_` table prefix; `idx_obligation_` index prefix.
**Output files:** `./output/bonds_<DD-MM-YYYY_HH-MM-SS>.csv` (`src/services/bonds_crawler.rs:454`).

## Where to Add New Code

**New service:**
- Implementation: `src/services/<name>.rs`.
- Register: add `pub mod <name>; pub use <name>::*;` to `src/services/mod.rs`.
- Inject `CrawlerConfig`; import `crate::error::Result`; follow new/initialize/run/close lifecycle.

**New data model:**
- Implementation: `src/models/<name>.rs`; register in `src/models/mod.rs`.
- Derive `Debug, Clone, Serialize, Deserialize`; nullable fields as `Option<T>`.

**New controller / orchestration:**
- Implementation: `src/controllers/<name>.rs`; register in `src/controllers/mod.rs`.
- May import `services`, `models`, `config`, `error` — never the reverse.

**New persistence query:**
- Add method to `BondsRepository` in `src/repository/bonds_repository.rs` (or a new repo file + `mod.rs` entry).

**New error variant:**
- Add to `CrawlerError` in `src/error.rs`; use `#[from]` for unique source types.

**New config option:**
- Add field to `CrawlerConfig` and read in `from_env()` (`src/config.rs`); document in `.env.example`.

**Schema change:**
- New file `migrations/00N_<description>.sql`.

**Utilities:**
- Pure helpers: `src/shared/utils.rs` (no async, no service deps).

## Special Directories

**`output/`:**
- Purpose: Generated CSV exports.
- Generated: Yes (created at runtime by `BondListItem::create_csv_file`).
- Committed: No (gitignored).

**`target/`:**
- Purpose: Cargo build artifacts.
- Generated: Yes.
- Committed: No (gitignored).

**`ai/` and `.claude/`:**
- Purpose: Knowledge base and Claude skills.
- Generated: No.
- Committed: Yes.

---

*Structure analysis: 2026-06-09*
