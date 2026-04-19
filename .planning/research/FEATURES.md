# Feature Research: Linux Input Remappers

**Domain:** Linux input device key remapping
**Researched:** 2026-02-16
**Confidence:** MEDIUM

## Executive Summary

The Linux input remapping landscape in 2025-2026 is characterized by a shift from X11-specific tools (xmodmap, xkb) to evdev/uinput-based solutions that work on both X11 and Wayland. The market is split between GUI-focused tools (input-remapper) and power-user daemons (keyd, kanata, kmonad, xremap).

**Key insight:** Table stakes have evolved. Users now expect Wayland compatibility, layers, and tap-hold functionality as baseline features. The primary differentiation point is UX quality (GUI vs config files) and advanced features like combos/chords.

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Single key to single key remapping** | Core purpose of any remapper | LOW | A -> B, e.g., CapsLock -> Ctrl |
| **X11 and Wayland support** | Wayland is now default on major distros (Fedora, Ubuntu) | MEDIUM | Legacy tools (xmodmap) don't work on Wayland |
| **Modifier key remapping** | Most common use case (CapsLock -> Ctrl/Escape) | LOW | Must handle Ctrl, Shift, Alt, Super |
| **Configuration persistence** | Users expect remaps to survive reboots | LOW | Auto-load on daemon start |
| **Per-device remapping** | Multiple keyboards with different layouts are common | MEDIUM | Apply different configs to different devices |
| **Low latency processing** | Input delay is immediately noticeable and unacceptable | MEDIUM | Sub-10ms processing required |
| **Hot-reload configuration** | Power users iterate frequently; restarting daemon is friction | MEDIUM | Reload on file change or IPC command |
| **Basic macro support** | Single key triggers multiple actions | MEDIUM | Key sequences, not just single keys |
| **Root daemon with IPC** | Required for /dev/uinput access; users expect unprivileged GUI | MEDIUM | Privilege separation is standard pattern |

### Differentiators (Competitive Advantage)

Features that set product apart. Not required, but valued.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Layers (multiple key layers)** | Power user feature; enables compact, ergonomic layouts | MEDIUM | Similar to QMK layers; modifier activates layer |
| **Tap-hold / mod-tap** | One key does two things; critical for ergonomic layouts | HIGH | Tap = one action, hold = modifier; timing-sensitive |
| **Key overloading** | Advanced tap-hold; different behaviors on tap vs hold vs long-press | HIGH | Requires configurable timeouts and state tracking |
| **One-shot modifiers** | Modifier activates for next keystroke only; reduces pinky strain | MEDIUM | Common in advanced remappers (keyd, kanata) |
| **Key chords (simultaneous keys)** | Advanced power users; expands available commands | HIGH | Detect simultaneous key presses; timing window critical |
| **Key sequences** | Single key triggers different actions based on sequence | HIGH | Like Vim keybindings; state machine required |
| **Application-specific remapping** | Context-aware; different bindings per app | HIGH | Window title matching; adds complexity |
| **GUI configuration tool** | Major adoption barrier removed; non-technical users | MEDIUM | input-remapper's primary advantage |
| **Visual key binding editor** | Click-to-assign; discoverable interface | HIGH | Requires key capture from GUI |
| **Profile switching at runtime** | Gamers and workflow switchers change setups frequently | LOW | Already have IPC infrastructure |
| **Macro recording** | Record once, replay; faster than manual config | MEDIUM | Already partially implemented in codebase |
| **Unicode/emoji input** | Compose alternative; useful for international users | MEDIUM | Requires mapping to text events |
| **Config file validation** | Clear error messages; prevent silent failures | LOW | Parse and validate before applying |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **X11-specific code paths** | Legacy X11 tools are familiar | Won't work on Wayland; maintenance burden | evdev/uinput works on both |
| **xkb/xmodmap compatibility layer** | Leverage existing configs | Doesn't map 1:1 to evdev; creates confusion | Native evdev key codes |
| **Per-application input injection** | Target specific windows | Wayland security model forbids this | System-level injection only |
| **Hardware RGB/LED control** | "Cool factor" for gaming keyboards | Hardware-specific protocol; not core to remapping value | Use vendor tools or OpenRazer |
| **Network remoting / IPC over TCP** | Control remapper from network | Security risk; unnecessary attack surface | Local Unix socket only |
| **Automatic config generation from XKB** | Migrate existing setups | XKB is stateful, evdev is event-based; mismatch | Manual config or dedicated converter |
| **Global hotkeys for system actions** | Convenient media/volume controls | Conflicts with DE/WM handling | Let DE handle these |
| **Multi-device macros spanning devices** | "Press keyboard key, then mouse button" | Event ordering is unreliable across devices | Single-device macros only |
| **Full key event logging/recording** | Debugging and analysis | Privacy/security concern; disk churn | Opt-in debugging mode only |
| **Infinite layer nesting** | "The more layers, the better" | User confusion; cognitive overhead | Limit to 3-4 layers max |

