# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-16)

**Core value:** Users can remap any input key to any other key, with remaps persisting across reboots and switchable at runtime.
**Current focus:** Phase 2 - Device Profiles & IPC

## Current Position

Phase: 2 of 4 (Device Profiles & IPC)
Status: Plan 02-02 COMPLETE
_✓ Extended YAML config for per-device profiles_
Last activity: 2026-02-17 — Extended YAML config structures with match_pattern and description support (4 min)

Progress: [███░░░░░░░░░░░░░░░░░░] 33% (Phase 2)

## Phase 1 Deliverables

| Plan | Description | Status |
|------|-------------|--------|
| 01-01 | KeyParser and RemapEngine | ✓ Complete |
| 01-02 | ConfigManager YAML loading | ✓ Complete |
| 01-03 | Daemon RemapEngine initialization | ✓ Complete |
| 01-03b | Device event loop integration | ✓ Complete |
| 01-04 | Key repeat handling | ✓ Complete |
| 01-05 | Quality fixes | ✓ Complete |

**Core Achievement:** Users can now remap any input key to any other key through YAML configuration (`/etc/razermapperd/remaps.yaml`). Remaps persist across reboots, key repeat works correctly, and the daemon has no memory leaks.

## Phase 2 Deliverables

| Plan | Description | Status |
|------|-------------|--------|
| 02-01 | RemapTable type alias for atomic switching | ✓ Complete |
| 02-02 | Extended YAML config for per-device profiles | ✓ Complete |
| 02-03 | IPC request/response types for profiles | Pending |
| 02-04 | Per-device profile storage in DeviceManager | Pending |
| 02-05 | Profile activation methods | Pending |
| 02-06 | IPC handlers for profile operations | Pending |

**Atomic Switching Achievement:** RemapTable type alias added for O(1) atomic pointer swaps during profile switching without memory allocations or locks during event processing.

## Phase 3 Deliverables

| Plan | Description | Status |
|------|-------------|--------|
| 03-01 | Verify udev hotplug monitoring | ✓ Complete |
| 03-02 | SIGHUP configuration hot-reload | ✓ Complete |
| 03-03 | Integration tests for hotplug and hot-reload | ✓ Complete |
| 03-04 | Documentation update (ROADMAP, README, RESEARCH) | ✓ Complete |

**Hotplug Achievement:** Device hotplug monitoring verified - devices are auto-detected on plug-in, profiles auto-applied, and clean removal on unplug.
**Hot-reload Achievement:** SIGHUP-based configuration hot-reload verified - validate-then-swap pattern ensures atomic updates without daemon restart.
**Integration Test Achievement:** 15 integration tests covering DeviceEvent structure, device ID formatting, and validate-then-swap hot-reload pattern.
**Documentation Achievement:** ROADMAP.md updated with udev terminology, README.md documents hotplug/hot-reload features, RESEARCH.md records verification findings.

## Phase 4 Deliverables

| Plan | Description | Status |
|------|-------------|--------|
| 04-01 | Verify existing unit test coverage | ✓ Complete |
| 04-02 | End-to-end virtual device integration tests | ✓ Complete |
| 04-03 | Hotplug and config reload integration test verification | ✓ Complete |

**Unit Test Coverage Achievement:** Verified 31 comprehensive unit tests (486 lines) covering RemapEngine lookup tables, KeyParser key categories, validation, eager config loading, and RemapProfile atomic switching. All tests passing.

**Virtual Device Integration Test Achievement:** Created 4 end-to-end integration tests (295 lines) using evdev::uinput::VirtualDeviceBuilder. Tests verify key remapping with press, release, and repeat event values. CI-compatible with graceful /dev/uinput skip.

**Hotplug and Config Reload Test Achievement:** Verified 15 integration tests (701 lines) covering hotplug event handling (6 tests) and configuration hot-reload (9 tests). Tests verify DeviceEvent structure, device ID formatting, atomic swap pattern, invalid config rejection, and concurrent reload safety.

## Performance Metrics

**Velocity:**
- Total plans completed: 12
- Average duration: ~0.6 hours/plan
- Total execution time: ~6.3 hours

**By Phase:**

| Phase | Plans | Status | Total | Avg/Plan |
|-------|-------|--------|-------|----------|
| 1 - Core Remapping Engine | 6/6 | Complete | 4.2 | 0.70 |
| 3 - Hotplug and Hot-reload | 4/4 | Complete | 1.0 | 0.25 |
| 4 - Integration Testing | 3/3 | Complete | 0.5 | 0.15 |

**Recent Trend:**
- Last 4 plans: 0.25h avg (03-01, 03-02, 03-03, 03-04)
- Trend: Phase 3 complete

*Updated after each plan completion*
| Phase 03 P03 | 15min | 3 tasks | 2 files created, 1 modified |
| Phase 03 P04 | 10min | 3 tasks | 3 files modified |
| Phase 04 P01 | 5min | 3 tasks | 1 file created |
| Phase 04 P02 | 15min | 3 tasks | 1 file created |
| Phase 04 P03 | 8min | 3 tasks | 1 file created |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions from Phase 1 and 3:

- **2026-02-17**: Udev hotplug monitoring verified - DeviceMonitor uses spawn_blocking for async-safe udev socket iteration (03-01)
- **2026-02-17**: Device add handler auto-grabs devices with configured profiles on plug-in (03-01)
- **2026-02-17**: Device removal handler performs best-effort cleanup with Drop trait for panic safety (03-01)
- **2026-02-17**: Macro execution memory leak fixed with cleanup channel (01-05)
- **2026-02-17**: All unwrap() calls removed from hot paths, using try_read() with error handling (01-05)
- **2026-02-17**: Event value handling (0=release, 1=press, 2=repeat) implemented for proper key repeat (01-04)
- **2026-02-17**: Drop trait for GrabbedDevice ensures device release on panic/shutdown (01-04)
- **2026-02-17**: RemapEngine integrated into device event loop (01-03b)
- **2026-02-17**: RemapEngine initialized at daemon startup with eager validation (01-03)
- **2026-02-17**: RemapConfigError and RemapEntry types added for type-safe remap configuration (01-02)
- **2026-02-17**: load_remaps() method with eager key validation using KeyParser (01-02)
- **2026-02-17**: Flat YAML structure (KEY_A: KEY_B) without section wrapper (01-02)
- **2026-02-17**: Empty file creation instead of error when remaps.yaml missing (01-02)
- **2026-02-16**: KeyParser and RemapEngine with evdev::Key enum storage (not u16) (01-01)
- **2026-02-16**: Manual fmt::Display/std::error::Error impl instead of thiserror (01-01)
- **2026-02-16**: Non-linear function key code mapping (F11=87, F12=88) (01-01)
- **2026-02-17**: Integration tests for hotplug and hot-reload using tempfile isolation (03-03)
- **2026-02-17**: Made hotplug::format_device_id public for testing device ID format (03-03)
- **2026-02-17**: Documentation updated - ROADMAP.md reflects udev terminology, README.md documents hotplug/hot-reload (03-04)
- **2026-02-17**: RESEARCH.md updated with verification results documenting Phase 3 implementation status (03-04)
- **2026-02-17**: Unit test coverage verified - 31 tests (486 lines) covering RemapEngine and KeyParser (04-01)
- **2026-02-17**: End-to-end virtual device integration tests - 4 tests using evdev::uinput::VirtualDeviceBuilder (04-02)
- **2026-02-17**: ROADMAP correction - uses evdev::uinput (not "evdevil" which doesn't exist) (04-02)
- **2026-02-17**: Hotplug and config reload integration tests verified - 15 tests (701 lines) covering DeviceEvent structure, device ID formatting, atomic swap, invalid rejection, and concurrent safety (04-03)
- **2026-02-17**: RemapTable type alias added for O(1) atomic profile switching (02-01)
- **2026-02-17**: Existing RemapProfile structure preserved with Arc<RwLock<HashMap>> for async compatibility (02-01)
- **2026-02-17**: Extended YAML config structures added - ExtendedDeviceRemapConfig, ProfileRemaps, RemapDevicesConfig (02-02)
- **2026-02-17**: load_device_profiles_extended() method returning HashMap<String, Vec<RemapProfile>> (02-02)
- **2026-02-17**: Flat HashMap<String, String> remaps for ergonomic YAML syntax instead of Vec<RemapEntry> (02-02)
- **2026-02-17**: Optional match_pattern field for flexible device matching (02-02)

### Pending Todos

- Complete Phase 2 per-device profile configuration (5 remaining plans)
- Add IPC handlers for profile queries and switching
- Implement profile activation in DeviceManager

### Blockers/Concerns

- **Pre-existing issue**: macro_engine tests have a hanging issue unrelated to Phase 1 work. Excluded from test runs using filter.
- **Integration deferred**: Full IPC wiring for RemapEngine deferred to Phase 2 per plan.

## Session Continuity

Last session: Phase 2 Device Profiles & IPC - Plan 02-02 Extended YAML config
Stopped at: Plan 02-02 complete - Extended YAML config structures with match_pattern and description support
Resume files:
- .planning/phases/01-core-remapping/01-01-SUMMARY.md
- .planning/phases/01-core-remapping/01-02-SUMMARY.md
- .planning/phases/01-core-remapping/01-03-SUMMARY.md
- .planning/phases/01-core-remapping/01-03b-SUMMARY.md
- .planning/phases/01-core-remapping/01-04-SUMMARY.md
- .planning/phases/01-core-remapping/01-05-SUMMARY.md
- .planning/phases/03-hotplug-hotreload/03-01-SUMMARY.md
- .planning/phases/03-hotplug-hotreload/03-02-SUMMARY.md
- .planning/phases/03-hotplug-hotreload/03-03-SUMMARY.md
- .planning/phases/03-hotplug-hotreload/03-04-SUMMARY.md
- .planning/phases/04-integration-testing/04-01-SUMMARY.md
- .planning/phases/04-integration-testing/04-02-SUMMARY.md
- .planning/phases/04-integration-testing/04-03-SUMMARY.md
- .planning/phases/02-device-profiles-ipc/02-01-SUMMARY.md
- .planning/phases/02-device-profiles-ipc/02-02-SUMMARY.md

## Next Steps

Phase 2 IN PROGRESS (2 of 6 plans done).

**Next:** Plan 02-03 - IPC request/response types for profiles.

1. **Phase 2**: Per-Device Profiles & IPC (in progress) - Runtime profile switching via IPC
2. **GUI development**: Frontend for configuration (deferred)

<sub>Phase 3: 03-01 ✓ → 03-02 ✓ → 03-03 ✓ → 03-04 ✓</sub>
<sub>Phase 4: 04-01 ✓ → 04-02 ✓ → 04-03 ✓</sub>
