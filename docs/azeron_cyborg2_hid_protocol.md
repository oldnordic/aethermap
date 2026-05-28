# Azeron Cyborg 2 HID Protocol Reference

## Document Information

- **Device**: Azeron Cyborg II
- **Vendor ID**: `0x16D0` (MCS / Azeron LTD)
- **Product ID**: `0x12F7`
- **USB Interfaces**: 5 total
- **Config Interface**: Interface 4 (`hidraw`)
- **Firmware Update**: STM32 DFU bootloader (`0x0483:0xDF11`)
- **Source**: Reverse-engineered from official Azeron Software v1.5.6, azeron-cli project, and noreza project

---

## 1. USB HID Identification

| Property | Value | Notes |
|----------|-------|-------|
| Vendor ID | `0x16D0` | MCS / Azeron LTD |
| Product ID | `0x12F7` | Cyborg II specific |
| USB Class | HID | Human Interface Device |
| Interfaces | 5 | Interface 0-3: input events, Interface 4: config |
| Config Interface | `hidraw` | `/dev/hidraw*` device node |
| Udev Rule | `SUBSYSTEM=="hidraw", ATTRS{idVendor}=="16d0", MODE="0666"` | For non-root access |

### lsusb Output

```bash
$ lsusb -d 16d0:12f7 -v
Bus XXX Device XXX: ID 16d0:12f7 MCS Azeron Cyborg II
  bNumInterfaces          5
  Interface 0: HID (keyboard)
  Interface 1: HID (mouse)
  Interface 2: HID (joystick)
  Interface 3: HID (consumer control)
  Interface 4: HID (vendor-specific config)
```

---

## 2. Communication Protocols

The Azeron devices use **two different protocols** depending on firmware generation.

### 2.1 Legacy Firmware (Text-Based Protocol)

Used by: Classic, early Cyborg models

**Message Framing:**
```
^<length>~<message>\n
Example: "^7~GET_FW_VERSION\n"
```

**Common Commands:**
| Command | Description |
|---------|-------------|
| `GET_FW_VERSION` | Request firmware version |
| `GET_LEDS` | Request LED state |
| `LEDBRT` | Request LED brightness |
| `GET_ANALOG_TYPE` | Request analog stick type |
| `GET_PROFILES` | Request all profiles |
| `ST\|1` / `ST\|0` | Set switch/button state |
| `HW_MODE` / `SW_MODE` | Set hardware/software mode |
| `DFU` | Enter firmware update mode |
| `RESET` | Reset device |
| `GET_THROTTLE_TYPE` | Request button throttle type |
| `BTNTHT` | Request button throttle value |
| `HWANLGOFST` | Request analog hardware offsets |
| `HWLWRDZ` | Request lower deadzone |
| `HWUPRDZ` | Request upper deadzone |
| `GET_RIGHT_ANALOG` | Request right analog settings |
| `GET_GSN` | Request global mouse sensitivity |
| `GET_GSA` | Request global mouse sensor angle |
| `GET_MOUSE_SENSOR` | Request mouse sensor state |
| `PI` | Request hardware profile index |
| `P_LIMIT` | Request profile limit |
| `PROFILE_ADD` | Add hardware profile |
| `PS_<n>` | Request profile n |
| `PURE_ANALOG_ON` / `PURE_ANALOG_OFF` | Toggle pure analog mode |
| `THROTTLE_TYPE_1` / `THROTTLE_TYPE_2` | Set throttle type |
| `LED\|<n>` | Set LED brightness |
| `LEDS_ON` / `LEDS_OFF` | Toggle LEDs |
| `HWD\|<x>\|<y>\|<dz>\|0` | Set analog calibration data |

### 2.2 Modern Firmware (Binary Protocol)

Used by: Cyborg V2 (fw >= 90), Cyro (fw >= 72), Keyzen (fw >= 100)

**Report Size:** 65 bytes (padded with zeros on Linux hidraw)

**Binary Frame Format:**
```
Byte 0: Report ID (always 0x00 for config reports)
Byte 1-2: Message type (big-endian uint16)
Byte 3: Echo/sequence byte
Byte 4: Total pages
Byte 5: Current page
Byte 6: Payload size
Byte 7+: Payload data
```

