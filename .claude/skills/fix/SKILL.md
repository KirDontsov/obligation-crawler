---
name: fix
description: >-
  Fix a Rust bug with minimal targeted changes. Asks about expected vs actual
  behavior, identifies root cause (ownership, async, parsing, WebDriver, RabbitMQ),
  applies the smallest correct fix, and adds a regression test. Trigger when the
  user reports a bug, panic, compile error, or unexpected behavior. Trigger on
  phrases like "fix X", "bug in X", "X panics", "X crashes", "X doesn't work",
  "error in X", "compile error in X". Does NOT refactor or add features.
user-invocable: true
argument-hint: "[BugDescription]"
model: sonnet
---

# Bug Fix Template

**Bug:** $ARGUMENTS

## Context
- Branch: !`git branch --show-current`
- Recent changes: !`git diff HEAD --name-only 2>/dev/null | head -10`

---

## Step 1 — Clarify Before Fixing

Ask these questions if not already answered:

1. What is the **expected** behavior?
2. What is the **actual** behavior? (panic message, wrong output, hang?)
3. Steps to reproduce?
4. Any error messages, backtrace, or logs? (`RUST_BACKTRACE=1`)
5. Which file/function is affected?

---

## Step 2 — Diagnose

Read the affected file(s) fully. Identify:
- Root cause (not just symptoms)
- Whether this is a compile error, runtime panic, or logic error
- Minimal change needed to fix it

### Common Rust Bug Categories

| Category | Symptoms | Fix Approach |
|----------|----------|--------------|
| Borrow checker | `cannot borrow as mutable`, `use of moved value` | Restructure ownership, clone at boundary |
| Stale WebDriver element | `StaleElementReference` from thirtyfour | Re-find element by selector after page navigation |
| `None` unwrap panic | `called Option::unwrap() on None` | Replace with `ok_or_else(|| ...)? ` or `if let` |
| Async blocking | Program hangs | Replace `std::thread::sleep` with `tokio::time::sleep` |
| Type mismatch | Compile error on `?` | Add `.map_err(|e| CrawlerError::SomethingError(e.to_string()))` |
| Missing error variant | No `From` impl | Add variant to `CrawlerError`, add `#[from]` or manual `From` |
| Integer overflow | Panic on arithmetic | Use `.checked_add()` or `saturating_add()` |
| Parse failure | `ParseError` at runtime | Check cleaning logic before `.parse::<f64>()` |
| RabbitMQ disconnect | Consumer stops | Consumer already has reconnect loop; check if error is propagated instead |

---

## Step 3 — Fix

Requirements:
1. Minimal change — only what's needed
2. No refactoring or new features alongside the fix
3. No new `unwrap()`/`expect()` in production code
4. Follow existing code style (log macros, `Result<T>` alias)

```rust
// BEFORE: [describe the bug]
// [buggy code snippet]

// AFTER: [describe the fix]
// [fixed code snippet]
```

---

## Step 4 — Add Regression Test

Add a test that would have caught this bug:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_[expected_behavior]_when_[condition_that_caused_bug]() {
        // Arrange: set up the bug scenario
        // Act: trigger the operation
        // Assert: verify correct behavior
    }

    // For async bugs:
    #[tokio::test]
    async fn should_[expected_behavior]_when_[async_condition]() {
        // ...
    }
}
```

---

## Step 5 — Validate

```bash
cargo build 2>&1 | head -30
cargo test [test_name] 2>&1
cargo clippy -- -D warnings 2>&1 | head -20
```

---

## Checklist

- [ ] Root cause identified (not just symptom)
- [ ] Minimal change — nothing extra changed
- [ ] No new `unwrap()` in production code
- [ ] Regression test added
- [ ] `cargo build` passes
- [ ] `cargo clippy` clean
- [ ] Other functionality unaffected
