---
phase: 13-wayland-portal-integration
plan: 04
subsystem: profile-switching
tags: [auto-switch, focus-tracking, yaml-config, layer-manager, ipc]

# Dependency graph
requires:
  - phase: 13-02
    provides: FocusTracker component for window focus detection
  - phase: 13-03
    provides: FocusChanged IPC protocol
  - phase: 10
    provides: LayerManager for profile and layer activation
provides:
  - AutoProfileSwitcher with app_id matching and profile switching
  - AutoSwitchRule struct for YAML configuration
  - Manual override tracking for user-initiated profile changes
affects: [13-05-global-hotkeys]

# Tech tracking
tech-stack:
  added: []
  patterns: [first-match-wins-rules, graceful-fallback, manual-override-clearing]

key-files:
  created: [aethermap/aethermapd/src/auto_profile_switcher.rs]
  modified: [aethermap/aethermapd/src/config.rs, aethermap/aethermapd/src/lib.rs, aethermap/aethermapd/src/ipc.rs]

key-decisions:
  - "Wildcard '*' app_id matches any application for default profile"
  - "Rules evaluated in order with first-match-wins semantics"
  - "Manual overrides cleared on every focus change to re-enable auto-switching"
  - "Prefix matching (org.mozilla.) and suffix matching (.firefox) for flexible app_id patterns"

patterns-established:
  - "Pattern 1: Optional auto_profile_switcher parameter allows graceful degradation when unavailable"
  - "Pattern 2: Manual override state is per-device, allowing selective auto-switching control"
  - "Pattern 3: Profile validation before switching prevents errors from missing configurations"

# Metrics
duration: 15min
completed: 2026-02-19
---

# Phase 13: Plan 04 - AutoProfileSwitcher Summary

**AutoProfileSwitcher with app_id to profile matching using YAML rules, wildcard pattern matching, and manual override tracking**

## Performance

- **Duration:** 15 minutes
- **Started:** 2026-02-19T00:14:02Z
- **Completed:** 2026-02-19T00:29:00Z
- **Tasks:** 5 completed
- **Files modified:** 4 (2 created, 2 modified)

## Accomplishments

- **AutoSwitchRule struct** in config.rs for YAML deserialization of auto-switch rules
- **AutoProfileSwitcher module** with app_id matching (exact, prefix, suffix, wildcard)
- **IPC integration** for FocusChanged events to trigger profile switching
- **Manual override tracking** to prevent auto-switching after manual profile changes
- **Comprehensive unit tests** covering all matching patterns and override behaviors

## Task Commits

Each task was committed atomically:

1. **Task 1: Add AutoSwitchRule to config.rs for YAML loading** - `1dc9980` (feat)
2. **Task 2-3: Create AutoProfileSwitcher and export from lib.rs** - `da8a481` (feat)
3. **Task 4: Wire AutoProfileSwitcher to IPC FocusChanged handler** - `0d323aa` (feat)
4. **Task 5: Add unit tests for AutoProfileSwitcher** - `4e55089` (test)

**Plan metadata:** (docs: complete plan)

## Files Created/Modified

- `aethermap/aethermapd/src/auto_profile_switcher.rs` - AutoProfileSwitcher with focus-based profile switching logic
- `aethermap/aethermapd/src/config.rs` - Added AutoSwitchRule struct and auto_switch_rules field
- `aethermap/aethermapd/src/lib.rs` - Added pub mod auto_profile_switcher and re-exports
- `aethermap/aethermapd/src/ipc.rs` - Integrated switcher with FocusChanged handler and manual override on ActivateProfile

## Decisions Made

- **Wildcard semantics:** "*" app_id matches any application, useful for default profile fallback
- **Rule ordering:** First match wins, allowing specific rules to override general ones
- **Manual override lifetime:** Overrides persist until next focus change (not indefinite)
- **Pattern matching flexibility:** Support for prefix (org.mozilla.) and suffix (.firefox) matching for Wayland app_id variations
- **Graceful degradation:** AutoProfileSwitcher is optional; IPC works without it for backward compatibility

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

**Test compilation errors:** After adding auto_profile_switcher parameter to handle_request, existing unit tests in ipc.rs failed to compile with missing parameter errors. Fixed by adding `None` as the auto_profile_switcher parameter to all test calls.

## User Setup Required

None - no external service configuration required.

## Verification

All verification criteria met:
- [x] cargo check passes for daemon
- [x] AutoSwitchRule struct exists in config.rs
- [x] AutoProfileSwitcher module created with all required methods
- [x] IPC FocusChanged handler calls apply_focus_change()
- [x] Manual overrides set on ActivateProfile
- [x] Manual overrides cleared on focus change
- [x] Unit tests cover all key scenarios (7 tests passing)

## Next Phase Readiness

- AutoProfileSwitcher foundation complete and ready for 13-05 (global hotkeys)
- Manual override mechanism allows global hotkeys to coexist with auto-switching
- AutoSwitchRule YAML format ready for GUI configuration interface

---
*Phase: 13-wayland-portal-integration*
*Plan: 04*
*Completed: 2026-02-19*
