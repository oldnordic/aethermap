# Razermapper

## What This Is

A Wayland-compatible input device remapper for Linux. The daemon runs as root to access `/dev/input/*` and `/dev/uinput`, creating virtual devices that emit remapped events. Users configure key remaps through YAML profiles and/or a GUI client. The project provides production-ready key remapping with hotplug support and runtime profile switching.

## Core Value

**Users can remap any input key to any other key, with remaps persisting across reboots and switchable at runtime.**

If key remapping doesn't work reliably, nothing else matters.

## Requirements

### Validated

<!-- Shipped and confirmed valuable in v1.0 -->

**Core Remapping Engine:**
- ✓ Single key to single key remapping (A→B) — v1.0
- ✓ Remap configuration via YAML file — v1.0
- ✓ Active remap set selection at runtime — v1.0
- ✓ Remap persistence across daemon restarts — v1.0
- ✓ Per-device remap profiles — v1.0

**Quality & Reliability:**
- ✓ Device hotplug handling (USB unplug/replug) — v1.0
- ✓ Configuration hot-reload (SIGHUP) — v1.0
- ✓ Memory safety (no unwrap() on hot paths) — v1.0
- ✓ Unit and integration test coverage — v1.0

**User Interface:**
- ✓ IPC protocol for profile operations — v1.0
- ✓ GUI profile switcher components — v1.0

### Active

<!-- Current scope. Building toward these. -->

**Milestone v1.1: Mouse & Mixed Macros**
- [ ] Mouse events in macros (buttons, movement, scroll)
- [ ] Mixed keyboard+mouse macro recordings
- [ ] GUI mouse button selection and recording
- [ ] Input-only device discovery filtering

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- **LED/RGB control** — Hardware-specific, not core to remapping value
- **Multi-device macros** — Adds significant complexity, defer until core works
- **Network remoting** — Security risk, local-only for now
- **Windows/macOS support** — Linux evdev/uinput specific
- **Application-specific remapping** — Wayland security model prevents window focus detection
- **Key sequences (Vim-style)** — Complex state machine; defer until v2+
- **Layer modes (modifier-activated)** — Advanced feature for v2+
- **Tap-hold functionality** — Advanced feature for v2+
- **Key chords (simultaneous combinations)** — Advanced feature for v2+

## Context

**Current codebase** (razermapper/) has a complete remapping engine with device discovery, grabbing, IPC, hotplug support, and comprehensive testing. The daemon handles device events through udev monitoring, supports atomic profile switching via RemapTable type alias, and provides configuration hot-reload via SIGHUP. The GUI includes profile switcher widgets for runtime profile management.

**Technical environment:** Linux kernel with evdev and uinput support. Daemon runs as root with CAP_SYS_RAWIO for `/dev/uinput` access. Wayland-compatible (no X11 dependencies). Uses Iced 0.12 for GUI. Configuration via YAML at `/etc/razermapperd/remaps.yaml`.

**Current state:** 55,413 LOC (Rust + TOML). 58 passing tests (4 macro_engine tests skipped due to pre-existing hang issue). All 4 phases complete (19 plans total). Shipped as v1.0 on 2026-02-17.

## Constraints

- **Linux only**: evdev, uinput, ioctl are Linux-specific — breaking change to support other platforms
- **Root required**: Daemon needs CAP_SYS_RAWIO for `/dev/uinput` access — non-negotiable for input injection
- **Rust 2021 edition**: Codebase uses 2021 edition patterns — async/await with tokio
- **Iced 0.12**: GUI framework pinned to this version — API may break on updates
- **Wayland focus**: No X11-specific codepaths — must work on Wayland compositors

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| KeyParser with evdev::Key enum | Type-safe, no manual u16 conversions | ✓ Good — prevents key code errors |
| RemapTable type alias for atomic switching | O(1) pointer swaps, no allocations during event processing | ✓ Good — enables fast profile switching |
| YAML config for remaps | Human-readable, easy to edit, supports comments | ✓ Good — power user friendly |
| udev for device monitoring | Standard Linux input subsystem API | ✓ Good — reliable hotplug detection |
| SIGHUP for config reload | Standard UNIX daemon signal | ✓ Good — familiar to admins |
| validate-then-swap pattern | Atomic updates, rollback on error | ✓ Good — prevents invalid config states |
| evdev::uinput for testing | CI-compatible, no hardware required | ✓ Good — comprehensive test coverage |
| Drop trait for device cleanup | RAII pattern, panic-safe | ✓ Good — clean resource management |

---
*Last updated: 2026-02-17 after v1.0 milestone*
