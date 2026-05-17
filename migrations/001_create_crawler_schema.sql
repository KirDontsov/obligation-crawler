-- ============================================================
-- Obligation Crawler — Database Schema
-- Run once against your PostgreSQL database:
--   psql $DATABASE_URL -f migrations/001_create_crawler_schema.sql
-- ============================================================

-- Enable UUID generation (required for gen_random_uuid())
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================
-- crawler_runs
-- One row per crawl session.
-- Stores when the run started/finished, which URL and config
-- was used, final status, and how many bonds were collected.
-- ============================================================
CREATE TABLE IF NOT EXISTS obligation_crawler_runs (
    id               UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    started_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    finished_at      TIMESTAMPTZ,
    tbank_url        TEXT         NOT NULL,
    headless_chrome  BOOLEAN      NOT NULL DEFAULT FALSE,

    -- Lifecycle: running → completed | failed
    status           TEXT         NOT NULL DEFAULT 'running'
                     CHECK (status IN ('running', 'completed', 'failed')),

    bonds_count      INTEGER      NOT NULL DEFAULT 0,
    error_message    TEXT,

    -- Computed on finish: EXTRACT(EPOCH FROM (finished_at - started_at))::INTEGER
    duration_seconds INTEGER
);

-- ============================================================
-- bonds
-- One row per bond collected during a crawl run.
-- All numeric fields use DOUBLE PRECISION to match Rust f64.
-- Foreign-keyed to crawler_runs — cascade delete keeps the DB
-- clean when a run record is removed.
-- ============================================================
CREATE TABLE IF NOT EXISTS obligation_crawler_bonds (
    id                      UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id                  UUID         NOT NULL
                            REFERENCES obligation_crawler_runs(id) ON DELETE CASCADE,

    -- Identity
    ticker                  TEXT         NOT NULL,
    name                    TEXT         NOT NULL,

    -- Pricing
    price                   DOUBLE PRECISION,          -- ₽, as scraped from T-Bank
    yield_to_maturity       DOUBLE PRECISION,          -- % annualised

    -- Coupon details
    coupon_type             TEXT,                      -- Фиксированный / Переменный / etc.
    next_coupon             TEXT,                      -- DD.MM.YYYY string from T-Bank
    coupon_amount           DOUBLE PRECISION,          -- ₽ per coupon
    accrued_coupon_income   DOUBLE PRECISION,          -- НКД, ₽
    payments_per_year       INTEGER,

    -- Redemption
    maturity                TEXT,                      -- DD.MM.YYYY string from T-Bank
    volume                  BIGINT,                    -- Nominal / lot size, ₽

    -- Risk flags (text as-scraped: "Да" / "Нет")
    subordinated            TEXT,
    amortization            TEXT,
    for_qualified_investors TEXT,

    -- Market data
    change_today            DOUBLE PRECISION,          -- % intraday change

    -- AI analysis produced by opencode CLI (may be NULL if skipped)
    analysis                TEXT,

    created_at              TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- ============================================================
-- Indexes
-- ============================================================

-- Most common query: all bonds for a specific run
CREATE INDEX IF NOT EXISTS idx_obligation_bonds_run_id
    ON obligation_crawler_bonds(run_id);

-- Look up bonds by ticker across runs
CREATE INDEX IF NOT EXISTS idx_obligation_bonds_ticker
    ON obligation_crawler_bonds(ticker);

-- Filter bonds by maturity date (stored as text DD.MM.YYYY)
CREATE INDEX IF NOT EXISTS idx_obligation_bonds_maturity
    ON obligation_crawler_bonds(maturity);

-- Browse runs newest-first
CREATE INDEX IF NOT EXISTS idx_obligation_runs_started_at
    ON obligation_crawler_runs(started_at DESC);

-- Filter runs by lifecycle status
CREATE INDEX IF NOT EXISTS idx_obligation_runs_status
    ON obligation_crawler_runs(status);

-- ============================================================
-- Useful views (optional, for reporting)
-- ============================================================

-- Summary of each run with bond count
CREATE OR REPLACE VIEW obligation_crawler_run_summary AS
SELECT
    r.id,
    r.started_at,
    r.finished_at,
    r.status,
    r.bonds_count,
    r.duration_seconds,
    r.tbank_url,
    r.headless_chrome,
    r.error_message
FROM obligation_crawler_runs r
ORDER BY r.started_at DESC;

-- Latest bond snapshot per ticker (most recent run where ticker appears)
CREATE OR REPLACE VIEW obligation_crawler_latest_bonds AS
SELECT DISTINCT ON (b.ticker)
    b.ticker,
    b.name,
    b.price,
    b.yield_to_maturity,
    b.coupon_type,
    b.next_coupon,
    b.maturity,
    b.volume,
    b.accrued_coupon_income,
    b.coupon_amount,
    b.payments_per_year,
    b.subordinated,
    b.amortization,
    b.for_qualified_investors,
    b.analysis,
    b.created_at,
    b.run_id
FROM obligation_crawler_bonds b
JOIN obligation_crawler_runs r ON b.run_id = r.id
WHERE r.status = 'completed'
ORDER BY b.ticker, b.created_at DESC;
