# Requirements: Razermapper

**Defined:** 2026-02-17
**Core Value:** Users can remap any input key to any other key, with remaps persisting across reboots and switchable at runtime.

## v1.0 Requirements (COMPLETE)

All v1.0 requirements have been shipped. See `.planning/milestones/v1.0-REQUIREMENTS.md` for archive.

## v1.1 Requirements (COMPLETE)

All v1.1 requirements have been shipped.

## v1.2 Requirements (IN PROGRESS)

See `.planning/milestones/v1.2-REQUIREMENTS.md` for v1.2 Azeron Cyborg support requirements.

## v1.3 Requirements

Requirements for Wayland compatibility and advanced analog processing. Each maps to roadmap phases 13-16.

### Wayland Integration

- [ ] **WAYL-01**: Daemon detects window focus changes on Wayland compositors
- [ ] **WAYL-02**: Window focus triggers automatic profile switching based on app_id
- [ ] **WAYL-03**: User can configure app-to-profile mapping rules in YAML
- [ ] **WAYL-04**: Global keyboard shortcuts for profile switching work on Wayland
- [ ] **WAYL-05**: GUI prompts for Wayland portal permissions on first use
- [ ] **WAYL-06**: System falls back to manual switching when portal unavailable

### Analog Processing Modes

- [ ] **ANLG-01**: User can configure analog stick to output gamepad events (Xbox 360 emulation)
- [ ] **ANLG-02**: User can configure analog stick to output WASD keys (8-way directional)
- [ ] **ANLG-03**: User can configure analog stick to control mouse movement
- [ ] **ANLG-04**: User can configure analog stick for camera rotation or scrolling
- [ ] **ANLG-05**: Analog mode switchable via GUI and IPC
- [ ] **ANLG-06**: Each layer can have different analog mode and calibration

### Analog Calibration

- [ ] **CAL-01**: User can configure deadzone (circular or square, adjustable radius)
- [ ] **CAL-02**: User can configure sensitivity curve (linear, quadratic, exponential)
- [ ] **CAL-03**: User can configure sensitivity multiplier
- [ ] **CAL-04**: User can configure axis range limits (min/max values)
- [ ] **CAL-05**: User can invert axis (X, Y independently)
- [ ] **CAL-06**: GUI provides visual preview of calibration settings

### GUI Integration

- [ ] **GUI-01**: Calibration screen shows real-time analog input visualization
- [ ] **GUI-02**: User can select analog mode from dropdown (Gamepad, WASD, Mouse, Camera)
- [ ] **GUI-03**: Deadzone control includes slider and shape selector
- [ ] **GUI-04**: Sensitivity control includes curve selector and multiplier slider
- [ ] **GUI-05**: Range controls include min/max sliders and inversion checkboxes
- [ ] **GUI-06**: All calibration changes apply immediately with live preview

### Quality & Reliability

- [ ] **QUAL-01**: Gamepad emulation works with standard Linux gamepad drivers
- [ ] **QUAL-02**: WASD mode correctly outputs diagonal key combinations
- [ ] **QUAL-03**: Mouse mode velocity is smooth and responsive
- [ ] **QUAL-04**: Camera mode scroll/key events have appropriate repeat rate
- [ ] **QUAL-05**: Portal integration handles permission errors gracefully
- [ ] **QUAL-06**: Global hotkeys work across all major Wayland compositors

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Advanced Features

- **ADV-01**: Per-application profile switching based on window title (beyond app_id)
- **ADV-02**: Advanced analog curves (Bezier, custom points)
- **ADV-03**: Multi-monitor aware mouse positioning
- **ADV-04**: Haptic feedback for analog stick
- **ADV-05**: Gyro/accelerometer support for motion controls
- **ADV-06**: Full gamepad button remapping (beyond analog pass-through)

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| X11 support | Project is Wayland-focused (as stated in PROJECT.md constraints) |
| Window title-based switching | Wayland security model prevents title access beyond app_id |
| Custom Bezier curves | Advanced feature, standard curves sufficient for v1.3 |
| Hardware-specific hacks | Generic solutions preferred for maintainability |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

### v1.3 Requirement Mapping

| Requirement | Phase | Status |
|-------------|-------|--------|
| WAYL-01 | Phase 13 | Pending |
| WAYL-02 | Phase 13 | Pending |
| WAYL-03 | Phase 13 | Pending |
| WAYL-04 | Phase 13 | Pending |
| WAYL-05 | Phase 13 | Pending |
| WAYL-06 | Phase 13 | Pending |
| ANLG-01 | Phase 14 | Pending |
| ANLG-02 | Phase 15 | Pending |
| ANLG-03 | Phase 15 | Pending |
| ANLG-04 | Phase 15 | Pending |
| ANLG-05 | Phase 15 | Pending |
| ANLG-06 | Phase 15 | Pending |
| CAL-01 | Phase 16 | Pending |
| CAL-02 | Phase 16 | Pending |
| CAL-03 | Phase 16 | Pending |
| CAL-04 | Phase 16 | Pending |
| CAL-05 | Phase 16 | Pending |
| CAL-06 | Phase 16 | Pending |
| GUI-01 | Phase 16 | Pending |
| GUI-02 | Phase 16 | Pending |
| GUI-03 | Phase 16 | Pending |
| GUI-04 | Phase 16 | Pending |
| GUI-05 | Phase 16 | Pending |
| GUI-06 | Phase 16 | Pending |
| QUAL-01 | Phase 14 | Pending |
| QUAL-02 | Phase 15 | Pending |
| QUAL-03 | Phase 15 | Pending |
| QUAL-04 | Phase 15 | Pending |
| QUAL-05 | Phase 13 | Pending |
| QUAL-06 | Phase 13 | Pending |

**Coverage:**
- v1.3 requirements: 33 total
- Mapped to phases: 33
- Unmapped: 0

---
*Requirements defined: 2026-02-17*
*Last updated: 2026-02-18 after v1.3 milestone definition*