**Message Types (from official app):**
| Type Code | Name | Description |
|-----------|------|-------------|
| `0x01` | `PROFILE_PAYLOAD_V2` | Profile data (modern) |
| `0x02` | `PROFILE_PAYLOAD` | Profile data (legacy binary) |
| `0x03` | `PING_DEVICE` | Keepalive ping |
| `0x04` | `BUTTON_BIND` | Button binding config |
| `0x05` | `ANALOG_BIND` | Analog stick binding |
| `0x06` | `SW_ANALOG_SETTINGS` | Software analog settings |
| `0x07` | `OBM_COPY` | Onboard memory copy |
| `0x08` | `SWITCH_PROFILE` | Switch active profile |
| `0x09` | `PROFILE_INPUT_PIECE` | Profile piece confirmation |
| `0x0A` | `ADD_PROFILE` | Add new profile |
| `0x0B` | `DELETE_PROFILE` | Delete profile |
| `0x0C` | `FIRMWARE_VERSION` | Firmware version response |
| `0x0D` | `LED_EFFECT` | LED effect control |
| `0x0E` | `LED_STATE` | LED on/off state |
| `0x0F` | `SET_LED_BRIGHTNESS` | LED brightness |
| `0x10` | `MODE_SWITCH` | HW/SW mode switch |
| `0x11` | `SOFTWARE_BUTTON` | Software button press |
| `0x12` | `SW_BUTTON_WHEEL` | Button wheel event |
| `0x13` | `UPDATE_DEVICE_ID` | Update device ID |
| `0x14` | `RELEASE_SW_BUTTONS` | Release all SW buttons |
| `0x15` | `PROFILE_NAME` | Profile name |
| `0x16` | `PROFILE_LIMIT` | Profile limit |
| `0x17` | `PROFILE_PAYLOAD_V` | Profile payload variant |
| `0x18` | `PURE_ANALOG` | Pure analog toggle |
| `0x19` | `RIGHT_ANALOG` | Right analog settings |
| `0x1A` | `HARDWARE_PROFILE_INDEX` | HW profile index |
| `0x1B` | `KEYPAD_DETAILS` | Keypad details |
| `0x1C` | `KEYPAD_STATUS` | Keypad status |
| `0x1D` | `MEMORY_DETAILS` | Memory details |
| `0x1E` | `DFU` | Enter DFU mode |
| `0x1F` | `CHECK_PROFILE` | Check profile existence |
| `0x20` | `REQUEST_PROFILE` | Request profile data |
| `0x21` | `SET_ANALOG_THROTTLE` | Set analog throttle |
| `0x22` | `SET_THROTTLE_VALUE` | Set throttle value |
| `0x23` | `SET_GLOBAL_ANGLE` | Set global angle |
| `0x24` | `SET_GLOBAL_MOUSE_SENSOR_ANGLE` | Set mouse sensor angle |
| `0x25` | `SET_GLOBAL_MOUSE_SENSOR_SENSITIVITY` | Set mouse sensitivity |
| `0x26` | `SET_GLOBAL_MOUSE_SENSOR_STATE` | Set mouse sensor state |
| `0x27` | `GET_ALL_GLOBAL_MOUSE_SETTINGS` | Get all mouse settings |
| `0x28` | `SET_MOUSE_SENSOR_SENSITIVITY_SETTINGS` | Mouse sensitivity settings |
| `0x29` | `SET_MOUSE_SENSOR_STATE_SETTINGS` | Mouse sensor state settings |
| `0x2A` | `SET_MOUSE_SENSOR_ANGLE_SETTINGS` | Mouse angle settings |
| `0x2B` | `ANALOG_CALIBRATION_DATA` | Analog calibration data |
| `0x2C` | `RESET` | Reset device |

### 2.3 Keypad Type Enumeration (from official app)

```typescript
enum KeypadType {
  Classic = 1,
  Cyborg = 3,
  Cyro = 4,
  CyborgTansy = 5,
  ClassicTansy = 6,
  CyroLefty = 7,
  CyborgV2 = 8,
  Keyzen = 9
}
```

