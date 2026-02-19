---
phase: 17-hotkey-and-auto-switch-persistence
plan: 01
subsystem: config
tags: [yaml, persistence, hotkey, tokio, serde]

# Dependency graph
requires:
  - phase: 13-wayland-portal-integration
    provides: [HotkeyBinding type, IPC protocol stubs]
provides:
  - ConfigManager hotkey binding persistence to device_profiles.yaml
  - Validation error type for duplicate detection
  - CRUD interface for hotkey bindings (add/remove/list)
affects: [17-02, 17-03, 17-04]

# Tech tracking
tech-stack:
  added: []
  patterns: [YAML persistence with tokio::fs::write, retain-based filtering for deletion, graceful degradation for missing data]

key-files:
  created: []
  modified:
    - razermapper/razermapperd/src/config.rs

key-decisions:
  - "Added Validation variant to RemapConfigError for hotkey duplicate detection"
  - "Used case-insensitive modifier comparison to match existing HotkeyBinding::normalize_modifiers() behavior"
  - "Single commit for all three CRUD methods since they form a complete interface"

patterns-established:
  - "ConfigManager CRUD pattern: load YAML -> modify in-memory -> serialize -> write back"
  - "Error-first validation: check duplicates before modifying state"
  - "Graceful degradation: get_hotkey_bindings returns empty Vec for missing file/device"

# Metrics
duration: 8min
completed: 2026-02-20
---

# Phase 17 Plan 01: ConfigManager Hotkey Binding Persistence Summary

**ConfigManager CRUD methods for hotkey bindings with YAML persistence to device_profiles.yaml using tokio async I/O**

## Performance

- **Duration:** 8 minutes
- **Started:** 2026-02-20T00:10:00Z
- **Completed:** 2026-02-20T00:18:00Z
- **Tasks:** 3 (all in one commit)
- **Files modified:** 1

## Accomplishments

- Added `Validation` variant to `RemapConfigError` for structured error handling
- Implemented `add_hotkey_binding()` with duplicate detection (same key + modifiers)
- Implemented `remove_hotkey_binding()` using retain() for functional filtering
- Implemented `get_hotkey_bindings()` with graceful degradation for missing data

## Task Commits

All three tasks were committed atomically in a single commit:

1. **Task 1-3: Hotkey binding CRUD methods** - `42fac6f` (feat)
   - Added RemapConfigError::Validation variant
   - Implemented add_hotkey_binding()
   - Implemented remove_hotkey_binding()
   - Implemented get_hotkey_bindings()

**Plan metadata:** (pending final docs commit)

## Files Created/Modified

- `razermapper/razermapperd/src/config.rs`
  - Added `RemapConfigError::Validation` variant for validation errors
  - Added `add_hotkey_binding()` method (lines ~2337-2415)
  - Added `remove_hotkey_binding()` method (lines ~2417-2495)
  - Added `get_hotkey_bindings()` method (lines ~2497-2530)

## Decisions Made

1. **Added Validation error variant** - The plan specified returning `RemapConfigError::Validation` but this variant didn't exist. Added it to the enum along with Display impl support.

2. **Single commit for all CRUD methods** - Since add/remove/list are tightly related and form a complete interface, committed them together rather than separately.

3. **Case-insensitive modifier comparison** - Used to_lowercase() normalization when comparing modifiers to match the existing `HotkeyBinding::normalize_modifiers()` behavior.

## Deviations from Plan

None - plan executed exactly as written. All three methods follow the `set_analog_calibration` pattern for YAML persistence:

1. Load RemapDevicesConfig from device_profiles_path (or create new)
2. Modify device entry in memory
3. Serialize to YAML with serde_yaml::to_string
4. Write to device_profiles_path with tokio::fs::write
5. Log info message on success

## Issues Encountered

**Repository structure:** The planning repository (`remapper_rs`) is separate from the code repository (`razermapper/`). Commits were made to the razermapper git repository instead of the parent planning repository.

**Resolution:** Committed changes in the razermapper directory as expected. The planning repository only tracks plan files and documentation.

## Next Phase Readiness

- ConfigManager now has complete CRUD interface for hotkey bindings
- IPC handlers (Plan 17-02) can call these methods for persistence
- GUI (Plan 17-03, 17-04) can persist hotkey changes via IPC

**No blockers or concerns.**

---
*Phase: 17-hotkey-and-auto-switch-persistence*
*Plan: 01*
*Completed: 2026-02-20*
