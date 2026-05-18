# Coding Conventions

**Analysis Date:** 2026-05-19

## Naming Patterns

**Files:**
- snake_case.rs - All Rust source files use snake_case
- Example: bonds_crawler.rs, rabbitmq_producer.rs

**Functions:**
- snake_case - All functions use snake_case
- Example: run_direct_mode, run_crawl_loop, parse_bond_row_inner

**Variables:**
- snake_case - Local variables use snake_case
- Example: db_pool, run_mode, csv_filename

**Types:**
- PascalCase - Structs, enums, traits use PascalCase
- Example: CrawlerConfig, CrawlerError, BondListItem

## Code Style

**Formatting:**
- rustfmt.toml: hard_tabs=true (uses tabs, not spaces)
- Standard Rust formatting conventions

**Linting:**
- clippy recommended (see CLAUDE.md: `cargo clippy -- -D warnings`)
- No additional linter config files detected

**Indentation:**
- Uses tabs (hard_tabs=true)

## Import Organization

**Order:**
1. Standard library imports (use std::...)
2. External crate imports (use crate::..., use tokio::...)
3. No explicit ordering enforcement detected

**Path Aliases:**
- crate:: prefix for internal modules
- Example: crate::config::CrawlerConfig, crate::error::Result

## Error Handling

**Patterns:**
- CrawlerError enum using thiserror derive
- Result<T> type alias in error.rs
- ? operator for propagating errors
- From implementations for error conversion

**Example (from src/error.rs):**
```rust
#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Crawler error: {0}")]
    CrawlerError(String),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    // ...
}

pub type Result<T> = std::result::Result<T, CrawlerError>;
```

**CRITICAL RULES (from CLAUDE.md):**
- No unwrap()/expect() in production code
- Use ? or handle explicitly
- Use crate::error::Result<T>, not raw std::result::Result

## Logging

**Framework:** log crate with env_logger

**Patterns:**
- info!, warn!, error!, debug! macros from log crate
- env_logger initialized in main.rs
- INCONSISTENCY: Some println! usage in services (should use log macros)

**Example (from src/services/bonds_crawler.rs):**
```rust
use log::{error, info, warn};
```

## Comments

**When to Comment:**
- Russian comments present in code (e.g., bonds_crawler.rs)
- CRITICAL RULE (from CLAUDE.md): English comments only - no Russian in code comments

**JSDoc/TSDoc:**
- Not applicable (Rust, not TypeScript)

## Function Design

**Size:**
- Variable - parse_bond_row_inner is 767 lines (large)
- No strict size limits enforced

**Parameters:**
- Explicit typing required
- Config injection via CrawlerConfig struct

**Return Values:**
- Use Result<T> or Option<T>
- Avoid unwrap in production

## Module Design

**Exports:**
- Use pub mod for public modules
- Use pub use for re-exports
- Example: mod api; creates api module from api/mod.rs or api.rs

**Barrel Files:**
- mod.rs files for module consolidation
- Example: src/services/mod.rs, src/models/mod.rs

## Project-Specific Rules (from CLAUDE.md)

1. **No unwrap()/expect()** in production code - use ? or handle explicitly
2. **log macros only** - info!, warn!, error!, debug! in all modules except main.rs
3. **English comments only** - no Russian in code comments
4. **crate::error::Result<T>** - use the type alias
5. **Never read env vars in services** - inject via CrawlerConfig
6. **Module layer rule** - services cannot import from controllers; models cannot import from services

---

*Convention analysis: 2026-05-19*