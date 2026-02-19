# Codebase Structure

**Analysis Date:** 2026-02-16

## Directory Layout

```
razermapper/
├── razermapper-common/     # Shared types and IPC protocol
├── razermapperd/           # Privileged system daemon
├── razermapper-gui/        # Unprivileged GUI client
└── tests/                  # Integration tests
```

## Directory Purposes

**razermapper-common:**
- Purpose: Shared data structures and IPC client library
- Contains: Type definitions, serialization helpers, IPC client
- Key files: `src/lib.rs`, `src/ipc_client.rs`

**razermapperd:**
- Purpose: System daemon running as root for hardware access
- Contains: Device management, macro engine, input injection, IPC server, security
- Key files: `src/main.rs`, `src/device.rs`, `src/injector.rs`, `src/macro_engine.rs`, `src/ipc.rs`

**razermapper-gui:**
- Purpose: User interface for configuration and control
- Contains: Iced UI components, state management, IPC client wrapper
- Key files: `src/main.rs`, `src/gui.rs`, `src/ipc.rs`

**tests:**
- Purpose: Integration and end-to-end tests
- Contains: Test fixtures, mock daemons
- Generated: Yes (cargo test artifacts)
- Committed: Yes

## Key File Locations

**Entry Points:**
- `razermapper/razermapperd/src/main.rs`: Daemon binary entry point
- `razermapper/razermapper-gui/src/main.rs`: GUI binary entry point

**Configuration:**
- `razermapper/Cargo.toml`: Workspace configuration
- `razermapper/razermapperd/Cargo.toml`: Daemon dependencies
- `razermapper/razermapper-gui/Cargo.toml`: GUI dependencies
- `razermapper/razermapper-common/Cargo.toml`: Shared dependencies

**Core Logic:**
- `razermapper/razermapperd/src/lib.rs`: Daemon library exports
- `razermapper/razermapperd/src/device.rs`: Device discovery and grabbing (EVIOCGRAB)
- `razermapper/razermapperd/src/injector.rs`: uinput virtual device creation
- `razermapper/razermapperd/src/macro_engine.rs`: Macro execution and recording
- `razermapper/razermapperd/src/ipc.rs`: Unix socket server
- `razermapper/razermapperd/src/config.rs`: YAML configuration and profile management
- `razermapper/razermapperd/src/security.rs`: Privilege dropping and authentication

**GUI:**
- `razermapper/razermapper-gui/src/gui.rs`: Iced Application implementation
- `razermapper/razermapper-gui/src/ipc.rs`: GUI-specific IPC client wrapper

**Testing:**
- `razermapper/razermapperd/src/*/tests.rs`: Unit tests embedded in modules
- `razermapper/razermapper-common/src/ipc_client.rs:406-705`: IPC client tests

## Naming Conventions

**Files:**
- `snake_case.rs` for Rust modules
- `lib.rs` for library roots
- `main.rs` for binary roots

**Directories:**
- `src/` for all source code
- Kebab-case for crate names (`razermapper-common`, `razermapperd`)

**Types:**
- `PascalCase` for structs and enums
- `snake_case` for functions and methods
- `SCREAMING_SNAKE_CASE` for constants

## Where to Add New Code

**New Daemon Feature:**
- Primary code: `razermapper/razermapperd/src/<feature>.rs`
- Module declaration: Add to `razermapperd/src/lib.rs`
- IPC integration: Add variant to `razermapper-common/src/lib.rs` Request/Response enums
- Tests: Inline `#[cfg(test)] mod tests` in same file

**New GUI Component:**
- Implementation: `razermapper/razermapper-gui/src/<component>.rs`
- Module declaration: Add to `razermapper-gui/src/lib.rs`
- View integration: Call from `gui.rs:view_*()` methods

**New IPC Message:**
- Type definition: `razermapper-common/src/lib.rs` in Request/Response enums
- Daemon handler: `razermapperd/src/ipc.rs:handle_request()` match arm
- GUI integration: `razermapper-gui/src/ipc.rs` new client method

**New Action Type:**
- Enum variant: `razermapper-common/src/lib.rs` Action enum
- Engine support: `razermapperd/src/macro_engine.rs:execute_action()` match arm
- Injector impl: `razermapperd/src/injector.rs` trait method

**Utilities:**
- Shared helpers: `razermapper-common/src/lib.rs` or new module
- Daemon-only: `razermapperd/src/utils.rs` (create if needed)

## Special Directories

**tests/:**
- Purpose: Integration tests spanning multiple crates
- Generated: No (source code)
- Committed: Yes

**.planning/:**
- Purpose: Generated planning documents (this file)
- Generated: Yes
- Committed: Yes

**target/:**
- Purpose: Cargo build artifacts
- Generated: Yes
- Committed: No (gitignored)

## Module Dependencies

```
razermapper-gui  --> razermapper-common
                       |
razermapperd  --> razermapper-common
```

Both daemon and GUI depend only on the common crate, not directly on each other.

## Public API Boundaries

**razermapper-common public API:**
- `DeviceInfo` struct - device information
- `MacroEntry` struct - macro definitions
- `KeyCombo` struct - trigger combinations
- `Action` enum - executable actions
- `Request`/`Response` enums - IPC protocol
- `ipc_client` module - client library
- `serialize()`/`deserialize()` helpers

**razermapperd public API:**
- `DaemonState` struct - shared daemon state
- `device` module - `DeviceManager`
- `injector` module - `UinputInjector`, `Injector` trait
- `macro_engine` module - `MacroEngine`
- `ipc` module - `IpcServer`
- `config` module - `ConfigManager`
- `security` module - `SecurityManager`

**razermapper-gui public API:**
- `State` struct - main application state
- `Message` enum - UI events
- `ipc` module - `GuiIpcClient`

---

*Structure analysis: 2026-02-16*
