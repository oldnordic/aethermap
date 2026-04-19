# Testing Patterns

**Analysis Date:** 2026-02-20

## Test Framework

**Runner:**
- Tokio Test Runtime (built into tokio)
- Config: Implicit via `#[tokio::test]` attribute

**Assertion Library:**
- Standard `assert!`, `assert_eq!`, `assert_matches!` macros from Rust std

**Run Commands:**
```bash
cargo test                          # Run all tests
cargo test --package aethermapd    # Run tests for specific package
cargo test --package aethermap-gui # Run GUI tests
cargo test test_device_discovery     # Run specific test
cargo test -- --nocapture            # Show test output
cargo test -- --ignored              # Run ignored tests
```

## Test File Organization

**Location:**
- **Unit tests:** Co-located in `#[cfg(test)]` modules at bottom of each source file
- **Integration tests:** Separate `tests/` directories at crate level
- **E2E tests:** `/home/feanor/Projects/remapper_rs/aethermap/tests/`

**Structure:**
```
aethermap/
├── tests/
│   ├── e2e.rs                    # End-to-end integration tests
│   └── Cargo.toml                # Test-only package config
├── aethermap-common/
│   └── src/
│       ├── lib.rs                # Contains #[cfg(test)] module (serialization tests)
│       └── ipc_client.rs         # Contains #[cfg(test)] module (IPC tests)
├── aethermapd/
│   ├── src/
│   │   ├── lib.rs                # No tests (re-exports only)
│   │   ├── main.rs               # No tests
│   │   ├── config.rs             # No inline tests (separate test files)
│   │   ├── remap_engine.rs       # No inline tests
│   │   └── ...
│   └── tests/
│       ├── hotplug_test.rs       # Device hotplug event tests
│       ├── remap_integration_test.rs  # Virtual device remapping tests
│       ├── config_reload_test.rs # Config hot-reload tests
│       └── macro_integration_test.rs  # Macro execution tests
└── aethermap-gui/
    ├── src/
    │   └── focus_tracker.rs      # Contains #[cfg(test)] module
    └── tests/
        └── gui_sanity.rs         # GUI state/update tests
```

**Naming:**
- Test functions: `test_<component>_<behavior>`
- Test modules: `mod tests` (within `#[cfg(test)]`)

## Test Structure

**Suite Organization:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_component_behavior() {
        // Setup
        let component = Component::new();

        // Exercise
        let result = component.do_something().await;

        // Verify
        assert!(result.is_ok());
    }
}
```

**Patterns:**
- **Setup:** Create test fixtures, temporary directories, mock objects
- **Exercise:** Call the function being tested
- **Verify:** Assert expected outcomes
- **Teardown:** Implicit via Drop (TempDir cleanup, etc.)

**Async Test Pattern:**
```rust
#[tokio::test]
async fn test_macro_creation() {
    let engine = MacroEngine::new();
    let macro_entry = MacroEntry { /* ... */ };

    engine.add_macro(macro_entry.clone()).await.unwrap();

    let retrieved = engine.get_macro("Test Macro").await.unwrap();
    assert_eq!(retrieved.name, macro_entry.name);
}
```

**Synchronous Test Pattern:**
```rust
#[test]
fn test_focus_event_creation() {
    let event = FocusEvent::new("org.alacritty".to_string(), Some("Alacritty".to_string()));
    assert_eq!(event.app_id, "org.alacritty");
}
```

## Mocking

**Framework:**
- Manual mock implementations using traits
- `#[async_trait::async_trait]` for async trait mocking

**Mock Injector Pattern** (from `/home/feanor/Projects/remapper_rs/aethermap/aethermapd/tests/macro_integration_test.rs`):
```rust
struct MockInjector {
    log: Arc<RwLock<Vec<String>>>,
}

#[async_trait::async_trait]
impl Injector for MockInjector {
    async fn key_press(&self, key_code: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_action(&format!("key_press:{}", key_code)).await;
        Ok(())
    }

    async fn mouse_press(&self, button: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_action(&format!("mouse_press:{}", button)).await;
        Ok(())
    }

    // ... other methods
}
```

**Mock Daemon** (from `/home/feanor/Projects/remapper_rs/aethermap/aethermap-common/src/ipc_client.rs`):
```rust
async fn mock_daemon(socket_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = UnixListener::bind(socket_path)?;

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                tokio::spawn(async move {
                    // Handle connection with mock responses
                });
            }
            // ...
        }
    }
}
```

**What to Mock:**
- External system dependencies (ioctl, file system in tests)
- Network services (Unix domain sockets)
- Hardware devices (/dev/uinput, evdev)

