# Code Style — Obligation Crawler

## Naming Conventions

| Element | Convention | Example |
|---------|-----------|---------|
| Functions / methods | `snake_case` | `parse_bond_row`, `collect_bonds` |
| Variables | `snake_case` | `bond_ticker`, `yield_to_maturity` |
| Structs / Enums | `PascalCase` | `BondListItem`, `CrawlerError` |
| Traits | `PascalCase` | `BondScraper` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_RETRIES`, `DEFAULT_TIMEOUT` |
| Modules / files | `snake_case` | `bonds_crawler.rs`, `rabbitmq_producer.rs` |
| Type aliases | `PascalCase` | `Result<T>` |
| Async fns | same as regular | `async fn collect_bonds(...)` |

---

## Error Handling

### Always use `?` operator, never `unwrap()` / `expect()` in production

```rust
// ❌ BAD — panics in production
let driver = self.driver.as_ref().unwrap();
let value = env::var("KEY").expect("KEY not set");

// ✅ GOOD — propagates error
let driver = self.driver.as_ref().ok_or_else(|| {
    CrawlerError::SeleniumError("WebDriver not initialized".to_string())
})?;
let value = env::var("KEY").map_err(|_| ConfigError::MissingEnvVar("KEY".to_string()))?;
```

`unwrap()` is acceptable only in:
- `#[cfg(test)]` blocks
- `main.rs` `println!` formatting (non-fallible operations)
- `impl Drop` where async context is unavailable

### Use the `Result<T>` alias from `crate::error`

```rust
// ✅ GOOD
use crate::error::Result;
pub async fn scrape(&mut self) -> Result<Vec<BondListItem>> { ... }

// ❌ BAD — verbose, inconsistent
pub async fn scrape(&mut self) -> std::result::Result<Vec<BondListItem>, CrawlerError> { ... }
```

---

## Logging

### Use `log` macros — NOT `println!` in non-main modules

```rust
use log::{info, warn, error, debug};

// ✅ GOOD
info!("Crawl loop started, max_pages={}", max_pages);
warn!("Row {} returned None, skipping", idx);
error!("Failed to connect to RabbitMQ: {}", e);
debug!("[parse_bond_row] cells.len()={}", cells.len());

// ❌ BAD (in services/models/controllers)
println!("[DEBUG] На странице {} найдено {} строк", page_num, rows.len());
println!("✅ RabbitMQ producer initialized");
```

`println!` is acceptable only in `main.rs` for high-level user-facing status.

---

## Struct Design

### Derive minimal required traits, add Debug always

```rust
// ✅ GOOD — Debug always, Clone/Serialize only when needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondListItem { ... }

// Config structs: Debug + Clone (no Serialize needed)
#[derive(Debug, Clone)]
pub struct CrawlerConfig { ... }

// ❌ BAD — no Debug
pub struct BondDetails { ... }
```

### Use `Option<T>` for nullable fields, not empty strings

```rust
// ✅ GOOD
pub coupon_type: Option<String>,
pub yield_to_maturity: Option<f64>,

// ❌ BAD
pub coupon_type: String,  // empty string as null
```

---

## Comments

### All comments must be in English

```rust
// ✅ GOOD
// Find the row again by index to avoid stale element references
let row = table_body.find_all(...).await?;

// ❌ BAD
// Находим строку заново по индексу
```

### Doc comments on public API

```rust
/// Creates a new CSV file with headers at the given path.
/// Creates the output directory if it does not exist.
pub fn create_csv_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> { ... }
```

### Inline comments only for non-obvious logic

```rust
// Skip analysis if maturity < 1 year or price > nominal + 5₽
let skip_analysis = ...;
```

---

## Async Patterns

### Prefer `tokio::time::sleep` over `std::thread::sleep`

```rust
use tokio::time::{sleep, Duration};

// ✅ GOOD — does not block the thread
sleep(Duration::from_secs(2)).await;

// ❌ BAD — blocks the entire thread
std::thread::sleep(std::time::Duration::from_secs(2));
```

### Always `.await` fallible operations and propagate with `?`

```rust
// ✅ GOOD
let elements = driver.find_all(By::Css("tr")).await?;

// ❌ BAD — silently ignores errors
let _ = driver.find_all(By::Css("tr")).await;
```

---

## Imports

### Group imports: std → external → crate

```rust
use std::env;
use std::time::Duration;

use log::{info, error};
use tokio::time::sleep;

use crate::config::CrawlerConfig;
use crate::error::{CrawlerError, Result};
use crate::models::BondListItem;
```

---

## Magic Values

### Extract magic numbers and strings into named constants

```rust
// ✅ GOOD
const MAX_PAGES: u32 = 50;
const WEBDRIVER_URL: &str = "http://localhost:9515";

// ❌ BAD
for page_num in 0..50 { ... }
WebDriver::new("http://localhost:9515", caps).await?;
```

---

## DRY

- If a text-cleaning pattern (replace + trim) appears 3+ times, extract it to `shared/utils.rs`
- If an env var read with default appears 2+ times, it belongs in `config.rs`
- If error construction (`CrawlerError::SeleniumError(format!(...))`) repeats, add a helper method to `CrawlerError`
