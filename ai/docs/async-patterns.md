# Async Patterns — Obligation Crawler

## Tokio Runtime

The project uses `#[tokio::main]` with `full` features. All async code runs on the Tokio multi-thread scheduler.

```rust
#[tokio::main]
async fn main() -> Result<(), CrawlerError> {
    // entry point
}
```

---

## Service Lifecycle Pattern

Services that own stateful resources follow this pattern:

```rust
pub struct MyService {
    config: CrawlerConfig,
    connection: Option<Connection>,  // Option = not initialized yet
}

impl MyService {
    pub fn new(config: CrawlerConfig) -> Self {
        Self { config, connection: None }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        let conn = Connection::connect(...).await?;
        self.connection = Some(conn);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<Output> {
        let conn = self.connection.as_ref()
            .ok_or_else(|| CrawlerError::CrawlerError("Not initialized".to_string()))?;
        // ...
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(conn) = self.connection.take() {
            conn.close(...).await?;
        }
        Ok(())
    }
}

impl Drop for MyService {
    fn drop(&mut self) {
        if let Some(conn) = self.connection.take() {
            tokio::spawn(async move {
                let _ = conn.close(...).await;
            });
        }
    }
}
```

---

## WebDriver (thirtyfour) Patterns

### Initialization with ChromeDriver

```rust
use thirtyfour::{ChromiumLikeCapabilities, DesiredCapabilities, WebDriver};

let mut caps = DesiredCapabilities::chrome();
if headless {
    caps.set_headless()?;
}
caps.add_arg("--no-sandbox")?;
caps.add_arg("--disable-dev-shm-usage")?;
caps.add_arg("--disable-blink-features=AutomationControlled")?;

let driver = WebDriver::new("http://localhost:9515", caps).await?;
```

### Finding Elements

```rust
use thirtyfour::By;

// Single element — errors if not found
let elem = driver.find(By::Css("tbody[data-qa-type=\"...\"]")).await?;

// Multiple elements — returns empty vec if none found
let rows = elem.find_all(By::Css("tr[data-qa-type=\"...\"]")).await?;

// Fallible find — use match instead of ?
match driver.find(By::Css(".optional-element")).await {
    Ok(elem) => { /* use it */ }
    Err(_) => { /* element not present, skip */ }
}
```

### Stale Element Avoidance

Elements become stale after page navigation or DOM updates. Re-find by index:

```rust
// ❌ BAD — rows become stale after opening detail tab
for row in rows.iter() {
    open_tab_and_parse(driver, row).await?;
    // row is now stale
}

// ✅ GOOD — re-find each row by its position before use
for idx in 0..rows.len() {
    let row = driver.find(By::Css(
        &format!("tr[data-qa-type=\"...\"]:nth-of-type({})", idx + 1)
    )).await?;
    process_row(driver, &row).await?;
}
```

### Opening and Closing Tabs

```rust
let main_window = driver.window().await?;

// Open link in new tab
driver.execute(&format!("window.open('{}', '_blank')", href), vec![]).await?;
sleep(Duration::from_secs(1)).await;

let windows = driver.windows().await?;
driver.switch_to_window(windows[1].clone()).await?;
sleep(Duration::from_secs(2)).await;

// ... work in new tab ...

driver.close_window().await?;
driver.switch_to_window(main_window).await?;
sleep(Duration::from_millis(500)).await;
```

### Text Cleaning Pattern

T-Bank DOM contains non-breaking spaces and mixed whitespace:

```rust
fn clean_number_text(raw: &str) -> String {
    raw.replace('\n', "")
        .replace('\r', "")
        .replace('\t', "")
        .replace(' ', "")
        .replace('\u{A0}', "")  // non-breaking space
        .replace(',', ".")       // Russian decimal separator
        .trim()
        .to_string()
}

// Parse after cleaning
let price: Option<f64> = clean_number_text(&price_text)
    .replace('₽', "")
    .replace('%', "")
    .parse()
    .ok();
```

---

## RabbitMQ Patterns (lapin)

### Producer

```rust
let producer = RabbitMQProducer::new(rabbitmq_url, exchange).await?;
producer.publish_bonds_data(&json).await?;
```

### Consumer (infinite reconnect loop)

```rust
let consumer = RabbitMQConsumer::new(rabbitmq_url, queue_name);
consumer.start_consuming(|message| {
    Box::pin(async move {
        info!("Received task: {}", message);
        // process...
        Ok(())
    })
}).await?;
```

The consumer reconnects automatically on connection failure with 5s backoff.

### BoxFuture for callbacks

When a closure must return an async value and be `'static`:

```rust
use futures::future::BoxFuture;

pub async fn start_consuming<F>(&self, mut handler: F) -> Result<()>
where
    F: FnMut(String) -> BoxFuture<'static, Result<()>> + Send + Sync + 'static,
{
    // ...
    if let Err(e) = handler(message_str).await { ... }
}
```

---

## PostgreSQL (sqlx) Patterns

### Pool creation

```rust
let pool = create_connection_pool().await?;
// pool: PgPool, cloneable (Arc internally)
```

### Queries

```rust
// Fetch one row
let row: (i32,) = sqlx::query_as("SELECT id FROM bonds WHERE ticker = $1")
    .bind(ticker)
    .fetch_one(&pool)
    .await?;

// Fetch all
let bonds: Vec<BondRecord> = sqlx::query_as!(BondRecord, "SELECT * FROM bonds")
    .fetch_all(&pool)
    .await?;

// Execute (insert/update/delete)
sqlx::query!("INSERT INTO bonds (ticker, name) VALUES ($1, $2)", ticker, name)
    .execute(&pool)
    .await?;
```

---

## Sleep / Delay

```rust
use tokio::time::{sleep, Duration};

// Specific delays
sleep(Duration::from_secs(2)).await;
sleep(Duration::from_millis(500)).await;

// Poll interval from config
sleep(Duration::from_secs(config.poll_interval_seconds)).await;
```

Never use `std::thread::sleep` — it blocks the Tokio executor thread.

---

## Spawning Background Tasks

```rust
// Fire-and-forget (no result needed)
tokio::spawn(async move {
    if let Err(e) = some_task().await {
        error!("Background task failed: {}", e);
    }
});

// Wait for result (use JoinHandle)
let handle = tokio::spawn(async move {
    compute_something().await
});
let result = handle.await??;  // outer ? = JoinError, inner ? = CrawlerError
```
