# Codebase Concerns

**Analysis Date:** 2026-05-19

## Tech Debt

**Println Usage Outside main.rs:**
- Issue: Using println! in service files violates project convention
- Files: `src/services/bonds_crawler.rs:21` and potentially other locations
- Impact: Inconsistent logging, not configurable
- Fix approach: Replace with log macros (info!, warn!, error!, debug!)

**Russian Comments:**
- Issue: Code contains Russian language comments violating CLAUDE.md rule
- Files: `src/services/bonds_crawler.rs`, `src/models/bonds.rs`
- Impact: Breaks English-only comment rule
- Fix approach: Translate comments to English

**Direct Env Var Access:**
- Issue: Some code reads env vars directly instead of using CrawlerConfig
- Files: `src/main.rs:39` (RUN_MODE), `src/main.rs:58-64` (DURATION_MINUTES, ENABLE_RABBITMQ)
- Impact: Violates "never read env vars in services" rule, inconsistent pattern
- Fix approach: Add to CrawlerConfig and inject

**Module Layer Violations:**
- Issue: Potential cross-layer imports
- Files: Not fully verified, but controllers → services → repository → models not strictly enforced
- Impact: Tight coupling, harder to test
- Fix approach: Review and enforce layer boundaries

## Known Bugs

**No known bugs detected.** Project appears functional for basic crawling operations.

## Security Considerations

**Env File in Repo:**
- Risk: .env file may contain secrets
- Files: `.env` file exists (gitignored, but checked in historically)
- Current mitigation: .gitignore excludes .env
- Recommendations: Verify .env is in .gitignore, never commit secrets

**ChromeDriver:**
- Risk: Old ChromeDriver binary in repo (March 2024)
- Files: `chromedriver` binary
- Current mitigation: None
- Recommendations: Update or generate dynamically

## Performance Bottlenecks

**Large Parse Function:**
- Problem: parse_bond_row_inner in bonds_crawler.rs is 767 lines
- Files: `src/services/bonds_crawler.rs`
- Cause: Single function handles all parsing logic
- Improvement path: Break into smaller helper functions

**No Caching:**
- Problem: Each run scrapes full page
- Files: `src/services/bonds_crawler.rs`
- Cause: No caching layer
- Improvement path: Add caching for repeated queries

## Fragile Areas

**WebDriver Scraper:**
- Files: `src/services/bonds_crawler.rs`
- Why fragile: CSS selectors are brittle, tied to T-Bank page structure
- Safe modification: Add more robust selectors, handle missing elements gracefully
- Test coverage: No tests - manual verification only

**RabbitMQ Reconnection:**
- Files: `src/services/rabbitmq_consumer.rs`
- Why fragile: Reconnection logic may not handle all failure scenarios
- Safe modification: Add exponential backoff, circuit breaker pattern
- Test coverage: No tests

## Dependencies at Risk

**thirtyfour:**
- Risk: WebDriver wrapper, depends on ChromeDriver compatibility
- Impact: T-Bank website changes could break scraping
- Migration plan: Could switch to another Selenium wrapper if needed

**sqlx:**
- Risk: Compile-time query checking requires migrations
- Impact: Schema changes need migration files
- Migration plan: Standard for Rust ORMs

## Missing Critical Features

**Test Coverage:**
- Problem: No unit or integration tests
- Blocks: Confidence in refactoring, regression detection

**Error Recovery:**
- Problem: Limited retry logic for transient failures
- Blocks: Reliable operation in production

**Configuration Validation:**
- Problem: Some env vars not validated at startup
- Blocks: Early failure detection

## Test Coverage Gaps

**All Services:**
- What's not tested: All service functions lack tests
- Files: src/services/*.rs
- Risk: Bugs go undetected until runtime
- Priority: HIGH

**Models:**
- What's not tested: Model serialization/deserialization
- Files: src/models/bonds.rs
- Risk: CSV parsing bugs, serialization issues
- Priority: MEDIUM

**Configuration:**
- What's not tested: Config validation
- Files: src/config.rs
- Risk: Invalid config causes runtime panics
- Priority: MEDIUM

---

*Concerns audit: 2026-05-19*