**What NOT to Mock:**
- Core business logic (macro engine state management)
- Data structures (serialization/deserialization)
- Simple utilities

## Fixtures and Factories

**Test Data Helpers** (from `/home/feanor/Projects/remapper_rs/aethermap/aethermap-gui/tests/gui_sanity.rs`):
```rust
fn create_test_device(name: &str, path: &str) -> DeviceInfo {
    DeviceInfo {
        name: name.to_string(),
        path: PathBuf::from(path),
        vendor_id: 0x1532,
        product_id: 0x0203,
        phys: "usb-0000:00:14.0-1/input/input0".to_string(),
    }
}

fn create_test_macro(name: &str, enabled: bool) -> MacroEntry {
    MacroEntry {
        name: name.to_string(),
        trigger: KeyCombo {
            keys: vec![30, 40],
            modifiers: vec![29],
        },
        actions: vec![
            Action::KeyPress(30),
            Action::Delay(100),
            Action::KeyRelease(30),
        ],
        device_id: Some("test_device".to_string()),
        enabled,
    }
}

fn create_test_state() -> State {
    let mut state = State {
        devices: vec![
            create_test_device("Razer Keyboard", "/dev/input/event0"),
            create_test_device("Razer Mouse", "/dev/input/event1"),
        ],
        macros: vec![
            create_test_macro("Test Macro 1", true),
            create_test_macro("Test Macro 2", false),
        ],
        selected_device: Some(0),
        status: "Test initialized".to_string(),
        // ... other fields
    };
    state
}
```

**Environment Pattern** (from `/home/feanor/Projects/remapper_rs/aethermap/tests/e2e.rs`):
```rust
struct TestEnvironment {
    temp_dir: TempDir,
    socket_path: PathBuf,
    daemon_handle: JoinHandle<()>,
    client: IpcClient,
}

impl TestEnvironment {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let socket_path = temp_dir.path().join("test.sock");
        let daemon_handle = Self::start_daemon(&socket_path).await?;
        let client = IpcClient::with_socket_path(&socket_path);
        Ok(Self { temp_dir, socket_path, daemon_handle, client })
    }
}
```

**Location:**
- In `#[cfg(test)]` module for file-local fixtures
- At top of test files for cross-test fixtures

## Coverage

**Requirements:**
- No formal coverage target enforced
- TDD approach required by CLAUDE.md: write failing test first

**Areas with Test Coverage:**

| Module | Coverage | Tests Location |
|--------|----------|----------------|
| `remap_engine` | Virtual device integration tests | `remap_integration_test.rs` |
| `macro_engine` | Mock injector tests | `macro_integration_test.rs` |
| `hotplug` | Device ID format, event structure | `hotplug_test.rs` |
| `config` | Hot-reload, atomic swap | `config_reload_test.rs` |
| `ipc_client` | Serialization, round-trip | `ipc_client.rs` tests module |
| `common/lib` | IPC serialization | `lib.rs` tests module |
| `focus_tracker` | Pattern matching, creation | `focus_tracker.rs` tests module |
| `gui` | State updates, view rendering | `gui_sanity.rs` |
| `e2e` | Full IPC workflows | `e2e.rs` |

## Test Types

**Unit Tests:**
- Scope: Single function or module
- Approach: Direct function calls with mock data
- No external dependencies (file system, network)
- Example: `test_ipc_client_creation()` in `ipc_client.rs`

**Integration Tests:**
- Scope: Multiple modules interacting
- Approach: Real IPC communication with mock daemon
- Temporary files/sockets used
- Example: `e2e.rs` tests full client-daemon communication

**E2E Tests:**
- Scope: Complete workflows
- Framework: Custom test environment in `e2e.rs`
- Tests verify: device discovery, macro recording, profile management, authentication
- No actual GUI (GUI tests are state-based in `gui_sanity.rs`)

## Common Patterns

**Async Testing:**
```rust
#[tokio::test]
async fn test_async_operation() {
    // Setup
    let component = Component::new();

    // Execute async operation
    let result = component.async_method().await;

    // Assert
    assert!(result.is_ok());
}
```

**Error Testing:**
```rust
#[tokio::test]
async fn test_error_handling() {
    let test_env = TestEnvironment::new().await?;

    // Try to execute a non-existent macro
    let response = test_env.client.send(&Request::TestMacro {
        name: "Non-existent Macro".to_string(),
    }).await?;

    match response {
        Response::Error(message) => {
            assert_eq!(message, "Macro 'Non-existent Macro' not found");
        }
        _ => panic!("Unexpected response: {:?}", response),
    }

    Ok(())
}
```

