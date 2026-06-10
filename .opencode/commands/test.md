---
description: Create Rust unit tests with #[cfg(test)] modules, tokio::test for async, should_X_when_Y naming, no unwrap in production paths. After implementing a feature.
---

# Testing Template

**Target:** $ARGUMENTS

## Context
- Branch: !`git branch --show-current`
- Target file: !`find src -name "*.rs" | xargs grep -l "$ARGUMENTS" 2>/dev/null | head -5`

---

## Task

Add tests for **$ARGUMENTS**:
- Pure functions → `#[cfg(test)]` module at bottom of same file
- Integration-level → `tests/[name].rs` at crate root

---

## Rules

1. `#[test]` for sync, `#[tokio::test]` for async — no mixing
2. `unwrap()` allowed in `#[cfg(test)]` blocks
3. No real network/DB/WebDriver in unit tests — use `#[ignore]` for integration
4. Each test covers ONE scenario — no branching in test
5. Arrange / Act / Assert structure

---

## Naming Convention

```
fn should_[expected_behavior]_when_[condition]()
fn should_[expected_behavior]_given_[state]()
```

Examples:
```rust
fn should_return_none_when_cells_count_is_less_than_four()
fn should_parse_price_given_text_with_ruble_sign()
fn should_use_default_url_when_env_var_not_set()
fn should_fail_with_rabbitmq_error_when_url_is_invalid()
```

---

## Unit Test Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_clean_nbsp_when_text_contains_non_breaking_space() {
        let input = "1\u{A0}234,56₽";
        let result = clean_number_text(input);
        assert_eq!(result, "1234.56");
    }

    #[test]
    fn should_return_empty_string_when_input_is_only_whitespace() {
        assert_eq!(clean_number_text("   "), "");
    }
}
```

---

## Config Test Pattern

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn should_use_default_poll_interval_when_env_var_not_set() {
        std::env::remove_var("POLL_INTERVAL_SECONDS");
        let config = CrawlerConfig::from_env().unwrap();
        assert_eq!(config.poll_interval_seconds, 5);
    }

    #[test]
    fn should_fail_with_invalid_value_when_poll_interval_is_not_a_number() {
        std::env::set_var("POLL_INTERVAL_SECONDS", "abc");
        let result = CrawlerConfig::from_env();
        assert!(result.is_err());
    }
}
```

---

## Async Service Test Pattern

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn should_fail_with_rabbitmq_error_when_url_is_invalid() {
        let result = RabbitMQProducer::new(
            "not-a-url".to_string(),
            "exchange".to_string(),
        ).await;
        assert!(result.is_err());
    }
}
```

---

## Model Serialization Test

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn should_serialize_bond_to_valid_json() {
        let bond = BondListItem {
            ticker: "SBR001".to_string(),
            name: "Test Bond".to_string(),
            price: Some(998.5),
            // ... other fields
        };
        let json = serde_json::to_string(&bond).unwrap();
        assert!(json.contains("SBR001"));
    }

    #[test]
    fn should_roundtrip_bond_through_json() {
        let original = BondListItem { ticker: "T1".to_string(), name: "N1".to_string(), ..Default::default() };
        let json = serde_json::to_string(&original).unwrap();
        let restored: BondListItem = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.ticker, original.ticker);
    }
}
```

---

## Integration Test (requires external service)

```rust
// tests/rabbitmq_integration.rs
#[tokio::test]
#[ignore = "requires running RabbitMQ on localhost:5672"]
async fn should_publish_bonds_data() {
    let url = std::env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());
    let producer = RabbitMQProducer::new(url, "test_exchange".to_string()).await.unwrap();
    let result = producer.publish_bonds_data(r#"[{"ticker":"T001"}]"#).await;
    assert!(result.is_ok());
}
```

Run with: `cargo test -- --ignored`

---

## Checklist

- [ ] Happy path tested
- [ ] Error path tested for every Result-returning function
- [ ] No real I/O in unit tests
- [ ] Integration tests marked `#[ignore]`
- [ ] Naming follows `should_X_when_Y`
- [ ] `#[cfg(test)]` at bottom of source file
- [ ] `cargo test` passes

## Validate

```bash
cargo test [test_name] 2>&1
cargo clippy -- -D warnings 2>&1 | head -20
```