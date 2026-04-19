# Architecture Research

**Domain:** Linux Input Remapping System
**Researched:** 2026-02-16
**Confidence:** HIGH

## Standard Architecture

### System Overview

Linux input remapping systems follow a layered event interception architecture:

```
┌─────────────────────────────────────────────────────────────────┐
│                     Application Layer                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │
│  │     GUI     │  │   CLI Tool  │  │ System Tray │           │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘           │
│         │                │                │                    │
├─────────┼────────────────┼────────────────┼────────────────────┤
│         │       IPC (Unix Socket / D-Bus)                      │
├─────────┼────────────────┼────────────────┼────────────────────┤
│         │                │                │                    │
│  ┌──────▼────────────────▼────────────────▼──────┐            │
│  │              Daemon Core (Privileged)          │            │
│  │  ┌─────────────────────────────────────────┐  │            │
│  │  │  Config Manager (Hot-Reload, Profiles)  │  │            │
│  │  └─────────────────────────────────────────┘  │            │
│  │  ┌─────────────────────────────────────────┐  │            │
│  │  │   Remap Engine (Layer 1: Translations)  │  │            │
│  │  │   - Key code lookup & substitution      │  │            │
│  │  │   - Modifier handling                   │  │            │
│  │  └─────────────────────────────────────────┘  │            │
│  │  ┌─────────────────────────────────────────┐  │            │
│  │  │    Macro Engine (Layer 2: Sequences)    │  │            │
│  │  │    - Trigger detection                  │  │            │
│  │  │    - Action execution                   │  │            │
│  │  └─────────────────────────────────────────┘  │            │
│  └───────────────────────────────────────────────┘            │
│         │                                                │      │
├─────────┼────────────────────────────────────────────────┼──────┤
│         │         Event Processing Pipeline               │      │
│  ┌──────▼──────┐    ┌──────────────┐    ┌──────────────┐ │      │
│  │ Device Grab │───▶│ Event Reader │───▶│ Event Filter │ │      │
│  │ (EVIOCGRAB) │    │  (evdev)     │    │  (Optional)  │ │      │
│  └─────────────┘    └──────────────┘    └──────────────┘ │      │
├────────────────────────────────────────────────────────────┤
│                    Virtual Output Layer                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │  uinput KB   │  │  uinput Mouse│  │Command Exec  │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
├────────────────────────────────────────────────────────────┤
│                     Kernel Input Layer                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │  Hardware    │──▶│ Input Core   │──▶│  evdev       │    │
│  │  Devices     │  │  (kernel)    │  │  (/dev/input)│    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| **Device Manager** | Device discovery, exclusive grab (EVIOCGRAB), event stream capture | evdev-rs with ioctl wrapper |
| **Remap Engine** | Single-key translations, modifier composition, key code lookups | HashMap/Vec-based lookup tables |
| **Macro Engine** | Multi-action sequences, trigger detection, timed execution | State machine with task spawning |
| **Config Manager** | Profile persistence, hot-reload, serialization/deserialization | YAML/TOML/JSON with inotify |
| **Uinput Injector** | Virtual device creation, event synthesis, output injection | uinput-rs with ioctl setup |
| **IPC Layer** | Client-daemon communication, authentication, request routing | Unix socket or D-Bus |
| **Security Manager** | Privilege dropping, capability management, token auth | libsecbind or manual drop_privileges |

## Recommended Project Structure

```
aethermap/
├── aethermap-common/          # Shared types and IPC protocol
│   └── src/
│       └── lib.rs               # DeviceInfo, KeyCombo, Action, MacroEntry, RemapEntry
├── aethermapd/                # Privileged daemon
│   └── src/
│       ├── main.rs              # Entry point, component initialization
│       ├── device.rs            # DeviceManager, EVIOCGRAB logic
│       ├── remap.rs             # RemapEngine (NEW - key translation layer)
│       ├── macro_engine.rs      # MacroEngine (sequences, triggers)
│       ├── injector.rs          # UinputInjector trait and impl
│       ├── config.rs            # ConfigManager, profile persistence
│       ├── ipc.rs               # IPC server, request routing
│       └── security.rs          # SecurityManager, privilege dropping
├── aethermap-gui/             # User interface (GTK/Tauri)
│   └── src/
│       ├── main.rs              # GUI entry
│       ├── daemon_client.rs     # IPC client wrapper
│       └── ui/                  # UI components
└── aethermap-cli/             # Command-line tool
    └── src/
        └── main.rs              # CLI interface
