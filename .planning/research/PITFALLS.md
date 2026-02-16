# Pitfalls Research

**Domain:** Linux Input Remapping (evdev/uinput)
**Researched:** 2026-02-16
**Confidence:** MEDIUM

## Critical Pitfalls

### Pitfall 1: Stuck Keys During Device Grab

**What goes wrong:**
When a key is physically held down during `EVIOCGRAB`, the key becomes "stuck" - the system continuously receives press events but never sees the corresponding release. This is commonly reported with the Enter key but affects any key.

**Why it happens:**
The `EVIOCGRAB` ioctl takes exclusive access to the device, but does not synchronize existing key states. If a key is already pressed when the grab occurs, the kernel's internal state indicates the key is down. Since the original device events are now intercepted, the release event may be lost or misrouted.

**Consequences:**
- Affected key continuously repeats
- User must physically press and release the key to unstuck it
- Creates poor UX especially during daemon restarts

**Prevention:**
1. **State cleanup on grab**: After `EVIOCGRAB`, actively query and release any keys that are currently pressed
2. **Synchronize internal state**: Use `libevdev_get_event_value()` to read current key states after grab
3. **Release injection**: Inject synthetic release events for any keys that are down

**Warning signs:**
- Keys repeating unexpectedly after daemon starts
- `EV_KEY` events with value 2 (repeat) appearing without corresponding value 1 (press)
- User reports: "I have to press X again to make it stop"

**Phase to address:**
Phase 1 - Device Grab Implementation

---

### Pitfall 2: Lost Release Events on Error Paths

**What goes wrong:**
When an error occurs during event processing (e.g., macro execution fails, injector unavailable), the corresponding key release event is never sent. The system thinks the key is still pressed.

**Why it happens:**
Current codebase has 81+ `unwrap()` calls and minimal error recovery. When a panic or early return occurs in the event processing pipeline, pending state changes are not rolled back.

**Consequences:**
- Modifier keys appear permanently active
- Subsequent typing produces wrong characters (e.g., lowercase becomes uppercase)
- Inconsistent state across reboots

**Prevention:**
1. **Defensive state tracking**: Maintain a `HashMap<u16, bool>` of pressed keys
2. **Drop implementation**: Ensure `Drop` releases all held keys on panic/shutdown
3. **Transaction-style operations**: Record state changes before applying, with rollback on error
4. **Replace unwrap()**: Use `?` propagation or `expect()` with context

**Warning signs:**
- Modifier LEDs (Caps Lock, Num Lock) don't match actual state
- Characters typed are unexpectedly shifted
- Need to press modifier twice to "toggle" it back to normal

**Phase to address:**
Phase 1 - Event Processing Foundation

---

### Pitfall 3: Modifier State Desynchronization

**What goes wrong:**
The internal modifier state (Shift, Ctrl, Alt, Super) becomes inconsistent with the actual keyboard state. This is especially problematic on Wayland where the compositor manages keyboard layout independently.

**Why it happens:**
1. **X server behavior**: On X11, the modifier map is cleared when keyboards are added/removed
2. **Wayland complexity**: Each compositor handles layouts independently; no centralized keyboard mapping
3. **Remapping conflicts**: Remapping a key to a modifier (e.g., CapsLock to Control) while the original modifier is pressed

**Consequences:**
- Shifted keys register as unshifted (or vice versa)
- Keyboard shortcuts trigger unexpectedly
- Layer switching in custom remappings fails

**Prevention:**
1. **Query modifier state**: Use `EVIOCGKEY` to read actual modifier states before injecting events
2. **Separate modifier tracking**: Don't rely on compositor state; maintain independent modifier state
3. **Atomic modifier changes**: When remapping modifiers, inject release of original before press of new

**Warning signs:**
- Bluetooth keyboards particularly affected (reported with Logitech MX Mechanical)
- Issue appears only with external keyboards, not built-in
- Restarting the daemon temporarily fixes the issue

**Phase to address:**
Phase 2 - Modifier Key Support

---

### Pitfall 4: Event Value 2 (Repeat) Mishandling

**What goes wrong:**
Key repeat events (EV_KEY value 2) are not properly handled, causing either missing repeats or infinite repeat loops.

**Why it happens:**
The Linux kernel generates EV_KEY events with value 2 for auto-repeat when `EV_REP` is enabled. These events:
1. Are exempt from state-change filtering (unlike press/release)
2. Should pass through even when key state hasn't changed
3. Are often mistakenly filtered out by remapping logic

**Consequences:**
- Held keys don't repeat
- Or: infinite repeat loop when remapping a key to itself
- Inconsistent typing feel

**Prevention:**
1. **Explicit repeat handling**: Check `event.value() == 2` and handle separately from press/release
2. **Repeat passthrough**: For simple remaps, forward repeat events unchanged
3. **Repeat suppression for macros**: Don't generate repeats for macro-triggered keys

