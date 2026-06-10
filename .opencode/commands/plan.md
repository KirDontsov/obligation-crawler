---
description: Create adversarial implementation plan using dual agents (Architect + Auditor). Architect proposes while Auditor challenges Rust patterns, ownership, async safety. Saves to ai/plans/. Use after /research.
---

# Plan Mode — Dual Agent

**Task:** $ARGUMENTS

## Context
- Branch: !`git branch --show-current`
- Date: !`date +%Y-%m-%d`
- Recent commits: !`git log --oneline -3`
- Research doc: !`ls ai/research/ 2>/dev/null | grep -i "$ARGUMENTS" | head -3 || echo "none found"`
- Services: !`ls src/services/ 2>/dev/null`
- Models: !`ls src/models/ 2>/dev/null`

---

## Initial Check

If `$ARGUMENTS` is empty:
> "What would you like to plan? Describe in one or two sentences."

---

## Phase 0 — Codebase Research

Spawn a research agent:

```
You are a Staff Rust Engineer researching: "$ARGUMENTS"

Read docs first:
- /ai/context.md
- /ai/docs/module-architecture.md
- /ai/docs/error-handling.md
- /ai/docs/async-patterns.md
- /ai/docs/testing-guidelines.md

Explore source:
- src/main.rs — run modes
- src/config.rs — config pattern
- src/error.rs — error enum
- src/services/ — all services
- src/models/ — all models
- src/controllers/ — controller pattern
- Cargo.toml — available deps

Answer:
1. What files are affected?
2. What patterns to reuse?
3. What constraints exist (ownership, lifetimes)?
4. What risks (borrow checker, WebDriver staleness, RabbitMQ reconnect)?

Output:
## Affected Files
[list]

## Reusable Patterns Found
[file paths]

## Constraints
[ownership, lifetimes]

## Risks / Anti-patterns
[specific to Rust]
```

Wait for output before Phase 1.

---

## Engineering Principles

Apply to ALL tasks:

1. **Ownership first** — avoid clones where possible
2. **`?` everywhere** — no unwrap in production
3. **Log with context** — info!/warn!/error! with details
4. **Async safety** — no std::thread::sleep, no blocking in async
5. **Test the fallible** — Result fns need success + error tests

---

## Phase 1 — Launch Both Agents

Spawn both agents **simultaneously**:

### Agent 1 — Architect

```
You are a Staff Rust Engineer creating plan for: "$ARGUMENTS"

Context from research:
[paste Phase 0 findings]

Read:
- /ai/context.md
- /ai/docs/module-architecture.md
- /ai/docs/error-handling.md
- /ai/docs/async-patterns.md
- /ai/docs/testing-guidelines.md

Principles:
1. Ownership first — minimize clones
2. No unwrap — use ?
3. Log with context
4. Async safety
5. Test fallible

For each decision:
1. What you propose
2. Why (reasoning)
3. Which files
4. What could go wrong

Output:

## Architectural Decisions
[Proposal → Rationale → Files → Risk]

## Atomic Task Plan
- [ ] Phase 1: [Name]
  - [ ] Task 1.1: [action] — `src/path/file.rs`
  - [ ] Task 1.2: [action] — `src/path/file.rs`
- [ ] Phase 2: [Name]
  - [ ] Task 2.1: [action] — `src/path/file.rs`
- [ ] Phase 3: Tests
  - [ ] Task 3.1: Unit tests — `src/path/file.rs#[cfg(test)]`

## Success Criteria
### Automated
- [ ] `cargo build` passes
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt --check` passes
### Manual
- [ ] [observable behavior]

## What We Are NOT Doing
[Explicit scope boundary]

## Key Risks
[Top 3 with mitigations]
```

### Agent 2 — Auditor

```
You are a Senior Rust Engineer auditing plan for: "$ARGUMENTS"

Read to validate:
- /ai/context.md
- /ai/docs/module-architecture.md
- /ai/docs/error-handling.md
- /ai/docs/async-patterns.md
- /ai/docs/code-style.md
- /ai/docs/testing-guidelines.md

For EACH decision evaluate:
1. Violates module layer rules?
2. Ownership/borrow issues?
3. Async-safe?
4. Handles errors correctly?
5. DRY (no duplicated code)?
6. FACTS (Feasible, Atomic, Clear, Testable, Scoped)?

Verdict per decision:
- ✅ CONFIRM — sound
- ❌ REJECT — violates conventions (cite doc)
- ⚠️ MODIFY — needs adjustment

Output:

## Audit Findings
### Decision: [Name]
**Verdict:** ✅/❌/⚠️
**Reasoning:** [cite /ai/docs/*]
**Alternative (if ⚠️/❌):** [concrete alternative]

## Missing Considerations
[What Architect missed]

## FACTS Validation
- [ ] Feasible: [assessment]
- [ ] Atomic: [assessment]
- [ ] Clear: [assessment]
- [ ] Testable: [assessment]
- [ ] Scoped: [assessment]
```

---

## Phase 2 — Synthesize

| Status | Rule |
|--------|------|
| ✅ Confirmed | Include as-is |
| ❌ Rejected | Exclude with evidence |
| ⚠️ Modified | Use Auditor's version |
| 🔄 Disputed | Present both, ask user |

---

## Phase 3 — Present Final Plan

```markdown
## Final Implementation Plan — [Task]

### Synthesis Summary
[What was confirmed / modified / rejected]

### Atomic Task Plan
- [ ] Phase 1: [Name]
  - [ ] Task 1.1: [action] — `src/path/file.rs`
- [ ] Phase 2: Tests
  - [ ] Task 2.1: Unit tests — `src/path/file.rs#[cfg(test)]`

### Success Criteria
#### Automated
- [ ] cargo build passes
- [ ] cargo test passes
- [ ] clippy clean
- [ ] fmt passes

#### Manual
- [ ] [behavior]

### What We Are NOT Doing
- [item] — [reason]
```

**Do not implement until user approves.**

---

## Phase 4 — Save Plan

After approval:

```
mkdir -p ai/plans
```

File: `ai/plans/[YYYY-MM-DD]-[task-slug].md`

---

## After Approval

Suggest:
- `/service [Name]`
- `/model [Name]`
- `/test [Name]`
- `/fix [Bug]`