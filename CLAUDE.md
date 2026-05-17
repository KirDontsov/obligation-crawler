# Obligation Crawler — Claude Instructions

## Project

Rust async crawler for T-Bank bond listings. Scrapes data via Selenium (thirtyfour), enriches with AI analysis (opencode CLI), publishes to RabbitMQ (lapin), stores to PostgreSQL (sqlx), outputs CSV.

See full context: `/ai/context.md`

---

## Critical Rules

- **No `unwrap()`/`expect()` in production code** — always use `?` or handle explicitly
- **`log` macros only** — `info!`, `warn!`, `error!`, `debug!` in all modules except `main.rs`
- **English comments only** — no Russian in code comments
- **`crate::error::Result<T>`** — use the type alias, not raw `std::result::Result<T, CrawlerError>`
- **Never read env vars in services** — inject via `CrawlerConfig`
- **Module layer rule** — services cannot import from controllers; models cannot import from services

---

## Build & Validate

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

---

## Skills

| Skill | Use for |
|-------|---------|
| `/research [Feature]` | Explore codebase before implementing |
| `/plan [Task]` | Create adversarial dual-agent plan |
| `/service [Name]` | Create new service module |
| `/model [Name]` | Create new model struct |
| `/test [Name]` | Write unit/integration tests |
| `/fix [Bug]` | Minimal targeted bug fix |
| `/review` | Code review of current diff |

Full workflow: `/ai/workflow.md`

---

## Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry, mode dispatch |
| `src/config.rs` | `CrawlerConfig` from env vars |
| `src/error.rs` | `CrawlerError`, `Result<T>` alias |
| `src/services/bonds_crawler.rs` | WebDriver scraping logic |
| `src/services/rabbitmq_producer.rs` | RabbitMQ publish |
| `src/services/rabbitmq_consumer.rs` | RabbitMQ consume (reconnect loop) |
| `src/services/opencode_service.rs` | AI analysis via opencode CLI |
| `src/models/bonds.rs` | `Bond`, `BondListItem`, CSV I/O |
| `.env.example` | All supported env variables |

---

## Docs

- `ai/docs/code-style.md` — naming, logging, comments, DRY
- `ai/docs/error-handling.md` — CrawlerError patterns
- `ai/docs/async-patterns.md` — Tokio, WebDriver, RabbitMQ patterns
- `ai/docs/testing-guidelines.md` — test structure and naming
- `ai/docs/module-architecture.md` — module layers, anti-patterns
