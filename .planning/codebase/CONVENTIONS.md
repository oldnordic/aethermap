# Coding Conventions

**Analysis Date:** 2026-02-20

## Naming Patterns

**Files:**
- Use `snake_case` for all Rust source files: `device.rs`, `macro_engine.rs`, `ipc_client.rs`
- Module directories use `kebab-case`: `razermapper-common/`, `razermapper-gui/`, `razermapperd/`
- Test files: `<module>_test.rs` in `tests/` directory, or co-located in `#[cfg(test)]` modules

**Functions:**
- Use `snake_case` for all functions: `start_discovery()`, `grab_device()`, `get_devices()`
- Async functions use same naming, distinguished by `async fn` keyword
- Builder-style methods use `with_` prefix: `with_timeout()`, `with_socket_path()`, `with_retry_params()`

**Types:**
- Use `PascalCase` for structs and enums: `DeviceInfo`, `MacroEntry`, `IpcError`, `DaemonState`
- Use `PascalCase` for traits: `Injector`
- Type aliases use `PascalCase`: `EngineResult<T>`, `Request`, `Response`, `RemapTable`

**Constants:**
- Use `SCREAMING_SNAKE_CASE` for constants: `DEFAULT_SOCKET_PATH`, `MAX_MESSAGE_SIZE`, `EVIOCGRAB`
- ioctl and Linux constants follow this pattern: `UI_SET_EVBIT`, `EV_KEY`, `CAP_SYS_RAWIO`
- Hardware constants: `ANALOG_MIN_RAW`, `ANALOG_MAX_RAW`, `EV_SYN`, `EV_KEY`

**Variables:**
- Use `snake_case` for local variables: `socket_path`, `macro_entry`, `event_sender`
- Use `snake_case` for struct fields: `pub name: String`, `pub device_id: Option<String>`

## Code Style

**Formatting:**
- Project uses standard Rust `rustfmt` conventions (no custom config)
- 4-space indentation
- Line length around 100 characters (rustfmt default)
- Struct field initialization aligned where convenient

**Linting:**
- `cargo clippy` for additional lint checking
- `#![warn(dead_code)]` enforced via project rules
- NO `#[allow(dead_code)]` to silence warnings (forbidden by CLAUDE.md)

## Import Organization

**Order:**
1. Standard library imports (`std::*`)
2. Third-party crates (`tokio`, `serde`, `tracing`, `thiserror`, `evdev`, etc.)
3. Workspace-local imports (`razermapper_common::*`)
4. Crate-relative imports (`crate::*`)
5. Module declarations (`pub mod xyz;`)
6. Re-exports (`pub use xyz::Abc;`)

**Example from `/home/feanor/Projects/remapper_rs/razermapper/razermapperd/src/lib.rs`:**
```rust
// std
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::collections::HashMap;

// external
use tokio::sync::RwLock;

// workspace
use razermapper_common::{DeviceInfo, MacroEntry, Profile};

// crate
use crate::remap_engine::RemapEngine;

// declarations
pub mod config;
pub mod device;

// re-exports
pub use config::{RemapEntry, RemapConfigError};
```

**Path Aliases:**
- Workspace dependencies use `razermapper_` prefix: `razermapper_common`, `razermapperd`
- Re-exports at crate root for common types

## Error Handling

**Primary Patterns:**
- Custom error enums with `thiserror` for domain-specific errors
- `Result<T, Box<dyn std::error::Error + Send + Sync>>` for async trait returns
- Type aliases for complex error types: `pub type EngineResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;`

**Custom Error Example:**
```rust
#[derive(Debug)]
pub enum RemapError {
    InvalidKey { key: String, source: String, parse_error: String },
    Config(String),
    ParseError(ParseError),
}

impl fmt::Display for RemapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RemapError::InvalidKey { key, source, parse_error } => {
                write!(f, "Invalid key name '{}' in {}: {}", key, source, parse_error)
            }
            RemapError::Config(msg) => write!(f, "Configuration error: {}", msg),
            RemapError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}
```

**Error Propagation:**
- `?` operator used throughout
- Context added via `map_err()` when converting errors
- `unwrap()` and `expect()` FORBIDDEN in production paths (CLAUDE.md rule)

## Logging

**Framework:** `tracing` crate (structued logging)

**Levels:**
- `error!()` - Fatal errors requiring intervention
- `warn!()` - Recoverable issues, degraded functionality
- `info!()` - Significant state changes (device connect/disconnect, profile switches)
- `debug!()` - Detailed flow information
- `trace!()` - Very granular (rarely used)

**Pattern:**
```rust
use tracing::{debug, info, warn, error};

info!("Device connected: {}", device.name);
debug!("Processing key event: {:?}", key_code);
warn!("Failed to grab device: {}", path);
error!("Configuration load failed: {}", reason);
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
- `//!` doc comments at top of every file
- Describe purpose, key functionality, usage patterns

