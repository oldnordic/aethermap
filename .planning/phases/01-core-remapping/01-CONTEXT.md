# Phase 1: Core Remapping Engine - Context

**Gathered:** 2026-02-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the RemapEngine component that translates input key codes to output key codes. This is internal daemon infrastructure — users configure via YAML, the daemon performs translation in the event loop. The component must integrate with existing DeviceManager, MacroEngine, and UinputInjector.

</domain>

<decisions>
## Implementation Decisions

### Config file syntax
- Flat YAML structure with simple key:value pairs (e.g., `KEY_A: KEY_B`, `capslock: KEY_LEFTCTRL`)
- No section wrapper — all remaps at root level
- Invalid key names cause daemon to fail fast with detailed error message (file/line/position)
- Error output to stderr, exit code 1

### Key name format
- Internal storage uses `evdev::Key` enum directly (not integer codes)
- Thread-safe wrapper: `Arc<RwLock<HashMap<evdev::Key, evdev::Key>>>`
- Parse key names lazily on first use (not eager at config load)
- Case-insensitive parsing (KEY_A, key_a, Key_A all valid)
- Expand friendly abbreviations to evdev codes (capslock → KEY_CAPSLOCK, a → KEY_A)

### Remap storage
- Simple `HashMap<evdev::Key, evdev::Key>` for O(1) lookup
- Wrapped in `Arc<RwLock<>>` for concurrent access from event loop
- Lazy parsing: config stores strings, convert to enum on first lookup

### Error handling
- Config parse errors print detailed message with file/line/position to stderr
- Daemon exits with code 1 on invalid config (fail fast)
- No skipping invalid entries — all-or-nothing validation

### Claude's Discretion
- Exact friendly name expansion table (which abbreviations to support)
- HashMap vs IndexMap final decision (if ordering proves useful later)
- Error message format specifics (human-friendly vs machine-parseable)

</decisions>

<specifics>
## Specific Ideas

- "Keep the config as simple as possible — users should be able to open it and understand immediately"
- "Fail fast on bad config so users know immediately something is wrong"
- Support both technical (KEY_A) and casual (a) naming for accessibility

</specifics>

<deferred>
## Deferred Ideas

- Per-device profiles — Phase 2
- Runtime profile switching — Phase 2
- Config hot-reload — Phase 3
- Layers and tap-hold — Phase 2+

</deferred>

---

*Phase: 01-core-remapping*
*Context gathered: 2026-02-16*
