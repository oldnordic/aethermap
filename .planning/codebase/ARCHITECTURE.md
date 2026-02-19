# Architecture

**Analysis Date:** 2026-02-16

## Pattern Overview

**Overall:** Client-Server with Privilege Separation

**Key Characteristics:**
- Privileged daemon (runs as root) handles hardware access
- Unprivileged GUI client communicates via IPC
- Shared types crate ensures protocol compatibility
- Token-based authentication (optional)
- Event-driven macro execution with async runtime

## Layers

**Daemon Core (razermapperd):**
- Purpose: System-level device management and macro execution
- Location: `/home/feanor/Projects/remapper_rs/razermapper/razermapperd/src/`
- Contains: Device discovery, input interception, macro engine, uinput injection, IPC server, security
- Depends on: `razermapper-common`, evdev, uinput-sys, tokio, nix
- Used by: Systemd (as service), GUI clients

**GUI Client (razermapper-gui):**
- Purpose: User interface for device and macro management
- Location: `/home/feanor/Projects/remapper_rs/razermapper/razermapper-gui/src/`
- Contains: Iced-based UI, IPC client wrapper, state management
- Depends on: `razermapper-common`, iced, tokio
- Used by: End users

**Common Types (razermapper-common):**
- Purpose: Shared IPC protocol and data structures
- Location: `/home/feanor/Projects/remapper_rs/razermapper/razermapper-common/src/`
- Contains: Request/Response enums, DeviceInfo, MacroEntry, Action types, IPC client
- Depends on: serde, bincode, tokio, thiserror
- Used by: Both daemon and GUI

## Data Flow

**Device Discovery Flow:**

1. Daemon starts (`razermapperd/src/main.rs`)
2. `DeviceManager::start_discovery()` scans `/dev/input/event*` and OpenRazer sysfs
3. Devices are stored in `DaemonState.devices`
4. GUI sends `Request::GetDevices` via IPC
5. Daemon responds with `Response::Devices(Vec<DeviceInfo>)`

**Macro Recording Flow:**

1. User grabs a device via GUI (sends `Request::GrabDevice`)
2. `DeviceManager::grab_device()` uses EVIOCGRAB ioctl for exclusive access
3. Event reader task spawns, forwards events via mpsc channel
4. User starts recording (sends `Request::RecordMacro`)
5. `MacroEngine` records events to `MacroEntry.actions`
6. User stops recording, macro is stored in `DaemonState.macros`

**Macro Execution Flow:**

1. Input event arrives from grabbed device
2. Event forwarded to `MacroEngine::process_input_event()`
3. `check_macro_triggers()` matches key codes against enabled macros
4. If match found, `execute_macro()` spawns async task
5. Each action dispatched to `Injector` trait (uinput)
6. Virtual input device emits synthesized events

**State Management:**
- `DaemonState` wrapped in `Arc<RwLock<>>` for concurrent access
- Device events flow via `mpsc::channel` from blocking tasks to async runtime
- Macro execution state tracked in `MacroEngine.executing: HashMap`

## Key Abstractions

**Injector Trait:**
- Purpose: Abstract input event synthesis
- Examples: `razermapperd/src/injector.rs:62-73`
- Pattern: Async trait with methods for key press/release, mouse actions, typing, commands

**Request/Response Protocol:**
- Purpose: Type-safe IPC communication
- Examples: `razermapper-common/src/lib.rs:71-224`
- Pattern: Enum-based RPC with bincode serialization over Unix socket

**MacroEntry:**
- Purpose: Serializable macro definition
- Examples: `razermapper-common/src/lib.rs:62-69`
- Pattern: Named trigger + ordered action list with device restriction

## Entry Points

**Daemon Entry:**
- Location: `razermapperd/src/main.rs:17-188`
- Triggers: Systemd startup, manual execution as root
- Responsibilities: Security initialization, component startup, privilege dropping, signal handling

**GUI Entry:**
- Location: `razermapper-gui/src/main.rs:11-14`
- Triggers: User launch from desktop/session
- Responsibilities: Iced application initialization, async command spawning

**IPC Server:**
- Location: `razermapperd/src/ipc.rs:26-126`
- Triggers: Client connection to `/run/razermapper/razermapper.sock`
- Responsibilities: Request routing, authentication, response serialization

## Error Handling

**Strategy:** Result<T, Box<dyn Error>> propagation with thiserror

**Patterns:**
- IPC uses `IpcError` enum for specific failure modes (`razermapper-common/src/ipc_client.rs:19-61`)
- Device operations return `Box<dyn std::error::Error>`
- Macro engine uses `EngineResult<T>` alias (`razermapperd/src/macro_engine.rs:11`)

## Cross-Cutting Concerns

**Logging:** tracing framework with INFO level default

**Validation:**
- Message size limits (1MB max)
- Device existence checks before macro assignment
- Token authentication when enabled

**Authentication:**
- Optional token-based auth (feature flag `token-auth`)
- Token expiration (24 hours)
- Socket permissions (0660, group "input")

**Privilege Management:**
- Start as root, drop capabilities after initialization
- Retain CAP_SYS_RAWIO for uinput access
- `SecurityManager` handles capability dropping (`razermapperd/src/security.rs:43-92`)

---

*Architecture analysis: 2026-02-16*