**Example from `/home/feanor/Projects/remapper_rs/razermapper/razermapperd/src/remap_engine.rs`:**
```rust
//! RemapEngine for translating input key codes to output key codes
//!
//! This module provides the core remapping functionality that translates
//! input key codes from grabbed devices to remapped output key codes.
//!
//! # Layer-Aware Remapping
//!
//! The RemapEngine supports multi-layer remapping with cascading fallback.
```

**Type Documentation:**
- `///` for public structs, enums, traits
- Include examples where appropriate

**Function Documentation:**
- `///` for public functions
- Describe arguments (`# Arguments`), returns (`# Returns`), panics
- Document async behavior

**Inline Comments:**
- Used sparingly for:
  - Complex unsafe code explanations
  - Linux-specific constants and syscalls
  - Non-obvious logic
- Commented-out dead code FORBIDDEN by CLAUDE.md

## Function Design

**Size:**
- Target: 20-100 lines
- Large functions split into helpers
- Complex operations split across multiple functions

**Parameters:**
- Use `self`, `&self`, or `&mut self` for methods
- Borrow with `&str` instead of `String` for read-only string parameters
- Use `AsRef<Path>` for flexible path parameters
- Consider struct for 3+ related parameters

**Return Values:**
- `Result<T, E>` for fallible operations
- `Option<T>` for optional returns
- Tuples for multiple related values

**Builder Pattern:**
```rust
pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
    self.timeout = Duration::from_millis(timeout_ms);
    self
}
```

## Module Design

**Exports:**
- Public types explicitly marked `pub`
- `pub use` for re-exports at crate level
- Module visibility controls internal vs public API

**Barrel Files:**
- `lib.rs` serves as barrel for each crate
- Example from `/home/feanor/Projects/remapper_rs/razermapper/razermapperd/src/lib.rs`:
```rust
pub mod config;
pub mod device;
pub mod macro_engine;
pub mod injector;
pub mod ipc;

pub use razermapper_common::{DeviceInfo, MacroEntry, Profile};
pub use config::{RemapEntry, RemapConfigError, AutoSwitchRule};
pub use remap_engine::{RemapProfile, RemapTable};
```

**Visibility:**
- Default: private
- `pub(crate)` for crate-internal sharing
- `pub` for public API

## Rust-Specific Patterns

**Shared State:**
- `Arc<RwLock<T>>` for concurrent async access
- `Arc<Mutex<T>>` for synchronous shared state
- Example: `pub macros: Arc<RwLock<HashMap<String, MacroEntry>>>`

**Async/Await:**
- `tokio::test` for async tests
- `async fn` for async functions
- `.await` used throughout

**Trait Usage:**
- `#[async_trait::async_trait]` for async traits
- Trait objects: `dyn Injector + Send + Sync`
- Trait bounds on generics: `T: Serialize + Deserialize`

**Serde Serialization:**
- `#[derive(Serialize, Deserialize)]` on config types
- `#[serde(default)]` for fields with defaults
- `#[serde(rename_all = "snake_case")]` for enum variants

## Code Organization

**Max LOC Per File:**
- Target: Under 600 lines
- Large files (exceptions):
  - `razermapper-gui/src/gui.rs` - 4489 lines (GUI view code)
  - `razermapperd/src/analog_processor.rs` - 3969 lines (complex analog processing)
  - `razermapperd/src/config.rs` - 3093 lines (configuration management)
  - `razermapperd/src/device.rs` - 2543 lines (device management)

**Module Structure:**
- One module per file
- `mod.rs` or `lib.rs` for module aggregation
- Tests co-located in `#[cfg(test)]` modules

**Separation of Concerns:**
- Config types in `config.rs`
- Business logic in engine modules (`remap_engine`, `macro_engine`)
- Hardware abstraction in `device.rs`, `injector.rs`
- IPC boundaries defined in `ipc.rs` and `razermapper-common`

## Thread Safety

**Send + Sync Bounds:**
- `Send + Sync` required for shared async types
- Trait objects: `dyn std::error::Error + Send + Sync`
- Arc-wrapped types for thread-safe sharing

**Lock Ordering:**
- RwLock preferred when reads dominate writes
- Mutex used for interior mutability without async

## Hardware Interaction

**FFI:**
- Direct libc calls where needed (`setuid`, `setgid`, `ioctl`)
- `repr(C)` structs for kernel interfaces
- Raw file descriptors via `AsRawFd`

**Constants:**
- Kernel constants defined as `const` items
- Example from `/home/feanor/Projects/remapper_rs/razermapper/razermapperd/src/injector.rs`:
```rust
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const UI_DEV_CREATE: u64 = 0x5501;
```

**Unsafe Code:**
- Used sparingly for syscalls and FFI
- Every unsafe block has explanatory comment

## Project-Specific Rules

**From CLAUDE.md:**
- Max 300 LOC per file (600 with justification)
- NO `unwrap()` in production paths
- NO `#[allow(dead_code)]` to silence warnings
- NO commented-out broken code
- NO mocks/stubs/placeholders in production
- TDD approach: write failing test first
- Verify with code before making changes

---

*Convention analysis: 2026-02-20*
