# Project State

**Last Updated:** 2026-02-17

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-17)

**Core value:** Remap any input key with persist & switch at runtime
**Current focus:** Phase 9 - Device Detection and Basic Input

## Current Position

Phase: 9 of 12 (Device Detection and Basic Input)
Plan: 6 of 7
Status: In progress
Last activity: 2026-02-17 — Plan 09-06 complete: IPC protocol for device capability queries

Progress:
```
v1.0 (Phases 1-4): [========================================] 100%
v1.1 (Phases 5-8): [========================================] 100%
v1.2 (Phases 9-12): [####..........................] 13% (4/31 plans)
Overall: [=======================================...] 59% (31/52 plans)
```

## Performance Metrics

**Velocity:**
- Total plans completed: 29
- Average duration: ~5-8 minutes per plan
- Total execution time: ~3 hours

**By Phase:**

| Phase | Plans | Status |
|-------|-------|--------|
| 1. Remap Engine | 3 | Complete |
| 2. Device Profiles & IPC | 6 | Complete |
| 3. Hotplug & Hot-Reload | 3 | Complete |
| 4. GUI Enhancements | TBD | Complete |
| 5. Device Filtering & Discovery | 3 | Complete |
| 6. Mouse Event Reading | 4 | Complete |
| 7. Mouse Macro Verification | 4 | Complete |
| 8. GUI Integration | 4 | Complete |
| 9. Device Detection and Basic Input | 7 | In progress |
| 10. Layer System and State Management | 8 | Not started |
| 11. Analog Processing and Calibration | 8 | Not started |
| 12. LED Control | 8 | Not started |

## Accumulated Context

### Decisions

**v1.0 Decisions:**
- RemapEngine uses Arc<RwLock<HashMap<Key, Key>>> for atomic profile switching
- Device profiles use vendor:product hex format for device identification
- SIGHUP handler for configuration hot-reload with nested tokio::select! pattern

**v1.1 Implementation Decisions:**
- Device type detection via capability queries (supported_keys, supported_relative_axes)
- Mouse events use high key codes (60000-60003) to distinguish from keyboard events
- REL_WHEEL_HI_RES events filtered to prevent double-speed scroll
- evdev 0.12.2 uses `RelAxis` not `RelativeAxis` as InputEventKind variant
- GUI subscribes to iced mouse events during recording for visual feedback

**v1.2 Roadmap Decisions:**
- Azeron support split into 4 phases (Device Detection, Layers, Analog, LED)
- hidapi 2.4 added as optional dependency for LED control
- LayerManager component for modifier-based layer activation
- AnalogProcessor for deadzone, sensitivity curves, D-pad emulation
- LED protocol research deferred to Phase 12 planning

**v1.2 Implementation Decisions (Plan 09-01):**
- Azeron Cyborg vendor ID: 0x32b6
- DeviceType::Keypad added for devices with many keys + analog stick
- Key count threshold: 50 keys to distinguish keypad from gamepad
- GUI icon for Keypad: 🎹 (musical keyboard emoji)

**v1.2 Implementation Decisions (Plan 09-02):**
- EV_ABS events use InputEventKind::AbsAxis(AbsoluteAxisType) in evdev 0.12.2
- Absolute axis events encoded with high key codes (61000-61005) to avoid conflicts
- Analog events flow through same mpsc channel as key/mouse events to macro engine
- Unsupported absolute axes logged and skipped (not sent to macro engine)

**v1.2 Implementation Decisions (Plan 09-04):**
- Hat switch (D-pad) uses ABS_HAT0X and ABS_HAT0Y axes with values -1, 0, 1
- HatSwitchState struct tracks X/Y position for 8-way direction decoding
- Cardinal directions map to single arrow keys (KEY_UP=103, KEY_DOWN=108, KEY_LEFT=105, KEY_RIGHT=106)
- Diagonal directions send TWO keys (composable from cardinal keys)
- Previous hat keys are always released before new ones to prevent stuck keys

**v1.2 Implementation Decisions (Plan 09-05):**
- JOY_BTN_N (N=0-25) names map to Linux BTN_N codes starting at 0x100 (256)
- KeyParser extended with joystick button and hat switch direction name support
- DeviceCapabilities struct stores optional metadata for GUI display and profile validation
- DeviceCapabilities fields use skip_serializing_if for backward compatibility with existing profiles

**v1.2 Implementation Decisions (Plan 09-06):**
- DeviceCapabilities IPC protocol uses type-based inference for capability detection
- Azeron keypad: has_analog_stick=true, has_hat_switch=true, joystick_button_count=26
- LED zones left empty pending Phase 12 (LED Control) implementation
- Capability detection uses DeviceType enum rather than querying evdev directly (devices may not be grabbed)

**v1.2 Implementation Decisions (Plan 09-07):**
- Grid-based visual keypad layout using row-based positioning (10 rows)
- KeypadButton struct with id, label, row, col, current_remap fields
- "Configure Keypad" button only shown for DeviceType::Keypad devices
- Visual feedback: Primary style for selected, Secondary for remapped, Text for unmapped
- Hat switch displayed as centered "HAT" indicator in button grid

### Pending Todos

None.

### Blockers/Concerns

**Phase 12 Research Flag:**
- Azeron HID LED protocol requires USB packet capture during planning
- Consider using official software for LED setup initially

**Phase 9 Verification:**
- Azeron VID/PID values (0x32b6:0x12f7) need verification with actual hardware

## Session Continuity

Last session: 2026-02-17
Stopped at: Plan 09-06 complete - IPC protocol for device capability queries
Resume file: None

**Next step:** Execute plan 09-07 - Joystick axis remapping
