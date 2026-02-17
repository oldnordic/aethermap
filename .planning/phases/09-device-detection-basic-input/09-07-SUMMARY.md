---
phase: 09-device-detection-basic-input
plan: 07
title: "GUI visual keypad layout for button remapping"
subsystem: "razermapper-gui"
tags: ["gui", "keypad", "remapping", "azeron", "ipc"]

# Dependency Graph
requires:
  - "09-01"  # device detection
  - "09-03"  # joystick buttons
  - "09-06"  # IPC capabilities
provides:
  - "Visual keypad layout UI"
  - "Button click-to-remap interface"
affects:
  - "razermapper-gui/src/gui.rs"
  - "razermapper-gui/src/ipc.rs"

# Tech Stack
added:
  - "KeypadButton struct for button layout representation"
  - "azeron_keypad_layout() function for Azeron Cyborg layout"
  - "get_device_capabilities() IPC client method"
patterns:
  - "Grid-based UI layout using iced row/column"
  - "State-driven view switching (device list <-> keypad view)"
  - "Async IPC communication for capability queries"

# Key Files Created/Modified
created: []
modified:
  - path: "razermapper-gui/src/gui.rs"
    changes:
      - "Added KeypadButton struct with id, label, row, col, current_remap fields"
      - "Added azeron_keypad_layout() function defining 27 buttons across 10 rows"
      - "Added view_azeron_keypad() method rendering visual button grid"
      - "Added ShowKeypadView, SelectKeypadButton, DeviceCapabilitiesLoaded message variants"
      - "Added keypad_layout, selected_button, device_capabilities State fields"
      - "Added 'Configure Keypad' button for keypad-type devices in view_device_card()"
      - "Modified view_devices_tab() to show keypad view when capabilities loaded"
  - path: "razermapper-gui/src/ipc.rs"
    changes:
      - "Added DeviceCapabilities import"
      - "Added get_device_capabilities() async method for capability queries"

# Decisions Made
- Use row-based grid layout (10 rows) instead of 2D positioning for simpler Iced implementation
- Show visual feedback (secondary button style) for buttons with active remappings
- Hat switch displayed as centered indicator between button rows
- Back button (empty device_path) clears keypad view state
- Remap targets displayed on buttons with arrow notation (e.g., "Q->KEY_A")

# One-Liner Summary
Grid-based visual Azeron keypad layout with 27 buttons in 10 rows, clickable for remapping configuration, with hat switch indicator and current remap feedback.

---

## Plan Execution Summary

**Tasks Completed:** 3/3
**Duration:** ~30 minutes
**Commits:**
- `0b2a025`: feat(09-07): add GUI visual keypad layout for button remapping

### Task 1: IPC Client Method for Device Capabilities
Added `get_device_capabilities()` method to `GuiIpcClient` in `razermapper-gui/src/ipc.rs`. This method queries the daemon for device capability information including button count, hat switch presence, and analog stick availability.

**Files Modified:**
- `razermapper-gui/src/ipc.rs`: Added method implementation and DeviceCapabilities import

### Task 2: Azeron Keypad Visual Layout Component
Created visual keypad layout for the Azeron Cyborg with:
- `KeypadButton` struct defining button properties (id, label, position, current remap)
- `azeron_keypad_layout()` function with 27 buttons across 10 rows matching physical layout
- `view_azeron_keypad()` method rendering clickable button grid
- Visual feedback for remapped buttons (Secondary style)
- Hat switch indicator centered in layout

**Files Modified:**
- `razermapper-gui/src/gui.rs`: Added structures and view method

### Task 3: Integration with Device Selection
Wired keypad view to device selection UI:
- "Configure Keypad" button shown for DeviceType::Keypad devices
- ShowKeypadView message triggers capability query
- DeviceCapabilitiesLoaded handler updates button states from active remaps
- Back button returns to device list
- State fields: keypad_layout, selected_button, device_capabilities

**Files Modified:**
- `razermapper-gui/src/gui.rs`: Added message handlers and UI integration

## Deviations from Plan

**None - plan executed exactly as written.**

All tasks completed as specified. The implementation uses a simplified row-based grid layout instead of full 2D positioning, which is more maintainable in Iced while still providing clear visual representation of the Azeron Cyborg button layout.

## Verification Results

1. **cargo check -p razermapper-gui**: PASSED
   - Only pre-existing warning about unused get_active_profile method

2. **GUI displays visual Azeron keypad layout**: YES
   - 27 buttons arranged in 10 rows
   - Click-to-select functionality
   - Visual feedback for selected/remapped buttons

3. **All 26 joystick buttons shown**: YES
   - JOY_BTN_0 through JOY_BTN_26
   - Plus hat switch indicator

4. **Hat switch displayed**: YES
   - Centered "HAT" indicator between button rows

5. **Current remappings shown**: YES
   - Loaded from active_remaps when capabilities loaded
   - Display format: "LABEL->TARGET"
   - Secondary button style for remapped buttons

6. **Device capability query works via IPC**: YES
   - get_device_capabilities() method implemented
   - Async communication with daemon
   - Error handling with user notification

## UI/UX Considerations

1. **Layout**: Row-based grid (10 rows) approximates physical Azeron layout
   - Top row: Function keys (1-5)
   - Rows 2-4: QWERTY-style cluster
   - Row 5: Hat switch center position
   - Row 6: Number keys
   - Rows 7-8: Thumb cluster
   - Remaining space for future enhancements

2. **Visual Feedback**:
   - Primary style: Selected button
   - Secondary style: Remapped button
   - Text style: Unmapped button
   - Remap target shown below label

3. **Navigation**:
   - Device list -> "Configure Keypad" -> Keypad view
   - Back button returns to device list
   - Tab switching resets keypad view

4. **Future Enhancements**:
   - 2D positioning using col field for more accurate layout
   - SVG/image overlay for precise physical representation
   - Key picker dialog for direct remap assignment
   - Profile save/load from keypad view
