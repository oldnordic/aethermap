# Project State

**Last Updated:** 2026-02-17

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-17)

**Core value:** Remap any input key with persist & switch at runtime
**Current focus:** Phase 9 - Device Detection and Basic Input

## Current Position

Phase: 9 of 12 (Device Detection and Basic Input)
Plan: 0 of 7
Status: Ready to plan
Last activity: 2026-02-17 — v1.2 Azeron Cyborg Support roadmap created

Progress:
```
v1.0 (Phases 1-4): [========================================] 100%
v1.1 (Phases 5-8): [========================================] 100%
v1.2 (Phases 9-12): [....................] 0% (0/31 plans)
Overall: [=======================================...] 52% (27/52 plans)
```

## Performance Metrics

**Velocity:**
- Total plans completed: 27
- Average duration: ~5-8 minutes per plan
- Total execution time: ~3 hours

**By Phase:**

| Phase | Plans | Status |
|-------|-------|--------|
| 1. Remap Engine | 3 | Complete |
| 2. Device Profiles & IPC | 6 | Complete |
| 3. Hotplug & Hot-Reload | 3 | Complete |
| 4. GUI Enhancements | TBD | Complete |
| 5. Device Filtering & Discovery | 3 | Complete |
| 6. Mouse Event Reading | 4 | Complete |
| 7. Mouse Macro Verification | 4 | Complete |
| 8. GUI Integration | 4 | Complete |
| 9. Device Detection and Basic Input | 7 | Not started |
| 10. Layer System and State Management | 8 | Not started |
| 11. Analog Processing and Calibration | 8 | Not started |
| 12. LED Control | 8 | Not started |

## Accumulated Context

### Decisions

**v1.0 Decisions:**
- RemapEngine uses Arc<RwLock<HashMap<Key, Key>>> for atomic profile switching
- Device profiles use vendor:product hex format for device identification
- SIGHUP handler for configuration hot-reload with nested tokio::select! pattern

**v1.1 Implementation Decisions:**
- Device type detection via capability queries (supported_keys, supported_relative_axes)
- Mouse events use high key codes (60000-60003) to distinguish from keyboard events
- REL_WHEEL_HI_RES events filtered to prevent double-speed scroll
- evdev 0.12.2 uses `RelAxis` not `RelativeAxis` as InputEventKind variant
- GUI subscribes to iced mouse events during recording for visual feedback

**v1.2 Roadmap Decisions:**
- Azeron support split into 4 phases (Device Detection, Layers, Analog, LED)
- hidapi 2.4 added as optional dependency for LED control
- LayerManager component for modifier-based layer activation
- AnalogProcessor for deadzone, sensitivity curves, D-pad emulation
- LED protocol research deferred to Phase 12 planning

### Pending Todos

None.

### Blockers/Concerns

**Phase 12 Research Flag:**
- Azeron HID LED protocol requires USB packet capture during planning
- Consider using official software for LED setup initially

**Phase 9 Verification:**
- Azeron VID/PID values (0x32b6:0x12f7) need verification with actual hardware

## Session Continuity

Last session: 2026-02-17
Stopped at: Roadmap created for v1.2 Azeron Cyborg Support
Resume file: None

**Next step:** `/gsd:plan-phase 9` to begin detailed planning for Device Detection and Basic Input
