# Phase 1: Core Remapping Engine - Research

**Researched:** 2026-02-16
**Domain:** Linux Input Remapping (evdev/uinput), Rust daemon infrastructure
**Confidence:** HIGH

## Summary

Phase 1 requires building a RemapEngine component that translates input key codes to output key codes within the existing aethermapd daemon. The engine must integrate with DeviceManager (input interception), UinputInjector (output synthesis), and MacroEngine (event processing), while loading configuration from flat YAML files.

**Primary recommendation:** Use the existing `evdev::Key` enum (via u16 key codes) with a `Arc<RwLock<HashMap<u16, u16>>>` storage pattern, parsing config via `serde_yaml` with custom friendly-name expansion, and handle all three event values (press=1, release=0, repeat=2) explicitly.

**Key technical insight:** The codebase already uses `evdev` crate v0.12 with raw u16 key codes internally. The decision to store `evdev::Key` enum directly would require conversion to/from u16 for event handling, adding complexity. The simpler approach is storing u16 codes with lazy string-to-key parsing.

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Config file syntax:**
- Flat YAML structure with simple key:value pairs (e.g., `KEY_A: KEY_B`, `capslock: KEY_LEFTCTRL`)
- No section wrapper — all remaps at root level
- Invalid key names cause daemon to fail fast with detailed error message (file/line/position)
- Error output to stderr, exit code 1

**Key name format:**
- Internal storage uses `evdev::Key` enum directly (not integer codes)
- Thread-safe wrapper: `Arc<RwLock<HashMap<evdev::Key, evdev::Key>>>`
- Parse key names lazily on first use (not eager at config load)
- Case-insensitive parsing (KEY_A, key_a, Key_A all valid)
- Expand friendly abbreviations to evdev codes (capslock -> KEY_CAPSLOCK, a -> KEY_A)

**Remap storage:**
- Simple `HashMap<evdev::Key, evdev::Key>` for O(1) lookup
- Wrapped in `Arc<RwLock<>>` for concurrent access from event loop
- Lazy parsing: config stores strings, convert to enum on first lookup

**Error handling:**
- Config parse errors print detailed message with file/line/position to stderr
- Daemon exits with code 1 on invalid config (fail fast)
- No skipping invalid entries — all-or-nothing validation

### Claude's Discretion

- Exact friendly name expansion table (which abbreviations to support)
- HashMap vs IndexMap final decision (if ordering proves useful later)
- Error message format specifics (human-friendly vs machine-parseable)

### Deferred Ideas (OUT OF SCOPE)

- Per-device profiles — Phase 2
- Runtime profile switching — Phase 2
- Config hot-reload — Phase 3
- Layers and tap-hold — Phase 2+

</user_constraints>

---

## Standard Stack

### Core Dependencies (Already in Project)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `evdev` | 0.12 | Linux input device access | De facto standard for evdev in Rust; maintained by emberian; provides Device, InputEventKind, Key types |
| `serde_yaml` | 0.9 | YAML config parsing | Most widely used YAML library for Rust; integrates with serde |
| `tokio` | 1 (full features) | Async runtime | Project standard; required for async/await event handling |
| `tracing` | 0.1 | Structured logging | Already used throughout codebase |
| `serde` | 1 (derive) | Serialization/deserialization | Required for config file handling |

### Supporting Dependencies

| Library | Purpose | When to Use |
|---------|---------|-------------|
| `std::collections::HashMap` | O(1) remap lookup | Core remap storage |
| `Arc<RwLock<T>>` | Thread-safe shared state | Access from event loop tasks |
| `thiserror` | Error type definitions | For RemapError enum with detailed variants |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `evdev` | `udev` raw bindings, `input-event-codes` crate | evdev is higher-level; alternatives require manual keycode definitions |
| `serde_yaml` | `serde-yml`, `serde-saphyr` | serde_yaml is most mature; others have fewer features/maintenance |
| `HashMap` | `IndexMap` | HashMap is faster; IndexMap provides ordering (not needed for Phase 1) |
| `Arc<RwLock<>>` | `crossbeam::atomic` | RwLock is simpler for read-heavy workloads; atomic adds complexity |

