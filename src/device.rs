use embedded_svc::mqtt::client::{Event, MessageId, QoS};
use esp_idf_svc::{
    mqtt::client::{EspMqttClient, EspMqttMessage, MqttClientConfiguration, MqttProtocolVersion},
    tls::X509,
};
use esp_idf_sys::EspError;

use crate::{Error, Result, State};

const BROKER_URL_TCP: &str = "mqtt://broker.losant.com:1883";
const BROKER_URL_TLS: &str = "mqtts://broker.losant.com:8883";

/// See <https://docs.losant.com/mqtt/overview/#message-limits>
const MAX_PAYLOAD_SIZE: usize = 256_000;

/// DigiCert Global Root CA certificate.
///
/// See <https://docs.losant.com/mqtt/overview/#root-ca-certificate>
#[allow(clippy::doc_markdown)]
const ROOT_CA_CERT: X509<'_> =
    X509::pem_until_nul(concat!(include_str!("RootCA.crt"), '\0').as_bytes());

pub trait MqttEventHandler =
    for<'b> FnMut(&'b std::result::Result<Event<EspMqttMessage<'b>>, EspError>) + Send + 'static;

// TODO: docs
pub struct Device<'a> {
    state_topic_form: String,
    pub config: MqttClientConfiguration<'a>,
    client: EspMqttClient,
}

impl<'a> Device<'a> {
    /// Create a `Builder` for building a `Device`.
    #[inline]
    #[must_use]
    pub fn builder() -> Builder<'a> {
        Builder { id: None, secure: true, event_handler: None, config: None }
    }

    /// Publish a message to the broker. `QoS::AtMostOnce` (0) or
    /// `QoS::AtLeastOnce` (1) must be used.
    ///
    /// # Errors
    ///
    /// - if `QoS::ExactlyOnce` (2) is used
    /// - if the payload is larger than 256KB
    /// - if there was an error publishing the payload
    pub fn publish(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: impl AsRef<[u8]>,
    ) -> Result<MessageId> {
        let payload = payload.as_ref();
        Self::check_publish(qos, payload)?;
        self.client.publish(topic.as_ref(), qos, retain, payload).map_err(Error::from)
    }

    /// Enqueue a message to be sent later (non-blocking `publish`).
    ///
    /// # Errors
    ///
    /// - if `QoS::ExactlyOnce` (2) is used
    /// - if the payload is larger than 256KB
    /// - if there was an error publishing the payload
    pub fn enqueue(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: impl AsRef<[u8]>,
    ) -> Result<MessageId> {
        let payload = payload.as_ref();
        Self::check_publish(qos, payload)?;
        self.client.enqueue(topic.as_ref(), qos, retain, payload).map_err(Error::from)
    }

    /// Publish device state to the broker. `QoS::AtMostOnce` (0) or
    /// `QoS::AtLeastOnce` (1) must be used.
    ///
    /// Takes a reference `state` to allow its reuse.
    ///
    /// # Errors
    ///
    /// - if `QoS::ExactlyOnce` (2) is used
    /// - if the payload is larger than 256KB
    /// - if there was an error publishing the payload
    ///
    /// See <https://docs.losant.com/mqtt/overview/#publishing-device-state>
    pub fn publish_state(
        &mut self,
        qos: QoS,
        retain: bool,
        state: &State<'_>,
    ) -> Result<MessageId> {
        let payload = serde_json::to_string(&state).map_err(Error::from)?;
        let payload = payload.as_bytes();
        Self::check_publish(qos, payload)?;
        self.client.publish(&self.state_topic_form, qos, retain, payload).map_err(Error::from)
    }

    /// Publish device state to the broker. `QoS::AtMostOnce` (0) or
    /// `QoS::AtLeastOnce` (1) must be used.
    ///
    /// If `state` needs to be reused, consider `publish_state()` instead.
    ///
    /// # Errors
    ///
    /// - if `QoS::ExactlyOnce` (2) is used
    /// - if the payload is larger than 256KB
    /// - if there was an error publishing the payload
    ///
    /// See <https://docs.losant.com/mqtt/overview/#publishing-device-state>
    #[allow(clippy::needless_pass_by_value)]
    pub fn publish_state_json(
        &mut self,
        qos: QoS,
        retain: bool,
        state: serde_json::Value,
    ) -> Result<MessageId> {
        let payload = state.to_string();
        let payload = payload.as_bytes();
        Self::check_publish(qos, payload)?;
        self.client.publish(&self.state_topic_form, qos, retain, payload).map_err(Error::from)
    }

    /// Subscribe to the `topic`. `QoS::AtMostOnce` (0) is used.
    ///
    /// # Errors
    ///
    /// - if there was an error subscribing to the topic
    pub fn subscribe(&mut self, topic: impl AsRef<str>) -> Result<MessageId> {
        self.client.subscribe(topic.as_ref(), QoS::AtMostOnce).map_err(Error::from)
    }

    /// Unsubscribe from the `topic`.
    ///
    /// # Errors
    ///
    /// - if there was an error unsubscribing from the topic
    pub fn unsubscribe(&mut self, topic: impl AsRef<str>) -> Result<MessageId> {
        self.client.unsubscribe(topic.as_ref()).map_err(Error::from)
    }

    /// Check QoS and payload size for use in message publishing functions.
    #[inline]
    #[allow(clippy::doc_markdown)] // complains about "QoS"
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

