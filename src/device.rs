use std::time::Duration;

use embedded_svc::mqtt::client::{Event, MessageId, QoS};
use esp_idf_svc::mqtt::client::{
    EspMqttClient, EspMqttMessage, MqttClientConfiguration, MqttProtocolVersion,
};
use esp_idf_svc::tls::X509;
use esp_idf_sys::EspError;

use crate::{client::Client, Error, Result};

const BROKER_HOST: &str = "broker.losant.com";
/// See <https://docs.losant.com/mqtt/overview/#message-limits>
const MAX_PAYLOAD_SIZE: usize = 256_000;
/// DigiCert Global Root CA certificate.
#[allow(clippy::doc_markdown)]
const ROOT_CA_CERT: X509<'_> =
    X509::pem_until_nul(concat!(include_str!("RootCA.crt"), '\0').as_bytes());

pub type EventResult<'a> = std::result::Result<Event<EspMqttMessage<'a>>, EspError>;
pub trait EventResultHandler = for<'b> FnMut(&'b EventResult<'b>) + Send + 'static;
pub trait ConfigUpdater = FnOnce(&mut MqttClientConfiguration<'_>) + 'static;
pub trait CommandHandler<Command> = for<'b> FnMut(&'b Command) + Send + 'static;

// TODO: docs
pub struct Device<'a> {
    state_topic: String,
    pub config: MqttClientConfiguration<'a>,
    client: EspMqttClient,
}

impl<'a> Device<'a> {
    /// Create a `Builder` for building a `Device`.
    #[inline]
    #[must_use]
    pub fn builder<Command>() -> Builder<'a, Command> {
        Builder {
            id: None,
            secure: true,
            handler: None,
            command_handler: None,
            config: None,
        }
    }

    /// Check QoS and payload size for use in message publishing functions.
    #[inline]
    #[allow(clippy::doc_markdown)]
    fn check_publish(qos: QoS, payload: &[u8]) -> Result<()> {
        if qos == QoS::ExactlyOnce {
            return Err(Error::QoS2NotSupported);
        }

        if payload.len() > MAX_PAYLOAD_SIZE {
            return Err(Error::PayloadSize);
        }

        Ok(())
    }
}

impl<'a> Client for Device<'a> {
    fn publish(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: impl AsRef<[u8]>,
    ) -> Result<MessageId> {
        let payload = payload.as_ref();
        Self::check_publish(qos, payload)?;
        self.client
            .publish(topic.as_ref(), qos, retain, payload)
            .map_err(Error::from)
    }

    fn enqueue(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: impl AsRef<[u8]>,
    ) -> Result<MessageId> {
        let payload = payload.as_ref();
        Self::check_publish(qos, payload)?;
        self.client
            .enqueue(topic.as_ref(), qos, retain, payload)
            .map_err(Error::from)
    }

    fn send_state<S>(&mut self, qos: QoS, retain: bool, state: &S) -> Result<MessageId>
    where
        S: serde::Serialize,
    {
        let payload = serde_json::to_string(&state).map_err(Error::from)?;
        let payload = payload.as_bytes();
        Self::check_publish(qos, payload)?;
        self.client
            .publish(&self.state_topic, qos, retain, payload)
            .map_err(Error::from)
    }

    fn send_state_json(
        &mut self,
        qos: QoS,
        retain: bool,
        state: serde_json::Value,
    ) -> Result<MessageId> {
        let payload = state.to_string();
        let payload = payload.as_bytes();
        Self::check_publish(qos, payload)?;
        self.client
            .publish(&self.state_topic, qos, retain, payload)
            .map_err(Error::from)
    }

    fn subscribe(&mut self, topic: impl AsRef<str>) -> Result<MessageId> {
        self.client
            .subscribe(topic.as_ref(), QoS::AtMostOnce)
            .map_err(Error::from)
    }

    fn unsubscribe(&mut self, topic: impl AsRef<str>) -> Result<MessageId> {
        self.client.unsubscribe(topic.as_ref()).map_err(Error::from)
    }
}