### 2.4 Firmware Versions by Device

| Device | Current FW | Min Binary | Threshold Payload | Threshold Details |
|--------|-----------|------------|-------------------|-------------------|
| Classic | 89 | 66 | 86 | 86 |
| Cyborg | 89 | 66 | 86 | 86 |
| Cyro | 102 | 72 | 94 | 94 |
| CyroLefty | 102 | 72 | 94 | 94 |
| CyborgTansy | 96 | 70 | 91 | 91 |
| ClassicTansy | 97 | 70 | 92 | 92 |
| **CyborgV2** | **100** | **90** | **90** | **91** |
| Keyzen | 100 | 100 | 100 | 100 |

---

## 3. Input Event Interface (evdev)

The Azeron Cyborg 2 exposes input events through the Linux evdev subsystem. This is the primary interface for remapper_rs.

### 3.1 Event Types

| evdev Type | Code | Description |
|------------|------|-------------|
| `EV_KEY` | `0x01` | Button presses/releases |
| `EV_REL` | `0x02` | Relative motion (mouse) |
| `EV_ABS` | `0x03` | Absolute position (joystick) |

### 3.2 Absolute Axis Events (Analog Stick)

The analog stick generates `EV_ABS` events with the following axis codes:

| Axis | evdev Code | Value Range | Description |
|------|-----------|-------------|-------------|
| `ABS_X` | `0x00` | 0-65535 | Left stick X (or primary stick X) |
| `ABS_Y` | `0x01` | 0-65535 | Left stick Y (or primary stick Y) |
| `ABS_RX` | `0x03` | 0-65535 | Right stick X (if present) |
| `ABS_RY` | `0x04` | 0-65535 | Right stick Y (if present) |
| `ABS_Z` | `0x02` | 0-255 | Trigger/pressure |
| `ABS_RZ` | `0x05` | 0-255 | Rudder/secondary trigger |
| `ABS_HAT0X` | `0x10` | -1/0/1 | D-pad X direction |
| `ABS_HAT0Y` | `0x11` | -1/0/1 | D-pad Y direction |

**Value interpretation:**
- Center position: ~32768 (midpoint of 0-65535)
- Minimum: 0
- Maximum: 65535
- Deadzone: Configurable in firmware (lower/upper deadzone)

### 3.3 Button Events

Buttons generate `EV_KEY` events with the following codes:

| Button | evdev Code | Description |
|--------|-----------|-------------|
| `BTN_JOYSTICK` | `0x120` | Joystick button 0 |
| `BTN_TRIGGER` | `0x120` | Alias for BTN_JOYSTICK |
| `BTN_THUMB` | `0x121` | Button 1 |
| `BTN_THUMB2` | `0x122` | Button 2 |
| `BTN_TOP` | `0x123` | Button 3 |
| `BTN_TOP2` | `0x124` | Button 4 |
| `BTN_PINKIE` | `0x125` | Button 5 |
| `BTN_BASE` | `0x126` | Button 6 |
| `BTN_BASE2` | `0x127` | Button 7 |
| `BTN_BASE3` | `0x128` | Button 8 |
| `BTN_BASE4` | `0x129` | Button 9 |
| `BTN_BASE5` | `0x12A` | Button 10 |
| `BTN_BASE6` | `0x12B` | Button 11 |
| `BTN_DEAD` | `0x12F` | Button 15 |

**Note:** The Cyborg 2 has 26+ physical buttons mapped to various evdev key codes.

### 3.4 Relative Events (Mouse Mode)

When configured as mouse, the device generates `EV_REL` events:

| Axis | evdev Code | Description |
|------|-----------|-------------|
| `REL_X` | `0x00` | X movement |
| `REL_Y` | `0x01` | Y movement |
| `REL_WHEEL` | `0x08` | Vertical wheel |
| `REL_HWHEEL` | `0x06` | Horizontal wheel |

---

## 4. Button Types (from official app)

The Azeron software defines the following button type codes:

| Code | Type | Description |
|------|------|-------------|
| `1` | `KeyboardKey` | Standard keyboard key |
| `2` | `Switch` | Toggle switch |
| `3` | `AnalogJoystick` | Analog joystick (no keys) |
| `4` | `AnalogJoystickWithKeys` | Analog + directional keys |
| `5` | `DirectInput` | DirectInput button |
| `6` | `Disabled` | Disabled button |
| `7` | `AnalogJoystickWithKeysUp` | Analog + up key only |
| `8` | `AnalogJoystickWithKeysRight` | Analog + right key only |
| `9` | `AnalogJoystickWithKeysDown` | Analog + down key only |
| `10` | `AnalogJoystickWithKeysLeft` | Analog + left key only |
| `11` | `None` | No binding |
| `12` | `DirectInputDpad` | D-pad direct input |
| `13` | `ToggleAnalog` | Toggle analog mode |
| `14` | `ToggleAnalogShort` | Short toggle analog |
| `15` | `MouseButton` | Mouse button |
| `16` | `Macro` | Macro execution |
| `17` | `AnalogJoystickAndWithKeysUp` | Analog AND up key |
| `18` | `AnalogJoystickAndWithKeysRight` | Analog AND right key |
| `19` | `AnalogJoystickAndWithKeysDown` | Analog AND down key |
| `20` | `AnalogJoystickAndWithKeysLeft` | Analog AND left key |
| `21` | `XInputJoystick` | XInput joystick |
| `22` | `XInputButton` | XInput gamepad button |
| `23` | `XInputTrigger` | XInput trigger (LT/RT) |
| `24` | `SwitchProfile` | Profile switcher |
| `25` | `AnalogJoystickWithDriftAndKeysUp` | Drift compensation + up |
| `26` | `AnalogJoystickWithDriftAndKeysRight` | Drift compensation + right |
| `27` | `AnalogJoystickWithDriftAndKeysDown` | Drift compensation + down |
| `28` | `AnalogJoystickWithDriftAndKeysLeft` | Drift compensation + left |
| `29` | `MouseWheelUp` | Mouse wheel up |
| `30` | `MouseWheelDown` | Mouse wheel down |
| `31` | `MouseWheelTrigger` | Mouse wheel trigger |
| `32` | `ProfileWheelModifier` | Profile wheel modifier |
| `33` | `DpiUp` | DPI increase |
| `34` | `DpiDown` | DPI decrease |
| `35` | `ButtonMouseWheel` | Button-as-mouse-wheel |
| `36` | `AnalogWheel` | Analog wheel mode |
| `37` | `InputSequence` | Input sequence |
| `38` | `MediaKeys` | Media key |
| `39` | `XInputJoystickAndKeys` | XInput joystick + keys |
| `40` | `JoystickMouse` | Joystick-as-mouse |

### 4.1 Button Type Categories

| Category | Types |
|----------|-------|
| `NONE` | None, Disabled |
| `KEYBOARD` | KeyboardKey |
| `JOYSTICK` | AnalogJoystick, DirectInput |
| `JOYSTICK_DPAD` | DirectInputDpad |
| `MOUSE` | MouseButton, MouseWheelUp, MouseWheelDown |
| `MEDIA` | MediaKeys |
| `MACRO` | Macro, InputSequence |
| `SPECIAL` | ToggleAnalog, ToggleAnalogShort |
| `SHORTCUT` | SwitchProfile |
| `DISABLE` | Disabled |
| `XINPUT` | XInputButton, XInputTrigger |
| `X360_STICK` | XInputJoystick |
| `KEYBOARD_STICK` | AnalogJoystickWithKeys variants |
| `ANALOG_JOYSTICK` | AnalogJoystick variants |
| `ANALOG_WHEEL` | AnalogWheel |
| `MOUSE_WHEEL` | MouseWheelUp, MouseWheelDown |
| `MOUSE_WHEEL_TRIGGER` | MouseWheelTrigger |
| `PROFILE_WHEEL_MODIFIER` | ProfileWheelModifier |
| `DPI_BUTTON` | DpiUp, DpiDown |
| `BUTTON_MOUSE_WHEEL` | ButtonMouseWheel |
| `X360_KEYBOARD_STICK` | XInputJoystickAndKeys |
| `JOYSTICK_MOUSE` | JoystickMouse |

