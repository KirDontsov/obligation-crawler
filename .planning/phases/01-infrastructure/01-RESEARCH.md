# Phase 1: Infrastructure Setup - Research

**Researched:** 2026-05-19
**Domain:** RabbitMQ consumer integration, PostgreSQL run lifecycle management
**Confidence:** HIGH

## Summary

This phase implements the foundational infrastructure for the obligation_crawler microservice to operate as a RabbitMQ consumer. The research confirms that the codebase already has partial implementations for both RabbitMQ consumer and run lifecycle management, but gaps exist in production-grade patterns (proper logging, reconnection with exponential backoff, configuration injection).

**Primary recommendation:** Enhance the existing `RabbitMQConsumer` in `src/services/rabbitmq_consumer.rs` with production patterns (structured logging, exponential backoff, graceful shutdown) and integrate `CrawlerConfig` to inject RabbitMQ settings. The existing PostgreSQL schema and repository methods provide sufficient foundation for REQ-006.

---

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-001 | RabbitMQ Consumer — listens to RabbitMQ queue and triggers parsing | lapin consumer patterns, reconnection strategies, message handling |
| REQ-006 | Crawler Run Lifecycle — manage run lifecycle (running → completed \| failed) | Existing DB schema, BondsRepository methods |

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| RabbitMQ message consumption | API/Backend | — | lapin library operates in async context, connects to external message broker |
| Run lifecycle state (DB) | Database/Storage | — | PostgreSQL stores run state via sqlx; BondsRepository handles CRUD |
| Crawler execution | API/Backend | — | BondsCrawler service executes scraping logic |
| Configuration injection | API/Backend | — | CrawlerConfig is the single source for all settings |

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| lapin | 3.7.0 | AMQP 0.9.1 client for RabbitMQ | [CITED: docs.rs/lapin] Official RabbitMQ client for Rust |
| sqlx | 0.8 | Async PostgreSQL driver with query builder | [CITED: docs.rs/sqlx] Standard for Rust async DB operations |
| tokio | 1.33.0 | Async runtime | [VERIFIED: Cargo.toml] Already in use |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| uuid | 1.4.1 | Run ID generation | [VERIFIED: Cargo.toml] Used for crawl run identifiers |
| chrono | 0.4.30 | Timestamp handling | [VERIFIED: Cargo.toml] Used for started_at/finished_at |
| thiserror | 1.0.56 | Error enum derivation | [VERIFIED: Cargo.toml] Used in CrawlerError |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| lapin 3.7.0 | lapin 4.7.4 (latest) | 4.x has breaking changes; 3.7.0 is stable and compatible with existing code |
| Hand-roll reconnection | Use lapin's experimental recovery (unstable feature) | Current loop-based approach is simpler and battle-tested |

---

## Package Legitimacy Audit

