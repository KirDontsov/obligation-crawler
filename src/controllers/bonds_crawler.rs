use crate::config::CrawlerConfig;
use crate::error::Result;
use crate::models::BondListItem;
use crate::services::bonds_crawler::BondsCrawler;

pub async fn run_bonds_crawler(config: CrawlerConfig, duration_minutes: Option<u64>) -> Result<Vec<BondListItem>> {
    let mut crawler = BondsCrawler::new(config);
    let bonds = crawler.run_crawl_loop(duration_minutes).await?;
    crawler.close().await?;
    Ok(bonds)
}

pub async fn collect_bonds_once(config: CrawlerConfig) -> Result<Vec<BondListItem>> {
    let mut crawler = BondsCrawler::new(config);
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