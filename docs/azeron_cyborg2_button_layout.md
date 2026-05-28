# Azeron Cyborg 2 Button Layout Reference

## Document Information

- **Device**: Azeron Cyborg II
- **Total Buttons**: 26 joystick buttons (JOY_BTN_0 to JOY_BTN_25)
- **Analog Input**: 1 analog stick (ABS_X, ABS_Y)
- **Hat Switch**: 1 D-pad (ABS_HAT0X, ABS_HAT0Y)
- **Source**: Reverse-engineered from official Azeron Software, azeron-cli, and noreza projects

---

## 1. Physical Button Layout

The Azeron Cyborg 2 has a distinctive layout with:
- **Finger buttons**: 5 columns x 3 rows = 15 buttons
- **Thumb cluster**: Additional buttons around the analog stick
- **Analog stick**: Thumb-operated, configurable as analog or D-pad
- **D-pad/Hat**: Directional pad (can be physical or emulated from analog)

### 1.1 Official Button IDs (from azeron-cli)

The official Azeron software uses button IDs 1-38. Not all are user-mappable.

```
                    [Tower Buttons - top row]
         [14] [13] [12] [11] [10]  (top pinky to top index)
         
         [Finger Buttons - main grid]
         [19] [18] [17] [16] [15]  (second row)
         [24] [23] [22] [21] [20]  (third row)
         [29] [28] [27] [26] [25]  (fourth row)
         
         [Thumb Area]
         [30] [31] [32] [33]       (thumb buttons)
         [Analog Stick]            (center)
         [34] [35] [36] [37]       (stick directions when in D-pad mode)
         [38]                      (mode switch / special)
```

**Note:** The exact physical layout varies by hand size configuration. The button IDs above are from the official software's internal numbering.

### 1.2 Noreza Hardware Profile Mapping

For use with Linux input mappers, the recommended hardware profile is:

1. **Joystick**: Set to "Analog Joystick" mode
2. **All buttons**: Set to joystick buttons (JOY_0 through JOY_25)
3. **Hat/D-pad**: Set to joystick dpad up, down, left, right
4. **Number in bottom left of key**: Should match which JOY key is assigned

---

## 2. evdev Event Mapping

### 2.1 Joystick Buttons to evdev

| JOY_BTN_N | evdev Code | evdev Name | Typical Physical Position |
|-----------|------------|------------|---------------------------|
| JOY_BTN_0 | 0x120 | BTN_JOYSTICK | Index finger top |
| JOY_BTN_1 | 0x121 | BTN_THUMB | Middle finger top |
| JOY_BTN_2 | 0x122 | BTN_THUMB2 | Ring finger top |
| JOY_BTN_3 | 0x123 | BTN_TOP | Pinky finger top |
| JOY_BTN_4 | 0x124 | BTN_TOP2 | Index finger middle |
| JOY_BTN_5 | 0x125 | BTN_PINKIE | Middle finger middle |
| JOY_BTN_6 | 0x126 | BTN_BASE | Ring finger middle |
| JOY_BTN_7 | 0x127 | BTN_BASE2 | Pinky finger middle |
| JOY_BTN_8 | 0x128 | BTN_BASE3 | Index finger bottom |
| JOY_BTN_9 | 0x129 | BTN_BASE4 | Middle finger bottom |
| JOY_BTN_10 | 0x12A | BTN_BASE5 | Ring finger bottom |
| JOY_BTN_11 | 0x12B | BTN_BASE6 | Pinky finger bottom |
| JOY_BTN_12 | 0x12C | - | Thumb button 1 |
| JOY_BTN_13 | 0x12D | - | Thumb button 2 |
| JOY_BTN_14 | 0x12E | - | Thumb button 3 |
| JOY_BTN_15 | 0x12F | BTN_DEAD | Thumb button 4 |

**Note:** JOY_BTN_16 through JOY_BTN_25 may map to additional buttons or extended ranges depending on firmware version.

### 2.2 Internal Remapper Codes

The remapper uses an internal mapping to provide consistent remappable interface:

```rust
const JOY_BTN_BASE: u16 = 50000;

// BTN_JOYSTICK (0x120) -> JOY_BTN_0 (50000)
// BTN_DEAD (0x12F) -> JOY_BTN_15 (50015)
```

### 2.3 Analog Stick Axes

