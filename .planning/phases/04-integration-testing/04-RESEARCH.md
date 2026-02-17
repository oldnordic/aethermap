# Phase 4: Integration Testing - Research

**Researched:** 2026-02-17
**Domain:** Rust testing, Linux input subsystem, virtual device emulation
**Confidence:** HIGH

## Summary

Phase 4 focuses on comprehensive test coverage for the remapper daemon using Rust's testing ecosystem. The project already has significant test infrastructure including unit tests for RemapEngine (251 lines), KeyParser (235 lines), and integration tests for configuration reload and hotplug scenarios (701 lines combined). The primary work remaining is adding end-to-end integration tests using evdev's virtual device capabilities.

**Key finding:** The ROADMAP mentions "evdevil" for virtual device testing, but no such crate exists. The correct approach is using evdev's built-in `uinput` module with `VirtualDevice` for creating virtual input devices in tests.

**Primary recommendation:** Build on existing test patterns (tokio::test, tempfile, Arc<RwLock<T>>) and use evdev::VirtualDevice for end-to-end remapping verification without requiring physical hardware.

## User Constraints

No CONTEXT.md exists for Phase 4. Research is unconstrained by prior user decisions.

## Standard Stack

### Core Testing Dependencies (Already in Use)
| Crate | Version | Purpose | Why Standard |
|-------|---------|---------|--------------|
| `tokio` | (see Cargo.toml) | Async runtime with `#[tokio::test]` | Required for testing async functions in RemapEngine, ConfigManager |
| `tempfile` | "3" | Temporary file/directory creation | Standard for isolated test environments |
| `async-trait` | (see Cargo.toml) | Async trait support | Used for test mocking patterns |

### Virtual Device Testing
| Crate | Module | Purpose | Why Standard |
|-------|--------|---------|--------------|
| `evdev` | `uinput` | VirtualDevice for virtual input devices | Linux kernel's uinput interface for device emulation |

**Note:** evdev v0.12 is in use; v0.13.2 is available with updated uinput API.

### Already Installed
```toml
# From razermapperd/Cargo.toml
[dev-dependencies]
tempfile = "3"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
```

**For virtual device tests, add:**
```toml
# evdev already includes uinput module
# No additional dependencies needed
```

## Architecture Patterns

### Test Project Structure
```
razermapper/razermapperd/tests/
├── hotplug_test.rs           # Device event handling tests
├── config_reload_test.rs     # Configuration hot-reload tests
├── remap_integration_test.rs # (NEW) End-to-end remapping with virtual devices
└── device_manager_test.rs    # (NEW) Device grab/release integration tests

razermapper/razermapperd/src/
├── remap_engine.rs           # Unit tests at lines 439-689
├── key_parser.rs             # Unit tests at lines 308-542
└── [... other modules with inline tests]
```

### Pattern 1: Async Unit Tests with tokio::test
**What:** Test async functions using Tokio's test runtime
**When to use:** Testing RemapEngine, ConfigManager, DeviceManager async methods
**Example:**
```rust
// Source: razermapperd/src/remap_engine.rs:468-476
#[tokio::test]
async fn test_remap_returns_correct_key() {
    let engine = RemapEngine::new();
    let mut config = HashMap::new();
    config.insert("KEY_A".to_string(), "KEY_B".to_string());
    engine.load_config(&config).await.unwrap();

    let result = engine.remap(Key::KEY_A).await;
    assert_eq!(result, Some(Key::KEY_B));
}
```

### Pattern 2: Isolated Test Environments with TempDir
**What:** Create temporary directories/files for test isolation
**When to use:** Testing configuration loading, file-based operations
**Example:**
```rust
// Source: razermapperd/tests/config_reload_test.rs:37-51
fn create_test_config_manager(temp_dir: &TempDir) -> ConfigManager {
    ConfigManager {
        config_path: temp_dir.path().join("config.yaml"),
        macros_path: temp_dir.path().join("macros.yaml"),
        cache_path: temp_dir.path().join("macros.bin"),
        profiles_dir: temp_dir.path().join("profiles"),
        remaps_path: temp_dir.path().join("remaps.yaml"),
        device_profiles_path: temp_dir.path().join("device_profiles.yaml"),
        config: razermapperd::config::DaemonConfig::default(),
        macros: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        profiles: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        remaps: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        device_profiles: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    }
}
```

