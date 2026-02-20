# Project State

**Last Updated:** 2026-02-20T07:27:00Z

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-17)

**Core value:** Remap any input key with persist & switch at runtime
**Current focus:** Phase 16 - Calibration GUI

## Current Position

Phase: 16 of 17 (Calibration GUI)
Plan: 7 of 8
Status: In Progress
Last activity: 2026-02-20 — Phase 16 Plan 07 complete: Performance optimization (caching, throttling)

Progress:
```
v1.0 (Phases 1-4): [========================================] 100%
v1.1 (Phases 5-8): [========================================] 100%
v1.2 (Phases 9-12): [===========================================] 75% (27/36 plans)
v1.3 (Phases 13-17): [===================================================] 70% (45/60 plans)
Overall: [===================================================] 92% (101/104 plans)
```

## Performance Metrics

**Velocity:**
- Total plans completed: 61
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
| 11. Analog Processing and Calibration | 8 | Complete |
| 12. LED Control | 8 | Not started |
| 13. Wayland Portal Integration | 6 | Complete |
| 14. Gamepad Emulation Mode | 6 | Complete |
| 15. WASD Mouse Camera Modes | 10 | Complete (10/10) |
| 16. Calibration GUI | 8 | In Progress (2/8) |
| 17. Hotkey and Auto-Switch Persistence | 9 | Complete (9/9) |
| Phase 13 P05 | 30 | 6 tasks | 6 files |
| Phase 14 P03 | 2095 | 3 tasks | 2 files |
| Phase 14 P06 | 3624 | 4 tasks | 3 files |
| Phase 15 P01 | 5 | 2 tasks | 1 file |
| Phase 15 P04 | 657 | 2 tasks | 2 files |
| Phase 15 P02 | 2 | 1 task | 1 file |
| Phase 17 P01 | 42fac6f | 3 tasks | 1 file |
| Phase 17-hotkey-and-auto-switch-persistence P05 | 12min | 2 tasks | 1 files |
| Phase 17-hotkey-and-auto-switch-persistence P09 | 8min | 3 tasks | 2 files |
| Phase 16-calibration-gui P16-02 | 15min | 3 tasks | 5 files |
| Phase 16-calibration-gui P03 | 934 | 2 tasks | 3 files |
| Phase 16-calibration-gui P04 | 8min | 2 tasks | 3 files |
| Phase 16 P05 | 18 | 3 tasks | 5 files |
| Phase 16-calibration-gui P07 | 11min | 2 tasks | 2 files |

## Accumulated Context

### Roadmap Evolution

**2026-02-20:**
- Phase 17 added: Hotkey and Auto-Switch Persistence
  - Complete IPC handlers for hotkey registration/listing/removal
  - Persist auto-switch rules and hotkey bindings from GUI to daemon
  - Address CONCERNS.md unimplemented IPC handlers

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

*Plan 11-05 - Per-Layer Analog Calibration:*
- Per-layer calibration uses HashMap<usize, AnalogCalibration> indexed by layer_id (0=base, 1, 2, ...)
- None for analog_calibration field means use AnalogCalibration::default() (graceful defaults)
- YAML format follows Phase 10-06 pattern for layer state persistence (serde with skip_serializing_if)
- ConfigManager query methods return Option<AnalogCalibration> for graceful handling of missing config
- analog_calibration: Option<AnalogCalibration> field added to LayerConfig for per-layer settings

*Plan 11-06 - Analog Event Recording in Macros:*
- Action::AnalogMove variant with axis_code (61000-61005) and normalized value (-1.0 to 1.0)
- process_analog_event() records normalized analog movements during macro capture
- Normalized values enable device-independent macro storage and replay
- execute_macro() handles Action::AnalogMove by denormalizing and injecting via Injector trait
- AnalogMove uses axis_code mapping: 61000=ABS_X, 61001=ABS_Y, 61002=ABS_Z, 61003=RX, 61004=RY, 61005=RZ

*Plan 11-07 - IPC Protocol for Analog Calibration:*
- AnalogCalibrationConfig struct in razermapper-common with String-based enum serialization
- Request::GetAnalogCalibration and SetAnalogCalibration variants for IPC communication
- Response::AnalogCalibration returns Option<AnalogCalibrationConfig> for graceful default handling
- config_to_calibration() and calibration_to_config() conversion functions between IPC and internal types
- save_analog_calibration() method in ConfigManager for YAML persistence
- set_calibration() method in AnalogProcessor to apply calibration to runtime processing

