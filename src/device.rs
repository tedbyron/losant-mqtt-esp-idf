use embedded_svc::mqtt::client::{Event, MessageId, QoS};
use esp_idf_svc::mqtt::client::{
    EspMqttClient, EspMqttMessage, MqttClientConfiguration, MqttProtocolVersion,
};
use esp_idf_sys::EspError;

use crate::{Error, Result, State};

type BoxedEventHandler = Box<
    dyn for<'b> FnMut(&'b std::result::Result<Event<EspMqttMessage<'b>>, EspError>)
        + Send
        + 'static,
>;

/// TODO: docs
pub struct Device<'a> {
    state_topic_form: String,
    pub config: MqttClientConfiguration<'a>,
    client: EspMqttClient,
}

impl<'a> Device<'a> {
    /// Create a [`Builder`] for building a [`Device`].
    #[inline]
    #[must_use]
    pub fn builder() -> Builder<'a> {
        Builder { id: None, secure: true, event_handler: None, config: None }
    }

    /// Publish a message to the broker. [`QoS::AtMostOnce`] (0) or
    /// [`QoS::AtLeastOnce`] (1) must be used.
    ///
    /// # Errors
    ///
    /// - if [`QoS::ExactlyOnce`] (2) is used
    /// - if the payload is larger than 256KB
    /// - if there was an error publishing the payload
    pub fn publish(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: &[u8],
    ) -> Result<MessageId> {
        Self::check_publish(qos, payload)?;
        self.client.publish(topic.as_ref(), qos, retain, payload).map_err(Error::from)
    }

    /// Enqueue a message to be sent later (non-blocking
    /// [`publish`](Device::publish)).
    ///
    /// # Errors
    ///
    /// - if [`QoS::ExactlyOnce`] (2) is used
    /// - if the payload is larger than 256KB
    /// - if there was an error publishing the payload
    pub fn enqueue(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: &[u8],
    ) -> Result<MessageId> {
        Self::check_publish(qos, payload)?;
        self.client.enqueue(topic.as_ref(), qos, retain, payload).map_err(Error::from)
    }

    /// Publish device state to the broker. [`QoS::AtMostOnce`] (0) or
    /// [`QoS::AtLeastOnce`] (1) must be used.
    ///
    /// # Errors
    ///
    /// - if [`QoS::ExactlyOnce`] (2) is used
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

    /// Publish device state to the broker. [`QoS::AtMostOnce`] (0) or
    /// [`QoS::AtLeastOnce`] (1) must be used.
    ///
    /// # Errors
    ///
    /// - if [`QoS::ExactlyOnce`] (2) is used
    /// - if the payload is larger than 256KB
    /// - if there was an error publishing the payload
    ///
    /// See <https://docs.losant.com/mqtt/overview/#publishing-device-state>
    pub fn publish_state_json(
        &mut self,
        qos: QoS,
        retain: bool,
        state: &serde_json::Value,
    ) -> Result<MessageId> {
        let payload = state.to_string();
        let payload = payload.as_bytes();
        Self::check_publish(qos, payload)?;
        self.client.publish(&self.state_topic_form, qos, retain, payload).map_err(Error::from)
    }

    /// Subscribe to the `topic`. [`QoS::AtMostOnce`] (0) is used.
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

    /// Create Losant state and command topic forms using the specified `id`.
    #[inline]
    fn topic_forms(id: &str) -> (String, String) {
        (format!("losant/{id}/state"), format!("losant/{id}/command"))
    }

    /// Check QoS and payload size for use in message publishing functions.
    #[inline]
    #[allow(clippy::doc_markdown)]
    fn check_publish(qos: QoS, payload: &[u8]) -> Result<()> {
        if qos == QoS::ExactlyOnce {
            return Err(Error::QoS2NotSupported);
        }

        if payload.len() > crate::MAX_PAYLOAD_SIZE {
            return Err(Error::PayloadSize);
        }

        Ok(())
    }
}

/// TODO: docs
#[derive(Default)]
pub struct Builder<'a> {
    id: Option<&'a str>,
    secure: bool,
    event_handler: Option<BoxedEventHandler>,
    config: Option<MqttClientConfiguration<'a>>,
}

impl<'a> Builder<'a> {
    /// Sets the client ID.
    #[inline]
    #[must_use]
    pub const fn id(mut self, id: &'a str) -> Self {
        self.id = Some(id);
        self
    }

    /// If `true` or if this function is not called, TLS will be used (mqtts,
    /// port 8883). If `false`, TCP will be used (mqtt, port 1883).
    #[inline]
    #[must_use]
    pub const fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Sets the MQTT event handler.
    #[must_use]
    pub fn event_handler(
        mut self,
        event_handler: impl for<'b> FnMut(&'b std::result::Result<Event<EspMqttMessage<'b>>, EspError>)
            + Send
            + 'static,
    ) -> Self {
        self.event_handler = Some(Box::new(event_handler));
        self
    }

    /// Sets the [`MqttClientConfiguration`].
    #[inline]
    #[must_use]
    pub const fn config(mut self, config: MqttClientConfiguration<'a>) -> Self {
        self.config = Some(config);
        self
    }

    /// Consumes the [`Builder`] and creates a [`Device`].
    ///
    /// # Errors
    ///
    /// - if a client ID was not provided
    /// - if the MQTT client could not be constructed
    /// - if the client failed to subscribe to the Losant `command` topic
    #[allow(clippy::missing_panics_doc)]
    pub fn build(self) -> Result<Device<'a>> {
        let mut config = self.config.unwrap_or_else(|| MqttClientConfiguration {
            protocol_version: Some(MqttProtocolVersion::V3_1_1),
            username: Some(crate::CONFIG.losant_key),
            password: Some(crate::CONFIG.losant_secret),
            ..MqttClientConfiguration::default()
        });

        // client_id can be set from id() or config(); prefer id() if called
        if self.id.is_some() {
            config.client_id = self.id;
        }
        if config.client_id.is_none() {
            return Err(Error::MissingId);
        }

        // unwrap: client_id is checked above
        let (state_topic_form, command_topic_form) = Device::topic_forms(config.client_id.unwrap());

        let mut device = Device {
            state_topic_form,
            client: EspMqttClient::new(
                if self.secure { crate::BROKER_URL_TLS } else { crate::BROKER_URL_TCP },
                &config,
                self.event_handler.unwrap_or_else(|| Box::new(|_| ())),
            )?,
            config,
        };
        device.subscribe(command_topic_form)?;

        Ok(device)
    }
}
