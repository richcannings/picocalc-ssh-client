# PicoCalc SSH Client

A standalone SSH client and VT100/ANSI terminal emulator for the [Raspberry Pi Pico 2 W](https://www.raspberrypi.com/products/raspberry-pi-pico-2/) running on the [ClockworkPi PicoCalc](https://www.clockworkpi.com/picocalc).

This project transforms your PicoCalc into a pocket-sized, WiFi-enabled terminal capable of connecting to remote servers via SSH. It is a fork of the [picocalc-wezterm](https://github.com/wez/picocalc-wezterm) project, with improved terminal character support and scrolling. Thus, making the terminal experience more usable.

<p align="center">
  <img src="img/picocalc-ssh-client-gemini.png" width="45%" />
  <img src="img/picocalc-ssh-client-mc.png" width="45%" />
</p>

## Features

*   **Standalone SSH Client**: Connect to any SSH server directly from the device.
*   **Robust Terminal Emulation**: Built on the `vte` crate for accurate ANSI/VT100 parsing.
*   **Extended Character Support**: Custom rendering for box-drawing characters (lines, corners, shades) for TUI applications like `vim`, `gemini-cli`, `mc`, `htop`, `ollama`, and `tmux`.
*   **Scrolling**: Scroll through the command history up to 500 lines.
*   **Local Shell**: Built-in commands for device management (WiFi config, battery status, backlight control).
*   **Hardware Accelerated**: Uses the RP2350's capabilities and the ILI9488 display for fast rendering.

## Hardware Requirements

*   **ClockworkPi PicoCalc**
*   **Raspberry Pi Pico 2 W** (RP2350 with WiFi)
    *   *Note: This firmware is specifically designed for the RP2350 architecture.*

## Installing the Released Firmware

1. Download the latest firmware from the [releases](https://github.com/richcannings/picocalc-ssh-client/releases) page, like [picocalc-ssh-client.v0.2.uf2](https://github.com/richcannings/picocalc-ssh-client/releases/download/v0.2/picocalc-ssh-client.v0.2.uf2).
2. Flash:
    * Hold the BOOTSEL button on your Pico 2 W while plugging it in.
    * Copy the downloaded firmware, e.g. `picocalc-ssh-client.v0.2.uf2`, to the mounted RP2350 drive.
    * Reboot the Pico 2 W.

## Getting Started with Development

### Prerequisites

You will need a standard Rust toolchain and a few helper tools:

1.  **Install Rust**: [rustup.rs](https://rustup.rs/)
2.  **Install the Nightly Toolchain**:
    ```bash
    rustup toolchain install nightly
    ```
3.  **Add the Compilation Target**:
    ```bash
    rustup target add thumbv8m.main-none-eabihf
    ```
4.  **Install Helper Tools**:
    ```bash
    cargo install flip-link
    # Install picotool (follow instructions at https://github.com/raspberrypi/picotool)
    ```

### Building & Flashing

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/richcannings/picocalc-ssh-client.git
    cd picocalc-ssh-client
    ```

2.  **Build the Firmware**:
    ```bash
    # For Pimoroni Pico Plus 2 W (standard PicoCalc upgrade)
    cargo +nightly build --release --features pimoroni2w
    ```

3.  **Generate UF2 File**:
    ```bash
    # Convert the ELF to UF2
    cp target/thumbv8m.main-none-eabihf/release/picocalc-wezterm target/thumbv8m.main-none-eabihf/release/picocalc-ssh-client.elf
    picotool uf2 convert target/thumbv8m.main-none-eabihf/release/picocalc-ssh-client.elf picocalc-ssh-client.uf2
    ```

4.  **Flash**:
    *   Hold the BOOTSEL button on your Pico 2 W while plugging it in.
    *   Copy `picocalc-ssh-client.uf2` to the mounted `RP2350` drive.

## Usage

### Initial Setup (WiFi)

On first boot, you need to configure your WiFi credentials. The device includes a local shell for configuration.

```bash
# Format the config storage (only needed once)
$ config format

# Set WiFi credentials
$ config set wifi_ssid MyNetwork
$ config set wifi_pw MyPassword

# Reboot to apply
$ reboot
```

> [!CAUTION]
> Credentials are stored in clear-text in the device's flash memory.

### Connecting via SSH

Once connected to WiFi (you'll see an IP address), you can connect to a remote host:

```bash
$ ssh mymachine
# or
$ ssh 192.168.1.10
```

You can also save credentials to avoid typing them every time:

```bash
$ config set ssh_user myuser
$ config set ssh_pw mypassword
```

### Scrolling

You can scroll through the command history (up to 500 lines) using the following key combinations:

*   `Ctrl + UpArrow`: Scroll up
*   `Ctrl + DownArrow`: Scroll down

Typing any character or receiving new output from the server will automatically reset the view to the bottom.

You can configure the number of lines in the scrollback buffer (default 200, max 500):

```bash
$ config set scroll 500
$ config get scroll
$ config rm scroll  # Resets to default (200)
```

### Local Commands

*   `cls`: Clear the screen.
*   `bat`: Show battery status.
*   `bl lcd <percent>`: Set LCD backlight brightness (e.g., `bl lcd 50`).
*   `bl kbd <percent>`: Set keyboard backlight brightness (requires updated keyboard firmware).
*   `free`: Show memory usage.
*   `bootsel`: Reboot into bootloader mode.

## Credits

*   Forked from [wezterm/picocalc-wezterm](https://github.com/wezterm/picocalc-wezterm).
*   Original SSH implementation using [sunset](https://github.com/wez/sunset).
*   Terminal emulation powered by [vte](https://github.com/alacritty/vte).
