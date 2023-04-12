#[toml_cfg::toml_config]
pub struct Config {
    #[default(MqttConfig::default())]
    pub mqtt: MqttConfig,
    #[default(WifiConfig::default())]
    pub wifi: WifiConfig,
}

#[derive(Default)]
pub struct MqttConfig {
    pub username: &'static str,
    pub password: &'static str,
}

#[derive(Default)]
pub struct WifiConfig {
    pub ssid: &'static str,
    pub psk: &'static str,
}
