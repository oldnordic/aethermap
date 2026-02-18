# Project State

**Last Updated:** 2026-02-19

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-17)

**Core value:** Remap any input key with persist & switch at runtime
**Current focus:** Phase 13 - Wayland Portal Integration

## Current Position

Phase: 13 of 16 (Wayland Portal Integration and Global Hotkeys)
Plan: 1 of 6
Status: In progress
Last activity: 2026-02-19 — Plan 13-01 complete: Ashpd dependency for Wayland portal integration

Progress:
```
v1.0 (Phases 1-4): [========================================] 100%
v1.1 (Phases 5-8): [========================================] 100%
v1.2 (Phases 9-12): [========================================] 100%
v1.3 (Phases 13-16): [#.......................................] 2% (1/58 plans)
Overall: [========================================...] 72% (53/74 plans)
```

## Performance Metrics

**Velocity:**
- Total plans completed: 53
- Average duration: ~5-8 minutes per plan
- Total execution time: ~6 hours

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
| 9. Device Detection and Basic Input | 7 | Complete |
| 10. Layer System and State Management | 8 | Complete |
| 11. Analog Processing and Calibration | 8 | Complete |
| 12. LED Control | 8 | Complete |
| 13. Wayland Portal Integration | 6 | 1 complete (13-01) |
| 14. Gamepad Emulation Mode | 8 | Not started |
| 15. WASD Mouse Camera Modes | 8 | Not started |
| 16. Calibration GUI | 8 | Not started |

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

**v1.2 Implementation Decisions (Phase 10):**

*Plan 10-01 - LayerManager Component:*
- LayerMode enum with Hold (modifier-held) and Toggle (press-on-press-off) variants
- LayerConfig uses custom serde serialization for HashMap<Key, Key> (serializes as Vec<(u16, u16)>)
- get_effective_layer() returns highest active layer ID using max(active_layers) for simple priority
- active_toggle_layers tracked separately from active_layers for state clarity
- Minimum 3 layers guaranteed per device (0=base, 1, 2) with extensible configuration

*Plan 10-02 - Hold Mode Activation:*
- active_hold_layers HashSet tracks currently held layers (modifier-key-held activation)
- activate_hold_layer() and deactivate_hold_layer() methods on DeviceLayerState for hold lifecycle
- LayerManager async wrapper methods with Result<(), String> error handling
- get_effective_layer() combines active_hold_layers and active_toggle_layers via union for priority
- activate_layer() updated to track hold mode layers in active_hold_layers based on LayerMode

*Plan 10-03 - Toggle Mode Activation:*
- toggle_layer() uses active_toggle_layers.contains() for state checking (not active_layers)
- is_toggle_layer_active() provides explicit toggle state query method
- LayerManager.toggle_layer() returns Result<bool, String> for error handling
- LayerManager.toggle_layer() validates layer_id against configured layers
- Toggle layers persist until explicitly toggled off (unlike hold layers)

*Plan 10-04 - Layer Stack Composition:*
- get_effective_layer() uses .chain() instead of .union().collect() to avoid intermediate HashSet allocation
- get_active_layers(device_id) returns sorted Vec of all active layer IDs for GUI display
- Highest layer ID wins when multiple layers active (simple priority ordering)
- Comprehensive unit tests for layer stack composition (8 new tests)

*Plan 10-05 - RemapEngine Layer-Aware Lookups:*
- layer_remaps: Vec<Arc<RwLock<HashMap<Key, Key>>>> for O(1) indexed layer access
- Cascade fallback searches from effective_layer down to base (0) using (0..=effective_layer).rev()
- remap_layer_aware() and process_event_layer_aware() for device-specific layer-aware lookups
- load_layer_remap() for per-layer config loading with eager validation
- Backward compatible remaps field maps to layer_remaps[0] for existing code
- 11 unit tests verifying layer creation, loading, cascade, priority, and isolation

*Plan 10-06 - Layer State Persistence:*
- LayerStateSnapshot struct with device_id, base_layer, active_toggle_layers
- Hold layers NOT persisted (physical key press state resets on device reconnect)
- Toggle layers persisted across daemon restarts (user mode preferences)
- save_to_path/load_from_path methods using YAML serialization (serde_yaml for consistency)
- ConfigManager has layer_state_path field for file location
- load_from_path returns Ok(()) when file doesn't exist (graceful first startup)

**v1.3 Implementation Decisions (Phase 13):**

*Plan 13-01 - Ashpd Dependency:*
- ashpd 0.8 added to razermapper-gui with wayland and tokio features
- default-features = false to minimize dependency bloat
- tokio feature required for async portal operations compatibility
- GUI runs as normal user for portal access (daemon runs as root)

### Pending Todos

None.

### Blockers/Concerns

**Phase 12 Research Flag:**
- Azeron HID LED protocol requires USB packet capture during planning
- Consider using official software for LED setup initially

**Phase 9 Verification:**
- Azeron VID/PID values (0x32b6:0x12f7) need verification with actual hardware

## Session Continuity

Last session: 2026-02-19
Stopped at: Plan 13-01 complete - Ashpd dependency for Wayland portal integration
Resume file: None

**Next step:** Execute plan 13-02 - FocusTracker component for window focus detection
