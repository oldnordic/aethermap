# Project Research Summary

**Project:** Aethermap (Linux Input Remapping Daemon)
**Domain:** Linux Input Remapping (evdev/uinput)
**Researched:** 2026-02-16
**Confidence:** HIGH

## Executive Summary

Aethermap is a Linux input remapping daemon built in Rust that intercepts keyboard events at the evdev layer and injects remapped events via uinput. This is the standard architecture for modern Linux remappers (keyd, xremap, kanata) because it works on both X11 and Wayland, avoiding the X11-specific tools (xmodmap, xkb) that don't work with Wayland compositors.

The recommended approach is to build on the existing Aethermap foundation, which already has the right core components (DeviceManager with EVIOCGRAB, MacroEngine, UinputInjector, IPC). The critical missing piece is a dedicated RemapEngine for key-to-key translations. Research shows that simple 1:1 key remapping is table stakes—users expect this as the core feature. Differentiators like layers, tap-hold, and key chords are power-user features that should be phased in after the foundation is solid.

The primary risks are state management bugs (stuck keys, lost release events, modifier desynchronization) and memory leaks in macro execution. The current codebase exhibits several warning signs: 81+ unwrap() calls, acknowledged macro cleanup issues, and incomplete privilege dropping. Phase 1 must prioritize defensive error handling, state cleanup on all error paths, and proper resource management before adding features.

## Key Findings

### Recommended Stack

The existing stack is well-chosen and requires minimal additions. The core evdev/uinput/tokio foundation is correct for this domain.

**Core technologies:**
- **evdev 0.12** — Linux input device access and KeyCode enum — Standard crate for evdev on Linux, provides Key::KEY_A enum directly
- **uinput-sys 0.1** — Virtual device creation for event injection — Required for injecting remapped events
- **tokio 1** — Async runtime for event processing — Already used for spawn_blocking around evdev's sync I/O
- **serde_yaml 0.9** — YAML config file parsing — Human-readable format, follows xremap/keyd pattern users recognize
- **indexmap 2.0** — Ordered HashMap for remap tables — Preserves remap order for cascading remaps

**New additions needed:**
- **evdevil 0.3** (dev-dependencies) — Virtual device creation for E2E testing

**Configuration format:** YAML (serde_yaml) — Already used in project, more flexible than TOML for nested remap rules.

### Expected Features

The Linux input remapping landscape has shifted from X11-specific tools to evdev/uinput-based solutions. Table stakes have evolved: users now expect Wayland compatibility and per-device remapping as baseline.

**Must have (table stakes) — launch requirements:**
- **Single key to single key remapping** — Core value proposition, A→B mapping
- **X11 and Wayland support** — Wayland is default on Fedora/Ubuntu, legacy tools don't work
- **Per-device remapping** — Different keyboards need different layouts
- **Modifier key remapping** — CapsLock→Ctrl/Escape is most common use case
- **Configuration persistence** — Auto-load on daemon start
- **Low latency processing** — Sub-10ms required, input lag is immediately noticeable

**Should have (competitive differentiation) — v1.x:**
- **Layers (3-4 max)** — Most requested power user feature, enables ergonomic layouts
- **Tap-hold / mod-tap** — One key does two things, critical for compact keyboards
- **GUI configuration tool** — Major adoption barrier removed for non-technical users
- **Macro playback** — Already have recording, need execution engine
- **Config hot-reload** — Quality of life for power users iterating on configs

**Defer (v2+) — not essential for product-market fit:**
- **Key chords** — Advanced, small user base, high complexity
- **Key sequences** — Complex state machine, niche use case
- **Application-specific remapping** — Wayland security model conflicts make this risky
- **Visual GUI binding editor** — Click-to-assign requires significant UI work
- **One-shot modifiers** — Nice to have but not table stakes

### Architecture Approach

Linux input remapping systems follow a layered event interception architecture: physical device → DeviceManager (EVIOCGRAB) → RemapEngine (translation) → MacroEngine (sequences) → UinputInjector (virtual device) → kernel input core → applications.

**Major components:**
1. **DeviceManager** — Device discovery, exclusive grab, event stream capture — Already implemented correctly with EVIOCGRAB
2. **RemapEngine (NEW)** — Key-to-key translations, device-specific remaps, modifier handling — **Critical missing component**
3. **MacroEngine** — Multi-action sequences, trigger detection, timed execution — Already implemented, needs cleanup fixes
4. **ConfigManager** — Profile persistence, hot-reload, YAML parsing — Already exists, needs RemapEntry support
5. **UinputInjector** — Virtual device creation, event synthesis — Already implemented
6. **IPC Layer** — Unix socket communication between GUI and daemon — Already established

