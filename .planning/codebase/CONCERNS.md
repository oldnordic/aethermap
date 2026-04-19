# Technical Concerns & Debt

**Analysis Date:** 2026-02-20

## Known Technical Debt

### LED Animation Architecture
**Issue:** Software LED animations (`breathing`, `rainbow`) cannot write HID from spawned tokio tasks
- Files: `aethermap/aethermapd/src/led_controller.rs:887`, `aethermap/aethermapd/src/led_controller.rs:918`
- Impact: Animation tasks calculate RGB values but never apply them; animations are non-functional
- Fix approach: Requires channel-based architecture to move HID writes out of spawned tasks
- Status: Documented in TODO comments, blocking Phase 12 completion

### Global Hotkey Configuration Loading
**Issue:** Hotkey bindings are hardcoded defaults instead of loading from device configurations
- Files: `aethermap/aethermapd/src/global_hotkey_manager.rs:129`
- Impact: User-configured hotkey bindings are not loaded; only Ctrl+Alt+Shift+1-9 work
- Fix approach: ConfigManager needs to expose `ExtendedDeviceRemapConfig.hotkey_bindings` field
- Status: TODO comment marks integration point

### Per-Layer Analog Calibration Storage
**Issue:** Layer-specific analog calibrations are not stored despite the infrastructure being in place
- Files: `aethermap/aethermapd/src/analog_processor.rs:847`, `aethermap/aethermapd/src/analog_processor.rs:858`
- Impact: All layers share the base layer calibration; per-layer sensitivity/curves don't persist
- Fix approach: Extend `LayerConfig` to store `Option<AnalogCalibration>` and update save/load paths
- Status: TODO comments note "for future per-layer support"

## Unimplemented IPC Handlers

### Hotkey Management (3 handlers)
**Issue:** IPC handlers exist but return stub responses without implementation
- Files: `aethermap/aethermapd/src/ipc.rs:1816`, `aethermap/aethermapd/src/ipc.rs:1832`, `aethermap/aethermapd/src/ipc.rs:1844`
- Handlers affected: `RegisterHotkey`, `ListHotkeys`, `RemoveHotkey`
- Impact: GUI cannot register, list, or remove custom hotkey bindings via IPC
- Fix approach: Integrate with `GlobalHotkeyManager.load_bindings()` and persist to YAML

### GUI Auto-Switch Rules Persistence
**Issue:** Auto-switch rules and hotkey rules in GUI have TODO comments for daemon persistence
- Files: `aethermap/aethermap-gui/src/gui.rs:918`, `aethermap/aethermap-gui/src/gui.rs:929`, `aethermap/aethermap-gui/src/gui.rs:1050`, `aethermap/aethermap-gui/src/gui.rs:1061`
- Impact: Rules configured in GUI are not saved to daemon configuration
- Fix approach: Send to daemon via IPC (add `SetAutoSwitchRules` and `SetHotkeyBindings` requests)

### LED Color/Zone GUI Configuration
**Issue:** LED color and zone are hardcoded defaults in layer config dialog
- Files: `aethermap/aethermap-gui/src/gui.rs:1826`, `aethermap/aethermap-gui/src/gui.rs:1827`, `aethermap/aethermap-gui/src/ipc.rs:435`, `aethermap/aethermap-gui/src/ipc.rs:436`
- Impact: Users cannot configure per-layer LED colors through the GUI
- Fix approach: Add color picker widget and zone selector to layer config dialog

## Phase 12 LED Control Research Blockers

### Azeron HID LED Protocol Unknown
**Issue:** Full RGB LED control protocol requires USB packet capture during official software operation
- Files: `aethermap/aethermapd/src/led_controller.rs:808`, `aethermap/.planning/research/AZERON_STACK.md:302`
- What's unknown: RGB color control commands, pattern commands (breathing, rainbow, wave)
- What's known: Brightness control protocol (0x0100-0x0196 range), keepalive packets
- Hardware limitation: Official Azeron software only exposes brightness control for side LED
- Fix approach: USB packet capture with Wireshark or usbmon during official software use
- Risk: Protocol may not be exposed by firmware; may be limited to brightness-only

## File Size Concerns