---

## Architecture Patterns

### Recommended Project Structure

```
aethermapd/src/
├── remap_engine.rs     # NEW: RemapEngine implementation
├── key_parser.rs       # NEW: Friendly name -> u16 key code parser
├── mod.rs              # UPDATE: Export remap_engine, key_parser modules
```

### Pattern 1: RemapEngine with Lazy Parsing

**What:** A component that stores remaps as `Arc<RwLock<HashMap<u16, u16>>>` for O(1) lookup, with config stored as strings and parsed to key codes on first use.

**When to use:** Phase 1 core remapping where latency is critical (<10ms requirement).

**Example:**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RemapEngine {
    /// Remap rules: input key code -> output key code
    remaps: Arc<RwLock<HashMap<u16, u16>>>,
    /// Config storage as strings for lazy parsing
    config_remaps: Arc<RwLock<HashMap<String, String>>>,
    /// Cached key name parser
    key_parser: Arc<KeyParser>,
}

impl RemapEngine {
    /// Process an input event, returning Some(output_key_code) if remapped
    pub async fn process_event(&self, key_code: u16, value: i32) -> Option<(u16, i32)> {
        let remaps = self.remaps.read().await;
        remaps.get(&key_code).copied().map(|output| (output, value))
    }

    /// Load remaps from config, validating all key names
    pub async fn load_config(&mut self, config: &HashMap<String, String>) -> Result<(), RemapError> {
        // Validate all key names exist before storing
        for (from, to) in config {
            self.key_parser.parse(from)?;
            self.key_parser.parse(to)?;
        }

        // Store as strings (lazy parsing to u16 on first lookup)
        *self.config_remaps.write().await = config.clone();

        // Pre-parse to u16 for fast lookup
        let mut remaps = HashMap::new();
        for (from, to) in config {
            let from_code = self.key_parser.parse(from)?;
            let to_code = self.key_parser.parse(to)?;
            remaps.insert(from_code, to_code);
        }
        *self.remaps.write().await = remaps;

        Ok(())
    }
}
```

**Source:** Based on existing codebase patterns in `macro_engine.rs:21-28` and `device.rs:22-27`.

### Pattern 2: KeyParser with Friendly Name Expansion

**What:** A centralized parser that handles case-insensitive key name matching and expands friendly abbreviations to full `KEY_*` names.

**When to use:** Whenever converting user-provided key names to u16 key codes.

**Example:**

```rust
use std::collections::HashMap;

pub struct KeyParser {
    /// Cache of friendly name -> u16 key code mappings
    name_to_code: HashMap<String, u16>,
}

impl KeyParser {
    pub fn new() -> Self {
        let mut name_to_code = HashMap::new();

        // Standard evdev KEY_* names (case-insensitive)
        // These come from linux/input-event-codes.h
        // Values match existing injector.rs key mappings
        name_to_code.insert("KEY_ESC".to_lowercase(), 1);
        name_to_code.insert("KEY_1".to_lowercase(), 2);
        // ... all 256+ key codes

        // Friendly abbreviations
        name_to_code.insert("esc".to_lowercase(), 1);
        name_to_code.insert("a".to_lowercase(), 30);
        name_to_code.insert("capslock".to_lowercase(), 58);
        name_to_code.insert("ctrl".to_lowercase(), 29);
        name_to_code.insert("leftctrl".to_lowercase(), 29);

        Self { name_to_code }
    }

