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

use std::collections::HashMap;
use std::time::Duration;

use esp_idf_sys::EspError;
pub use serde_json::json;

mod device;

pub use device::Device;

const BROKER_URL_TCP: &str = "mqtt://broker.losant.com:1883";
const BROKER_URL_TLS: &str = "mqtts://broker.losant.com:8883";
const MAX_PAYLOAD_SIZE: usize = 256_000;

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

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("a client ID must be provided")]
    MissingId,

    #[error(
        "invalid QoS: expected `AtMostOnce` (0) or `AtLeastOnce` (1), found `ExactlyOnce` (2)"
    )]
    QoS2NotSupported,

    #[error("payload exceeded maximum size of 256KB")]
    PayloadSize,
}
pub type Result<T> = std::result::Result<T, Error>;

/// A Losant `state` topic message. For the `time` field, see
/// [`EspSystemTime::now()`](esp_idf_svc::systime::EspSystemTime::now).
///
/// See <https://docs.losant.com/mqtt/overview/#publishing-device-state>
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State<'a> {
    #[serde(borrow)]
    pub data: HashMap<&'a str, &'a str>,
    pub time: Option<Duration>,
    pub flow_version: Option<&'a str>,
    #[serde(borrow)]
    pub meta: Option<HashMap<&'a str, &'a str>>,
}

/// A Losant `command` topic message.
///
/// See <https://docs.losant.com/mqtt/overview/#subscribing-to-commands>
#[derive(Debug, serde::Deserialize)]
pub struct Command<'a, T = HashMap<&'a str, &'a str>> {
    pub name: &'a str,
    pub payload: T,
}
