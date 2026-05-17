---
name: research
description: >-
  Research a new feature using the RPI strategy before any implementation begins.
  Asks clarifying questions one at a time (Reverse Prompting), spawns an analysis
  agent to explore existing codebase patterns, and creates a research document at
  ai/research/[feature].md. Use before /plan for any non-trivial feature.
  Trigger when the user wants to research a feature, understand existing patterns,
  explore the codebase for similar implementations, or create a research document.
  Trigger on phrases like "research X", "what patterns exist for X", "how is X done
  in this project", "analyze before building X", "look into X". Does NOT write code.
user-invocable: true
argument-hint: "[FeatureName]"
model: sonnet
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

Ask the following questions **ONE AT A TIME** — wait for each answer before asking the next.

1. What problem does **$ARGUMENTS** solve in the context of bond crawling?
2. Which existing services or modules will it interact with?
3. Are there similar patterns already in the codebase to follow?
4. What are the success criteria (output format, performance, reliability)?

After all four answers are collected, proceed to Step 2.

---

## Step 2 — Codebase Analysis

Spawn an **analysis agent** with this exact prompt:

```
Analyze the Rust codebase to research the implementation of: "$ARGUMENTS"

Read these project docs first:
- /ai/context.md
- /ai/docs/module-architecture.md
- /ai/docs/async-patterns.md
- /ai/docs/error-handling.md

Then read these source locations:
1. src/services/ — list all service files, read the most similar ones fully
2. src/models/ — read all model structs
3. src/config.rs — understand config pattern
4. src/error.rs — understand error variants
5. src/main.rs — understand run modes and orchestration

For each finding output:

### [Pattern/File Name]
**File:** `src/path/to/file.rs`
**Pattern demonstrated:** [what this code shows]
**Reuse for $ARGUMENTS:** [how to follow/reuse this]
**Key snippet:**
[relevant code snippet]

At the end output:

## Dependency Status
| Item | Type | Status | Notes |
|------|------|--------|-------|
| New struct/model | Internal | Exists / Create | [location if exists] |
| New service | Internal | Exists / Create | [service name] |
| New error variant | Internal | Exists / Create | [variant name] |
| Config field | Internal | Exists / Create | [field name] |
| External crate | External | Available / Add to Cargo.toml | [crate name] |

## Proposed File Structure
[Based on existing patterns — list files to create/modify]
```

---

## Step 3 — Create Research Document

After the agent completes, synthesize findings with Step 1 answers and create `ai/research/$ARGUMENTS.md`:

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
| [pattern] | `src/path/to/file.rs` | High/Med |

### Dependency Status
| Item | Type | Status |
|------|------|--------|
| [item] | External/Internal | ✅ Available / ❌ Create |

---

## Proposed File Structure

```
src/services/[name].rs       — new service
src/models/[name].rs         — new model (if needed)
src/error.rs                 — add variant(s)
src/config.rs                — add config field(s)
.env.example                 — document new env vars
```

---

## FAR Validation

- [ ] **Factual** — based on actual code analysis, not assumptions
- [ ] **Actionable** — clear what files to create and patterns to follow
- [ ] **Relevant** — solves the identified need

---

## Open Questions

[Any unresolved questions before planning]
```

---

## Step 4 — Review & Iterate

After creating the document:
> "Research doc saved to `ai/research/$ARGUMENTS.md`. Does this look correct? Anything to adjust before we move to planning?"

---

## Step 5 — Handoff

Once confirmed:
> ✅ Research complete: `ai/research/$ARGUMENTS.md`
>
> Start a **fresh context** and run:
> ```
> /plan $ARGUMENTS
> ```
