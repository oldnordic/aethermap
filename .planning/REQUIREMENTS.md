# Requirements: Razermapper - v1.1

**Defined:** 2026-02-17
**Core Value:** Users can remap any input key to any other key, with remaps persisting across reboots and switchable at runtime.

## v1.1 Requirements

Requirements for mouse and mixed macro support. Each maps to roadmap phases.

### Device Discovery & Filtering

- [ ] **DISC-01**: System only lists devices with input capabilities (keyboard, mouse, gamepad) in GUI
- [ ] **DISC-02**: Device list shows type indicators (keyboard icon, mouse icon, etc.)
- [ ] **DISC-03**: Hybrid devices (e.g., keyboard + touchpad) display multiple type indicators
- [ ] **DISC-04**: System excludes non-input devices (sensors, power buttons, audio devices) from device list

### Mouse Event Recording

- [ ] **MOUSE-01**: User can record mouse button clicks (left, right, middle) in macros
- [ ] **MOUSE-02**: User can record mouse scroll wheel events (up/down) in macros
- [ ] **MOUSE-03**: User can record mouse movement (relative dx, dy) in macros
- [ ] **MOUSE-04**: User can create macros combining keyboard and mouse events in single recording
- [ ] **MOUSE-05**: System applies movement batching for high-DPI mice during recording

### GUI Integration

- [ ] **GUI-01**: User can click mouse buttons during macro recording to capture them
- [ ] **GUI-02**: User can select mouse buttons from dropdown list (manual selection)
- [ ] **GUI-03**: Macro editor visualizes mouse events distinctly from keyboard events

### Quality & Reliability

- [ ] **QUAL-01**: Mouse macros execute with correct event ordering (preserves sequence)
- [ ] **QUAL-02**: Macro playback handles mixed keyboard+mouse events without desync
- [ ] **QUAL-03**: System validates uinput EV_REL capabilities before macro playback

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Advanced Mouse Features

- **MOUSE-20**: Per-device DPI normalization for mouse movement
- **MOUSE-21**: Touchpad vs mouse event distinction
- **MOUSE-22**: Absolute axis device support (tablets, touchpads)
- **MOUSE-23**: Button hold vs click distinction in macros
- **MOUSE-24**: Force feedback support in macros

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Mouse button remapping (not macros) | Different feature; requires RemapEngine refactoring |
| Absolute position mapping | Requires different approach; relative is standard for macros |
| Gesture recognition | Complex state machine; defer to later milestone |
| Multi-device mouse coordination | Event ordering unreliable across devices |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DISC-01 | Phase 5 | Pending |
| DISC-02 | Phase 5 | Pending |
| DISC-03 | Phase 5 | Pending |
| DISC-04 | Phase 5 | Pending |
| MOUSE-01 | Phase 6 | Pending |
| MOUSE-02 | Phase 6 | Pending |
| MOUSE-03 | Phase 6 | Pending |
| MOUSE-04 | Phase 6 | Pending |
| MOUSE-05 | Phase 6 | Pending |
| GUI-01 | Phase 7 | Pending |
| GUI-02 | Phase 7 | Pending |
| GUI-03 | Phase 7 | Pending |
| QUAL-01 | Phase 6 | Pending |
| QUAL-02 | Phase 6 | Pending |
| QUAL-03 | Phase 6 | Pending |

**Coverage:**
- v1.1 requirements: 15 total
- Mapped to phases: 0 (roadmap not created yet)
- Unmapped: 15 ⚠️

---
*Requirements defined: 2026-02-17*
*Last updated: 2026-02-17 after initial definition*
