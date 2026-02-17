---
phase: 03-hotplug-hotreload
plan: 01
subsystem: hotplug
tags: [udev, evdev, tokio, hotplug, device-monitoring]

# Dependency graph
requires:
  - phase: 01-core-remapping
    provides: device manager with grab/ungrab, RemapEngine integration
provides:
  - Verified udev-based device hotplug monitoring implementation
  - Device add/remove event handlers with automatic profile application
  - Graceful device cleanup on removal
affects: [profile-switching, ipc, daemon]

# Tech tracking
tech-stack:
  added: []
  patterns:
  - spawn_blocking for blocking udev socket iteration
  - Best-effort cleanup for device removal
  - Drop trait for panic-safe device release

key-files:
  created: []
  modified:
    - razermapper/razermapperd/src/hotplug.rs - Udev monitor implementation (verified)
    - razermapper/razermapperd/src/device.rs - Device add/remove handlers (verified)

key-decisions:
  - "Implementation uses udev (not inotify) - ROADMAP.md terminology note"

patterns-established:
  - "Pattern: spawn_blocking for blocking I/O - udev monitor runs in blocking task to avoid blocking async runtime"
  - "Pattern: Best-effort cleanup - device removal logs errors but continues"
  - "Pattern: Panic-safe resource management - Drop trait on GrabbedDevice ensures EVIOCGRAB release"
  - "Pattern: Device ID format - vendor:product hex format (e.g., '1532:0220')"

# Metrics
duration: ~15min
completed: 2026-02-17
---

# Phase 03-01: Verify udev-based hotplug monitoring Summary

**Udev-based device hotplug monitoring with spawn_blocking, DeviceEvent streaming, and graceful add/remove handling**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-02-17 (checkpoint continuation)
- **Completed:** 2026-02-17
- **Tasks:** 3 verification tasks + 1 checkpoint (user approved)
- **Files verified:** 2 (hotplug.rs, device.rs)
- **No code changes** - verification only plan

## Accomplishments

- **Verified udev monitor implementation** - DeviceMonitor correctly uses udev::MonitorBuilder with subsystem filtering for input devices
- **Confirmed async-safe blocking** - spawn_blocking used for udev socket iteration to avoid blocking async runtime
- **Verified device add handler** - handle_device_add() re-discovers devices, checks for profiles, auto-grabs if configured
- **Verified device removal handler** - handle_device_removal() performs best-effort cleanup with ungrab and HashMap removal
- **Confirmed panic safety** - Drop trait on GrabbedDevice ensures EVIOCGRAB release even on panic

## Task Commits

No commits - this was a verification-only plan with no code modifications.

1. **Task 1: Verify udev monitor implementation** - VERIFIED (no changes)
2. **Task 2: Verify device add handler** - VERIFIED (no changes)
3. **Task 3: Verify device removal handler** - VERIFIED (no changes)
4. **Checkpoint: User verification approval** - APPROVED

## Files Created/Modified

**Verified (no changes):**
- `razermapper/razermapperd/src/hotplug.rs` (262 lines)
  - DeviceMonitor struct with mpsc channel for DeviceEvent streaming
  - spawn_blocking wrapper for udev monitor (line 69-73)
  - udev::MonitorBuilder with match_subsystem("input") (line 120-122)
  - DeviceEvent enum with Add/Remove variants (line 27-42)
  - device_map HashMap for devnode -> device_id tracking (line 127)
  - extract_device_id() using evdev::Device::input_id() (line 199-214)

- `razermapper/razermapperd/src/device.rs` (874 lines)
  - handle_device_add() method (line 277-337)
    - Re-opens device with evdev::Device::open() (line 287)
    - Extracts VID/PID and formats device_id (line 290-297)
    - Checks config_manager for profiles (line 313-325)
    - Auto-grabs if profiles exist (line 329)
  - handle_device_removal() method (line 244-259)
    - Best-effort ungrab with error logging (line 248-250)
    - Removes from devices HashMap (line 253-255)
  - Drop trait for GrabbedDevice (line 31-38)
    - Releases EVIOCGRAB on drop (line 36)

## Verification Results

### Task 1: Udev Monitor Implementation ✓
- Confirmed udev::MonitorBuilder monitors "input" subsystem (line 120-122)
- Confirmed spawn_blocking used for blocking udev socket (line 68-73)
- Confirmed DeviceEvent enum has Add/Remove variants with device_id (line 26-42)
- Confirmed device_map HashMap tracks devnode -> device_id (line 127)
- Confirmed extract_device_id() uses evdev::Device::input_id() for VID/PID (line 199-214)

### Task 2: Device Add Handler ✓
- Confirmed handle_device_add() re-opens device with evdev::Device::open() (line 287-294)
- Confirmed VID/PID extraction and device_id formatting (line 297)
- Confirmed config_manager.list_device_profiles() check (line 313-325)
- Confirmed grab_device() call if profiles exist (line 329)
- Confirmed device added to devices HashMap (line 310)

### Task 3: Device Removal Handler ✓
- Confirmed handle_device_removal() calls ungrab_device() with error handling (line 248-250)
- Confirmed removal from devices HashMap (line 253-255)
- Confirmed errors are logged but don't cause failure (best-effort)
- Confirmed Drop trait on GrabbedDevice handles ungrab on panic (line 31-38)

## Key Implementation Details

**Udev Monitor Flow:**
1. DeviceMonitor::new() spawns blocking task with run_udev_monitor()
2. udev::MonitorBuilder filters for "input" subsystem events
3. For Add events: extract_device_id() opens device, reads VID/PID, stores in device_map
4. For Remove events: looks up device_id from device_map, sends Remove event
5. Events sent via mpsc::channel to main daemon loop

**Device Add Flow:**
1. DeviceEvent::Add received from udev monitor
2. handle_device_add() re-opens device to get DeviceInfo
3. Formats device_id using format_device_id()
4. Checks config_manager for existing profiles
5. If profiles exist: calls grab_device() to intercept input
6. Device added to devices HashMap

**Device Removal Flow:**
1. DeviceEvent::Remove received from udev monitor
2. handle_device_removal() calls ungrab_device() (best-effort)
3. Removes from devices HashMap
4. Drop trait ensures EVIOCGRAB release even on panic

## Decisions Made

**Terminology Note:** The existing implementation uses udev for device monitoring, while ROADMAP.md mentions "inotify". This is not an issue - udev is the correct approach for device hotplug events (inotify is for file system events). The ROADMAP should be updated to reflect "udev" instead of "inotify" for accuracy.

## Deviations from Plan

None - plan executed exactly as specified. This was a verification-only plan with no code modifications required.

## Issues Encountered

None - all verification tasks completed successfully. The existing implementation is complete and correct.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Hotplug monitoring is verified and working. Ready for:
- Plan 03-02: Profile hot-reload (inotify YAML watching)
- Plan 03-03: Runtime profile switching (IPC integration)
- ROADMAP.md update: inotify -> udev terminology for hotplug section

**Note:** The ROADMAP.md mentions "inotify" for device hotplug, but the implementation correctly uses udev. Consider updating ROADMAP terminology for clarity.

---
*Phase: 03-hotplug-hotreload*
*Completed: 2026-02-17*
