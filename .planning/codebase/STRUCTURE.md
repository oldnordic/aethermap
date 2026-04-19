# Codebase Structure

**Analysis Date:** 2026-02-16

## Directory Layout

```
aethermap/
├── aethermap-common/     # Shared types and IPC protocol
├── aethermapd/           # Privileged system daemon
├── aethermap-gui/        # Unprivileged GUI client
└── tests/                  # Integration tests
```

## Directory Purposes

**aethermap-common:**
- Purpose: Shared data structures and IPC client library
- Contains: Type definitions, serialization helpers, IPC client
- Key files: `src/lib.rs`, `src/ipc_client.rs`

**aethermapd:**
- Purpose: System daemon running as root for hardware access
- Contains: Device management, macro engine, input injection, IPC server, security
- Key files: `src/main.rs`, `src/device.rs`, `src/injector.rs`, `src/macro_engine.rs`, `src/ipc.rs`

**aethermap-gui:**
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
- `aethermap/aethermapd/src/main.rs`: Daemon binary entry point
- `aethermap/aethermap-gui/src/main.rs`: GUI binary entry point

**Configuration:**
- `aethermap/Cargo.toml`: Workspace configuration
- `aethermap/aethermapd/Cargo.toml`: Daemon dependencies
- `aethermap/aethermap-gui/Cargo.toml`: GUI dependencies
- `aethermap/aethermap-common/Cargo.toml`: Shared dependencies

**Core Logic:**
- `aethermap/aethermapd/src/lib.rs`: Daemon library exports
- `aethermap/aethermapd/src/device.rs`: Device discovery and grabbing (EVIOCGRAB)
- `aethermap/aethermapd/src/injector.rs`: uinput virtual device creation
- `aethermap/aethermapd/src/macro_engine.rs`: Macro execution and recording
- `aethermap/aethermapd/src/ipc.rs`: Unix socket server
- `aethermap/aethermapd/src/config.rs`: YAML configuration and profile management
- `aethermap/aethermapd/src/security.rs`: Privilege dropping and authentication

**GUI:**
- `aethermap/aethermap-gui/src/gui.rs`: Iced Application implementation
- `aethermap/aethermap-gui/src/ipc.rs`: GUI-specific IPC client wrapper

**Testing:**
- `aethermap/aethermapd/src/*/tests.rs`: Unit tests embedded in modules
- `aethermap/aethermap-common/src/ipc_client.rs:406-705`: IPC client tests

## Naming Conventions

**Files:**
- `snake_case.rs` for Rust modules
- `lib.rs` for library roots
- `main.rs` for binary roots

**Directories:**
- `src/` for all source code
- Kebab-case for crate names (`aethermap-common`, `aethermapd`)

**Types:**
- `PascalCase` for structs and enums
- `snake_case` for functions and methods
- `SCREAMING_SNAKE_CASE` for constants

## Where to Add New Code

**New Daemon Feature:**
- Primary code: `aethermap/aethermapd/src/<feature>.rs`
- Module declaration: Add to `aethermapd/src/lib.rs`
- IPC integration: Add variant to `aethermap-common/src/lib.rs` Request/Response enums
- Tests: Inline `#[cfg(test)] mod tests` in same file

**New GUI Component:**
- Implementation: `aethermap/aethermap-gui/src/<component>.rs`
- Module declaration: Add to `aethermap-gui/src/lib.rs`
- View integration: Call from `gui.rs:view_*()` methods

**New IPC Message:**
- Type definition: `aethermap-common/src/lib.rs` in Request/Response enums
- Daemon handler: `aethermapd/src/ipc.rs:handle_request()` match arm
- GUI integration: `aethermap-gui/src/ipc.rs` new client method

**New Action Type:**
- Enum variant: `aethermap-common/src/lib.rs` Action enum
- Engine support: `aethermapd/src/macro_engine.rs:execute_action()` match arm
- Injector impl: `aethermapd/src/injector.rs` trait method

**Utilities:**
- Shared helpers: `aethermap-common/src/lib.rs` or new module
- Daemon-only: `aethermapd/src/utils.rs` (create if needed)

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
aethermap-gui  --> aethermap-common
                       |
aethermapd  --> aethermap-common
```

Both daemon and GUI depend only on the common crate, not directly on each other.

## Public API Boundaries

**aethermap-common public API:**
- `DeviceInfo` struct - device information
- `MacroEntry` struct - macro definitions
- `KeyCombo` struct - trigger combinations
- `Action` enum - executable actions
- `Request`/`Response` enums - IPC protocol
- `ipc_client` module - client library
- `serialize()`/`deserialize()` helpers

**aethermapd public API:**
- `DaemonState` struct - shared daemon state
- `device` module - `DeviceManager`
- `injector` module - `UinputInjector`, `Injector` trait
- `macro_engine` module - `MacroEngine`
- `ipc` module - `IpcServer`
- `config` module - `ConfigManager`
- `security` module - `SecurityManager`

**aethermap-gui public API:**
- `State` struct - main application state
- `Message` enum - UI events
- `ipc` module - `GuiIpcClient`

---

*Structure analysis: 2026-02-16*
