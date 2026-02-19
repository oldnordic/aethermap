# Coding Conventions

**Analysis Date:** 2026-02-16

## Naming Patterns

**Files:**
- Use `snake_case` for all Rust source files: `device.rs`, `macro_engine.rs`, `ipc_client.rs`
- Module directories use `kebab-case`: `razermapper-common/`, `razermapper-gui/`
- Test files follow pattern: `<module>_test.rs` or co-located in `tests/` directory

**Functions:**
- Use `snake_case` for all functions: `start_discovery()`, `grab_device()`, `get_devices()`
- Async functions use same naming convention, distinguished by `async fn` keyword
- Builder-style methods use `with_` prefix: `with_timeout()`, `with_socket_path()`, `with_retry_params()`

**Types:**
- Use `PascalCase` for structs and enums: `DeviceInfo`, `MacroEntry`, `IpcError`, `DaemonState`
- Use `PascalCase` for traits: `Injector`
- Type aliases use `PascalCase`: `EngineResult<T>`, `Request`, `Response`

**Constants:**
- Use `SCREAMING_SNAKE_CASE` for constants: `DEFAULT_SOCKET_PATH`, `MAX_MESSAGE_SIZE`, `EVIOCGRAB`
- ioctl and Linux constants follow this pattern: `UI_SET_EVBIT`, `EV_KEY`, `CAP_SYS_RAWIO`

**Variables:**
- Use `snake_case` for local variables: `socket_path`, `macro_entry`, `event_sender`
- Use `snake_case` for struct fields: `pub name: String`, `pub device_id: Option<String>`

## Code Style

**Formatting:**
- Project uses standard Rust `rustfmt` conventions
- 4-space indentation
- Line length appears to be around 100 characters (based on many lines)
- Struct field initialization aligned where convenient

**Linting:**
- Project enforces `#![warn(dead_code)]` and similar via CLAUDE.md rules
- No `#[allow(dead_code)]` to silence warnings (forbidden by project rules)
- Errors are propagated with `?` operator rather than explicit match/unwrap where possible

## Import Organization

**Order:**
1. Standard library imports (`std::*`)
2. Third-party crates (`tokio`, `serde`, `tracing`, `thiserror`, etc.)
3. Workspace-local imports (`razermapper_common::*`)
4. Module declarations (`pub mod xyz;`)
5. Re-exports (`pub use xyz::Abc;`)

**Path Aliases:**
- Workspace dependencies use `razermapper_` prefix: `razermapper_common`, `razermapperd`
- Re-exports at crate root for common types: `pub use razermapper_common::{DeviceInfo, MacroEntry, Profile};`

**Example from `/home/feanor/Projects/remapper_rs/razermapper/razermapperd/src/lib.rs`:**
```rust
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::RwLock;
use std::collections::HashMap;

pub mod config;
pub mod device;
// ... other modules

pub use razermapper_common::{DeviceInfo, MacroEntry, Profile};
```

## Error Handling

**Patterns:**
- Use `thiserror` for error enum definitions (seen in `ipc_client.rs`)
- Use `Box<dyn std::error::Error + Send + Sync>` for async trait object errors
- Use `Result<T, Box<dyn std::error::Error>>` for application-level errors
- NEVER use `unwrap()` in production paths (forbidden by CLAUDE.md)
- Log errors with `tracing::error!()` before returning

**Error Type Example from `/home/feanor/Projects/remapper_rs/razermapper/razermapper-common/src/ipc_client.rs`:**
```rust
#[derive(Error, Debug)]
pub enum IpcError {
    #[error("failed to connect to daemon: {0}")]
    Connect(std::io::Error),
    #[error("request timed out")]
    Timeout,
    #[error("Daemon not running at {0}")]
    DaemonNotRunning(String),
    // ... other variants
}
```

**Result Propagation:**
```rust
pub async fn connect(&self) -> Result<UnixStream, IpcError> {
    // Uses ? operator for error propagation
    let stream = timeout(self.timeout, UnixStream::connect(&self.socket_path)).await
        .map_err(|_| IpcError::ConnectionTimeout)?;
    Ok(stream)
}
```

**Type Aliases for Async Results:**
```rust
pub type EngineResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
```

## Logging

**Framework:** `tracing` and `tracing-subscriber`

**Levels:**
- `error!()` - For failures that prevent operation
- `warn!()` - For recoverable issues and degraded functionality
- `info!()` - For significant state changes and operations
- `debug!()` - For detailed diagnostic information
- `tracing::debug!()` - Used when crate-level import needed

**Patterns:**
```rust
use tracing::{info, error, warn, debug};

info!("Starting Razermapper Daemon v0.1.0");
error!("Failed to grab device {}: {}", device_path, err);
warn!("Privileges already dropped");
debug!("Event from {}: key={}, pressed={}", path, key_code, pressed);
```

**Initialization:**
```rust
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .with_target(false)
    .init();
```

## Comments

