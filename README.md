# losant-mqtt-esp-idf

ESP-IDF MQTT Client for connecting devices to the Losant IoT Platform

```toml
[dependencies]
losant-mqtt-esp-idf = "1.0"
```

## Usage

- add your application key and secret to a `cfg.toml` file in your crate root; see
  [`cfg.example.toml`](https://github.com/tedbyron/losant-mqtt-esp-idf/blob/main/cfg.example.toml)

- see the [`examples`](https://github.com/tedbyron/losant-mqtt-esp-idf/tree/main/examples)

## Examples

- add Losant and wifi info to a `cfg.toml` file in the crate root (make sure to .gitignore!); see
  `cfg.example.toml`

- if using WSL, use `usbipd` to expose a USB device to WSL
  ([Microsoft docs](https://learn.microsoft.com/en-us/windows/wsl/connect-usb))

  - install `usbipd` in a PowerShell/CMD terminal

    ```ps1
    # all of the following require admin mode
    winget install --interactive --exact dorssel.usbipd-win
    usbipd wsl list # may need a new terminal window to refresh env
    usbipd wsl attach --auto-attach --busid <BUSID>
    ```

  - check that the device is accessible within WSL

    ```sh
    lsusb # e.g. Bus 001 Device 002: ID 303a:1001 Espressif USB JTAG/serial debug unit
    ```

- run an example; replace the `--target` argument with your board's respective compiler target

  - using `espflash` v2

    ```sh
    cargo run --example=esp32-c3-devkit-rust-1 --release --target=riscv32imc-esp-espidf
    ```

  - using `cargo-espflash` v2

    ```sh
    cargo espflash flash --monitor --example=esp32-c3-devkit-rust-1 --release --target=riscv32imc-esp-espidf
    ```

  - using `espflash` v2 (manual)

    ```sh
    cargo build --example=esp32-c3-devkit-rust-1 --release --target=riscv32imc-esp-espidf
    espflash flash --monitor target/riscv32imc-esp-espidf/release/examples/wifi
    ```
