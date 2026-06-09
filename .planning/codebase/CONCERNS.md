# Codebase Concerns

**Analysis Date:** 2026-06-09

## Tech Debt

**Logging discipline violated project-wide (CLAUDE.md rule: "`log` macros only, except `main.rs`"):**
- Issue: Production modules use `println!`/`eprintln!` instead of `info!`/`warn!`/`error!`/`debug!`. The crawler emits 28 `println!` calls; only 5 lines use proper `log` macros.
- Files:
  - `src/services/bonds_crawler.rs` — 28 `println!`/`eprintln!` (e.g. lines 21, 100-111, 198-216, 356-439, 459-461, 474, 500, 567), only 5 `log` macros (lines 604, 656, 700, 702)
  - `src/services/rabbitmq_consumer.rs` — 12 `println!`/`eprintln!` (lines 27, 34, 38, 109, 114, 118, 122, 125), zero `log` macros
  - `src/services/rabbitmq_producer.rs` — `println!` at line 36, 60
  - `src/services/opencode_service.rs` — `eprintln!` at lines 23, 31
  - `src/models/bonds.rs` — `println!` at line 65
  - `src/database.rs` — `eprintln!` at line 28
- Impact: No log levels, no structured output, debug spam in production, cannot filter via `RUST_LOG`. Violates the stated convention rule.
- Fix approach: Replace all `println!`→`debug!`/`info!`, `eprintln!`→`warn!`/`error!`. Add `use log::...` to each service module. `main.rs` is exempt by rule.

**Russian comments in code (CLAUDE.md rule: "English comments only"):**
- Issue: Numerous Russian-language comments and debug strings.
- Files: `src/services/bonds_crawler.rs` lines 17, 33, 47, 54, 58, 79, 100-111, 147, 175-177, 196, 199, 218, and the `[DEBUG]` Russian strings throughout; `src/models/bonds.rs` lines 73 (`// Открываем файл в режиме добавления`), CSV headers (lines 47-61) are intentionally Russian (data labels) but comments are not.
- Impact: Violates stated convention; harms maintainability for non-Russian readers.
- Fix approach: Translate all code comments to English. CSV column headers and AI prompt content (`src/services/opencode_service.rs` lines 72-114) are user-facing data and may remain Russian.

**`expect()` in production code (CLAUDE.md rule: "No `unwrap()`/`expect()` in production code"):**
- Issue: `DATABASE_URL` read with `.expect(...)`, which panics if unset.
- Files: `src/database.rs:7`
- Impact: Process panics instead of returning a `Result`. `main.rs:28` already wraps `create_connection_pool()` in a `match` expecting graceful degradation ("continuing without DB"), but the `expect` panics *before* that fallback can engage — the graceful-degradation path is dead code when `DATABASE_URL` is missing.
- Fix approach: Replace with `env::var("DATABASE_URL").map_err(|_| sqlx::Error::Configuration(...))?` so the `main.rs:33` fallback works as intended.

**Env vars read directly inside `database.rs` (CLAUDE.md rule: "Never read env vars in services — inject via `CrawlerConfig`"):**
- Issue: `DATABASE_URL` read via `env::var` inside `create_connection_pool`. `RABBITMQ_URL`, `RABBITMQ_EXCHANGE`, `RABBITMQ_QUEUE`, `ENABLE_RABBITMQ`, `DURATION_MINUTES`, `RUN_MODE` are read in `main.rs` (acceptable) but RabbitMQ/DB settings never enter `CrawlerConfig`.
- Files: `src/database.rs:6`, `src/main.rs:39,58,62,68-71,179-183`
- Impact: Configuration scattered, not centralized in `CrawlerConfig` as the architecture mandates. `CrawlerConfig` (`src/config.rs`) lacks fields for DB URL, RabbitMQ URL/exchange/queue, run mode, WebDriver URL.
- Fix approach: Extend `CrawlerConfig` (`src/config.rs:13`) with `database_url`, `rabbitmq_url`, `rabbitmq_exchange`, `rabbitmq_queue`, `run_mode`, `webdriver_url`; inject everywhere.