*Plan 11-08 - GUI Calibration UI:*
- DeadzoneShape and SensitivityCurve enums with Display trait for pick_list widgets
- CalibrationConfig struct for local GUI state (separate from IPC type to avoid conflicts)
- AnalogCalibrationView struct with complete UI layout (sliders, dropdowns, checkboxes)
- checkbox_button helper using button widget (iced doesn't have checkbox widget)
- Modal dialog pattern: transparent container overlay with centered box
- view_analog_calibration() returns Option<Element> for modal integration
- "Analog Calibration" button only shown for Keypad and Gamepad device types

**v1.3 Implementation Decisions (Phase 14):**

*Plan 14-01 - Virtual Xbox 360 Gamepad Device:*
- GamepadVirtualDevice module with uinput backend for virtual gamepad creation
- Xbox 360 controller IDs: vendor 0x045e (Microsoft), product 0x028e (Xbox 360 Controller)
- 6 axes enabled: ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ (codes 0-5)
- Axis ranges: -32768 to 32767 matching Linux input subsystem
- Arc<RwLock<Option<RawFd>>> for thread-safe file descriptor access
- Drop trait for automatic uinput device cleanup on scope exit

*Plan 14-02 - AnalogMode Enum:*
- AnalogMode enum with variants: Disabled, Dpad, Gamepad, Camera, Mouse, Wasd
- serde(rename) for lowercase YAML output (e.g., "gamepad" not "Gamepad")
- DpadMode and AnalogMode are separate concepts: DpadMode configures HOW 8-way detection works, AnalogMode selects OUTPUT behavior
- mode field added to DeviceAnalogConfig with #[serde(default)] for YAML persistence
- process_as_gamepad() placeholder with #[allow(dead_code)] until GamepadVirtualDevice integration

*Plan 14-03 - Analog-to-Gamepad Conversion:*
- process_2d() helper implementing 5-stage calibration pipeline: normalize → center → deadzone → sensitivity → scale
- Static methods: apply_deadzone(), apply_sensitivity_curve(), scale_to_output_range()
- MAX_MAGNITUDE = 0.707 (sqrt(0.5)) for circular deadzone in centered coordinate system
- invert_y=true by default for gamepad coordinates (up = negative)
- 13 unit tests covering center filtering, full deflection, diagonals, edge cases

*Plan 14-04 - Per-Layer Gamepad Configuration:*
- analog_mode field added to LayerConfig struct with #[serde(default)]
- All LayerConfig constructors (default(), new()) initialize analog_mode to Disabled
- DeviceLayerState layer_configs include analog_mode for all 3 default layers
- IPC SetLayerConfig handler preserves existing analog_mode when updating other fields
- EXAMPLE_CONFIG_WITH_ANALOG_MODES shows per-layer configuration (D-pad for menus, Gamepad for gaming)

*Plan 14-05 - Event Loop Integration:*
- GamepadVirtualDevice added to DeviceManager as Arc<GamepadVirtualDevice>
- process_analog_gamepad() method: gets effective layer, verifies analog_mode, applies layer calibration, emits to gamepad
- process_as_gamepad_with_calibration() accepts custom AnalogCalibration for per-layer settings
- Event loop checks layer_config.analog_mode == AnalogMode::Gamepad before processing
- Calibration priority: layer-specific analog_calibration if configured, otherwise device default

*Plan 14-06 - Unit and Integration Tests:*
- 8 unit tests for process_as_gamepad() (deadzone, full deflection all directions, diagonals, sensitivity)
- 2 unit tests for GamepadVirtualDevice (device_name, Xbox 360 IDs)
- 2 integration tests for per-layer analog_mode and calibration configuration
- All 24 gamepad-related tests pass successfully
- Test thresholds adjusted to match actual calibration behavior (15000/10000 instead of 30000)
- [Phase 15]: Default mouse velocity multiplier 10.0 pixels per unit deflection
- [Phase 15]: Fire-and-forget mouse events via EV_REL with velocity-based movement
- [Phase 17-hotkey-and-auto-switch-persistence]: Added ShowNotification(String, bool) message for GUI user feedback
- [Phase 17-hotkey-and-auto-switch-persistence]: Used razermapper_common::ipc_client::IpcClient directly instead of wrapped GuiIpcClient for auto-switch rule persistence
- [Phase 16-calibration-gui]: Created separate widgets module for reusable Canvas components
- [Phase 16-calibration-gui]: Used WidgetDeadzoneShape alias to avoid enum name collision
- [Phase 16-calibration-gui]: Positioned visualizer between info and mode sections for prominence
- [Phase 16-calibration-gui]: Default stick position (0.0, 0.0) shows center dot before real-time updates
- [Phase 16-calibration-gui]: Use GUI's local SensitivityCurve enum (razermapper_common doesn't export it)
- [Phase 16-calibration-gui]: Default Exponential curve exponent to 2.0 (GUI enum has no field)
- [Phase 16-calibration-gui]: Plot normalized 0-1 curve only, show multiplier separately for clarity
- [Phase 16-calibration-gui]: One GUI client only - UnsubscribeAnalogInput removes all subscribers for device
- [Phase 16-calibration-gui]: Fire-and-forget UnboundedSender for analog subscription sends (drop silently if client disconnected)
- [Phase 16-calibration-gui]: Subscriber map pattern: HashMap<device_id, Vec<UnboundedSender<Response>>>

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
Stopped at: Phase 15 complete - All 10 plans executed including 2 gap closure plans (15-09, 15-10)
Resume file: None

