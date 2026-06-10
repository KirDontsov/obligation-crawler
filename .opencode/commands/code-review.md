---
description: Perform strict adversarial code review using dual agents (Finder + Auditor). Finder finds issues (architecture, Rust idioms, async safety, error handling). Auditor challenges findings with evidence from docs.
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

If `$ARGUMENTS` is empty → review current diff (`git diff HEAD`).
If ambiguous:
> "Which files to review? (path or blank for all changes)"

---

## Step 0 — Check mgrep

Run `which mgrep`. If not installed, fall back to `grep`.

---

## Step 1 — Get Changes

- No target → `git diff HEAD`
- File provided → `git diff HEAD -- <path>`

Read every changed file **fully**.

---

## Step 1 — Launch Both Agents

### Agent 1 — Reviewer (Finder)

```
You are a Senior Rust Engineer performing strict code review.

Target: $ARGUMENTS
Project root: [paste project root]

Read ALL changed files fully.

Before reviewing, read:
- /ai/docs/code-style.md
- /ai/docs/module-architecture.md
- /ai/docs/error-handling.md
- /ai/docs/async-patterns.md
- /ai/docs/testing-guidelines.md

For each issue:

### [Category]: [Title]
**File:** `src/path/file.rs:LINE`
**Severity:** 🔴 Critical / 🟡 Medium / 🟢 Low
**Issue:** [description]
**Current:** [code snippet]
**Fix:** [fixed snippet]
**Why:** [reason]

Categories to check:

ARCHITECTURE:
- Module layer violations (services → controllers)
- Config read in services instead of via CrawlerConfig
- Business logic in models

RUST IDIOMS:
- unwrap()/expect() in non-test production
- println! instead of log macros
- Missing ? propagation
- Blocking sleep in async
- Non-English comments

ERROR HANDLING:
- Missing CrawlerError variants
- Box<dyn Error> where variant should be used
- Silently swallowed errors

ASYNC SAFETY:
- Stale WebDriver element after navigation
- Missing await
- Non-Send held across await
- Blocking in async fn

OWNERSHIP:
- Unnecessary clones
- &mut self borrowed multiple times
- Moved value used after move

LOGGING:
- No context in log messages
- println! in services/models

TESTING:
- Public functions without tests
- Missing error-path tests
- Bad test naming

End with:
## Summary
🔴 Critical: [count] | 🟡 Medium: [count] | 🟢 Low: [count]
```

---

### Agent 2 — Auditor (Challenger)

```
You are a Senior Rust Engineer auditing a code review.

Target: $ARGUMENTS
Project root: [paste project root]

Read ALL changed files. Also read:
- /ai/context.md
- /ai/docs/code-style.md
- /ai/docs/module-architecture.md
- /ai/docs/error-handling.md
- /ai/docs/async-patterns.md

For each finding:
1. Is this actually a violation?
2. Does docs prohibit this?
3. Is severity correct?
4. What was missed?

Output per finding:

### [Category] Audit
**Verdict:** ✅ CONFIRM / ❌ FALSE POSITIVE / ⚠️ SEVERITY MISMATCH / 🔍 MISSED
**Evidence:** [quote from docs]
**Reasoning:** [why you agree/disagree]

## Missed Issues
[Same format]

## Audit Summary
Confirmed: [n] | False positives: [n] | Changed: [n] | Added: [n]
```

---

## Step 2 — Synthesize

| Status | Rule |
|--------|------|
| ✅ Confirmed | Include at original severity |
| ❌ False Positive | Exclude with reason |
| ⚠️ Severity Mismatch | Use Auditor's severity |
| 🔍 Missed Issue | Add to final |
| 🔄 Disputed | Present both — ask user |

---

## Step 3 — Present Final Review

```markdown
## Code Review — [Target]

### Synthesis Note
[X confirmed, Y dismissed, Z severity, W new]

---

### ❌ Changes Requested / ✅ Approved / ⚠️ Approved with Comments

#### Critical Issues
| File | Line | Issue | Doc Reference |
|------|------|-------|---------------|

#### Medium Issues
[table]

#### Low / Suggestions
[table]

#### Dismissed
[What was rejected and why]
```

---

## Step 4 — Save Review

If Critical/Medium issues:
> "Save to `ai/reviews/[YYYY-MM-DD]-review.md`?"

---

## Common Issues

### unwrap in production
```rust
// BAD
let driver = self.driver.as_ref().unwrap();

// GOOD
let driver = self.driver.as_ref()
    .ok_or_else(|| CrawlerError::SeleniumError("Not initialized".to_string()))?;
```

### println! in services
```rust
// BAD
println!("[DEBUG] Found {} rows", rows.len());

// GOOD
debug!("[collect] Found {} rows on page {}", rows.len(), page_num);
```

### Blocking sleep in async
```rust
// BAD
std::thread::sleep(Duration::from_secs(2));

// GOOD
tokio::time::sleep(Duration::from_secs(2)).await;
```

### Stale element after navigation
```rust
// BAD — stale after tab opens
for row in rows.iter() {
    open_detail_tab(driver, row).await?;
    process_data(driver, row).await?; // panic
}

// GOOD — re-find by index
for idx in 0..rows.len() {
    let row = table.find_all(By::Css("tr")).await?
        .into_iter().nth(idx)
        .ok_or_else(|| CrawlerError::SeleniumError("Row not found".into()))?;
    process_row(driver, &row).await?;
}
```