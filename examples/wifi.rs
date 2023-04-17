#![warn(clippy::all, clippy::nursery, clippy::pedantic, rust_2018_idioms)]
#![feature(iter_intersperse)]

use std::{net::Ipv4Addr, thread, time::Duration};

use anyhow::{anyhow, bail, Result};
use embedded_svc::{
    mqtt::client::{Details, Event},
    wifi::{ClientConfiguration, Configuration, Wifi},
};
use esp_idf_hal::{modem::Modem, peripheral::Peripheral, prelude::Peripherals};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    log::EspLogger,
    mqtt::client::EspMqttMessage,
    netif::{EspNetif, EspNetifWait},
    wifi::{EspWifi, WifiWait},
};
use esp_idf_sys::EspError;
use esp_losant_mqtt::Device;

#[toml_cfg::toml_config]
struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
}

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    EspLogger::initialize_default();
    EspError::convert(unsafe { esp_idf_sys::nvs_flash_init() })?;

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let (_wifi, mac) = wifi(peripherals.modem, &sysloop)?;
    println!("Using wifi interface MAC address {mac} as Losant device ID",);

    let _device = Device::builder()
        .id(&mac)
        .event_handler(|event| match event {
            Ok(Event::Received(message)) => on_message(message),
            Ok(event) => println!("MQTT event: {event:?}"),
            Err(e) => eprintln!("MQTT error: {e}"),
        })
        .build()?;

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

fn on_message(msg: &EspMqttMessage<'_>) {
    if *msg.details() == Details::Complete {
        println!("MQTT message: {msg:?}");
    }
}

fn wifi(
    modem: impl Peripheral<P = Modem> + 'static,
    sysloop: &EspSystemEventLoop,
) -> Result<(Box<EspWifi<'static>>, String)> {
    let mut wifi = Box::new(EspWifi::new(modem, sysloop.clone(), None)?);
    let ap = wifi
        .scan()?
        .into_iter()
        .find(|ap| ap.ssid == CONFIG.wifi_ssid)
        .ok_or_else(|| anyhow!("configured SSID not found during wifi scan"))?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ap.ssid,
        password: CONFIG.wifi_psk.into(),
        channel: Some(ap.channel),
        auth_method: ap.auth_method,
        ..ClientConfiguration::default()
    }))?;

    wifi.start()?;
    if !WifiWait::new(sysloop)?
        .wait_with_timeout(Duration::from_secs(20), || wifi.is_started().unwrap_or(true))
    {
        bail!("wifi did not start");
    }

    wifi.connect()?;
    if !EspNetifWait::new::<EspNetif>(wifi.sta_netif(), sysloop)?.wait_with_timeout(
        Duration::from_secs(20),
        || {
            wifi.is_connected().unwrap_or(false)
                && wifi
                    .sta_netif()
                    .get_ip_info()
                    .map_or(true, |info| info.ip != Ipv4Addr::UNSPECIFIED)
        },
    ) {
        bail!("wifi did not connect or did not receive a DHCP lease")
    }

    let mac = wifi
        .sta_netif()
        .get_mac()?
        .into_iter()
        .map(|octet| format!("{octet:02X}"))
        .intersperse(":".to_owned())
        .collect::<String>();
    Ok((wifi, mac))
}
