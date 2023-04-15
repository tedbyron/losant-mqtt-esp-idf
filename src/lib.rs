#![warn(clippy::all, clippy::cargo, clippy::nursery, rust_2018_idioms)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

use embedded_svc::mqtt::client::{Event, MessageId, QoS};
use esp_idf_svc::mqtt::client::{EspMqttClient, EspMqttMessage, MqttClientConfiguration};
use esp_idf_sys::EspError;

pub const BROKER_HOST: &str = "broker.losant.com";
pub const BROKER_PORT: u16 = 1883;
pub const BROKER_PORT_SECURE: u16 = 8883;
pub const TOPIC_FORMAT_STATE: &str = "losant/{}/state";
pub const TOPIC_FORMAT_MESSAGE: &str = "losant/{}/command";
pub const MAX_PACKET_SIZE: u16 = 256;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    EspError(#[from] EspError),

    #[error(
        "Packet size of {0} bytes exceeded maximum of {} bytes",
        MAX_PACKET_SIZE
    )]
    PacketSize(usize),
}

#[toml_cfg::toml_config]
struct Config {
    #[default("")]
    losant_username: &'static str,
    #[default("")]
    losant_password: &'static str,
}

pub struct Device<'a> {
    id: String,

    _config: MqttClientConfiguration<'a>,
    client: EspMqttClient,
}

impl<'a> Device<'a> {
    pub fn new<F>(
        id: impl ToString,
        callback: impl for<'b> FnMut(&'b Result<Event<EspMqttMessage<'b>>, EspError>) + Send + 'static,
    ) -> Result<Self, EspError> {
        let config = MqttClientConfiguration::default();

        Ok(Self {
            id: id.to_string(),
            client: EspMqttClient::new(Self::broker_url(), &config, callback)?,
            _config: config,
        })
    }

    pub fn with_config(
        id: impl ToString,
        config: MqttClientConfiguration<'a>,
        callback: impl for<'b> FnMut(&'b Result<Event<EspMqttMessage<'b>>, EspError>) + Send + 'static,
    ) -> Result<Self, EspError> {
        Ok(Self {
            id: id.to_string(),
            client: EspMqttClient::new(Self::broker_url(), &config, callback)?,
            _config: config,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn enqueue(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: &[u8],
    ) -> Result<MessageId, EspError> {
        self.client.enqueue(topic.as_ref(), qos, retain, payload)
    }

    pub fn publish(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: &[u8],
    ) -> Result<MessageId, EspError> {
        self.client.publish(topic.as_ref(), qos, retain, payload)
    }

    pub fn subscribe(&mut self, topic: impl AsRef<str>, qos: QoS) -> Result<MessageId, EspError> {
        self.client.subscribe(topic.as_ref(), qos)
    }

    pub fn unsubscribe(&mut self, topic: impl AsRef<str>) -> Result<MessageId, EspError> {
        self.client.unsubscribe(topic.as_ref())
    }

    fn broker_url() -> String {
        format!(
            "mqtt://{}:{}@{BROKER_HOST}",
            CONFIG.losant_username, CONFIG.losant_password
        )
    }
}