**Hardcoded WebDriver URL ignores configured `chrome_driver_path`:**
- Issue: `let webdriver_url = format!("http://localhost:9515");` is hardcoded, and `format!` with no args triggers a clippy `useless_format` warning (fails `cargo clippy -- -D warnings`).
- Files: `src/services/bonds_crawler.rs:493`
- Impact: `CrawlerConfig.chrome_driver_path` (`src/config.rs:17`, default `./chromedriver`) is never used — dead config. WebDriver host/port cannot be changed without recompiling. Clippy build gate likely failing.
- Fix approach: Add a `webdriver_url` config field; replace `format!` with a plain `String` or config value.

**Unused/dead public API surface:**
- Issue: `src/controllers/bonds_crawler.rs` (`run_bonds_crawler`, `collect_bonds_once`), `src/api/bonds.rs` (`BondsApiResponse`, `BondsResponse`), `src/shared/utils.rs` (all four helpers), `CrawlerConfig::new` (`src/config.rs:64`), and repository methods `get_bonds_for_run`/`get_recent_runs` (`src/repository/bonds_repository.rs:148,163`) appear unreferenced by `main.rs` dispatch.
- Files: `src/controllers/bonds_crawler.rs`, `src/api/bonds.rs`, `src/shared/utils.rs`, `src/config.rs:64`
- Impact: Dead code, maintenance overhead, likely `dead_code` warnings.
- Fix approach: Either wire these into the run modes or remove them.

## Known Bugs

**`max_retries` config is never used:**
- Symptoms: `CrawlerConfig.max_retries` (`src/config.rs:19`, default 3) is parsed from env but never read anywhere. No retry logic exists in the crawler.
- Files: `src/config.rs:49`, no consumers
- Trigger: Set `MAX_RETRIES`; value silently ignored.
- Workaround: None needed (no behavior change), but the config implies a feature that does not exist.

**`finish_crawl_run` only ever marked "completed"; failures never recorded:**
- Symptoms: `run_crawl_loop` (`src/services/bonds_crawler.rs:694-704`) always calls `finish_crawl_run(..., "completed", None)`. Crawl errors inside the loop are swallowed (`Err(_) => {}` at lines 678, 681, 633) so a failed run is still recorded as completed with whatever bonds were collected.
- Files: `src/services/bonds_crawler.rs:672-704`
- Trigger: Any scraping error mid-run.
- Workaround: None. The `error_message` column and "failed"/"error" status are never written.

**`run_crawl_loop` never terminates when `duration_minutes` is `None`:**
- Symptoms: The outer `loop` (`src/services/bonds_crawler.rs:672`) only breaks on `start_time.elapsed() > duration*60` (lines 684-688). In `direct` mode `DURATION_MINUTES` is optional (`src/main.rs:58`); if unset, the crawler loops forever re-scraping every `poll_interval_seconds`, never reaching the `finish_crawl_run`/`close` code, and `bonds` accumulates duplicates across iterations via `total_bonds.extend(bonds)` (line 676).
- Files: `src/services/bonds_crawler.rs:672-691`, `src/main.rs:58-60`
- Trigger: Run `direct` mode without `DURATION_MINUTES`.
- Workaround: Always set `DURATION_MINUTES`.

**Duplicate bond accumulation across poll iterations:**
- Symptoms: Each loop iteration calls `collect_bonds()` from scratch and extends `total_bonds`, so multi-iteration runs duplicate every bond in memory, in the returned vec, in RabbitMQ payload, and in DB (`save_bond` has no dedup/upsert).
- Files: `src/services/bonds_crawler.rs:672-691`, `src/repository/bonds_repository.rs:98` (plain INSERT, no `ON CONFLICT`)
- Trigger: Any run lasting more than one poll interval.
- Workaround: Single-iteration via short duration.

**`final_maturity` always prefers the (possibly empty) list-page string:**
- Symptoms: `let final_maturity = Some(maturity_date.clone()).or(details.maturity);` (`src/services/bonds_crawler.rs:149`). Because `Some(...)` is always `Some`, `.or()` never falls through to the richer detail-page `details.maturity`. If the list page yielded an empty string, maturity is `Some("")`.
- Files: `src/services/bonds_crawler.rs:149`
- Trigger: List-page maturity cell empty/malformed.
- Workaround: None. Logic bug — should be `if maturity_date.is_empty() { details.maturity } else { Some(maturity_date) }`.

