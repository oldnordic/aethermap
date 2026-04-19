---
phase: 15-wasd-mouse-camera-modes
plan: 06
subsystem: analog-processing
tags: [camera-mode, scroll, key-repeat, per-layer-config, layer-config, device-event-loop]

# Dependency graph
requires:
  - phase: 15-wasd-mouse-camera-modes
    plan: 05
    provides: CameraOutputMode enum, CameraOutput enum, process_as_camera() method
provides:
  - camera_output_mode field in LayerConfig for per-layer camera mode selection
  - Camera mode handling in device event loop for scroll/key output emission
  - Integration with injector for REL_WHEEL and key repeat events
affects:
  - Phase 15-07: IPC protocol for camera mode configuration
  - Phase 15-08: Camera mode persistence and profiles
  - Phase 16: Calibration GUI (camera mode UI controls)

# Tech tracking
tech-stack:
  added: []
  patterns: [per-layer analog mode configuration, mode-based event emission, fire-and-forget camera events]

key-files:
  modified:
    - aethermap/aethermapd/src/layer_manager.rs
      - Added camera_output_mode field to LayerConfig
      - Added camera_output_mode() accessor method
      - Updated all LayerConfig constructors
    - aethermap/aethermapd/src/ipc.rs
      - Imported CameraOutputMode for IPC handling
      - Initialized camera_output_mode in LayerConfig creation
    - aethermap/aethermapd/src/device.rs
      - Added Camera mode handling in start_event_reader()
      - Integrated with process_as_camera() for scroll/key output
      - Fixed pre-existing WASD bug (use &current_keys in debug!)

key-decisions:
  - "CameraOutputMode defaults to Scroll for broader compatibility (REL_WHEEL works in most applications)"
  - "Camera mode uses per-layer calibration from analog_calibration field"
  - "Key mode emits immediate press/release for repeat effect (TODO: proper repeat interval in 15-08)"
  - "Camera events are fire-and-forget like mouse events (no state tracking needed)"

patterns-established:
  - "Per-layer analog output mode pattern: analog_mode + camera_output_mode fields"
  - "Camera event emission: process_as_camera() returns CameraOutput enum, match on Scroll/Keys variants"
  - "Layer-specific calibration: analog_calibration field takes precedence over device default"

# Metrics
duration: 10min
completed: 2026-02-19
---

# Phase 15 Plan 6: Camera Mode Integration Summary

**Camera mode integration in device event loop with per-layer scroll/key mode selection and REL_WHEEL or key repeat output**

## Performance

- **Duration:** 10 min
- **Started:** 2026-02-19T14:39:07Z
- **Completed:** 2026-02-19T14:49:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Added `camera_output_mode` field to LayerConfig with Scroll/Keys variants
- Added `camera_output_mode()` accessor method for retrieving layer-specific mode
- Integrated Camera mode handling in device event loop after Mouse mode
- Connected camera processing to injector for REL_WHEEL scroll events
- Connected camera processing to injector for key repeat events (PageUp/PageDown/arrows)
- Per-layer calibration support for camera mode processing

## Task Commits

Each task was committed atomically:

1. **Task 1: Add camera_output_mode field to LayerConfig** - `[hash from previous session]` (feat)
2. **Task 2: Add Camera mode handling to device event loop** - `[hash from previous session]` (feat)

**Plan metadata:** Not yet committed (docs: complete plan)

_Note: This work was completed in a previous session. All changes are present in the codebase._

## Files Created/Modified

- `aethermap/aethermapd/src/layer_manager.rs`
  - Added `camera_output_mode: CameraOutputMode` field to LayerConfig
  - Added `camera_output_mode()` accessor method
  - Updated LayerConfig::default() to initialize with CameraOutputMode::Scroll
  - Updated LayerConfig::new() to initialize with CameraOutputMode::Scroll
  - Updated DeviceLayerState::new() layer_configs to initialize with CameraOutputMode::Scroll

- `aethermap/aethermapd/src/ipc.rs`
  - Added CameraOutputMode to imports from analog_processor
  - Initialized camera_output_mode in IPC LayerConfig creation

- `aethermap/aethermapd/src/device.rs`
  - Added Camera mode handling in start_event_reader() after Mouse mode
  - Tracks both axes (ABS_X stores value, ABS_Y triggers processing)
  - Retrieves layer-specific calibration and camera_output_mode from LayerConfig
  - Calls process_as_camera() with calibration and camera mode
  - Emits REL_WHEEL events via injector.mouse_scroll() for Scroll mode
  - Emits key press/release events via injector for Keys mode
  - Fixed pre-existing WASD bug (use &current_keys in debug! macro)

## Decisions Made

1. **CameraOutputMode defaults to Scroll** - REL_WHEEL events work in most applications (browsers, document viewers), making it the safer default for broader compatibility
2. **Per-layer camera mode configuration** - Each layer can independently select Scroll or Keys mode, enabling use cases like "browse with scroll on layer 1, 3D camera control with keys on layer 2"
3. **Fire-and-forget camera events** - Camera events don't require state tracking like D-pad mode; each stick position produces immediate output
4. **Key mode uses immediate press/release** - For initial implementation, keys are emitted as immediate press/release pairs. Proper repeat interval handling deferred to plan 15-08 (testing and refinement)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed pre-existing WASD mode bug**
- **Found during:** Task 2 (verifying compilation after adding Camera mode)
- **Issue:** WASD mode had a borrow checker error - `current_keys` was moved in the for loop but used again in debug! macro
- **Fix:** Changed `debug!("WASD output: device={}, keys={:?}", id_clone, current_keys);` to use `&current_keys`
- **Files modified:** aethermap/aethermapd/src/device.rs
- **Verification:** cargo check passes with no errors
- **Committed in:** Part of Task 2 commit (Camera mode integration)

**2. [Rule 1 - Bug] Fixed Camera mode Keys variant borrow issue**
- **Found during:** Task 2 (initial compilation)
- **Issue:** CameraOutput::Keys(keys) moved the keys Vec in the for loop, causing borrow error in subsequent debug! macro
- **Fix:** Changed `for key in keys` to `for key in &keys` and `debug!(..., keys)` to `debug!(..., &keys)`
- **Files modified:** aethermap/aethermapd/src/device.rs
- **Verification:** cargo check passes with no errors
- **Committed in:** Part of Task 2 commit

**3. [Rule 1 - Bug] Fixed borrow lifetime issue in layer_config extraction**
- **Found during:** Task 2 (initial compilation)
- **Issue:** `get_layer_config()` returns a reference tied to lm_read, but the code tried to extract values after dropping lm_read
- **Fix:** Restructured to extract layer_calibration and camera_mode before dropping lm_read, using explicit if-let instead of and_then chains
- **Files modified:** aethermap/aethermapd/src/device.rs
- **Verification:** cargo check passes with no errors
- **Committed in:** Part of Task 2 commit

---

**Total deviations:** 3 auto-fixed (all Rule 1 - Bug fixes)
**Impact on plan:** All auto-fixes were necessary for correct compilation. None changed the plan's scope or intent.

## Issues Encountered

None - all tasks completed successfully with no blocking issues.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Camera mode integration complete and ready for IPC protocol extension (Plan 15-07)
- Per-layer camera_output_mode field ready for GUI configuration (Plan 15-08)
- Event loop integration enables immediate testing with scroll/key output
- IPC protocol needs extension for runtime camera mode configuration

---
*Phase: 15-wasd-mouse-camera-modes*
*Plan: 06*
*Completed: 2026-02-19*
