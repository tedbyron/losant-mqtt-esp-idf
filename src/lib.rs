#![warn(clippy::all, clippy::cargo, clippy::nursery, rust_2018_idioms)]
#![forbid(unsafe_code)]
#![feature(lazy_cell)]
#![doc = include_str!("../README.md")]

use embedded_svc::mqtt::client::{Event, MessageId, QoS};
use esp_idf_svc::mqtt::client::{EspMqttClient, EspMqttMessage, MqttClientConfiguration};
use esp_idf_sys::EspError;

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    pub username: &'static str,
    #[default("")]
    pub password: &'static str,
}

pub type Result<T> = std::result::Result<T, EspError>;

pub struct Device<'a> {
    pub mqtt_config: MqttClientConfiguration<'a>,
    pub mqtt_client: EspMqttClient,
}

impl<'a> Device<'a> {
    fn broker_url() -> String {
        let Config { username, password } = CONFIG;
        if username.is_empty() || password.is_empty() {
            panic!("username and password must be set in cfg.toml");
        } else {
            format!("mqtt://{username}:{password}@broker.losant.com")
        }
    }

    pub fn new<F>(
        callback: impl for<'b> FnMut(&'b Result<Event<EspMqttMessage<'b>>>) + Send + 'static,
    ) -> Result<Self> {
        let mqtt_config = MqttClientConfiguration::default();
        let mqtt_client = EspMqttClient::new(Self::broker_url(), &mqtt_config, callback)?;
        Ok(Self {
            mqtt_config,
            mqtt_client,
        })
    }

    pub fn with_config(
        config: MqttClientConfiguration<'a>,
        callback: impl for<'b> FnMut(&'b Result<Event<EspMqttMessage<'b>>>) + Send + 'static,
    ) -> Result<Self> {
        let mqtt_client = EspMqttClient::new(Self::broker_url(), &config, callback)?;
        Ok(Self {
            mqtt_config: config,
            mqtt_client,
        })
    }

    pub fn enqueue(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: &[u8],
    ) -> Result<MessageId> {
        self.mqtt_client
            .enqueue(topic.as_ref(), qos, retain, payload)
    }

    pub fn publish(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: &[u8],
    ) -> Result<MessageId> {
        self.mqtt_client
            .publish(topic.as_ref(), qos, retain, payload)
    }
}
