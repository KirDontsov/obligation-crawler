use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlerTask {
	pub task_id: String,
	pub task_type: String,
	pub request_data: CrawlerRequestData,
}

impl CrawlerTask {
	pub fn new(task_type: String, request_data: CrawlerRequestData) -> Self {
		Self {
			task_id: Uuid::new_v4().to_string(),
			task_type,
			request_data,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlerRequestData {
	pub request: String,
	pub city: String,
	pub request_id: Option<i64>,
	pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondCrawlerTask {
	pub task_id: String,
	pub created_at: chrono::DateTime<chrono::Utc>,
}

impl BondCrawlerTask {
	pub fn new() -> Self {
		Self {
			task_id: Uuid::new_v4().to_string(),
			created_at: chrono::Utc::now(),
		}
	}
}
