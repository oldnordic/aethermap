---
phase: 09-device-detection-basic-input
plan: 05
title: "Azeron Keypad Profile Persistence with YAML Support"
subsystem: "Device Profiles - Azeron Support"
tags: ["azeron", "yaml", "profile", "persistence"]
wave: 2

# Dependency Graph
provides:
  - "Profile persistence for Azeron keypad remappings"
  - "JOY_BTN_N key name parsing for joystick buttons"
  - "DeviceCapabilities metadata structure"

requires:
  - "09-01 (Device detection and Azeron classification)"

affects:
  - "aethermapd/src/config.rs (DeviceCapabilities)"
  - "aethermapd/src/key_parser.rs (JOY_BTN_N support)"

# Tech Stack
added:
  - "JOY_BTN_0 through JOY_BTN_25 key name support"
  - "DeviceCapabilities struct for device metadata"

patterns:
  - "Optional serde fields with skip_serializing_if for backward compatibility"
  - "Case-insensitive key name parsing extended to joystick buttons"

# Key Files Created/Modified
created:
  - "None (all modifications to existing files)"

modified:
  - "aethermapd/src/key_parser.rs"
  - "aethermapd/src/config.rs"

# Decisions Made
1. **JOY_BTN_N Naming Convention**: Extended KeyParser to recognize JOY_BTN_N (N=0-25) names, using Linux input code base 0x100 (256) for BTN_0.
2. **Hat Switch Direction Names**: Added hat_up, hat_down, hat_left, hat_right for logical D-pad direction mapping.
3. **DeviceCapabilities Optional**: Made capabilities optional in YAML config to ensure backward compatibility with existing profiles.
4. **Metadata-Only Design**: DeviceCapabilities is metadata for GUI use - actual remapping still uses the remaps list.

# Metrics
duration: "8 minutes"
tasks_completed: 2
created_at: "2026-02-17"
completed_at: "2026-02-17"
commits: 2

# Deviations from Plan
None - plan executed exactly as written.

---

# Phase 09 Plan 05: Azeron Keypad Profile Persistence with YAML Support

## Summary

Added support for Azeron keypad profile persistence with YAML configuration. The KeyParser was extended to recognize joystick button names (JOY_BTN_0 through JOY_BTN_25), and a DeviceCapabilities structure was added to store device metadata for GUI display and profile validation.

## Changes Made

### 1. KeyParser Extensions (aethermapd/src/key_parser.rs)

Extended the KeyParser to support joystick button names for devices like the Azeron keypad:

- **JOY_BTN_N support**: JOY_BTN_0 through JOY_BTN_25 (26 buttons total)
- **Alternative naming**: BTN_N format also supported
- **Hat switch directions**: hat_up, hat_down, hat_left, hat_right
- **Base code**: 0x100 (256) following Linux input subsystem BTN_0 definition

```rust
const JOY_BTN_BASE: u16 = 0x100;
for i in 0..=25u16 {
    let name = format!("joy_btn_{}", i);
    Self::insert_all_variants(&mut name_to_key, &name, JOY_BTN_BASE + i);
}
```

### 2. DeviceCapabilities Structure (aethermapd/src/config.rs)

Added DeviceCapabilities struct to store optional device metadata:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub has_analog_stick: Option<bool>,
    pub has_hat_switch: Option<bool>,
    pub joystick_button_count: Option<usize>,
    pub led_zones: Option<Vec<String>>,
    pub device_type: Option<String>,
}
```

Added to both DeviceRemapConfig and ExtendedDeviceRemapConfig with optional serialization.

### 3. Test Coverage

Added comprehensive tests for:
- Joystick button parsing (JOY_BTN_0 through JOY_BTN_25)
- Hat switch direction parsing
- DeviceCapabilities YAML serialization/deserialization
- Backward compatibility (profiles without capabilities)

## Sample Azeron Profile YAML

```yaml
devices:
  "32b6:12f7":  # Azeron Cyborg
    profiles:
      gaming:
        name: "Gaming Profile"
        remaps:
          joy_btn_0: KEY_A
          joy_btn_1: KEY_B
          joy_btn_2: KEY_C
          hat_up: KEY_W
          hat_down: KEY_S
          hat_left: KEY_A
          hat_right: KEY_D
    capabilities:
      has_analog_stick: true
      has_hat_switch: true
      joystick_button_count: 26
      led_zones:
        - "thumbstick"
        - "wrist_rest"
        - "logo"
      device_type: "Keypad"
```

## Verification

All tests pass:
- JOY_BTN_N names parse correctly for N=0-25
- YAML serialization/deserialization works for DeviceCapabilities
- Optional fields ensure backward compatibility with existing profiles
- cargo check -p aethermapd passes with no errors

## Commits

1. `69afdd6`: feat(09-05): add JOY_BTN_N parsing support for Azeron keypad profiles
2. `9cdfde7`: feat(09-05): add DeviceCapabilities structure for Azeron profile metadata
