# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-23

First public release.

### Added

- Native desktop interface built with iced.
- Color controls with a saturation/value square, hue bar, HEX input, RGB channels and presets.
- Five brightness levels, five speed levels and all 19 lighting effects.
- Automatic application of changes at up to about 8 sends per second, with the color picker sending on mouse release.
- Delayed configuration saving, manual apply and restore defaults.
- HID device discovery with vendor-specific Usage Page detection and a sysfs report descriptor fallback.
- Device list with test, use and forget actions, plus manual entry of vendor ID, product ID, interface and Usage Page.
- Command line diagnostics with `--list`, `--test`, `--version` and `--help`.
- udev rule, desktop entry and icon installation instructions.
- Configuration file at `~/.huskynext/config.json`, written atomically with owner-only permissions.
- Continuous integration workflow for formatting, linting, tests and release build.
- English and Brazilian Portuguese documentation, protocol reference and changelog.

### Changed

- Device discovery results are cached and reused between sends.
- A stale device path triggers one rediscovery and a new send attempt.
- Empty configuration fields are omitted when saving.
- After forgetting a device, the lighting state is preserved and the next device overwrites the previous link fields.
- The recommended udev rule uses `TAG+="uaccess"` instead of making the device writable by every user.

### Fixed

- `--test` fails when the saved device is missing instead of sending the color to a different device.
- Device reports are validated before the device is opened, including the report length and the Report ID.
- Invalid configuration values are sanitized before any send.
- Permission errors are reported with a udev hint and do not cause repeated rediscovery.

### Notes

- Linux only, with Wayland or X11.
- Distributed as source; build with Rust 1.88 or newer.
- The protocol was obtained through reverse engineering and may vary between models and firmware revisions.
