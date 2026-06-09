# Testing Patterns

**Analysis Date:** 2026-06-09

Reference doc: `ai/docs/testing-guidelines.md`.

## Test Framework

**Runner:**
- Built-in Rust test harness via `cargo test` (no external test runner).
- Async tests use `#[tokio::test]` (tokio `full` features, `Cargo.toml`).

**Assertion Library:**
- Built-in macros: `assert!`, `assert_eq!`, and `matches!` for error-variant checks.

**Dev-dependencies (`Cargo.toml`):**
- `time = "0.3.36"` (the only declared `[dev-dependencies]`).
- `uuid` and `tempfile` appear in `ai/docs` examples; `uuid` is a regular dependency,
  `tempfile` is NOT currently a dependency — add it if a test needs it.

**Run Commands:**
```bash
cargo test                 # Run all unit tests
cargo test -- --ignored    # Run integration tests gated with #[ignore]
cargo test should_have     # Filter by test-name substring
```

## Validation Commands

All four must pass before code is considered done (`CLAUDE.md` Build & Validate):
```bash
cargo build                # Compiles
cargo test                 # All tests pass
cargo clippy -- -D warnings  # Lint, warnings are errors
cargo fmt --check          # Formatting (hard_tabs=true) is clean
```

## Test File Organization

**Location (`ai/docs/testing-guidelines.md`):**
- **Unit tests:** a `#[cfg(test)]` module at the BOTTOM of the same source file
  (not a separate file).
- **Integration tests:** in a `tests/` directory at the crate root (does not yet
  exist in this repo).

**Structure:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_do_x_when_y() { ... }
}
```

## Naming Convention

```
fn should_[expected_behavior]_when_[condition]()
fn should_[expected_behavior]_given_[state]()
```

Examples (from `ai/docs/testing-guidelines.md`):
```rust
fn should_parse_price_when_text_contains_ruble_sign()
fn should_return_none_when_cells_count_less_than_four()
fn should_use_defaults_when_env_vars_not_set()
fn should_create_csv_with_headers_given_valid_path()
```

## Existing Test Coverage (actual)

Only ONE test module currently exists in the codebase:

- `src/repository/bonds_repository.rs` (lines 181-209):
  - `#[test] fn should_have_all_bond_record_fields()` — a synchronous compile-time
    check that constructs a fully-populated `BondListItem` (all fields explicit,
    no `Default`) and asserts on `ticker` and `price`. Follows the `should_X_when_Y`
    naming pattern (here without a `_when_` clause).

No `#[tokio::test]` async tests exist yet. No `tests/` integration directory exists.
The following modules have NO tests and are candidates for coverage:
`src/config.rs`, `src/error.rs`, `src/models/bonds.rs` (CSV + serde),
`src/shared/utils.rs`, `src/services/*`.

## Test Structure Patterns

**Synchronous unit test (Arrange / Act / Assert):**
```rust
#[test]
fn should_use_defaults_when_env_vars_not_set() {
    std::env::remove_var("POLL_INTERVAL_SECONDS");
    let config = CrawlerConfig::from_env().unwrap();
    assert_eq!(config.poll_interval_seconds, 5);
}
```

**Async unit test:**
```rust
#[tokio::test]
async fn should_fail_on_invalid_rabbitmq_url() {
    let result = RabbitMQProducer::new(
        "invalid://url".to_string(),
        "exchange".to_string(),
    ).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), CrawlerError::RabbitMQError(_)));
}
```

`unwrap()` / `expect()` are explicitly ALLOWED inside `#[cfg(test)]` blocks.

## Mocking

**Framework:** None. There is no mocking library in `Cargo.toml`.

**Approach:**
- Test pure functions directly (no I/O), e.g. text-cleaning helpers in
  `src/shared/utils.rs`.
- For services that hold network connections, test the error path by passing an
  invalid URL/config (no live service required) and assert the returned
  `CrawlerError` variant with `matches!`.

**What NOT to mock:** No real network/DB calls in unit tests — gate any test that
needs a live service behind `#[ignore]` and put it in `tests/`.

## Fixtures and Factories

**Approach:** Build structs inline in each test. There are no shared factory helpers.
- `BondListItem` does NOT derive `Default`, so tests must construct every field
  explicitly (as in `src/repository/bonds_repository.rs`). If you add `Default`,
  the `..Default::default()` shorthand shown in `ai/docs/testing-guidelines.md`
  becomes usable.

**Temp files:** CSV tests create temp files (e.g. `format!("/tmp/test_bonds_{}.csv",
uuid::Uuid::new_v4())`) and must clean them up with `fs::remove_file` at the end.

## Coverage

**Requirements:** None enforced (no coverage gate / CI config detected).

**Checklist target (`ai/docs/testing-guidelines.md`):**
- Every public function has at least one happy-path test.
- Every fallible operation has an error-path test.
- No real network/DB calls in unit tests.
- Tests clean up temp files they create.
- Naming follows `should_X_when_Y`.

## Test Types

**Unit Tests:** `#[cfg(test)]` module at the bottom of each source file. Cover pure
functions, config parsing, serde round-trips, error-variant mapping.

**Integration Tests:** `tests/` directory (not yet present). Gate tests requiring
external services (RabbitMQ, PostgreSQL) with `#[ignore = "requires running X"]` and
read connection details from env with a sensible default.

```rust
// tests/rabbitmq_integration.rs
#[tokio::test]
#[ignore = "requires running RabbitMQ"]
async fn should_publish_bonds_data() { ... }
```

**E2E Tests:** Not used.

## Common Patterns

**Async testing:** `#[tokio::test] async fn ...`, `.await` the call, then assert.

**Error testing:** Use `matches!` to assert the variant:
```rust
assert!(result.is_err());
assert!(matches!(result.unwrap_err(), CrawlerError::ParseError(_)));
```

**Config testing:** Set/remove env vars with `std::env::set_var` /
`std::env::remove_var`, then call `CrawlerConfig::from_env()` and assert on fields or
the `ConfigError` variant. Always remove the env var afterward to avoid cross-test
contamination.

---

*Testing analysis: 2026-06-09*
