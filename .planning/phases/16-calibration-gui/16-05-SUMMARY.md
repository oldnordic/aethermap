---
phase: 16-calibration-gui
plan: 05
subsystem: gui
tags: [real-time, streaming, ipc, analog, visualization]

# Dependency graph
requires:
  - phase: 16-calibration-gui
    provides: [IPC subscription protocol (16-04), AnalogVisualizer widget (16-02)]
provides:
  - Real-time analog input streaming from daemon to GUI
  - GUI subscription lifecycle (subscribe on open, unsubscribe on close)
  - broadcast_analog_input() daemon function for event broadcasting
affects: [16-06-sensitivity-curve-graph, 16-07-live-preview, 16-08-testing]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - IPC subscription pattern with UnboundedSender<Response>
    - Real-time update handler in Iced GUI (Message::AnalogInputUpdated)
    - DaemonState reference in DeviceManager for broadcast access

key-files:
  created: []
  modified:
    - aethermap/aethermapd/src/ipc.rs
    - aethermap/aethermapd/src/device.rs
    - aethermap/aethermapd/src/main.rs
    - aethermap/aethermap-gui/src/ipc.rs
    - aethermap/aethermap/aethermap-gui/src/gui.rs

key-decisions:
  - "Use fire-and-forget UnboundedSender for analog event broadcasting (no backpressure needed)"
  - "Broadcast RAW input values (-1.0 to 1.0) before calibration for visualization accuracy"
  - "Subscribe in parallel with calibration load via Command::batch for faster UI startup"
  - "Unsubscribe using background thread spawn to avoid blocking UI close"

patterns-established:
  - "IPC Subscription Pattern: SubscribeAnalogInput request -> AnalogInputSubscribed response -> stream of AnalogInputUpdate responses"
  - "GUI Streaming Lifecycle: Open view -> Subscribe -> Receive updates -> Update visualizer -> Close view -> Unsubscribe"

# Metrics
duration: 18min
completed: 2026-02-20
tasks: 3
files: 5
commits: 4
---

# Phase 16: Plan 05 Summary

**Real-time analog input streaming from daemon via IPC subscription with broadcast_analog_input() and GUI update handlers**

## Performance

- **Duration:** 18 min
- **Started:** 2026-02-20T07:00:18Z
- **Completed:** 2026-02-20T07:18:00Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- Added `broadcast_analog_input()` function to daemon IPC module for broadcasting analog events to subscribed GUI clients
- Extended DeviceManager with daemon_state reference and set_daemon_state() method for broadcast access
- Added `subscribe_analog_input()` and `unsubscribe_analog_input()` functions to GUI IPC client
- Added `Message::AnalogInputUpdated(f32, f32)` variant to GUI for receiving streaming updates
- Wired full subscription lifecycle in GUI: subscribe on calibration view open, update stick_x/stick_y on events, unsubscribe on close

## Task Commits

Each task was committed atomically:

1. **Task 1: Add broadcast_analog_input function to daemon** - `e2bd0a2` (feat)
2. **Task 2: Add subscription functions to GUI IPC client** - `533d2ee` (feat)
3. **Task 3: Wire subscription in GUI with message handling** - `ae3fe3c` (feat)
4. **Fix: Correct RwLock access pattern in broadcast_analog_input** - `2993318` (fix)

**Plan metadata:** N/A (plan already created in prior wave)

## Files Created/Modified

- `aethermap/aethermapd/src/ipc.rs` - Added `broadcast_analog_input()` function (35 lines)
- `aethermap/aethermapd/src/device.rs` - Added daemon_state field to DeviceManager, set_daemon_state(), updated start_event_reader signature, added broadcast call in event loop
- `aethermap/aethermapd/src/main.rs` - Set daemon_state and layer_manager on device_manager
- `aethermap/aethermap-gui/src/ipc.rs` - Added `subscribe_analog_input()` and `unsubscribe_analog_input()` functions (52 lines)
- `aethermap/aethermap-gui/src/gui.rs` - Added `Message::AnalogInputUpdated`, subscription on open, handler for updates, unsubscribe on close (48 lines)

## Decisions Made

- **Use UnboundedSender for analog event broadcasting** - No backpressure needed for GUI updates; if GUI can't keep up, events are dropped (acceptable for visualization)
- **Broadcast RAW input before calibration** - Visualizer shows actual stick position; deadzone overlay shows how calibration affects it
- **Subscribe in parallel with calibration load** - Use Command::batch to avoid sequential latency; user sees visualizer update sooner
- **Unsubscribe via background thread spawn** - Avoids blocking UI close; unsubscribe is fire-and-forget since daemon cleans up stale subscriptions

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed RwLock access pattern in broadcast_analog_input**
- **Found during:** Task 1 compilation
- **Issue:** Initial implementation incorrectly tried to call `.get()` on `Arc<RwLock<HashMap>>` without acquiring the inner read lock
- **Fix:** Changed from `state.read().await.analog_subscribers.clone()` to `state.read().await.analog_subscribers.read().await` with proper guard lifetime
- **Files modified:** aethermap/aethermapd/src/ipc.rs
- **Verification:** `cargo check --manifest-path aethermap/aethermapd/Cargo.toml` passes
- **Committed in:** `2993318` (separate fix commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Fix was necessary for correctness - RwLock requires proper lock acquisition pattern. No scope creep.

## Issues Encountered

- **Initial compilation error in broadcast_analog_input** - Misunderstanding of nested RwLock structure; fixed by properly acquiring both outer (DaemonState) and inner (analog_subscribers HashMap) read locks with guard lifetime management

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Real-time streaming foundation complete; ready for sensitivity curve graph (16-06) and live preview (16-07)
- GUI now receives analog input updates; visualizer will show live stick position once subscription receives events
- Need to verify end-to-end streaming in 16-08 (unit and integration tests)

---
*Phase: 16-calibration-gui*
*Plan: 05*
*Completed: 2026-02-20*
