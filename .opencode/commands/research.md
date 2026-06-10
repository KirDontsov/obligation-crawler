---
description: Research a new feature using RPI strategy before implementation. Ask clarifying questions one at a time, spawn analysis agent, create research doc at ai/research/[feature].md. Use before /plan.
---

# Research Template (RPI Strategy)

**Feature:** $ARGUMENTS

## Context
- Branch: !`git branch --show-current`
- Date: !`date +%Y-%m-%d`
- Existing research docs: !`ls ai/research/ 2>/dev/null || echo "none"`
- Existing services: !`ls src/services/ 2>/dev/null`
- Existing models: !`ls src/models/ 2>/dev/null`

---

## Step 1 — Reverse Prompting

Ask these questions **ONE AT A TIME**, wait for each answer:

1. What problem does **$ARGUMENTS** solve in bond crawling context?
2. Which existing services/modules will it interact with?
3. Are there similar patterns already in the codebase?
4. What are success criteria (output format, performance, reliability)?

After collecting answers, proceed to Step 2.

---

## Step 2 — Codebase Analysis

Spawn an analysis agent:

```
Analyze the Rust codebase to research: "$ARGUMENTS"

Read project docs first:
- /ai/context.md
- /ai/docs/module-architecture.md
- /ai/docs/async-patterns.md
- /ai/docs/error-handling.md

Then read these:
1. src/services/ — list files, read similar ones
2. src/models/ — read model structs
3. src/config.rs — config pattern
4. src/error.rs — error variants
5. src/main.rs — run modes and orchestration

For each finding output:

### [Pattern/File Name]
**File:** `src/path/to/file.rs`
**Pattern demonstrated:** [what code shows]
**Reuse for $ARGUMENTS:** [how to follow]
**Key snippet:** [relevant code]

At the end output:

## Dependency Status
| Item | Type | Status | Notes |
|------|------|--------|-------|
| New struct/model | Internal | Exists / Create | [location] |
| New service | Internal | Exists / Create | [name] |
| New error variant | Internal | Exists / Create | [variant] |
| Config field | Internal | Exists / Create | [field] |
| External crate | External | Available / Add | [crate] |

## Proposed File Structure
[Based on existing patterns]
```

---

## Step 3 — Create Research Document

After agent completes, create `ai/research/$ARGUMENTS.md`:

```markdown
# $ARGUMENTS — Research

**Date:** [date]
**Status:** Draft

---

## Problem Statement

### Need
[From Step 1]

### Success Criteria
- [ ] [criterion]

---

## Existing Patterns

### Patterns to Follow
| Pattern | File | Relevance |
|---------|------|-----------|
| [pattern] | `src/path/file.rs` | High/Med |

### Dependency Status
| Item | Type | Status |
|------|------|--------|
| [item] | External/Internal | Available/Create |

---

## Proposed File Structure
src/services/[name].rs — new service
src/models/[name].rs — new model (if needed)
src/error.rs — add variant(s)
src/config.rs — add field(s)
.env.example — document new env vars

---

## FAR Validation

- [ ] **Factual** — based on actual code, not assumptions
- [ ] **Actionable** — clear what to create and patterns to follow
- [ ] **Relevant** — solves the identified need

---

## Open Questions

[Any unresolved questions before planning]
```

---

## Step 4 — Review

> "Research doc saved to `ai/research/$ARGUMENTS.md`. Looks correct? Adjust before planning?"

---

## Step 5 — Handoff

Once confirmed:
> "Research complete: `ai/research/$ARGUMENTS.md`. Start fresh context and run `/plan $ARGUMENTS`"