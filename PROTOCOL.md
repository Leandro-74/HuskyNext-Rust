# HuskyNext Protocol

Technical description of the USB HID protocol used by HuskyNext to control the lighting of the **Husky Sled** keyboard. The information was obtained through reverse engineering of the USB traffic and matches the Rust implementation in this repository.

> ⚠️ Unofficial protocol, with no affiliation with Husky. It may vary between models and firmware revisions.

## 1. Overview

| Item | Value |
| --- | --- |
| Bus | USB HID |
| Direction | Host to device (write only) |
| Report type | Output (`SET_REPORT`) |
| Report ID | `0x04` |
| Report size | 64 bytes (including the Report ID) |
| Byte order | Little-endian |
| State | Every report carries the complete state (effect + brightness + speed + RGB) |

The application talks to the device through `hidapi` with `write` only. The keyboard does not send input reports or responses for this protocol. The implementation is in `src/device.rs` and `src/protocol.rs`.

## 2. Device identification

The keyboard exposes several HID interfaces and collections. The lighting control interface is identified by a **vendor-specific Usage Page** in the range `0xFF00`–`0xFFFF`.

Discovery works like this (`src/device.rs`):

1. Enumerate the HID devices with `hidapi`.
2. Treat a device as a candidate when its `usage_page` is in `0xFF00`–`0xFFFF`.
3. On Linux, also read the report descriptors from `/sys/class/hidraw/*/device/report_descriptor` and look for a vendor-specific Usage Page in the descriptor.
4. Group duplicates by path, interface number, vendor ID and product ID. When two entries share the same key, keep the vendor-specific one and, between vendor-specific entries, the one with the higher Usage Page.
5. Sort the list with vendor-specific devices first, then by display name, IDs, interface and path.

The descriptor is read item by item. Long items (`0xFE`) are skipped, and the `Usage Page` tag (`0x04`) is read as a little-endian value.

### Selection

`find_candidate` filters the candidates by vendor ID and product ID. It then applies the saved interface number and the saved Usage Page only when they still match at least one candidate. A stale interface or Usage Page therefore does not block a device that still matches the IDs.

Within the filtered pool, a vendor-specific candidate is preferred.

When no device is saved, `default_candidate` prefers a vendor-specific device whose manufacturer contains `sonix` or whose product contains `sled`. If none matches, it takes the first vendor-specific device.

The `--test` command uses `select_for_test`. When a device is saved, the saved device must be found. The command fails with `SavedDeviceNotFound` instead of sending to another device. The default selection applies only when no device is saved.

### Persisted data

After confirmation in the interface, the connection is saved in `~/.huskynext/config.json`:

```json
{
  "vendor_id": 3141,
  "product_id": 34049,
  "interface_number": 1,
  "usage_page": 65308,
  "effect": 6,
  "bright": 4,
  "speed": 4,
  "r": 255,
  "g": 255,
  "b": 255
}
```

The decimal values above correspond to `0C45`, `8501` and `0xFF1C`. The interface number and the Usage Page are optional. The lighting state uses `effect`, `bright`, `speed`, `r`, `g` and `b`.

The configuration module (`src/config.rs`) treats the file as a flat JSON object:

- Missing fields and `null` values fall back to safe defaults.
- Empty fields are omitted when saving. A configuration with no device and no lighting values is written as `{}`.
- The file is written atomically and receives `0600` permissions on Linux.
- Invalid values are sanitized before any send. An unknown effect becomes **Static**, brightness is limited to `0x00`–`0x04` and speed is limited to levels 1–5.

**Forget device** clears `vendor_id`, `product_id`, `interface_number` and `usage_page`, and preserves the lighting state. A new device overwrites all four link fields, even when its interface or Usage Page is absent. After a forget, the file contains only the lighting state.

## 3. Report format

The report has 64 bytes. The first byte is the Report ID (`0x04`) and the remaining 63 bytes form the payload. The base report is `BASE_REPORT` in `src/protocol.rs`:

```text
offset  00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F
        04 A1 01 06 38 00 00 55 00 06 04 01 00 00 FF FF
offset  10 11 12 13 14 15 16 17 18 19 1A 1B 1C 1D 1E 1F
        FF 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
offset  20 ... 3F
        00 ... 00
```

### Variable fields

| Offset | Field | Values | Default |
| --- | --- | --- | --- |
| `0x00` | Report ID | fixed `0x04` | `0x04` |
| `0x09` | Effect | `0x00`–`0x12` (see table) | `0x06` (Static) |
| `0x0A` | Brightness | `0x00`–`0x04` (0–100%) | `0x04` (100%) |
| `0x0B` | Speed | `0x00`–`0x04` (5=fast … 1=slow) | `0x01` (level 4) |
| `0x0E` | R channel | `0x00`–`0xFF` | `0xFF` |
| `0x0F` | G channel | `0x00`–`0xFF` | `0xFF` |
| `0x10` | B channel | `0x00`–`0xFF` | `0xFF` |

Constants in `src/protocol.rs`:

```rust
pub const REPORT_ID: u8 = 0x04;
pub const REPORT_LEN: usize = 64;
pub const MODE_OFFSET: usize = 9;     // 0x09
pub const BRIGHT_OFFSET: usize = 10;  // 0x0A
pub const SPEED_OFFSET: usize = 11;   // 0x0B
pub const COLOR_OFFSET_R: usize = 14; // 0x0E
pub const COLOR_OFFSET_G: usize = 15; // 0x0F
pub const COLOR_OFFSET_B: usize = 16; // 0x10
```

The other bytes (`0x01`–`0x08`, `0x0C`–`0x0D`, `0x11`–`0x3F`) stay constant in the implemented protocol:

```text
A1 01 06 38 00 00 55 00 <effect> <brightness> <speed> 00 00 <R> <G> <B> 00...
```

Before opening the device, `write_to_candidate` checks the report length and the Report ID. A short report returns `Incomplete`, and a wrong Report ID returns `InvalidReport`. Both checks run before `hidapi` opens the device.

## 4. Lighting effects

Values accepted at offset `0x09`. The labels match the interface and `Effect::label()` in `src/protocol.rs`:

| Code | Effect |
| --- | --- |
| `0x00` | Off |
| `0x01` | Wave |
| `0x02` | Blinds |
| `0x03` | Diffusion |
| `0x04` | Neon |
| `0x05` | Breathing |
| `0x06` | Static |
| `0x07` | Reactive |
| `0x08` | Raindrop |
| `0x09` | Line Reactive |
| `0x0A` | Starry |
| `0x0B` | Blooming |
| `0x0C` | Cascade |
| `0x0D` | Rainbow |
| `0x0E` | Cartoon |
| `0x0F` | Rain Scene |
| `0x10` | Scan |
| `0x11` | Single Off |
| `0x12` | Flow |

`Effect::ALL` contains the 19 effects in this order. `Effect::from_code` maps an unknown code to `None`, and the configuration sanitization then falls back to **Static**.

## 5. Brightness

Brightness uses five levels at offset `0x0A`:

| Level | Byte (`0x0A`) | Percentage |
| --- | --- | --- |
| 0 | `0x00` | 0% |
| 1 | `0x01` | 25% |
| 2 | `0x02` | 50% |
| 3 | `0x03` | 75% |
| 4 | `0x04` | 100% |

`MAX_BRIGHTNESS` is `4`. `brightness_percent` returns `level.min(4) * 25`.

## 6. Speed

Speed uses five levels at offset `0x0B`, but the byte is inverted relative to the interface level:

| Level | Byte (`0x0B`) | Behavior |
| --- | --- | --- |
| 1 | `0x04` | Slowest |
| 2 | `0x03` | Slow |
| 3 | `0x02` | Medium |
| 4 | `0x01` | Fast (default) |
| 5 | `0x00` | Fastest |

`speed_byte` clamps the level to the range `1`–`5` and returns `5 - level`. The base report value `0x01` therefore corresponds to level 4, which is `DEFAULT_SPEED`.

Speed is visible in animated effects. In **Static** and **Off**, it is stored for the next animated effect.