// TODO: docs
#[derive(Default)]
pub struct Builder<'a, Command> {
    id: Option<&'a str>,
    secure: bool,
    handler: Option<Box<dyn EventResultHandler>>,
    command_handler: Option<Box<dyn CommandHandler<Command>>>,
    config: Option<Box<dyn ConfigUpdater>>,
}

impl<'a, Command> Builder<'a, Command>
where
    Command: for<'de> serde::Deserialize<'de> + 'static,
{
    /// Sets the device ID. This ID is preferred over `client_id` set in
    /// `config()` or `losant_device_id` in cfg.toml.
    #[inline]
    #[must_use]
    pub const fn id(mut self, id: &'a str) -> Self {
        self.id = Some(id);
        self
    }

    /// If set `false`, TCP will be used (mqtt, port 1883). Otherwise, TLS will
    /// be used (mqtts, port 8883)
    #[inline]
    #[must_use]
    pub const fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Sets the handler for all MQTT events except Losant commands, which are
    /// intercepted by `command_handler()`.
    #[must_use]
    pub fn handler(mut self, handler: impl EventResultHandler) -> Self {
        self.handler = Some(Box::new(handler));
        self
    }

    /// Sets the handler for all Losant command messages.
    #[must_use]
    pub fn command_handler(mut self, handler: impl CommandHandler<Command>) -> Self {
        self.command_handler = Some(Box::new(handler));
        self
    }

    /// Updates the `MqttClientConfiguration` using the provided closure, after
    /// the config is built. If `client_id` is set, it will have lower
    /// priority than `id()` or `losant_device_id` in cfg.toml.
    #[must_use]
    pub fn config(mut self, updater: impl ConfigUpdater) -> Self {
        self.config = Some(Box::new(updater));
        self
    }

    /// Consumes the `Builder` to create a `Device`.
    ///
    /// # Errors
    ///
    /// - if a device ID was not provided
    /// - if the MQTT client could not be constructed
    /// - if the client failed to subscribe to the Losant `command` topic
    #[allow(clippy::missing_panics_doc)]
    pub fn build(self) -> Result<Device<'a>> {
        let mut config = MqttClientConfiguration {
            // https://docs.losant.com/mqtt/overview/#mqtt-version-and-limitations
            protocol_version: Some(MqttProtocolVersion::V3_1_1),
            // https://docs.losant.com/devices/overview/#connection-log
            keep_alive_interval: Some(Duration::from_secs(90)),
            username: Some(crate::CONFIG.losant_key),
            password: Some(crate::CONFIG.losant_secret),
            server_certificate: Some(ROOT_CA_CERT),
            ..MqttClientConfiguration::default()
        };

        if let Some(config_fn) = self.config {
            config_fn(&mut config);
        }

        if self.id.is_some() {
            config.client_id = self.id;
        } else if !crate::CONFIG.losant_device_id.is_empty() {
            config.client_id = Some(crate::CONFIG.losant_device_id);
        }

        let mut handler = self.handler.unwrap_or_else(|| Box::new(|_| {}));
        let mut command_handler = self.command_handler.unwrap_or_else(|| Box::new(|_| {}));
        let (state_topic, command_topic) = Self::topics(config.client_id.ok_or(Error::MissingId)?);
        let client = EspMqttClient::new(
            format!("mqtt{}://{BROKER_HOST}", if self.secure { "s" } else { "" }),
            &config,
            {
                let command_topic = command_topic.clone();
                move |event| {
                    if let Ok(Event::Connected(_)) = event {}

                    if let Ok(Event::Received(msg)) = event {
                        if let Some(topic) = msg.topic() {
                            if topic == &*command_topic {
                                if let Ok(command) = serde_json::from_slice::<Command>(msg.data()) {
                                    {
                                        command_handler(&command);
                                    }
                                }

                                return;
                            }
                        }
                    }

                    handler(event);
                }
            },
        )?;
        let mut device = Device {
            state_topic,
            config,
            client,
        };

        device.subscribe(command_topic)?;

        Ok(device)
    }

    /// Create Losant state and command topic forms using the specified `id`.
    #[inline]
    fn topics(id: &str) -> (String, String) {
        (format!("losant/{id}/state"), format!("losant/{id}/command"))
    }
}
