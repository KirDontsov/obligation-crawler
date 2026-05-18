use chrono::Utc;
use chrono::{DateTime, NaiveDateTime};

#[allow(dead_code)]
pub fn now() -> DateTime<Utc> {
	Utc::now()
}

#[allow(dead_code)]
pub fn current_timestamp() -> i64 {
	Utc::now().timestamp()
}

#[allow(dead_code)]
pub fn format_datetime(dt: DateTime<Utc>) -> String {
	dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[allow(dead_code, deprecated)]
pub fn parse_timestamp(ts: i64) -> Option<DateTime<Utc>> {
	NaiveDateTime::from_timestamp_opt(ts, 0)
		.map(|ndt| DateTime::from_naive_utc_and_offset(ndt, Utc))
}
