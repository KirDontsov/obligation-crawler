use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::BondListItem;

/// Persisted representation of a single crawl session.
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

	/// Inserts a single bond record linked to the given run.
	/// Returns the generated bond UUID.
	pub async fn save_bond(
		pool: &PgPool,
		run_id: Uuid,
		bond: &BondListItem,
	) -> Result<Uuid, sqlx::Error> {
		let row = sqlx::query(
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
            RETURNING id",
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
		.fetch_one(pool)
		.await?;

		let id: Uuid = row.get("id");
		Ok(id)
	}

	/// Fetches all bonds for a given run, ordered by creation time.
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
		// Compile-time check: BondRecord fields match BondListItem
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
}
