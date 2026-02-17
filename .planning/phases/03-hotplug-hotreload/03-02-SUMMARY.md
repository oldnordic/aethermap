---
phase: 03-hotplug-hotreload
plan: 02
subsystem: hot-reload
tags: [sighup, atomic-swap, arc-rwlock, tokio-signal, yaml-hot-reload]

# Dependency graph
requires:
  - phase: 01-core-remapping
    provides: [RemapEngine, KeyParser, ConfigManager]
provides:
  - Verified SIGHUP-based configuration hot-reload without daemon restart
  - Documented atomic swap pattern using Arc<RwLock<>>
  - Validated eager validation before config application
affects: [03-03-runtime-profile-switching, ipc-layer]

# Tech tracking
tech-stack:
  added: []
  patterns: [validate-then-swap, atomic-config-swap, eager-validation]

key-files:
  created: [.planning/phases/03-hotplug-hotreload/03-02-SUMMARY.md]
  modified: []

key-decisions:
  - "Existing SIGHUP implementation verified - no changes needed"
  - "Atomic swap pattern via Arc<RwLock<>> provides all-or-nothing config updates"
  - "Validation-before-swap ensures invalid configs never partially apply"

patterns-established:
  - "Validate-then-swap: Read file -> Parse YAML -> Validate all keys -> Acquire lock -> Atomic swap"
  - "Error logging with daemon continuation: Log errors but don't exit on reload failure"
  - "Tokio signal handling: SIGHUP in tokio::select! with nested select! for continued operation"

# Metrics
duration: 8min
completed: 2026-02-17
---

# Phase 03 Plan 02: SIGHUP Configuration Hot-Reload Summary

**SIGHUP-based configuration hot-reload verified with atomic swap pattern via Arc<RwLock<>>**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-17T10:50:15Z
- **Completed:** 2026-02-17T10:58:00Z
- **Tasks:** 3
- **Files verified:** 2

## Accomplishments

- Verified SIGHUP signal handler triggers reload without daemon restart
- Confirmed validate-then-swap pattern in device profile reload
- Confirmed validate-then-swap pattern in global remap reload
- Documented atomic swap semantics using Arc<RwLock<>>

## Task Commits

Each task was verified (no code changes required):

1. **Task 1: Verify SIGHUP signal handler** - VERIFIED (main.rs:230-296, 312-323)
2. **Task 2: Verify device profile reload** - VERIFIED (config.rs:717-795)
3. **Task 3: Verify global remap reload** - VERIFIED (config.rs:826-860, remap_engine.rs:260-301)

**Plan metadata:** No commits - verification-only plan

## Files Verified

- `razermapper/razermapperd/src/main.rs` - SIGHUP signal handler with nested tokio::select! for continued operation
- `razermapper/razermapperd/src/config.rs` - reload_device_profiles() and reload_remaps() with validate-then-swap
- `razermapper/razermapperd/src/remap_engine.rs` - load_config() with eager validation and atomic swap

## Verification Findings

### Task 1: SIGHUP Signal Handler (main.rs)

**Location:** Lines 230-296, 312-323

**Findings:**
1. Line 230: `tokio::signal::unix::SignalKind::hangup()` initialized
2. Lines 234-250: SIGHUP recv() branch calls reload methods
3. Lines 237-241: `reload_device_profiles()` called with error logging
4. Lines 243-249: `reload_remaps()` called with error logging
5. Lines 251-296: Nested tokio::select! ensures continued operation after SIGHUP
6. Line 312: Duplicate handler in second tokio::select! branch

**Result:** VERIFIED - SIGHUP triggers reload without daemon restart

### Task 2: Device Profile Reload (config.rs)

**Location:** Lines 717-795

**Findings:**
1. Lines 724-736: File read and YAML parse BEFORE acquiring write lock
2. Lines 739-741: KeyParser created for validation
3. Lines 744-780: Validation loop processes all profiles before swap
4. Lines 755-766: `RemapProfile::with_key_parser` validates each remap
5. Lines 770-775: Returns error immediately on invalid profile (no partial apply)
6. Lines 785-786: Atomic swap via `*config_guard = all_profiles`

**Result:** VERIFIED - Uses validate-then-swap pattern with atomic update

### Task 3: Global Remap Reload (config.rs)

**Location:** Lines 826-860

**Findings:**
1. Line 836: Calls `load_remaps()` for validation
2. Lines 839-842: Converts entries to HashMap
3. Lines 845-852: `RemapEngine.load_config()` applies validated config
4. Lines 848-852: Returns `RemapConfigError` on failure

**RemapEngine.load_config verification (remap_engine.rs:260-301):**
1. Lines 267-292: Eager validation - ALL keys parsed before storing
2. Lines 295-296: Atomic swap via `*remaps = parsed_remaps`

**Result:** VERIFIED - Validates before applying to RemapEngine

## Decisions Made

- **Existing SIGHUP implementation verified** - No changes needed; implementation already follows best practices
- **Atomic swap pattern confirmed** - Arc<RwLock<>> enables O(1) atomic pointer exchange
- **Validation-before-swap pattern confirmed** - Invalid configurations are rejected entirely, never partially applied

## Deviations from Plan

None - plan executed exactly as specified (verification-only plan).

## Issues Encountered

None - verification proceeded smoothly.

## Atomic Swap Pattern

The hot-reload implementation uses a **validate-then-swap** pattern:

```
1. Read file from disk
2. Parse YAML
3. Validate ALL entries (eager validation)
4. If validation fails -> Return error (old config unchanged)
5. If validation succeeds -> Acquire write lock
6. Atomic swap: *guard = new_config
7. Release lock
```

**Benefits:**
- All-or-nothing semantics (no partial updates)
- Old config continues working during validation
- Minimal lock time (only during swap)
- Thread-safe via Arc<RwLock<>>

## Next Phase Readiness

- Hot-reload infrastructure verified and ready
- Atomic swap pattern documented for future reference
- Ready for runtime profile switching (Plan 03-03)

---
*Phase: 03-hotplug-hotreload*
*Completed: 2026-02-17*
