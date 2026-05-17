# postgres-integration — Research

**Date:** 2026-05-17
**Status:** Complete

---

## Problem Statement

### Need
Each crawl run should be persisted in PostgreSQL so results are queryable across time, not just available as CSV files. Two tables: one run record (config + status) and N bond records linked to it by FK.

### Success Criteria
- [ ] Each `run_crawl_loop()` creates a `crawler_runs` row at start, marks it `completed`/`failed` at end
- [ ] Each bond parsed is saved to `bonds` immediately (same pattern as CSV append)
- [ ] DB is optional: `Option<PgPool>` — if pool is `None`, crawler runs normally without DB
- [ ] `main.rs` creates pool at startup, passes it down (same as reference `crawler` project)
- [ ] Errors on DB save are logged (`warn!`) but do NOT abort the crawl

---

## Reference Pattern Found

**File:** `/Users/kirilldoncov/Documents/rust/crawler/src/controllers/avito_analytics/avito_analytics_db.rs`

Pattern:
- `AdRecord` struct with `#[derive(sqlx::FromRow, Debug)]`
- `save_to_db(&self, pool: &PgPool) -> Result<Uuid, sqlx::Error>` on the struct
- Uses `sqlx::query()` (not macro) with positional `$N` params and `.bind()`
- `RETURNING id` to get the generated UUID back

**File:** `/Users/kirilldoncov/Documents/rust/crawler/src/main.rs`

Pattern:
- Pool created once at startup with graceful fallback: `None` if DB unavailable
- `Option<PgPool>` passed by value down to handlers
- `db_pool.clone()` when passing into closures or multiple call sites

---

## Proposed File Structure

```
migrations/
  001_create_crawler_schema.sql   ← SQL to run once

src/repository/
  mod.rs                          ← pub mod + re-exports
  bonds_repository.rs             ← CrawlRun, BondRecord structs + save fns

src/main.rs                       ← add pool init + pass to run_direct_mode
src/services/bonds_crawler.rs     ← add db_pool/run_id fields, call repository
.env.example                      ← DATABASE_URL already present ✅
```

---

## Dependency Status

| Item | Type | Status |
|------|------|--------|
| `sqlx` with postgres | Cargo.toml | ✅ Already present (0.8) |
| `uuid` | Cargo.toml | ✅ Already present (1.4) |
| `chrono` | Cargo.toml | ✅ Already present |
| `repository` module | Internal | ❌ Create |
| `crawler_runs` table | DB | ❌ Create via migration |
| `bonds` table | DB | ❌ Create via migration |

---

## FAR Validation
- [x] **Factual** — based on actual code in both projects
- [x] **Actionable** — exact files and patterns identified
- [x] **Relevant** — solves persistence need across crawl sessions