### Large Source Files (Potential Complexity)
Files exceeding 3000 lines may benefit from modularization:
- `aethermap/aethermap-gui/src/gui.rs` - 4489 lines (GUI state, views, message handling)
- `aethermap/aethermapd/src/analog_processor.rs` - 3969 lines (all analog modes, calibration, tests)
- `aethermap/aethermapd/src/config.rs` - 3093 lines (YAML structures, manager, tests)

**Impact:** Larger files are harder to navigate, modify, and test. No immediate action required unless modification frequency increases.

## Security Considerations

### Root Privilege Requirement
**Files:** `aethermap/aethermapd/src/main.rs:35`, `aethermap/aethermapd/src/security.rs`
- Daemon must run as root for `/dev/input` device access and `/dev/uinput` virtual device creation
- Privilege dropping implemented: Keeps only `CAP_SYS_RAWIO` after initialization
- Socket permissions: Set to 0660 with group "input" ownership
- Risk: Compromised daemon has input injection capabilities
- Mitigation: Token-based authentication available via `token-auth` feature

### Unsafe Code Blocks
**Files:** Multiple uses of `unsafe` for `libc` syscalls and `uinput` operations
- `aethermap/aethermapd/src/injector.rs` - uinput setup, event writing
- `aethermap/aethermapd/src/gamepad_device.rs` - gamepad device creation
- `aethermap/aethermapd/src/security.rs` - prctl, chown, setgroups
- `aethermap/aethermapd/src/device.rs` - EVIOCGRAB ioctl

**Assessment:** All unsafe blocks are for FFI with Linux kernel APIs. Well-contained and documented.

## Unresolved unwrap() Calls

### Test Code
Most `unwrap()` calls are in test code where failure is acceptable:
- `aethermap/aethermapd/tests/` - All test files use unwrap for test setup
- `aethermap/aethermapd/src/config.rs` - Test functions (lines 2360+)

### Production Code
Limited unwrap usage in hot paths (good practice observed):
- `aethermap/aethermap-gui/src/gui.rs:1329` - `.iter().next().unwrap()` on grabbed_devices
- `aethermap/aethermap-gui/src/gui.rs:3305` - `auto_switch_view.as_ref().unwrap()` (checked via Option before)
- `aethermap/aethermap-gui/src/gui.rs:3464` - `hotkey_view.as_ref().unwrap()` (checked via Option before)

**Assessment:** Production unwrap calls are defensive and follow established patterns.

## Dead Code Warnings

### Allowed Dead Code (4 instances)
All marked with `#[allow(dead_code)]` for valid reasons:
- `aethermap/aethermap-gui/src/focus_tracker.rs:79` - Future use or async pattern
- `aethermap/aethermap-gui/src/gui.rs:58`, `aethermap/aethermap-gui/src/gui.rs:692` - Reserved for future features
- `aethermap/aethermapd/src/gamepad_device.rs:69` - Test utility or future integration

**Assessment:** No concerning dead code accumulation.

## Missing Phases

### Phase 12: LED Control (0/8 plans)
**Blocker:** HID LED protocol research incomplete
**Files:** Phase 12 implementation in `aethermap/aethermapd/src/led_controller.rs` partially complete
**What's done:** Device enumeration, brightness control, keepalive, async bridge
**What's missing:** RGB color control, pattern commands, per-layer colors, GUI color picker
**Risk:** Hardware may not support RGB beyond brightness

### Phase 16: Calibration GUI (0/8 plans)
**Status:** Not started
**Dependencies:** Phase 15 (all analog modes complete)
**Scope:** Real-time analog input visualization, deadzone/curve/range controls, live preview
**Files to create:** New GUI views and IPC protocol for calibration read/write

## Future Work (Prioritized)

1. **Phase 12: Complete LED Control** - Blocked by protocol research
2. **Phase 16: Calibration GUI** - Next logical phase after 15
3. **LED Animation Channel Architecture** - Fix breathing/rainbow animations
4. **Per-Layer Analog Calibration Persistence** - Already designed, needs storage integration
5. **Global Hotkey Config Loading** - IPC handlers exist, need ConfigManager integration

---

*Concerns audit: 2026-02-20*
