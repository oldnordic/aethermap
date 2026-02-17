---
phase: 10-layer-system-state-management
plan: 01
subsystem: layer-state
tags: [layers, remapping, evdev, tokio, serde]

# Dependency graph
requires:
  - phase: 09-device-detection
    provides: DeviceType::Keypad, joystick button mapping, hat switch handling
provides:
  - LayerManager for per-device layer state management
  - DeviceLayerState for tracking active layers per device
  - LayerConfig with Hold/Toggle activation modes
  - LayerMode enum for layer activation behavior
affects: [11-analog-processing, 12-led-control]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Arc<RwLock<T>> for concurrent layer state access
    - Layer stacking with highest-active-layer priority
    - Custom serde serialization for evdev::Key types

key-files:
  created:
    - razermapper/razermapperd/src/layer_manager.rs
  modified:
    - razermapper/razermapperd/src/lib.rs

key-decisions:
  - LayerConfig uses custom serialize/deserialize for HashMap<Key, Key> compatibility
  - get_effective_layer() returns highest active layer ID for simple priority semantics
  - Minimum 3 layers always available (0=base, 1, 2) per device

patterns-established:
  - "Layer activation: HashSet<usize> tracks active layers, max() determines effective layer"
  - "Per-device isolation: each device has independent layer state"
  - "Hold vs Toggle: mode tracked separately in active_toggle_layers"

# Metrics
duration: 12min
completed: 2026-02-17
---

# Phase 10 Plan 01: LayerManager Component Summary

**Per-device layer state management with Hold/Toggle activation modes and priority-based layer stacking**

## Performance

- **Duration:** 12 min
- **Started:** 2026-02-17T22:38:00Z
- **Completed:** 2026-02-17T22:50:00Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Created LayerManager component with thread-safe per-device layer state
- Implemented Hold (modifier-held) and Toggle (press-on-press-off) layer activation modes
- Added 12 unit tests covering layer creation, activation, priority, and configuration
- Minimum 3 layers guaranteed per device with extensible configuration system

## Task Commits

Each task was committed atomically:

1. **Task 1: Create layer_manager.rs module with LayerManager, DeviceLayerState, LayerConfig** - `143af9b` (feat)
2. **Task 2: Export LayerManager types from lib.rs** - `9867710` (feat)
3. **Task 3: Add unit tests for LayerManager** - `9867710` (feat - combined with Task 2)

**Note:** Tests were included in Task 1's implementation commit, so Task 3 verification was already complete.

## Files Created/Modified

- `razermapper/razermapperd/src/layer_manager.rs` - 750 lines including LayerManager, DeviceLayerState, LayerConfig, LayerMode, and 12 unit tests
- `razermapper/razermapperd/src/lib.rs` - Added `pub mod layer_manager` and re-exports

## Decisions Made

- Used custom serde serialization for `HashMap<Key, Key>` because evdev::Key doesn't implement Serialize/Deserialize
  - Remaps serialize as Vec<(u16, u16)> for IPC compatibility
  - This enables LayerConfig to be used in future IPC protocols
- get_effective_layer() uses max(active_layers) rather than a stack for simplicity
  - Higher layer IDs always override lower ones
  - No need for manual push/pop operations
- active_toggle_layers tracked separately from active_layers for state clarity
  - Enables proper toggle state management across hold/toggle modes

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed evdev::Key serialization error**
- **Found during:** Task 1 (LayerConfig struct compilation)
- **Issue:** evdev::Key doesn't implement Serialize/Deserialize, causing compile error
- **Fix:** Added custom serialize_remaps/deserialize_remaps functions using Visitor pattern
- **Files modified:** razermapper/razermapperd/src/layer_manager.rs
- **Verification:** cargo check passes, LayerConfig now serializes correctly
- **Committed in:** 9867710 (Task 2 commit)

**2. [Rule 2 - Missing Critical] Added comprehensive unit tests beyond plan requirement**
- **Found during:** Task 1 implementation
- **Issue:** Plan specified 6 tests but added 12 for better coverage
- **Fix:** Added additional tests for LayerConfig, toggle functionality, layer manager operations
- **Files modified:** razermapper/razermapperd/src/layer_manager.rs
- **Verification:** All 12 tests pass (test_layer_manager_creation through test_layer_mode_display)
- **Committed in:** 143af9b (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 missing critical)
**Impact on plan:** Both auto-fixes necessary for correctness and completeness. No scope creep.

## Issues Encountered

None - all tasks completed as planned with auto-fixes for serialization and test coverage.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- LayerManager component ready for integration with RemapEngine
- Per-device layer state can be queried for effective layer during remap lookups
- Layer activation/deactivation API ready for modifier key bindings
- Requires: Plan 10-02 to add modifier-to-layer mapping for activation
- Enables: Layer-aware remapping in subsequent plans

---
*Phase: 10-layer-system-state-management*
*Completed: 2026-02-17*

## Self-Check: PASSED

- FOUND: razermapper/razermapperd/src/layer_manager.rs (22,146 bytes)
- FOUND: 10-01-SUMMARY.md (5,228 bytes)
- FOUND: commit 143af9b (Task 1: Create layer_manager.rs module)
- FOUND: commit 9867710 (Task 2: Export LayerManager types from lib.rs)
