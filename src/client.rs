use embedded_svc::mqtt::client::{MessageId, QoS};

use crate::Result;

pub trait Client {
    /// Publish a message to the broker. `QoS::AtMostOnce` (0) or
    /// `QoS::AtLeastOnce` (1) must be used.
    ///
    /// # Errors
    ///
    /// - if `QoS::ExactlyOnce` (2) is used
    /// - if the payload is larger than 256KB
    /// - if there was an error publishing the payload
    fn publish(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: impl AsRef<[u8]>,
    ) -> Result<MessageId>;

    /// Enqueue a message to be sent later (non-blocking `publish()`).
    ///
    /// # Errors
    ///
    /// - if `QoS::ExactlyOnce` (2) is used
    /// - if the payload is larger than 256KB
    /// - if there was an error publishing the payload
    fn enqueue(
        &mut self,
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: impl AsRef<[u8]>,
    ) -> Result<MessageId>;

    /// Publish device state to the broker. `QoS::AtMostOnce` (0) or
    /// `QoS::AtLeastOnce` (1) must be used.
    ///
    /// Takes a reference `state` to allow its reuse.
    ///
    /// # Errors
    ///
    /// - if `QoS::ExactlyOnce` (2) is used
    /// - if the payload is larger than 256KB
    /// - if there was an error serializing `state`
    /// - if there was an error publishing the payload
    ///
    /// See <https://docs.losant.com/mqtt/overview/#publishing-device-state>
    fn send_state<S>(&mut self, qos: QoS, retain: bool, state: &S) -> Result<MessageId>
    where
        S: serde::Serialize;

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
    fn send_state_json(
        &mut self,
        qos: QoS,
        retain: bool,
        state: serde_json::Value,
    ) -> Result<MessageId>;

    /// Subscribe to the `topic`. `QoS::AtMostOnce` (0) is used.
    ///
    /// # Errors
    ///
    /// - if there was an error subscribing to the topic
    fn subscribe(&mut self, topic: impl AsRef<str>) -> Result<MessageId>;

    /// Unsubscribe from the `topic`.
    ///
    /// # Errors
    ///
    /// - if there was an error unsubscribing from the topic
    fn unsubscribe(&mut self, topic: impl AsRef<str>) -> Result<MessageId>;
}
