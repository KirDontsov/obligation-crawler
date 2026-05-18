# Codebase Structure

**Analysis Date:** 2026-05-19

## Directory Layout

```
obligation_crawler/
├── src/
│   ├── main.rs              # Entry point, mode dispatch
│   ├── config.rs            # CrawlerConfig struct
│   ├── error.rs             # CrawlerError, Result<T>
│   ├── database.rs         # Database connection pool
│   ├── api/                 # API handlers
│   ├── controllers/         # Controllers
│   ├── models/              # Data models
│   ├── repository/          # Database layer
│   ├── services/           # Core services
│   └── shared/             # Shared utilities
├── migrations/             # SQLx migrations
├── output/                 # CSV output directory
├── ai/                     # Project docs and context
├── .opencode/              # OpenCode config
├── .claude/                # Claude skills
├── Cargo.toml             # Project manifest
├── Cargo.lock             # Dependency lock
├── rustfmt.toml           # Formatting config
├── .env.example           # Env var template
├── docker-compose.yml     # Docker setup
├── Dockerfile             # Container build
├── chromedriver           # WebDriver binary
└── start*.sh              # Startup scripts
```

## Directory Purposes

**src/:**
- Purpose: All Rust source code
- Contains: Modules, services, models, configuration
- Key files: main.rs, config.rs, error.rs

**src/services/:**
- Purpose: Core business logic services
- Contains: bonds_crawler.rs, rabbitmq_producer.rs, rabbitmq_consumer.rs, opencode_service.rs
- Key files: `src/services/bonds_crawler.rs` - Main scraping logic

**src/models/:**
- Purpose: Data structures
- Contains: bonds.rs, rabbitmq.rs, mod.rs
- Key files: `src/models/bonds.rs` - Bond and BondListItem structs

**src/repository/:**
- Purpose: Database persistence
- Contains: bonds_repository.rs
- Key files: `src/repository/bonds_repository.rs`

**src/controllers/:**
- Purpose: Request handling
- Contains: bonds_crawler controller
- Note: Module layer rule - services cannot import from controllers

**src/api/:**
- Purpose: API layer
- Contains: bonds.rs, mod.rs

**migrations/:**
- Purpose: Database schema migrations
- Contains: SQL migration files

**output/:**
- Purpose: Generated CSV files
- Contains: Bond data exports

**ai/:**
- Purpose: Project documentation
- Contains: context.md, workflow.md, docs/

## Key File Locations

**Entry Points:**
- `src/main.rs`: Main entry with run_direct_mode and run_consumer_mode

**Configuration:**
- `src/config.rs`: CrawlerConfig struct and ConfigError
- `.env.example`: Environment variable documentation

**Core Logic:**
- `src/services/bonds_crawler.rs`: WebDriver scraping (767 lines)
- `src/services/rabbitmq_producer.rs`: Message queue publishing
- `src/services/rabbitmq_consumer.rs`: Message queue consumption

**Testing:**
- No test files detected in src/ (dev-dependencies present but unused)

## Naming Conventions

**Files:**
- snake_case.rs: All Rust source files use snake_case
- Example: bonds_crawler.rs, rabbitmq_producer.rs

**Directories:**
- snake_case: All directories use snake_case
- Example: src/services/, src/models/

**Functions:**
- snake_case: All functions use snake_case
- Example: run_direct_mode, run_crawl_loop

**Types:**
- PascalCase: Structs, enums use PascalCase
- Example: CrawlerConfig, CrawlerError, Bond

**Modules:**
- snake_case: Module directories use snake_case
- Example: mod api, mod services

## Where to Add New Code

**New Service:**
- Primary code: `src/services/`
- Pattern: Create service.rs file, add to services/mod.rs

**New Model:**
- Primary code: `src/models/`
- Pattern: Create model.rs file, add to models/mod.rs

**New Repository:**
- Primary code: `src/repository/`
- Pattern: Create repository.rs file, add to repository/mod.rs

**New Controller:**
- Primary code: `src/controllers/`
- Pattern: Create controller.rs file, add to controllers/mod.rs

**Utilities:**
- Shared helpers: `src/shared/`
- Add to shared/mod.rs

## Special Directories

**migrations/:**
- Purpose: PostgreSQL schema migrations
- Generated: By sqlx CLI
- Committed: Yes

**output/:**
- Purpose: Runtime CSV output
- Generated: At runtime
- Committed: No (gitignored)

**target/:**
- Purpose: Cargo build output
- Generated: By cargo build
- Committed: No (gitignored)

---

*Structure analysis: 2026-05-19*