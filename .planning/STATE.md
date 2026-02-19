# Project State

**Last Updated:** 2026-02-19

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-17)

**Core value:** Remap any input key with persist & switch at runtime
**Current focus:** Phase 11 - Analog Processing and Calibration

## Current Position

Phase: 11 of 16 (Analog Processing and Calibration)
Plan: 3 of 8
Status: In Progress
Last activity: 2026-02-19 — Plan 11-03 complete: D-pad mode for analog stick with 8-way direction detection

Progress:
```
v1.0 (Phases 1-4): [========================================] 100%
v1.1 (Phases 5-8): [========================================] 100%
v1.2 (Phases 9-12): [=======================.................] 56% (20/36 plans)
v1.3 (Phases 13-16): [...........................................] 0% (0/40 plans)
Overall: [==========================================....] 68% (51/74 plans)
```

## Performance Metrics

**Velocity:**
- Total plans completed: 52
- Average duration: ~5-8 minutes per plan
- Total execution time: ~6.5 hours

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
| 11. Analog Processing and Calibration | 8 | 4 complete, 4 pending |
| 12. LED Control | 8 | Not started |
| 13. Wayland Portal Integration | 6 | Complete |
| 14. Gamepad Emulation Mode | 8 | Not started |
| 15. WASD Mouse Camera Modes | 8 | Not started |
| 16. Calibration GUI | 8 | Not started |
| Phase 13 P05 | 30 | 6 tasks | 6 files |

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

*Plan 13-02 - FocusTracker Component:*
- FocusTracker uses graceful fallback - returns no-op tracker when portal unavailable
- Pattern matching supports suffix (.firefox) and prefix (org.mozilla.) for flexible app_id matching
- Focus event monitoring implemented as async task with stop() flag for clean shutdown
- Portal availability check via WAYLAND_DISPLAY env var before attempting connection
- Async task spawning with Arc<AtomicBool> running flag for cancellation pattern
- Optional<T> portal field with is_available() query method

*Plan 13-03 - IPC Protocol for Focus Events:*
- FocusChanged IPC uses Option<String> for window_title to handle compositors that don't provide titles
- AutoProfileSwitcher integration deferred to 13-04 with explicit TODO comment
- send_focus_change returns Result<(), String> for GUI error handling
- Request/Response variant pairs for IPC protocol extension pattern
- Placeholder handler with TODO for cross-plan integration pattern

*Plan 13-04 - AutoProfileSwitcher:*
- Wildcard "*" app_id matches any application for default profile fallback
- Rules evaluated in order with first-match-wins semantics
- Manual overrides cleared on every focus change to re-enable auto-switching
- Prefix matching (org.mozilla.) and suffix matching (.firefox) for flexible app_id patterns
- Optional auto_profile_switcher parameter allows graceful degradation when unavailable
- Manual override state is per-device, allowing selective auto-switching control

*Plan 13-05 - Global Hotkeys:*
- Hotkey detection happens before remap processing in event pipeline
- Modifier keys tracked as HashSet<u16> for LEFT/RIGHT variants separately
- Matched hotkey events consumed (not forwarded to applications)
- Default Ctrl+Alt+Shift+1-9 bindings for profile slots 1-9
- Wayland-compatible - no compositor support required
- Hotkey combo detection uses case-insensitive modifier name matching

*Plan 13-06 - GUI Views for Auto-Switch and Hotkeys:*
- AutoSwitchRulesView struct with rule management UI (add/edit/delete)
- HotkeyBindingsView struct with binding management UI and modifier checkboxes
- FocusTracker integration with GUI main loop (auto-trigger on daemon connect)
- Navigation buttons added to all device cards for easy access
- "Use Current App" button fills app_id from detected focus
- YAML config examples with comprehensive documentation in config.rs
- View state pattern: Option<View> for modal-like screen management
- Module visibility workaround: WAYLAND_DISPLAY check instead of ashpd in async closure

**v1.2 Implementation Decisions (Phase 11):**

*Plan 11-01 - Analog Calibration Data Types:*
- AnalogCalibration struct with deadzone (0.0-1.0), sensitivity_multiplier (0.1-5.0), range_min/max, and invert_x/y
- DeadzoneShape enum (Circular, Square) for deadzone behavior - Circular default for smoother diagonals
- SensitivityCurve enum (Linear, Quadratic, Exponential { exponent }) for response curves
- YAML serialization with serde using snake_case renaming for enum variants
- Default values: 0.15 deadzone (15%), Linear curve, -32768/32767 range (Linux input limits)
- Validation method for bounds checking with Result<(), String> return type
- 19 unit tests covering defaults, serialization, validation, and roundtrip

*Plan 11-02 - Analog Processing Pipeline:*
- MAX_MAGNITUDE = 0.707 (sqrt(0.5)) for circular deadzone scaling to account for centered coordinate bounds
- MAX_AXIS = 0.5 for square deadzone (centered coordinates range from -0.5 to 0.5)
- Process pipeline: normalize (0-255 to 0-1) → center (-0.5 to 0.5) → deadzone → sensitivity → scale (to output range)
- Polar coordinate transformation for sensitivity curves to maintain direction while scaling magnitude
- Deadzone scaling uses linear interpolation to prevent "jump" at boundary
- 12 unit tests covering all pipeline stages, deadzone shapes, and sensitivity curves

*Plan 11-03 - D-pad Mode:*
- DpadDirection enum with 8 directions + None for complete directional coverage
- detect_dpad_direction() using 22.5-degree sectors for 8-way direction detection
- dpad_direction_to_keys() helper mapping directions to evdev::Key codes (KEY_UP=103, KEY_DOWN=108, KEY_LEFT=105, KEY_RIGHT=106)
- process_as_dpad() method converting analog stick position to Vec<(Key, bool)> for current frame state
- Axis inversion applied directly in process_as_dpad() before direction detection (invert_x/invert_y support)
- Y-axis coordinate system: center() negates Y (cy=positive means visual up), accounted for in angle calculation
- 20 unit tests covering all 8 directions, deadzone filtering, direction threshold, and axis inversion

*Plan 11-04 - AnalogProcessor Integration:*
- Centralized AnalogProcessor in DeviceManager with internal per-device config HashMap
- Per-device configuration identified by device_id (vendor:product format)
- Event loop receives AnalogProcessor reference and handles D-pad mode processing
- IPC handlers for runtime calibration: deadzone, sensitivity, response curve, D-pad mode
- Profile integration: load_config() and save_config() for configuration persistence
- Async RwLock for thread-safe configuration access

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
Stopped at: Plan 11-03 complete - D-pad mode for analog stick with 8-way direction detection
Resume file: None

**Next step:** Execute plan 11-04 - AnalogProcessor integration into device event loop