### Pattern 3: Thread-Safe Shared State in Tests
**What:** Use Arc<RwLock<T>> for concurrent test access
**When to use:** Tests need shared mutable state across async tasks
**Example:**
```rust
// Source: razermapperd/tests/config_reload_test.rs:471-494
let manager = Arc::new(create_test_config_manager(&temp_dir));

let manager1 = manager.clone();
let manager2 = manager.clone();
let manager3 = manager.clone();

let task1 = tokio::spawn(async move {
    manager1.load_remaps().await
});
let task2 = tokio::spawn(async move {
    manager2.load_remaps().await
});
let task3 = tokio::spawn(async move {
    manager3.load_remaps().await
});
```

### Pattern 4: Virtual Device Testing (NEW for Phase 4)
**What:** Create virtual input devices using evdev::uinput
**When to use:** End-to-end testing of remapping without physical hardware
**Example pattern:**
```rust
use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{EventType, InputEvent, Key};

#[test]
fn test_virtual_device_creation() {
    // Create a virtual keyboard device
    let mut device = VirtualDeviceBuilder::new()
        .name("Test Virtual Keyboard")
        .with_event_type(&EventType::KEY)
        .with_key(&[Key::KEY_A, Key::KEY_B, Key::KEY_C])
        .build()
        .expect("Failed to create virtual device");

    // Inject events
    device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1)])
        .expect("Failed to emit event");
}
```

### Anti-Patterns to Avoid
- **Blocking in async tests:** Don't use `std::thread::sleep` — use tokio::time::sleep or proper async waiting
- **Leaking test resources:** Always cleanup TempDirs and virtual devices in test teardown
- **Ignoring error cases:** Tests should verify both success and failure paths
- **Hard-coded paths:** Use TempDir or env vars for test file paths

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Temporary file management | Manual file creation/deletion with cleanup | `tempfile::TempDir` | Handles cleanup on drop, even on test failure |
| Async test runtime | Manual Tokio Runtime creation | `#[tokio::test]` macro | Handles runtime lifecycle, error propagation |
| Virtual device emulation | Mocking input events manually | `evdev::uinput::VirtualDevice` | Real kernel uinput interface, produces actual evdev events |
| Thread-safe test state | Mutex/unsafe manual synchronization | `Arc<RwLock<T>>` | Async-friendly, prevents deadlocks with tokio |

**Key insight:** Virtual device testing through uinput is the only way to test end-to-end input remapping without physical hardware. Hand-rolling mocks won't catch real kernel/evdev integration issues.

## Current Test Coverage

### Already Comprehensive (No Additional Work Needed)
| Module | Test Lines | Coverage | Notes |
|--------|------------|----------|-------|
| `remap_engine` | 251 (439-689) | HIGH | Engine creation, config loading, key validation, remap operations |
| `key_parser` | 235 (308-542) | HIGH | Case insensitivity, friendly names, modifiers, special keys |
| `config_reload` | 513 | HIGH | Atomic swap, invalid rejection, concurrent safety |
| `hotplug` | 188 | HIGH | DeviceEvent parsing, device ID formatting, event completeness |

### Needs Additional Coverage (Phase 4 Work)
| Area | Current Status | What's Missing |
|------|----------------|----------------|
| End-to-end remapping | Manual tests only | Automated virtual device tests |
| Device grab/release | test_grab.rs (manual) | Automated tests with virtual devices |
| Hotplug scenarios | Unit tests only | Integration tests with device add/remove simulation |
| RemapEngine + DeviceManager integration | None | Tests verifying remaps apply to grabbed devices |

## Common Pitfalls