**Warning signs:**
- Holding a key produces only one character
- Or: characters infinitely repeat when key held
- `evtest` shows value 2 events not being processed

**Phase to address:**
Phase 1 - Event Processing Foundation

---

### Pitfall 5: Device Hotplug Race Conditions

**What goes wrong:**
When keyboards are added/removed (USB plug/unplug, Bluetooth connect), the daemon loses track of devices or attempts to grab non-existent devices.

**Why it happens:**
1. **No device monitoring**: Current code scans devices once at startup
2. **Grab failure not handled**: `EVIOCGRAB` can fail if device disappears during probe
3. **FD leaks**: Device file descriptors aren't cleaned up when devices disappear

**Consequences:**
- Daemon crashes on device removal
- Newly connected keyboards aren't recognized
- Memory leaks from unclosed file descriptors

**Prevention:**
1. **Inotify monitoring**: Watch `/dev/input/` for device changes
2. **Graceful degradation**: Continue operating if one device fails
3. **FD cleanup**: Use `Drop` and signal handlers to close all FDs on shutdown
4. **Device re-acquisition**: Attempt to re-grab devices after hotplug event

**Warning signs:**
- Daemon crashes when USB keyboard unplugged
- New keyboard not detected until restart
- `lsof` shows many open `/dev/input/event*` files

**Phase to address:**
Phase 1 - Device Management

---

### Pitfall 6: Memory Leaks in Macro Execution

**What goes wrong:**
The `MacroEngine::executing` HashMap is never cleaned up after macros complete, causing unbounded memory growth.

**Why it happens:**
In `macro_engine.rs:352`, the code explicitly notes it cannot modify `self.executing` from the spawned task. There is no mechanism to remove completed macros from the tracking map.

**Consequences:**
- Memory usage grows with each macro execution
- Eventually hits process limits
- "Max concurrent macros" falsely limits even when old macros are done

**Prevention:**
1. **Completion channel**: Spawned tasks send completion message back
2. **Self-cleanup**: Use a channel-based pattern where the task removes itself
3. **Timeout cleanup**: Periodically scan for stale entries (via `Instant::elapsed()`)

**Warning signs:**
- `executing.len()` only increases, never decreases
- RSS memory grows over time
- After many macro executions, "max concurrent macros" hit despite no active macros

**Phase to address:**
Phase 1 - Macro Engine Refactoring

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| `unwrap()` instead of `?` | Less typing, "works for now" | Panics in production, poor error messages | NEVER - directly violates CLAUDE.md |
| `mem::forget()` for FD lifetime | Avoids complex ownership | FD leaks on all error paths | Only with explicit Drop implementation |
| `Arc<RwLock<T>>` everywhere | Easy sharing | Lock contention, unclear ownership | For shared state only, not per-connection |
| Ignoring repeat events (value 2) | Simpler event handling | Broken auto-repeat | NEVER - core functionality |
| No hotplug handling | Works for static setup | Daemon unusable with laptops | ONLY for initial prototype |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| libevdev | Assuming `fetch_events()` always returns new events | Handles both sync and async, may return empty |
| uinput | Creating device without setting capabilities | Must call `UI_SET_EVBIT` for each event type |
| X11/Wayland | Assuming remapping works everywhere | X11 requires XKB, Wayland needs compositor support |
| systemd | Not notifying when ready | Use `sd_notify(READY=1)` for service management |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| O(n) lookup in hot path | High CPU during fast typing | Use `HashMap<u16, RemapRule>` instead of Vec | At ~100 keypresses/second |
| Excessive syscalls | Latency between keypress and action | Batch events where possible, cache lookups | With complex remappings |
| Channel flooding | Backpressure, dropped events | Bounded channels, select! for prioritization | Under macro execution load |
| spin-wait on events | 100% CPU when idle | Use proper async/await with tokio | Always noticeable |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Injecting shell commands via macros | Command injection, privilege escalation | Whitelist only, no shell expansion |
| Running as root entire session | Privilege escalation via compromised daemon | Drop privileges after uinput setup |
| Unprivileged IPC access | Any user can inject events | Unix socket permissions (0660, group:input) |
| No rate limiting on macros | DoS via infinite macro loop | Per-macro execution limits, global timeout |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No feedback on remap active | User doesn't know if remap worked | LED indicator, logging, tray icon |
| Keys "disappear" when remapped | User thinks keyboard broken | Passthrough mode, disable remap easily |
| No way to escape stuck key | Must restart daemon/unplug keyboard | Global "panic key" combination |
| Daemons restarts lose state | Have to re-apply all remaps | Persistent config, reload on SIGHUP |

## "Looks Done But Isn't" Checklist

