# Module Architecture — Obligation Crawler

## Dependency Direction

```
main.rs
  └── controllers/        (orchestration)
        └── services/     (business logic)
              └── models/ (data structs)
              └── api/    (DTO shapes)
              └── shared/ (pure utilities)
        └── config        (env vars)
        └── error         (error types)
        └── database      (pool factory)
```

**Rule:** Lower layers must NOT import from higher layers.

| Layer | Can import from |
|-------|----------------|
| `main.rs` | everything |
| `controllers` | `services`, `models`, `config`, `error` |
| `services` | `models`, `api`, `config`, `error`, `shared`, `database` |
| `models` | `error` (optional), `shared` |
| `api` | `models` |
| `shared` | nothing from `crate::` except `error` |
| `config` | `error` (ConfigError) |
| `error` | `config` (ConfigError), external crates |
| `database` | nothing (returns `sqlx::Error`) |

---

## Module Responsibilities

### `main.rs`
- Loads `.env` via `dotenv`
- Initializes `env_logger`
- Reads `RUN_MODE` and dispatches to `run_direct_mode()` or `run_consumer_mode()`
- High-level user-facing `println!` output

### `config.rs`
- `CrawlerConfig` struct — all application config
- `CrawlerConfig::from_env()` — reads env vars with defaults and validates
- `ConfigError` enum — missing vars, invalid values

### `error.rs`
- `CrawlerError` enum — all error variants for the application
- `Result<T>` type alias
- `From` impls for external error types

### `database.rs`
- `create_connection_pool() -> Result<PgPool>` — single factory function
- Connection pool config (max/min connections, timeouts)

### `models/`
- Pure data structs: `Bond`, `BondListItem`
- No async operations, no service dependencies
- CSV I/O methods directly on structs (acceptable because tightly coupled to data shape)
- RabbitMQ message structs

### `api/`
- HTTP response shapes: `BondsResponse`, `BondsApiResponse`
- These are DTO structs used for JSON serialization
- No business logic, only `impl` helpers for constructing responses

### `services/`
- `BondsCrawler` — owns the `WebDriver`, manages scraping lifecycle
- `RabbitMQProducer` — owns `Connection` + `Channel`, publishes messages
- `RabbitMQConsumer` — reconnect loop, consumes from queue
- `opencode_service` — calls `opencode` CLI subprocess, builds prompts

### `controllers/`
- Orchestration layer: calls multiple services in sequence
- Does NOT contain scraping logic — delegates to services
- Error handling at the orchestration level

### `shared/`
- Pure utility functions (no I/O, no async, no service deps)
- Date/time helpers: `now()`, `format_datetime()`, `parse_timestamp()`
- Text cleaning helpers (candidate for extraction from services)

---

## Adding a New Feature

### New data type

1. Add struct to `src/models/[name].rs`
2. Add `pub mod [name];` and `pub use [name]::*;` to `src/models/mod.rs`
3. Derive `Debug`, `Clone`, `Serialize`, `Deserialize` at minimum
4. Add fields as `Option<T>` if nullable

### New service

1. Create `src/services/[name].rs`
2. Add `pub mod [name]; pub use [name]::*;` to `src/services/mod.rs`
3. Follow the service lifecycle pattern (see `ai/docs/async-patterns.md`)
4. Import `crate::error::Result`, `crate::config::CrawlerConfig`

### New error variant

1. Open `src/error.rs`
2. Add variant to `CrawlerError` — use `#[from]` if the source type is unique
3. If `String`-based: `#[error("X: {0}")] XError(String)`

### New config option

1. Add field to `CrawlerConfig` in `src/config.rs`
2. Read in `from_env()` with `env::var("KEY").unwrap_or_else(|_| "default".to_string())`
3. Document in `.env.example`
4. Document in `ai/context.md` config table

---

## Anti-Patterns to Avoid

### Cross-layer imports

```rust
// ❌ BAD — services importing from controllers
use crate::controllers::bonds_crawler::SomeType;

// ❌ BAD — shared importing from services
use crate::services::bonds_crawler::BondsCrawler;
```

### Business logic in models

```rust
// ❌ BAD — scraping logic in BondListItem
impl BondListItem {
    pub async fn fetch_from_web(&mut self, driver: &WebDriver) -> Result<()> { ... }
}

// ✅ GOOD — scraping stays in services
impl BondsCrawler {
    pub async fn collect_bonds(&mut self) -> Result<Vec<BondListItem>> { ... }
}
```

### Config in services directly

```rust
// ❌ BAD — service reads env vars itself
pub async fn initialize(&mut self) {
    let url = env::var("TBANK_URL").unwrap();
    ...
}

// ✅ GOOD — config injected via CrawlerConfig
pub fn new(config: CrawlerConfig) -> Self { ... }
```

### Uninitialized state accessed without guard

```rust
// ❌ BAD
let driver = self.driver.as_ref().unwrap();

// ✅ GOOD
let driver = self.driver.as_ref().ok_or_else(|| {
    CrawlerError::SeleniumError("WebDriver not initialized".to_string())
})?;
```
