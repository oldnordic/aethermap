# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-17)

**Core value:** Users can remap any input key to any other key, with remaps persisting across reboots and switchable at runtime.
**Current focus:** Milestone v1.1 - Mouse & Mixed Macros

## Current Position

**Milestone:** v1.1 Mouse & Mixed Macros
**Phase:** Not started (defining requirements)
**Status:** Gathering requirements

Last activity: 2026-02-17 — Started v1.1 milestone planning

## Milestone v1.0 Deliverables (Complete)

All 4 phases shipped:
- Phase 1: Core Remapping Engine (6 plans) — KeyParser, RemapEngine, YAML config
- Phase 2: Per-Device Profiles & IPC (6 plans) — Atomic profile switching, IPC protocol
- Phase 3: Hotplug & Hot-Reload (4 plans) — udev monitoring, SIGHUP reload
- Phase 4: Integration Testing (3 plans) — Unit tests, virtual device tests

## Known Issues

- **Macro limitation**: Current macro engine only supports keyboard events
- **Device discovery**: Shows all /dev/input/event* devices, not just input devices

## Accumulated Context

### Key Decisions (from v1.0)

- KeyParser with evdev::Key enum (type-safe)
- RemapTable type alias for atomic switching (O(1) swaps)
- YAML config for remaps (human-readable)
- udev for device monitoring (standard Linux API)
- SIGHUP for config reload (UNIX standard)
- validate-then-swap pattern (atomic updates)
- evdev::uinput for testing (CI-compatible)
- Drop trait for device cleanup (RAII, panic-safe)

## Pending Todos

- Define requirements for v1.1
- Create roadmap for mouse macro support
- Fix device discovery to only show input devices

## Session Continuity

Last session: v1.0 milestone completion
Stopped at: Starting v1.1 milestone planning
Resume files:
- All phase SUMMARY.md files from v1.0 (19 total)
- Milestone archives: .planning/milestones/v1.0-ROADMAP.md, v1.0-REQUIREMENTS.md
- MILESTONES.md

---
*Last updated: 2026-02-17 after starting v1.1 milestone*
