# Obligation Crawler — Project Context

**Language:** Rust 2021  
**Async runtime:** Tokio (full features)  
**Purpose:** Scrapes bond (облигация) listings from T-Bank investments portal, enriches with AI analysis, publishes to RabbitMQ or saves to CSV/PostgreSQL

---

## RPI Workflow (Research — Plan — Implement)

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  RESEARCH   │────▶│    PLAN     │────▶│  IMPLEMENT  │
│  (FAR scale)│     │ (FACTS scale)│    │ (validation)│
└─────────────┘     └─────────────┘     └─────────────┘
```

### Phase 1: RESEARCH

**Goal:** Gather facts, understand existing patterns, create research doc.

**Tools:**
- `/research [FeatureName]` — Create research documentation
- Reverse Prompting — Ask clarifying questions ONE AT A TIME

**FAR Validation:**
| Criterion | Description |
|-----------|-------------|
| **F**actual | Based on actual code, not assumptions |
| **A**ctionable | Clear what to build |
| **R**elevant | Solves real need |

**Output:** `ai/research/[feature-name].md`

### Phase 2: PLAN

**Goal:** Decompose work into atomic tasks.

**Tools:**
- `/plan [TaskDescription]` — Dual-agent adversarial planning

**FACTS Validation:**
| Criterion | Description |
|-----------|-------------|
| **F**easible | Technically achievable |
| **A**tomic | One task = one action |
| **C**lear | Clear formulation |
| **T**estable | Has success criteria |
| **S**coped | Right scope |

**Output:** `ai/plans/YYYY-MM-DD-[task-slug].md`

### Phase 3: IMPLEMENT

**Goal:** Systematic task execution with validation.

**Tools:**
- `/service`, `/model`, `/test`, `/fix` — Implementation skills

**Validation gates:**
- `cargo build` passes
- `cargo test` passes
- `cargo clippy -- -D warnings` clean
- `cargo fmt --check` passes

---

## Stack

### Core Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1.33 | Async runtime (full features) |
| `thirtyfour` | 0.32 | Selenium/WebDriver automation |
| `lapin` | 3.7 | RabbitMQ (AMQP) producer + consumer |
| `sqlx` | 0.8 | PostgreSQL async ORM |
| `reqwest` | 0.12 | HTTP client |
| `serde` / `serde_json` | 1.x | Serialization |
| `thiserror` | 1.x | Error type derivation |
| `log` / `env_logger` | 0.4/0.11 | Structured logging |
| `chrono` | 0.4 | Date/time handling |
| `csv` | 1.3 | CSV output |
| `uuid` | 1.4 | UUID generation |
| `dotenv` | 0.15 | `.env` loading |

### External Services

| Service | How used |
|---------|----------|
| ChromeDriver (port 9515) | WebDriver automation for T-Bank scraping |
| RabbitMQ | Task queue (consumer mode) + results publishing |
| PostgreSQL | Optional persistent storage |
| OpenCode CLI | AI analysis of bonds via `opencode run` subprocess |

---

## Run Modes

| Mode | Env `RUN_MODE` | Description |
|------|----------------|-------------|
| Direct | `direct` | Standalone: initialize WebDriver → wait for login → crawl → output |
| Consumer | `consumer` | Listen on RabbitMQ queue, handle tasks via callback |

---

## Project Structure

```
src/
├── main.rs              # Entry point, mode dispatch
├── config.rs            # CrawlerConfig from env vars
├── error.rs             # CrawlerError enum (thiserror), Result<T> alias
├── database.rs          # PgPool factory (sqlx)
├── models/
│   ├── mod.rs
│   ├── bonds.rs         # Bond, BondListItem structs + CSV helpers
│   └── rabbitmq.rs      # RabbitMQ message types
├── api/
│   ├── mod.rs
│   └── bonds.rs         # BondsResponse, BondsApiResponse (HTTP API shapes)
├── services/
│   ├── mod.rs
│   ├── bonds_crawler.rs # BondsCrawler — WebDriver lifecycle + scraping
│   ├── opencode_service.rs # analyze_bond() — AI analysis via opencode CLI
│   ├── rabbitmq_producer.rs # RabbitMQProducer — publish to exchange
│   └── rabbitmq_consumer.rs # RabbitMQConsumer — consume from queue
├── controllers/
│   ├── mod.rs
│   └── bonds_crawler.rs # Controller layer (orchestration)
└── shared/
    ├── mod.rs
    └── utils.rs         # DateTime utilities (now, format_datetime, etc.)