| Axis | evdev Code | Name | Range | Description |
|------|------------|------|-------|-------------|
| X | 0x00 | ABS_X | -32767 to 32767 | Left/Right stick movement |
| Y | 0x01 | ABS_Y | -32767 to 32767 | Up/Down stick movement |

**Normalized values:**
```rust
let x_norm = value as f32 / 32767.0; // -1.0 to 1.0
```

### 2.4 Hat Switch (D-pad)

| Axis | evdev Code | Values |
|------|------------|--------|
| X | 0x10 (ABS_HAT0X) | -1 = Left, 0 = Center, 1 = Right |
| Y | 0x11 (ABS_HAT0Y) | -1 = Up, 0 = Center, 1 = Down |

**Direction mapping:**
```
Y=-1: UP          Y=-1,X=1: UP+RIGHT
X=1: RIGHT        Y=1,X=1: DOWN+RIGHT
Y=1: DOWN         Y=1,X=-1: DOWN+LEFT
X=-1: LEFT        Y=-1,X=-1: UP+LEFT
```

---

## 3. Button Type Configuration

### 3.1 Official Software Button Types

| Type | Use Case |
|------|----------|
| KeyboardKey | Map to keyboard key |
| MouseButton | Map to mouse button |
| JoystickButton | Pass through as joystick button |
| AnalogJoystick | Use stick as analog input |
| JoystickHat | Use as D-pad |
| XInputButton | Xbox controller button |
| XInputTrigger | Xbox controller trigger |
| Macro | Execute recorded macro |
| SwitchProfile | Switch between profiles |

### 3.2 Linux Mapper Configuration

For Linux input mappers (noreza/remapper_rs), the typical configuration is:

```yaml
# All buttons set to joystick buttons for maximum flexibility
buttons:
  - id: 0
    type: joystick_button
  - id: 1
    type: joystick_button
  # ... etc for all 26 buttons

# Stick set to analog
analog:
  mode: analog_joystick
  deadzone: 14000  # ~43%

# Hat set to D-pad
hat:
  mode: joystick_hat
```

---

## 4. Default Key Mappings

### 4.1 Common Gaming Layouts

**FPS Layout (example):**
```
JOY_BTN_0  -> W (Forward)
JOY_BTN_1  -> A (Left)
JOY_BTN_2  -> S (Backward)
JOY_BTN_3  -> D (Right)
JOY_BTN_4  -> SPACE (Jump)
JOY_BTN_5  -> LCTRL (Crouch)
JOY_BTN_6  -> LSHIFT (Sprint)
JOY_BTN_7  -> R (Reload)
JOY_BTN_8  -> 1 (Weapon 1)
JOY_BTN_9  -> 2 (Weapon 2)
JOY_BTN_10 -> 3 (Weapon 3)
JOY_BTN_11 -> G (Grenade)
# ... etc
```

**MMO Layout (example):**
```
JOY_BTN_0  -> 1
JOY_BTN_1  -> 2
JOY_BTN_2  -> 3
JOY_BTN_3  -> 4
JOY_BTN_4  -> 5
JOY_BTN_5  -> Q
JOY_BTN_6  -> E
JOY_BTN_7  -> R
JOY_BTN_8  -> F
# ... etc
```

### 4.2 Arrow Key Mapping (for D-pad/hat)

| Direction | Default Key Code | Name |
|-----------|-----------------|------|
| Up | 103 | KEY_UP |
| Down | 108 | KEY_DOWN |
| Left | 105 | KEY_LEFT |
| Right | 106 | KEY_RIGHT |
| Up+Right | 103+106 | KEY_UP + KEY_RIGHT |
| Down+Right | 108+106 | KEY_DOWN + KEY_RIGHT |
| Down+Left | 108+105 | KEY_DOWN + KEY_LEFT |
| Up+Left | 103+105 | KEY_UP + KEY_LEFT |

---

## 5. Event Processing Pipeline

### 5.1 Event Flow

```
[Hardware Button Press]
    |
    v
[evdev InputEvent (EV_KEY)]
    |
    v
[map_joystick_button()] -> JOY_BTN_N code (50000-50025)
    |
    v
[Hotkey Manager Check] -> Consumed? (yes -> stop)
    |
    v
[Remap Engine] -> Remapped? (yes -> inject, stop)
    |
    v
[Macro Engine] -> Consumed? (yes -> stop)
    |
    v
[Passthrough] -> Inject original event
```