**Key pattern:** Layered event processing. Events pass through RemapEngine first (key code lookup), then MacroEngine (trigger detection on remapped keys), then UinputInjector (output). This separation keeps remaps simple and synchronous while macros remain stateful.

**Recommended addition:** Create `aethermapd/src/remap.rs` with RemapEngine struct. Insert into event pipeline between DeviceManager's event channel and MacroEngine in main.rs.

### Critical Pitfalls

Research reveals six critical pitfalls that plague evdev/uinput remappers. The current codebase already exhibits warning signs for several.

1. **Stuck keys during EVIOCGRAB** — Keys become "stuck" when held during device grab
   - **Prevention:** Query device state after grab with EVIOCGKEY, inject synthetic release events for pressed keys
   - **Current status:** Not implemented, likely issue

2. **Lost release events on error paths** — Modifier keys appear permanently active after errors
   - **Prevention:** Defensive state tracking (HashMap<u16, bool>), Drop impl releases all keys, replace unwrap() with ?
   - **Current status:** 81+ unwrap() calls, direct violation of CLAUDE.md Rule #6

3. **Modifier state desynchronization** — Internal state inconsistent with actual keyboard state
   - **Prevention:** Query modifier state with EVIOCGKEY before injecting, maintain independent modifier tracking
   - **Current status:** Not implemented, especially problematic on Wayland

4. **Event value 2 (repeat) mishandling** — Key repeat events broken or cause infinite loops
   - **Prevention:** Explicit repeat handling, check value==2, forward repeats unchanged for simple remaps
   - **Current status:** Unknown, needs testing with evtest

5. **Device hotplug race conditions** — Daemon crashes or loses devices on USB unplug/replug
   - **Prevention:** Inotify monitoring of /dev/input/, graceful degradation, FD cleanup in Drop
   - **Current status:** No device monitoring, scans once at startup only

6. **Memory leaks in macro execution** — MacroEngine::executing HashMap never cleaned up
   - **Prevention:** Completion channel from spawned tasks, self-cleanup pattern, timeout-based cleanup
   - **Current status:** Explicitly acknowledged in macro_engine.rs:352

**Technical debt patterns to avoid:**
- unwrap() instead of ? — NEVER acceptable, violates CLAUDE.md
- mem::forget() for FD lifetime — Only with explicit Drop implementation
- Ignoring repeat events (value 2) — Breaks core functionality
- No hotplug handling — Only acceptable for initial prototype

## Implications for Roadmap

Based on combined research, the roadmap should be structured around foundation-first, features-second. The existing codebase has the right architecture but critical quality issues that will block feature development.

### Phase 1: Remapping Foundation

**Rationale:** The RemapEngine is the missing core component. Without it, there is no key remapping functionality. This phase establishes the fundamental event flow: device → remap → macro → inject. Must also fix the memory leak in macro execution and address error handling before adding complexity.

**Delivers:**
- RemapEngine with HashMap-based key code translation
- Per-device and global remap support
- ConfigManager extension for RemapEntry (YAML)
- RemapEngine integration into main event loop
- Macro execution cleanup (fix memory leak)
- Basic error recovery (remove unwrap() panic risk)

**Addresses (from FEATURES.md):**
- Single key to single key remapping (P1)
- Per-device remapping (P1)
- Configuration persistence (P1)

**Avoids (from PITFALLS.md):**
- Pitfall #6: Memory leaks in macro execution — Fix completion channel cleanup
- Pitfall #2: Lost release events — Add defensive state tracking
- Technical debt: Remove unwrap() calls, add proper error propagation

**Features:** Simple A→B remapping, device-specific overrides, YAML config format

### Phase 2: Modifier Keys & Hotplug

**Rationale:** Modifiers are the most common remapping target (CapsLock→Ctrl/Escape). This phase requires careful state synchronization to avoid Pitfall #3 (modifier desync). Hotplug support is critical for laptop users and USB keyboards—without it, daemon crashes on device removal.

