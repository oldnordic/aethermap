# Requirements: Razermapper

**Defined:** 2026-02-16
**Core Value:** Users can remap any input key to any other key, with remaps persisting across reboots and switchable at runtime.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Core Remapping

- [ ] **REMAP-01**: System remaps single input key code to single output key code (e.g., KEY_A → KEY_B)
- [ ] **REMAP-02**: System remaps modifier keys (CapsLock → Ctrl_L, Escape → Ctrl_L, etc.)
- [ ] **REMAP-03**: Remaps apply only to specified device (per-device profiles)
- [ ] **REMAP-04**: Remaps process with latency under 10ms from input event to output injection
- [ ] **REMAP-05**: System handles key repeat events (event value 2) correctly for remapped keys
- [ ] **REMAP-06**: Remaps prevent stuck keys during device grab (clear existing key states)
- [ ] **REMAP-07**: System properly handles modifier key state synchronization

### Configuration

- [ ] **CONFIG-01**: User defines remaps in YAML configuration file with key name syntax
- [ ] **CONFIG-02**: Daemon auto-loads remap configuration on startup
- [ ] **CONFIG-03**: System reloads configuration without daemon restart (hot-reload)
- [ ] **CONFIG-04**: User activates/deactivates remap profiles at runtime via IPC
- [ ] **CONFIG-05**: Configuration validates before applying (clear error messages for invalid key names)
- [ ] **CONFIG-06**: Multiple named profiles supported with easy switching

### Quality & Reliability

- [ ] **QUAL-01**: System processes input events without panics (eliminate unwrap() calls on hot paths)
- [ ] **QUAL-02**: Macro engine cleans up completed executions (fix memory leak)
- [ ] **QUAL-03**: System handles device hotplug (USB unplug/replug) without crashing
- [ ] **QUAL-04**: Remap functionality covered by unit tests
- [ ] **QUAL-05**: Remap functionality covered by integration tests with virtual devices
- [ ] **QUAL-06**: Daemon releases device grabs gracefully on error/shutdown

### IPC & Integration

- [ ] **IPC-01**: GUI can query active remap configuration
- [ ] **IPC-02**: GUI can request remap profile activation
- [ ] **IPC-03**: GUI can request remap profile deactivation
- [ ] **IPC-04**: IPC protocol supports remap configuration messages

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Advanced Remapping

- **REMAP-20**: Layer support (3-4 layers max, modifier-activated)
- **REMAP-21**: Tap-hold functionality (tap = one key, hold = modifier)
- **REMAP-22**: Key chord detection (simultaneous key combinations)

### User Interface

- **GUI-10**: Basic GUI key binding editor (text-based key selection)
- **GUI-11**: Visual key picker (press key to select)

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Application-specific remapping | Wayland security model prevents window focus detection |
| X11-specific code paths | Wayland is future; evdev/uinput works on both |
| Network remoting | Security risk; unnecessary attack surface |
| Hardware RGB/LED control | Hardware-specific; not core to remapping value |
| Key sequences (Vim-style) | Complex state machine; defer until v2+ |
| Multi-device macros | Event ordering unreliable across devices |
| Infinite layer nesting | User confusion; cognitive overhead |
| Visual GUI click-to-assign | Significant UI work; defer to v2+ |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| REMAP-01 | Phase 1 | Pending |
| REMAP-02 | Phase 1 | Pending |
| REMAP-03 | Phase 2 | Pending |
| REMAP-04 | Phase 1 | Pending |
| REMAP-05 | Phase 1 | Pending |
| REMAP-06 | Phase 1 | Pending |
| REMAP-07 | Phase 2 | Pending |
| CONFIG-01 | Phase 1 | Pending |
| CONFIG-02 | Phase 1 | Pending |
| CONFIG-03 | Phase 3 | Pending |
| CONFIG-04 | Phase 2 | Pending |
| CONFIG-05 | Phase 1 | Pending |
| CONFIG-06 | Phase 2 | Pending |
| QUAL-01 | Phase 1 | Pending |
| QUAL-02 | Phase 1 | Pending |
| QUAL-03 | Phase 4 | Pending |
| QUAL-04 | Phase 1 | Pending |
| QUAL-05 | Phase 4 | Pending |
| QUAL-06 | Phase 1 | Pending |
| IPC-01 | Phase 2 | Pending |
| IPC-02 | Phase 2 | Pending |
| IPC-03 | Phase 2 | Pending |
| IPC-04 | Phase 2 | Pending |

**Coverage:**
- v1 requirements: 21 total
- Mapped to phases: 0
- Unmapped: 21 ⚠️

---
*Requirements defined: 2026-02-16*
*Last updated: 2026-02-16 after initial definition*