**`skip_analysis` semantics inverted vs. comment:**
- Symptoms: Comment (lines 175-177) says skip when "maturity < 1 year" OR "price > nominal by >5". The code's first branch returns `date < one_year_later` (true when maturity is within a year — correct), but on parse failure returns `false` (line 187) meaning unparseable dates are always analyzed. Combined with the `final_maturity` bug above, a `Some("")` maturity fails parse → never skipped.
- Files: `src/services/bonds_crawler.rs:178-194`
- Trigger: Malformed maturity dates.
- Workaround: None.

## Security Considerations

**Credentials in env vars / default guest RabbitMQ creds:**
- Risk: `RABBITMQ_URL` defaults to `amqp://guest:guest@localhost:5672` (`src/main.rs:69,180`). `DATABASE_URL` carries DB credentials. These are logged in plaintext on connect.
- Files: `src/services/rabbitmq_consumer.rs:27` (`println!("Connecting to RabbitMQ at: {}", self.connection_string)` — prints full URL including credentials), `src/main.rs:185`
- Current mitigation: `.env` via `dotenv` (file present, not committed — verify `.gitignore`).
- Recommendations: Never log connection strings containing credentials; redact before logging. Remove the `guest:guest` default or fail loudly in production. Confirm `.env` is gitignored.

**AI prompt built via string interpolation of scraped data:**
- Risk: `build_prompt` (`src/services/opencode_service.rs:41-117`) interpolates scraped `bond.name`/`ticker` directly into the prompt passed to the external `opencode` CLI. Scraped values are attacker-influenceable (the website content) and could carry prompt-injection payloads.
- Files: `src/services/opencode_service.rs:9-12` (`Command::new("opencode").arg("run").arg(&prompt)`)
- Current mitigation: Args passed as separate argv (no shell), so no shell-injection — good. But prompt-injection into the LLM is unmitigated.
- Recommendations: Sanitize/escape scraped text in prompts; treat AI output as untrusted before persisting to DB/CSV.

**`window.open` with interpolated href:**
- Risk: `driver.execute(&format!("window.open('{}', '_blank')", href), ...)` (`src/services/bonds_crawler.rs:128`) interpolates a scraped `href` into JS. A crafted href containing `')` could break out of the string and execute arbitrary JS in the browser context.
- Files: `src/services/bonds_crawler.rs:120-129`
- Current mitigation: None.
- Recommendations: Pass `href` as a script argument (`driver.execute("window.open(arguments[0],'_blank')", vec![json!(href)])`) instead of interpolating.

## Performance Bottlenecks

**Synchronous blocking subprocess in async context:**
- Problem: `analyze_bond` uses synchronous `std::process::Command::output()` (`src/services/opencode_service.rs:9-18`) and is called via plain `analyze_bond(&bond_result)` (`src/services/bonds_crawler.rs:207`) inside an async fn. This blocks the Tokio worker thread for the full duration of the LLM call (potentially many seconds), starving the runtime.
- Files: `src/services/opencode_service.rs:9`, `src/services/bonds_crawler.rs:207`
- Cause: Blocking syscall on an async executor thread; no `tokio::process::Command` or `spawn_blocking`.
- Improvement path: Use `tokio::process::Command` and `await`, or wrap in `tokio::task::spawn_blocking`.

**Per-row new-tab navigation with fixed sleeps:**
- Problem: Each bond opens a new browser window, sleeps 1s + 2s + 0.5s (`src/services/bonds_crawler.rs:131,138,145`), scrapes, closes. For 50 pages × N rows this is extremely slow and entirely serial.
- Files: `src/services/bonds_crawler.rs:115-145`
- Cause: Fixed `sleep` instead of explicit waits; serial tab-per-bond.
- Improvement path: Replace fixed sleeps with WebDriver explicit waits on element presence; consider scraping detail data without opening a new tab where possible.