---

## 5. Profile Structure

### 5.1 Profile Header

```
Profile ID: 4 bytes (UUID)
Name: Variable length string
Is Software: 1 byte boolean
Is Favorite: 1 byte boolean
Version: 1 byte
```

### 5.2 Button/Input Structure

```
Input ID: 1 byte (0-31)
Types: Array[3] of button type codes (primary, long press, double press)
Pin One: 1 byte (physical pin number)
Pin Two: 1 byte (physical pin number, 255 = unused)
Key Values: Array[4] of key codes
Meta Values: Array[3] of modifier codes
Macro: Object { repeat: bool, steps: Array }
Long Macro: Object { repeat: bool, steps: Array }
Double Macro: Object { repeat: bool, steps: Array }
Label: String (user-defined)
Feature Delay: 2 bytes (ms, default 500)
Double Delay: 2 bytes (ms, default 150)
X/Y/Interval: Mouse wheel settings
Is Hold: 3x boolean (press, long, double)
Hold Time: 3x 2-byte values (ms)
Analog Settings: Object (for stick input ID 24)
```

### 5.3 Analog Settings Structure

```javascript
{
  angle: 0,                    // Global angle offset
  lowerLimit: 0,               // Lower output limit
  upperLimit: 0,               // Upper output limit
  sensitivity: 0,              // Stick sensitivity
  mouseSensitivity: 5,         // Mouse mode sensitivity
  analogKeys: {
    left: { up: [0,0,0], right: [0,0,0], down: [0,0,0], left: [0,0,0] },
    right: { up: [0,0,0], right: [0,0,0], down: [0,0,0], left: [0,0,0] }
  },
  diagonalKeys: {
    left: { up_right: [0,0,0], up_left: [0,0,0], down_left: [0,0,0], down_right: [0,0,0] },
    right: { up_right: [0,0,0], up_left: [0,0,0], down_left: [0,0,0], down_right: [0,0,0] }
  },
  analogCones: {
    verticalCone: 45,          // Vertical deadzone cone (degrees)
    horizontalCone: 45         // Horizontal deadzone cone (degrees)
  },
  isEightDirectionalTrigger: false,
  analogThrottle: 0,           // Output throttle (0-255)
  isAnalogSmoothing: false,    // Enable input smoothing
  triggerMagnitude: 4          // Trigger activation threshold
}
```

---

## 6. XInput Button Bitmask

When in XInput mode, gamepad buttons are encoded as a 16-bit bitmask:

| Bit | Button | Label |
|-----|--------|-------|
| 0 | `0x0001` | DPAD UP |
| 1 | `0x0002` | DPAD DOWN |
| 2 | `0x0004` | DPAD LEFT |
| 3 | `0x0008` | DPAD RIGHT |
| 4 | `0x0010` | START |
| 5 | `0x0020` | BACK |
| 6 | `0x0040` | PUSH L STICK |
| 7 | `0x0080` | PUSH R STICK |
| 8 | `0x0100` | LB |
| 9 | `0x0200` | RB |
| 10 | `0x0400` | HOME (Xbox button) |
| 12 | `0x1000` | A |
| 13 | `0x2000` | B |
| 14 | `0x4000` | X |
| 15 | `0x8000` | Y |
| T1 | Analog | LT (Left Trigger) |
| T2 | Analog | RT (Right Trigger) |

---

## 7. Trigger Types

| Code | Type | Description |
|------|------|-------------|
| `0` | `SINGLE_CLICK` | Standard press/release |
| `1` | `LONG_PRESS` | Hold for feature delay (default 500ms) |
| `2` | `DOUBLE_PRESS` | Double-click within double delay (default 150ms) |
| `3` | `CHORDING` | Multi-button combination |
| `4` | `NONE` | No trigger |

---

## 8. LED Control

### 8.1 LED Effects

