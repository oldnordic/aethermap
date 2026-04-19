# Technology Stack

**Analysis Date:** 2026-02-20

## Languages

**Primary:**
- Rust 2021 Edition - All project code

## Runtime

**Environment:**
- Linux (kernel with evdev/uinput support required)

**Package Manager:**
- Cargo workspace (resolver = "2")
- Lockfile: `Cargo.lock` present

## Frameworks

**Core:**
- Tokio 1.x - Async runtime (full features)
- evdev 0.12 - Linux input device handling
- uinput-sys 0.1 - Virtual input device creation

**GUI:**
- iced 0.12 - GUI framework (tokio, svg features)
- ashpd 0.8 - Wayland portal integration (default-features = false, features = ["wayland", "tokio"])

**Testing:**
- Built-in Rust test framework
- tokio-test 0.4 - Async test utilities
- Integration tests in `/aethermap/tests/`

**Build/Dev:**
- serde 1.x - Serialization (derive feature)
- bincode 1.3 - Binary serialization
- tracing 0.1 / tracing-subscriber 0.3 - Structured logging
- thiserror 1.0 - Error derivation

## Key Dependencies

**Daemon (aethermapd):**
- evdev 0.12 - Input device access, event handling, Key enums
- uinput-sys 0.1 - Virtual device creation via ioctl
- hidapi 2.4 - HID device communication (LED control for Azeron Cyborg 2)
- udev 0.9 - Device hotplug monitoring via MonitorBuilder
- nix 0.29 - System calls (user, ioctl, fs features), privilege dropping
- serde_yaml 0.9 - YAML configuration parsing
- libc 0.2 - FFI bindings (ioctl, time types)
- tempfile 3 - Temporary file handling
- async-trait 0.1 - Async trait support

**GUI (aethermap-gui):**
- iced 0.12 - GUI framework (Elm-inspired architecture)
- ashpd 0.8 - Wayland xdg-desktop-portal integration

**Common (aethermap-common):**
- serde - IPC message serialization
- bincode - Binary IPC protocol
- tokio - Async primitives

## Configuration

**Environment:**
- YAML-based configuration files
- No .env files used (systemd service manages environment)

**Build:**
- Cargo workspace with 4 members: aethermap-common, aethermapd, aethermap-gui, tests
- Feature flag: `token-auth` (disabled by default)
- Debian package metadata in `aethermapd/Cargo.toml`

## Platform Requirements

**Development:**
- Rust 1.70+
- Linux with evdev/uinput headers
- pkg-config for system dependencies

**Production:**
- Linux kernel with evdev and uinput support
- systemd (for service management)
- udev daemon (for hotplug monitoring)
- Wayland compositor with xdg-desktop-portal (optional, for focus tracking)

## Database

**SQLite:**
- `.codemcp/codegraph.db` - Code graph database (SQLiteGraph)
- Used for code analysis and symbol navigation (development tooling)
- Not part of runtime application

---

*Stack analysis: 2026-02-20*
