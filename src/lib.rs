#![warn(
    clippy::all,
    clippy::cargo,
    clippy::nursery,
    clippy::pedantic,
    rust_2018_idioms
)]
#![forbid(unsafe_code)]
#![feature(lazy_cell)]
#![doc = include_str!("../README.md")]

use std::sync::LazyLock;

use embedded_svc::mqtt::client::{Event, MessageId, QoS};
use esp_idf_svc::mqtt::client::{
    EspMqttClient, EspMqttMessage, MqttClientConfiguration, MqttProtocolVersion,
};
use esp_idf_sys::EspError;

const BROKER_HOST: &str = "broker.losant.com";
const TOPIC_FORMAT_STATE: &str = "losant/{}/state";
const TOPIC_FORMAT_MESSAGE: &str = "losant/{}/command";
const MAX_PAYLOAD_SIZE: usize = 256_000;

static BROKER_URL: LazyLock<String> = LazyLock::new(|| {
    format!(
        "mqtt://{}:{}@{BROKER_HOST}",
        CONFIG.losant_key, CONFIG.losant_secret,
    )
});

#[toml_cfg::toml_config]
struct Config {
    #[default("")]
    losant_key: &'static str,
    #[default("")]
    losant_secret: &'static str,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    EspError(#[from] EspError),

    #[error("invalid QoS: expected {expected}, found {}", *.found as u8)]
    QoSNotSupported { expected: &'static str, found: QoS },

    #[error("payload exceeded maximum size of 256KB")]
    PayloadSize,
}
pub type Result<T> = std::result::Result<T, Error>;

pub struct Device<'a> {
    pub config: MqttClientConfiguration<'a>,
    client: EspMqttClient,
}

impl<'a> Device<'a> {
    /// Create a new [`Device`] with the specified ID and MQTT event handler.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the MQTT client could not be constructed.
    pub fn new(
        client_id: &'a str,
        event_handler: impl for<'b> FnMut(&'b std::result::Result<Event<EspMqttMessage<'b>>, EspError>)
            + Send
            + 'static,
    ) -> crate::Result<Self> {
        let config = MqttClientConfiguration {
            protocol_version: Some(MqttProtocolVersion::V3_1_1),
            client_id: Some(client_id),
            username: Some(CONFIG.losant_key),
            password: Some(CONFIG.losant_secret),
            ..MqttClientConfiguration::default()
        };

        Ok(Self {
            client: EspMqttClient::new(&*BROKER_URL, &config, event_handler)?,
            config,
        })
    }

    /// Create a new [`Device`] with the specified MQTT event handler and
    /// [`MqttClientConfiguration`].
    ///
    /// # Errors
    ///
    /// Will return `Err` if the MQTT client could not be constructed.
    pub fn with_config(
        event_handler: impl for<'b> FnMut(&'b std::result::Result<Event<EspMqttMessage<'b>>, EspError>)
            + Send
            + 'static,
        config: MqttClientConfiguration<'a>,
    ) -> crate::Result<Self> {
        Ok(Self {
            client: EspMqttClient::new(&*BROKER_URL, &config, event_handler)?,
            config,
        })
    }

    /// Enqueue a message to be sent later (non-blocking
    /// [`publish`](Device::publish)).
    ///
    /// # Errors
    ///
    /// Will return `Err` if [`QoS::ExactlyOnce`] is used, if the payload is
    /// larger than 256KB, or if there was an error enqueuing the payload.
    pub fn enqueue(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: &[u8],
    ) -> crate::Result<MessageId> {
        if qos == QoS::ExactlyOnce {
            return Err(Error::QoSNotSupported {
                expected: "0 or 1",
                found: qos,
            });
        }

        if payload.len() > MAX_PAYLOAD_SIZE {
            return Err(Error::PayloadSize);
        }

        self.client
            .enqueue(topic.as_ref(), qos, retain, payload)
            .map_err(Into::into)
    }

    /// Publish a message to the broker. [`QoS::AtMostOnce`] (0) or
    /// [`QoS::AtLeastOnce`] (1) must be used.
    ///
    /// # Errors
    ///
    /// Will return `Err` if [`QoS::ExactlyOnce`] is used, if the payload is
    /// larger than 256KB, or if there was an error publishing the payload.
    pub fn publish(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: &[u8],
    ) -> crate::Result<MessageId> {
        if qos == QoS::ExactlyOnce {
            return Err(Error::QoSNotSupported {
                expected: "0 or 1",
                found: qos,
            });
        }

        if payload.len() > MAX_PAYLOAD_SIZE {
            return Err(Error::PayloadSize);
        }

        self.client
            .publish(topic.as_ref(), qos, retain, payload)
            .map_err(Into::into)
    }

    /// Subscribe the device to the specified `topic`. [`QoS::AtMostOnce`] (0)
    /// is used.
    ///
    /// # Errors
    ///
    /// Will return `Err` if there was an error subscribing to the topic.
    pub fn subscribe(&mut self, topic: impl AsRef<str>) -> crate::Result<MessageId> {
        self.client
            .subscribe(topic.as_ref(), QoS::AtMostOnce)
            .map_err(Into::into)
    }

    /// Unsubscribe the device from the specified `topic`.
    ///
    /// # Errors
    ///
    /// Will return `Err` if there was an error unsubscribing from the topic.
    pub fn unsubscribe(&mut self, topic: impl AsRef<str>) -> crate::Result<MessageId> {
        self.client.unsubscribe(topic.as_ref()).map_err(Into::into)
    }
}
