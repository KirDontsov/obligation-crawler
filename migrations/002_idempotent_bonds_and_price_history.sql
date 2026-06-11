-- ============================================================
-- Obligation Crawler — Migration 002
-- Idempotent bond refresh: one row per ticker + price history.
-- Run once against your PostgreSQL database:
--   psql $DATABASE_URL -f migrations/002_idempotent_bonds_and_price_history.sql
--
-- This migration is idempotent and safe to re-run. Parts of it may already
-- have been applied manually (e.g. dedup + UNIQUE(ticker)); every step is
-- guarded so re-application is a no-op.
-- ============================================================

BEGIN;

-- ------------------------------------------------------------
-- 1. Dedup before UNIQUE: collapse duplicate rows per ticker,
--    keeping the most recent one (created_at DESC, id DESC tie-break).
--    No-op if duplicates were already removed.
-- ------------------------------------------------------------
DELETE FROM obligation_crawler_bonds
WHERE id IN (
    SELECT id FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                   PARTITION BY ticker
                   ORDER BY created_at DESC, id DESC
               ) AS rn
        FROM obligation_crawler_bonds
    ) t
    WHERE t.rn > 1
);

-- ------------------------------------------------------------
-- 2. updated_at: last-touched timestamp. created_at stays as the
--    first-seen time. Added + backfilled only on first run.
-- ------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'obligation_crawler_bonds'
          AND column_name = 'updated_at'
    ) THEN
        ALTER TABLE obligation_crawler_bonds
            ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
        -- Backfill existing rows so updated_at is meaningful from the start.
        UPDATE obligation_crawler_bonds SET updated_at = created_at;
    END IF;
END $$;

-- ------------------------------------------------------------
-- 3. UNIQUE(ticker): target key for INSERT ... ON CONFLICT (ticker).
--    Guarded so it does not error if already present.
-- ------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'obligation_crawler_bonds_ticker_key'
    ) THEN
        ALTER TABLE obligation_crawler_bonds
            ADD CONSTRAINT obligation_crawler_bonds_ticker_key UNIQUE (ticker);
    END IF;
END $$;

-- ------------------------------------------------------------
-- 4. price_history: append-only trend log. A row is written only
--    when a bond's price (and/or coupon) changes on a refresh.
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS obligation_crawler_price_history (
    id            UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    ticker        TEXT         NOT NULL,
    price         DOUBLE PRECISION,
    coupon_amount DOUBLE PRECISION,
    run_id        UUID         NOT NULL
                  REFERENCES obligation_crawler_runs(id) ON DELETE CASCADE,
    recorded_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Fetch a ticker's price trend newest-first.
CREATE INDEX IF NOT EXISTS idx_price_history_ticker_recorded
    ON obligation_crawler_price_history(ticker, recorded_at DESC);

-- ------------------------------------------------------------
-- 5. Redefine the latest-bonds view for the current-state model.
--
--    With one row per ticker (UNIQUE above), each bonds row IS the
--    latest good scrape for that ticker, so DISTINCT ON is no longer
--    needed. The JOIN to obligation_crawler_runs and the
--    `status = 'completed'` filter are DROPPED on purpose: an upsert
--    repoints a row's run_id to the currently-'running' run, and the
--    old filter would have hidden the bond from the API for the whole
--    duration of every crawl (visibility regression). The downstream
--    API (obligation-api) only ever SELECTs from this view and never
--    reads obligation_crawler_runs / status, so dropping the gate is
--    safe for the read contract. created_at and run_id are taken
--    directly from the bonds row.
--
--    The view MUST keep returning exactly these 17 columns in this
--    order — it is the read contract for obligation-api (LatestBond).
-- ------------------------------------------------------------
CREATE OR REPLACE VIEW obligation_crawler_latest_bonds AS
SELECT
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
ORDER BY b.ticker;

COMMIT;
