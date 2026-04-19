---
phase: 16-calibration-gui
plan: 04
subsystem: ipc-protocol
tags: [tokio, mpsc, unbounded-channel, subscription-pattern]

# Dependency graph
requires:
  - phase: 16-calibration-gui
    plan: 03
    provides: analog visualization widgets (CurveGraph)
provides:
  - IPC protocol for analog input streaming (SubscribeAnalogInput, UnsubscribeAnalogInput)
  - Response::AnalogInputUpdate variant for real-time stick position data
  - Daemon-side subscriber tracking in DaemonState
affects: [16-05-live-preview-integration, 16-06-calibration-controls-ui]

# Tech tracking
tech-stack:
  added: [tokio::sync::mpsc::UnboundedSender]
  patterns: [subscription pattern per-device, fire-and-forget broadcasting]

key-files:
  created: []
  modified:
    - aethermap/aethermap-common/src/lib.rs
    - aethermap/aethermapd/src/lib.rs
    - aethermap/aethermapd/src/ipc.rs

key-decisions:
  - "One GUI client only - UnsubscribeAnalogInput removes all subscribers for device (deferred per-client tracking to Phase 17)"
  - "Fire-and-forget UnboundedSender for simplicity - sends drop silently if client disconnected"

patterns-established:
  - "Subscriber map: HashMap<device_id, Vec<UnboundedSender<Response>>>"
  - "DaemonState Arc<RwLock<>> wrapper for subscriber tracking"

# Metrics
duration: 8min
completed: 2026-02-20T07:05:00Z
---

# Phase 16: Calibration GUI - Plan 04 Summary

**IPC subscription protocol with SubscribeAnalogInput/UnsubscribeAnalogInput requests and AnalogInputUpdate response for real-time analog streaming**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-20T06:57:00Z
- **Completed:** 2026-02-20T07:05:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Added Request::SubscribeAnalogInput and Request::UnsubscribeAnalogInput to IPC protocol
- Added Response::AnalogInputUpdate for streaming analog data (axis_x, axis_y from -1.0 to 1.0)
- Added Response::AnalogInputSubscribed acknowledgment for subscription confirmation
- Added analog_subscribers field to DaemonState for tracking subscribers per device
- Implemented daemon-side handlers for subscribe/unsubscribe requests

## Task Commits

Each task was committed atomically:

1. **Task 1: Add IPC subscription protocol to aethermap-common** - `1383355` (feat)
   - Added Request::SubscribeAnalogInput variant
   - Added Request::UnsubscribeAnalogInput variant
   - Added Response::AnalogInputUpdate variant (axis_x, axis_y)
   - Added Response::AnalogInputSubscribed acknowledgment

2. **Task 2: Implement daemon-side subscription handling** - `648e6fd` (feat)
   - Added analog_subscribers field to DaemonState
   - Implemented SubscribeAnalogInput handler (adds client to subscribers map)
   - Implemented UnsubscribeAnalogInput handler (removes device from subscribers)

**Plan metadata:** N/A (source files not tracked in this git repo)

## Files Created/Modified

- `aethermap/aethermap-common/src/lib.rs` - Added subscription protocol variants to Request and Response enums
- `aethermap/aethermapd/src/lib.rs` - Added analog_subscribers field to DaemonState
- `aethermap/aethermapd/src/ipc.rs` - Added handlers for SubscribeAnalogInput and UnsubscribeAnalogInput

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed borrow checker issue in subscription handlers**
- **Found during:** Task 2 (Initial compilation attempt)
- **Issue:** Chained write locks (`state.write().await.analog_subscribers.write().await`) created temporary value that was freed while still borrowed (E0716)
- **Fix:** Split into two statements: first acquire state write lock, then acquire subscribers write lock from state_guard
- **Files modified:** aethermap/aethermapd/src/ipc.rs
- **Verification:** `cargo check --manifest-path aethermap/aethermapd/Cargo.toml` passes successfully
- **Committed in:** Part of Task 2

---

**Total deviations:** 1 auto-fixed (1 borrow checker fix)
**Impact on plan:** Fix necessary for compilation. No scope creep - subscription handlers work as specified.

## Issues Encountered

- **Source files not tracked in git:** The aethermap source code is in a separate repository. Files committed in aethermap/.git but not visible in planning repo.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- IPC subscription protocol complete and ready for GUI integration
- Daemon tracks subscribers per device (ready for broadcasting in Phase 16-05)
- Response::AnalogInputUpdate defined for streaming stick position data

---
*Phase: 16-calibration-gui*
*Plan: 04*
*Completed: 2026-02-20T07:05:00Z*