**Serialization Testing:**
```rust
#[test]
fn test_ipc_serialization() {
    let request = Request::GetDevices;
    let serialized = serialize(&request);
    let deserialized: Request = deserialize(&serialized).unwrap();
    assert!(matches!(deserialized, Request::GetDevices));
}
```

**State Testing (GUI):**
```rust
#[test]
fn test_devices_loaded_message() {
    let mut state = State::default();

    let devices = vec![
        create_test_device("Test Device 1", "/dev/input/event0"),
        create_test_device("Test Device 2", "/dev/input/event1"),
    ];

    let message = Message::DevicesLoaded(Ok(devices.clone()));
    let _command = state.update(message);

    assert_eq!(state.devices.len(), 2);
    assert_eq!(state.devices[0].name, "Test Device 1");
}
```

**CI-Skipped Tests:**
```rust
#[tokio::test]
async fn test_end_to_end_key_remapping() {
    // Skip if /dev/uinput not available (CI compatibility)
    if !std::path::Path::new("/dev/uinput").exists() {
        println!("Skipping: /dev/uinput not available");
        return;
    }

    // Test continues...
}
```

**Timing Tests:**
```rust
#[tokio::test]
async fn test_delay_action_timing() {
    let start = std::time::Instant::now();
    let result = engine.execute_macro(macro_entry).await;
    assert!(result.is_ok());

    // Wait for macro to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let elapsed = start.elapsed();
    // Should take at least 50ms for the delay
    assert!(elapsed >= tokio::time::Duration::from_millis(50));
}
```

## Notable Test Suites

**`/home/feanor/Projects/remapper_rs/aethermap/tests/e2e.rs` (722 lines):**
- Comprehensive end-to-end IPC tests
- Mock daemon with Unix socket
- Tests: authentication, device discovery, macros, profiles, privileged operations

**`/home/feanor/Projects/remapper_rs/aethermap/aethermapd/tests/remap_integration_test.rs` (296 lines):**
- Virtual device tests using `evdev::uinput::VirtualDeviceBuilder`
- Tests: key remapping, repeat events, complex remappings
- CI-compatible (skips if `/dev/uinput` not available)

**`/home/feanor/Projects/remapper_rs/aethermap/aethermapd/tests/macro_integration_test.rs` (328 lines):**
- Macro engine tests with MockInjector
- Tests: mixed keyboard/mouse macros, mouse movement, scroll, delays, concurrent macros

**`/home/feanor/Projects/remapper_rs/aethermap/aethermapd/tests/config_reload_test.rs` (513 lines):**
- Configuration hot-reload tests
- Tests: atomic swap, invalid rejection, concurrent reload safety
- Uses `tempfile::TempDir` for isolated test filesystems

**`/home/feanor/Projects/remapper_rs/aethermap/aethermapd/tests/hotplug_test.rs` (188 lines):**
- Device hotplug event handling tests
- Tests: device ID formatting, event structure, clone behavior

**`/home/feanor/Projects/remapper_rs/aethermap/aethermap-gui/tests/gui_sanity.rs` (325 lines):**
- GUI state and update tests
- Tests: device loading, macro loading, recording state, status updates, error handling

## Test Coverage Gaps

**Untested Areas:**

| Area | What's Not Tested | Risk | Priority |
|------|-------------------|------|----------|
| **Main daemon initialization** | Full startup sequence, privilege dropping | Medium | Medium |
| **Signal handling** | SIGTERM/SIGINT graceful shutdown | Low | Low |
| **Real device interaction** | Actual evdev device grabbing (requires root) | Medium | High |
| **Uinput injection** | Real input injection (requires root) | High | High |
| **GUI rendering** | Visual Iced element tree rendering | Low | Medium |
| **Concurrent connections** | Limited testing of multiple simultaneous clients | Medium | Medium |
| **Macro recursion** | Self-triggering macros, infinite loops | Medium | Medium |
| **Security token expiration** | Token expiry handling | Medium | High |
| **Config file corruption** | Handling of malformed YAML | Medium | High |

**Privileged Operation Testing:**
- Most hardware-access tests require root
- Pattern: Tests check for `/dev/uinput` existence and skip if not available
- Example: Virtual device tests in `remap_integration_test.rs`

## Running Privileged Tests

```bash
# Run tests requiring root access
sudo cargo test --package aethermapd

# Run specific privileged test
sudo cargo test test_injector_creation
```

---

*Testing analysis: 2026-02-20*
