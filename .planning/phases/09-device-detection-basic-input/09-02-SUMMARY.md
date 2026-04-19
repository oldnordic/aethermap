---
phase: 09-device-detection-basic-input
plan: 02
subsystem: event-reader
tags: [evdev, analog-input, absolute-axis, event-handling]
dependency_graph:
  requires: [09-01]
  provides: [09-03]
  affects: [macro-engine]
tech-stack:
  added: []
  patterns: [event-encoding-with-high-key-codes]
key-files:
  created: []
  modified:
    - path: aethermap/aethermapd/src/device.rs
      changes: Added AbsoluteAxis event handling in event reader
decisions: []
metrics:
  duration: PT8M
  completed_date: 2026-02-17
---

# Phase 09 Plan 02: EV_ABS Event Handling Summary

## One-Liner
Added EV_ABS (absolute axis) event handling to the event reader for analog stick capture using 61000-61005 event code encoding.

## Implementation Details

### Changes Made

**File: `aethermap/aethermapd/src/device.rs`**

1. **Import Added (line 10):**
   - Added `AbsoluteAxisType` to the evdev imports
   - `use evdev::{Device as EvdevDevice, InputEventKind, Key, RelativeAxisType, AbsoluteAxisType};`

2. **New Match Arm in Event Reader (lines 525-554):**
   - Added `InputEventKind::AbsAxis(axis)` match arm
   - Placed after `RelAxis` handling and before the catch-all `_ => {}`
   - Handles 6 common absolute axis types: ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ
   - Unsupported axes are logged with `debug!()` and skipped with `continue`

### Axis Code Mapping

| Linux Axis Constant | Event Code | Description |
|---------------------|------------|-------------|
| `ABS_X` | 61000 | Primary X axis (left stick horizontal) |
| `ABS_Y` | 61001 | Primary Y axis (left stick vertical) |
| `ABS_Z` | 61002 | Z axis (often left trigger) |
| `ABS_RX` | 61003 | Secondary X axis (right stick horizontal) |
| `ABS_RY` | 61004 | Secondary Y axis (right stick vertical) |
| `ABS_RZ` | 61005 | RZ axis (often right trigger) |

The high event code range (61000+) ensures analog events don't conflict with:
- Keyboard events: 0-255
- Mouse events: 60000-60003 (REL_X, REL_Y, REL_WHEEL, REL_HWHEEL)

## Verification Results

### Compilation
```bash
$ cargo check -p aethermapd
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.08s
```

### Code Review Confirmed
- AbsoluteAxis match arm exists in event reader loop
- All 6 axis types (ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ) are mapped
- Events are sent via mpsc channel to macro engine using encoded codes
- Unsupported axes trigger debug log and continue without crashing
- No existing Key or RelAxis handling was modified

## Deviations from Plan

### None

Plan executed exactly as written:
- Added AbsoluteAxis match arm after RelAxis
- Used codes 61000-61005 for axis encoding
- Added debug logging for unsupported axes
- Did not modify existing Key or RelAxis handling

## Testing Notes

The implementation provides the foundation for analog input processing. Future phases will:
- Add deadzone and sensitivity calibration (Phase 11)
- Implement analog-to-digital mapping for D-pad emulation
- Add axis binding to macros and remaps

## Success Criteria Met

- [x] AbsoluteAxis match arm exists in event reader
- [x] Axis codes map to 61000-61005 range
- [x] Unsupported axes are logged and skipped
- [x] cargo check passes without errors

## Next Steps

Plan 09-03 will implement absolute axis capability detection for device discovery.
