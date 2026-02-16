# Stack Research

**Domain:** Linux Key Remapping (Rust)
**Researched:** 2025-02-16
**Confidence:** HIGH

## Recommended Stack

### Core Technologies (Already Present)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| **evdev** | 0.12 | Linux input device access, event reading, KeyCode enum | Standard crate for evdev on Linux. Provides `KeyCode` enum with all Linux key codes (KEY_A, KEY_B, etc.). Actively maintained, follows kernel input-event-codes.h conventions. |
| **uinput-sys** | 0.1 | Low-level uinput bindings for virtual device creation | Required for creating virtual input devices to inject remapped events. Existing code already uses this correctly. |
| **tokio** | 1 (full features) | Async runtime for event processing | Already used for async device handling. Required for spawn_blocking around evdev's sync I/O. |
| **serde** | 1 (derive feature) | Serialization framework | Foundation for config file parsing. Already in workspace. |
| **bincode** | 1.3 | Binary IPC serialization | Already used for daemon-GUI IPC. Keep for fast IPC. |
| **nix** | 0.29 | Linux system calls (ioctl) | Required for EVIOCGRAB and other ioctls. Already present. |

### Key Remapping Additions

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| **evdev::KeyCode** | (via evdev 0.12) | Key code to name mapping, enum of all Linux keys | Use for internal key representation. The enum already provides `Key::KEY_A` etc. No separate mapping library needed. |
| **serde_yaml** | 0.9 | YAML config file parsing | Use for key remapping configuration format. Already in razermapperd dependencies. |
| **indexmap** | 2.0 | Ordered HashMap for key mapping tables | Use when remapping order matters (e.g., cascading remaps). Provides serde support. |

### Testing Utilities

| Library | Version | Purpose | Notes |
|---------|---------|---------|-------|
| **evdevil** | 0.3 | Virtual uinput device creation for tests | Creates real virtual devices for E2E testing. Tests require root/dev/uinput access. |
| **tempfile** | 3.0 | Temporary config files in tests | Already in workspace. Use for testing config loading. |
| **tokio-test** | 0.4 | Async test utilities | Already in dev-dependencies. |

### Configuration File Format

| Format | Library | Why |
|--------|---------|-----|
| **YAML** | serde_yaml 0.9 | Human-readable, supports nesting, already used in project. Follows xremap/keyd pattern for user familiarity. |
| **TOML** | toml | Alternative if simpler syntax desired. Not recommended - harder to express nested remap rules. |

## Installation

```toml
# Add to razermapperd/Cargo.toml if not already present:

[dependencies]
# Already present - keep these versions
evdev = "0.12"
uinput-sys = "0.1"
serde_yaml = "0.9"
nix = { version = "0.29", features = ["user", "ioctl", "fs"] }

# New additions for key remapping
indexmap = { version = "2.0", features = ["serde"] }

[dev-dependencies]
# For testing input device handling
evdevil = "0.3"
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| **evdev crate** | key-remapper crate | Only if you need held-vs-tapped distinction (CapsLock as Esc/modifier). For simple A→B remapping, evdev is sufficient. |
| **YAML (serde_yaml)** | TOML (toml crate) | Use TOML only if you want simpler, flatter config and don't need complex nested structures. YAML is more flexible for layers/macros. |
| **serde_yaml** | serde-yml | Avoid serde-yml - less mature, fewer examples. Stay with serde_yaml (established, widely used). |
| **uinput-sys** | evdev's uinput module | evdev removed uinput support in later versions. uinput-sys is the maintained path. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **keycode crate** | Cross-platform abstraction adds unnecessary complexity. Project is Linux-only. | evdev::KeyCode (native, no abstraction overhead) |
| **xkbcommon bindings** | Overkill for evdev-level remapping. XKB is for X11/Wayland compositor level. | Direct evdev key code translation |
| **libevdev-sys** | Low-level FFI bindings. evdev crate provides safe Rust wrapper. | evdev crate (safe Rust API) |
| **config-rs** | Over-engineered for this use case. ConfigManager already exists with serde_yaml. | Keep existing ConfigManager with serde_yaml |
| **JSON for config** | Less human-readable than YAML, no comments. Users want to document remaps. | YAML (serde_yaml) |

## Key Code Mapping Strategy

### Internal Representation
- Use `evdev::Key` (or `evdev::KeyCode` type alias) for all internal key handling
- The enum provides `Key::KEY_A`, `Key::KEY_ESC`, etc. directly
- Convert to/from `u16` using `.code()` method when needed for IPC/storage

### User-Facing Names
- Accept both names (e.g., "KEY_A", "a") and numeric codes in config
- Parse names using `evdev::Key::from_str()` or custom lookup
- Example config format:
```yaml
remap:
  - from: KEY_CAPSLOCK
    to: KEY_ESC
  - from: a
    to: b