**DB connection pool sized at max 5 but writes are serial:**
- Problem: `save_bond` is called sequentially per row (`src/services/bonds_crawler.rs:602-606`); pool capacity (`src/database.rs:10`) is unused concurrency.
- Files: `src/database.rs:9-17`
- Cause: Architecture is single-threaded scrape loop.
- Improvement path: Acceptable for now; no fix needed unless throughput becomes an issue.

## Fragile Areas

**Hardcoded obfuscated CSS class selectors (highest-risk fragility):**
- Files: `src/services/bonds_crawler.rs:35,42,49,60,81` — selectors like `.SecurityRow__showName_inlal`, `.SecurityRow__ticker_KMm7A`, `.BondsTable__dateToClient_LjMTe`, `.Money-module__money_UZBbh`.
- Why fragile: These are CSS-modules hashed class names (the `_inlal`, `_KMm7A` suffixes regenerate on every frontend build). Any T-Bank deploy silently breaks parsing — `find` returns `Err`, fields fall back to `unwrap_or_default()` (empty), and bonds are saved with blank data rather than erroring.
- Safe modification: Prefer stable `data-qa-*`/`data-qa-type` attributes (already used for table/rows at lines 526, 555, 562, 250, 259) over hashed classes. Add a sanity check that aborts the run if >X% of rows yield empty names.
- Test coverage: Zero. No parsing tests exist.

**Label-text matching against space-stripped Russian strings:**
- Files: `src/services/bonds_crawler.rs:286-351` — matches like `label_str.contains("Накопленныйкупонныйдоход")` after stripping all spaces.
- Why fragile: Any wording change, added punctuation, or different whitespace handling breaks the match silently (field stays `None`).
- Safe modification: Externalize label constants; log when a known label table yields zero matches.
- Test coverage: None.

**RabbitMQ consumer reconnect loop swallows handler errors and never NACKs:**
- Files: `src/services/rabbitmq_consumer.rs:111-129`
- Why fragile: On handler error (line 117-119) it logs but still `ack`s (line 121, `let _ = delivery.ack(...)`) — message lost on failure, no requeue/DLQ. The `ack` result is discarded. Inner `while` breaks on delivery error and falls through to a 5s sleep then full reconnect (lines 124-131), re-declaring queue each time.
- Safe modification: NACK with requeue on handler failure; propagate ack errors; bound reconnect attempts with backoff.
- Test coverage: None.

**RabbitMQ producer has no reconnect; connection held for whole run:**
- Files: `src/services/rabbitmq_producer.rs:6-46`
- Why fragile: `RabbitMQProducer` opens connection at startup (`src/main.rs:73`) and only publishes once at the very end (`src/main.rs:166-170`). If the connection drops during a long crawl, the final publish fails with no retry, losing all results.
- Safe modification: Publish incrementally, or reconnect-on-publish, or move publish before the long-lived crawl is not possible — add retry around `publish`.
- Test coverage: None.

**WebDriver lifecycle / `Drop` spawns detached cleanup:**
- Files: `src/services/bonds_crawler.rs:756-767`
- Why fragile: `Drop` spawns a detached `tokio::spawn(d.quit())`; if the runtime is shutting down the task may never run, leaking the chromedriver session/browser process. `close()` (line 709) is the intended path but is skipped on any early `?` return in `run_direct_mode`.
- Safe modification: Ensure `close()` runs on all exit paths (e.g. wrap crawl body and call `close` in cleanup regardless of error). Avoid relying on `Drop` for async cleanup.
- Test coverage: None.

**External `opencode` binary hard dependency:**
- Files: `src/services/opencode_service.rs:9`
- Why fragile: Assumes `opencode` is on `PATH`. If absent, every bond analysis fails (`Failed to execute opencode`), but failures are only logged (`src/services/bonds_crawler.rs:212-214`) and the bond is saved with `analysis: None` — silent degradation. No version pinning, no timeout (a hung CLI blocks the thread indefinitely).
- Safe modification: Add a startup preflight check for the binary; add a subprocess timeout; make the dependency optional via config.
- Test coverage: None.

