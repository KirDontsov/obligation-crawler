use crate::error::CrawlerError;
use futures_util::stream::StreamExt;
use lapin::{message::Delivery, options::*, types::FieldTable, Connection, ConnectionProperties};
use tokio::time::{sleep, Duration};

pub struct RabbitMQConsumer {
	connection_string: String,
	queue_name: String,
}

impl RabbitMQConsumer {
	pub fn new(connection_string: String, queue_name: String) -> Self {
		Self {
			connection_string,
			queue_name,
		}
	}

	pub async fn start_consuming<F>(&self, mut message_handler: F) -> Result<(), CrawlerError>
	where
		F: FnMut(String) -> futures::future::BoxFuture<'static, Result<(), CrawlerError>>
			+ Send
			+ Sync
			+ 'static,
	{
		loop {
			println!("Connecting to RabbitMQ at: {}", self.connection_string);

			let connection =
				match Connection::connect(&self.connection_string, ConnectionProperties::default())
					.await
				{
					Ok(conn) => {
						println!("✅ Connected to RabbitMQ");
						conn
					}
					Err(e) => {
						eprintln!(
							"❌ Failed to connect to RabbitMQ: {}. Retrying in 5 seconds...",
							e
						);
						sleep(Duration::from_secs(5)).await;
						continue;
					}
				};

			let channel = match connection.create_channel().await {
				Ok(chan) => chan,
				Err(e) => {
					eprintln!(
						"❌ Failed to create channel: {}. Retrying in 5 seconds...",
						e
					);
					sleep(Duration::from_secs(5)).await;
					continue;
				}
			};

			let queue = match channel
				.queue_declare(
					self.queue_name.as_str(),
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
					eprintln!(
						"❌ Failed to declare queue: {}. Retrying in 5 seconds...",
						e
					);
					sleep(Duration::from_secs(5)).await;
					continue;
				}
			};

			if let Err(e) = channel.basic_qos(1, BasicQosOptions::default()).await {
				eprintln!("❌ Failed to set QoS: {}. Retrying in 5 seconds...", e);
				sleep(Duration::from_secs(5)).await;
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
					eprintln!(
						"❌ Failed to create consumer: {}. Retrying in 5 seconds...",
						e
					);
					sleep(Duration::from_secs(5)).await;
					continue;
				}
			};

			println!("🚀 Consumer started. Waiting for messages...");

			while let Some(message_result) = consumer.next().await {
				match message_result {
					Ok(delivery) => {
						println!("📨 Received message");
						let message_str = String::from_utf8_lossy(&delivery.data).to_string();

						if let Err(e) = message_handler(message_str).await {
							eprintln!("❌ Error handling delivery: {}", e);
						}

						let _ = delivery.ack(BasicAckOptions::default()).await;
						println!("✅ Message acknowledged");
					}
					Err(e) => {
						eprintln!("❌ Error receiving delivery: {}", e);
						break;
					}
				}
			}

			sleep(Duration::from_secs(5)).await;
		}
	}
}
