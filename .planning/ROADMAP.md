# Roadmap: Razermapper

## Milestones

- ✅ **v1.0 MVP** — Phases 1-4 (shipped 2026-02-17)
- ✅ **v1.1 Mouse & Mixed Macros** — Phases 5-8 (shipped 2026-02-17)
- 🚧 **v1.2 Azeron Support** — Phases 9-12 (in progress)
- 📋 **v1.3 Wayland & Analog Advanced** — Phases 13-16 (planned)

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
<summary>✅ v1.1 Mouse & Mixed Macros (Phases 5-8) — SHIPPED 2026-02-17</summary>

**Goal:** Add mouse button remapping and mixed mouse/keyboard macros

**Depends on:** Phase 4

**Status:** Complete

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
<summary>🚧 v1.2 Azeron Support (Phases 9-12) — IN PROGRESS</summary>

**Goal:** Full support for Azeron Cyborg keypad with analog stick, hat switch, joystick buttons, layers, and LED control

**Depends on:** Phase 8

**Status:** In progress (Phase 10 complete, Phases 11-12 pending)

- [x] Phase 9: Device Detection and Basic Input (7/7 plans) — completed 2026-02-17
- [x] Phase 10: Layer System and State Management (8/8 plans) — completed 2026-02-17
- [ ] Phase 11: Analog Processing and Calibration (8/8 plans) — Ready to execute
- [ ] Phase 12: LED Control (0/8 plans) — Not started

**Phase 9 Plans (Complete):**
- [x] 09-01: Azeron device detection
- [x] 09-02: Joystick button remapping
- [x] 09-03: Hat switch remapping
- [x] 09-04: Profile persistence
- [x] 09-05: IPC capability queries
- [x] 09-06: GUI keypad layout

**Phase 10 Plans (Layer System - Complete):**
- [x] 10-01: LayerManager component with per-device layer stack
- [x] 10-02: Hold mode activation (layer active while modifier held)
- [x] 10-03: Toggle mode activation (press on, press off)
- [x] 10-04: Layer stack composition for overlapping shift keys
- [x] 10-05: RemapEngine layer-aware lookups with fallback
- [x] 10-06: Layer state persistence across profile switches
- [x] 10-07: IPC protocol for layer management (query, activate, configure)
- [x] 10-08: GUI active layer display and configuration

**Phase 11 Plans (Analog Processing):**
- [ ] 11-01: Calibration types (AnalogCalibration, DeadzoneShape, SensitivityCurve)
- [ ] 11-02: AnalogProcessor with deadzone, sensitivity, range pipeline
- [ ] 11-03: D-pad mode for 8-way directional output
- [ ] 11-04: Analog event integration in device loop
- [ ] 11-05: Per-layer calibration in YAML config
- [ ] 11-06: Analog event macro recording
- [ ] 11-07: IPC protocol for runtime calibration
- [ ] 11-08: GUI calibration UI with live preview

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

### 📋 v1.3 Wayland & Analog Advanced (Phases 13-16) — PLANNED

**Milestone Goal:** Full Wayland compatibility with portal-based auto-profile switching, global hotkeys, and advanced analog stick modes (gamepad, WASD, mouse, camera) with full calibration GUI

**Depends on:** Phase 10 (Layer System), can proceed in parallel with Phases 11-12

#### Phase 13: Wayland Portal Integration and Global Hotkeys
**Goal:** Auto-profile switching based on window focus and global hotkeys that work on Wayland
**Depends on:** Phase 10 (Layer Manager for profile switching infrastructure)
**Requirements:** WAYL-01, WAYL-02, WAYL-03, WAYL-04, WAYL-05, WAYL-06, QUAL-05, QUAL-06
**Success Criteria:**
  1. GUI detects window focus changes via xdg-desktop-portal
  2. Focus changes trigger profile switches in daemon based on app_id rules
  3. Global hotkeys (Ctrl+Alt+Shift+Number) work on Wayland compositors
  4. Portal permission errors handled gracefully with fallback to manual switching
  5. Auto-switch rules configurable in YAML (app_id to profile mapping)
**Plans:** 6 plans

Plans:
- [ ] 13-01: Add ashpd dependency to GUI for portal integration
- [ ] 13-02: Create FocusTracker component for window focus detection
- [ ] 13-03: Add IPC protocol for focus events (app_id, window_title)
- [ ] 13-04: Create AutoProfileSwitcher in daemon matching focus to profiles
- [ ] 13-05: Implement global hotkey detection at daemon level
- [ ] 13-06: Add auto-switch rules to YAML config and GUI