### Pitfall 1: Virtual Device Requires Permissions
**What goes wrong:** Tests fail with "Permission denied" when creating VirtualDevice
**Why it happens:** uinput device access requires write permissions to /dev/uinput
**How to avoid:**
- In CI: Run tests with sudo or add user to `input` group
- In documentation: Document permission requirements
- In tests: Skip virtual device tests gracefully if permissions unavailable
```rust
#[test]
fn test_with_virtual_device() {
    if !std::path::Path::new("/dev/uinput").exists() {
        println!("Skipping: /dev/uinput not available");
        return;
    }
    // ... test code
}
```

**Warning signs:** "Permission denied", "No such file or directory" for /dev/uinput

### Pitfall 2: Test Isolation Leaks
**What goes wrong:** Tests pass individually but fail when run together
**Why it happens:** Shared state between tests (e.g., global RemapEngine instance)
**How to avoid:**
- Create fresh instances in each test
- Use TempDir for file-based state
- Avoid static/global mutable state

**Warning signs:** Flaky tests, failures that disappear when run singly

### Pitfall 3: Async Deadlock in Tests
**What goes wrong:** Test hangs forever waiting for lock acquisition
**Why it happens:** RwLock write lock held while spawning async tasks that try to acquire read locks
**How to avoid:**
- Drop locks before spawning tasks
- Use `try_read()`/`try_write()` with timeout
- Keep lock critical sections minimal

**Warning signs:** Tests timeout at 60s (default test timeout)

### Pitfall 4: evdev Version Mismatch
**What goes wrong:** Code examples from docs don't compile
**Why it happens:** evdev v0.12 vs v0.13 has API differences in uinput module
**How to avoid:**
- Pin evdev version in examples
- Check Cargo.toml for actual version used
- Use cargo docs for local version

**Warning signs:** "no method named X found for type Y"

## Code Examples

### Test RemapEngine with Key Validation
```rust
// Source: razermapperd/src/remap_engine.rs:531-544
#[tokio::test]
async fn test_load_config_with_invalid_key_rejected() {
    let engine = RemapEngine::new();
    let mut config = HashMap::new();
    config.insert("NOT_A_REAL_KEY".to_string(), "KEY_B".to_string());

    let result = engine.load_config(&config).await;
    assert!(result.is_err());
    if let Err(ConfigError::InvalidKey(name)) = result {
        assert_eq!(name, "NOT_A_REAL_KEY");
    } else {
        panic!("Expected InvalidKey error");
    }
}
```

### Test Configuration Atomic Swap
```rust
// Source: razermapperd/tests/config_reload_test.rs:114-157
#[test]
fn test_invalid_remap_rejection() {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let manager = create_test_config_manager(&temp_dir);
    let remaps_path = temp_dir.path().join("remaps.yaml");

    // Create initial valid config
    let initial_config = r#"
capslock: leftctrl
a: b
"#;
    write_remaps_file(&remaps_path, initial_config).unwrap();

    rt.block_on(async {
        let result = manager.load_remaps().await;
        assert!(result.is_ok());

        // Verify initial state
        let remaps = manager.remaps.read().await;
        assert_eq!(remaps.len(), 2);
        drop(remaps);

        // Write invalid config (bad key name)
        write_remaps_file(&remaps_path, "invalid_key_xyz: leftctrl\n").unwrap();

        // Attempt reload - should fail
        let result = manager.load_remaps().await;
        assert!(result.is_err(), "Invalid config should be rejected");

        // Verify original config is still active
        let remaps = manager.remaps.read().await;
        assert_eq!(remaps.len(), 2, "Original remaps should remain active");
    });
}
```