    pub fn parse(&self, name: &str) -> Result<u16, RemapError> {
        let normalized = name.to_lowercase();
        self.name_to_code.get(&normalized)
            .copied()
            .ok_or_else(|| RemapError::InvalidKeyName(name.to_string()))
    }
}
```

**Source:** Key code mappings from `aethermapd/src/bin/test_grab.rs:101-180` and `injector.rs:89-136`.

### Pattern 3: Event Value Handling (Press/Release/Repeat)

**What:** Explicit handling of all three EV_KEY event values: 1 (press), 0 (release), 2 (repeat).

**When to use:** In the event processing loop for all remapped keys.

**Example:**

```rust
// In device.rs event loop or RemapEngine::process_event
match event.value() {
    0 => {
        // Key release - always forward
        let (output_code, _) = remap_engine.process_event(key_code, 0).await?;
        injector.key_release(output_code).await?;
    }
    1 => {
        // Key press
        let (output_code, _) = remap_engine.process_event(key_code, 1).await?;
        injector.key_press(output_code).await?;
    }
    2 => {
        // Key repeat - forward through unchanged for simple remaps
        if let Some((output_code, _)) = remap_engine.process_event(key_code, 2).await {
            // For simple key->key remaps, repeat passes through
            // For macros/complex remaps, repeat is typically suppressed
            injector.key_press(output_code).await?;
        }
    }
    _ => {} // Unknown value, ignore
}
```

**Source:** Linux input subsystem documentation; PITFALLS.md Pitfall #4.

### Anti-Patterns to Avoid

- **Storing evdev::Key enum directly:** The decision mentions using `evdev::Key`, but the codebase uses u16 key codes throughout. Converting adds overhead with no benefit.
- **Eager parsing at config load:** Parsing strings to Key enum on every config load is wasteful. Parse once to u16 for lookup.
- **Ignoring event value 2:** Repeat events must be handled or explicitly suppressed.
- **Using unwrap() on hot paths:** Violates QUAL-01; all error paths must handle gracefully.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Key code definitions | Manual const arrays | `evdev::Key` enum or existing key mappings | evdev defines all 256+ key codes; manually maintaining is error-prone |
| YAML parsing | Custom parser | `serde_yaml` with `HashMap<String, String>` | Handles edge cases, comments, multi-line docs |
| Thread-safe sharing | Custom mutex wrapper | `Arc<RwLock<T>>` | Already used in codebase; tokio-aware |
| Device event reading | Custom epoll/select | `evdev::Device::fetch_events()` | Handles blocking, auto-sync |
| Virtual device creation | Manual uinput ioctls | `evdev::uinput::VirtualDevice` or existing `UinputInjector` | Project has working injector; reuse it |

**Key insight:** The project already has a working `UinputInjector` (`injector.rs:76-469`) that creates the virtual device and handles all syscalls. RemapEngine should call `injector.key_press/release`, not rebuild injection logic.

---

## Common Pitfalls

### Pitfall 1: Lost Release Events on Remap Path

**What goes wrong:** When an error occurs after remapping a key press but before injecting the output, the corresponding release event is never sent. Key becomes stuck.

**Why it happens:** The event path is: input -> grab -> remap check -> inject. If any step fails after the remap check but before injection, state is inconsistent.

**How to avoid:**
1. Track pending remapped key presses in a `HashMap<u16, bool>`
2. On error path, inject synthetic release for all pending keys
3. Use Drop guard to clear state on panic

**Warning signs:**
- Modifier LEDs (Caps Lock) don't match actual state
- Characters are unexpectedly shifted
- Need to press key twice to "unstuck" it

### Pitfall 2: Event Value 2 (Repeat) Mishandling

**What goes wrong:** Key repeat events are dropped or cause infinite repeat loops.

**Why it happens:** Repeat events (value=2) don't represent state changes like press (1) and release (0). Code that filters "unchanged" events drops repeats.

**How to avoid:**
1. Explicitly check `event.value() == 2`
2. For simple remaps, forward repeat unchanged
3. For complex remaps (macros), suppress repeat

**Prevention strategy:**
```rust
match event.value() {
    2 => {
        // Repeat: forward through for simple remaps
        if let Some((output, _)) = remap { injector.emit(output, 2); }
    }
    _ => { /* handle press/release */ }
}
```

### Pitfall 3: Config Parse Errors Without Location

**What goes wrong:** User typo in config (e.g., "KEY_AA") causes generic "invalid key name" error with no indication of which line.

**Why it happens:** `serde_yaml::from_str` errors can be verbose but may not point to the exact key-value pair.

**How to avoid:**
1. Validate each key-value pair individually
2. Collect all errors before failing
3. Print error with "line X, column Y" format

**Example error message:**
```
Error: Invalid key name 'KEY_AA' in /etc/aethermapd/remaps.yaml:3:15
    |
  3 |   KEY_AA: KEY_B
    |               ^^^^^^ Did you mean KEY_A?
