# Phase 13: Wayland Portal Integration - Context

**Phase:** 13 - Wayland Portal Integration and Global Hotkeys
**Created:** 2026-02-18
**Status:** Ready to plan

## Overview

This phase enables automatic profile switching based on window focus on Wayland compositors, plus global hotkey support for manual profile switching. The GUI (running as user) integrates with xdg-desktop-portal to detect window focus changes, which are sent to the daemon via IPC. The daemon matches app_id to configured profiles and switches accordingly. Global hotkeys are implemented at the daemon level using existing device grabbing capabilities.

## Dependencies

- **Phase 10:** Layer System (provides layer-aware profile switching infrastructure)
- **Phase 9:** Device Detection (Azeron Cyborg analog stick events flow through same pipeline)

## Decisions (Locked)

### Must Implement
- Use ashpd (Rust xdg-desktop-portal bindings) for Wayland integration
- Global hotkeys implemented at daemon level (root, has device access)
- Auto-profile switching based on app_id matching (not window title - Wayland security restriction)
- Graceful fallback when portal unavailable

### Claude's Discretion
- Portal permission UX (when to prompt user)
- Specific hotkey combinations (default: Ctrl+Alt+Shift+Number)
- Auto-switch rule storage format (YAML structure)

### Out of Scope
- Window title-based profile switching (Wayland doesn't allow access)
- Per-compositor workarounds (use standard portal only)

## Technical Context

**Wayland Portal Architecture:**
```
┌─────────────────┐         xdg-desktop-portal         ┌──────────────────┐
│   aethermap   │ ──────────────────────────────────│  Wayland Compositor│
│   (GUI, user)   │  ashpd (zbus D-Bus)               │  (sway, GNOME, etc)│
└────────┬────────┘                                     └──────────────────┘
         │
         │ IPC (existing socket)
         │ FocusChanged{app_id, window_title}
         ▼
┌─────────────────┐
│  aethermapd   │
│  (daemon, root) │
└─────────────────┘
```

**Key Challenge:** GUI runs as normal user (for portal access), daemon runs as root (for input device access). Focus detection must bridge this privilege boundary via IPC.

**Global Hotkeys:**
Since daemon already grabs input devices, global hotkey detection is straightforward:
1. Intercept key events before normal remapping
2. Check if key combination matches hotkey binding
3. If match, execute profile switch action and consume event
4. Otherwise, pass through normal remapping pipeline

## Success Criteria

1. GUI detects window focus changes via xdg-desktop-portal
2. Focus changes propagate to daemon via IPC
3. Daemon switches profiles based on app_id rules
4. Global hotkeys (Ctrl+Alt+Shift+Number) work on Wayland
5. Portal permission errors handled gracefully
6. Auto-switch rules configurable in YAML

## Open Questions

1. **Portal permission UX:** Should GUI auto-trigger portal request on startup, or wait for user to configure auto-switching?

2. **Profile priority:** When both auto-switch and manual hotkey occur, which wins? (Recommend: manual overrides until focus changes)

3. **Hotkey defaults:** Should Ctrl+Alt+Shift+1-9 be default, or make configurable?

## File Context

**Daemon (aethermap/aethermapd):**
- `src/lib.rs` - Main daemon entry, IPC handlers
- `src/layer_manager.rs` - Profile switching via LayerManager
- `src/config.rs` - YAML config loading (will need extension)

**GUI (aethermap/aethermap):**
- `src/main.rs` - GUI entry point
- `src/ipc.rs` - IPC client for daemon communication

**New Dependencies:**
```toml
# aethermap/Cargo.toml (GUI)
ashpd = { version = "0.8", default-features = false, features = ["wayland"] }
```

---
*Context created: 2026-02-18*