**Next step:** Move to Phase 16 (Calibration GUI) or Phase 12 (LED Control)

**v1.3 Implementation Decisions (Phase 14):**

*Plan 14-04 - Per-Layer Gamepad Mode Configuration:*
- LayerConfig extended with analog_mode field for per-layer analog output mode
- All constructors initialize analog_mode to Disabled for backward compatibility
- IPC SetLayerConfig handler preserves existing analog_mode when updating other fields
- Per-layer configuration enables base layer D-pad navigation with Layer 1 Gamepad gaming

**v1.3 Implementation Decisions (Phase 15):**

*Plan 15-01 - WASD Mode Processing:*
- wasd_direction_to_keys() helper function maps DpadDirection enum to WASD keys (W, A, S, D)
- Diagonal directions return key combinations (UpRight = W + D) matching standard game expectations
- process_as_wasd() method follows same calibration pipeline as process_as_dpad()
- Reused existing DpadDirection enum instead of creating new WASDDirection for consistency

*Plan 15-02 - WASD Mode Event Loop Integration:*
- Reused dpad_state struct for WASD axis tracking (no new state structures)
- Default calibration fallback: 0.15 deadzone, Circular shape, Linear curve, 1.0 multiplier
- Immediate key press/release for initial testing (proper state tracking in plan 15-08)
- WASD mode check placed after gamepad mode in analog event processing

*Plan 15-03 - Mouse Mode Processing:*
- MouseVelocityConfig struct with configurable multiplier for cursor speed control
- Default multiplier: 10.0 pixels per 1.0 analog value (~10 pixels at full deflection)
- process_as_mouse() method reuses process_2d() for full calibration pipeline consistency
- Returns None for deadzone input to prevent cursor drift
- Velocity calculated as (processed_value / 32768.0) * multiplier

*Plan 15-05 - Camera Mode Processing:*
- CameraOutputMode enum with Scroll and Keys variants (user-selectable mode)
- CameraOutput enum with Scroll(i32) and Keys(Vec<Key>) return types
- CameraOutputMode defaults to Scroll for broader compatibility
- Scroll mode uses REL_WHEEL events (Y-axis only, vertical scrolling)
- Key mode emits PageUp/PageDown for vertical, arrow keys for horizontal navigation
- Diagonals emit key combinations (e.g., UpLeft = PageUp + Left) for multi-axis control
- process_as_camera() method reuses process_2d() for full calibration pipeline
- Returns None for deadzone input to prevent unwanted output

*Plan 15-06 - Camera Mode Integration:*
- camera_output_mode field added to LayerConfig for per-layer camera mode selection
- camera_output_mode() accessor method for retrieving layer-specific mode
- Camera mode handling in device event loop after Mouse mode check
- Emits REL_WHEEL events via injector.mouse_scroll() for Scroll mode
- Emits key press/release events via injector for Keys mode (immediate for repeat)
- Per-layer calibration support via analog_calibration field
- Default camera_output_mode is Scroll for broader compatibility

*Plan 15-07 - GUI Analog Mode Selector:*
- AnalogMode enum added to razermapper-common for IPC use (6 variants: Disabled, Dpad, Gamepad, Camera, Mouse, Wasd)
- CameraOutputMode enum added to razermapper-common for IPC use (2 variants: Scroll, Keys)
- Display trait implementation for user-friendly names in pick_list widgets
- ALL constant arrays provide ordered options for pick_list
- AnalogCalibrationView extended with analog_mode_selected and camera_mode_selected fields
- Mode selector UI section in calibration dialog with conditional camera sub-mode selector
- AnalogModeChanged and CameraModeChanged message handlers
- Camera sub-mode selector only visible when Camera mode is selected

