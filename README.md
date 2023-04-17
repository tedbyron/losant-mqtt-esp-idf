# esp-losant-mqtt

MQTT Client for connecting ESP32 (ESP-IDF) devices to the Losant IoT Platform

## Usage

- see the [Rust on ESP book](https://esp-rs.github.io/book/installation/index.html) for toolchain
  and tooling setup

- see the [ESP-IDF template](https://github.com/esp-rs/esp-idf-template) to get started making a
  binary crate that uses ESP-IDF

- see the [`examples`](examples)

- add Losant info to a `cfg.toml` file in your crate root (make sure to .gitignore!); see
  `cfg.example.toml`

```rs
use esp_losant_mqtt::Device;

# TODO
```

## Message limits

- refer to the [Losant docs](https://docs.losant.com/mqtt/overview/#message-limits); this library
  does not currently rate limit, but does check message size.

## Running examples

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

- run the `wifi` example; replace the `--target` argument with your board's respective compiler
  target

  - using `espflash` v2

    ```sh
    cargo run --example=wifi --features="esp-idf-sys/binstart" --release --target=riscv32imc-esp-espidf
    ```

  - using `cargo-espflash` v2

    ```sh
    cargo espflash flash --monitor --example=wifi --features="esp-idf-sys/binstart" --release --target=riscv32imc-esp-espidf
    ```

  - using `espflash` v2 (manual)

    ```sh
    cargo build --example=wifi --features="esp-idf-sys/binstart" --release --target=riscv32imc-esp-espidf
    espflash flash --monitor target/riscv32imc-esp-espidf/release/examples/wifi
    ```
