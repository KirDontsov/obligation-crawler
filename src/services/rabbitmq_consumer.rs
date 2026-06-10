use crate::error::CrawlerError;
use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties, ExchangeKind};
use log::{error, info, warn};
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};

const INITIAL_RETRY_DELAY_MS: u64 = 1000;
const MAX_RETRY_DELAY_MS: u64 = 30000;
const BACKOFF_MULTIPLIER: f64 = 2.0;

pub struct RabbitMQConsumer {
	connection_string: String,
	queue_name: String,
	exchange_name: String,
}

impl RabbitMQConsumer {
	pub fn new(connection_string: String, queue_name: String, exchange_name: String) -> Self {
		Self {
			connection_string,
			queue_name,
			exchange_name,
		}
	}

	async fn connect_with_backoff(&self) -> Result<Connection, CrawlerError> {
		let mut delay_ms = INITIAL_RETRY_DELAY_MS;

		loop {
			match Connection::connect(&self.connection_string, ConnectionProperties::default())
				.await
			{
				Ok(conn) => {
					info!("Connected to RabbitMQ at {}", self.connection_string);
					return Ok(conn);
				}
				Err(e) => {
					warn!(
						"Failed to connect to RabbitMQ: {}. Retrying in {}ms...",
						e, delay_ms
					);
					sleep(Duration::from_millis(delay_ms)).await;
					delay_ms = (delay_ms as f64 * BACKOFF_MULTIPLIER) as u64;
					delay_ms = delay_ms.min(MAX_RETRY_DELAY_MS);
				}
			}
		}
	}

	pub async fn start_consuming<F>(self, mut message_handler: F) -> Result<(), CrawlerError>
	where
		F: FnMut(String) -> futures::future::BoxFuture<'static, Result<(), CrawlerError>>
			+ Send
			+ 'static,
	{
		let (shutdown_tx, _) = broadcast::channel::<()>(1);
		let mut shutdown_rx = shutdown_tx.subscribe();

		loop {
			info!("Connecting to RabbitMQ at: {}", self.connection_string);

			let connection = match self.connect_with_backoff().await {
				Ok(conn) => conn,
				Err(e) => {
					error!("Failed to connect after retries: {}", e);
					continue;
				}
			};

			let channel = match connection.create_channel().await {
				Ok(chan) => chan,
				Err(e) => {
					error!("Failed to create channel: {}", e);
					continue;
				}
			};

			let dlx_exchange = format!("{}_dlx", self.exchange_name);
			let _ = channel
				.exchange_declare(
					dlx_exchange.as_str(),
					ExchangeKind::Direct,
					ExchangeDeclareOptions::default(),
					FieldTable::default(),
				)
				.await;

			let dlq_name = format!("{}_dlq", self.queue_name);
			let dlq = match channel
				.queue_declare(
					dlq_name.as_str(),
					QueueDeclareOptions {
						durable: true,
						exclusive: false,
						auto_delete: false,
						..QueueDeclareOptions::default()
					},
					FieldTable::default(),
				)
				.await
			{
				Ok(q) => q,
				Err(e) => {
					error!("Failed to declare DLQ: {}", e);
					continue;
				}
			};

			let _ = channel
				.queue_bind(
					dlq.name().as_str(),
					dlx_exchange.as_str(),
					"failed",
					QueueBindOptions::default(),
					FieldTable::default(),
				)
				.await;

			let mut queue_args = FieldTable::default();
			queue_args.insert(
				"x-dead-letter-exchange".into(),
				lapin::types::AMQPValue::LongString(dlx_exchange.into()),
			);
			queue_args.insert(
				"x-dead-letter-routing-key".into(),
				lapin::types::AMQPValue::LongString("failed".into()),
			);

			let queue = match channel
				.queue_declare(
					self.queue_name.as_str(),
					QueueDeclareOptions {
						durable: true,
						exclusive: false,
						auto_delete: false,
						..QueueDeclareOptions::default()
					},
					queue_args,
				)
				.await
			{
				Ok(q) => q,
				Err(e) => {
					error!("Failed to declare queue: {}", e);
					continue;
				}
			};

			if let Err(e) = channel.basic_qos(1, BasicQosOptions::default()).await {
				error!("Failed to set QoS: {}", e);
				continue;
			}

			let mut consumer = match channel
				.basic_consume(
					queue.name().as_str(),
					"obligation_crawler_consumer",
					BasicConsumeOptions::default(),
					FieldTable::default(),
				)
				.await
			{
				Ok(c) => c,
				Err(e) => {
					error!("Failed to create consumer: {}", e);
					continue;
				}
			};

			info!(
				"Consumer started, waiting for messages on queue: {}",
				self.queue_name
			);

			tokio::select! {
				_ = shutdown_rx.recv() => {
					info!("Shutdown signal received, stopping consumer");
					break;
				}
				result = async {
					while let Some(message_result) = consumer.next().await {
						match message_result {
							Ok(delivery) => {
								info!("Received message");
								let message_str = String::from_utf8_lossy(&delivery.data).to_string();

								match message_handler(message_str).await {
									Ok(_) => {
										if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
											warn!("Failed to ack message: {}", e);
										} else {
											info!("Message acknowledged");
										}
									}
									Err(e) => {
										error!("Error handling delivery: {}", e);
										if let Err(nack_err) = delivery
											.nack(BasicNackOptions {
												requeue: false,
												..BasicNackOptions::default()
											})
											.await
										{
											warn!("Failed to nack message: {}", nack_err);
										} else {
											info!("Message sent to DLQ");
										}
									}
								}
							}
							Err(e) => {
								error!("Error receiving delivery: {}", e);
								break;
							}
						}
					}
					Ok::<(), CrawlerError>(())
				} => {
					if let Err(e) = result {
						warn!("Consumer stream ended with error: {}", e);
					}
				}
			}

			warn!("Consumer disconnected, will reconnect...");
		}

		Ok(())
	}
}