#### Phase 14: Gamepad Emulation Mode
**Goal:** Analog stick outputs Xbox 360 compatible gamepad events
**Depends on:** Phase 11 (AnalogProcessor foundation)
**Requirements:** ANLG-01, ANLG-06, QUAL-01
**Success Criteria:**
  1. Analog stick position outputs to virtual Xbox 360 gamepad device
  2. Games recognize device as standard gamepad via Linux input subsystem
  3. Calibration (deadzone, sensitivity, range) affects gamepad output
  4. Per-layer gamepad mode configuration supported
**Plans:** 6 plans

Plans:
- [x] 14-01: Create virtual Xbox 360 gamepad device via uinput
- [x] 14-02: Add AnalogMode::Gamepad to AnalogProcessor
- [x] 14-03: Implement analog-to-gamepad event conversion
- [x] 14-04: Add gamepad mode to layer configuration
- [x] 14-05: Integrate gamepad output with calibration pipeline
- [x] 14-06: Unit tests for gamepad mode and calibration

#### Phase 15: WASD, Mouse, and Camera Modes
**Goal:** Analog stick outputs directional keys, mouse movement, or scroll/camera events
**Depends on:** Phase 14 (gamepad mode provides foundation)
**Requirements:** ANLG-02, ANLG-03, ANLG-04, ANLG-05, ANLG-06, QUAL-02, QUAL-03, QUAL-04
**Success Criteria:**
  1. WASD mode outputs 8-way directional keys with diagonals (W+A, etc.)
  2. Mouse mode moves cursor with velocity proportional to stick deflection
  3. Camera mode emits scroll events or key repeats for rotation
  4. Mode switchable via GUI and IPC, per-layer configurable
  5. Smooth transitions between analog directions
**Plans:** 10 plans

Plans:
- [x] 15-01: Add AnalogMode::WASD with 8-way sector detection
- [x] 15-02: Implement WASD key output with diagonal combinations
- [x] 15-03: Add AnalogMode::Mouse with velocity-based movement
- [x] 15-04: Implement mouse event output with acceleration curve
- [x] 15-05: Add AnalogMode::Camera with scroll/key repeat options
- [x] 15-06: Implement camera mode output (configurable axis mapping)
- [x] 15-07: Add mode switching via IPC protocol
- [x] 15-08: Unit tests for all analog modes
- [ ] 15-09: Fix GUI mode persistence (gap closure)
- [ ] 15-10: Implement WASD key state tracking (gap closure)

#### Phase 16: Calibration GUI
**Goal:** Full GUI for analog calibration with live preview and visual feedback
**Depends on:** Phase 15 (all modes implemented)
**Requirements:** CAL-01, CAL-02, CAL-03, CAL-04, CAL-05, CAL-06, GUI-01, GUI-02, GUI-03, GUI-04, GUI-05, GUI-06
**Success Criteria:**
  1. Calibration screen shows real-time analog input visualization (stick position)
  2. Deadzone control with slider and shape selector (circular/square)
  3. Sensitivity control with curve selector (linear/quadratic/exponential) and multiplier
  4. Range controls with min/max sliders and inversion checkboxes
  5. All changes apply immediately with live preview
  6. Calibration persists to device profile YAML
**Plans:** 8 plans

Plans:
- [ ] 16-01: Create calibration screen layout with mode selector
- [ ] 16-02: Implement real-time analog input visualization widget
- [ ] 16-03: Add deadzone controls (slider, shape selector, visual preview)
- [ ] 16-04: Add sensitivity controls (curve selector, multiplier slider, curve graph)
- [ ] 16-05: Add range controls (min/max sliders, inversion checkboxes)
- [ ] 16-06: Implement live preview with immediate application
- [ ] 16-07: Add IPC protocol for calibration read/write
- [ ] 16-08: Integrate calibration with device profile persistence

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
| 10. Layer System and State Management | v1.2 | 8/8 | Complete | 2026-02-17 |
| 11. Analog Processing and Calibration | v1.2 | 8/8 | Complete | 2026-02-19 |
| 12. LED Control | v1.2 | 0/8 | Not started | - |
| 13. Wayland Portal Integration | v1.3 | 6/6 | Complete | 2026-02-19 |
| 14. Gamepad Emulation Mode | v1.3 | 6/6 | Complete | 2026-02-19 |
| 15. WASD, Mouse, and Camera Modes | v1.3 | 8/10 | In Progress | 2026-02-19 |
| 16. Calibration GUI | v1.3 | 0/8 | Not started | - |

**Overall Progress:** 79/92 plans complete (86%)

---

*See `.planning/MILESTONES.md` for shipped release history*
