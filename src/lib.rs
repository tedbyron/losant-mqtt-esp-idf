#![warn(clippy::all, clippy::cargo, clippy::nursery, rust_2018_idioms)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

use dotenvy_macro::dotenv;
use embedded_svc::mqtt::client::{Event, MessageId, QoS};
use esp_idf_svc::mqtt::client::{EspMqttClient, EspMqttMessage, MqttClientConfiguration};
use esp_idf_sys::EspError;

pub const BROKER_HOST: &str = "broker.losant.com";
pub const BROKER_PORT: u16 = 1883;
pub const BROKER_PORT_SECURE: u16 = 8883;
pub const TOPIC_FORMAT_STATE: &str = "losant/{}/state";
pub const TOPIC_FORMAT_MESSAGE: &str = "losant/{}/command";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Esp(#[from] EspError),
}
pub type Result<T> = std::result::Result<T, Error>;
pub type EventResult<'a> = std::result::Result<Event<EspMqttMessage<'a>>, EspError>;

pub struct Device {
    client: EspMqttClient,
}

impl Device {
    fn broker_url() -> String {
        format!(
            "mqtt://{}:{}@{BROKER_HOST}",
            dotenv!("LOSANT_USERNAME"),
            dotenv!("LOSANT_PASSWORD")
        )
    }

    pub fn new<F>(
        callback: impl for<'b> FnMut(&'b EventResult<'b>) + Send + 'static,
    ) -> Result<Self> {
        Ok(Self {
            client: EspMqttClient::new(
                Self::broker_url(),
                &MqttClientConfiguration::default(),
                callback,
            )?,
        })
    }

    pub fn with_config<'a>(
        config: MqttClientConfiguration<'a>,
        callback: impl for<'b> FnMut(&'b EventResult<'b>) + Send + 'static,
    ) -> Result<Self> {
        Ok(Self {
            client: EspMqttClient::new(Self::broker_url(), &config, callback)?,
        })
    }

    pub fn enqueue(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: &[u8],
    ) -> Result<MessageId> {
        self.client
            .enqueue(topic.as_ref(), qos, retain, payload)
            .map_err(Into::into)
    }

    pub fn publish(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: &[u8],
    ) -> Result<MessageId> {
        self.client
            .publish(topic.as_ref(), qos, retain, payload)
            .map_err(Into::into)
    }

    pub fn subscribe(&mut self, topic: impl AsRef<str>, qos: QoS) -> Result<MessageId> {
        self.client
            .subscribe(topic.as_ref(), qos)
            .map_err(Into::into)
    }

    pub fn unsubscribe(&mut self, topic: impl AsRef<str>) -> Result<MessageId> {
        self.client.unsubscribe(topic.as_ref()).map_err(Into::into)
    }
}