**Delivers:**
- Modifier key remapping with state query (EVIOCGKEY)
- Atomic modifier changes (release original before press new)
- Inotify-based device monitoring (watch /dev/input/)
- Graceful device removal handling (FD cleanup, continue operating)
- Device re-acquisition after hotplug
- Modifier state testing and validation

**Addresses (from FEATURES.md):**
- Modifier key remapping (P1)
- Device-specific profiles already enabled from Phase 1

**Avoids (from PITFALLS.md):**
- Pitfall #3: Modifier state desynchronization — Query state on grab
- Pitfall #5: Device hotplug race conditions — Inotify monitoring
- Pitfall #1: Stuck keys during grab — State cleanup on EVIOCGRAB

**Features:** CapsLock→Ctrl/Escape remapping, keyboard unplug/replug without restart

### Phase 3: Advanced Features & Polish

**Rationale:** With foundation solid and modifiers working, add competitive differentiators. Layers are the most requested power user feature and build naturally on modifier detection. Tap-hold is complex but high-value. GUI support removes adoption barrier for non-technical users.

**Delivers:**
- Layer support (3-4 layers max, activated by modifiers)
- Tap-hold / mod-tap for one key (timer-based decision)
- Basic GUI remap editor (text-based key selection)
- Config hot-reload (inotify + atomic swap)
- LED feedback for active layer
- Repeat event handling verification

**Addresses (from FEATURES.md):**
- Layers (P2) — Most requested power user feature
- Tap-hold / mod-tap (P2) — Critical for ergonomic layouts
- GUI configuration tool (P2) — Adoption barrier removal
- Config hot-reload (P2) — Quality of life
- Low latency processing validation

**Avoids (from PITFALLS.md):**
- Pitfall #4: Event value 2 handling — Explicit repeat testing
- Performance traps — Verify <10ms latency under load

**Features:** QMK-style layers, CapsLock as tap-hold, GUI for creating remaps

### Phase 4: Security Hardening & Optimization

**Rationale:** Privilege separation is incomplete (acknowledged in security.rs). This phase addresses security concerns and optimizes performance for power users with large remap tables.

**Delivers:**
- Complete privilege dropping (libcap, keep only CAP_SYS_ADMIN)
- Unix socket permission hardening (0660, group:input)
- Rate limiting on macro execution
- Lock-free remap tables (crossbeam) for scalability
- E2E testing with evdevil (virtual devices)
- Comprehensive integration tests

**Addresses (from FEATURES.md):**
- Root daemon with IPC (already implemented, harden security)
- Low latency processing (optimize for large configs)

**Avoids (from SECURITY_MISTAKES in PITFALLS.md):**
- Running as root entire session — Drop privileges after uinput setup
- Unprivileged IPC access — Unix socket permissions
- No rate limiting — Per-macro and global timeouts

**Features:** Non-root daemon operation, hardening against privilege escalation

### Phase Ordering Rationale

- **Foundation before features:** Phase 1 builds the RemapEngine missing from current architecture. Cannot add layers/tap-hold without basic 1:1 remapping working.
- **State management early:** Phases 1-2 address all 6 critical pitfalls upfront. Adding features on top of buggy state tracking creates impossible debugging.
- **Modifiers before layers:** Layers depend on modifier key detection. Must get modifiers right before building layer activation logic.
- **Hotplug before GUI:** Laptops are common target. GUI users will unplug keyboards—daemon must not crash.
- **Security last:** Privilege dropping is important but can wait until functionality works. Over-optimizing security early slows development.
- **Advanced features after polish:** Tap-hold, layers, and GUI are differentiators but not table stakes. Launch with solid core, iterate on power-user features.

### Research Flags

**Phases likely needing deeper research during planning:**

- **Phase 3 (Layers):** Layer activation timing is tricky. Need to research keyd/kanata layer switching implementation details—how they handle layer transitions, rollbacks, and edge cases like rapid modifier presses.

- **Phase 3 (Tap-hold):** Timer-based tap vs hold decision is sensitive. Research keyd's tap-hold implementation—what timeout values work, how to handle rapid tap-hold-tap sequences, and integration with repeat events.

- **Phase 4 (Lock-free optimization):** Current architecture uses Arc<RwLock>. If lock contention becomes bottleneck, need to research crossbeam-based lock-free patterns. Defer until profiling shows actual bottleneck.

**Phases with standard patterns (skip /gsd:research-phase):**

