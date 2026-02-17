# Roadmap: Razermapper

## Milestones

- ✅ **v1.0 MVP** — Phases 1-4 (shipped 2026-02-17)
- 📋 **v1.1 Mouse & Mixed Macros** — Phases 5-8
- 📋 **v1.2 Azeron Support** — Phases 9-12

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-4) — SHIPPED 2026-02-17</summary>

- [x] Phase 1: Core Remapping Engine (6/6 plans) — completed 2026-02-17
- [x] Phase 2: Per-Device Profiles & IPC (6/6 plans) — completed 2026-02-17
- [x] Phase 3: Hotplug & Hot-Reload (4/4 plans) — completed 2026-02-17
- [x] Phase 4: Integration Testing (3/3 plans) — completed 2026-02-17

**Details archived to:** `.planning/milestones/v1.0-ROADMAP.md`

</details>

<details>
<summary>📋 v1.1 Mouse & Mixed Macros (Phases 5-8)</summary>

**Goal:** Add mouse button remapping and mixed mouse/keyboard macros

**Depends on:** Phase 4

**Status:** In progress

- [x] Phase 5: Device Filtering & Discovery (3/3 plans) — completed 2026-02-17
- [x] Phase 6: Mouse Event Reading (4/4 plans) — completed 2026-02-17
- [x] Phase 7: Mouse Macro Verification (4/4 plans) — completed 2026-02-17
- [x] Phase 8: GUI Integration (4/4 plans) — completed 2026-02-17

**Plans:**
- [x] 05-01: Device capability detection
- [x] 05-02: Device type filtering
- [x] 05-03: Device discovery optimization
- [x] 06-01: Mouse button event capture
- [x] 06-02: Relative axis event handling
- [x] 06-03: Mouse event encoding
- [x] 06-04: Mouse event macro integration
- [x] 07-01: Mouse-only macro recording
- [x] 07-02: Mixed input macro recording
- [x] 07-03: Macro execution verification
- [x] 07-04: Macro timing verification
- [x] 08-01: Mouse picker UI
- [x] 08-02: Macro editor enhancements
- [x] 08-03: Macro list view
- [x] 08-04: Macro testing interface

</details>

<details>
<summary>📋 v1.2 Azeron Support (Phases 9-12)</summary>

**Goal:** Full support for Azeron Cyborg keypad with analog stick, hat switch, joystick buttons, layers, and LED control

**Depends on:** Phase 8

**Status:** In progress

- [x] Phase 9: Device Detection and Basic Input (7/7 plans) — completed 2026-02-17
- [ ] Phase 10: Layer System and State Management (0/8 plans) — **Ready to plan**
- [ ] Phase 11: Analog Processing and Calibration (0/8 plans) — Not started
- [ ] Phase 12: LED Control (0/8 plans) — Not started

**Phase 9 Plans (Complete):**
- [x] 09-01: Azeron device detection
- [x] 09-02: Joystick button remapping
- [x] 09-03: Hat switch remapping
- [x] 09-04: Profile persistence
- [x] 09-05: IPC capability queries
- [x] 09-06: GUI keypad layout

**Phase 10 Plans (Layer System):**
- [ ] 10-01: LayerManager component with per-device layer stack
- [ ] 10-02: Hold mode activation (layer active while modifier held)
- [ ] 10-03: Toggle mode activation (press on, press off)
- [ ] 10-04: Layer stack composition for overlapping shift keys
- [ ] 10-05: RemapEngine layer-aware lookups with fallback
- [ ] 10-06: Layer state persistence across profile switches
- [ ] 10-07: IPC protocol for layer management (query, activate, configure)
- [ ] 10-08: GUI active layer display and configuration

**Phase 11 Plans (Analog Processing):**
- [ ] 11-01: Analog deadzone configuration
- [ ] 11-02: Sensitivity curves and scaling
- [ ] 11-03: D-pad mode for analog stick
- [ ] 11-04: Analog event remapping
- [ ] 11-05: Per-layer analog calibration
- [ ] 11-06: Analog macro recording
- [ ] 11-07: IPC analog config
- [ ] 11-08: GUI analog calibration UI

**Phase 12 Plans (LED Control):**
- [ ] 12-01: HID API integration for LED control
- [ ] 12-02: LED zone enumeration
- [ ] 12-03: Color set operation
- [ ] 12-04: Per-layer LED colors
- [ ] 12-05: LED effect patterns
- [ ] 12-06: LED state persistence
- [ ] 12-07: IPC LED protocol
- [ ] 12-08: GUI LED color picker

</details>

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Core Remapping Engine | v1.0 | 6/6 | Complete | 2026-02-17 |
| 2. Per-Device Profiles & IPC | v1.0 | 6/6 | Complete | 2026-02-17 |
| 3. Hotplug & Hot-Reload | v1.0 | 4/4 | Complete | 2026-02-17 |
| 4. Integration Testing | v1.0 | 3/3 | Complete | 2026-02-17 |
| 5. Device Filtering & Discovery | v1.1 | 3/3 | Complete | 2026-02-17 |
| 6. Mouse Event Reading | v1.1 | 4/4 | Complete | 2026-02-17 |
| 7. Mouse Macro Verification | v1.1 | 4/4 | Complete | 2026-02-17 |
| 8. GUI Integration | v1.1 | 4/4 | Complete | 2026-02-17 |
| 9. Device Detection and Basic Input | v1.2 | 7/7 | Complete | 2026-02-17 |
| 10. Layer System and State Management | v1.2 | 0/8 | Ready to plan | - |
| 11. Analog Processing and Calibration | v1.2 | 0/8 | Not started | - |
| 12. LED Control | v1.2 | 0/8 | Not started | - |

**Overall Progress:** 41/60 plans complete (68%)

---

*See `.planning/MILESTONES.md` for shipped release history*
