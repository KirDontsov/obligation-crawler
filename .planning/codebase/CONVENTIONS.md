# Coding Conventions

**Analysis Date:** 2026-06-09

This project is a Rust async crawler for T-Bank bond listings. Conventions below are
enforced by `CLAUDE.md` (Critical Rules) and documented in detail under `ai/docs/`.
Reference docs:
- `ai/docs/code-style.md` — naming, logging, struct design, comments, imports, DRY
- `ai/docs/error-handling.md` — `CrawlerError` / `Result<T>` patterns
- `ai/docs/async-patterns.md` — Tokio, WebDriver (thirtyfour), RabbitMQ (lapin), sqlx
- `ai/docs/module-architecture.md` — layer dependency rules and anti-patterns

## Naming Patterns

**Files / Modules:** `snake_case`
- Examples: `bonds_crawler.rs`, `rabbitmq_producer.rs`, `opencode_service.rs`
- Service files live under `src/services/`, models under `src/models/`,
  controllers under `src/controllers/`, repositories under `src/repository/`.

**Functions / methods:** `snake_case`
- Examples: `parse_bond_row`, `collect_bonds`, `run_crawl_loop`, `from_env`, `publish_bonds_data`
- Async functions use the same casing: `async fn collect_bonds(...)`.

**Variables:** `snake_case`
- Examples: `bond_ticker`, `yield_to_maturity`, `poll_interval_seconds`, `rabbitmq_url`.

**Types (Structs / Enums / Traits):** `PascalCase`
- Examples: `BondListItem`, `Bond`, `CrawlerConfig`, `CrawlerError`, `ConfigError`,
  `RabbitMQProducer`, `BondsCrawler`.

**Constants:** `SCREAMING_SNAKE_CASE`
- Examples: `MAX_RETRIES`, `DEFAULT_TIMEOUT`, `WEBDRIVER_URL`.

**Type aliases:** `PascalCase`
- The project-wide alias is `Result<T>` (defined in `src/error.rs`).

## Code Style

**Formatting:**
- Tool: `cargo fmt` (rustfmt), config in `rustfmt.toml`.
- Key setting: `hard_tabs=true` — indentation uses TAB characters, not spaces.
  All `src/` files are tab-indented; match this when adding code.
- Edition: 2021 (`Cargo.toml`).

**Linting:**
- Tool: `cargo clippy`.
- Enforced with `-D warnings` (clippy warnings are treated as errors). See Validation Commands.

## Import Organization

Group imports in three blocks, separated by a blank line:

1. `std` imports
2. External crate imports (`log`, `tokio`, `serde`, `lapin`, `thirtyfour`, `sqlx`, ...)
3. `crate::` imports

```rust
use std::env;
use std::time::Duration;

use log::{info, error};
use tokio::time::sleep;

use crate::config::CrawlerConfig;
use crate::error::{CrawlerError, Result};
use crate::models::BondListItem;
```

**Path aliases:** None. Modules are referenced via `crate::` (no `#[path]` or
Cargo workspace aliasing). Module re-exports use `pub use [name]::*;` in each
`mod.rs` (e.g. `src/services/mod.rs`, `src/models/mod.rs`).

## Error Handling

**Use the `Result<T>` alias from `crate::error`** — never the raw long form.

```rust
// GOOD
use crate::error::Result;
pub async fn scrape(&mut self) -> Result<Vec<BondListItem>> { ... }

// BAD
pub async fn scrape(&mut self) -> std::result::Result<Vec<BondListItem>, CrawlerError> { ... }
```

`Result<T>` is defined in `src/error.rs` as
`pub type Result<T> = std::result::Result<T, CrawlerError>;`.

**No `unwrap()` / `expect()` in production code.** Always propagate with `?` or
handle explicitly. `unwrap()` / `expect()` are acceptable ONLY in:
- `#[cfg(test)]` blocks
- `main.rs` for non-fallible `println!` formatting
- `impl Drop` where the async context is unavailable

```rust
// BAD — panics in production
let driver = self.driver.as_ref().unwrap();

// GOOD — guard uninitialized Option state
let driver = self.driver.as_ref().ok_or_else(|| {
    CrawlerError::SeleniumError("WebDriver not initialized".to_string())
})?;
```

**`CrawlerError` variants** (`src/error.rs`):
`CrawlerError(String)`, `DatabaseError(#[from] sqlx::Error)`,
`RabbitMQError(#[from] lapin::Error)`, `SeleniumError(String)`, `ParseError(String)`,
`IoError(#[from] std::io::Error)`, `RequestError(#[from] reqwest::Error)`,
`AckError(String)`, `ConfigError(#[from] ConfigError)`,
`WebDriverError(#[from] WebDriverError)`. `serde_json::Error` maps to `ParseError`
via a manual `From` impl.

- Prefer `#[from]` automatic conversion when the source type is unique (use `?` directly).
- Use `.map_err(|e| CrawlerError::Variant(format!(...)))` when the source type is
  not uniquely mappable (e.g. subprocess/`Box<dyn Error>` cases).

**Log vs propagate (`src/error.rs` rule of thumb):**
- If failure means the program cannot continue → propagate with `?`.
- If failure means one item is skipped → `log` and continue.

