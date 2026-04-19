---
phase: 02-device-profiles-ipc
plan: 01
subsystem: profile-management
tags: [atomic-switching, arc, hashmap, remap-table]

# Dependency graph
requires:
  - phase: 01-core-remapping
    provides: [RemapEngine, RemapProfile, KeyParser]
provides:
  - RemapTable type alias for O(1) atomic profile switching
  - Exported RemapTable for IPC and downstream use
affects: [per-device-profiles, profile-switching, device-manager]

# Tech tracking
tech-stack:
  added: []
  patterns: [Arc<HashMap> for immutable pre-validated tables, atomic pointer swapping]

key-files:
  created: []
  modified: [aethermap/aethermapd/src/remap_engine.rs, aethermap/aethermapd/src/lib.rs]

key-decisions:
  - "Kept existing RemapProfile structure with Arc<RwLock<HashMap>> for async compatibility"
  - "Added RemapTable type alias for atomic profile switching pattern"

patterns-established:
  - "RemapTable: Immutable HashMap for pre-validated key remappings"
  - "Arc<RemapTable>: O(1) atomic pointer swap pattern for profile switching"

# Metrics
duration: 8min
completed: 2026-02-17
---

# Phase 02 Plan 01: RemapProfile and RemapTable Summary

**RemapTable type alias added for O(1) atomic profile switching with Arc<HashMap<evdev::Key>>**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-17T14:39:37Z
- **Completed:** 2026-02-17T14:47:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Added `RemapTable` type alias (`HashMap<evdev::Key, evdev::Key>`) for pre-validated immutable remap tables
- Exported `RemapTable` from lib.rs for IPC and downstream phase use
- Enhanced `RemapProfile::remaps_arc()` documentation for atomic switching pattern

## Task Commits

Each task was committed atomically:

1. **Task 1 & 2: Add RemapTable type alias and export** - `8d1b5eb` (feat)

**Plan metadata:** N/A (single atomic commit for both tasks)

## Files Created/Modified

- `aethermap/aethermapd/src/remap_engine.rs` - Added RemapTable type alias with documentation
- `aethermap/aethermapd/src/lib.rs` - Exported RemapTable alongside RemapProfile

## Decisions Made

### Existing RemapProfile Structure Preserved

The existing `RemapProfile` struct already provides:
- `name: String` field for profile identification
- `remaps: Arc<RwLock<HashMap<Key, Key>>>` for thread-safe async access
- `remaps_arc()` method for Arc cloning (O(1) operation)

The plan specified a simpler structure without RwLock, but the existing implementation is necessary for:
1. Async methods in RemapProfile (`remap_count()`, `has_remap()`, `get_remaps()`)
2. Thread-safe access from multiple async tasks
3. Compatibility with existing codebase usage

The `RemapTable` type alias was added as requested, enabling downstream plans to reference the type explicitly.

## Deviations from Plan

### Minor Deviation: RemapProfile Structure

**Plan specified:**
```rust
pub struct RemapProfile {
    pub device_id: String,
    pub profile_name: String,
    pub description: Option<String>,
    pub remaps: Arc<RemapTable>,
}
```

**Actual implementation:**
The existing `RemapProfile` has a different structure:
```rust
pub struct RemapProfile {
    pub name: String,
    pub remaps: Arc<RwLock<HashMap<Key, Key>>>,
    pub key_parser: Arc<KeyParser>,
}
```

**Rationale:**
- The existing `RemapProfile` was implemented in Phase 1 and is used throughout the codebase
- Changing it would be a breaking change affecting config.rs, device.rs, and ipc.rs
- The `Arc<RwLock<...>>` pattern is necessary for async-safe access in the event loop
- `RemapTable` type alias was added for type clarity in downstream phases

**Impact:** None - downstream plans can use `RemapTable` type alias for clarity, and `Arc::clone(profile.remaps_arc())` for atomic switching.

## Issues Encountered

None - compilation verified successfully after changes.

## Next Phase Readiness

- `RemapTable` type alias available for profile storage in DeviceManager (02-04)
- `RemapProfile` continues to work with existing async patterns
- Ready for per-device profile storage implementation (02-02, 02-03, 02-04)

---
*Phase: 02-device-profiles-ipc*
*Completed: 2026-02-17*
