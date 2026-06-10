mod api;
mod config;
mod controllers;
mod database;
mod error;
mod models;
mod repository;
mod services;
mod shared;

use dotenv::dotenv;
use sqlx::PgPool;
use std::env;

use crate::config::CrawlerConfig;
use crate::error::CrawlerError;
use crate::services::bonds_crawler::BondsCrawler;
use crate::services::rabbitmq_producer::RabbitMQProducer;

#[tokio::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), CrawlerError> {
	dotenv().ok();
	env_logger::init();

	println!("Starting Obligation Crawler...");

	// Initialize DB pool once at startup; crawler continues without DB if unavailable
	let db_pool: Option<PgPool> = match database::create_connection_pool().await {
		Ok(pool) => {
			println!("✅ Database connection pool created");
			Some(pool)
		}
		Err(e) => {
			eprintln!("⚠️ Database unavailable, continuing without DB: {}", e);
			None
		}
	};

	let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "direct".to_string());

	match run_mode.as_str() {
		"direct" => run_direct_mode(db_pool).await?,
		"consumer" => run_consumer_mode().await?,
		_ => {
			return Err(CrawlerError::CrawlerError(format!(
				"Invalid RUN_MODE: {}",
				run_mode
			)))
		}
	}

	Ok(())
}

async fn run_direct_mode(db_pool: Option<PgPool>) -> Result<(), CrawlerError> {
	let config = CrawlerConfig::from_env()?;

	let duration_minutes = env::var("DURATION_MINUTES")
		.ok()
		.and_then(|v| v.parse().ok());

	let save_to_rabbitmq = env::var("ENABLE_RABBITMQ")
		.unwrap_or_else(|_| "false".to_string())
		.parse()
		.unwrap_or(false);

	let mut producer = if save_to_rabbitmq {
		Some(
			RabbitMQProducer::new(
				config.rabbitmq_url.clone(),
				config.rabbitmq_exchange.clone(),
			)
			.await?,
		)
	} else {
		None
	};

	println!("Starting bonds crawler...");
	let mut crawler = BondsCrawler::new(config, db_pool);

	let bonds = crawler.run_crawl_loop(duration_minutes).await?;

	println!("\n=== COLLECTED BONDS ({}) ===\n", bonds.len());

	for (i, bond) in bonds.iter().enumerate() {
		println!("{}. {}", i + 1, bond.name);
		println!("   Тикер: {}", bond.ticker);
		println!(
			"   Цена: {}₽",
			bond.price
				.map(|p| p.to_string())
				.unwrap_or_else(|| "N/A".to_string())
		);
		println!(
			"   Доходность к погашению: {}%",
			bond.yield_to_maturity
				.map(|y| y.to_string())
				.unwrap_or_else(|| "N/A".to_string())
		);
		println!(
			"   Дата погашения: {}",
			bond.maturity.as_deref().unwrap_or("N/A")
		);
		println!(
			"   Дата выплаты купона: {}",
			bond.next_coupon.as_deref().unwrap_or("N/A")
		);
		println!(
			"   Тип купона: {}",
			bond.coupon_type.as_deref().unwrap_or("N/A")
		);
		println!(
			"   Накопленный купонный доход: {}₽",
			bond.accrued_coupon_income
				.map(|v| v.to_string())
				.unwrap_or_else(|| "N/A".to_string())
		);
		println!(
			"   Величина купона: {}₽",
			bond.coupon_amount
				.map(|v| v.to_string())
				.unwrap_or_else(|| "N/A".to_string())
		);
		println!(
			"   Номинал: {}₽",
			bond.volume
				.map(|v| v.to_string())
				.unwrap_or_else(|| "N/A".to_string())
		);
		println!(
			"   Количество выплат в год: {}",
			bond.payments_per_year
				.map(|v| v.to_string())
				.unwrap_or_else(|| "N/A".to_string())
		);
		println!(
			"   Субординированность: {}",
			bond.subordinated.as_deref().unwrap_or("N/A")
		);
		println!(
			"   Амортизация: {}",
			bond.amortization.as_deref().unwrap_or("N/A")
		);
		println!(
			"   Для квалифицированных инвесторов: {}",
			bond.for_qualified_investors.as_deref().unwrap_or("N/A")
		);
		println!();
	}

	if let Some(ref mut p) = producer {
		let json =
			serde_json::to_string(&bonds).map_err(|e| CrawlerError::ParseError(e.to_string()))?;
		p.publish_bonds_data(&json).await?;
	}

	crawler.close().await?;

	println!("✅ Done");
	Ok(())
}

async fn run_consumer_mode() -> Result<(), CrawlerError> {
	let config = CrawlerConfig::from_env()?;

	println!(
		"Starting RabbitMQ consumer for queue: {}",
		config.rabbitmq_queue
	);

	let consumer = services::rabbitmq_consumer::RabbitMQConsumer::new(
		config.rabbitmq_url,
		config.rabbitmq_queue,
		config.rabbitmq_exchange,
	);

	consumer
		.start_consuming(|message| {
			Box::pin(async move {
				println!("📩 Received task: {}", message);
				Ok(())
			})
		})
		.await?;

	Ok(())
}