```

---

## Module Responsibilities

| Module | Responsibility |
|--------|----------------|
| `config` | Reads env vars, validates, produces `CrawlerConfig` |
| `error` | Defines `CrawlerError` variants, `Result<T>` alias |
| `database` | Creates `PgPool` connection pool |
| `models` | Pure data structs: `Bond`, `BondListItem`, CSV I/O |
| `api` | HTTP response shapes (serializable DTOs) |
| `services` | Business logic: crawler, producer, consumer, AI analysis |
| `controllers` | Orchestration: sequences service calls |
| `shared` | Reusable pure utilities (no service dependencies) |

---

## Architecture Patterns

### Error Handling

```rust
// All fallible functions return crate::error::Result<T>
pub async fn scrape(&mut self) -> Result<Vec<BondListItem>> { ... }

// New error variants added to CrawlerError in error.rs
#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Crawler error: {0}")]
    CrawlerError(String),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    // ...
}
```

### Config Pattern

```rust
// Config loaded once at startup, cloned into services
let config = CrawlerConfig::from_env()?;
let crawler = BondsCrawler::new(config.clone());
```

### Service Pattern (struct with async methods)

```rust
pub struct SomeService {
    config: CrawlerConfig,
    // stateful resources: WebDriver, Channel, PgPool
}

impl SomeService {
    pub async fn new(config: CrawlerConfig) -> Result<Self> { ... }
    pub async fn run(&mut self) -> Result<Output> { ... }
    pub async fn close(&mut self) -> Result<()> { ... }
}

impl Drop for SomeService {
    fn drop(&mut self) {
        // async cleanup via tokio::spawn
    }
}
```

### Logging

```rust
use log::{info, warn, error, debug};

// Use log macros — NOT println! in production code
info!("Starting crawler for {} bonds", count);
warn!("Retrying after error: {}", e);
error!("Failed to connect: {}", e);
debug!("[parse_row] cells={}", cells.len());

// println! only acceptable in main.rs for user-facing progress
```

### WebDriver Pattern

```rust
// Driver accessed via Option<WebDriver> field
let driver = self.driver.as_mut().ok_or_else(|| {
    CrawlerError::SeleniumError("WebDriver not initialized".to_string())
})?;

// Always close via .quit() in Drop or explicit close()
```

### RabbitMQ Consumer Pattern

```rust
// Infinite reconnect loop with 5s backoff
// message_handler is FnMut(String) -> BoxFuture<Result<()>>
consumer.start_consuming(|msg| Box::pin(async move {
    // handle message
    Ok(())
})).await?;
```

---

## Configuration (`.env`)

| Variable | Default | Description |
|----------|---------|-------------|
| `RUN_MODE` | `direct` | `direct` or `consumer` |
| `TBANK_URL` | tbank bonds url | URL to scrape |
| `HEADLESS_CHROME` | `false` | Run Chrome headlessly |
| `CHROME_DRIVER_PATH` | `./chromedriver` | Path to chromedriver binary |
| `WAIT_AFTER_LOGIN_SECONDS` | `60` | Wait for manual login |
| `POLL_INTERVAL_SECONDS` | `30` | Polling interval |
| `MAX_RETRIES` | `3` | Max retry attempts |
| `DURATION_MINUTES` | (unset) | Run duration; unset = infinite |
| `ENABLE_RABBITMQ` | `false` | Publish results to RabbitMQ |
| `RABBITMQ_URL` | `amqp://guest:guest@localhost:5672` | RabbitMQ connection |
| `RABBITMQ_EXCHANGE` | `obligation_exchange` | Exchange name |
| `RABBITMQ_QUEUE` | `obligation_crawler_queue` | Queue name (consumer mode) |
| `DATABASE_URL` | `postgresql://...` | PostgreSQL connection string |

---

## Build & Run

```bash
# Build
cargo build
cargo build --release

# Run with env file
source .env && cargo run

# Or use the start script
./start.sh

# Check code quality
cargo clippy -- -D warnings
cargo fmt --check

# Tests
cargo test
```

---

## Code Style Rules

- **No `unwrap()` or `expect()` in production code** — always use `?` or handle explicitly
- **Log macros, not `println!`** — except `main.rs` for user-facing output
- **English comments** — all doc comments and inline comments must be in English
- **`Result<T>` alias** — always use `crate::error::Result<T>`, not `std::result::Result<T, CrawlerError>`
- **Explicit types** — no implicit inference on public API signatures
- **`derive` what you need** — `Debug`, `Clone`, `Serialize`, `Deserialize` on all data structs

See full guidelines:
- `ai/docs/code-style.md`
- `ai/docs/error-handling.md`
- `ai/docs/async-patterns.md`
- `ai/docs/testing-guidelines.md`
- `ai/docs/module-architecture.md`