## 7. Color

The color is stored as three independent bytes on offsets `0x0E`, `0x0F` and `0x10`. The interface accepts HEX in the `RRGGBB` or `#RRGGBB` format, with optional surrounding spaces and either letter case:

```text
FF5733  ->  R = 0xFF, G = 0x57, B = 0x33
```

`Rgb::from_hex` rejects values that are not exactly six hexadecimal digits. `Rgb::to_hex` formats the color as uppercase `#RRGGBB`.

Every change resends the complete report and preserves the current effect, brightness and speed. Changes to effect, brightness and speed preserve the color. The state lives in `KeyboardState`, whose default is **Static** (`0x06`), 100% (`0x04`), speed level 4 (`0x01`) and white (`0xFF, 0xFF, 0xFF`).

`KeyboardState::sanitized` applies the same limits used when loading the configuration: invalid effect to **Static**, brightness to `0x00`–`0x04` and speed to levels 1–5.

## 8. Send sequence

```text
host                                   keyboard
  |  SET_REPORT (Report ID 0x04, 64 B)    |
  |-------------------------------------->|
  |  (no response)                        |
```

The application flow (`src/app.rs` and `src/device.rs`):

1. Load the configuration and sanitize the lighting state.
2. Discover the devices once and keep the candidate list in memory.
3. Resolve the candidate from the saved vendor ID and product ID.
4. Build the 64 bytes from the current state with `KeyboardState::report()`.
5. Validate the report length and the Report ID.
6. Open the HID path with `hidapi` and write the report.
7. Close the device.
8. Update the configuration and save it after the changes stop.

A normal send reuses the cached candidates and does not enumerate the HID bus again. If the path is stale, the application discovers the devices once and retries the send. Permission errors are not treated as stale paths and do not enter this cycle.

The graphical interface applies changes automatically with a timer of 120 ms, which gives about 8 sends per second. The color picker sends only when the mouse is released. The configuration is saved after a pause of about 450 ms without changes, or immediately after a manual apply.

The `--test` command follows a shorter path: it discovers the devices, resolves the target, builds a static report and writes it directly with `write_to_candidate`. It never saves the test color.

## 9. Report examples

Default state: **Static**, 100%, speed level 4, white:

```text
04 A1 01 06 38 00 00 55 00 06 04 01 00 00 FF FF FF 00 ... 00
```

Color `FF5733` with **Static**, 100% and speed level 4:

```text
04 A1 01 06 38 00 00 55 00 06 04 01 00 00 FF 57 33 00 ... 00
```

**Rainbow** (`0x0D`) with 50% brightness and speed level 5 (fastest):

```text
04 A1 01 06 38 00 00 55 00 0D 02 00 00 00 FF FF FF 00 ... 00
```

**Rainbow** (`0x0D`) with 50% brightness and speed level 1 (slowest):

```text
04 A1 01 06 38 00 00 55 00 0D 02 04 00 00 FF FF FF 00 ... 00
```

**Off** (`0x00`), 0%, speed level 4, keeping the color:

```text
04 A1 01 06 38 00 00 55 00 00 00 01 00 00 FF FF FF 00 ... 00
```

Identification test: **Static**, 100%, speed level 4, purple `#8835E9`:

```text
04 A1 01 06 38 00 00 55 00 06 04 01 00 00 88 35 E9 00 ... 00
```

## 10. Limitations

- The protocol is write only. There is no handshake and no way to read the current state of the keyboard.
- The identification test sends static purple `#8835E9` at 100%, which matches the text shown in the interface.
- The speed byte (`0x0B`) is inverted relative to the displayed level: `0x04` is level 1 and `0x00` is level 5.
- The constants were confirmed on a Sonix controller with IDs `0C45:8501`. Other models or firmware revisions may use different offsets, report sizes or IDs.
- Access on Linux requires udev rules for the `usb` and `hidraw` subsystems. Without permission, opening the device fails with `EACCES` or `EPERM`. The interface shows a udev hint for permission errors only.
- There is no support for profiles, macros or custom effects.
