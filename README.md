<h1 align="center">HuskyNext</h1>

<p align="center">
  <img src="assets/logo.png" alt="HuskyNext logo" width="120">
</p>

<p align="center">
  Control the lighting of the <b>Husky Sled</b> keyboard on Linux with a native Rust and iced interface, without the official Windows-only software.
</p>

<p align="center">
  <a href="https://github.com/Leandro-74/HuskyNext-Rust">English</a> | <a href="README.pt-BR.md">Português</a>
</p>

<p align="center">
  <a href="https://github.com/Leandro-74/HuskyNext-Rust/actions/workflows/ci.yml"><img src="https://github.com/Leandro-74/HuskyNext-Rust/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
</p>

> ⚠️ Unofficial project. The protocol was obtained through reverse engineering of the USB HID traffic. It has no affiliation with Husky. The protocol may vary between models and firmware revisions. Use it at your own risk.

## About

HuskyNext is an independent, self-contained Rust implementation of the lighting control for the Husky Sled keyboard. It talks directly to the vendor-specific HID interface and runs as a native desktop application on Wayland or X11.

The interface is built with iced. Settings live in a small JSON file, and every change is sent to the keyboard as a complete 64-byte output report.

## Features

- Control the color, brightness, speed and all 19 lighting effects.
- Pick colors with a saturation/value square, a hue bar, HEX input, RGB channels or presets.
- Apply changes automatically at up to about 8 sends per second.
- Send the color from the picker only when you release the mouse.
- Save the configuration after a short pause in the adjustments.
- Detect the lighting interface by vendor-specific Usage Page, with a sysfs report descriptor fallback.
- Cache discovered devices and rediscover only when a path becomes stale or when you scan again.
- Recover from a disconnected or reconnected keyboard without restarting the application.
- Identify the device with a real static purple light (#8835E9) at 100%.
- Forget a device without losing the lighting state; the next device overwrites the old fields.
- Read old flat JSON configuration files and sanitize invalid values before sending.
- Write the configuration atomically, with owner-only permissions.
- Diagnose the device from the command line with `--list` and `--test`.
- Grant access with a udev rule that uses `TAG+="uaccess"` instead of opening the device to every user.

## Requirements

- Linux with Wayland or X11.
- Rust 1.88 or newer.
- Husky Sled keyboard (Sonix controller `0C45:8501` in this project).
- `pkg-config` or `pkgconf`.
- libudev development headers: `libudev-dev` on Debian and Ubuntu, `systemd-libs` on Arch Linux.

`hidapi` is compiled with the `hidraw` backend. The system `hidapi` library is not required, but the libudev headers are required at build time.

## Installation

```bash
git clone https://github.com/Leandro-74/HuskyNext-Rust.git
cd HuskyNext-Rust
cargo build --release
sudo install -Dm755 target/release/huskynext /usr/local/bin/huskynext
```

To run without installing, use `./target/release/huskynext`.

### udev rule

Without a udev rule, access to `/dev/hidraw*` may be denied. Install the rule from the repository:

```bash
sudo install -Dm644 packaging/99-huskynext.rules /etc/udev/rules.d/99-huskynext.rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Reconnect the keyboard after loading the rule.

The rule matches the Sonix IDs `0c45:8501`. Adjust the IDs if your keyboard uses different values. `TAG+="uaccess"` grants access only to the user of the active local session. Do not use `MODE="0666"` on shared machines, because it lets every local user write to the device.

### Desktop entry

```bash
install -Dm644 packaging/huskynext.desktop ~/.local/share/applications/huskynext.desktop
update-desktop-database ~/.local/share/applications
```

For a system-wide entry, install the same file in `/usr/share/applications` with `sudo`. The entry uses `Exec=huskynext`, so the binary must be in `PATH`.

### Icon

`assets/logo.png` is a 512×512 image. Install it in the matching icon directory:

```bash
install -Dm644 assets/logo.png ~/.local/share/icons/hicolor/512x512/apps/huskynext.png
gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor
```

For a system-wide icon, use `/usr/share/icons/hicolor/512x512/apps/huskynext.png` with `sudo` and update the cache in that directory. If you need another size, resize the image before copying it.

## Usage

```bash
huskynext
```

The application opens with three pages: **Lighting**, **Connection** and **About**. The sidebar shows the current connection state.

### First run

1. Connect the keyboard.
2. Start the application. If no keyboard is configured, the **Connection** page opens first.
3. Click **Detect again** if the list is empty.
4. Click **Test** on a device. The keyboard should turn purple (`#8835E9`) at 100%.
5. Click **Use this device** to save the vendor ID, product ID, interface and Usage Page.
6. Open the **Lighting** page and adjust the color, brightness, speed or effect.

The configuration is stored in `~/.huskynext/config.json`. To reset it, close the application and remove that file. **Forget device** removes only the device link and keeps the lighting settings.

### Options

- `huskynext`: opens the graphical interface.
- `huskynext --list`: lists the detected HID devices with their IDs, interface, Usage Page and path.
- `huskynext --test [HEX]`: sends a static color at 100% to the saved device. The default color is `8835E9`.
- `huskynext --version`: shows the version.
- `huskynext --help`: shows the help text.

`--test` uses the device saved in the configuration. If the saved device is not found, the command fails and does not send the color to another device. Without a saved device, it prefers the Sonix or Sled controller, then the first vendor-specific device. An invalid color returns exit code 2. A warning about an invalid configuration file is printed to standard error.

The options follow this priority: `--help`, `--version`, `--list`, `--test`.

## Project structure

```text
HuskyNext-Rust/
├── src/
│   ├── app.rs       # iced interface, application state and automatic apply
│   ├── color.rs     # HSV and RGB conversion
│   ├── config.rs    # reading and writing ~/.huskynext/config.json
│   ├── device.rs    # HID enumeration, discovery, selection and writes
│   ├── lib.rs       # library root
│   ├── main.rs      # entry point, command line and diagnostics
│   └── protocol.rs  # effects, brightness, speed, color and report layout
├── assets/
│   └── logo.png
├── packaging/
│   ├── 99-huskynext.rules
│   └── huskynext.desktop
├── .github/workflows/ci.yml
├── Cargo.toml
├── Cargo.lock
├── CHANGELOG.md
├── LICENSE
├── PROTOCOL.md
├── README.md
└── README.pt-BR.md
```

## How it works

### Device discovery

The application enumerates the HID devices with `hidapi`. A device is treated as a lighting candidate when its Usage Page is in the vendor-specific range `0xFF00`–`0xFFFF`. On Linux, the application also reads the report descriptors from `/sys/class/hidraw/*/device/report_descriptor` and looks for a vendor-specific Usage Page in the descriptor.

Duplicated entries are grouped by path, interface and IDs. Vendor-specific devices come first in the list.

When connecting, the application filters the candidates by vendor ID and product ID. The saved interface and Usage Page are used only when they still match a candidate. This keeps an old configuration usable after a firmware update changes those values.

The device list is cached. A new scan runs only when a path becomes stale, when the user clicks **Detect again**, or during the manual connection flow. If a cached path is stale, the application rediscovers the devices once and retries the send. Permission errors do not trigger this cycle.

### Automatic apply

Changes mark the state as dirty. A timer runs every 120 ms, which gives about 8 sends per second. The color picker is the exception: its color is sent when the mouse is released, not on every movement.

After the last change, the configuration is saved after a pause of about 450 ms. A manual **Apply** sends the state and saves the file immediately.

If an automatic send fails, the automatic apply pauses and the interface shows the error. The final state is sent when the user clicks **Try again**. The configuration is saved only after a successful send.

### Configuration

The file `~/.huskynext/config.json` is a flat JSON object. The application accepts files with missing fields and old values. Invalid values are sanitized before any send: an unknown effect becomes **Static**, brightness is limited to 0–100% and speed is limited to levels 1–5.

Empty fields are omitted when saving. A configuration without a device and without lighting values is saved as `{}`. The file is written atomically and receives `0600` permissions on Linux.

**Forget device** clears the vendor ID, product ID, interface and Usage Page. The lighting state is preserved. A new device overwrites all four link fields, even when its interface or Usage Page is absent. After **Forget device**, the file contains only the lighting state, and round-trip compatibility with older tools is not guaranteed.

### Protocol

The complete description is in [PROTOCOL.md](PROTOCOL.md). In short:

| Item | Value |
| --- | --- |
| Type | HID output report (`SET_REPORT`) |
| Report ID | `0x04` |
| Size | 64 bytes (ID plus 63 payload bytes) |
| Effect | offset `0x09`, `0x00`–`0x12` |
| Brightness | offset `0x0A`, `0x00`–`0x04` |
| Speed | offset `0x0B`, `0x04`–`0x00` (1=slow, 5=fast) |
| Color | offsets `0x0E`, `0x0F` and `0x10` (R, G, B) |

Every report carries the complete state. The keyboard does not answer and does not provide state readings in this protocol.

## Quality

The continuous integration workflow runs on every push to `main` and on pull requests. It uses Rust 1.88.0 and stable, installs `libudev-dev` and runs:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```

Run the same checks locally before sending changes. The test suite has 62 tests: 55 in the library and 7 in the binary. The tests cover report assembly, color parsing, configuration sanitization and atomic saving, report descriptor parsing, device selection with old configurations, stale path detection and report validation.

## Troubleshooting

- **Permission denied when opening the device**: install the udev rule from `packaging/99-huskynext.rules`, reload the rules and reconnect the keyboard. The interface shows the expected rule with the IDs of the detected device for permission errors.
- **Configured keyboard not found**: run `huskynext --list`, check the USB connection and click **Detect again**. `--test` fails in this case; it does not send to a different device.
- **Warning about an invalid configuration**: the file was ignored and the defaults were used. Fix the JSON or remove `~/.huskynext/config.json`.
- **The keyboard stays purple after a test**: that is the identification color. Click **Use this device** and then choose the desired lighting.
- **Build fails with missing libudev**: install the development headers. Use `libudev-dev` on Debian or Ubuntu and `systemd-libs` on Arch Linux.
- **Icon does not appear**: confirm the file name `huskynext.png` and the `512x512` directory, then update the icon cache.
- **Window size on Wayland**: the compositor controls the initial size and position. Move or resize the window with the compositor tools.

## Contributing

Found a bug or have an idea? Open an [issue](https://github.com/Leandro-74/HuskyNext-Rust/issues) or send a pull request.

Please keep the public text in English and run the quality checks before submitting a change.

## License

This project is licensed under the MIT license. See the [LICENSE](LICENSE) file for details.
