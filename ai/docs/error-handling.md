# Error Handling — Obligation Crawler

## Core Pattern

All fallible functions return `crate::error::Result<T>` (a type alias for `std::result::Result<T, CrawlerError>`).

```rust
// error.rs
pub type Result<T> = std::result::Result<T, CrawlerError>;
```

---

## `CrawlerError` Enum

Defined in `src/error.rs` using `thiserror`:

```rust
#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Crawler error: {0}")]
    CrawlerError(String),          // generic string error

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("RabbitMQ error: {0}")]
    RabbitMQError(#[from] lapin::Error),

    #[error("Selenium error: {0}")]
    SeleniumError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Ack error: {0}")]
    AckError(String),

    #[error("Config error: {0}")]
    ConfigError(#[from] ConfigError),

    #[error("WebDriver error: {0}")]
    WebDriverError(#[from] WebDriverError),
}
```

### Adding a new variant

1. Add the variant to the enum in `error.rs`
2. If the source type implements `std::error::Error`, use `#[from]` for automatic conversion
3. For `String`-based variants (no source type), use `CrawlerError::SomethingError(msg.to_string())`

```rust
// Adding a new CSV error variant:
#[error("CSV error: {0}")]
CsvError(String),
```

---

## Error Conversion Rules

### `#[from]` automatic conversion (use when source type is unique)

```rust
// ✅ Use ? directly — From impl is generated
let pool = PgPool::connect(&url).await?;  // sqlx::Error → CrawlerError::DatabaseError
let conn = Connection::connect(&url, ...).await?;  // lapin::Error → CrawlerError::RabbitMQError
```

### Manual `.map_err()` (use when source type conflicts or is `Box<dyn Error>`)

```rust
// ✅ GOOD
let output = Command::new("opencode")
    .output()
    .map_err(|e| CrawlerError::CrawlerError(format!("opencode exec failed: {}", e)))?;

// For serde_json — already implemented via manual From in error.rs
let json = serde_json::to_string(&bonds)?;  // works via impl From<serde_json::Error>
```

---

## Error Propagation Layers

| Layer | Rule |
|-------|------|
| `models/` | Return `Result<(), Box<dyn std::error::Error>>` for CSV methods (no service dep) |
| `services/` | Return `Result<T>` (`crate::error::Result`) — use `?` everywhere |
| `controllers/` | Return `Result<T>` — orchestrate service calls, log on error |
| `main.rs` | Top-level handler — `eprintln!` errors, return `Result<(), CrawlerError>` |

---

## When to Log vs Propagate

```rust
// ✅ GOOD — log non-fatal errors and continue
match parse_bond_row(driver, &row).await {
    Ok(Some(bond)) => all_bonds.push(bond),
    Ok(None) => warn!("Row {} returned None, skipping", idx),
    Err(e) => error!("Failed to parse row {}: {:?}", idx, e),
}

// ✅ GOOD — propagate fatal errors
let pool = create_connection_pool().await?;

// ❌ BAD — silently swallow errors
let _ = driver.quit().await;
```

**Rule of thumb:** If the operation failing means the program cannot continue → propagate. If it means one item is skipped → log and continue.

---

## Error in `impl Drop`

Drop cannot be async. Use `tokio::spawn` to fire-and-forget cleanup:

```rust
impl Drop for BondsCrawler {
    fn drop(&mut self) {
        if let Some(driver) = self.driver.take() {
            tokio::spawn(async move {
                let _ = driver.quit().await;
            });
        }
    }
}
```

---

## Testing Errors

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_error_missing_var() {
        // Use unwrap() freely in tests
        let err = CrawlerConfig::from_env();
        // assertions...
    }

    #[tokio::test]
    async fn service_returns_error_on_invalid_url() {
        let result = RabbitMQProducer::new("invalid://url".to_string(), "x".to_string()).await;
        assert!(result.is_err());
        // Check variant:
        assert!(matches!(result.unwrap_err(), CrawlerError::RabbitMQError(_)));
    }
}
```