## Feature Dependencies

```
[single key remapping]
    └──requires──> [device discovery and grabbing]
                   └──requires──> [uinput virtual device creation]

[layers]
    └──requires──> [modifier key detection]
                   └──enhanced by──> [layer activation feedback]

[tap-hold / mod-tap]
    └──requires──> [timer-based key event processing]
                   └──requires──> [key state tracking]

[key chords]
    └──requires──> [simultaneous key detection]
                   └──requires──> [chord timeout configuration]

[key sequences]
    └──requires──> [state machine for partial sequences]
                   └──requires──> [sequence timeout]

[application-specific remapping]
    └──requires──> [window title/process detection]
                   └──conflicts──> [Wayland security model]

[GUI key binding editor]
    └──requires──> [key capture from GUI context]
    └──enhanced by──> [visual device representation]

[macro recording]
    └──requires──> [event capture and storage]
                   └──enhanced by──> [macro playback engine]
```

### Dependency Notes

- **Single key remapping requires device discovery/grabbing:** Can't remap what you can't capture. Already implemented in codebase.
- **Layers require modifier key detection:** Need to track which modifiers are currently active to determine active layer.
- **Tap-hold requires timer-based processing:** Must decide between tap and hold based on duration; adds state machine complexity.
- **Key chords require simultaneous detection:** Need to track multiple keys pressed within timing window.
- **Application-specific remapping conflicts with Wayland security:** Wayland deliberately prevents apps from knowing which window has focus. Anti-feature for Wayland-first approach.
- **GUI key binding editor requires key capture:** GUI needs to receive key events from daemon without triggering remaps.

## MVP Definition

### Launch With (v1)

Minimum viable product — what's needed to validate the concept.

- [x] **Device discovery and grabbing** — Already implemented; required foundation
- [ ] **Single key to single key remapping** — Core value proposition; A -> B mapping
- [ ] **YAML configuration for remaps** — Text-based config is fastest to implement
- [ ] **Per-device remap profiles** — Different keyboards, different layouts
- [ ] **Persistence across daemon restarts** — Auto-load on startup
- [ ] **Runtime profile switching** — Already have IPC; add remap activation

### Add After Validation (v1.x)

Features to add once core is working.

- [ ] **Layers (3-4 max)** — Most requested power user feature
- [ ] **Tap-hold for one key** — Prove the concept before generalizing
- [ ] **Macro playback** — Already have recording; add execution
- [ ] **Config hot-reload** — Quality of life improvement
- [ ] **Basic GUI remap editor** — Text-based key selection

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] **Key chords** — Advanced; small user base
- [ ] **Key sequences** — Complex state machine; niche use case
- [ ] **Application-specific remapping** — Wayland incompatibility makes this risky
- [ ] **Visual GUI binding editor** — Click-to-assign; significant UI work
- [ ] **One-shot modifiers** — Nice to have; not table stakes
- [ ] **Unicode/emoji input** — Convenience feature; not core

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Single key to single key remapping | HIGH | LOW | **P1** |
| X11 and Wayland support | HIGH | MEDIUM (already on Wayland path) | **P1** |
| Per-device remapping | HIGH | MEDIUM | **P1** |
| Configuration persistence | HIGH | LOW | **P1** |
| Modifier key remapping | HIGH | LOW | **P1** |
| Layers | MEDIUM | MEDIUM | **P2** |
| Tap-hold / mod-tap | MEDIUM | HIGH | **P2** |
| GUI configuration tool | MEDIUM | MEDIUM | **P2** |
| Macro playback | MEDIUM | MEDIUM | **P2** |
| Key chords | LOW | HIGH | **P3** |
| Key sequences | LOW | HIGH | **P3** |
| Application-specific remapping | LOW (Wayland issues) | HIGH | **P3** |

