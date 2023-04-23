#![warn(clippy::all, clippy::nursery, clippy::pedantic, rust_2018_idioms)]
#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use embedded_svc::mqtt::client::{Details, Event, MessageId, QoS};
use esp_idf_hal::delay::Ets;
use esp_idf_hal::i2c::{config::Config as I2cConfig, I2cDriver};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use losant_mqtt_esp_idf::{json, Device, EventResult};

mod util;

use util::led::{Ws2812Rmt, RGB8};
use util::wifi;

// enum to describe possible Losant commands
#[derive(serde::Deserialize)]
#[serde(tag = "name", rename_all = "camelCase")]
enum Command {
    // this would be a command with "name": "setLed" (if rename_all = "camelCase")
    SetLed { r: u8, g: u8, b: u8 },
}

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    EspLogger::initialize_default();
    EspDefaultNvsPartition::take()?;

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let _wifi = wifi::connect(peripherals.modem, &sysloop)?;
    let led = Arc::new(Mutex::new(Ws2812Rmt::new(
        peripherals.pins.gpio2,
        peripherals.rmt.channel0,
    )?));
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio10,
        peripherals.pins.gpio8,
        &I2cConfig::default(),
    )?;
    let mut shtc3 = shtcx::shtc3(i2c);
    shtc3.reset(&mut Ets).unwrap();
    let shtc3_id = &format!("{:#02X}", shtc3.device_identifier().unwrap());

    // create a new Device
    //
    // the device will connect to the Losant broker and subscribe to commands
    //
    // you can set the device ID with, in order of priority: id(),
    // losant_device_id in cfg.toml, or the client_id field of config()
    //
    // defaults to using TLS, but you can disable it with secure(false)
    let mut device = Device::builder()
        .handler(|event: &EventResult<'_>| match event {
            Ok(Event::Received(msg)) => {
                if *msg.details() == Details::Complete {
                    println!("MQTT message: {msg:?}");
                }
            }
            Ok(event) => println!("MQTT event: {event:?}"),
            Err(e) => eprintln!("MQTT error: {e}"),
        })
        .command_handler({
            // led is wrapped in an Arc<Mutex<_>> so it can be moved into the
            // closure and mutated
            let led_ptr = led.clone();

            move |command: &Command| match *command {
                Command::SetLed { r, g, b } => {
                    let mut guard = led_ptr.lock().unwrap();
                    guard.set(RGB8::new(r, g, b)).unwrap();
                }
            }
        })
        .build()?;

    // send temperature and humidity values from the SHTC3 sensor
    let mut publish_shtc3_state = |temperature: f32, humidity: f32| -> Result<MessageId> {
        Ok(device.send_state_json(
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
        )?)
    };

    // main loop
    loop {
        if let Ok(measurement) = shtc3.measure(shtcx::PowerMode::NormalMode, &mut Ets) {
            let temperature = measurement.temperature.as_degrees_celsius();
            let humidity = measurement.humidity.as_percent();
            publish_shtc3_state(temperature, humidity)?;
        }

        thread::sleep(Duration::from_secs(60));
    }
}
