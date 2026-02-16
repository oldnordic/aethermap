# Razermapper

## What This Is

A Wayland-compatible input device remapper for Linux. The daemon runs as root to access `/dev/input/*` and `/dev/uinput`, creating virtual devices that emit remapped events. Users configure key remaps through YAML profiles and/or a GUI client. The project is currently experimental — basic device grabbing and macro recording work, but the core key remapping engine is missing.

## Core Value

**Users can remap any input key to any other key, with remaps persisting across reboots and switchable at runtime.**

If key remapping doesn't work reliably, nothing else matters.

## Requirements

### Validated

<!-- Shipped and confirmed valuable. -->

From existing codebase analysis:
- ✓ Device discovery via `/proc/bus/input/devices` and evdev scanning — existing
- ✓ Exclusive device capture via EVIOCGRAB ioctl — existing
- ✓ IPC communication between daemon and GUI via Unix socket — existing
- ✓ Macro recording (partial implementation) — existing, needs testing
- ✓ Profile save/load via YAML — existing, needs verification

### Active

<!-- Current scope. Building toward these. -->

**Core Remapping Engine:**
- [ ] Single key to single key remapping (A→B)
- [ ] Remap configuration via YAML file
- [ ] Active remap set selection at runtime
- [ ] Remap persistence across daemon restarts
- [ ] Per-device remap profiles

**Advanced Remapping (Phase 2+):**
- [ ] Key sequences (one key triggers multiple outputs)
- [ ] Layer modes (modifier changes key meanings)
- [ ] Modifier combination remaps (Ctrl+A → Ctrl+B)
- [ ] Key chords (simultaneous key combinations)

**User Interface:**
- [ ] Config file syntax for remap definitions
- [ ] GUI key binding builder (click-to-assign)
- [ ] Profile activation/deactivation commands

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- **LED/RGB control** — Hardware-specific, not core to remapping value
- **Multi-device macros** — Adds significant complexity, defer until core works
- **Network remoting** — Security risk, local-only for now
- **Windows/macOS support** — Linux evdev/uinput specific

## Context

**Existing codebase** (razermapper/) has working device discovery, grabbing, and IPC infrastructure. The macro engine exists but has memory leaks and incomplete trigger handling. The GUI is functional but basic. Over 80 `.unwrap()` calls create crash risk. Security hardening needed (socket permissions, privilege dropping).

**Technical environment:** Linux kernel with evdev and uinput support. Daemon must run as root initially but can drop privileges. Wayland-compatible (no X11 dependencies). Uses Iced 0.12 for GUI.

## Constraints

- **Linux only**: evdev, uinput, ioctl are Linux-specific — breaking change to support other platforms
- **Root required**: Daemon needs CAP_SYS_RAWIO for `/dev/uinput` access — non-negotiable for input injection
- **Rust 2021 edition**: Codebase uses 2021 edition patterns — async/await with tokio
- **Iced 0.12**: GUI framework pinned to this version — API may break on updates
- **Wayland focus**: No X11-specific codepaths — must work on Wayland compositors

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Config file syntax before GUI | Faster to implement, easier to debug, enables power users | — Pending |
| Phased implementation (basic → advanced) | Validates core architecture before adding complexity | — Pending |
| Reuse existing macro engine infrastructure | `MacroEngine` already has event routing, can extend for remaps | — Pending |

---
*Last updated: 2026-02-16 after initialization*
