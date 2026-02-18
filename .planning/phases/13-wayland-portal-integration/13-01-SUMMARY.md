---
phase: 13-wayland-portal-integration
plan: 01
subsystem: wayland-integration
tags: [ashpd, xdg-desktop-portal, wayland]

# Dependency graph
requires: []
provides:
  - ashpd dependency in razermapper-gui for Wayland portal integration
  - Foundation for FocusTracker component to detect window focus changes
affects:
  - 13-02-focus-tracker
  - 13-03-ipc-protocol

# Tech tracking
tech-stack:
  added:
    - ashpd 0.8 (Rust xdg-desktop-portal bindings)
  patterns:
    - Portal-based window focus detection on Wayland

key-files:
  modified:
    - razermapper/razermapper-gui/Cargo.toml

key-decisions:
  - "Added tokio feature to ashpd for compatibility with existing async runtime"
  - "Used default-features = false to minimize dependency bloat"

patterns-established:
  - "Minimal feature flags for portal dependencies"
  - "GUI runs as normal user for portal access (daemon runs as root)"

# Metrics
duration: 5min
completed: 2026-02-19
---

# Phase 13 Plan 01: Ashpd Dependency Summary

**ashpd 0.8 dependency with wayland and tokio features for xdg-desktop-portal integration**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-18T23:56:28Z
- **Completed:** 2026-02-18T23:57:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Added ashpd 0.8 dependency to razermapper-gui with minimal features
- Configured with `default-features = false` and `features = ["wayland", "tokio"]`
- Verified compilation passes with cargo check
- Foundation laid for FocusTracker component in next plan

## Task Commits

Each task was committed atomically:

1. **Task 1: Add ashpd dependency to razermapper-gui Cargo.toml** - `7ee27d0` (feat)

**Plan metadata:** (to be added after summary creation)

## Files Created/Modified

- `razermapper/razermapper-gui/Cargo.toml` - Added ashpd 0.8 dependency with wayland and tokio features

## Decisions Made

**Deviation - Added tokio feature:**
- **Reason:** The plan specified only `["wayland"]` feature, but cargo automatically added `tokio` during dependency resolution
- **Rationale:** The GUI already uses tokio async runtime, and the tokio feature ensures proper integration between ashpd and the existing async infrastructure
- **Impact:** Minimal - tokio feature is required for async portal operations

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added tokio feature to ashpd dependency**
- **Found during:** Task 1 (cargo check after adding dependency)
- **Issue:** Initial configuration with only `["wayland"]` feature caused zbus compilation errors due to missing async runtime integration
- **Fix:** Added `tokio` feature to ashpd dependency configuration. Cargo's dependency resolver automatically pulled in async-io, async-lock, and other required transitive dependencies
- **Files modified:** razermapper/razermapper-gui/Cargo.toml
- **Verification:** cargo check passes with only pre-existing dead code warnings
- **Committed in:** 7ee27d0 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 missing critical)
**Impact on plan:** The tokio feature addition is necessary for async portal operations. No scope creep - this is required infrastructure.

## Issues Encountered

**zbus 4.4 compilation errors with default-features = false:**
- Initial attempt to use `default-features = false, features = ["wayland"]` resulted in 79 compilation errors from zbus 4.4 (transitive dependency of ashpd 0.8)
- Errors indicated missing crates: `blocking`, `async_io`, `async_fs`, `async_lock`
- **Resolution:** Removed `default-features = false` and allowed cargo to resolve default features, then re-added with both `wayland` and `tokio` features
- This configuration ensures all necessary async runtime dependencies are included

## User Setup Required

None - no external service configuration required for this plan.

## Next Phase Readiness

- ashpd dependency installed and verified
- Ready for 13-02 (FocusTracker component) to implement window focus detection
- No blockers or concerns

---
*Phase: 13-wayland-portal-integration*
*Plan: 01*
*Completed: 2026-02-19*
