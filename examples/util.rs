pub mod led {
    use std::time::Duration;

    use anyhow::Result;
    use esp_idf_hal::gpio::OutputPin;
    use esp_idf_hal::peripheral::Peripheral;
    use esp_idf_hal::rmt::config::TransmitConfig;
    use esp_idf_hal::rmt::{FixedLengthSignal, PinState, Pulse, TxRmtDriver, CHANNEL0};
    pub use rgb::RGB8;

    pub struct Ws2812Rmt<'a> {
        tx_rmt_driver: TxRmtDriver<'a>,
    }

    impl<'a> Ws2812Rmt<'a> {
        // ESP32-C3-DevKit-RUST-1 gpio2,  ESP32-C3-DevKitC-02 gpio8
        pub fn new(
            pin: impl Peripheral<P = impl OutputPin> + 'a,
            channel: CHANNEL0,
        ) -> Result<Self> {
            let config = TransmitConfig::new().clock_divider(2);
            let tx_rmt_driver = TxRmtDriver::new(channel, pin, &config)?;
            Ok(Self { tx_rmt_driver })
        }

        pub fn set(&mut self, rgb: RGB8) -> Result<()> {
            let color: u32 = (u32::from(rgb.g) << 16) | (u32::from(rgb.r) << 8) | u32::from(rgb.b);
            let ticks_hz = self.tx_rmt_driver.counter_clock()?;
            let t0_hi = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(350))?;
            let t0_lo = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(800))?;
            let t1_hi = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(700))?;
            let t1_lo = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(600))?;
            let mut signal = FixedLengthSignal::<24>::new();

            for i in (0..24).rev() {
                let p = 2_u32.pow(i);
                let bit = p & color != 0;
                let (pulse_hi, pulse_lo) = if bit { (t1_hi, t1_lo) } else { (t0_hi, t0_lo) };
                signal.set(23 - i as usize, &(pulse_hi, pulse_lo))?;
            }

            Ok(self.tx_rmt_driver.start_blocking(&signal)?)
        }
    }

    const fn ns(nanos: u64) -> Duration {
        Duration::from_nanos(nanos)
    }
}

pub mod wifi {
    use std::net::Ipv4Addr;
    use std::time::Duration;

    use anyhow::{anyhow, bail, Result};
    use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};
    use esp_idf_hal::modem::Modem;
    use esp_idf_hal::peripheral::Peripheral;
    use esp_idf_svc::eventloop::EspSystemEventLoop;
    use esp_idf_svc::netif::{EspNetif, EspNetifWait};
    use esp_idf_svc::wifi::{EspWifi, WifiWait};

    #[toml_cfg::toml_config]
    struct Config {
        #[default("")]
        wifi_ssid: &'static str,
        #[default("")]
        wifi_psk: &'static str,
    }

    pub fn connect(
        modem: impl Peripheral<P = Modem> + 'static,
        sysloop: &EspSystemEventLoop,
    ) -> Result<Box<EspWifi<'static>>> {
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

        Ok(wifi)
    }
}