### 5.2 Analog Event Flow

```
[Stick Movement]
    |
    v
[evdev InputEvent (EV_ABS)]
    |
    v
[Is Hat Switch?] -> Yes -> Map to D-pad keys
    |                    |
    |                    v
    |               [Send key events]
    |
    v
[Is Analog Stick?] -> Yes -> Broadcast for GUI
    |                        |
    |                        v
    |                   [Check D-pad Mode]
    |                        |
    |              [Enabled?] -> Yes -> Emulate D-pad
    |                        |
    |                        v
    |                   [Check Gamepad Mode]
    |                        |
    |              [Enabled?] -> Yes -> Virtual gamepad
    |                        |
    |                        v
    |                   [Check WASD Mode]
    |                        |
    |              [Enabled?] -> Yes -> WASD keys
    |                        |
    |                        v
    |                   [Check Mouse Mode]
    |                        |
    |              [Enabled?] -> Yes -> Mouse movement
    |                        |
    |                        v
    |                   [Check Camera Mode]
    |                        |
    |              [Enabled?] -> Yes -> Camera movement
    |                        |
    |                        v
    |                   [Send to macro engine]
```

---

## 6. Calibration

### 6.1 Deadzone Configuration

```rust
// Default: 43% of 32767 = ~14000 raw units
const DEFAULT_DEADZONE: u16 = 14000;

// Per-axis deadzone
pub struct AnalogDeviceConfig {
    pub deadzone_percentage: u8,      // 0-100
    pub deadzone_percentage_x: u8,    // 0-100
    pub deadzone_percentage_y: u8,    // 0-100
    pub outer_deadzone_percentage: u8, // 0-100
    pub sensitivity: f32,             // 0.1-5.0
    pub response_curve: String,       // "linear" or "exponential"
    pub dpad_mode: String,            // "disabled", "eight_way", "four_way"
}
```

### 6.2 Response Curves

**Linear:**
```
output = input * sensitivity
```

**Exponential:**
```
output = sign(input) * |input|^exponent * sensitivity
```

---

## 7. Implementation Constants

### 7.1 From device.rs

```rust
// Azeron vendor ID
const AZERON_VENDOR_ID: u16 = 0x16d0;

// Joystick button base for internal mapping
const JOY_BTN_BASE: u16 = 50000;

// Joystick button range
const BTN_JOYSTICK_START: u16 = 0x120;
const BTN_JOYSTICK_END: u16 = 0x12F;

// Hat switch directions
const HAT_UP: u16 = 103;    // KEY_UP
const HAT_DOWN: u16 = 108;  // KEY_DOWN
const HAT_LEFT: u16 = 105;  // KEY_LEFT
const HAT_RIGHT: u16 = 106; // KEY_RIGHT
```

### 7.2 From gamepad_device.rs

```rust
// Absolute axis codes
pub const ABS_X: u16 = 0x00;
pub const ABS_Y: u16 = 0x01;
pub const ABS_Z: u16 = 0x02;
pub const ABS_RX: u16 = 0x03;
pub const ABS_RY: u16 = 0x04;
pub const ABS_RZ: u16 = 0x05;

// Xbox 360 controller identifiers
const XBOX360_VENDOR_ID: u16 = 0x045e;  // Microsoft
const XBOX360_PRODUCT_ID: u16 = 0x028e; // Xbox 360 Controller

// Axis range
const AXIS_MIN: i32 = -32768;
const AXIS_MAX: i32 = 32767;
```

### 7.3 From analog_processor.rs

```rust
// Default deadzone
const DEFAULT_DEADZONE: u16 = 14000;

// Maximum absolute value
pub const MAX_ABS_VALUE: i32 = 32767;

// Analog event codes (for macro engine)
// 61000: ABS_X, 61001: ABS_Y, 61002: ABS_Z
// 61003: ABS_RX, 61004: ABS_RY, 61005: ABS_RZ
```

---

## 8. References

- **azeron-linux**: https://github.com/renatoi/azeron-linux
- **azeron-cli**: https://github.com/cozyGalvinism/azeron-cli
- **noreza**: https://github.com/Caedis/noreza
- **Linux Input Event Codes**: https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h

---

## 9. Change Log

| Date | Change |
|------|--------|
| 2026-05-29 | Initial button layout documentation |
| 2026-05-29 | Updated with AbsAxis event type and event flow changes |
