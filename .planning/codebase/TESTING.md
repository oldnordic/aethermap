# Testing Patterns

**Analysis Date:** 2026-02-16

## Test Framework

**Runner:**
- Tokio Test Runtime (built into tokio)
- Config: Implicit via `#[tokio::test]` attribute

**Assertion Library:**
- Standard `assert!`, `assert_eq!`, `assert_matches!` macros from Rust std

**Run Commands:**
```bash
cargo test                          # Run all tests
cargo test --package razermapperd    # Run tests for specific package
cargo test test_device_discovery     # Run specific test
cargo test -- --nocapture            # Show test output
```

## Test File Organization

**Location:**
- **Unit tests:** Co-located in `#[cfg(test)]` modules at bottom of each source file
- **Integration tests:** Separate `tests/` directory at workspace level
- **GUI tests:** Co-located in `razermapper-gui/tests/`

**Structure:**
```
razermapper/
├── tests/
│   ├── e2e.rs                    # End-to-end integration tests
│   └── Cargo.toml                # Test-only package config
├── razermapper-common/
│   └── src/
│       ├── lib.rs                # Contains unit tests module
│       └── ipc_client.rs         # Contains unit tests module
├── razermapperd/
│   └── src/
│       ├── lib.rs                # No tests currently
│       ├── main.rs               # No tests currently
│       ├── config.rs             # Contains unit tests module
│       ├── device.rs             # Contains unit tests module
│       ├── injector.rs           # Contains unit tests module
│       ├── ipc.rs                # Contains unit tests module
│       ├── macro_engine.rs       # Contains unit tests module
│       └── security.rs           # Contains unit tests module
└── razermapper-gui/
    └── tests/
        └── gui_sanity.rs         # GUI integration tests
```

**Naming:**
- Test functions: `test_<component>_<behavior>`
- Test modules: `tests` (within `#[cfg(test)]`)

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

**Teardown Pattern:**
- Use `tempfile::TempDir` for automatic cleanup
- Use `drop()` or scope-based cleanup for resources
- Abort background tasks: `test_env.daemon_handle.abort();`

## Mocking

**Framework:**
- Manual mock implementations using traits
- `#[async_trait::async_trait]` for async trait mocking

**Patterns:**

**Mock Injector (from `macro_engine.rs`):**
```rust
struct MockInjector;

#[async_trait::async_trait]
impl Injector for MockInjector {
    async fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn key_press(&self, _key_code: u16) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    // ... other methods returning Ok(())
}
```

**Mock Daemon (from `e2e.rs`):**
```rust
async fn run_mock_daemon(socket_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let listener = UnixListener::bind(socket_path)?;

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
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

**Test Data:**
```rust
// Helper function from gui_sanity.rs
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
```

**Location:**
- In `tests` module for file-local fixtures
- At top of test files for cross-test fixtures

**Environment Pattern (from `e2e.rs`):**
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

## Coverage

**Requirements:**
- No formal coverage target enforced
- TDD approach required by CLAUDE.md: write failing test first

**Areas with Test Coverage:**

| Module | Coverage | Tests Location |
|--------|----------|----------------|
| `ipc_client` | Good serialization/communication tests | `ipc_client.rs` tests module |
| `macro_engine` | Core functionality tests | `macro_engine.rs` tests module |
| `config` | Persistence tests | `config.rs` tests module |
| `security` | Auth/token tests | `security.rs` tests module |
| `device` | Basic creation tests | `device.rs` tests module |
| `injector` | Basic creation/keymap tests | `injector.rs` tests module |
| `ipc` | Request handling tests | `ipc.rs` tests module |
| `gui` | State update/render tests | `gui_sanity.rs` |
| `e2e` | Comprehensive integration tests | `e2e.rs` |

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
#[tokio::test]
async fn test_serialization_roundtrip() {
    let request = Request::GetDevices;
    let serialized = serialize(&request).unwrap();
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

**Permission-Skipped Tests:**
```rust
// From device.rs - tests that may fail without /dev/input access
#[tokio::test]
async fn test_device_discovery() {
    let mut manager = DeviceManager::new();
    let result = manager.start_discovery().await;

    if result.is_ok() {
        println!("Found {} devices", manager.get_devices().len());
    }
    // Test doesn't panic even if discovery fails due to permissions
}
```

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
| **Large payload handling** | Only basic test for 1000-action macro | Low | Low |
| **Edge cases in macro engine** | Macro recursion, self-triggering | Medium | Medium |
| **Security token expiration** | Basic test exists, needs more coverage | Medium | High |
| **Config file corruption** | Handling of malformed YAML | Medium | High |

**Privileged Operation Testing:**
- Most hardware-access tests require root
- Pattern: Tests will `panic!()` with descriptive message if run without proper permissions
- Example from `ipc.rs`:
```rust
fn create_test_injector() -> Arc<injector::UinputInjector> {
    match injector::UinputInjector::new() {
        Ok(injector) => Arc::new(injector),
        Err(_) => {
            panic!("Test requires root access to create UinputInjector. Run with sudo...");
        }
    }
}
```

## Running Privileged Tests

```bash
# Run tests requiring root access
sudo cargo test --package razermapperd

# Run specific privileged test
sudo cargo test test_injector_creation
```

---

*Testing analysis: 2026-02-16*