```

### Structure Rationale

- **aethermap-common/**: Shared types ensure daemon and GUI speak the same protocol. IPC messages defined once.
- **aethermapd/src/remap.rs** (NEW): Dedicated remap engine separates concerns from macros. Remaps are simple 1:1 translations; macros are complex sequences.
- **aethermapd/src/macro_engine.rs**: Existing macro engine handles sequences. Can receive events from RemapEngine.
- **device.rs**: Device discovery and grabbing already implemented. Event channel feeds into remap layer first.
- **security.rs**: Privilege dropping after initialization minimizes attack surface.

## Architectural Patterns

### Pattern 1: Layered Event Processing

**What:** Events pass through remap layer before macro layer. Remaps are applied first; modified events then trigger macro detection.

**When to use:** When you need both simple key remapping AND complex macros. Remaps change the key code seen by the macro engine.

**Trade-offs:**
- Pro: Clean separation - remaps are simple lookups, macros are stateful
- Pro: Remaps can be hot-reloaded without affecting macro state
- Con: Slight latency overhead (two hash lookups instead of one)
- Con: Order matters - remapped keys change macro trigger behavior

**Example:**
```rust
// In main.rs event loop
tokio::spawn(async move {
    let mut event_receiver = event_receiver;
    loop {
        if let Some((device_path, key_code, pressed)) = event_receiver.recv().await {
            // Layer 1: Apply remap (translation)
            let remapped_code = remap_engine.apply_remap(key_code, &device_path).await;

            // Layer 2: Process macro with remapped key
            let state = state_clone.read().await;
            if let Some(macro_engine) = &state.macro_engine {
                macro_engine.process_input_event(remapped_code, pressed, &device_path).await;
            }

            // Layer 3: Inject output (if not consumed by macro)
            if !macro_engine.is_executing() {
                injector.key_event(remapped_code, pressed).await;
            }
        }
    }
});
```

### Pattern 2: Two-Phase Remap Lookup

**What:** Device-specific remaps checked first, then global remaps. Allows per-device overrides.

**When to use:** When different keyboards need different layouts (e.g., gaming keyboard vs laptop keyboard).

**Trade-offs:**
- Pro: Flexible configuration - one key behaves differently per device
- Pro: Can disable remaps for specific devices
- Con: More complex configuration UI
- Con: Two lookups per event (device map + global map)

**Example:**
```rust
impl RemapEngine {
    pub async fn apply_remap(&self, key_code: u16, device_path: &str) -> u16 {
        // Check device-specific remaps first
        if let Some(device_map) = self.device_remaps.get(device_path) {
            if let Some(&remapped) = device_map.get(&key_code) {
                return remapped;
            }
        }

        // Fall back to global remaps
        self.global_remaps.get(&key_code).copied().unwrap_or(key_code)
    }
}
```

### Pattern 3: Trie-Based Key Combo Storage

**What:** Use a trie (prefix tree) for multi-key remap combinations (e.g., Ctrl+C -> Escape).

**When to use:** When supporting chord or combo remaps beyond single-key translation.

**Trade-offs:**
- Pro: Efficient lookup for key sequences
- Pro: Handles overlapping combos naturally
- Con: More complex than HashMap for simple 1:1 remaps
- Con: Overkill if only single-key remaps needed

**Example:**
```rust
struct RemapTrie {
    children: HashMap<u16, RemapTrie>,
    remap_to: Option<u16>,  // Some if this node is a complete remap target
}

impl RemapTrie {
    pub fn lookup(&self, keys: &[u16]) -> Option<u16> {
        let mut current = self;
        for &key in keys {
            current = current.children.get(&key)?;
        }
        current.remap_to
    }
}
```

## Data Flow

### Request Flow (Client → Daemon)

```
[GUI Client]
    ↓ (Unix Socket: RemapEntry)
