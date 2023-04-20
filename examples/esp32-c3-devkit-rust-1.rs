#![warn(clippy::all, clippy::nursery, clippy::pedantic, rust_2018_idioms)]
#![forbid(unsafe_code)]

use std::{thread, time::Duration};

use embedded_svc::mqtt::client::{Details, Event, QoS};
use esp_idf_hal::{
    delay::Ets,
    i2c::{config::Config as I2cConfig, I2cDriver},
    prelude::Peripherals,
};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop, log::EspLogger, mqtt::client::EspMqttMessage,
    nvs::EspDefaultNvsPartition,
};
use losant_mqtt_esp_idf::{json, Device};

mod util;

use util::{
    led::{Ws2812Rmt, RGB8},
    wifi,
};

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    EspLogger::initialize_default();
    EspDefaultNvsPartition::take()?; // must initialize nvs for wifi

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let _wifi = wifi::connect(peripherals.modem, &sysloop)?; // don't drop or wifi will disconnect
    let mut led = Ws2812Rmt::new(peripherals.pins.gpio2, peripherals.rmt.channel0)?;
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio10,
        peripherals.pins.gpio8,
        &I2cConfig::default(),
    )?;
    let mut shtc3 = shtcx::shtc3(i2c);
    shtc3.reset(&mut Ets).unwrap();
    let shtc3_id = &format!("{:#02X}", shtc3.device_identifier().unwrap());

    // create a new `Device` to connect to Losant
    //
    // the device will automatically connect to the broker and subscribe to the
    // command topic
    //
    // you can set the device ID with, in order of precedence: id(),
    // losant_device_id in cfg.toml, or the client_id field of config()
    //
    // defaults to using TLS, but you can disable it with secure(false)
    let mut device = Device::builder()
        .event_handler(|event| match event {
            Ok(Event::Received(message)) => on_message(message),
            Ok(event) => println!("MQTT event: {event:?}"),
            Err(e) => eprintln!("MQTT error: {e}"),
        })
        .build()?;

    led.set(RGB8::new(0, 20, 0))?;

    // main loop
    loop {
        if let Ok(measurement) = shtc3.measure(shtcx::PowerMode::NormalMode, &mut Ets) {
            let temperature = measurement.temperature.as_degrees_celsius();
            let humidity = measurement.humidity.as_percent();

            // for payload format, see https://docs.losant.com/mqtt/overview/#publishing-device-state
            device.publish_state_json(
                QoS::AtLeastOnce,
                false,
                json!({
                    "data": {
                        "temperature": temperature,
                        "humidity": humidity,
                    },
                    "meta": {
                        "sensorDeviceIdentifier": shtc3_id,
                    }
                }),
            )?;
        }

        thread::sleep(Duration::from_secs(60));
    }
}

fn on_message(msg: &EspMqttMessage<'_>) {
    if *msg.details() == Details::Complete {
        println!("MQTT message: {msg:?}");
    }
}
