---
description: Create new Rust model (struct/enum) with serde derives, Option for nullable fields, impl methods for CSV/JSON. Place in src/models/. After /plan. Use /service and /test for services and tests.
---

# Model Template

**Model:** $ARGUMENTS

## Context
- Branch: !`git branch --show-current`
- Existing models: !`ls src/models/ 2>/dev/null`
- Similar structs: !`grep -r "pub struct" src/models/ 2>/dev/null | head -10`

---

## Task

Create `src/models/[model_name].rs`.

Add to `src/models/mod.rs`:
```rust
pub mod [model_name];
pub use [model_name]::*;
```

---

## Requirements

1. Always derive `Debug` — required for error messages/logging
2. Derive `Clone` if passed to multiple owners
3. Derive `Serialize, Deserialize` if sent to RabbitMQ or JSON
4. Use `Option<T>` for nullable fields — never empty strings as null
5. No async code, no service dependencies in models
6. CSV I/O methods belong directly on struct

---

## Pattern A — Data Model

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct [ModelName] {
    pub id: Option<uuid::Uuid>,
    pub name: String,
    pub ticker: String,
    pub price: Option<f64>,
    pub yield_value: Option<f64>,
    pub maturity: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl [ModelName] {
    pub fn new(name: String, ticker: String) -> Self {
        Self {
            id: Some(uuid::Uuid::new_v4()),
            name,
            ticker,
            price: None,
            yield_value: None,
            maturity: None,
            created_at: Some(chrono::Utc::now()),
        }
    }
}
```

---

## Pattern B — CSV I/O

```rust
use csv::Writer;
use std::fs::{self, OpenOptions};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct [ModelName] {
    pub ticker: String,
    pub name: String,
    pub price: Option<f64>,
}

impl [ModelName] {
    pub fn create_csv_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all("./output")?;
        let mut wtr = Writer::from_path(filename)?;
        wtr.write_record(&["Ticker", "Name", "Price"])?;
        wtr.flush()?;
        Ok(())
    }

    pub fn append_to_csv(item: &Self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = OpenOptions::new().create(true).append(true).open(filename)?;
        let mut wtr = Writer::from_writer(file);
        wtr.write_record(&[
            &item.ticker,
            &item.name,
            &item.price.map(|p| p.to_string()).unwrap_or_default(),
        ])?;
        wtr.flush()?;
        Ok(())
    }
}
```

---

## Pattern C — API Response (DTO)

```rust
use serde::{Deserialize, Serialize};
use crate::models::[OtherModel];

#[derive(Debug, Serialize, Deserialize)]
pub struct [ModelName]Response {
    pub total: usize,
    pub items: Vec<[OtherModel]>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Api[ModelName]Response {
    pub success: bool,
    pub data: Option<[ModelName]Response>,
    pub error: Option<String>,
}

impl Api[ModelName]Response {
    pub fn success(items: Vec[OtherModel]) -> Self {
        Self {
            success: true,
            data: Some([ModelName]Response {
                total: items.len(),
                items,
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
```

---

## Pattern D — RabbitMQ Message

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct [ModelName]Message {
    pub task_id: String,
    pub payload: String,
    pub timestamp: i64,
}

impl [ModelName]Message {
    pub fn new(payload: String) -> Self {
        Self {
            task_id: uuid::Uuid::new_v4().to_string(),
            payload,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}
```

---

## Checklist

- [ ] Debug derived
- [ ] Clone if multi-ownership
- [ ] Serialize/Deserialize if JSON/RabbitMQ
- [ ] Option<T> for nullable — no empty strings
- [ ] No async, no service/config imports
- [ ] Added to src/models/mod.rs
- [ ] Tests in #[cfg(test)] module

## Validate

```bash
cargo build 2>&1 | head -30
cargo clippy -- -D warnings 2>&1 | head -20
```