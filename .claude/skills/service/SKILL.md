---
name: service
description: >-
  Create a new Rust service module following project patterns: struct with CrawlerConfig,
  async lifecycle methods (new/initialize/run/close), CrawlerError error handling,
  log macros, Drop impl for cleanup. Part of the IMPLEMENT phase after /plan. Trigger
  when the user needs to create a new service, scraper, integration, or background worker.
  Trigger on phrases like "create service for X", "add X service", "implement X integration",
  "new crawler for X". Does NOT create models or tests (use /model and /test for those).
user-invocable: true
argument-hint: "[ServiceName]"
model: sonnet
---

# Service Template

**Service:** $ARGUMENTS

## Context
- Branch: !`git branch --show-current`
- Existing services: !`ls src/services/ 2>/dev/null`

---

## Task

Create `src/services/[service_name].rs` for **$ARGUMENTS**.

Then add to `src/services/mod.rs`:
```rust
pub mod [service_name];
pub use [service_name]::*;
```

---

## Requirements

1. Accept `CrawlerConfig` in constructor — never read env vars directly in the service
2. Use `crate::error::Result<T>` for all fallible methods
3. Use `log` macros (`info!`, `warn!`, `error!`, `debug!`) — not `println!`
4. Implement `Drop` for async resource cleanup (use `tokio::spawn`)
5. Store stateful resources as `Option<T>` — `None` until initialized
6. All production code must use `?` — never `unwrap()`/`expect()`

---

## Pattern A — Stateful Service (owns a connection/driver)

```rust
use crate::config::CrawlerConfig;
use crate::error::{CrawlerError, Result};
use log::{info, warn, error, debug};
use tokio::time::{sleep, Duration};

pub struct [ServiceName] {
    config: CrawlerConfig,
    connection: Option<SomeConnection>,
}

impl [ServiceName] {
    pub fn new(config: CrawlerConfig) -> Self {
        Self {
            config,
            connection: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing [ServiceName]...");
        let conn = SomeConnection::connect(&self.config.some_url).await
            .map_err(|e| CrawlerError::CrawlerError(format!("Connect failed: {}", e)))?;
        self.connection = Some(conn);
        info!("[ServiceName] initialized");
        Ok(())
    }

    pub async fn run(&mut self) -> Result<OutputType> {
        let conn = self.connection.as_mut().ok_or_else(|| {
            CrawlerError::CrawlerError("[ServiceName] not initialized".to_string())
        })?;

        // implementation
        info!("[run] processing...");
        Ok(result)
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(conn) = self.connection.take() {
            conn.close().await?;
            info!("[ServiceName] closed");
        }
        Ok(())
    }
}

impl Drop for [ServiceName] {
    fn drop(&mut self) {
        if let Some(conn) = self.connection.take() {
            tokio::spawn(async move {
                let _ = conn.close().await;
            });
        }
    }
}
```

---

## Pattern B — Stateless Service (no persistent connection)

```rust
use crate::config::CrawlerConfig;
use crate::error::{CrawlerError, Result};
use crate::models::SomeModel;
use log::{info, error};

pub struct [ServiceName] {
    config: CrawlerConfig,
}

impl [ServiceName] {
    pub fn new(config: CrawlerConfig) -> Self {
        Self { config }
    }

    pub async fn process(&self, input: &InputType) -> Result<OutputType> {
        info!("[process] input={:?}", input);
        // implementation using self.config fields
        Ok(result)
    }
}
```

---

## Pattern C — Consumer Service (RabbitMQ-style infinite loop)

```rust
use crate::error::{CrawlerError, Result};
use futures::future::BoxFuture;
use log::{info, error};
use tokio::time::{sleep, Duration};

pub struct [ServiceName] {
    connection_string: String,
    queue_name: String,
}

impl [ServiceName] {
    pub fn new(connection_string: String, queue_name: String) -> Self {
        Self { connection_string, queue_name }
    }

    pub async fn start<F>(&self, mut handler: F) -> Result<()>
    where
        F: FnMut(String) -> BoxFuture<'static, Result<()>> + Send + Sync + 'static,
    {
        loop {
            info!("Connecting to {} ...", self.connection_string);

            let connection = match connect(&self.connection_string).await {
                Ok(c) => c,
                Err(e) => {
                    error!("Connection failed: {}. Retrying in 5s...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            // consume messages...
            // reconnect on error
        }
    }
}
```

---

## Checklist

- [ ] No env var reads inside the service — config injected via `CrawlerConfig`
- [ ] All `Result`-returning methods use `?`, no `unwrap()`
- [ ] `log` macros used, not `println!`
- [ ] `Drop` implemented if the service owns an async resource
- [ ] `Option<T>` for stateful resources
- [ ] Added to `src/services/mod.rs`
- [ ] Tests in `#[cfg(test)]` module at bottom of the file

## Validate

```bash
cargo build 2>&1 | head -30
cargo clippy -- -D warnings 2>&1 | head -20
```