**`nth-of-type` row re-selection assumption:**
- Files: `src/services/bonds_crawler.rs:578-597`
- Why fragile: Re-finds rows via `:nth-of-type(idx+1)` after navigating away and back. If the table re-renders/reorders/lazy-loads between iterations, the wrong row is parsed. Fallback re-fetches all rows by index.
- Safe modification: Capture all hrefs up front, then iterate hrefs rather than live elements.
- Test coverage: None.

## Scaling Limits

**Hardcoded `max_pages = 50`:**
- Current capacity: Caps crawl at 50 "show more"/pagination clicks (`src/services/bonds_crawler.rs:546`).
- Limit: Silently stops at 50 pages; larger listings truncated with no warning.
- Scaling path: Make configurable via `CrawlerConfig`; warn when cap is hit.

**All bonds held in memory + serialized to one JSON blob:**
- Current capacity: `total_bonds: Vec<BondListItem>` and a single `serde_json::to_string(&bonds)` (`src/main.rs:167`) for the entire run published as one RabbitMQ message.
- Limit: Large runs produce oversized messages (RabbitMQ frame/size limits) and high memory.
- Scaling path: Stream/batch publishing per page.

## Dependencies at Risk

**`chrono` deprecated API:**
- Risk: `NaiveDateTime::from_timestamp_opt` (`src/shared/utils.rs:17`) is deprecated in chrono 0.4.x.
- Impact: Deprecation warnings; will break on future major bump. (Module is also unused dead code.)
- Migration plan: Use `DateTime::from_timestamp(ts, 0)`.

**`thirtyfour` 0.32 pinned to a fragile target:**
- Risk: WebDriver/Chrome version coupling; `thirtyfour` API churns between versions.
- Impact: Browser/driver mismatch breaks scraping at runtime.
- Migration plan: Pin chromedriver + Chrome versions; document required versions.

**`reqwest`, `regex`, `urlencoding`, `time` appear unused:**
- Risk: `reqwest` (and its `CrawlerError::RequestError` variant `src/error.rs:26`), `regex`, `urlencoding` are declared in `Cargo.toml` but no source references found; `time` is a dev-dependency with no test using it.
- Impact: Unnecessary build time / attack surface.
- Migration plan: Remove unused dependencies after confirming.

## Missing Critical Features

**No "failed" run recording / error propagation to DB:**
- Problem: As noted, runs always recorded "completed"; `error_message`/"failed" status never used (`src/services/bonds_crawler.rs:694-704`).
- Blocks: Operational visibility into failed crawls.

**No retry logic despite `max_retries` config:**
- Problem: `max_retries` exists but no retries implemented anywhere.
- Blocks: Resilience to transient WebDriver/network failures.

**Consumer does nothing with messages:**
- Problem: `run_consumer_mode` handler just prints the message (`src/main.rs:190-195`); no processing, no crawl trigger.
- Blocks: The consumer mode has no functional purpose yet.

**No graceful shutdown / signal handling:**
- Problem: The infinite `run_crawl_loop` and consumer loop have no SIGINT/SIGTERM handling; abrupt termination leaks browser/DB/RabbitMQ resources.
- Blocks: Clean operation in containers.

## Test Coverage Gaps

**Effectively zero meaningful test coverage:**
- What's not tested: The entire scraping pipeline (`parse_bond_row_inner`, `collect_bond_details_inner`, `try_click_show_more`), all parsing/cleaning logic (numeric `replace`/`parse` chains), `skip_analysis` logic, `final_maturity`/`final_yield` merge, RabbitMQ producer/consumer, opencode integration, `CrawlerConfig::from_env`, DB repository SQL.
- Files: Only one trivial test exists — `src/repository/bonds_repository.rs:181-209` (`should_have_all_bond_record_fields`), a compile-time struct-shape assertion that exercises no logic.
- Risk: All the fragile parsing/merge bugs above (lines 149, 178-194) and selector breakage would go completely undetected. Per CLAUDE.md the build gate is `cargo test`, but there is nothing substantive to catch regressions.
- Priority: High — extract pure parsing helpers (text-cleaning, date-skip logic, label matching) from WebDriver calls so they can be unit-tested without a browser; add config parsing tests; add DB integration tests behind a feature flag.

---

*Concerns audit: 2026-06-09*