```rust
match parse_bond_row(driver, &row).await {
    Ok(Some(bond)) => all_bonds.push(bond),
    Ok(None) => warn!("Row {} returned None, skipping", idx),
    Err(e) => error!("Failed to parse row {}: {:?}", idx, e),
}
```

**Error layering (`ai/docs/error-handling.md`):**

| Layer | Rule |
|-------|------|
| `models/` | CSV methods may return `Result<(), Box<dyn std::error::Error>>` (no service dep) |
| `services/` | Return `crate::error::Result<T>` — use `?` everywhere |
| `controllers/` | Return `Result<T>`, orchestrate services, log on error |
| `main.rs` | Top-level handler — returns `Result<(), CrawlerError>`, `eprintln!` for status |

## Logging

**Use `log` macros (`info!`, `warn!`, `error!`, `debug!`) in all modules except `main.rs`.**
`println!` is allowed only in `main.rs` for high-level, user-facing status.

```rust
use log::{info, warn, error, debug};

info!("Crawl loop started, max_pages={}", max_pages);
warn!("Row {} returned None, skipping", idx);
error!("Failed to connect to RabbitMQ: {}", e);
debug!("[parse_bond_row] cells.len()={}", cells.len());
```

`env_logger` is initialized once in `main.rs` via `env_logger::init()`.

> Drift note: some service files (`src/services/rabbitmq_producer.rs`,
> `src/models/bonds.rs`, `src/services/rabbitmq_consumer.rs`) still contain
> `println!` calls (e.g. `println!("✅ RabbitMQ producer initialized ...")`).
> These violate the rule and should be migrated to `log` macros when touched.

## Configuration Injection

**Never read env vars inside services.** All env reads happen in
`CrawlerConfig::from_env()` (`src/config.rs`) or in `main.rs` dispatch; config is
injected into services via the `CrawlerConfig` struct.

```rust
// BAD — service reads env directly
pub async fn initialize(&mut self) {
    let url = env::var("TBANK_URL").unwrap();
}

// GOOD — config injected
pub fn new(config: CrawlerConfig) -> Self { ... }
```

`CrawlerConfig` fields: `tbank_url`, `poll_interval_seconds`, `headless_chrome`,
`chrome_driver_path`, `wait_after_login_seconds`, `max_retries`. New config options
must be added to `CrawlerConfig`, read in `from_env()` with a default, and documented
in `.env.example` and `ai/context.md`.

## Comments

**All comments must be in English** — no Russian in code comments. (Russian string
literals for CSV headers and user-facing output are acceptable, e.g. `"Тикер"` in
`src/models/bonds.rs`.)

- Doc comments (`///`) on public API.
- Inline comments only for non-obvious logic.

## Struct Design

- **Always derive `Debug`.** Add `Clone`, `Serialize`, `Deserialize` only when needed.
  - Data models: `#[derive(Debug, Clone, Serialize, Deserialize)]` (e.g. `Bond`,
    `BondListItem` in `src/models/bonds.rs`).
  - Config structs: `#[derive(Debug, Clone)]` (no `Serialize`) — e.g. `CrawlerConfig`.
- **Use `Option<T>` for nullable fields**, never empty strings as null sentinels.
  `BondListItem` uses `Option<f64>` / `Option<String>` / `Option<i32>` for every
  field that may be absent.
- `serde` derive feature is enabled (`serde = { features = ["derive"] }`).

> Drift note: `ai/docs/testing-guidelines.md` examples assume `BondListItem` derives
> `Default` (`..Default::default()`). The current struct in `src/models/bonds.rs`
> does NOT derive `Default`. Tests must construct all fields explicitly (see the
> existing test in `src/repository/bonds_repository.rs`) or `Default` must be added.

## Module Design

**Layer dependency rule (`ai/docs/module-architecture.md`):** lower layers must NOT
import from higher layers.

```
main.rs → controllers → services → models / api / shared
                      → config, error, database
```

| Layer | Can import from |
|-------|----------------|
| `main.rs` | everything |
| `controllers` | `services`, `models`, `config`, `error` |
| `services` | `models`, `api`, `config`, `error`, `shared`, `database` |
| `models` | `error` (optional), `shared` |
| `api` | `models` |
| `shared` | nothing from `crate::` except `error` |
| `config` | `error` (`ConfigError`) |
| `error` | `config` (`ConfigError`), external crates |
| `database` | nothing |

**Forbidden cross-layer imports:**
- Services importing from `controllers`.
- `shared` importing from `services`.
- Models containing business/scraping logic (keep scraping in `services`).

**Exports / barrel files:** Each `mod.rs` declares `pub mod [name];` and re-exports
with `pub use [name]::*;`.

## Magic Values & DRY

- Extract magic numbers / URLs into named `const` (e.g. `MAX_PAGES`, `WEBDRIVER_URL`).
- A text-cleaning pattern repeated 3+ times belongs in `src/shared/utils.rs`.
- An env-var-with-default read repeated 2+ times belongs in `src/config.rs`.
- Repeated `CrawlerError` construction belongs in a helper method on `CrawlerError`.

---

*Convention analysis: 2026-06-09*
