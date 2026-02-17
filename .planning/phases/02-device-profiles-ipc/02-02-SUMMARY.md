---
phase: 02-device-profiles-ipc
plan: 02
subsystem: config
tags: [yaml, serde, device-profiles, hierarchical-config]

# Dependency graph
requires:
  - phase: 02-device-profiles-ipc
    provides: RemapTable type alias for atomic profile switching
provides:
  - Extended YAML configuration structures for per-device profiles with match_pattern and description support
  - load_device_profiles_extended() method returning HashMap<String, Vec<RemapProfile>>
affects: [02-03, 02-04, 02-05]

# Tech tracking
tech-stack:
  added: []
  patterns: [hierarchical-yaml-config, eager-validation, serde-default-derive]

key-files:
  created: []
  modified: [razermapper/razermapperd/src/config.rs]

key-decisions:
  - "Extended structures added alongside existing DeviceProfilesConfig to maintain backward compatibility"
  - "Flat HashMap<String, String> remaps instead of Vec<RemapEntry> for more ergonomic YAML syntax"
  - "Optional match_pattern field for flexible device matching in future plans"

patterns-established:
  - "Extended YAML config: devices -> device_id -> profiles -> profile_name -> remaps (HashMap)"
  - "Default global remaps fallback in top-level config"

# Metrics
duration: 4min
completed: 2026-02-17
---

# Phase 02 Plan 02: Extended YAML Config for Per-Device Profiles Summary

**Extended YAML configuration structures with match_pattern support, profile descriptions, and flat HashMap-based remaps**

## Performance

- **Duration:** 4 min
- **Started:** 2026-02-17T14:47:26Z
- **Completed:** 2026-02-17T14:52:15Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Added ExtendedDeviceRemapConfig, ProfileRemaps, and RemapDevicesConfig structures
- Added load_device_profiles_extended() method returning HashMap<String, Vec<RemapProfile>>
- Added 3 unit tests verifying extended profile loading, invalid key rejection, and empty file handling
- Maintained backward compatibility with existing DeviceProfilesConfig structures

## Task Commits

Each task was committed atomically:

1. **Task 1: Add device profile YAML structures and loading** - `67dbdd9` (feat)

**Plan metadata:** Pending final docs commit

## Files Created/Modified

- `razermapper/razermapperd/src/config.rs` - Added extended YAML structures and loading method

## YAML Structure Example

```yaml
devices:
  "1532:0220":  # vendor:product as key
    match_pattern: "1532:0220"  # Optional flexible matching
    profiles:
      gaming:
        name: "Gaming Profile"
        description: "Optimized for gaming"
        remaps:
          capslock: leftctrl  # Flat key:value pairs
          a: b
      work:
        name: "Work Profile"
        remaps:
          esc: grave
default:  # Optional global fallback
  KEY_1: KEY_2
```

## Decisions Made

- Extended structures added alongside existing DeviceProfilesConfig to maintain backward compatibility
- Flat HashMap<String, String> remaps instead of Vec<RemapEntry> for more ergonomic YAML syntax
- Optional match_pattern field added for flexible device matching (implementation deferred to future plan)
- Optional description field for profiles to support UI display

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Extended YAML configuration structures ready for IPC handler implementation (Plan 02-03)
- Profile descriptions available for GUI display
- Match pattern field prepared for flexible device matching implementation

---
*Phase: 02-device-profiles-ipc*
*Completed: 2026-02-17*
