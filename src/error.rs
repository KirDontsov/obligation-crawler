use thiserror::Error;
use crate::config::ConfigError;
use thirtyfour::error::WebDriverError;

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Crawler error: {0}")]
    CrawlerError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("RabbitMQ error: {0}")]
    RabbitMQError(#[from] lapin::Error),

    #[error("Selenium error: {0}")]
    SeleniumError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    
    #[error("Ack error: {0}")]
    AckError(String),
    
    #[error("Config error: {0}")]
    ConfigError(#[from] ConfigError),
    
    #[error("WebDriver error: {0}")]
    WebDriverError(#[from] WebDriverError),
}

impl From<serde_json::Error> for CrawlerError {
    fn from(e: serde_json::Error) -> Self {
        CrawlerError::ParseError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CrawlerError>;