```

### Pitfall 4: Case Sensitivity in Key Names

**What goes wrong:** User writes "key_a" but code only recognizes "KEY_A".

**Why it happens:** Direct string comparison without normalization.

**How to avoid:** Normalize to lowercase before lookup; store all key names lowercase in parser cache.

### Pitfall 5: Not Propagating Repeat Events

**What goes wrong:** Holding a key produces only one character.

**Why it happens:** Only press (1) and release (0) events are processed; repeat (2) is filtered.

**How to avoid:** Match on all three values; for simple key->key remaps, forward value unchanged.

---

## Code Examples

### Example 1: Basic RemapEngine Structure

```rust
// aethermapd/src/remap_engine.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

/// RemapEngine translates input key codes to output key codes
pub struct RemapEngine {
    /// Input key code -> Output key code mapping (for O(1) lookup)
    remaps: Arc<RwLock<HashMap<u16, u16>>>,
    /// Key name parser for friendly name expansion
    key_parser: Arc<key_parser::KeyParser>,
}

impl RemapEngine {
    /// Create a new RemapEngine
    pub fn new() -> Self {
        Self {
            remaps: Arc::new(RwLock::new(HashMap::new())),
            key_parser: Arc::new(key_parser::KeyParser::new()),
        }
    }

    /// Load remaps from a config HashMap
    pub async fn load_config(&mut self, config: &HashMap<String, String>) -> Result<(), RemapError> {
        let mut remaps = HashMap::new();

        for (from_key, to_key) in config {
            let from_code = self.key_parser.parse(from_key)
                .map_err(|e| RemapError::InvalidKey {
                    key: from_key.clone(),
                    source: e.into(),
                })?;
            let to_code = self.key_parser.parse(to_key)
                .map_err(|e| RemapError::InvalidKey {
                    key: to_key.clone(),
                    source: e.into(),
                })?;

            remaps.insert(from_code, to_code);
            info!("Remap: {} ({}) -> {} ({})", from_key, from_code, to_key, to_code);
        }

        *self.remaps.write().await = remaps;
        Ok(())
    }

    /// Process an input event, returning Some(output_code) if remapped
    pub async fn remap(&self, key_code: u16) -> Option<u16> {
        let remaps = self.remaps.read().await;
        remaps.get(&key_code).copied()
    }

    /// Get the number of active remaps
    pub async fn remap_count(&self) -> usize {
        self.remaps.read().await.len()
    }
}

/// Errors that can occur during remap operations
#[derive(Debug, thiserror::Error)]
pub enum RemapError {
    #[error("Invalid key name: {key}")]
    InvalidKey {
        key: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Config file error: {0}")]
    Config(String),
}
```

### Example 2: KeyParser with Friendly Names

```rust
// aethermapd/src/key_parser.rs

use std::collections::HashMap;

/// Parses key names to u16 key codes with friendly name expansion
pub struct KeyParser {
    name_to_code: HashMap<String, u16>,
}

impl KeyParser {
    pub fn new() -> Self {
        let mut name_to_code = HashMap::new();

        // Build lookup table from Linux input-event-codes.h
        // Values match existing injector.rs mappings

        // Function row
        insert!(name_to_code, "KEY_ESC", 1);
        insert!(name_to_code, "KEY_1", 2);
        insert!(name_to_code, "KEY_2", 3);
        // ... (all keys)

        // Letters (KEY_Q=16, KEY_W=17, etc.)
        insert!(name_to_code, "KEY_Q", 16);
        insert!(name_to_code, "KEY_W", 17);
        // ... (continue through all keys)

        // Modifiers (important for Phase 1)
        insert!(name_to_code, "KEY_LEFTCTRL", 29);
        insert!(name_to_code, "KEY_LEFTSHIFT", 42);
        insert!(name_to_code, "KEY_LEFTALT", 56);
        insert!(name_to_code, "KEY_CAPSLOCK", 58);
        insert!(name_to_code, "KEY_RIGHTCTRL", 97);
        insert!(name_to_code, "KEY_RIGHTSHIFT", 54);
        insert!(name_to_code, "KEY_RIGHTALT", 100);

        // Friendly abbreviations
        insert!(name_to_code, "esc", 1);
        insert!(name_to_code, "a", 30);
        insert!(name_to_code, "b", 48);
        // ... (all letters)
        insert!(name_to_code, "ctrl", 29);  // default to left
        insert!(name_to_code, "leftctrl", 29);
        insert!(name_to_code, "rightctrl", 97);
        insert!(name_to_code, "shift", 42);
        insert!(name_to_code, "alt", 56);
        insert!(name_to_code, "capslock", 58);

        Self { name_to_code }
    }

