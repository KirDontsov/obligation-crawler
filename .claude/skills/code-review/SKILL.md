---
name: code-review
description: >-
  Perform a strict adversarial code review using dual agents (Finder + Auditor).
  The Finder identifies issues across architecture, Rust idioms, async safety,
  error handling, WebDriver patterns, RabbitMQ patterns, and testing. The Auditor
  challenges each finding with evidence from project docs, eliminates false positives,
  and adds missed issues. Produces a synthesized review with severity levels.
  Trigger on "review", "check my code", "review changes", "look at my changes",
  "is this correct", "code review", or after implementing something before merging.
user-invocable: true
argument-hint: "[FileName or empty for current diff]"
model: opus
---

# Code Review

**Target:** $ARGUMENTS

## Context
- Branch: !`git branch --show-current`
- Date: !`date +%Y-%m-%d`
- Project root: !`pwd`
- Changed files: !`git diff HEAD --name-only 2>/dev/null`
- Recent commits: !`git log --oneline -5`

---

## Initial Check

If `$ARGUMENTS` is empty → review current branch diff (`git diff HEAD`).
If ambiguous:
> "Which files should I review? (file path, or leave blank for all current changes)"

---

## STEP 0 — Check mgrep

Run `which mgrep`. If not installed, fall back to `grep` in agent prompts.

---

## STEP 1 — Get the Changes

- No target → `git diff HEAD`
- File provided → `git diff HEAD -- <path>`

Read every changed file **fully**, not just the diff.

---

## STEP 1 — Launch Both Agents in Parallel

### Agent 1 — Code Reviewer (Finder)

```
You are a Senior Rust Engineer performing a strict code review.

Target: $ARGUMENTS
Project root: [paste project root]

Read ALL changed files fully.

Before reviewing, read:
- /ai/docs/code-style.md
- /ai/docs/module-architecture.md
- /ai/docs/error-handling.md
- /ai/docs/async-patterns.md
- /ai/docs/testing-guidelines.md

For each issue output:

### [Category]: [Short title]
**File:** `src/path/to/file.rs:LINE`
**Severity:** 🔴 Critical / 🟡 Medium / 🟢 Low
**Issue:** [clear description]
**Current:** [code snippet]
**Fix:** [fixed snippet]
**Why:** [reason this matters]

Categories to check:

ARCHITECTURE: per /ai/docs/module-architecture.md
- Module layer violations (services importing from controllers, etc.)
- Config read directly in services instead of via CrawlerConfig
- Business logic in models

RUST IDIOMS: per /ai/docs/code-style.md
- unwrap()/expect() in non-test production code
- println! in non-main modules (use log macros)
- Missing ? propagation (silently ignoring Result/Option)
- Blocking sleep (std::thread::sleep in async context)
- Comments in non-English language

ERROR HANDLING: per /ai/docs/error-handling.md
- Missing error variants for new error sources
- Box<dyn Error> where CrawlerError variant should be used
- Silently swallowed errors (let _ = fallible_op())

ASYNC SAFETY: per /ai/docs/async-patterns.md
- Stale WebDriver element access after page navigation
- Missing await on async calls
- Non-Send types held across await points
- Blocking operations inside async fn

OWNERSHIP / BORROWING:
- Unnecessary clones
- &mut self borrowed multiple times
- Moved value used after move

LOGGING:
- No context in log messages (what ticker? what page?)
- println! in services/models/controllers

TESTING: per /ai/docs/testing-guidelines.md
- New public functions without tests
- Missing error-path tests for Result-returning functions
- unwrap() in tests on non-test code paths
- Test naming not following should_X_when_Y pattern

End with:
## Summary
🔴 Critical: [count] | 🟡 Medium: [count] | 🟢 Low: [count]
```

---

### Agent 2 — Review Auditor (Challenger)

