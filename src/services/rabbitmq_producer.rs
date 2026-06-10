use crate::error::CrawlerError;
use lapin::{
	options::*, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties,
};

pub struct RabbitMQProducer {
	#[allow(dead_code)]
	connection: Connection,
	channel: Channel,
	exchange: String,
}

impl RabbitMQProducer {
	pub async fn new(connection_string: String, exchange: String) -> Result<Self, CrawlerError> {
		let connection = Connection::connect(&connection_string, ConnectionProperties::default())
			.await
			.map_err(CrawlerError::RabbitMQError)?;

		let channel = connection
			.create_channel()
			.await
			.map_err(CrawlerError::RabbitMQError)?;

		channel
			.exchange_declare(
				&exchange,
				lapin::ExchangeKind::Topic,
				ExchangeDeclareOptions {
					durable: true,
					..ExchangeDeclareOptions::default()
				},
				FieldTable::default(),
			)
			.await
			.map_err(CrawlerError::RabbitMQError)?;

		println!(
			"✅ RabbitMQ producer initialized with exchange: {}",
			exchange
		);

		Ok(Self {
			connection,
			channel,
			exchange,
		})
	}

	pub async fn publish(&self, routing_key: &str, message: &str) -> Result<(), CrawlerError> {
		self.channel
			.basic_publish(
				&self.exchange,
				routing_key,
				BasicPublishOptions::default(),
				message.as_bytes(),
				BasicProperties::default(),
			)
			.await
			.map_err(CrawlerError::RabbitMQError)?;

		println!("📤 Published message to {}:{}", self.exchange, routing_key);
		Ok(())
	}

	pub async fn publish_bonds_data(&self, bonds_json: &str) -> Result<(), CrawlerError> {
		self.publish("bonds.data", bonds_json).await
	}

	#[allow(dead_code)]
	pub async fn publish_error(&self, error_message: &str) -> Result<(), CrawlerError> {
		self.publish("bonds.error", error_message).await
	}
}