```

### Remapping Logic Pattern
```rust
use evdev::Key;

// Simple key-to-key remap table
type RemapTable = IndexMap<Key, Key>; // Preserves order for cascading

// During event processing:
if let Some(remapped_key) = remap_table.get(&original_key) {
    inject_event(*remapped_key);
} else {
    inject_event(original_key);
}
```

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| evdev 0.12 | tokio 1 | Works via spawn_blocking for sync I/O |
| serde_yaml 0.9 | serde 1.0 | Standard serde derive compatibility |
| uinput-sys 0.1 | Linux kernel 3.0+ | uinput interface is stable |
| nix 0.29 | libc | Required for EVIOCGRAB ioctl constant |

## Missing Pieces Analysis

Based on the existing codebase review:

| Missing Component | Recommended Solution | Priority |
|-------------------|---------------------|----------|
| **Key name→code parsing** | Use `evdev::Key` enum's `from_str()` or implement lookup table | HIGH |
| **Remap configuration schema** | Extend existing `DaemonConfig` with `remap_rules: Vec<RemapRule>` | HIGH |
| **Remap application logic** | Add `Remapper` struct in new `remap.rs` module | HIGH |
| **Key code constants** | Already available via `evdev::Key` enum | N/A |
| **Testing utilities** | Add `evdevil` for virtual device E2E tests | MEDIUM |
| **Layer support (optional)** | Extend config to support layered remaps (like keyd) | LOW (future enhancement) |

## Sources

### HIGH Confidence (Context7/Official Docs)
- [/websites/rs_toml](https://context7.com/websites/rs_toml) — TOML crate with serde integration
- [/rust-cli/config-rs](https://context7.com/rust-cli/config-rs) — Layered configuration patterns (for reference, not adoption)
- [/websites/serde_rs](https://context7.com/websites/serde_rs) — Serde derive macros and attributes

### MEDIUM Confidence (Official Repositories/Docs)
- [evdev crate on docs.rs](https://docs.rs/evdev) — Official evdev crate documentation with KeyCode enum
- [evdev source on GitHub](https://github.com/emberian/evdev) — Source code defining KeyCode enum variants
- [xremap GitHub](https://github.com/xremap/xremap) — Rust key remapper using evdev/uinput, YAML config
- [Mastering xremap guide](https://www.paolomainardi.com/posts/linux-remapping-keys-with-xremap/) — YAML configuration format examples

### MEDIUM Confidence (Established Libraries)
- [evdevil on crates.io](https://crates.io/crates/evdevil) — Virtual device testing for evdev
- [key-remapper on lib.rs](https://lib.rs/crates/key-remapper) — Alternative held-vs-tapped remapper
- [keycode crate docs](https://docs.rs/keycode) — Cross-platform key mapping (avoid, but informative)

### LOW Confidence (Web Search - Verify Before Use)
- [keyd GitHub](https://github.com/rvaiya/keyd) — C-based remapper daemon, reference for features
- [keyd Arch manual pages](https://man.archlinux.org/man/extra/keyd/keyd.1.en) — Feature reference (layering, oneshot modifiers)

---
*Stack research for: Linux Key Remapping (Rust)*
*Researched: 2025-02-16*
