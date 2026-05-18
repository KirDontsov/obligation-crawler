# Architecture

**Analysis Date:** 2026-05-19

## System Overview

```text
┌─────────────────────────────────────────────────────────────┐
│                      main.rs (Entry)                         │
├──────────────────┬──────────────────┬───────────────────────┤
│  run_direct_mode │  run_consumer_mode │                     │
│  (crawl + queue) │  (RabbitMQ listener) │                   │
└────────┬─────────┴────────┬─────────┴──────────┬────────────┘
         │                  │                     │
         ▼                  ▼                     ▼
┌─────────────────────────────────────────────────────────────┐
│                    Services Layer                            │
│         `[src/services/]`                                    │
│  bonds_crawler.rs | rabbitmq_producer.rs | rabbitmq_consumer.rs |
│  opencode_service.rs                                        │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│                  Repository Layer                            │
│         `[src/repository/]`                                  │
│  bonds_repository.rs (PostgreSQL)                           │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│                    Models Layer                              │
│         `[src/models/]`                                      │
│  bonds.rs | rabbitmq.rs                                     │
└─────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| BondsCrawler | WebDriver scraping, bond data extraction | `src/services/bonds_crawler.rs` |
| RabbitMQProducer | Publish bond data to RabbitMQ | `src/services/rabbitmq_producer.rs` |
| RabbitMQConsumer | Listen and consume from RabbitMQ queue | `src/services/rabbitmq_consumer.rs` |
| OpencodeService | AI analysis via opencode CLI | `src/services/opencode_service.rs` |
| BondsRepository | PostgreSQL persistence | `src/repository/bonds_repository.rs` |
| CrawlerConfig | Configuration from environment | `src/config.rs` |
| CrawlerError | Error types and Result alias | `src/error.rs` |
| Bond/BondListItem | Data models with CSV I/O | `src/models/bonds.rs` |

## Pattern Overview

**Overall:** Async Tokio-based service layer with pluggable outputs

**Key Characteristics:**
- Async-first architecture using Tokio runtime
- Config injection via CrawlerConfig struct
- Error handling via thiserror-derived CrawlerError enum
- Module layer separation: controllers → services → repository → models
- Optional dependencies (DB, RabbitMQ) with graceful fallback

## Layers

**Controllers Layer** (`src/controllers/`):
- Purpose: Request handling and coordination
- Location: `src/controllers/`
- Contains: bonds_crawler controller
- Depends on: Services
- Used by: (not used directly in current flow - main.rs orchestrates)

**Services Layer** (`src/services/`):
- Purpose: Core business logic - scraping, messaging, analysis
- Location: `src/services/`
- Contains: bonds_crawler, rabbitmq_producer, rabbitmq_consumer, opencode_service
- Depends on: Models, Config, Repository
- Used by: main.rs, Controllers

**Repository Layer** (`src/repository/`):
- Purpose: Database persistence
- Location: `src/repository/`
- Contains: bonds_repository with sqlx queries
- Depends on: Models, Database connection
- Used by: Services

**Models Layer** (`src/models/`):
- Purpose: Data structures and serialization
- Location: `src/models/`
- Contains: bonds (Bond, BondListItem), rabbitmq
- Depends on: None (pure data)
- Used by: Services, Repository, Controllers

## Data Flow

### Primary Request Path (Direct Mode)

1. **Entry** (`src/main.rs:55`): `run_direct_mode` called with optional DB pool
2. **Config Load** (`src/config.rs:23`): `CrawlerConfig::from_env()` reads environment
3. **Init** (`src/main.rs:79`): `BondsCrawler::new(config, db_pool)` creates crawler
4. **Crawl** (`src/main.rs:81`): `crawler.run_crawl_loop()` performs scraping
   - Inside: WebDriver navigation, element finding, data extraction
   - Per row: parse_bond_row_inner extracts fields
5. **Output** (`src/main.rs:166-169`): Optional RabbitMQ publish or continue
6. **Close** (`src/main.rs:172`): WebDriver cleanup

### Secondary Request Path (Consumer Mode)

1. **Entry** (`src/main.rs:178`): `run_consumer_mode` starts
2. **Connect** (`src/main.rs:187`): `RabbitMQConsumer::new()` creates consumer
3. **Listen** (`src/main.rs:190-196`): `start_consuming()` with callback
4. **Callback**: Process received message (currently just prints)

**State Management:**
- WebDriver session in BondsCrawler (managed via Drop trait)
- Optional PostgreSQL pool in main.rs passed to crawler
- Optional RabbitMQ producer in main.rs

## Key Abstractions

**CrawlerConfig:**
- Purpose: Injected configuration from environment
- Examples: `src/config.rs` - CrawlerConfig struct
- Pattern: Builder/constructor from env vars

**CrawlerError:**
- Purpose: Unified error type with source tracking
- Examples: `src/error.rs` - enum with From impls
- Pattern: thiserror derive, Result<T> alias

**Result<T> alias:**
- Purpose: Shortcut for crate::error::Result<T>
- Examples: Used throughout services
- Pattern: Type alias in error.rs module

## Entry Points

**Direct Mode:**
- Location: `src/main.rs:55` - `run_direct_mode`
- Triggers: RUN_MODE=direct env var
- Responsibilities: Init crawler, run scrape loop, output results

**Consumer Mode:**
- Location: `src/main.rs:178` - `run_consumer_mode`
- Triggers: RUN_MODE=consumer env var
- Responsibilities: Init RabbitMQ consumer, listen for messages

## Architectural Constraints

- **Threading:** Single-threaded async (Tokio) - no manual thread spawning
- **Global state:** None detected - all state passed via structs
- **Circular imports:** None - module boundaries respected
- **Blocking calls:** WebDriver (thirtyfour) is async-compatible

## Anti-Patterns

### println! Usage Outside main.rs

**What happens:** Using `println!` in service files (e.g., bonds_crawler.rs:21)
**Why it's wrong:** Violates project convention - should use log macros
**Do this instead:** Use `info!`, `warn!`, `error!`, or `debug!` from log crate

### Direct Env Var Access in Services

**What happens:** Some code reads env vars directly instead of using CrawlerConfig
**Why it's wrong:** Violates CLAUDE.md rule: "Never read env vars in services"
**Do this instead:** Inject all config via CrawlerConfig in constructors

## Error Handling

**Strategy:** Result<T> with CrawlerError enum

**Patterns:**
- `?` operator for propagating errors
- From implementations for error conversion
- thiserror for deriving Display/Debug
- ConfigError separate from CrawlerError in config.rs

## Cross-Cutting Concerns

**Logging:** env_logger + log crate macros (not fully consistent - some println!)
**Validation:** Parsing with .parse().ok() for Option<T> fields
**Authentication:** Manual browser login (not programmatic)

---

*Architecture analysis: 2026-05-19*