**Priority key:**
- **P1**: Must have for launch (table stakes)
- **P2**: Should have, competitive differentiation
- **P3**: Nice to have, niche or high cost

## Competitor Feature Analysis

| Feature | input-remapper | keyd | kanata/kmonad | xremap | Our Approach |
|---------|---------------|------|---------------|--------|--------------|
| **Interface** | GUI | Config file | Config file | Config file | Start with config, add GUI |
| **Wayland support** | Yes | Yes | Yes (via evdev) | Yes | Yes (already on evdev) |
| **Layers** | Limited | Yes (2 layers) | Yes (unlimited) | Yes | Start with 3-4 |
| **Tap-hold** | No | Yes | Yes | Yes | Phase 2 |
| **Key chords** | No | No | Yes | Some | Future |
| **Key sequences** | Via macros | Via macros | Yes | Via macros | Future |
| **App-specific** | No | No | No | Yes | Avoid (Wayland issues) |
| **Device-specific** | Yes | Yes | No | No | Yes (already have device ID) |
| **Mouse support** | Yes | Limited | Limited | Limited | Phase 2+ |
| **Macro recording** | Yes | No | No | No | Partially implemented |

## Linux-Specific Considerations

### Wayland vs X11

**Critical:** Wayland's security model fundamentally changes what's possible:

- **X11:** Any application can grab input, inject events anywhere, read window titles
- **Wayland:** Compositor arbitrates all input; apps can't see other windows' input

**Implication:** Application-specific remapping (different bindings per app) is effectively an anti-feature for a Wayland-first remapper. Requires compositor-specific protocols (foreign-toplevel, etc.) and creates fragmentation.

### evdev vs uinput

- **evdev:** Reading from `/dev/input/event*` — already implemented
- **uinput:** Writing to virtual input device — already implemented

**Implication:** The codebase already has the right foundation. No architecture changes needed.

### Root Privilege Model

All modern remappers use a privileged daemon with unprivileged client:

| Tool | Privilege Model |
|------|-----------------|
| keyd | Daemon as root, drops capabilities |
| xremap | Daemon as root |
| input-remapper | Systemd service with capabilities |
| kanata | User runs with sudo/polkit |

**Our approach:** Already implemented; continue with privilege separation.

## Quality Gates

- [x] Categories are clear (table stakes vs differentiators vs anti-features)
- [x] Complexity noted for each feature
- [x] Dependencies between features identified
- [x] Competitor analysis complete
- [x] MVP definition with clear v1/v1.x/v2+ boundaries

## Sources

### Primary Research (Web Search - CONFIDENCE: MEDIUM)

- [Hackaday: "Linux Fu: Stopping A Runaway" (2025-04-14)](https://hackaday.com/2025/04/14/linux-fu-stopping-a-runaway/) - Linux input remapping landscape overview
- [XDA Developers: Kanata coverage (Nov 2025)](https://www.xda-developers.com/) - Advanced cross-platform remapper features
- [input-remapper GitHub](https://github.com/sezanzeb/input-remapper) - Popular GUI remapper for Linux
- [keyd GitHub](https://github.com/rvaiya/keyd) - Daemon-based remapper with layers
- [kanata GitHub](https://github.com/jtroo/kanata) - Cross-platform advanced remapper
- [xremap documentation](https://github.com/xremap/xremap) - Wayland-friendly remapper
- [Interception-tools](https://github.com/meicale/Interception-Tools) - Low-level evdev processing
- [Arch Wiki: Keyboard configuration](https://wiki.archlinux.org/title/Keyboard_configuration) - Community standards

### Issues and Limitations (CONFIDENCE: HIGH)

- [input-remapper issues](https://github.com/sezanzeb/input-remapper/issues) - User-reported problems and feature requests
- [Wayland input protocol discussion](https://www.csslayer.info/wordpress/linux/key-repetition-and-key-event-handling-issue-with-wayland-input-method-protocols/) - Wayland input handling complexities
- [Fedora mailing list: Input grab issues](https://lists.fedoraproject.org/archives/list/desktop@lists.fedoraproject.org/) - Platform-specific problems

### Community Standards (CONFIDENCE: MEDIUM)

- Reddit discussions (r/linux, r/ErgoMechKeyboards) - User expectations and pain points
- Arch Wiki and man pages - Documentation of what's considered standard

---

*Feature research for: Aethermap (Linux input remapper)*
*Researched: 2026-02-16*
*Research mode: Ecosystem*