- **Phase 1 (RemapEngine):** Well-documented pattern. evdev→lookup table→uinput is straightforward. xremap source code provides reference implementation.

- **Phase 2 (Modifiers & Hotplug):** Standard Linux patterns. EVIOCGKEY for state queries, inotify for device monitoring. kernel input documentation covers this.

- **Phase 4 (Security):** Standard privilege separation. libcap documentation, systemd service unit patterns. No novel security challenges.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Verified against official docs (evdev-rs, kernel input subsystem). All choices standard for domain. |
| Features | MEDIUM | Based on web search and competitor analysis. Table stakes clear from multiple sources (keyd, xremap, kanata GitHub issues). Some subjectivity in prioritization. |
| Architecture | HIGH | Verified against keyd architecture (reference implementation), Linux input subsystem docs. Layered event processing is established pattern. |
| Pitfalls | MEDIUM-HIGH | 6 critical pitfalls from Stack Overflow, GitHub issues, kernel docs. Stuck keys during grab confirmed from libevdev bug report. Current codebase exhibits warning signs. |

**Overall confidence:** HIGH

The research is solid because:
1. Stack and architecture verified against official documentation (kernel input subsystem, evdev-rs docs)
2. Feature landscape validated across multiple competing projects (keyd, xremap, kanata, input-remapper)
3. Pitfalls confirmed from primary sources (libevdev GitHub, Stack Overflow, kernel docs)
4. Current codebase analysis confirms research matches reality (macro leaks, unwrap() calls)

### Gaps to Address

**Minor gaps (acceptable to proceed):**

- **Exact timeout values for tap-hold:** Research shows keyd uses 200ms default, but optimal value may vary. Should be configurable. Address in Phase 3 planning.

- **Layer activation UX:** How do users know which layer is active? LED feedback, tray icon, or sound? Research doesn't provide clear answer. User testing in Phase 3.

- **Maximum remap table size:** At what point does HashMap lookup become bottleneck? Research says "100k users" need optimization, but no specific table size threshold. Profile in Phase 4.

**Research gaps to validate during implementation:**

- **Wayland compositor variations:** Research shows Wayland handles keyboard layouts per-compositor. Need to test on GNOME (KWin), Sway, Weston. Verify remapper doesn't interfere.

- **Bluetooth keyboard compatibility:** Research notes Bluetooth keyboards particularly prone to modifier desync. Need testing with Logitech MX Mechanical, other common BT keyboards.

- **evdevil testing limitations:** Virtual devices work for E2E tests but may not catch all hardware-specific bugs. Physical device testing required before Phase 3 launch.

## Sources

### Primary (HIGH confidence)
- [evdev crate documentation](https://docs.rs/evdev) — KeyCode enum, device API, uinput support
- [Linux Kernel Input Subsystem Documentation](https://www.kernel.org/doc/html/latest/input/input.html) — Official evdev/uinput behavior
- [keyd GitHub Repository](https://github.com/rvaiya/keyd) — Reference architecture for C-based remapper
- [xremap GitHub](https://github.com/xremap/xremap) — Rust implementation using evdev/uinput, YAML config
- [libevdev Documentation](https://www.freedesktop.org/software/libevdev/doc/latest/) — Authoritative libevdev API

### Secondary (MEDIUM confidence)
- [Stack Overflow - libevdev stuck key bug](https://stackoverflow.com/questions/75945088) — Confirms Pitfall #1 with specific behavior
- [Understanding evdev - Peter Hutterer](https://who-t.blogspot.com/2016/09/understanding-evdev.html) — Input subsystem maintainer's detailed explanation
- [Hackaday: "Linux Fu: Stopping A Runaway" (2025)](https://hackaday.com/2025/04/14/linux-fu-stopping-a-runaway/) — Input remapping landscape overview
- [kanata GitHub](https://github.com/jtroo/kanata) — Cross-platform advanced remapper, tap-hold reference
- [input-remapper GitHub](https://github.com/sezanzeb/input-remapper) — Popular GUI remapper, feature comparison

### Tertiary (LOW confidence)
- [keyd Arch manual pages](https://man.archlinux.org/man/extra/keyd/keyd.1.en) — Feature reference
- Reddit discussions (r/linux, r/ErgoMechKeyboards) — User expectations and pain points
- Arch Wiki keyboard configuration — Community standards

---
*Research completed: 2026-02-16*
*Ready for roadmap: yes*