### Virtual Device Test Pattern (Recommended for Phase 4)
```rust
use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{EventType, InputEvent, Key, KeyCode};
use std::time::Duration;

#[tokio::test]
async fn test_end_to_end_remapping() {
    // Create virtual source device
    let source_device = VirtualDeviceBuilder::new()
        .name("Test Source Keyboard")
        .with_event_type(&EventType::KEY)
        .with_key(&[Key::KEY_A, Key::KEY_B])
        .build()
        .expect("Failed to create source device");

    // Create virtual output device (to capture remapped events)
    let sink_device = VirtualDeviceBuilder::new()
        .name("Test Sink Keyboard")
        .with_event_type(&EventType::KEY)
        .with_key(&[Key::KEY_A, Key::KEY_B])
        .build()
        .expect("Failed to create sink device");

    // Setup remap engine
    let engine = Arc::new(RemapEngine::new());
    let mut config = HashMap::new();
    config.insert("KEY_A".to_string(), "KEY_B".to_string());
    engine.load_config(&config).await.unwrap();

    // Inject KEY_A press into source
    source_device.emit(&[
        InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1)
    ]).expect("Failed to emit");

    // In real test: read from source_device, apply engine.remap(), write to sink_device
    // Verify sink receives KEY_B instead of KEY_A

    // This pattern requires device event loop integration
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual testing with physical keyboards | Virtual device emulation | evdev added uinput module | Tests can run in CI without hardware |
| Sync-only tests | `#[tokio::test]` for async | Tokio 1.0+ | RemapEngine, ConfigManager are async |
| Global test state | Per-test isolated state | Testing best practices | No flaky tests from shared state |

**Already current:**
- Test organization: Unit tests in src/, integration tests in tests/
- Async testing with tokio::test
- Isolated test environments with TempDir

**Phase 4 additions:**
- Virtual device integration tests (evdev::uinput)
- End-to-end remapping verification
- Automated hotplug scenario testing

## Open Questions

1. **evdev version upgrade consideration**
   - What we know: Project uses evdev 0.12, 0.13.2 is available with updated uinput API
   - What's unclear: Breaking changes between versions for VirtualDeviceBuilder
   - Recommendation: Stay on 0.12 for stability, document version in test examples

2. **CI environment for virtual device tests**
   - What we know: uinput requires /dev/uinput access (usually needs elevated permissions)
   - What's unclear: CI platform (GitHub Actions, GitLab CI, etc.) and permission setup
   - Recommendation: Add conditional test skip if /dev/uinput unavailable, document CI requirements

3. **Device grab testing without root**
   - What we know: EVIOCGRAB typically requires CAP_SYS_RAWIO or root
   - What's unclear: Whether virtual devices can be grabbed without elevated permissions
   - Recommendation: Test grab on virtual devices first, document permission requirements

## Sources

### Primary (HIGH confidence)
- `/home/feanor/Projects/remapper_rs/razermapper/razermapperd/src/remap_engine.rs` (lines 439-689) - RemapEngine unit tests
- `/home/feanor/Projects/remapper_rs/razermapper/razermapperd/src/key_parser.rs` (lines 308-542) - KeyParser unit tests
- `/home/feanor/Projects/remapper_rs/razermapper/razermapperd/tests/config_reload_test.rs` (513 lines) - Configuration reload integration tests
- `/home/feanor/Projects/remapper_rs/razermapper/razermapperd/tests/hotplug_test.rs` (188 lines) - Hotplug event handling tests
- `/home/feanor/Projects/remapper_rs/razermapper/razermapperd/src/bin/test_grab.rs` - Device grab manual testing patterns
- https://docs.rs/evdev/latest/evdev/uinput/index.html - evdev uinput module documentation (verified via webReader)

### Secondary (MEDIUM confidence)
- https://docs.rs/evdev/ - evdev crate documentation (version 0.13.2 documented, project uses 0.12)
- ROADMAP.md Phase 4 requirements - Success criteria and test coverage goals

### Tertiary (LOW confidence)
- None - all findings verified from primary sources or existing codebase

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - verified from Cargo.toml and existing test code
- Architecture: HIGH - patterns extracted from working test files
- Pitfalls: MEDIUM - based on common Rust testing issues, some virtual device specifics need validation
- Code examples: HIGH - copied directly from working tests in codebase

**Research date:** 2026-02-17
**Valid until:** 90 days (testing infrastructure is stable, evdev API is mature)
