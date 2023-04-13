#![warn(clippy::all, clippy::nursery, rust_2018_idioms)]

use anyhow::Result;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::log::EspLogger;
use esp_idf_sys::EspError;

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    EspLogger::initialize_default();
    EspError::convert(unsafe { esp_idf_sys::nvs_flash_init() })?;

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    Ok(())
}
