use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use embedded_svc::mqtt::client::{Details, Event, QoS};
use esp_idf_hal::delay::Ets;
use esp_idf_hal::i2c::{config::Config as I2cConfig, I2cDriver};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use losant_mqtt_esp_idf::prelude::*;

mod util;

use util::led::{Ws2812Rmt, RGB8};
use util::wifi;

// adjacently tagged enum to describe possible Losant commands
#[derive(serde::Deserialize)]
#[serde(tag = "name", content = "payload", rename_all = "camelCase")]
enum Command {
    // e.g. { "name": "setLed", "payload": { "ledR": 0, "ledG": 0, "ledB": 0 }}
    SetLed(LedColor),
}

// e.g. { "data": { "ledR": 0, "ledG": 0, "ledB": 0 }}
type LedState = State<LedColor>;
#[derive(Default, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LedColor {
    led_r: u8,
    led_g: u8,
    led_b: u8,
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

    // a state object to hold the led color
    let led_state = Arc::new(RwLock::new(LedState {
        data: LedColor {
            led_r: 0,
            led_g: 0,
            led_b: 0,
        },
        ..LedState::default()
    }));

    // create a new Device
    //
    // the device will connect to the Losant broker and subscribe to commands
    //
    // you can set the device ID with, in order of priority: id(),
    // losant_device_id in cfg.toml, or the client_id field of config()
    //
    // defaults to using TLS, but you can disable it with secure(false)
    let mut device = Device::builder()
        // sets the handler for all MQTT events except Losant commands, which
        // are intercepted by command_handler().
        .handler({
            |event: &EventResult| match event {
                Ok(Event::Received(msg)) => {
                    if *msg.details() == Details::Complete {
                        println!("MQTT message: {msg:?}");
                    }
                }
                Ok(event) => println!("MQTT event: {event:?}"),
                Err(e) => eprintln!("MQTT error: {e}"),
            }
        })
        .command_handler({
            let led = Arc::clone(&led);
            let led_state = Arc::clone(&led_state);

            move |command: &Command| match *command {
                Command::SetLed(LedColor {
                    led_r: r,
                    led_g: g,
                    led_b: b,
                }) => {
                    if led.lock().unwrap().set(RGB8::new(r, g, b)).is_ok() {
                        led_state.write().unwrap().data = LedColor {
                            led_r: r,
                            led_g: g,
                            led_b: b,
                        };
                    }
                }
            }
        })
        .build()?;

    // main loop
    loop {
        if let Ok(measurement) = shtc3.measure(shtcx::PowerMode::NormalMode, &mut Ets) {
            let temperature = measurement.temperature.as_degrees_celsius();
            let humidity = measurement.humidity.as_percent();
            device.send_state_json(
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
