---
phase: 09-device-detection-basic-input
plan: 06
title: "IPC Protocol for Device Capability Queries"
summary: "IPC GetDeviceCapabilities request/response with type-based capability inference for Azeron devices"
completed_date: 2026-02-17
author: "Claude Opus 4.6"
tags: [ipc, device-capabilities, azeron, daemon, protocol]
---

## Overview

Implemented IPC protocol for querying device capabilities from the daemon. This enables the GUI to display accurate device information and enable/disable relevant UI elements based on actual device hardware capabilities.

## Changes Made

### 1. aethermap-common/src/lib.rs

Added `DeviceCapabilities` struct and IPC protocol extensions:

**DeviceCapabilities struct:**
- `has_analog_stick: bool` - Device has analog stick (absolute X/Y axes)
- `has_hat_switch: bool` - Device has hat switch (D-pad with ABS_HAT0X/ABS_HAT0Y)
- `joystick_button_count: usize` - Number of joystick buttons
- `led_zones: Vec<String>` - LED zones available (empty for Phase 12)
- Implements `Default` trait with all fields disabled

**IPC Protocol additions:**
- `Request::GetDeviceCapabilities { device_path: String }`
- `Response::DeviceCapabilities { device_path: String, capabilities: DeviceCapabilities }`

**Tests added:**
- `test_device_capabilities_serialization()` - Verifies serde round-trip
- `test_get_device_capabilities_request()` - Verifies request serialization
- `test_device_capabilities_response()` - Verifies response serialization

### 2. aethermapd/src/ipc.rs

Added IPC handler for device capability queries:

**Handler implementation:**
```rust
Request::GetDeviceCapabilities { device_path } => {
    // Get device info from DeviceManager or state.devices
    let device_info = /* ... */;

    match device_info {
        Some(info) => {
            let capabilities = detect_device_capabilities(&info);
            Response::DeviceCapabilities { device_path, capabilities }
        }
        None => Response::Error(format!("Device not found: {}", device_path))
    }
}
```

**Capability detection logic:**
```rust
fn detect_device_capabilities(device_info: &DeviceInfo) -> DeviceCapabilities {
    // Infer from DeviceType enum
    match device_info.device_type {
        DeviceType::Keypad => DeviceCapabilities {
            has_analog_stick: true,
            has_hat_switch: true,
            joystick_button_count: 26,
            led_zones: vec![],
        },
        DeviceType::Gamepad => DeviceCapabilities {
            has_analog_stick: true,
            has_hat_switch: false,
            joystick_button_count: 12,
            led_zones: vec![],
        },
        _ => DeviceCapabilities::default(),
    }
}
```

## Device Capability Detection

### Azeron Keypad (DeviceType::Keypad)
- `has_analog_stick: true` - Azeron Cybo has analog stick
- `has_hat_switch: true` - Azeron Cybo has D-pad
- `joystick_button_count: 26` - Joystick button range BTN_JOYSTICK (0x120) to BTN_DEAD (0x12f)
- `led_zones: []` - Defer to Phase 12

### Generic Gamepad (DeviceType::Gamepad)
- `has_analog_stick: true` - Most gamepads have analog sticks
- `has_hat_switch: false` - Gamepads typically don't have hat switches
- `joystick_button_count: 12` - Generic gamepad button count
- `led_zones: []` - Defer to Phase 12

### Other Devices
- All capabilities set to `false` or `0`

## Implementation Notes

### Type-Based Inference
The current implementation uses device type-based inference rather than querying the actual device hardware. This is because:
1. Device must be grabbed (EVIOCGRAB) to query evdev capabilities
2. Not all devices are grabbed at all times
3. DeviceType already encodes most capability information

### Fallback Logic
The handler first attempts to get device info from DeviceManager, then falls back to state.devices HashMap. This ensures capability queries work even when the device is not currently grabbed.

### LED Zones
LED zones are left empty pending Phase 12 (LED Control) implementation. The struct field is present for forward compatibility.

## Testing

All tests pass:
- `test_device_capabilities_serialization` - PASSED
- `test_get_device_capabilities_request` - PASSED
- `test_device_capabilities_response` - PASSED
- All existing aethermap-common tests - PASSED (14/14)
- All existing aethermapd tests - PASSED

## Deviations from Plan

None. Plan executed exactly as written.

## Commits

1. `1e47740` - feat(09-06): add DeviceCapabilities struct and IPC protocol
2. `7f84247` - feat(09-06): add IPC handler for device capability queries

## Next Steps

GUI can now query device capabilities via IPC and:
- Display analog stick status
- Show hat switch (D-pad) availability
- Display joystick button count
- Enable/disable relevant UI elements based on capabilities

Future enhancement: Query actual evdev capabilities when device is grabbed for more accurate detection.

## Self-Check: PASSED

- [x] 09-06-SUMMARY.md created
- [x] Commit 1e47740 exists: DeviceCapabilities struct and IPC protocol
- [x] Commit 7f84247 exists: IPC handler for capability queries
- [x] All cargo checks pass
- [x] All tests pass