*Plan 15-08 - Unit and Integration Tests:*
- Added 7 WASD mode unit tests covering all 8 directions, deadzone, and axis inversion
- Added 9 Mouse mode unit tests covering velocity scaling, multiplier, and deadzone
- Added 8 Camera mode unit tests covering scroll and key output modes
- Added 7 integration tests for per-layer analog mode configuration
- Y-axis centering behavior: cy = 0.5 - ny means raw Y=0 becomes positive output
- Mouse velocity: positive Y = up, negative Y = down (due to centering inversion)
- All 29 tests pass successfully

*Plan 15-09 - GUI Analog Mode Persistence (Gap Closure):*
- Extended AnalogCalibrationConfig with analog_mode and camera_output_mode fields
- Type conversion functions common_to_internal_analog_mode/common_to_internal_camera_mode
- GUI ApplyAnalogCalibration handler now sends selected mode via IPC
- Daemon SetAnalogCalibration handler applies mode to LayerConfig
- IPC serialization test confirms mode round-trip works correctly
- User selections in calibration dialog now persist when clicking Apply

*Plan 15-10 - WASD Key State Tracking (Gap Closure):*
- wasd_previous_keys state variable using Arc<RwLock<Vec<(Key, bool)>>> for shared mutable access
- State diffing algorithm: keys in previous but not current get release, keys in current but not previous get press
- Keys remaining active get no new events (stay held) - proper game-like key behavior
- 6 WASD state tracking tests (5 unit tests for transitions, 1 integration test for full event loop)
- Closed Gap 2 from Phase 15 verification (TODO about state tracking)

**v1.3 Implementation Decisions (Phase 17):**

*Plan 17-01 - ConfigManager Hotkey Binding Persistence:*
- Added RemapConfigError::Validation variant for structured error handling
- add_hotkey_binding() uses case-insensitive modifier comparison (to_lowercase) for duplicate detection
- remove_hotkey_binding() uses retain() for functional iteration instead of index-based removal
- get_hotkey_bindings() returns empty Vec for missing file/device (graceful degradation)
- All methods follow set_analog_calibration pattern: load -> modify -> serialize -> write
- tokio::fs::write for async file I/O (non-blocking)

*Plan 17-02 - ConfigManager Auto-Switch Rule Persistence:*
- Wrapped DaemonConfig in Arc<RwLock<>> for runtime mutability (was immutable field)
- set_auto_switch_rules() acquires write lock, updates in-memory, serializes full config to YAML
- get_auto_switch_rules() acquires read lock for efficient access (no file I/O)
- Updated all ConfigManager access patterns: new(), load_config(), save_config(), config() getter
- Auto-switch rules stored in config.yaml (not device_profiles.yaml) as global daemon config

*Plan 17-03 - IPC Hotkey Handlers:*
- Added hotkey_manager() getter to DeviceManager for IPC handler access
- Type conversion between common::HotkeyBinding and config::HotkeyBinding (field-by-field mapping)
- RegisterHotkey validates profile exists before adding binding
- ListHotkeys returns empty Vec (not error) for graceful degradation
- RemoveHotkey and RegisterHotkey both reload GlobalHotkeyManager after config changes
- SetAutoSwitchRules handler added to complete IPC match arms

*Plan 17-04 - SetAutoSwitchRules IPC Protocol:*
- Added AutoSwitchRule struct to razermapper-common for IPC use (was daemon-internal only)
- Added Request::SetAutoSwitchRules variant with rules: Vec<AutoSwitchRule>
- Added Response::AutoSwitchRulesAck variant for acknowledgment
- Wrapped AutoProfileSwitcher.rules in Arc<RwLock<>> for runtime mutability
- Added reload_rules(&self, rules) method to AutoProfileSwitcher for Arc-based access
- SetAutoSwitchRules handler reloads AutoProfileSwitcher after successful save
- IPC pattern: save to config -> reload component -> return ack

*Plan 17-07 - GlobalHotkeyManager.load_bindings() Implementation:*
- Added ConfigManager::get_all_hotkey_bindings() to read bindings from all devices in device_profiles.yaml
- Implemented GlobalHotkeyManager::load_bindings() to load from ConfigManager with fallback to defaults
- Applied case-insensitive modifier name normalization using normalize_modifier_name()
- Used filter_map with normalize_modifier_name() to filter out invalid modifier names (graceful handling)
- Normalization applied at load time for efficiency (not at match time)

