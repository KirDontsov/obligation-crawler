# Technology Stack

**Analysis Date:** 2026-05-19

## Languages

**Primary:**
- Rust 2021 edition - Core application logic, services, models

## Runtime

**Environment:**
- Tokio async runtime 1.33.0 with full features
- ChromeDriver for browser automation

**Package Manager:**
- Cargo (Rust package manager)
- Cargo.lock present

## Frameworks

**Core:**
- thirtyfour 0.32.0 - Selenium WebDriver wrapper for Rust (Chrome automation)
- lapin 3.7.0 - RabbitMQ client for async message publishing/consuming
- sqlx 0.8 - Async PostgreSQL ORM with compile-time checking

**Testing:**
- time 0.3.36 (dev dependency)

**Build/Dev:**
- ChromeDriver (binary at project root)

## Key Dependencies

**Web Automation:**
- thirtyfour 0.32.0 - Selenium WebDriver for scraping T-Bank bond listings

**Async Runtime:**
- tokio 1.33.0 (full features) - Async runtime for all services
- futures-util 0.3.31 - Async utilities
- futures 0.3.31 - Futures abstractions

**Message Queue:**
- lapin 3.7.0 - RabbitMQ client

**Database:**
- sqlx 0.8 with features: runtime-tokio-rustls, postgres, chrono, uuid

**Web Client:**
- reqwest 0.12.2 - HTTP client for API calls

**Data Processing:**
- serde 1.0.188 - Serialization/deserialization
- serde_json 1.0.107 - JSON handling
- csv 1.3.1 - CSV file output
- chrono 0.4.30 with serde - Date/time handling
- uuid 1.4.1 with serde, v4 - Unique identifiers

**Error Handling:**
- thiserror 1.0.56 - Error type derivation
- log 0.4.20 - Logging facade

**Utilities:**
- regex 1.0 - Pattern matching
- dotenv 0.15.0 - Environment variable loading
- env_logger 0.11.0 - Logging initialization
- urlencoding 2.1.3 - URL encoding

## Configuration

**Environment:**
- Configuration via `.env` files (dotenv 0.15.0)
- CrawlerConfig struct for all settings
- Key config: TBANK_URL, HEADLESS_CHROME, POLL_INTERVAL_SECONDS, MAX_RETRIES

**Build:**
- Cargo.toml with all dependencies
- rustfmt.toml (hard_tabs=true)

**Code Quality:**
- Clippy for linting (see CLAUDE.md build commands)
- cargo fmt for formatting

## Platform Requirements

**Development:**
- Rust 2021 edition compatible
- Chrome/ChromeDriver for Selenium
- PostgreSQL for database (optional)
- RabbitMQ for message queue (optional)

**Production:**
- Binary executable via `cargo build --release`
- Requires ChromeDriver in PATH
- PostgreSQL connection optional (graceful fallback)
- RabbitMQ connection optional

---

*Stack analysis: 2026-05-19*