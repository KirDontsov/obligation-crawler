mod api;
mod config;
mod controllers;
mod database;
mod error;
mod models;
mod services;
mod shared;

use chrono::Utc;
use dotenv::dotenv;
use std::env;

use crate::config::CrawlerConfig;
use crate::error::CrawlerError;
use crate::models::BondListItem;
use crate::services::bonds_crawler::BondsCrawler;
use crate::services::rabbitmq_producer::RabbitMQProducer;

#[tokio::main]
async fn main() -> Result<(), CrawlerError> {
    dotenv().ok();
    env_logger::init();

    println!("Starting Obligation Crawler...");

    let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "direct".to_string());

    match run_mode.as_str() {
        "direct" => run_direct_mode().await?,
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

async fn run_direct_mode() -> Result<(), CrawlerError> {
    let config = CrawlerConfig::from_env()?;

    let duration_minutes = env::var("DURATION_MINUTES")
        .ok()
        .and_then(|v| v.parse().ok());

    let save_to_rabbitmq = env::var("ENABLE_RABBITMQ")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let mut producer = if save_to_rabbitmq {
        let rabbitmq_url = env::var("RABBITMQ_URL")
            .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());
        let exchange = env::var("RABBITMQ_EXCHANGE")
            .unwrap_or_else(|_| "obligation_exchange".to_string());

        Some(RabbitMQProducer::new(rabbitmq_url, exchange).await?)
    } else {
        None
    };

    println!("Starting bonds crawler...");
    let mut crawler = BondsCrawler::new(config);

    let bonds = crawler.run_crawl_loop(duration_minutes).await?;

    println!("\n=== COLLECTED BONDS ({}) ===\n", bonds.len());

    for (i, bond) in bonds.iter().enumerate() {
        println!("{}. {}", i + 1, bond.name);
        println!("   Тикер: {}", bond.ticker);
        println!("   Цена: {}₽", bond.price.map(|p| p.to_string()).unwrap_or_else(|| "N/A".to_string()));
        println!("   Доходность к погашению: {}%", bond.yield_to_maturity.map(|y| y.to_string()).unwrap_or_else(|| "N/A".to_string()));
        println!("   Дата погашения: {}", bond.maturity.as_ref().map(|m| m.as_str()).unwrap_or("N/A"));
        println!("   Дата выплаты купона: {}", bond.next_coupon.as_ref().map(|s| s.as_str()).unwrap_or("N/A"));
        println!("   Тип купона: {}", bond.coupon_type.as_ref().map(|s| s.as_str()).unwrap_or("N/A"));
        println!("   Накопленный купонный доход: {}₽", bond.accrued_coupon_income.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()));
        println!("   Величина купона: {}₽", bond.coupon_amount.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()));
        println!("   Номинал: {}₽", bond.volume.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()));
        println!("   Количество выплат в год: {}", bond.payments_per_year.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()));
        println!("   Субординированность: {}", bond.subordinated.as_ref().map(|s| s.as_str()).unwrap_or("N/A"));
        println!("   Амортизация: {}", bond.amortization.as_ref().map(|s| s.as_str()).unwrap_or("N/A"));
        println!("   Для квалифицированных инвесторов: {}", bond.for_qualified_investors.as_ref().map(|s| s.as_str()).unwrap_or("N/A"));
        println!("");
    }

    if let Some(ref mut p) = producer {
        let json = serde_json::to_string(&bonds).map_err(|e| CrawlerError::ParseError(e.to_string()))?;
        p.publish_bonds_data(&json).await?;
    }

    crawler.close().await?;

    println!("✅ Done");
    Ok(())
}

async fn run_consumer_mode() -> Result<(), CrawlerError> {
    let rabbitmq_url = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

    let queue_name = env::var("RABBITMQ_QUEUE")
        .unwrap_or_else(|_| "obligation_crawler_queue".to_string());

    println!("Starting RabbitMQ consumer for queue: {}", queue_name);

    let consumer = services::RabbitMQConsumer::new(rabbitmq_url, queue_name);

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