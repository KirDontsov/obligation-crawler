# Skills Workflow

## Skills

| Skill | Input | Output |
|-------|-------|--------|
| `/research` | feature name | `ai/research/[feature].md` |
| `/plan` | task description | `ai/plans/YYYY-MM-DD-[slug].md` |
| `/service` | service name | `src/services/[name].rs` |
| `/model` | model name | `src/models/[name].rs` |
| `/test` | module or fn name | tests in `#[cfg(test)]` or `tests/` |
| `/fix` | bug description | minimal fix + regression test |
| `/review` | file / empty | `ai/reviews/YYYY-MM-DD-review.md` (optional) |

---

## Documents read by skills

| Document | research | plan | review |
|----------|:--------:|:----:|:------:|
| `ai/context.md` | ✅ | ✅ | ✅ |
| `ai/docs/module-architecture.md` | ✅ | ✅ | ✅ |
| `ai/docs/error-handling.md` | — | ✅ | ✅ |
| `ai/docs/async-patterns.md` | — | ✅ | ✅ |
| `ai/docs/testing-guidelines.md` | — | ✅ | ✅ |
| `ai/docs/code-style.md` | — | ✅ | ✅ |

---

## Common scenarios

**New feature (full cycle)**
```
/research → /plan → /model → /service → /test → /review
```

**Bug fix**
```
/fix → /review
```

**New service from existing plan**
```
/service → /test
```

**Review before merge**
```
/review   (no arguments — looks at current git diff)
```