| Code | Effect |
|------|--------|
| `0` | `ProfileSwitch` |
| `1` | `Profile_1` |
| `2` | `Profile_2` |
| `3` | `Profile_3` |
| `4` | `Profile_4` |
| `5` | `Profile_5` |
| `6` | `Profile_6` |
| `7` | `Cancel` |
| `8` | `Idle` |

### 8.2 Default LED Color

```javascript
{ r: 55, g: 173, b: 173 }  // Teal/cyan default
```

---

## 9. Mouse Sensor Settings

### 9.1 Sensitivity Presets

```javascript
{
  currentSensitivityIndex: 2,
  sensitivityValues: [500, 2500, 5000, 10000, 15000]
}
```

### 9.2 Global Mouse Settings

| Setting | Range | Default |
|---------|-------|---------|
| Sensitivity | 50-16000 | 5000 |
| Angle | 0-359 | 0 |
| DPI Levels | 1-5 | 3 |

---

## 10. Firmware Update Process

### 10.1 DFU Mode Entry

1. Send `DFU` command (text) or binary `DFU` message
2. Device disconnects and reconnects as STM32 bootloader
3. New USB ID: `0x0483:0xDF11`

### 10.2 DFU Update

```bash
# List DFU devices
dfu-util -l -v

# Flash firmware (example for Cyborg V2)
dfu-util -d 0483:df11 -a <alt> -s <address>:leave -S <serial> -D azeron-fw-cyborg-v2-<version>.bin
```

### 10.3 Firmware File Names

| Device | Performance | Standard |
|--------|-------------|----------|
| Cyborg V2 | `cyborg-v2-p` | `cyborg-v2` |
| Cyro | `cyro-p` | `cyro` |
| Cyro Lefty | `cyro-lefty-p` | `cyro-lefty` |
| Keyzen | `keyzen-p` | `keyzen` |

---

## 11. Software Button Injection

The Azeron software can inject button presses via the `SOFTWARE_BUTTON` binary message:

```
Byte 0: Is pressed (1/0)
Byte 1: Input ID
Byte 2: Unknown (0)
Byte 3: Is toggle on hold (1/0)
Byte 4-5: Key code (big-endian uint16)
Byte 6: Is macro (1/0)
Byte 7-8: Macro ID (big-endian uint16)
Byte 9: Is hold (1/0)
```

---

## 12. Implementation Notes for remapper_rs

### 12.1 Event Loop Integration

The evdev event loop in remapper_rs currently handles:
- `EV_KEY` -> `DeviceEventType::KeyPress` / `KeyRelease`
- `EV_REL` -> `DeviceEventType::MouseMove`

**Missing:** `EV_ABS` handling for analog sticks.

### 12.2 Required Changes

1. ✅ **Add `DeviceEventType::AbsAxis` variant** — DONE. Added `AbsAxis { axis: AbsoluteAxisType, value: i32 }` to the enum.
2. ✅ **Handle `EventType::ABSOLUTE` in the evdev event loop** — DONE. The event loop processes `AbsAxis` events inline and sends them through the channel via `abs_axis_event()`.
3. ✅ **Route ABS_X/ABS_Y events to analog_processor** — DONE. Gamepad/WASD/Mouse/Camera/D-pad modes are processed inline with `analog_processor`.
4. ✅ **Support ABS_HAT0X/ABS_HAT0Y for D-pad emulation** — DONE. Hat switch events are mapped to key presses/releases.
5. ✅ **Wire AbsAxis through the channel** — DONE. Events are sent via `sender.try_send(msg)` and handled in `main.rs` by forwarding to `injector.analog_move()`.

### 12.3 Event Coalescing (Recommended Optimization)

**Problem:** Rapid analog stick events (every 1-8ms) flood the event loop and channel.

**Solution:** Implement event coalescing in the device event loop:

