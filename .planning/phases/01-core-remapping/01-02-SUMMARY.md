---
phase: 01-core-remapping
plan: 02
subsystem: config
tags: [remap-config, yaml, validation, types]
wave: 2
dependency_graph:
  requires:
    - "01-01: KeyParser and RemapEngine"
  provides:
    - "RemapEntry type for type-safe remap configuration"
    - "load_remaps() for YAML remap file loading"
  affects:
    - "01-04: Daemon integration"
    - "01-05: Testing and documentation"

tech_stack:
  added:
    - "serde_yaml: for parsing remap configuration files"
  patterns:
    - "Flat YAML key:value pairs (no section wrapper)"
    - "Eager validation with fail-fast semantics"
    - "Arc<RwLock<T>> for concurrent access"

key_files:
  created:
    - "razermapper/razermapperd/src/config.rs: RemapEntry, RemapConfigError, load_remaps()"
  modified:
    - "razermapper/razermapperd/src/config.rs: added remaps_path, remaps fields to ConfigManager"
    - "razermapper/razermapperd/src/lib.rs: re-exported RemapEntry, RemapConfigError"

decisions:
  - "2026-02-17: Use flat YAML structure (KEY_A: KEY_B) without section wrapper for simplicity"
  - "2026-02-17: Validate ALL keys before returning (fail-fast per 01-01 decision)"
  - "2026-02-17: Create empty file if missing instead of erroring"
  - "2026-02-17: Manual error Display impl instead of thiserror (consistency with 01-01)"

metrics:
  duration: "0.5 hours"
  completed_date: "2026-02-17"
  tasks_completed: 3
---

# Phase 1 Plan 2: Remap Configuration Loading Summary

## One-Liner

Implemented YAML-based remap configuration loading with eager key validation using KeyParser for type-safe user-facing configuration layer.

## What Was Built

Extended ConfigManager to load key remap configuration from YAML files with complete key name validation.

### Type Definitions

**RemapEntry** (`razermapper/razermapperd/src/config.rs:12-17`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemapEntry {
    pub from: String,  // Input key name (e.g., "KEY_A", "a", "capslock")
    pub to: String,    // Output key name (e.g., "KEY_B", "KEY_LEFTCTRL")
}
```

**RemapConfigError** (`razermapper/razermapperd/src/config.rs:20-78`):
- `ReadError`: File I/O errors with path
- `ParseError`: YAML parsing errors with path
- `InvalidKey`: Key name validation errors with path, key name, and expected format
- `WriteError`: File write errors (for empty file creation)

### ConfigManager Extensions

**New Fields** (`razermapper/razermapperd/src/config.rs:25-26, 29`):
- `remaps_path: PathBuf` - Path to remaps.yaml (default: `/etc/razermapperd/remaps.yaml`)
- `remaps: Arc<RwLock<HashMap<String, String>>>` - Parsed remap storage

**New Method** (`razermapper/razermapperd/src/config.rs:533-615`):
- `load_remaps(&self) -> Result<Vec<RemapEntry>, RemapConfigError>`

### YAML Format

Flat key:value pairs (no section wrapper):
```yaml
# /etc/razermapperd/remaps.yaml
KEY_A: KEY_B
a: b
capslock: leftctrl
esc: grave
```

### Validation Behavior

1. **Empty file**: Creates new empty file if missing
2. **Eager validation**: ALL keys validated before any are accepted (fail-fast)
3. **Case-insensitive**: "key_a", "KEY_A", "Key_A" all map to KEY_A
4. **Friendly names**: "a", "capslock", "ctrl" work alongside standard KEY_* names

## Deviations from Plan

**None** - plan executed exactly as written.

## Testing

Build verification passed:
```bash
cargo check -p razermapperd
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.37s
```

## Key Design Decisions

1. **Flat YAML structure**: Chose simple key:value pairs over nested sections for user simplicity
2. **Fail-fast validation**: Consistent with 01-01 decision - invalid config causes immediate error
3. **Empty file creation**: Provides sensible default instead of requiring manual file creation
4. **Manual error impl**: Consistent with existing codebase patterns (no thiserror dependency)

## Next Steps

- **Plan 01-03**: Add CLI commands for remap management
- **Plan 01-04**: Integrate load_remaps into daemon startup
- **Plan 01-05**: Add comprehensive tests and documentation
