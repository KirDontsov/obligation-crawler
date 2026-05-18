use crate::models::BondListItem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BondsResponse {
	pub total: usize,
	pub bonds: Vec<BondListItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BondsApiResponse {
	pub success: bool,
	pub data: Option<BondsResponse>,
	pub error: Option<String>,
}

impl BondsApiResponse {
	pub fn success(bonds: Vec<BondListItem>) -> Self {
		Self {
			success: true,
			data: Some(BondsResponse {
				total: bonds.len(),
				bonds,
			}),
			error: None,
		}
	}

	pub fn error(msg: String) -> Self {
		Self {
			success: false,
			data: None,
			error: Some(msg),
		}
	}
}
