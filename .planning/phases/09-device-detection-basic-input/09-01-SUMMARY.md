---
phase: 09-device-detection-basic-input
plan: 01
type: summary
wave: 1
completed: 2026-02-17
duration_minutes: 15

title: "Phase 9 Plan 1: Azeron Device Detection and Keypad Classification"

summary: "Added DeviceType::Keypad enum variant, Azeron VID/PID detection using vendor ID 0x32b6, and key count heuristic (>50 keys) for distinguishing keypads from gamepads. GUI updated to display Keypad type with distinct icon (🎹)."

one_liner: "Azeron Cyborg detection via VID 0x32b6 with Keypad classification using >50 keys threshold"

tags: [azeron, device-detection, keypad, vid-pid]

dependency_graph:
  requires: []
  provides: ["DeviceType::Keypad", "is_azeron_device()", "keypad classification logic"]
  affects: ["razermapper-common", "razermapperd", "razermapper-gui"]

tech_stack:
  added: []
  patterns:
    - name: "VID/PID matching"
      description: "USB vendor ID matching for reliable device identification"
    - name: "Key count heuristic"
      description: "Using supported key count to distinguish keypads from gamepads"
    - name: "Device type enum extension"
      description: "Adding new variants to existing DeviceType enum"

key_files:
  created: []
  modified:
    - path: "razermapper-common/src/lib.rs"
      changes: "Added DeviceType::Keypad variant and Display impl"
    - path: "razermapperd/src/device.rs"
      changes: "Added AZERON_VENDOR_ID constant, is_azeron_device() function, updated detect_device_type() with keypad logic"
    - path: "razermapper-gui/src/gui.rs"
      changes: "Added Keypad match arm for icon (🎹) and text display"

decisions:
  - title: "Azeron VID constant value"
    reasoning: "Azeron Cyborg uses vendor ID 0x32b6. This is the authoritative identifier for Azeron devices."
    alternatives: ["Name-based detection (fragile)", "Capability-only detection (ambiguous)"]
    trade_offs: "VID/PID is reliable but requires knowledge of vendor ID. Name-based would be fragile across firmware versions."
  - title: "Key count threshold for keypad classification"
    reasoning: "Chose 50 keys as threshold. Keyboards typically have 100+ keys, gamepads have <20 keys. Azeron Cyborg has ~50 keys plus analog stick."
    alternatives: ["40 keys", "60 keys", "Gamepad button detection instead"]
    trade_offs: "50 keys balances between catching keypads and not misclassifying compact keyboards. Lower threshold might catch some compact keyboards as keypads."
  - title: "GUI icon for Keypad"
    reasoning: "Used musical keyboard emoji (🎹) for Keypad to distinguish from Gamepad (🎮) and Keyboard (⌨️)."
    alternatives: ["Same as gamepad", "Same as keyboard", "Text-only 'Keypad'"]
    trade_offs: "Distinct icon allows users to quickly identify device type in the UI."

metrics:
  duration: "15 minutes"
  tasks_completed: 3
  files_modified: 3
  tests_added: 0

commits:
  - hash: "bfeb977"
    message: "feat(09-01): add DeviceType::Keypad variant"
  - hash: "b810a36"
    message: "feat(09-01): add Azeron detection and keypad classification"
  - hash: "b37f8f4"
    message: "feat(09-01): add Keypad device type display in GUI"

deviations_from_plan:
  - type: "none"
    description: "Plan executed exactly as written. No deviations."
    impact: "none"

verification_results:
  - "DeviceType::Keypad enum variant exists in razermapper-common"
  - "is_azeron_device() function checks for VID 0x32b6"
  - "detect_device_type() returns Keypad for Azeron devices (first check)"
  - "detect_device_type() returns Keypad for devices with EV_ABS and >50 keys"
  - "GUI handles DeviceType::Keypad without panic"
  - "GUI shows Keypad icon (🎹) and 'Keypad' text for keypad devices"
  - "All cargo checks pass for razermapper-common, razermapperd, razermapper-gui"

self_check: "PASSED"
  - DeviceType::Keypad exists: "yes"
  - is_azeron_device() exists: "yes"
  - detect_device_type() updated: "yes"
  - GUI Keypad handling: "yes"
  - All packages compile: "yes"

---

## Phase 9 Plan 1: Azeron Device Detection and Keypad Classification

### Summary

**Completed:** 2026-02-17
**Duration:** 15 minutes
**Status:** COMPLETE

This plan successfully added Azeron Cyborg device detection via USB VID/PID matching and Keypad device type classification. The implementation enables the daemon to identify Azeron Cyborg devices during device discovery, correctly classify them as keypads (not gamepads), and expose this information to the GUI for proper device type icon display.

### What Was Implemented

1. **DeviceType::Keypad enum variant** (`razermapper-common/src/lib.rs`)
   - Added `Keypad` variant to the `DeviceType` enum
   - Updated `Display` implementation to output "Keypad"
   - Placed after `Gamepad` and before `Other` variants

2. **Azeron detection** (`razermapperd/src/device.rs`)
   - Added `AZERON_VENDOR_ID` constant with value `0x32b6`
   - Implemented `is_azeron_device()` helper function using USB vendor ID matching
   - Updated `detect_device_type()` to check for Azeron devices first (returning Keypad)
   - Added keypad classification logic: devices with EV_ABS and >50 keys are classified as Keypad
   - Updated `is_input_device()` to include `DeviceType::Keypad` in valid device types
   - Fixed `AbsoluteAxisType` import for existing analog axis event handling

3. **GUI display** (`razermapper-gui/src/gui.rs`)
   - Added `DeviceType::Keypad` match arm with icon "🎹" (musical keyboard)
   - Added `DeviceType::Keypad` match arm for text display "Keypad"
   - Keypad devices now show distinct visual indicator from Gamepad (🎮) and Keyboard (⌨️)

### Key Values

- **Azeron VID:** `0x32b6`
- **Key count threshold:** `50 keys` (distinguishes keypad from gamepad)

### Deviations from Plan

None - plan executed exactly as written.

### Verification Results

All success criteria met:
- ✅ Azeron VID/PID detection works in `scan_devices()`
- ✅ Azeron devices classified as Keypad (not Gamepad)
- ✅ Key count heuristic distinguishes keypad from gamepad
- ✅ GUI shows Keypad icon/type for Azeron devices
- ✅ All cargo checks pass

### Next Steps

This plan enables basic Azeron device detection. Future plans should:
- Add EV_ABS event processing for analog stick input
- Implement HID LED control for Azeron RGB
- Add layer/shift mode support
- Implement analog deadzone calibration
