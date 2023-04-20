#![warn(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic, rust_2018_idioms)]
#![forbid(unsafe_code)]
#![feature(trait_alias)]
#![doc = include_str!("../README.md")]

use std::{collections::HashMap, marker::PhantomData, time::Duration};

use esp_idf_sys::EspError;
pub use serde_json::json;

mod device;

pub use device::{Builder as DeviceBuilder, Device, MqttEventHandler};

#[toml_cfg::toml_config]
struct Config {
    #[default("")]
    losant_key: &'static str,
    #[default("")]
    losant_secret: &'static str,
    #[default("")]
    losant_device_id: &'static str,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Esp(#[from] EspError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("a device ID was not provided")]
    MissingId,

    #[error(
        "invalid QoS: expected `AtMostOnce` (0) or `AtLeastOnce` (1), found `ExactlyOnce` (2)"
    )]
    QoS2NotSupported,

    #[error("payload exceeded maximum size of 256KB")]
    PayloadSize,
}
pub type Result<T> = std::result::Result<T, Error>;

/// A serializable Losant `state` topic message. For the `time` field, see
/// `esp_idf_svc::systime::EspSystemTime::now()`.
///
/// See <https://docs.losant.com/mqtt/overview/#publishing-device-state>
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State<
    'a,
    Data = HashMap<&'a str, &'a str>,
    Time = Duration,
    FlowVersion = &'a str,
    Meta = HashMap<&'a str, &'a str>,
> {
    pub data: Data,
    pub time: Option<Time>,
    pub flow_version: Option<FlowVersion>,
    pub meta: Option<Meta>,

    phantom: PhantomData<&'a ()>,
}

/// A deserializable Losant `command` topic message.
///
/// See <https://docs.losant.com/mqtt/overview/#subscribing-to-commands>
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Command<'a, Name = &'a str, Payload = HashMap<&'a str, &'a str>> {
    pub name: Name,
    pub payload: Payload,

    phantom: PhantomData<&'a ()>,
}