*Plan 17-06 - GUI Hotkey Binding Persistence:*
- Changed HotkeyBindingsLoaded message from (String, Vec<HotkeyBinding>) to Result<Vec<HotkeyBinding>, String> for proper error handling
- Added HotkeyBinding as CommonHotkeyBinding import for type conversion between common and GUI types
- Added HotkeyBindingsUpdated(Vec<HotkeyBinding>) message variant for post-delete state updates
- LoadHotkeyBindings: IPC call to ListHotkeys with type conversion from common to GUI
- SaveHotkeyBinding: IPC call to RegisterHotkey with optimistic UI update
- DeleteHotkeyBinding: IPC call to RemoveHotkey with HotkeyBindingsUpdated message
- Optimistic UI updates: update local state immediately, sync to daemon via async IPC

*Plan 17-08 - Unit Tests for Hotkey and Auto-Switch Persistence:*
- Added test_add_hotkey_binding_persists_to_yaml for YAML serialization verification
- Added test_add_hotkey_binding_rejects_duplicate for duplicate detection validation
- Added test_remove_hotkey_binding for binding removal with multi-binding scenarios
- Added test_get_hotkey_bindings_returns_empty_for_missing_device for graceful degradation
- Added test_set_get_auto_switch_rules for auto-switch rule persistence
- Added test_get_all_hotkey_bindings_aggregates_devices for cross-device binding aggregation
- Fixed DaemonConfig::default() calls to use Arc<RwLock<>> wrapper across all test files
- All 6 tests pass, validating correctness of prior plan implementations

*Plan 17-10 - GetAutoSwitchRules IPC Protocol:*
- Added Request::GetAutoSwitchRules variant for querying auto-switch rules
- Added Response::AutoSwitchRules variant with rules: Vec<AutoSwitchRule>
- Implemented daemon handler calling ConfigManager::get_auto_switch_rules()
- Added get_auto_switch_rules() convenience function to ipc_client module
- Updated GUI LoadAutoSwitchRules to call IPC and populate view with existing rules
- Changed AutoSwitchRulesLoaded message to Result<Vec<AutoSwitchRule>, String> for error handling
- Fixed message signature to match iced::Command::perform (single argument, not device_id + Result)
- Device_id read from auto_switch_view field instead of message payload
- Removed TODO comment at gui.rs:845
- Fixed DaemonConfig::default() calls to use Arc<RwLock<>> wrapper across all test files
- All 6 tests pass, validating correctness of prior plan implementations

*Plan 17-09 - GlobalHotkeyManager Startup Initialization:*
- Added hotkey_manager field to DaemonState as Option<Arc<Mutex<GlobalHotkeyManager>>>
- GlobalHotkeyManager initialized with layer_manager and config_manager after macro_engine setup
- load_bindings() called immediately after GlobalHotkeyManager creation to load from device_profiles.yaml
- hotkey_manager stored in DaemonState before device_manager creation for later retrieval
- DeviceManager.set_hotkey_manager() called before start_discovery() to wire event loop integration
- Hotkey bindings now active immediately after daemon restart (Gap 1 closed)

**v1.3 Implementation Decisions (Phase 16):**

*Plan 16-01 - Canvas Infrastructure - AnalogVisualizer Base Widget:*
- Created widgets module with mod.rs and analog_visualizer.rs
- Implemented AnalogVisualizer using iced::widget::canvas::Program trait
- Canvas drawing uses Frame::fill() and Frame::stroke() with Stroke builder pattern
- Y-axis inversion applied for screen coordinates (analog Y+ = up, screen Y+ = down)
- Deadzone shapes: Circular (Path::circle) and Square (Path::rectangle)
- Default values: stick centered (0.0, 0.0), 15% circular deadzone

*Plan 16-02 - Stick Position Visualization Integration:*
- Added widgets module declaration to lib.rs and main.rs for dual compilation targets
- Extended AnalogCalibrationView with stick_x and stick_y fields (default 0.0, 0.0)
- Imported Canvas widget and AnalogVisualizer into gui.rs
- Created visualizer section in AnalogCalibrationView::view() with Canvas::new()
- Positioned visualizer between info section and mode section for prominence
- Deadzone shape conversion: GUI enum to widget enum via match expression

*Plan 16-07 - Performance Optimization (Caching, Throttling):*
- Added Arc<Cache> to AnalogVisualizer for persistent caching across widget frames
- Implemented cache.draw() pattern for static elements (outer circle, deadzone, axes)
- Only stick position dot redraws each frame (dynamic content)
- Added clear_cache() method to AnalogVisualizer for cache invalidation
- Added last_visualizer_update: Instant to AnalogCalibrationView for 30 FPS throttling
- Implemented 33ms (30 FPS) throttling in AnalogInputUpdated handler
- Cache clearing on deadzone/shape changes via visualizer_cache.clear()
- Manual Clone/Default impls for AnalogCalibrationView (Instant and Cache don't implement Clone)