// TODO: docs
#[derive(Default)]
pub struct Builder<'a> {
    id: Option<&'a str>,
    secure: bool,
    event_handler: Option<Box<dyn MqttEventHandler>>,
    config: Option<MqttClientConfiguration<'a>>,
}

impl<'a> Builder<'a> {
    /// Sets the device ID. This ID is preferred over
    /// `losant_mqtt_esp_idf::CONFIG.losant_device_id` or `losant_device_id` in
    /// cfg.toml.
    #[inline]
    #[must_use]
    pub const fn id(mut self, id: &'a str) -> Self {
        self.id = Some(id);
        self
    }

    /// If `false`, TCP will be used (mqtt, port 1883). Otherwise, TLS will be
    /// used (mqtts, port 8883)
    #[inline]
    #[must_use]
    pub const fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Sets the handler for MQTT events.
    #[must_use]
    pub fn event_handler(mut self, event_handler: impl MqttEventHandler) -> Self {
        self.event_handler = Some(Box::new(event_handler));
        self
    }

    /// Sets the `MqttClientConfiguration`. If `client_id` is set, it will have
    /// lower priority than `id` or `losant_device_id` in your cfg.toml.
    #[inline]
    #[must_use]
    pub const fn config(mut self, config: MqttClientConfiguration<'a>) -> Self {
        self.config = Some(config);
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
        let mut config = self.config.unwrap_or_else(|| MqttClientConfiguration {
            protocol_version: Some(MqttProtocolVersion::V3_1_1),
            username: Some(crate::CONFIG.losant_key),
            password: Some(crate::CONFIG.losant_secret),
            server_certificate: Some(ROOT_CA_CERT),
            ..MqttClientConfiguration::default()
        });

        // client_id can be set from cfg.toml, id(), or config(); prefer id() if called
        if self.id.is_some() {
            config.client_id = self.id;
        } else if !crate::CONFIG.losant_device_id.is_empty() {
            config.client_id = Some(crate::CONFIG.losant_device_id);
        }
        if config.client_id.is_none() {
            return Err(Error::MissingId);
        }

        // unwrap: client_id is checked above
        let (state_topic_form, command_topic_form) = Self::topic_forms(config.client_id.unwrap());

        let mut device = Device {
            state_topic_form,
            client: EspMqttClient::new(
                if self.secure { BROKER_URL_TLS } else { BROKER_URL_TCP },
                &config,
                self.event_handler.unwrap_or_else(|| Box::new(|_| ())),
            )?,
            config,
        };
        device.subscribe(command_topic_form)?;

        Ok(device)
    }

    /// Create Losant state and command topic forms using the specified `id`.
    #[inline]
    fn topic_forms(id: &str) -> (String, String) {
        (format!("losant/{id}/state"), format!("losant/{id}/command"))
    }
}
