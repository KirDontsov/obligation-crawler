# External Integrations

**Analysis Date:** 2026-06-09

## APIs & External Services

**Web scraping target (T-Bank):**
- T-Bank Invest bonds listing - the data source being crawled
  - URL: `https://www.tbank.ru/invest/bonds/` (configurable via `TBANK_URL`, `src/config.rs:24`)
  - Access method: browser automation, not a public API. Scraped via Selenium/ChromeDriver.
  - Auth: interactive manual login in the browser. The crawler waits `WAIT_AFTER_LOGIN_SECONDS` (default 60) for the user to log in (`src/services/bonds_crawler.rs:499`).
  - Parsing: CSS selectors against T-Bank's React markup (e.g. `.SecurityRow__showName_inlal`, `tbody[data-qa-type="uikit/dataTable.tableBody"]`) in `src/services/bonds_crawler.rs`. Fragile - tied to T-Bank's class names / `data-qa` attributes.

**Selenium WebDriver (ChromeDriver):**
- Browser automation layer via `thirtyfour` 0.32.0
  - SDK/Client: `thirtyfour::WebDriver` (`src/services/bonds_crawler.rs:8`)
  - Endpoint: hardcoded `http://localhost:9515` (`src/services/bonds_crawler.rs:493`). NOTE: not driven by `CHROME_DRIVER_PATH` or the `WEBDRIVER_URL` compose var.
  - Driver process: launched externally by `start_driver.sh` / `start.sh` (`./chromedriver --port=9515 --allowed-origins=* --no-sandbox ... --host=0.0.0.0`)
  - Chrome capabilities set in `initialize()`: headless (when `HEADLESS_CHROME=true`), `--no-sandbox`, `--disable-dev-shm-usage`, `--disable-blink-features=AutomationControlled`, etc. (`src/services/bonds_crawler.rs:476-491`)
  - Auth: none for the driver; relies on the human-driven browser session for T-Bank.

**opencode CLI (AI analysis):**
- Per-bond risk analysis via the `opencode` command-line tool
  - Client: `std::process::Command::new("opencode").arg("run").arg(&prompt)` (`src/services/opencode_service.rs:9`)
  - Synchronous subprocess call; captures stdout/stderr.
  - Prompt: large Russian-language fixed-income risk-model prompt built in `build_prompt()` (`src/services/opencode_service.rs:41`).
  - Invoked from the crawl path (`src/services/bonds_crawler.rs:207`), skipped when maturity < 1 year or price exceeds nominal by > 5 (`bonds_crawler.rs:178-194`).
  - Auth: external - depends on `opencode` CLI being installed and authenticated on the host. No credentials managed in this repo.

## Data Storage

**Databases:**
- PostgreSQL via `sqlx` 0.8.6 (`runtime-tokio-rustls`, `postgres`, `chrono`, `uuid` features)
  - Connection: `DATABASE_URL` env var, read in `src/database.rs:6` (with `.expect()` - panics if missing when pool creation is attempted)
  - Client/pool: `PgPoolOptions` - max 5 / min 1 connections, 30s acquire timeout, 600s idle, 1800s max lifetime (`src/database.rs:9-17`); validated with `SELECT 1`
  - Optional at runtime: if pool creation fails, the app logs a warning and continues without DB (`src/main.rs:33-37`)
  - Schema: `migrations/001_create_crawler_schema.sql` (applied manually: `psql $DATABASE_URL -f migrations/001_create_crawler_schema.sql`). Requires `pgcrypto` extension for `gen_random_uuid()`.
  - Tables: `obligation_crawler_runs` (one row per crawl session), `obligation_crawler_bonds` (one row per bond, FK to runs with cascade delete)
  - Queries: runtime-checked `sqlx::query` / `query_as` (not compile-time `query!` macros) in `src/repository/bonds_repository.rs`

**File Storage:**
- Local filesystem CSV output. Files written to `./output/bonds_<timestamp>.csv` (`src/services/bonds_crawler.rs:455`). Directory auto-created; bonds appended row-by-row during crawl (`src/models/bonds.rs:69`). `output/` is gitignored.

**Caching:**
- None

## Authentication & Identity

**Auth Provider:**
- None in-app. Target-site auth (T-Bank) is performed manually by a human in the automated browser; no credentials stored or managed by the crawler.

## Monitoring & Observability

**Error Tracking:**
- None (no Sentry/etc.)

**Logs:**
- `log` facade + `env_logger` backend, initialized in `src/main.rs:23`. Log level controlled via `RUST_LOG` env var (env_logger convention).
- Note: much of `src/services/bonds_crawler.rs` and `opencode_service.rs` uses `println!`/`eprintln!` directly rather than `log` macros, contrary to the CLAUDE.md project rule (see CONCERNS).

## CI/CD & Deployment

**Hosting:**
- Docker. `Dockerfile` (multi-stage) + `docker-compose.yml` defining two `consumer`-mode services (`two_phase_ads_crawler`, `ad_details_processor`). Compose content appears partly copied from an unrelated Avito crawler (volume `crawler_avito_feeds_data`, `PROCESSING_TYPE: two_phase_ads`) - see CONCERNS.

**CI Pipeline:**
- None detected (no `.github/workflows`, no CI config).

## Environment Configuration

**Required env vars:**
- `DATABASE_URL` - PostgreSQL connection string (only when DB persistence is used)
- `RABBITMQ_URL` - AMQP connection (defaults to `amqp://guest:guest@localhost:5672`)

**Optional / feature-flag env vars:**
- `RUN_MODE` (`direct` | `consumer`), `ENABLE_RABBITMQ`, `DURATION_MINUTES`
- `RABBITMQ_EXCHANGE`, `RABBITMQ_QUEUE`
- `TBANK_URL`, `POLL_INTERVAL_SECONDS`, `HEADLESS_CHROME`, `CHROME_DRIVER_PATH`, `WAIT_AFTER_LOGIN_SECONDS`, `MAX_RETRIES`
- Docker overrides: `DATABASE_URL_OVERRIDE`, `RABBITMQ_URL_OVERRIDE`, `WEBDRIVER_URL`, `PROCESSING_TYPE`

**Secrets location:**
- `.env` (loaded via `dotenv`) and `.env.docker` (compose `env_file`). All `.env*` patterns are gitignored. No secrets committed. No `.env.example` template present despite CLAUDE.md reference.

## Webhooks & Callbacks

**Incoming:**
- None (no HTTP server). RabbitMQ consumer mode receives messages from queue `obligation_crawler_queue` (`src/services/rabbitmq_consumer.rs`).

**Outgoing (RabbitMQ via `lapin`):**
- Producer publishes to a durable topic exchange (default `obligation_exchange`), declared in `RabbitMQProducer::new` (`src/services/rabbitmq_producer.rs:23-34`)
  - Routing key `bonds.data` - serialized bonds JSON (`publish_bonds_data`, `src/main.rs:169`)
  - Routing key `bonds.error` - error messages (`publish_error`)
- Consumer (`src/services/rabbitmq_consumer.rs`): durable queue, prefetch QoS=1, manual ack, with a reconnect loop retrying every 5s on failure. Consumer tag `obligation_crawler_consumer`.

---

*Integration audit: 2026-06-09*
