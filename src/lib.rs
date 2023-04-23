#![warn(
    clippy::all,
    clippy::cargo,
    clippy::nursery,
    clippy::pedantic,
    rust_2018_idioms
)]
#![forbid(unsafe_code)]
#![feature(trait_alias)]
#![doc = include_str!("../README.md")]

use std::time::Duration;

use esp_idf_sys::EspError;

pub mod client;
mod device;
pub mod serde;

pub use crate::device::{CommandHandler, ConfigUpdater, Device, EventResult, EventResultHandler};

pub mod prelude {
    pub use serde_json::json;

    pub use crate::client::Client as _;
    pub use crate::device::{
        CommandHandler, ConfigUpdater, Device, EventResult, EventResultHandler,
    };
    pub use crate::State;
}

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
#[derive(Debug, Default, Clone, PartialEq, Eq, ::serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State<Data, Time = Duration, FlowVersion = &'static str, Meta = ()>
where
    FlowVersion: AsRef<str>,
{
    pub data: Data,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<Time>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_version: Option<FlowVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}
