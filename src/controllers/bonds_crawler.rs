use crate::config::CrawlerConfig;
use crate::error::Result;
use crate::models::bonds::BondListItem;
use crate::services::bonds_crawler::BondsCrawler;

#[allow(unused)]
pub async fn run_bonds_crawler(
	config: CrawlerConfig,
	duration_minutes: Option<u64>,
	db_pool: Option<sqlx::PgPool>,
) -> Result<Vec<BondListItem>> {
	let mut crawler = BondsCrawler::new(config, db_pool);
	let bonds = crawler.run_crawl_loop(duration_minutes).await?;
	crawler.close().await?;
	Ok(bonds)
}

#[allow(unused)]
pub async fn collect_bonds_once(
	config: CrawlerConfig,
	db_pool: Option<sqlx::PgPool>,
) -> Result<Vec<BondListItem>> {
	let mut crawler = BondsCrawler::new(config, db_pool);
	crawler.initialize().await?;
	crawler.navigate_to_bonds().await?;
	crawler.wait_for_login().await?;

	let available = crawler.check_page_available().await?;

	if !available {
		return Ok(Vec::new());
	}

	let bonds = crawler.collect_bonds().await?;
	crawler.close().await?;

	Ok(bonds)
}