```
You are a skeptical Senior Rust Engineer auditing a code review.

Target: $ARGUMENTS
Project root: [paste project root]

Read ALL changed files fully. Also read:
- /ai/context.md
- /ai/docs/code-style.md
- /ai/docs/module-architecture.md
- /ai/docs/error-handling.md
- /ai/docs/async-patterns.md
- /ai/docs/testing-guidelines.md

For each finding from the Reviewer:
1. Is this actually a violation, or acceptable in this project's context?
2. Does project documentation explicitly prohibit this?
3. Is the severity level correct?
4. What did the Reviewer miss?

Output per finding:

### [Category] Audit
**Verdict:** ✅ CONFIRM / ❌ FALSE POSITIVE / ⚠️ SEVERITY MISMATCH / 🔍 MISSED ISSUE
**Evidence:** [quote from /ai/docs/...]
**Reasoning:** [why you agree or disagree]

## Missed Issues
[Issues the Reviewer likely missed — same format]

## Audit Summary
Confirmed: [n] | False positives: [n] | Severity changes: [n] | Missed added: [n]
```

---

## STEP 2 — Synthesize

| Status | Rule |
|--------|------|
| ✅ Confirmed | Include at original severity |
| ❌ False Positive | Exclude — note why |
| ⚠️ Severity Mismatch | Use Auditor's severity |
| 🔍 Missed Issue | Add to final output |
| 🔄 Disputed | Present both — ask user |

---

## STEP 3 — Present Final Review

```markdown
## Code Review — $ARGUMENTS

### Synthesis Note
[X confirmed, Y dismissed, Z severity changes, W new issues]

---

### ❌ Changes Requested / ✅ Approved / ⚠️ Approved with Comments

#### Critical Issues (Must Fix Before Merge)
| File | Line | Issue | Severity | Doc Reference |
|------|------|-------|----------|---------------|

#### Medium Issues (Should Fix)
[same table]

#### Low / Suggestions
[same table]

#### Dismissed Findings
[What was rejected and why]
```

---

## STEP 4 — Save Review (Optional)

If there are Critical or Medium issues:
> "Save this review to `ai/reviews/[YYYY-MM-DD]-review.md`?"

---

## Common Issues Reference

### unwrap in production code

```rust
// ❌ BAD
let driver = self.driver.as_ref().unwrap();
let url = env::var("KEY").unwrap();

// ✅ GOOD
let driver = self.driver.as_ref()
    .ok_or_else(|| CrawlerError::SeleniumError("Not initialized".to_string()))?;
let url = env::var("KEY")
    .map_err(|_| ConfigError::MissingEnvVar("KEY".to_string()))?;
```

### println! in services

```rust
// ❌ BAD (in src/services/*.rs)
println!("[DEBUG] Found {} rows", rows.len());

// ✅ GOOD
debug!("[collect_bonds] Found {} rows on page {}", rows.len(), page_num);
```

### Blocking sleep in async

```rust
// ❌ BAD
std::thread::sleep(std::time::Duration::from_secs(2));

// ✅ GOOD
tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
```

### Silently swallowed errors

```rust
// ❌ BAD
let _ = driver.quit().await;

// ✅ GOOD (in Drop — acceptable)
// ✅ GOOD (in service methods — log the error)
if let Err(e) = driver.quit().await {
    error!("Failed to quit WebDriver: {}", e);
}
```

### Stale element after tab switch

```rust
// ❌ BAD — rows stale after opening a tab
for row in rows.iter() {
    open_detail_tab(driver, row).await?;  // row is stale after this
    process_data(driver, row).await?;     // panic: stale element
}

// ✅ GOOD — re-find by index
for idx in 0..rows.len() {
    let row = table_body
        .find_all(By::Css("tr[data-qa-type=\"...\"]")).await?
        .into_iter().nth(idx)
        .ok_or_else(|| CrawlerError::SeleniumError("Row not found".to_string()))?;
    process_row(driver, &row).await?;
}
```
