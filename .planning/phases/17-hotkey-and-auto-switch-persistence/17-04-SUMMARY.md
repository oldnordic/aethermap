---
phase: 17-hotkey-and-auto-switch-persistence
plan: 04
subsystem: ipc
tags: [auto-switch, ipc, set-auto-switch-rules, auto-profile-switcher]

# Dependency graph
requires:
  - phase: 17-hotkey-and-auto-switch-persistence
    plan: 02
    provides: ConfigManager::set_auto_switch_rules, ConfigManager::get_auto_switch_rules
provides:
  - Request::SetAutoSwitchRules variant for IPC
  - Response::AutoSwitchRulesAck variant for IPC
  - SetAutoSwitchRules IPC handler with AutoProfileSwitcher reload
affects: [gui, phase-17-05]

# Tech tracking
tech-stack:
  added: [AutoSwitchRule in common library, reload_rules method]
  patterns: [Interior mutability via Arc<RwLock<>> for runtime rule updates]

key-files:
  created: []
  modified: [razermapper-common/src/lib.rs, razermapperd/src/auto_profile_switcher.rs, razermapperd/src/ipc.rs]

key-decisions:
  - "Added AutoSwitchRule to common library for IPC use (was daemon-internal only)"
  - "Added reload_rules(&self) to AutoProfileSwitcher for Arc-based access"
  - "Wrapped AutoProfileSwitcher.rules in Arc<RwLock<>> for runtime mutability"

patterns-established:
  - "IPC handler: save to config -> reload component -> return ack"

# Metrics
duration: 8min
completed: 2026-02-20
---

# Phase 17 Plan 04: SetAutoSwitchRules IPC Protocol Summary

**IPC protocol for auto-switch rule management with runtime AutoProfileSwitcher reload**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-20T23:41:40Z
- **Completed:** 2026-02-20T23:49:34Z
- **Tasks:** 3 (all completed)
- **Files modified:** 3

## Accomplishments

- Added `AutoSwitchRule` struct to razermapper-common for IPC use
- Added `Request::SetAutoSwitchRules` variant to Request enum with rules: Vec<AutoSwitchRule>
- Added `Response::AutoSwitchRulesAck` variant to Response enum
- Added `reload_rules(&self, rules)` method to AutoProfileSwitcher for Arc-based access
- Wrapped `AutoProfileSwitcher.rules` field in Arc<RwLock<>> for interior mutability
- Implemented SetAutoSwitchRules IPC handler with AutoProfileSwitcher reload

## Task Commits

Each task was committed atomically:

1. **Task 1: Add AutoSwitchRule struct and SetAutoSwitchRules Request variant** - `3a8fca0` (feat)
2. **Task 2: Add AutoSwitchRulesAck Response variant** - `e7bfecf` (feat)
3. **Task 3: Add reload_rules method to AutoProfileSwitcher** - `92c925b` (feat)
4. **Task 3: Implement SetAutoSwitchRules IPC handler with AutoProfileSwitcher reload** - `85d9fed` (feat)

## Files Created/Modified

- `razermapper/razermapper-common/src/lib.rs` - Added AutoSwitchRule struct, SetAutoSwitchRules Request variant, AutoSwitchRulesAck Response variant
- `razermapper/razermapperd/src/auto_profile_switcher.rs` - Added reload_rules method, wrapped rules in Arc<RwLock<>>
- `razermapper/razermapperd/src/ipc.rs` - Implemented SetAutoSwitchRules handler with switcher reload

## Decisions Made

**AutoSwitchRule added to common library for IPC**
- AutoSwitchRule was daemon-internal (defined in config.rs)
- GUI needs to send rules via IPC, requiring common library definition
- Struct is identical to config::AutoSwitchRule for compatibility

**Interior mutability for AutoProfileSwitcher.rules**
- Original `rules: Vec<AutoSwitchRule>` required `&mut self` for updates
- IPC handler has `Arc<AutoProfileSwitcher>` which doesn't provide `&mut self`
- Wrapped rules in `Arc<RwLock<Vec<AutoSwitchRule>>>` for thread-safe runtime updates
- Added `reload_rules(&self, rules)` async method for Arc-based access

**IPC handler pattern: save -> reload -> ack**
- Save rules via ConfigManager::set_auto_switch_rules()
- Reload AutoProfileSwitcher via switcher.reload_rules()
- Return AutoSwitchRulesAck on success
- This ensures rules take effect immediately without daemon restart

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Added interior mutability to AutoProfileSwitcher.rules**
- **Found during:** Task 3 (SetAutoSwitchRules handler implementation)
- **Issue:** Plan showed `switcher.write().await.load_rules(rules)` but auto_profile_switcher is `Option<Arc<AutoProfileSwitcher>>`, not `Option<Arc<RwLock<AutoProfileSwitcher>>>`
- **Fix:** Wrapped `rules` field in Arc<RwLock<Vec<AutoSwitchRule>>> and added `reload_rules(&self, rules)` method
- **Files modified:** razermapper/razermapperd/src/auto_profile_switcher.rs
- **Verification:** cargo check passes, switcher can be reloaded through Arc
- **Committed in:** 92c925b (Task 3 - reload_rules method)

**2. [Rule 1 - Bug] Discovered existing SetAutoSwitchRules handler was incomplete**
- **Found during:** Task 3 (IPC handler implementation)
- **Issue:** SetAutoSwitchRules handler already existed but didn't reload AutoProfileSwitcher
- **Fix:** Added switcher.reload_rules() call after successful save
- **Files modified:** razermapper/razermapperd/src/ipc.rs
- **Verification:** cargo check passes, handler now reloads switcher
- **Committed in:** 85d9fed (Task 3 - IPC handler)

---

**Total deviations:** 2 auto-fixed (1 missing functionality, 1 incomplete implementation)
**Impact on plan:** Deviations were necessary for correct behavior. No scope creep.

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- IPC protocol complete for auto-switch rule management
- GUI can now persist rules via SetAutoSwitchRules request
- Rules take effect immediately via AutoProfileSwitcher reload
- Ready for Plan 05 (GUI auto-switch rule persistence integration)

---

*Phase: 17-hotkey-and-auto-switch-persistence*
*Plan: 04*
*Completed: 2026-02-20*