```rust
// Coalesce analog events: only process every N ms or when delta exceeds threshold
const ANALOG_COALESCE_MS: u64 = 5;  // Process at most every 5ms
const ANALOG_DELTA_THRESHOLD: i32 = 256;  // ~0.4% of full range

// In the event loop:
let now = Instant::now();
let should_process = match axis {
    AbsoluteAxisType::ABS_X | AbsoluteAxisType::ABS_Y => {
        let time_ok = last_analog_time.map(|t| t.elapsed().as_millis() >= ANALOG_COALESCE_MS).unwrap_or(true);
        let delta_ok = last_analog_values.get(&axis).map(|last| (value - last).abs() >= ANALOG_DELTA_THRESHOLD).unwrap_or(true);
        time_ok || delta_ok
    }
    _ => true,  // Non-stick axes always process
};

if should_process {
    last_analog_time = Some(now);
    last_analog_values.insert(axis, value);
    // ... process event
}
```

### 12.4 Caching analog_mode and effective_layer (Recommended Optimization)

**Problem:** Double `layer_manager.read()` per analog event causes runtime starvation.

**Solution:** Cache per-device state:

```rust
// Cached per-device analog state
struct CachedAnalogState {
    analog_mode: AnalogMode,
    effective_layer: u32,
    last_updated: Instant,
}

// In the event loop, use cached value if fresh (< 100ms):
let cached = cached_analog_states.get(&id);
let (analog_mode, effective_layer) = if let Some(cached) = cached {
    if cached.last_updated.elapsed().as_millis() < 100 {
        (cached.analog_mode, cached.effective_layer)
    } else {
        // Refresh from layer_manager
        let (mode, layer) = fetch_from_layer_manager(&id).await;
        cached_analog_states.insert(id, CachedAnalogState { analog_mode: mode, effective_layer: layer, last_updated: Instant::now() });
        (mode, layer)
    }
} else {
    // First time: fetch and cache
    let (mode, layer) = fetch_from_layer_manager(&id).await;
    cached_analog_states.insert(id, CachedAnalogState { analog_mode: mode, effective_layer: layer, last_updated: Instant::now() });
    (mode, layer)
};
```

### 12.5 Analog Value Processing

```rust
// Raw evdev value: 0-65535 (u16)
// Center: ~32768
// Convert to signed: value as i32 - 32768
// Results in range: -32768 to +32767
```

### 12.4 Deadzone Handling

The firmware supports hardware deadzone configuration:
- Lower deadzone: `HWLWRDZ` command
- Upper deadzone: `HWUPRDZ` command
- Offset: `HWANLGOFST` command

Software deadzone should be applied in addition to hardware deadzone.

### 12.5 Completed Changes (2026-05-29)

| Change | Status | File |
|--------|--------|------|
| Added `DeviceEventType::AbsAxis` variant | ✅ Complete | `aethermapd/src/device.rs` |
| Added `abs_axis_event()` constructor | ✅ Complete | `aethermapd/src/device.rs` |
| Wired AbsAxis through event channel | ✅ Complete | `aethermapd/src/device.rs`, `aethermapd/src/main.rs` |
| Handle AbsAxis in main event loop | ✅ Complete | `aethermapd/src/main.rs` |
| Handle AbsAxis in test_grab tool | ✅ Complete | `aethermapd/src/bin/test_grab.rs` |
| Removed fake key-event encoding (61000-61005) | ✅ Complete | `aethermapd/src/device.rs` |
| Forward AbsAxis to `injector.analog_move()` | ✅ Complete | `aethermapd/src/main.rs` |

### 12.6 Remaining Optimizations (Future Work)

1. **Event coalescing** — Process analog events every N ms or when delta exceeds threshold
2. **Caching analog_mode/effective_layer** — Cache per-device state to avoid double `layer_manager.read()`
3. **Add `DeviceEventType::AbsAxis` to macro engine** — Currently macro engine only handles `Key` events

---

## 13. References

- **azeron-linux**: Node.js patcher for Azeron software (Wine + node-hid)
- **azeron-cli** (cozyGalvinism/azeron-cli): Rust CLI tool with protocol RE
- **noreza** (Caedis/noreza): Go-based Linux input mapper
- **Official Azeron Software**: Electron app with minified JS protocol implementation

---

*Document version: 1.2*
*Last updated: 2026-05-29*
*Sources: azeron-linux project, azeron-cli GitHub, noreza GitHub, official Azeron Software v1.5.6*
