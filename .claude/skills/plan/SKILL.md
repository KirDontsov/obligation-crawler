---
name: plan
description: >-
  Create an adversarial implementation plan using dual agents (Architect + Auditor).
  Starts with codebase research (reads all modules: services/, models/, config, error),
  then Architect proposes a plan while Auditor challenges every decision against
  Rust patterns, ownership/borrowing rules, async safety, and FACTS criteria.
  Produces a validated atomic task checklist saved to ai/plans/. Use after /research,
  before any implementation. Trigger on "plan X", "break down X into tasks",
  "how should I implement X", "what steps for X". Does NOT write implementation code.
user-invocable: true
argument-hint: "[TaskDescription]"
model: opus
---

# Senior Rust Engineer — Plan Mode

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

If `$ARGUMENTS` is empty or unclear:
> "What would you like to plan? Describe the feature or task in one or two sentences."

---

## Phase 0 — Codebase Research

Spawn a research agent:

```
You are a Staff Rust Engineer researching the codebase before planning: "$ARGUMENTS"

Read these docs first:
- /ai/context.md
- /ai/docs/module-architecture.md
- /ai/docs/error-handling.md
- /ai/docs/async-patterns.md
- /ai/docs/testing-guidelines.md

Then explore the actual source:
- src/main.rs — run modes, orchestration
- src/config.rs — config pattern
- src/error.rs — error enum
- src/services/ — all service files
- src/models/ — all model structs
- src/controllers/ — controller pattern
- Cargo.toml — available dependencies

Answer:
1. What files are directly affected?
2. What existing patterns can be reused (don't reinvent)?
3. What constraints exist (ownership, async lifetimes, existing error variants)?
4. What risks: borrow checker issues, stale WebDriver elements, RabbitMQ reconnect?

Output:

## Affected Files
[list with brief description]

## Reusable Patterns Found
[list with file paths and what to reuse]

## Constraints
[ownership, lifetimes, existing interfaces]

## Risks / Anti-patterns to Avoid
[specific to Rust: borrow issues, async pitfalls, WebDriver staleness]
```

Wait for research output before Phase 1.

---

## Engineering Principles

Apply to ALL tasks — both agents check against them:

1. **Ownership first** — Design data flow to avoid clones where possible; clone only at boundaries
2. **`?` everywhere** — No `unwrap()`/`expect()` in production code
3. **Log with context** — `info!`/`warn!`/`error!` with enough detail to debug
4. **Async safety** — No `std::thread::sleep`, no blocking in async context
5. **Test the fallible** — Every `Result`-returning fn needs a success + error test

---

## Phase 1 — Launch Both Agents in Parallel

Spawn both agents **simultaneously**:

### Agent 1 — Architect (Proposer)

```
You are a Staff Rust Engineer creating an implementation plan for:
"$ARGUMENTS"

Context from codebase research:
[paste Phase 0 findings]

Read these before proposing:
- /ai/context.md
- /ai/docs/module-architecture.md
- /ai/docs/error-handling.md
- /ai/docs/async-patterns.md
- /ai/docs/testing-guidelines.md

Engineering principles:
1. Ownership first — minimize unnecessary clones
2. No unwrap in production — always ?
3. Log with context — info!/warn!/error! with field values
4. Async safety — tokio::time::sleep, no blocking
5. Test the fallible — Result-returning fns need success + error tests

For each architectural decision:
1. What you propose
2. Why (concrete reasoning)
3. Which files are affected
4. What could go wrong (ownership, lifetime, async)

Output:

## Architectural Decisions
[Each: Proposal → Rationale → Files → Risk]

## Atomic Task Plan
- [ ] Phase 1: [Name]
  - [ ] Task 1.1: [Single action] — `src/path/file.rs`
  - [ ] Task 1.2: [Single action] — `src/path/file.rs`
- [ ] Phase 2: [Name]
  - [ ] Task 2.1: [Single action] — `src/path/file.rs`
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

### Agent 2 — Auditor (Challenger)

```
You are a skeptical Senior Rust Engineer auditing an implementation plan for:
"$ARGUMENTS"

Read to validate against:
- /ai/context.md
- /ai/docs/module-architecture.md
- /ai/docs/error-handling.md
- /ai/docs/async-patterns.md
- /ai/docs/code-style.md
- /ai/docs/testing-guidelines.md

For EACH architectural decision, evaluate:
1. Does it violate module layer rules? (higher layers must not import from lower)
2. Are there ownership/borrow issues (sharing &mut across awaits)?
3. Is it async-safe (no blocking in tokio context)?
4. Does it handle errors correctly (no unwrap in production)?
5. Is it DRY (duplicated error construction, repeated env var reads)?
6. Does it follow FACTS? (Feasible, Atomic, Clear, Testable, Scoped)

Verdict per decision:
- ✅ CONFIRM — sound
- ❌ REJECT — violates project conventions (cite doc source)
- ⚠️ MODIFY — right direction, needs adjustment

Output:

## Audit Findings
### Decision: [Name]
**Verdict:** ✅/❌/⚠️
**Reasoning:** [cite /ai/docs/* if relevant]
**Alternative (if ⚠️ or ❌):** [concrete alternative]

## Missing Considerations
[What the Architect missed]

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
| ❌ Rejected (with doc evidence) | Exclude — note why |
| ⚠️ Modified | Use Auditor's version |
| 🔄 Disputed | Present both — ask user |

---

## Phase 3 — Present Final Plan

```markdown
## Final Implementation Plan — [Task Name]

### Synthesis Summary
[What was confirmed / modified / rejected]

### Atomic Task Plan (Validated)
- [ ] Phase 1: [Name]
  - [ ] Task 1.1: [action] — `src/path/file.rs`
- [ ] Phase 2: Tests
  - [ ] Task 2.1: Unit tests — `src/path/file.rs#[cfg(test)]`

### Success Criteria
#### Automated
- [ ] `cargo build` passes with no new errors
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt --check` passes

#### Manual
- [ ] [Observable behavior]

### What We Are NOT Doing
- [Excluded item] — [reason]
```

**Do not start implementation until the user approves the plan.**

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
- `/service [Name]` — new service
- `/model [Name]` — new model/struct
- `/test [Name]` — tests
- `/fix [Bug]` — bug fix
