use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::bonds::BondListItem;

/// Persisted representation of a single crawl session.
#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug)]
pub struct ObligationCrawlerRun {
	pub id: Uuid,
	pub started_at: DateTime<Utc>,
	pub finished_at: Option<DateTime<Utc>>,
	pub tbank_url: String,
	pub headless_chrome: bool,
	pub status: String,
	pub bonds_count: i32,
	pub error_message: Option<String>,
	pub duration_seconds: Option<i32>,
}

/// Persisted representation of a single bond collected during a run.
#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug)]
pub struct BondRecord {
	pub id: Uuid,
	pub run_id: Uuid,
	pub ticker: String,
	pub name: String,
	pub price: Option<f64>,
	pub yield_to_maturity: Option<f64>,
	pub coupon_type: Option<String>,
	pub next_coupon: Option<String>,
	pub coupon_amount: Option<f64>,
	pub accrued_coupon_income: Option<f64>,
	pub payments_per_year: Option<i32>,
	pub maturity: Option<String>,
	pub volume: Option<i64>,
	pub subordinated: Option<String>,
	pub amortization: Option<String>,
	pub for_qualified_investors: Option<String>,
	pub change_today: Option<f64>,
	pub analysis: Option<String>,
	pub created_at: DateTime<Utc>,
}

/// Outcome of an idempotent `save_bond` upsert. Lets callers tally per-run counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveOutcome {
	Created,
	Updated,
	Unchanged,
}

pub struct BondsRepository;

impl BondsRepository {
	/// Inserts a new `crawler_runs` row with `status='running'` and returns the generated UUID.
	pub async fn create_crawl_run(
		pool: &PgPool,
		tbank_url: &str,
		headless_chrome: bool,
	) -> Result<Uuid, sqlx::Error> {
		let row = sqlx::query(
			"INSERT INTO obligation_crawler_runs (tbank_url, headless_chrome, status)
             VALUES ($1, $2, 'running')
             RETURNING id",
		)
		.bind(tbank_url)
		.bind(headless_chrome)
		.fetch_one(pool)
		.await?;

		let id: Uuid = row.get("id");
		Ok(id)
	}

	/// Updates a `crawler_runs` row on completion: sets `finished_at`, `bonds_count`,
	/// `status`, `error_message`, and computes `duration_seconds`.
	pub async fn finish_crawl_run(
		pool: &PgPool,
		run_id: Uuid,
		bonds_count: i32,
		status: &str,
		error_message: Option<&str>,
	) -> Result<(), sqlx::Error> {
		sqlx::query(
			"UPDATE obligation_crawler_runs
             SET finished_at      = NOW(),
                 bonds_count      = $2,
                 status           = $3,
                 error_message    = $4,
                 duration_seconds = EXTRACT(EPOCH FROM (NOW() - started_at))::INTEGER
             WHERE id = $1",
		)
		.bind(run_id)
		.bind(bonds_count)
		.bind(status)
		.bind(error_message)
		.execute(pool)
		.await?;

		Ok(())
	}

	/// Upserts a single bond keyed by `ticker` (relies on the UNIQUE(ticker) constraint),
	/// so a repeated run refreshes the existing row instead of erroring on a duplicate key.
	///
	/// Runs SELECT-then-upsert in one transaction so the caller learns whether the bond was
	/// Created / Updated / Unchanged, and so a `price_history` row is written only when the
	/// price or coupon actually changed.
	///
	/// On conflict it deliberately does NOT update:
	/// - `ticker`     — the conflict key
	/// - `created_at` — preserves first-seen time
	/// - `analysis`   — owned by the downstream API; the crawler must not clobber it
	///
	/// `run_id` IS repointed to the current run. This is safe because the
	/// `obligation_crawler_latest_bonds` view no longer filters on run status (migration 002),
	/// so the bond stays visible to the API mid-run.
	pub async fn save_bond(
		pool: &PgPool,
		run_id: Uuid,
		bond: &BondListItem,
	) -> Result<SaveOutcome, sqlx::Error> {
		let mut tx = pool.begin().await?;

		// Snapshot the volatile fields before the upsert to classify the outcome and to
		// decide whether a price_history point is warranted.
		let previous = sqlx::query(
			"SELECT price, coupon_amount FROM obligation_crawler_bonds WHERE ticker = $1",
		)
		.bind(&bond.ticker)
		.fetch_optional(&mut *tx)
		.await?
		.map(|row| {
			let price: Option<f64> = row.get("price");
			let coupon_amount: Option<f64> = row.get("coupon_amount");
			(price, coupon_amount)
		});

		sqlx::query(
			"INSERT INTO obligation_crawler_bonds (
                run_id,
                ticker, name,
                price, yield_to_maturity,
                coupon_type, next_coupon, coupon_amount, accrued_coupon_income, payments_per_year,
                maturity, volume,
                subordinated, amortization, for_qualified_investors,
                change_today, analysis
            ) VALUES (
                $1,
                $2,  $3,
                $4,  $5,
                $6,  $7,  $8,  $9,  $10,
                $11, $12,
                $13, $14, $15,
                $16, $17
            )
            ON CONFLICT (ticker) DO UPDATE SET
                run_id                  = EXCLUDED.run_id,
                name                    = EXCLUDED.name,
                price                   = EXCLUDED.price,
                yield_to_maturity       = EXCLUDED.yield_to_maturity,
                coupon_type             = EXCLUDED.coupon_type,
                next_coupon             = EXCLUDED.next_coupon,
                coupon_amount           = EXCLUDED.coupon_amount,
                accrued_coupon_income   = EXCLUDED.accrued_coupon_income,
                payments_per_year       = EXCLUDED.payments_per_year,
                maturity                = EXCLUDED.maturity,
                volume                  = EXCLUDED.volume,
                subordinated            = EXCLUDED.subordinated,
                amortization            = EXCLUDED.amortization,
                for_qualified_investors = EXCLUDED.for_qualified_investors,
                change_today            = EXCLUDED.change_today,
                updated_at              = NOW()",
		)
		.bind(run_id)
		.bind(&bond.ticker)
		.bind(&bond.name)
		.bind(bond.price)
		.bind(bond.yield_to_maturity)
		.bind(&bond.coupon_type)
		.bind(&bond.next_coupon)
		.bind(bond.coupon_amount)
		.bind(bond.accrued_coupon_income)
		.bind(bond.payments_per_year)
		.bind(&bond.maturity)
		.bind(bond.volume)
		.bind(&bond.subordinated)
		.bind(&bond.amortization)
		.bind(&bond.for_qualified_investors)
		.bind(bond.change_today)
		.bind(&bond.analysis)
		.execute(&mut *tx)
		.await?;

