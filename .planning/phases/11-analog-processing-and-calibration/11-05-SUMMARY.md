---
phase: 11-analog-processing-and-calibration
plan: 05
subsystem: configuration
tags: [analog, calibration, layers, yaml, serde]

# Dependency graph
requires:
  - phase: 11-analog-processing-and-calibration
    provides: [AnalogCalibration data types, AnalogProcessor integration]
  - phase: 10-layer-system-state-management
    provides: [LayerConfig structure, layer management patterns]
provides:
  - Per-layer analog calibration configuration in LayerConfig
  - YAML config support for analog_calibration HashMap
  - ConfigManager methods for querying per-layer calibration
affects:
  - Phase 11-06 (analog macro recording), Phase 14-15 (output modes)

# Tech tracking
tech-stack:
  added: []
  patterns:
  - "Pattern: Optional<T> fields with #[serde(skip_serializing_if = "Option::is_none")]"
  - "Pattern: HashMap<layer_id, Config> for per-layer configuration"
  - "Pattern: Getter methods returning Option<T> with None as default"

key-files:
  created: []
  modified:
  - razermapper/razermapperd/src/layer_manager.rs (analog_calibration field in LayerConfig)
  - razermapper/razermapperd/src/config.rs (YAML structures and ConfigManager methods)
  - razermapper/razermapperd/src/ipc.rs (LayerConfig construction fix)
  - razermapper/razermapperd/tests/config_reload_test.rs (layer_state_path fix)
  - razermapper/razermapperd/tests/macro_integration_test.rs (analog_move method)

key-decisions:
  - "Per-layer calibration uses HashMap<usize, AnalogCalibration> indexed by layer_id"
  - "None for analog_calibration means use AnalogCalibration::default()"
  - "YAML format follows Phase 10-06 pattern for layer state persistence"

patterns-established:
  - "Pattern: Per-layer configuration uses Optional fields with serde defaults"
  - "Pattern: ConfigManager query methods return Option<T> for graceful defaults"

# Metrics
duration: 12min
completed: 2026-02-19
---

# Phase 11: Analog Processing and Calibration - Plan 05 Summary

**Per-layer analog calibration configuration in LayerConfig and YAML profiles, enabling layer-specific sensitivity and deadzone settings**

## Performance

- **Duration:** 12 min
- **Started:** 2026-02-19T04:22:00Z
- **Completed:** 2026-02-19T04:34:00Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- Added `analog_calibration: Option<AnalogCalibration>` field to LayerConfig
- Extended ExtendedDeviceRemapConfig YAML with per-layer analog_calibration HashMap
- Added ConfigManager methods for querying per-layer calibration from config
- Unit tests verify YAML loading and query methods work correctly

## Task Commits

Each task was committed atomically:

1. **Task 1: Add analog_calibration field to LayerConfig** - `15047e9` (feat)
2. **Task 2: Add analog_calibration to YAML config structures** - `8ff6951` (feat)
3. **Task 3: Add method to load per-layer calibration** - `0c2bec0` (feat)

**Plan metadata:** `TODO` (docs: complete plan)

## Files Created/Modified

- `razermapper/razermapperd/src/layer_manager.rs` - Added analog_calibration field to LayerConfig with getter method
- `razermapper/razermapperd/src/config.rs` - Added analog_calibration HashMap to ExtendedDeviceRemapConfig and ConfigManager query methods
- `razermapper/razermapperd/src/ipc.rs` - Fixed LayerConfig construction to include analog_calibration
- `razermapper/razermapperd/tests/config_reload_test.rs` - Fixed ConfigManager construction with layer_state_path
- `razermapper/razermapperd/tests/macro_integration_test.rs` - Added analog_move method to MockInjector

## Decisions Made

- Per-layer calibration uses HashMap<usize, AnalogCalibration> indexed by layer_id (0=base, 1, 2, ...)
- None for analog_calibration field means use AnalogCalibration::default() (graceful defaults)
- YAML format follows Phase 10-06 pattern for layer state persistence (serde with skip_serializing_if)
- ConfigManager query methods return Option<AnalogCalibration> for graceful handling of missing config

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- **Issue:** LayerConfig construction in ipc.rs was missing the new analog_calibration field
  - **Fix:** Added `analog_calibration: None` to the LayerConfig construction
- **Issue:** ExtendedDeviceRemapConfig construction in config.rs was missing analog_calibration field
  - **Fix:** Added `analog_calibration: HashMap::new()` to both construction sites
- **Issue:** Test files needed updates for new layer_state_path field and analog_move method
  - **Fix:** Updated config_reload_test.rs to include layer_state_path and macro_integration_test.rs to implement analog_move

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- LayerConfig now supports per-layer analog calibration
- YAML config can specify analog_calibration per device per layer
- ConfigManager can query per-layer calibration at runtime
- Ready for Phase 11-06 (analog macro recording) to use calibration-aware analog events

---
*Phase: 11-analog-processing-and-calibration*
*Completed: 2026-02-19*
