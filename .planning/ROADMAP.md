# Roadmap: Razermapper

## Overview

Razermapper transforms from a prototype with working device grabbing and macro recording into a production-ready input remapping daemon. The journey builds the missing RemapEngine component (core value), extends it with per-device profiles and runtime control via IPC, then hardens the system with hotplug support and comprehensive testing. Each phase delivers verifiable capabilities toward the goal: users can remap any input key to any other key, with remaps persisting across reboots and switchable at runtime.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3, 4): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Core Remapping Engine** - Build the missing RemapEngine for single-key-to-single-key remapping
- [ ] **Phase 2: Per-Device Profiles & IPC** - Enable device-specific remaps and runtime profile switching
- [ ] **Phase 3: Hotplug & Hot-Reload** - Handle device changes and runtime configuration reload
- [ ] **Phase 4: Integration Testing** - Verify end-to-end functionality with virtual devices

## Phase Details

### Phase 1: Core Remapping Engine
**Goal**: Users can remap any key to any other key via YAML configuration, with remaps applying on daemon startup
**Depends on**: Nothing (first phase)
**Requirements**: REMAP-01, REMAP-02, REMAP-04, REMAP-05, REMAP-06, REMAP-07, CONFIG-01, CONFIG-02, CONFIG-05, QUAL-01, QUAL-02, QUAL-04, QUAL-06
**Success Criteria** (what must be TRUE):
  1. User creates YAML config file with key name syntax (e.g., `KEY_A: KEY_B`), daemon loads it on startup, and pressing A outputs B
  2. Remapping modifier keys (CapsLock→Ctrl_L, Escape→Ctrl_L) works correctly
  3. Key repeat events (event value 2) are handled properly for remapped keys
  4. No stuck keys occur when device is grabbed (existing key states cleared)
  5. Modifier state synchronization prevents keys from appearing permanently held down
  6. System processes input events without panics (unwrap() calls eliminated on hot paths)
  7. Macro engine cleans up completed executions (memory leak fixed)
  8. Daemon releases device grabs gracefully on error/shutdown
**Plans**: TBD

Plans:
- [ ] 01-01: Create RemapEngine with HashMap-based key code translation
- [ ] 01-02: Integrate RemapEngine into main event loop (device → remap → macro → inject)
- [ ] 01-03: Extend ConfigManager for RemapEntry YAML parsing and validation
- [ ] 01-04: Implement key repeat handling (event value 2) and stuck key prevention
- [ ] 01-05: Fix macro execution memory leak and remove unwrap() panic risk

### Phase 2: Per-Device Profiles & IPC
**Goal**: Users can define different remap profiles per device and switch between them at runtime via GUI
**Depends on**: Phase 1
**Requirements**: REMAP-03, CONFIG-04, CONFIG-06, IPC-01, IPC-02, IPC-03, IPC-04
**Success Criteria** (what must be TRUE):
  1. User defines device-specific remaps in YAML config (e.g., per-device profiles section), daemon applies correct profile for each keyboard
  2. GUI displays currently active remap configuration for each device
  3. GUI can request activation of a specific remap profile, daemon switches to it without restart
  4. GUI can request deactivation of current remap profile, daemon stops applying those remaps
  5. Multiple named profiles coexist in configuration, user can switch between them
**Plans**: TBD

Plans:
- [ ] 02-01: Implement per-device remap profile storage and lookup
- [ ] 02-02: Add IPC protocol messages for remap configuration query and profile activation/deactivation
- [ ] 02-03: Extend GUI to display active remaps and profile switching UI

### Phase 3: Hotplug & Hot-Reload
**Goal**: System handles device hotplug without crashing and reloads configuration without daemon restart
**Depends on**: Phase 2
**Requirements**: CONFIG-03, QUAL-03
**Success Criteria** (what must be TRUE):
  1. User unplugs USB keyboard and plugs it back in, daemon continues operating and re-acquires the device
  2. User edits YAML config file and sends reload signal, daemon applies new remaps without restart
  3. Device monitoring uses inotify on /dev/input/ for hotplug detection
  4. Configuration reload validates before applying, provides clear error messages for invalid configs
**Plans**: TBD

Plans:
- [ ] 03-01: Implement inotify-based device monitoring for hotplug detection
- [ ] 03-02: Add graceful device removal handling (FD cleanup, continue operating)
- [ ] 03-03: Implement configuration hot-reload with atomic swap and validation

### Phase 4: Integration Testing
**Goal**: Remap functionality is verified by comprehensive unit and integration tests with virtual devices
**Depends on**: Phase 3
**Requirements**: QUAL-05
**Success Criteria** (what must be TRUE):
  1. Unit tests cover core remapping logic (key code lookup, modifier handling, repeat events)
  2. Integration tests use evdevil virtual devices to test end-to-end remapping
  3. Tests verify hotplug scenarios (device removal, re-acquisition)
  4. Tests verify configuration reload scenarios
  5. Test suite runs cleanly with all tests passing
**Plans**: TBD

Plans:
- [ ] 04-01: Add unit tests for RemapEngine (lookup tables, modifier state, repeat handling)
- [ ] 04-02: Add integration tests with evdevil virtual devices
- [ ] 04-03: Add tests for hotplug and configuration reload scenarios

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Core Remapping Engine | 0/5 | Not started | - |
| 2. Per-Device Profiles & IPC | 0/3 | Not started | - |
| 3. Hotplug & Hot-Reload | 0/3 | Not started | - |
| 4. Integration Testing | 0/3 | Not started | - |