		let outcome = match previous {
			None => SaveOutcome::Created,
			Some((prev_price, prev_coupon)) => {
				if prev_price != bond.price || prev_coupon != bond.coupon_amount {
					// Price or coupon moved — record a history point.
					sqlx::query(
						"INSERT INTO obligation_crawler_price_history
                            (ticker, price, coupon_amount, run_id)
                         VALUES ($1, $2, $3, $4)",
					)
					.bind(&bond.ticker)
					.bind(bond.price)
					.bind(bond.coupon_amount)
					.bind(run_id)
					.execute(&mut *tx)
					.await?;
					SaveOutcome::Updated
				} else {
					SaveOutcome::Unchanged
				}
			}
		};

		tx.commit().await?;

		Ok(outcome)
	}

	/// Fetches all bonds for a given run, ordered by creation time.
	#[allow(dead_code)]
	pub async fn get_bonds_for_run(
		pool: &PgPool,
		run_id: Uuid,
	) -> Result<Vec<BondRecord>, sqlx::Error> {
		let records = sqlx::query_as::<_, BondRecord>(
			"SELECT * FROM obligation_crawler_bonds WHERE run_id = $1 ORDER BY created_at ASC",
		)
		.bind(run_id)
		.fetch_all(pool)
		.await?;

		Ok(records)
	}

	/// Fetches the N most recent completed runs.
	#[allow(dead_code)]
	pub async fn get_recent_runs(
		pool: &PgPool,
		limit: i64,
	) -> Result<Vec<ObligationCrawlerRun>, sqlx::Error> {
		let runs = sqlx::query_as::<_, ObligationCrawlerRun>(
			"SELECT * FROM obligation_crawler_runs
             WHERE status = 'completed'
             ORDER BY started_at DESC
             LIMIT $1",
		)
		.bind(limit)
		.fetch_all(pool)
		.await?;

		Ok(runs)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn should_have_all_bond_record_fields() {
		let bond = BondListItem {
			ticker: "T001".to_string(),
			name: "Test".to_string(),
			price: Some(998.0),
			yield_to_maturity: Some(14.5),
			coupon_type: None,
			next_coupon: None,
			maturity: None,
			volume: None,
			accrued_coupon_income: None,
			coupon_amount: None,
			payments_per_year: None,
			subordinated: None,
			amortization: None,
			for_qualified_investors: None,
			change_today: None,
			analysis: None,
		};
		assert_eq!(bond.ticker, "T001");
		assert_eq!(bond.price, Some(998.0));
	}

	#[test]
	fn obligation_crawler_run_has_required_fields() {
		let run = ObligationCrawlerRun {
			id: Uuid::new_v4(),
			started_at: chrono::Utc::now(),
			finished_at: None,
			tbank_url: "https://example.com".to_string(),
			headless_chrome: true,
			status: "running".to_string(),
			bonds_count: 0,
			error_message: None,
			duration_seconds: None,
		};
		assert_eq!(run.status, "running");
		assert!(run.started_at <= chrono::Utc::now());
	}

	#[test]
	fn run_status_values_are_valid() {
		let valid_statuses = vec!["running", "completed", "failed"];
		for status in valid_statuses {
			let run = ObligationCrawlerRun {
				id: Uuid::new_v4(),
				started_at: chrono::Utc::now(),
				finished_at: Some(chrono::Utc::now()),
				tbank_url: "test".to_string(),
				headless_chrome: false,
				status: status.to_string(),
				bonds_count: 5,
				error_message: None,
				duration_seconds: Some(60),
			};
			assert!(matches!(
				run.status.as_str(),
				"running" | "completed" | "failed"
			));
		}
	}

	#[test]
	fn run_with_completed_status_has_finished_at() {
		let run = ObligationCrawlerRun {
			id: Uuid::new_v4(),
			started_at: chrono::Utc::now(),
			finished_at: Some(chrono::Utc::now()),
			tbank_url: "test".to_string(),
			headless_chrome: false,
			status: "completed".to_string(),
			bonds_count: 10,
			error_message: None,
			duration_seconds: Some(120),
		};
		assert!(run.finished_at.is_some());
		assert_eq!(run.bonds_count, 10);
	}

	#[test]
	fn run_with_failed_status_has_error_message() {
		let run = ObligationCrawlerRun {
			id: Uuid::new_v4(),
			started_at: chrono::Utc::now(),
			finished_at: Some(chrono::Utc::now()),
			tbank_url: "test".to_string(),
			headless_chrome: false,
			status: "failed".to_string(),
			bonds_count: 0,
			error_message: Some("Connection timeout".to_string()),
			duration_seconds: Some(30),
		};
		assert!(run.error_message.is_some());
		assert_eq!(run.error_message.as_ref().unwrap(), "Connection timeout");
	}
}
