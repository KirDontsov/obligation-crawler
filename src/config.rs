use std::env;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Environment variable {0} not set")]
    MissingEnvVar(String),
    #[error("Invalid value for {0}: {1}")]
    InvalidValue(String, String),
}

#[derive(Debug, Clone)]
pub struct CrawlerConfig {
    pub tbank_url: String,
    pub poll_interval_seconds: u64,
    pub headless_chrome: bool,
    pub chrome_driver_path: String,
    pub wait_after_login_seconds: u64,
    pub max_retries: u32,
}

impl CrawlerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let tbank_url = env::var("TBANK_URL")
            .unwrap_or_else(|_| "https://www.tbank.ru/invest/bonds/".to_string());

        let poll_interval_seconds = env::var("POLL_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u64>()
            .map_err(|e| ConfigError::InvalidValue("POLL_INTERVAL_SECONDS".to_string(), e.to_string()))?;

        let headless_chrome = env::var("HEADLESS_CHROME")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .map_err(|e| ConfigError::InvalidValue("HEADLESS_CHROME".to_string(), e.to_string()))?;

        let chrome_driver_path = env::var("CHROME_DRIVER_PATH")
            .unwrap_or_else(|_| "./chromedriver".to_string());

        let wait_after_login_seconds = env::var("WAIT_AFTER_LOGIN_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse::<u64>()
            .map_err(|e| ConfigError::InvalidValue("WAIT_AFTER_LOGIN_SECONDS".to_string(), e.to_string()))?;

        let max_retries = env::var("MAX_RETRIES")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<u32>()
            .map_err(|e| ConfigError::InvalidValue("MAX_RETRIES".to_string(), e.to_string()))?;

        Ok(Self {
            tbank_url,
            poll_interval_seconds,
            headless_chrome,
            chrome_driver_path,
            wait_after_login_seconds,
            max_retries,
        })
    }

    pub fn new(
        tbank_url: String,
        poll_interval_seconds: u64,
        headless_chrome: bool,
        chrome_driver_path: String,
        wait_after_login_seconds: u64,
        max_retries: u32,
    ) -> Self {
        Self {
            tbank_url,
            poll_interval_seconds,
            headless_chrome,
            chrome_driver_path,
            wait_after_login_seconds,
            max_retries,
        }
    }
}