# postgres-integration — Implementation Plan

**Date:** 2026-05-17
**Status:** Approved

---

## Atomic Task Plan

- [x] Phase 1: SQL Schema
  - [x] Task 1.1: Create `migrations/001_create_crawler_schema.sql` — `crawler_runs` + `bonds` tables + indexes + views
- [x] Phase 2: Repository Layer
  - [x] Task 2.1: Create `src/repository/mod.rs`
  - [x] Task 2.2: Create `src/repository/bonds_repository.rs` — `CrawlRun`, `BondRecord` structs + `create_crawl_run`, `finish_crawl_run`, `save_bond` async fns
- [x] Phase 3: Wire into BondsCrawler
  - [x] Task 3.1: Add `db_pool: Option<PgPool>`, `run_id: Option<Uuid>` fields to `BondsCrawler`
  - [x] Task 3.2: Change `BondsCrawler::new()` signature to accept `db_pool: Option<PgPool>`
  - [x] Task 3.3: In `run_crawl_loop()` — call `create_crawl_run` before loop, `finish_crawl_run` after
  - [x] Task 3.4: In `collect_bonds()` — call `save_bond` after each successful bond parse
- [x] Phase 4: Wire into main.rs
  - [x] Task 4.1: Init DB pool at startup in `main()` (graceful `Option<PgPool>`)
  - [x] Task 4.2: Change `run_direct_mode()` to accept `db_pool: Option<PgPool>`, pass to `BondsCrawler::new()`

---

## Success Criteria

### Automated
- [ ] `cargo build` passes with no new errors
- [ ] `cargo clippy -- -D warnings` clean

### Manual
- [ ] Run crawler, check `crawler_runs` table: one row with `status='completed'`
- [ ] Check `bonds` table: N rows linked to that run_id
- [ ] Kill crawler mid-run: `crawler_runs` row stays `status='running'` (expected — no crash handler yet)
- [ ] Set `DATABASE_URL` to invalid: crawler starts without DB, logs `⚠️`, completes normally via CSV

---

## What We Are NOT Doing
- No `sqlx migrate run` integration — migration is a one-time manual SQL script
- No `status='failed'` on crawler crash (would require panic hook — future work)
- No consumer mode DB integration — only direct mode for now
- No `UPDATE bonds` (no deduplication by ticker — each run creates fresh rows)