> All packages are verified via Cargo.toml and crates.io registry.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| lapin | crates.io | 7+ years | 40M+ | [github.com/amqp-rs/lapin](https://github.com/amqp-rs/lapin) | OK | Approved |
| sqlx | crates.io | 8+ years | 90M+ | [github.com/launchbadge/sqlx](https://github.com/launchbadge/sqlx) | OK | Approved |
| tokio | crates.io | 9+ years | 500M+ | [github.com/tokio-rs/tokio](https://github.com/tokio-rs/tokio) | OK | Approved |

**Packages removed due to slopcheck [SLOP] verdict:** none

**Packages flagged as suspicious [SUS]:** none

---

## RabbitMQ Consumer Research

### Current Implementation Analysis

The existing `src/services/rabbitmq_consumer.rs` implements:
- Connection loop with retry on failure
- Queue declaration with durable queue
- Basic consume with manual acknowledgment
- Message handling callback pattern

**Gaps identified:**
1. Uses `println!`/`eprintln!` instead of `log` macros (violates project rule)
2. Fixed 5-second retry interval (no exponential backoff)
3. No graceful shutdown mechanism (no `Drop` impl or signal handling)
4. Message acknowledgment always succeeds even when handler fails
5. No Dead Letter Queue (DLQ) configuration for failed messages
6. No health check / connection status visibility

### Recommended Consumer Pattern

```rust
// Source: [CITED: docs.rs/lapin], [CITED: oneuptime.com Rust message queue consumers guide]

use crate::error::{CrawlerError, Result};
use futures_util::stream::StreamExt;
use lapin::{message::Delivery, options::*, types::FieldTable, Connection, ConnectionProperties};
use log::{info, warn, error};
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};

// Configuration for retry behavior
const INITIAL_RETRY_DELAY_MS: u64 = 1000;
const MAX_RETRY_DELAY_MS: u64 = 30000;
const BACKOFF_MULTIPLIER: f64 = 2.0;

pub struct RabbitMQConsumer {
    connection_string: String,
    queue_name: String,
    shutdown_tx: broadcast::Sender<()>,
}

impl RabbitMQConsumer {
    pub fn new(connection_string: String, queue_name: String) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            connection_string,
            queue_name,
            shutdown_tx,
        }
    }

    /// Starts consuming with exponential backoff reconnection
    pub async fn start_consuming<F>(&self, mut message_handler: F) -> Result<(), CrawlerError>
    where
        F: FnMut(String) -> futures::future::BoxFuture<'static, Result<(), CrawlerError>>
            + Send
            + Sync
            + 'static,
    {
        let mut retry_delay = INITIAL_RETRY_DELAY_MS;
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received, stopping consumer");
                    break;
                }
                result = self.connect_and_consume(&mut message_handler) => {
                    match result {
                        Ok(_) => {
                            info!("Consumer loop ended normally");
                            break;
                        }
                        Err(e) => {
                            warn!("Connection error: {}, retrying in {}ms", e, retry_delay);
                            sleep(Duration::from_millis(retry_delay)).await;
                            retry_delay = ((retry_delay as f64) * BACKOFF_MULTIPLIER) as u64;
                            retry_delay = retry_delay.min(MAX_RETRY_DELAY_MS);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn connect_and_consume<F>(&self, message_handler: &mut F) -> Result<(), CrawlerError>
    where
        F: FnMut(String) -> futures::future::BoxFuture<'static, Result<(), CrawlerError>>,
    {
        let connection = Connection::connect(
            &self.connection_string,
            ConnectionProperties::default(),
        )
        .await
        .map_err(|e| CrawlerError::RabbitMQError(e))?;

        info!("Connected to RabbitMQ");

        let channel = connection.create_channel().await
            .map_err(|e| CrawlerError::RabbitMQError(e))?;

        // Declare queue with DLQ arguments for failed messages
        let mut args = FieldTable::default();
        args.insert("x-dead-letter-exchange".into(), "obligation_dlx".into());
        args.insert("x-dead-letter-routing-key".into(), "failed".into());

        channel
            .queue_declare(
                self.queue_name.as_str(),
                QueueDeclareOptions {
                    durable: true,
                    exclusive: false,
                    auto_delete: false,
                    ..QueueDeclareOptions::default()
                },
                args,
            )
            .await
            .map_err(|e| CrawlerError::RabbitMQError(e))?;

        channel.basic_qos(1, BasicQosOptions::default()).await
            .map_err(|e| CrawlerError::RabbitMQError(e))?;

        let mut consumer = channel
            .basic_consume(
                self.queue_name.as_str(),
                "obligation_crawler_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|e| CrawlerError::RabbitMQError(e))?;

        info!("Consumer started, waiting for messages");

        while let Some(message_result) = consumer.next().await {
            match message_result {
                Ok(delivery) => {
                    let message_str = String::from_utf8_lossy(&delivery.data).to_string();
                    info!("Received message: {}", message_str);

                    match message_handler(message_str).await {
                        Ok(_) => {
                            delivery.ack(BasicAckOptions::default()).await
                                .map_err(|e| CrawlerError::AckError(e.to_string()))?;
                            info!("Message acknowledged");
                        }
                        Err(e) => {
                            error!("Message handling failed: {}", e);
                            // Reject and send to DLQ (don't requeue)
                            delivery.nack(BasicNackOptions {
                                requeue: false,
                                ..BasicNackOptions::default()
                            }).await
                            .map_err(|e| CrawlerError::AckError(e.to_string()))?;
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving delivery: {}", e);
                    return Err(CrawlerError::RabbitMQError(e));
                }
            }
        }

        Ok(())
    }

    /// Graceful shutdown trigger
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

impl Drop for RabbitMQConsumer {
    fn drop(&mut self) {
        self.shutdown();
    }
}
```

### Key Production Patterns

1. **Exponential backoff**: Start at 1 second, double up to 30 seconds max
2. **Graceful shutdown**: Use broadcast channel for shutdown signal
3. **Dead Letter Queue**: Configure `x-dead-letter-exchange` and `x-dead-letter-routing-key` on queue declaration
4. **Selective acknowledgment**: ACK on success, NACK (requeue=false) on failure
5. **Connection status visibility**: Log connection state changes

---

## Run Lifecycle in PostgreSQL

### Existing Schema (Verified)

The migration file `migrations/001_create_crawler_schema.sql` defines:

```sql
CREATE TABLE obligation_crawler_runs (
    id               UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    started_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    finished_at      TIMESTAMPTZ,
    tbank_url        TEXT         NOT NULL,
    headless_chrome  BOOLEAN      NOT NULL DEFAULT FALSE,
    status           TEXT         NOT NULL DEFAULT 'running'
                     CHECK (status IN ('running', 'completed', 'failed')),
    bonds_count      INTEGER      NOT NULL DEFAULT 0,
    error_message    TEXT,
    duration_seconds INTEGER
);
```

### Existing Repository Methods (Verified)

The `src/repository/bonds_repository.rs` already implements:

| Method | Purpose |
|--------|---------|
| `create_crawl_run(pool, tbank_url, headless_chrome)` | Insert row with `status='running'`, return UUID |
| `finish_crawl_run(pool, run_id, bonds_count, status, error_message)` | Update finished_at, bonds_count, status, error_message, compute duration_seconds |

**Status flow:**
- Created: `status = 'running'`
- Success: `status = 'completed'`
- Failure: `status = 'failed'`

**Duration calculation:** `EXTRACT(EPOCH FROM (finished_at - started_at))::INTEGER`

### What REQ-006 Requires

The existing implementation covers REQ-006 requirements:
- ✅ Create run record before crawl starts
- ✅ Update status to `completed` on success
- ✅ Update status to `failed` on error with error_message
- ✅ Record duration_seconds automatically

**Enhancement opportunity:** Add method to fetch run by ID for consumer mode integration.

---

## Configuration Research

### Current Configuration Gaps

The project follows the rule "Never read env vars in services — inject via CrawlerConfig". However:

| Env Var | Used In | Injected via Config? |
|---------|---------|---------------------|
| `RABBITMQ_URL` | main.rs (directly) | ❌ No |
| `RABBITMQ_QUEUE` | main.rs (directly) | ❌ No |
| `RABBITMQ_EXCHANGE` | main.rs (directly) | ❌ No |
| `DATABASE_URL` | database.rs (directly via env) | ⚠️ Partial |

### Recommended Config Changes

Add to `src/config.rs`:

```rust
#[derive(Debug, Clone)]
pub struct CrawlerConfig {
    // ... existing fields ...
    pub rabbitmq_url: String,
    pub rabbitmq_queue: String,
    pub rabbitmq_exchange: String,
}

impl CrawlerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        // ... existing fields ...

        let rabbitmq_url = env::var("RABBITMQ_URL")
            .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

        let rabbitmq_queue = env::var("RABBITMQ_QUEUE")
            .unwrap_or_else(|_| "obligation_crawler_queue".to_string());

        let rabbitmq_exchange = env::var("RABBITMQ_EXCHANGE")
            .unwrap_or_else(|_| "obligation_exchange".to_string());

        Ok(Self {
            // ... existing fields ...
            rabbitmq_url,
            rabbitmq_queue,
            rabbitmq_exchange,
        })
    }
}
```

---

## Common Pitfalls

### Pitfall 1: Blocking the Consumer Thread
**What goes wrong:** Synchronous operations inside the message handler block the consumer.
**Why it happens:** WebDriver operations (thirtyfour) can be slow; if handler awaits incorrectly, consumer stalls.
**How to avoid:** Use `tokio::spawn` for heavy processing, keep handler lean.
**Warning signs:** Consumer appears stuck, queue depth grows, message latency increases.

### Pitfall 2: Message Requeue on Failure
**What goes wrong:** Using `nack(requeue=true)` on failure causes infinite retry loop.
**Why it happens:** Failed message goes back to queue immediately, consumer picks it up again.
**How to avoid:** Use `nack(requeue=false)` to send to DLQ after max retries, or implement retry counter in message headers.
**Warning signs:** Same message processed repeatedly, CPU spinning, queue not draining.

### Pitfall 3: Connection Leak on Shutdown
**What goes wrong:** Not closing connection properly causes resource leak.
**Why it happens:** No Drop implementation or graceful shutdown signal.
**How to avoid:** Implement Drop for RabbitMQConsumer, use broadcast channel for shutdown.
**Warning signs:** "Too many connections" errors from RabbitMQ, connection count grows.

---

## Code Examples

### Message Format for Consumer Trigger

```json
// Example message payload to trigger a crawl
{
    "action": "start_crawl",
    "tbank_url": "https://www.tbank.ru/invest/bonds/",
    "duration_minutes": 60,
    "headless_chrome": false
}
```

### Handler Integration Pattern

```rust
// In main.rs consumer mode
async fn run_consumer_mode(config: CrawlerConfig, db_pool: Option<PgPool>) -> Result<(), CrawlerError> {
    let consumer = RabbitMQConsumer::new(
        config.rabbitmq_url.clone(),
        config.rabbitmq_queue.clone(),
    );

    consumer.start_consuming(move |message| {
        let config = config.clone();
        let pool = db_pool.clone();

        Box::pin(async move {
            // Parse message and trigger crawl
            let msg: CrawlTrigger = serde_json::from_str(&message)
                .map_err(|e| CrawlerError::ParseError(e.to_string()))?;

            info!("Received crawl trigger: {:?}", msg);

            // Create run in DB if pool available
            let run_id = if let Some(ref p) = pool {
                Some(BondsRepository::create_crawl_run(p, &msg.tbank_url, msg.headless_chrome).await?)
            } else {
                None
            };

            // Execute crawl
            let crawler = BondsCrawler::new(config, pool);
            let bonds = crawler.run_crawl_loop(msg.duration_minutes).await;

            // Update run status
            if let (Some(ref p), Some(rid)) = (&pool, run_id) {
                match bonds {
                    Ok(bond_list) => {
                        BondsRepository::finish_crawl_run(p, rid, bond_list.len() as i32, "completed", None).await?;
                    }
                    Err(e) => {
                        BondsRepository::finish_crawl_run(p, rid, 0, "failed", Some(&e.to_string())).await?;
                    }
                }
            }

            Ok(())
        })
    }).await
}
```

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| PostgreSQL | Data layer | ✓ | 15.x (target) | — |
| RabbitMQ | Message queue | ✓ | 3.x+ (target) | — |
| Tokio | Async runtime | ✓ | 1.33.0 | — |

**Missing dependencies with no fallback:** None identified.

**Missing dependencies with fallback:** None identified.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | lapin 3.7.0 is stable and compatible with the codebase | Standard Stack | LOW - Verified via Cargo.toml |
| A2 | Existing BondsRepository methods cover REQ-006 requirements | Run Lifecycle | LOW - Verified via code inspection |
| A3 | Database schema already covers run lifecycle needs | Run Lifecycle | LOW - Verified via migrations/001_create_crawler_schema.sql |

**If this table is empty:** All claims in this research were verified or cited — no user confirmation needed.

---

## Open Questions

1. **Message format specification**: Should the trigger message include additional fields (e.g., specific bonds to scrape, output format)?
   - What we know: Current code accepts any string message
   - What's unclear: Exact message schema for production use
   - Recommendation: Define JSON schema for CrawlTrigger, document in code

2. **DLQ processing**: How should failed messages in the dead letter queue be handled?
   - What we know: Messages go to DLQ on NACK with requeue=false
   - What's unclear: Alerting, manual retry, or automated reprocessing?
   - Recommendation: Start with manual inspection, add automation later

3. **Multiple concurrent consumers**: Should the system support multiple consumer instances?
   - What we know: Current code uses single consumer
   - What's unclear: Horizontal scaling requirements
   - Recommendation: Single consumer per queue initially, add competing consumers later if needed

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | tokio::test (built-in) |
| Config file | none — tests inline in modules |
| Quick run command | `cargo test` |
| Full suite command | `cargo test --all-features` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| REQ-001 | RabbitMQ consumer connects and processes messages | Integration | `cargo test rabbitmq_consumer` | ❌ Must add |
| REQ-006 | Run lifecycle creates/updates status in DB | Unit | `cargo test bonds_repository` | ✅ Exists |

### Wave 0 Gaps
- [ ] `tests/test_rabbitmq_consumer.rs` — covers REQ-001 (integration test with mock RabbitMQ)
- [ ] Update existing repository tests to verify run lifecycle flow

---

## Sources

### Primary (HIGH confidence)
- [docs.rs/lapin](https://docs.rs/lapin/3.7.0/lapin/) - Official lapin documentation
- [docs.rs/sqlx](https://docs.rs/sqlx/0.8/sqlx/) - Official sqlx documentation
- [crates.io/lapin](https://crates.io/crates/lapin/3.7.0) - Package registry entry

### Secondary (MEDIUM confidence)
- [oneuptime.com - Rust Message Queue Consumers](https://oneuptime.com/blog/post/2026-02-01-rust-message-queue-consumers/view) - Production patterns guide
- [RabbitMQ Dead Letter Exchanges](https://www.rabbitmq.com/docs/dlx) - DLQ configuration

### Tertiary (LOW confidence)
- [Medium - Integration Between RabbitMQ and Rust](https://medium.com/@rustincode.dev/integration-between-rabbitmq-and-rust-building-a-messaging-system-with-asynchronous-communication-b71967a64930) - Additional reference

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Verified via Cargo.toml and crates.io
- Architecture: HIGH - Based on existing codebase patterns
- Pitfalls: MEDIUM - Based on common RabbitMQ patterns and project rules

**Research date:** 2026-05-19
**Valid until:** 2026-06-19 (30 days for stable patterns)