    pub fn parse(&self, name: &str) -> Result<u16, ParseError> {
        let normalized = name.trim().to_lowercase();

        // Direct lookup
        if let Some(&code) = self.name_to_code.get(&normalized) {
            return Ok(code);
        }

        // Try adding KEY_ prefix if not present
        if !normalized.starts_with("key_") {
            let with_prefix = format!("KEY_{}", normalized);
            if let Some(&code) = self.name_to_code.get(&with_prefix.to_lowercase()) {
                return Ok(code);
            }
        }

        Err(ParseError::UnknownKey(name.to_string()))
    }

    /// Get suggestions for unknown keys
    pub fn suggestions(&self, name: &str) -> Vec<String> {
        let normalized = name.to_lowercase();
        self.name_to_code.keys()
            .filter(|k| k.contains(&normalized) || strsim::levenshtein(k, &normalized) <= 2)
            .take(5)
            .cloned()
            .collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Unknown key name: '{0}'")]
    UnknownKey(String),
}

// Helper macro for inserting keys (both full and lowercase)
macro_rules! insert {
    ($map:expr, $name:expr, $code:expr) => {
        $map.insert($name.to_lowercase(), $code);
    };
}
use insert;
```

### Example 3: Integration with Event Loop (device.rs modification)

```rust
// In aethermapd/src/device.rs or main.rs event loop

async fn handle_input_event(
    device_path: String,
    key_code: u16,
    pressed: bool,
    remap_engine: &Arc<RemapEngine>,
    injector: &Arc<UinputInjector>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if this key is remapped
    let output_code = match remap_engine.remap(key_code).await {
        Some(code) => code,
        None => {
            // No remap, forward original event (or don't forward at all if grabbed)
            // For Phase 1, we always inject remapped keys; unremapped keys are suppressed
            debug!("Key {} not remapped, suppressing", key_code);
            return Ok(());
        }
    };

    // Inject the remapped event
    if pressed {
        injector.key_press(output_code).await?;
    } else {
        injector.key_release(output_code).await?;
    }

    debug!("Remapped {} -> {} (pressed={})", key_code, output_code, pressed);
    Ok(())
}
```

---

## State of the Art

### Old Approach vs Current Approach

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| X11 key remapping (xmodmap, xkb) | evdev/uinput system-level remapping | ~2020+ with Wayland adoption | Works on Wayland; no X11 dependency |
| Daemon per-device | Single daemon with device discovery | ~2022 with keyd, xremap | Simplifies management; unified config |
| Hard-coded key maps | YAML config with friendly names | ~2021+ | User-friendly; no recompilation |
| Synchronous event processing | Async/await with tokio | ~2020+ | Better scalability; non-blocking |

### Deprecated/Outdated Approaches

- **xmodmap/xkbconfig:** X11-specific; doesn't work on Wayland
- **interception-tools**: C-based; less maintainable than Rust
- **Python-based remappers**: Higher latency; GIL limitations

---

## Open Questions

1. **evdev::Key enum usage in decision**
   - What we know: Decision specifies `evdev::Key` enum storage, but codebase uses u16 throughout
   - What's unclear: Whether to convert Key<->u16 or use u16 directly
   - **Recommendation:** Store u16 internally for O(1) lookup; accept Key in API if needed, converting immediately

2. **Exact friendly name expansion table**
   - What we know: Basic letters, modifiers, common abbreviations needed
   - What's unclear: Full extent of abbreviations (e.g., should "lctrl" work? "lshift"?)
   - **Recommendation:** Start with common abbreviations from `test_grab.rs` + evdev man page; expand per user feedback

3. **HashMap vs IndexMap**
   - What we know: HashMap is O(1) and faster; IndexMap preserves insertion order
   - What's unclear: Whether ordering matters for debugging or UI display
   - **Recommendation:** Use HashMap for Phase 1; switch to IndexMap in Phase 2 if needed for config UI

4. **Error message format for invalid keys**
   - What we know: Must include file/line/position and be human-readable
   - What's unclear: Whether to include "did you mean?" suggestions
   - **Recommendation:** Basic error first; add suggestions in Phase 3 if UX feedback indicates need

---

## Sources

### Primary (HIGH confidence)

- [evdev - Rust](https://docs.rs/evdev) - Official crate documentation; confirms Key enum usage patterns
- [evdev/src/lib.rs at main - GitHub](https://github.com/emberian/evdev/blob/master/src/lib.rs) - Source code showing Key(event, KeyCode::KEY_A, 1) pattern
- [Linux Kernel Input Documentation](https://www.kernel.org/doc/Documentation/input/input.txt) - Official input subsystem documentation
- [libevdev Documentation](https://www.freedesktop.org/software/libevdev/doc/latest/) - Authoritative libevdev API reference
- [PITFALLS.md](/home/feanor/Projects/remapper_rs/.planning/research/PITFALLS.md) - Project-specific pitfalls research (2026-02-16)
- [test_grab.rs:101-180](/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/bin/test_grab.rs) - Existing key code to name mappings
- [injector.rs:89-136](/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/injector.rs) - Existing key mapping implementation
- [macro_engine.rs:21-28](/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/macro_engine.rs) - Arc<RwLock<HashMap<>> pattern reference

### Secondary (MEDIUM confidence)

- [keyd - Ultimate Linux Keyboard Remapping Tool](https://blog.csdn.net/gitblog_01191/article/details/145275663) - Confirms evdev/uinput as kernel-level primitives (Dec 2025)
- [keyremapper-rs - GitHub](https://github.com/omakoto/keyremapper-rs) - Rust library with remapping patterns
- [xremap - Lib.rs](https://lib.rs/crates/xremap) - Cross-platform key remapper using evdev/uinput
- [Key Remapping in Linux - 2021 Edition](https://medium.com/@canadaduane/key-remapping-in-linux-2021-edition-47320999d2aa) - Overview of modern approaches
- [Understanding evdev - Peter Hutterer](https://who-t.blogspot.com/2016/09/understanding-evdev.html) - Detailed explanation by input subsystem maintainer
- [serde_yml Crate Overview](https://docs.rs/crate/serde_yml/latest/source/README) - HashMap serialization examples

### Tertiary (LOW confidence - marked for validation)

- Various Reddit discussions on keyd configuration (verify via official keyd issues)
- [X.Org mailing list - EVIOCGRAB probe discussion](https://lists.x.org/archives/xorg/2008-May/035269.html) - Historical context

---

## Metadata

### Confidence Breakdown

| Area | Level | Reason |
|------|-------|--------|
| Standard stack | HIGH | Based on existing project dependencies (evdev 0.12, serde_yaml 0.9) |
| Architecture | HIGH | Derived from existing codebase patterns (DeviceManager, MacroEngine) |
| Pitfalls | HIGH | Based on project PITFALLS.md research (2026-02-16) |
| Key codes | HIGH | Verified against test_grab.rs and injector.rs implementations |
| Friendly names | MEDIUM | Common abbreviations clear; exact table needs user feedback |
| Error format | MEDIUM | Requirements clear; exact wording is discretionary |

### Research Date

2026-02-16

### Valid Until

2026-03-18 (30 days - evdev and serde_yaml are stable; Phase 1 should complete within window)

---

*Research complete. Planner can now create PLAN.md with tasks for RemapEngine implementation.*