- [ ] **Key Remapping**: Often missing repeat event handling — verify with `evtest` that value 2 events work
- [ ] **Modifier Keys**: Often missing state query on grab — verify modifier state after `EVIOCGRAB`
- [ ] **Multiple Devices**: Often missing per-device tracking — verify two keyboards can have different remaps
- [ ] **Hotplug**: Often missing device monitoring — verify keyboard works after unplug/replug
- [ ] **Macro Cleanup**: Often missing execution cleanup — verify `executing` map empties after macro completes
- [ ] **Privilege Dropping**: Often incomplete — verify `getpcaps` shows only needed capabilities after init

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Stuck key | LOW | Inject synthetic release events for all pressed keys on grab |
| Lost state | MEDIUM | Query actual device state with `EVIOCGKEY` and resync |
| Memory leak | HIGH | Requires restart; add monitoring and alerting |
| Lock poisoning | HIGH | Restart daemon; use try_lock patterns to avoid permanent poisoning |
| FD leak | MEDIUM | Close all FDs in Drop handler; add monitoring |

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Stuck keys during grab | Phase 1 - Device Grab | Test: Hold key while starting daemon, verify key not stuck |
| Lost release events | Phase 1 - Event Processing | Test: Cause error during macro, verify no stuck modifiers |
| Modifier desync | Phase 2 - Modifier Support | Test: Press Shift, unplug keyboard, replug, verify Shift state |
| Event value 2 handling | Phase 1 - Event Processing | Test: Hold key, verify character repeats properly |
| Device hotplug races | Phase 1 - Device Management | Test: Unplug/replug keyboard, verify daemon continues |
| Macro execution leaks | Phase 1 - Macro Engine | Test: Execute 1000 macros, verify memory stable |
| Performance issues | Phase 3 - Optimization | Test: Type at 150 WPM, verify no dropped events |
| Security issues | Phase 2 - Security Hardening | Test: Run as non-root, verify daemon still functions |

## Existing Codebase Issues

The current razermapper codebase already exhibits several of these pitfalls:

1. **81 unwrap() calls** (confirmed via grep) - directly violates CLAUDE.md Rule #6, creates panic risk
2. **Macro execution never cleaned up** - `macro_engine.rs:352-354` explicitly notes inability to clean up
3. **Incomplete privilege dropping** - `security.rs:75-82` contains TODO for proper libcap usage
4. **No graceful shutdown on errors** - limited error recovery in event processing paths
5. **Memory leak acknowledged** - `injector.rs:167` uses `mem::forget()` with comment about cleanup in Drop

## Sources

### HIGH Confidence (Official/Primary Sources)
- [Linux Kernel Input Documentation](https://www.kernel.org/doc/Documentation/input/input.txt) - Official input subsystem documentation
- [libevdev Documentation](https://www.freedesktop.org/software/libevdev/doc/latest/) - Authoritative libevdev API reference
- [evdev(4) man page](https://man7.org/linux/man-pages/man4/evdev.4.html) - Official evdev interface documentation

### MEDIUM Confidence (Verified Community Sources)
- [Stack Overflow - libevdev Key Remapping Stuck Key Bug](https://stackoverflow.com/questions/75945088/libevdev-key-remapping-stuck-key-bug) - Confirms stuck key issue during grab (April 2023)
- [Understanding evdev - Peter Hutterer](https://who-t.blogspot.com/2016/09/understanding-evdev.html) - Detailed explanation by input subsystem maintainer
- [The Input Stack on Linux - Venam's Blog](https://venam.net/blog/pdf/input_stack_linux/input_devices_linux.pdf) - Comprehensive input stack guide (November 2025)
- [KarsMulder/evsieve GitHub](https://github.com/KarsMulder/evsieve) - Event transformation tool with working remap implementation
- [keyd GitHub Issues](https://github.com/rvaiya/keyd/issues) - Real-world bug reports from key remapping daemon
- [Kanata GitHub Issues](https://github.com/jtroo/kanata/issues) - Rust-based remapper with known device detection issues

### LOW Confidence (Web Search Only - Verify Before Relying)
- Various Reddit discussions on r/linux, r/archlinux about keyd configuration issues
- [X.Org mailing list - EVIOCGRAB probe discussion](https://lists.x.org/archives/xorg/2008-May/035269.html) - Historical context on grab behavior

### Additional Reading
- [Wayland Input Protocol Documentation](https://wayland.freedesktop.org/docs/html/apc.html#Client-structwl__keyboard__interface) - Wayland-specific keyboard handling
- [Python-evdev GitHub Issue #36](https://github.com/gvalkov/python-evdev/issues/36) - Discussion of EV_REP and repeat events

---
*Pitfalls research for: Linux Input Remapping (evdev/uinput)*
*Researched: 2026-02-16*