[IPC Server] → [Config Manager]
    ↓                              ↓
[Store to Profile]           [Hot-Reload Signal]
                                 ↓
                          [Remap Engine Updates]
                                 ↓
                          [Live Event Processing]
```

### Event Flow (Device → Output)

```
[Physical Device]
    ↓ (EVIOCGRAV)
[Device Manager] → [Event Channel]
                        ↓
                  [Remap Engine Layer]
                  (key_code lookup)
                        ↓
                  [Modified Event]
                        ↓
                  [Macro Engine Layer]
                  (trigger detection)
                        ↓
                  [Uinput Injector]
                  (virtual device write)
                        ↓
                  [Kernel Input Core]
                        ↓
                  [Applications receive]
```

### State Management

```
[Config Manager]───(read)──▶[Disk (YAML/TOML)]
       │
       ├──(load)──▶[Remap Engine: HashMap<u16, u16>]
       │
       └──(load)──▶[Macro Engine: HashMap<String, MacroEntry>

[Remap Engine]───(subscribe)──▶[Config Change Events]
                               │
                               └──▶[Hot-reload: swap maps atomically]
```

### Key Data Flows

1. **Configuration Load:** Disk → ConfigManager → RemapEngine (global_remaps) + MacroEngine (macros)
2. **Hot Reload:** inotify event → ConfigManager → atomic swap of RemapEngine maps → events use new remaps immediately
3. **Event Processing:** Device → RemapEngine (translate) → MacroEngine (detect trigger) → UinputInjector (output)

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 0-1k users (single user) | Single-threaded event loop is fine. HashMap remaps scale linearly. |
| 1k-100k users (power users) | Consider per-device remap caching. May need optimized trie if supporting complex combos. |
| 100k+ users (enterprise/gaming cafe) | Multi-threaded event processing per device. Distributed config (network profiles). |

### Scaling Priorities

1. **First bottleneck:** Event loop latency. HashMap lookups are O(1), but async overhead adds up.
   - Fix: Batch events, use lock-free structures for remap maps (crossbeam)
2. **Second bottleneck:** Configuration reload time. Large remap tables (10k+ entries) take time to parse.
   - Fix: Incremental config reload, mmap-based config storage

## Anti-Patterns

### Anti-Pattern 1: Remap Logic Inside Macro Engine

**What people do:** Add remap as a special "one-action macro" or modify MacroEngine to handle simple translations.

**Why it's wrong:** Macros and remaps are fundamentally different. Remaps are synchronous translations (A→B), macros are asynchronous sequences (A→B, C, D, sleep). Mixing them complicates both.

**Do this instead:** Create a dedicated RemapEngine that sits before MacroEngine in the event pipeline. Keep macro detection on remapped keys.

### Anti-Pattern 2: Blocking Event Loop for Config Reload

**What people do:** When config file changes, stop processing events to reload configuration.

**Why it's wrong:** Causes input lag during reload. For large configs, reload can take 100ms+.

**Do this instead:** Atomic swap pattern. Load new config into a separate HashMap, then atomic swap with active map using Arc::swap(). Events never blocked.

```rust
// Anti-pattern: blocks events
self.remaps = ConfigManager::load().await?;  // SLOW

// Better: atomic swap
let new_remaps = Arc::new(ConfigManager::load().await?);
std::mem::swap(&mut self.remaps, &mut new_remaps);
```

### Anti-Pattern 3: Global Device Grab Without Filters

**What people do:** Grab ALL input devices unconditionally.

**Why it's wrong:** Breaks other input devices (power buttons, special-purpose devices). Security risk.

**Do this instead:** Filter devices by capability (EV_KEY) and user configuration. Allow device blacklist/whitelist.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| systemd | Service unit with socket activation | RuntimeDirectory=/run/aethermap for socket |
| D-Bus (optional) | org.aethermap.Daemon interface | For desktop integration (status icon, notifications) |
| X11/Wayland | No direct integration needed | uinput events go through normal input path |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| GUI ↔ Daemon | Unix socket (bincode protocol) | Already implemented in ipc.rs |
| RemapEngine ↔ MacroEngine | Direct function call (in-event-loop) | RemapEngine returns u16, passed to MacroEngine |
| DeviceManager → RemapEngine | tokio::sync::mpsc channel | Event flow: device → channel → remap → macro |
| ConfigManager → Engines | Arc<RwLock<HashMap>> shared state | Atomic swap for hot-reload |

## Sources

### HIGH Confidence (Official Documentation)
- [keyd GitHub Repository](https://github.com/rvaiya/keyd) - Architecture reference for C-based remapping daemon
- [Linux Input Subsystem Documentation](https://www.kernel.org/doc/html/latest/input/input.html) - Official kernel input docs
- [evdev-rs Documentation](https://docs.rs/evdev/) - Rust evdev bindings
- [uinput-rs Documentation](https://docs.rs/uinput/) - Rust uinput virtual device creation

### MEDIUM Confidence (Technical Articles)
- [A Comprehensive Overview of the Linux Input Stack](https://venam.nixers.net/blog/unix/jotting/2022-12-30-linux-input.html) - Deep dive into evdev, uinput, input core

### LOW Confidence (Requires Verification)
- ArchWiki interception-tools page (access denied during research - verify independently)
- Specific performance benchmarks for remap engines (no current benchmarks found)

## Key Findings for Aethermap

### Current Architecture Alignment

The existing Aethermap architecture is well-aligned with standard patterns:

1. **DeviceManager**: Already implements device discovery and EVIOCGRAB grabbing
2. **MacroEngine**: Handles sequences and triggers correctly
3. **UinputInjector**: Virtual device output already implemented
4. **IPC Layer**: Unix socket communication established

### Missing Component: RemapEngine

**Recommendation:** Create new `aethermapd/src/remap.rs` with `RemapEngine` struct.

**Responsibilities:**
- Store remap tables: `HashMap<(device_path, key_code), remapped_code>`
- Apply remap: `pub async fn apply_remap(&self, key_code: u16, device_path: &str) -> u16`
- Hot-reload: Accept config updates and swap maps atomically
- Device filtering: Support per-device and global remaps

### Event Flow Integration

Insert RemapEngine between DeviceManager event channel and MacroEngine:

```rust
// In main.rs, modify the event processing loop:
tokio::spawn(async move {
    let mut event_receiver = event_receiver;
    loop {
        if let Some((device_path, key_code, pressed)) = event_receiver.recv().await {
            // NEW: Apply remap first
            let remapped_key = remap_engine.apply_remap(key_code, &device_path).await;

            // Then process macro (may consume event)
            let state = state.read().await;
            if let Some(macro_engine) = &state.macro_engine {
                let consumed = macro_engine.process_input_event(
                    remapped_key,
                    pressed,
                    &device_path
                ).await?;

                // Only inject if macro didn't consume the event
                if !consumed {
                    injector.key_event(remapped_key, pressed).await;
                }
            }
        }
    }
});
```

### Build Order Implications

1. **Phase 1:** Create RemapEngine with HashMap storage, no device filtering
2. **Phase 2:** Add RemapEntry to aethermap-common, extend IPC protocol
3. **Phase 3:** Integrate RemapEngine into main event loop (modify main.rs)
4. **Phase 4:** Add config persistence (extend ConfigManager)
5. **Phase 5:** Implement hot-reload (inotify + atomic swap)
6. **Phase 6:** Add per-device remaps and filtering
7. **Phase 7:** GUI support for remap configuration

### Configuration Structure (Recommended)

```yaml
profiles:
  default:
    remaps:
      global:
        30: 46   # A -> Enter (using evdev keycodes)
        46: 30   # Enter -> A (swap)
      devices:
        "/dev/input/event0":
          30: 48  # On device 0, A -> B
    macros:
      # ... existing macro structure ...
```

---
*Architecture research for: Linux Input Remapping System*
*Researched: 2026-02-16*
