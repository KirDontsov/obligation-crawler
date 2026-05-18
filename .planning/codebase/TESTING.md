# Testing Patterns

**Analysis Date:** 2026-05-19

## Test Framework

**Runner:**
- cargo test (built-in Rust test runner)
- No explicit test framework configured (uses std testing)

**Assertion Library:**
- Built-in Rust assertions (assert!, assert_eq!, assert_ne!)

**Run Commands:**
```bash
cargo test              # Run all tests
cargo test -- --nocapture  # Show output
```

## Test File Organization

**Location:**
- Inline tests within source files using #[cfg(test)] modules
- No dedicated test directory (standard Rust pattern)

**Naming:**
- test_* function naming convention
- Example: #[test] fn test_something()

**Structure:**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_function() {
        // test code
    }
}
```

## Test Coverage

**Current State:**
- No test files detected in src/
- dev-dependencies include: time 0.3.36
- No tests written in the codebase

**Recommendation:**
- Add tests following CLAUDE.md skill: /test [Name]
- Use tokio::test for async tests

## Test Types

**Unit Tests:**
- Not present in codebase
- Should test individual functions and modules

**Integration Tests:**
- Not present in codebase
- Should test service interactions

**E2E Tests:**
- Not used - WebDriver tests would require browser setup
- Manual testing via HEADLESS_CHROME=false

## Common Patterns

**Async Testing:**
```rust
#[tokio::test]
async fn test_async_function() {
    // async test code
}
```

**Error Testing:**
```rust
#[test]
fn test_error_case() {
    let result = function_that_returns_result();
    assert!(result.is_err());
}
```

## Fixtures and Factories

**Test Data:**
- Not detected - no fixture files
- Would create BondListItem instances for tests

**Location:**
- Could place in tests/ subdirectory or inline in #[cfg(test)] modules

---

*Testing analysis: 2026-05-19*