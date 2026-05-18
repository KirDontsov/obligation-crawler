# External Integrations

**Analysis Date:** 2026-05-19

## APIs & External Services

**Web Scraping:**
- T-Bank Investments (tbank.ru) - Bond listings
  - SDK/Client: thirtyfour 0.32.0 (Selenium WebDriver)
  - URL: Configurable via TBANK_URL (default: https://www.tbank.ru/invest/bonds/)
  - Auth: Manual login (see WAIT_AFTER_LOGIN_SECONDS)

**AI Analysis:**
- OpenCode CLI - Bond analysis enrichment
  - Implementation: `src/services/opencode_service.rs`
  - Executable: opencode (external CLI)

## Data Storage

**Databases:**
- PostgreSQL
  - Connection: DATABASE_URL env var
  - Client: sqlx 0.8 with tokio runtime
  - Optional: Graceful fallback if unavailable
  - Tables: bonds (via repository layer)

**File Storage:**
- CSV output via csv crate
  - Location: `./output/` directory
  - Files: BondListItem CSV with Russian headers

**Caching:**
- None

## Authentication & Identity

**Auth Provider:**
- T-Bank: Manual browser login
  - WAIT_AFTER_LOGIN_SECONDS: 60 (default)
  - Session persists in ChromeDriver

**Internal:**
- UUID generation for bond tracking (uuid crate)

## Monitoring & Observability

**Error Tracking:**
- None (custom CrawlerError enum in `src/error.rs`)

**Logs:**
- env_logger 0.11.0 - Initialized in main.rs
- log crate macros (info!, warn!, error!, debug!)
- println! for direct output in some places

## CI/CD & Deployment

**Hosting:**
- Docker support via docker-compose.yml and Dockerfile

**CI Pipeline:**
- None detected

## Environment Configuration

**Required env vars:**
- TBANK_URL - Target URL for scraping
- RUN_MODE - "direct" or "consumer"

**Optional env vars:**
- HEADLESS_CHROME - Browser mode
- CHROME_DRIVER_PATH - WebDriver location
- POLL_INTERVAL_SECONDS - Scraping interval
- WAIT_AFTER_LOGIN_SECONDS - Login wait time
- MAX_RETRIES - Error retry count
- DURATION_MINUTES - Run duration limit
- ENABLE_RABBITMQ - Enable message publishing
- RABBITMQ_URL, RABBITMQ_EXCHANGE, RABBITMQ_QUEUE - RabbitMQ config
- DATABASE_URL - PostgreSQL connection

**Secrets location:**
- `.env` file in project root (not committed to git - see .gitignore)

## Webhooks & Callbacks

**Outgoing:**
- RabbitMQ message publishing (optional)
  - Exchange: obligation_exchange (default)
  - Queue: obligation_crawler_queue (default)
  - Format: JSON serialized bond data

**Incoming:**
- RabbitMQ consumer mode (RUN_MODE=consumer)
  - Listens on configurable queue
  - Callback processing in `src/services/rabbitmq_consumer.rs`

---

*Integration audit: 2026-05-19*