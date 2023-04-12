#![warn(clippy::all, clippy::cargo, clippy::nursery, rust_2018_idioms)]
#![forbid(unsafe_code)]
#![feature(lazy_cell)]
#![doc = include_str!("../README.md")]

use std::borrow::Cow;
use std::cell::LazyCell;

use embedded_svc::mqtt::client::{Details, Event, MessageId, QoS};
use esp_idf_svc::mqtt::client::{EspMqttClient, EspMqttMessage, MqttClientConfiguration};
use esp_idf_sys::EspError;

mod config;

pub const BROKER_URL: LazyCell<Cow<str>> = LazyCell::new(|| {
    let MqttConfig { username, password } = CONFIG.mqtt;
    if username.is_empty() {
        Cow::Borrowed("mqtt://broker.losant.com")
    } else {
        Cow::Owned(format!("mqtt://{username}:{password}@broker.losant.com"))
    }
});

pub struct Device<'a> {
    pub mqtt_config: MqttClientConfiguration,
    pub mqtt_client: EspMqttClient,
}

impl<'a> Device<'a> {
    pub fn new() -> Self {
        Self {
            mqtt_config: MqttClientConfiguration::default(),
            mqtt_client: EspMqttClient::new_with_conn(*BROKER_URL, &mqtt_config),
        }
    }

    pub fn with_conn(config: &'a MqttClientConfiguration<'a>) -> Self {
        Self {
            mqtt_config: MqttClientConfiguration::default(),
            mqtt_client: EspMqttClient::new_with_conn(*BROKER_URL, config),
        }
    }

    pub fn enqueue(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: &[u8],
    ) -> Result<MessageId, EspError> {
        self.mqtt_client
            .enqueue(topic.as_ref(), qos, retain, payload)
    }

    pub fn publish(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: &[u8],
    ) -> Result<MessageId, EspError> {
        self.mqtt_client
            .publish(topic.as_ref(), qos, retain, payload)
    }
}

pub fn topic_builder(uuid: impl AsRef<str>) -> impl Fn(impl AsRef<str>) -> String {
    move |topic| format!("{}/{}", uuid.as_ref(), topic)
}
