# Testing Guidelines — Obligation Crawler

## Test Types

| Type | Location | Crate |
|------|----------|-------|
| Unit tests | `#[cfg(test)]` module at bottom of each file | built-in |
| Integration tests | `tests/` directory at crate root | built-in |
| Async tests | `#[tokio::test]` | tokio |

---

## Unit Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Synchronous test
    #[test]
    fn should_return_error_when_env_var_missing() {
        // Arrange
        std::env::remove_var("TBANK_URL");
        // Act
        let result = CrawlerConfig::from_env();
        // Assert
        assert!(result.is_ok()); // TBANK_URL has a default
    }

    // Async test
    #[tokio::test]
    async fn should_fail_on_invalid_rabbitmq_url() {
        let result = RabbitMQProducer::new(
            "invalid://url".to_string(),
            "exchange".to_string()
        ).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CrawlerError::RabbitMQError(_)));
    }
}
```

---

## Naming Convention

```
fn should_[expected_behavior]_when_[condition]()
fn should_[expected_behavior]_given_[state]()
```

Examples:
```rust
fn should_parse_price_when_text_contains_ruble_sign()
fn should_return_none_when_cells_count_less_than_four()
fn should_retry_on_connection_failure()
fn should_create_csv_with_headers_given_valid_path()
```

---

## Testing Pure Functions (no I/O)

Put in `#[cfg(test)]` inside the same file.

```rust
// src/shared/utils.rs
pub fn clean_number_text(raw: &str) -> String {
    raw.replace('\u{A0}', "")
        .replace(',', ".")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_replace_nbsp_when_cleaning_number() {
        assert_eq!(clean_number_text("1\u{A0}234,56"), "1234.56");
    }

    #[test]
    fn should_return_empty_when_input_is_whitespace() {
        assert_eq!(clean_number_text("   "), "");
    }
}
```

---

## Testing Config

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_use_defaults_when_env_vars_not_set() {
        // Clear env vars that have defaults
        std::env::remove_var("POLL_INTERVAL_SECONDS");
        let config = CrawlerConfig::from_env().unwrap();
        assert_eq!(config.poll_interval_seconds, 5);
    }

    #[test]
    fn should_parse_bool_env_var() {
        std::env::set_var("HEADLESS_CHROME", "true");
        let config = CrawlerConfig::from_env().unwrap();
        assert!(config.headless_chrome);
        std::env::remove_var("HEADLESS_CHROME");
    }

    #[test]
    fn should_fail_on_invalid_integer() {
        std::env::set_var("MAX_RETRIES", "not_a_number");
        let result = CrawlerConfig::from_env();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::InvalidValue(_, _)));
        std::env::remove_var("MAX_RETRIES");
    }
}
```

---

## Testing Error Variants

Use `matches!` macro for variant checking:

```rust
#[test]
fn should_map_to_parse_error() {
    let json = "invalid json {{{";
    let result: Result<serde_json::Value, _> = serde_json::from_str(json);
    let err = CrawlerError::from(result.unwrap_err());
    assert!(matches!(err, CrawlerError::ParseError(_)));
}
```

---

## Testing Models / Serialization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_serialize_bond_list_item_to_json() {
        let bond = BondListItem {
            ticker: "SBR001".to_string(),
            name: "Sberbank Bond".to_string(),
            price: Some(998.5),
            yield_to_maturity: Some(14.5),
            coupon_type: Some("Фиксированный".to_string()),
            // ... rest as None
            ..Default::default()  // if Default is derived
        };
        let json = serde_json::to_string(&bond).unwrap();
        assert!(json.contains("SBR001"));
        assert!(json.contains("998.5"));
    }

    #[test]
    fn should_roundtrip_through_json() {
        let original = BondListItem { ticker: "T001".to_string(), ..Default::default() };
        let json = serde_json::to_string(&original).unwrap();
        let restored: BondListItem = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.ticker, original.ticker);
    }
}
```

---

## Testing CSV Output

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;  // or use std::env::temp_dir()

    #[test]
    fn should_create_csv_with_headers() {
        let tmp = format!("/tmp/test_bonds_{}.csv", uuid::Uuid::new_v4());
        BondListItem::create_csv_file(&tmp).unwrap();
        let content = fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("Тикер"));
        assert!(content.contains("Цена"));
        fs::remove_file(&tmp).unwrap();
    }

    #[test]
    fn should_append_bond_to_existing_csv() {
        let tmp = format!("/tmp/test_bonds_{}.csv", uuid::Uuid::new_v4());
        BondListItem::create_csv_file(&tmp).unwrap();
        let bond = BondListItem {
            ticker: "TEST01".to_string(),
            name: "Test Bond".to_string(),
            price: Some(1000.0),
            ..Default::default()
        };
        BondListItem::append_to_csv(&bond, &tmp).unwrap();
        let content = fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("TEST01"));
        fs::remove_file(&tmp).unwrap();
    }
}
```

---

## Integration Tests (`tests/` directory)

For tests that require external services (DB, RabbitMQ) — skip by default, guard with env:

```rust
// tests/rabbitmq_integration.rs
use obligation_crawler::services::rabbitmq_producer::RabbitMQProducer;

#[tokio::test]
#[ignore = "requires running RabbitMQ"]
async fn should_publish_bonds_data() {
    let url = std::env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());
    let producer = RabbitMQProducer::new(url, "test_exchange".to_string()).await.unwrap();
    let result = producer.publish_bonds_data(r#"[{"ticker":"T001"}]"#).await;
    assert!(result.is_ok());
}
```

Run integration tests explicitly:
```bash
cargo test -- --ignored
```

---

## Checklist

- [ ] Every public function has at least one happy-path test
- [ ] Every fallible operation has an error-path test
- [ ] No real network/DB calls in unit tests (use `#[ignore]` for integration)
- [ ] Tests clean up temp files they create
- [ ] Naming follows `should_X_when_Y` pattern
- [ ] `#[cfg(test)]` module at bottom of each file, not a separate file (for unit tests)
