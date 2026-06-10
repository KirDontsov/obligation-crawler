---
description: Fix a Rust bug with minimal targeted changes. Ask expected vs actual behavior, identify root cause (ownership, async, parsing, WebDriver, RabbitMQ), apply smallest fix, add regression test.
---

# Bug Fix Template

**Bug:** $ARGUMENTS

## Context
- Branch: !`git branch --show-current`
- Recent changes: !`git diff HEAD --name-only 2>/dev/null | head -10`

---

## Step 1 — Clarify

Ask if not already answered:
1. What is **expected** behavior?
2. What is **actual** behavior? (panic, wrong output, hang?)
3. Steps to reproduce?
4. Error messages/backtrace? (`RUST_BACKTRACE=1`)
5. Which file/function is affected?

---

## Step 2 — Diagnose

Read affected file(s). Identify:
- Root cause (not symptoms)
- Compile error, runtime panic, or logic error
- Minimal change needed

### Common Bug Categories

| Category | Symptoms | Fix |
|----------|----------|-----|
| Borrow checker | `cannot borrow as mutable`, `use of moved value` | Restructure ownership, clone at boundary |
| Stale WebDriver element | `StaleElementReference` | Re-find element after page navigation |
| None unwrap panic | `called Option::unwrap() on None` | Replace with `ok_or_else()?` or `if let` |
| Async blocking | Program hangs | Replace `std::thread::sleep` with `tokio::time::sleep` |
| Type mismatch | Compile error on `?` | Add `.map_err(|e| CrawlerError::Something(e.to_string()))` |
| Missing error variant | No `From` impl | Add variant to `CrawlerError`, add `#[from]` |
| Integer overflow | Panic on arithmetic | Use `.checked_add()` or `saturating_add()` |
| Parse failure | `ParseError` at runtime | Check cleaning logic before `.parse::<f64>()` |
| RabbitMQ disconnect | Consumer stops | Check error propagation |

---

## Step 3 — Fix

Requirements:
1. Minimal change — only what's needed
2. No refactoring or new features
3. No new unwrap/expect in production
4. Follow existing style (log macros, Result<T>)

```rust
// BEFORE: [bug description]
// [buggy code]

// AFTER: [fix description]
// [fixed code]
```

---

## Step 4 — Regression Test

Add test that would catch this bug:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn should_[expected]_when_[condition]() {
        // Arrange: bug scenario
        // Act: trigger
        // Assert: verify correct
    }

    #[tokio::test]
    async fn should_[expected]_when_[async_condition]() {
        // ...
    }
}
```

---

## Step 5 — Validate

```bash
cargo build 2>&1 | head -30
cargo test [name] 2>&1
cargo clippy -- -D warnings 2>&1 | head -20
```

---

## Checklist

- [ ] Root cause identified (not symptom)
- [ ] Minimal change — nothing extra
- [ ] No new unwrap in production
- [ ] Regression test added
- [ ] cargo build passes
- [ ] clippy clean
- [ ] Other functionality unaffected