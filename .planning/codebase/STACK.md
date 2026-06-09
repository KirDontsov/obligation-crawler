# Technology Stack

**Analysis Date:** 2026-06-09

## Languages

**Primary:**
- Rust (edition 2021) - Entire codebase under `src/`. Async service-oriented crawler.

**Secondary:**
- SQL (PostgreSQL dialect) - Schema migration in `migrations/001_create_crawler_schema.sql`
- Bash - Operational scripts `start.sh`, `start_driver.sh`

## Runtime

**Environment:**
- Tokio async runtime (full features) - entry via `#[tokio::main]` in `src/main.rs:20`
- Rust toolchain: Cargo build. `Dockerfile` builds with `rustlang/rust:nightly-bookworm`; no `rust-toolchain.toml` pinning a stable channel is committed.

**Package Manager:**
- Cargo
- Lockfile: present (`Cargo.lock`, committed)

## Frameworks

**Core:**
- `tokio` 1.52.3 (declared `1.33.0`, features = `["full"]`) - Async runtime, timers (`tokio::time::sleep`), task spawning
- `thirtyfour` 0.32.0 - Selenium/WebDriver client for Chrome automation (`src/services/bonds_crawler.rs`)
- `lapin` 3.7.2 (declared `3.7.0`) - AMQP 0.9.1 client for RabbitMQ (`src/services/rabbitmq_producer.rs`, `rabbitmq_consumer.rs`)
- `sqlx` 0.8.6 (declared `0.8`, features = `["runtime-tokio-rustls", "postgres", "chrono", "uuid"]`) - Async PostgreSQL access (`src/database.rs`, `src/repository/bonds_repository.rs`)

**Testing:**
- Built-in Rust test harness (`#[cfg(test)]` modules, e.g. `src/repository/bonds_repository.rs:181`)
- `time` 0.3.36 - dev-dependency only

**Build/Dev:**
- `env_logger` 0.11.0 - Initializes `log` backend in `src/main.rs:23`
- `rustfmt` - config `rustfmt.toml` (`hard_tabs=true`)

## Key Dependencies

**Critical:**
- `serde` 1.0.228 + `serde_json` 1.0.107 (features = `["derive"]`) - Serialization of `Bond`/`BondListItem` for RabbitMQ payloads and API responses
- `chrono` 0.4.44 (declared `0.4.30`, features = `["serde"]`) - Timestamps, maturity-date parsing (`NaiveDate::parse_from_str`)
- `uuid` 1.4.1 (features = `["serde", "v4"]`) - Crawl run IDs and DB primary keys
- `thiserror` 1.0.56 - Error enum derivation (`src/error.rs`, `src/config.rs`)
- `log` 0.4.20 - Logging facade (project rule: macros only, no `println!` in production per CLAUDE.md)

**Infrastructure:**
- `csv` 1.3.1 - CSV writer for bond output (`src/models/bonds.rs`)
- `reqwest` 0.12.28 (declared `0.12.2`) - HTTP client (dependency present; error variant wired in `src/error.rs:26`)
- `futures` 0.3.31 / `futures-util` 0.3.31 - Stream consumption in RabbitMQ consumer (`StreamExt`), `BoxFuture` handler signature
- `regex` 1.0 - text matching utilities
- `urlencoding` 2.1.3 - URL encoding helper
- `dotenv` 0.15.0 - Loads `.env` at startup (`src/main.rs:22`)

## Configuration

**Environment:**
- Loaded via `dotenv().ok()` at startup, then `std::env::var`.
- Typed config struct `CrawlerConfig` in `src/config.rs` (`from_env()`), with sane defaults for all values.
- Some env vars are read directly in `src/main.rs` and `src/database.rs` rather than through `CrawlerConfig` (see notes below).

**Config-struct env vars (`src/config.rs`):**
- `TBANK_URL` - default `https://www.tbank.ru/invest/bonds/`
- `POLL_INTERVAL_SECONDS` - default `5`
- `HEADLESS_CHROME` - default `false`
- `CHROME_DRIVER_PATH` - default `./chromedriver`
- `WAIT_AFTER_LOGIN_SECONDS` - default `60`
- `MAX_RETRIES` - default `3`

**Direct-read env vars (outside `CrawlerConfig`):**
- `RUN_MODE` - `direct` (default) or `consumer` (`src/main.rs:39`)
- `DURATION_MINUTES` - optional crawl duration (`src/main.rs:58`)
- `ENABLE_RABBITMQ` - default `false` (`src/main.rs:62`)
- `RABBITMQ_URL` - default `amqp://guest:guest@localhost:5672` (`src/main.rs:68`, `:179`)
- `RABBITMQ_EXCHANGE` - default `obligation_exchange` (`src/main.rs:70`)
- `RABBITMQ_QUEUE` - default `obligation_crawler_queue` (`src/main.rs:182`)
- `DATABASE_URL` - required for DB pool (`src/database.rs:6`, read with `.expect()`)

**Docker-only env vars (`docker-compose.yml`):**
- `WEBDRIVER_URL`, `PROCESSING_TYPE`, plus `DATABASE_URL_OVERRIDE` / `RABBITMQ_URL_OVERRIDE`. Note: `WEBDRIVER_URL` is set in compose but the code hardcodes `http://localhost:9515` in `src/services/bonds_crawler.rs:493`.

**Build:**
- `Cargo.toml`, `Cargo.lock`
- `Dockerfile` - multi-stage build (nightly builder → `debian:bookworm-slim` runtime); installs `libssl-dev`, `libpq-dev`, `postgresql-client`. Note: copies binary from `target/release/crawler`, but the package name is `obligation-crawler` (binary mismatch risk - see CONCERNS).

## Platform Requirements

**Development:**
- Rust + Cargo
- Running ChromeDriver on `localhost:9515` (`start_driver.sh` / `start.sh` launch `./chromedriver --port=9515 ...`)
- Chrome/Chromium browser
- `opencode` CLI on `PATH` (invoked via `std::process::Command` in `src/services/opencode_service.rs:9`)
- PostgreSQL instance (optional at runtime - crawler continues without DB if unavailable, `src/main.rs:33`)
- RabbitMQ broker (optional - only when `ENABLE_RABBITMQ=true` or in consumer mode)

**Production:**
- Docker (`Dockerfile`, `docker-compose.yml`). Runtime image is Debian slim with `ca-certificates`, `libssl3`, `libpq5`.
- `.env.docker` env file expected by compose (`env_file` directive). `.env*` is gitignored.

> Note: `.env.example` is referenced in `CLAUDE.md` but is not present in the repository (gitignored pattern `.env*` would exclude it). No example env template is committed.

---

*Stack analysis: 2026-06-09*