**Module Documentation:**
- Every module has a `//!` doc comment explaining purpose
- Format: `//! <Module description>`
- Example from `device.rs`: `//! Manages the discovery and monitoring of input devices`

**Function Documentation:**
- Public functions have `///` doc comments
- Includes `# Arguments` and `# Returns` sections for significant functions
- Async functions document await behavior

**Example from `ipc_client.rs`:**
```rust
/// Send a request to the daemon using the default client
///
/// # Arguments
///
/// * `request` - The request to send to the daemon
///
/// # Returns
///
/// Returns the response from the daemon or an IpcError if communication fails
///
/// # Example
///
/// ```rust,no_run
/// use razermapper_common::{ipc_client, Request};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let response = ipc_client::send(&Request::GetDevices).await?;
///     println!("Got response: {:?}", response);
///     Ok(())
/// }
/// ```
pub async fn send(request: &Request) -> Result<Response, IpcError>
```

**Inline Comments:**
- Used sparingly, mainly for:
  - Complex unsafe code explanations
  - Linux-specific constants and syscalls
  - Non-obvious logic
- Commented-out dead code is FORBIDDEN by CLAUDE.md

## Function Design

**Size:** Target under 50 lines, complex functions may reach 100-150 lines

**Parameters:**
- Use `self`, `&self`, or `&mut self` for methods
- Borrow with `&str` instead of `String` for read-only string parameters
- Use `AsRef<Path>` for flexible path parameters
- Async functions use `async fn` and return `Future<Output = Result<...>>`

**Return Values:**
- Use `Result<T, E>` for fallible operations
- Use `Option<T>` for optional returns
- Async functions that can't fail return plain type
- Complex state returns may use tuples or custom structs

**Builder Pattern:**
```rust
pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
    self.timeout = Duration::from_millis(timeout_ms);
    self
}

pub fn with_retry_params(mut self, max_retries: u32, retry_delay_ms: u64) -> Self {
    self.max_retries = max_retries;
    self.retry_delay = Duration::from_millis(retry_delay_ms);
    self
}
```

## Module Design

**Exports:**
- Public types are re-exported at module root for convenience
- Common dependencies re-exported: `pub use razermapper_common::{Request, Response};`
- Module visibility controls internal vs public API

**Barrel Files:**
- Each crate's `lib.rs` declares and exports modules
- Example from `/home/feanor/Projects/remapper_rs/razermapper/razermapperd/src/lib.rs`:
```rust
pub mod config;
pub mod device;
pub mod macro_engine;
pub mod injector;
pub mod ipc;
pub mod security;

pub use razermapper_common::{DeviceInfo, MacroEntry, Profile};
```

## Async/Await Patterns

**Tokio Runtime:**
- All async code uses `tokio` with `#[tokio::main]` for entry points
- Test functions use `#[tokio::test]`
- Use `tokio::spawn` for background tasks
- Use `tokio::select!` for multiple async branches

**Async Traits:**
- Use `#[async_trait::async_trait]` for trait methods that are async
- Return `Box<dyn std::error::Error + Send + Sync>` for trait object errors

**Lock Usage:**
- Use `tokio::sync::RwLock` for async-safe locking
- Use `Arc<RwLock<T>>` for shared mutable state across tasks
- Read/write pattern: `let state = state.read().await;`

## Unsafe Code

**Usage:**
- Used sparingly, only for:
  - Linux syscall wrappers (ioctl, libc calls)
  - FFI operations
  - Raw pointer manipulation for evdev/uinput

**Documentation:**
- Every unsafe block has comment explaining safety
- Example from `device.rs`:
```rust
// Grab the device exclusively with EVIOCGRAB
let result = unsafe {
    libc::ioctl(fd, EVIOCGRAB, 1 as libc::c_int)
};
```

## Project-Specific Rules

**From CLAUDE.md:**
- Max 300 LOC per file (600 with justification)
- NO `unwrap()` in production paths
- NO `#[allow(dead_code)]` to silence warnings
- NO commented-out broken code
- NO mocks/stubs/placeholders in production
- TDD approach: write failing test first
- Verify with code before making changes

**File Size Enforcement:**
- Most source files stay under 400 lines
- Larger files like `ipc_client.rs` (~700 lines) and `e2e.rs` (~720 lines) contain significant test code
- `ipc.rs` is the largest at ~720 lines due to comprehensive request handling

## Testing Conventions

**Unit Tests:**
- Located in `#[cfg(test)]` modules at bottom of each file
- Use `#[tokio::test]` for async tests
- Test functions named `test_<what_is_tested>`

**Integration/E2E Tests:**
- Located in `/home/feanor/Projects/remapper_rs/razermapper/tests/`
- Use `tokio::test` attribute
- Create test environments with temporary directories/sockets

**Test Helpers:**
- Helper functions named `create_test_*` or `mock_*`
- Use `tempfile::TempDir` for isolated test filesystems

---

*Convention analysis: 2026-02-16*
