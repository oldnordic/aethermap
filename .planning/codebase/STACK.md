# Technology Stack

**Analysis Date:** 2026-02-16

## Languages

**Primary:**
- Rust 2021 Edition - All code (workspace crates use edition = "2021")

**Secondary:**
- None (pure Rust project)

## Runtime

**Environment:**
- Linux (Wayland-compatible, targets `/dev/input` and uinput subsystems)

**Package Manager:**
- Cargo (workspace with 3 crates + tests)
- Lockfile: `Cargo.lock` (present)

## Frameworks

**Core:**
- Tokio 1.x - Async runtime (full features) - Used for async I/O, task spawning, and Unix socket communication
- Iced 0.12 - GUI framework for razermapper-gui (with tokio and svg features)

**Testing:**
- Tokio-test 0.4 - Async test utilities
- tempfile 3.0 - Temporary file/directory creation for tests

**Build/Dev:**
- None specified in Cargo.toml (uses standard cargo build)

## Key Dependencies

**Critical:**

| Package | Version | Purpose |
|---------|---------|---------|
| evdev | 0.12 | Linux input device handling (reading from `/dev/input/event*`) |
| uinput-sys | 0.1 | Raw FFI bindings for uinput kernel module (virtual device creation) |
| serde | 1.x | Serialization framework (with derive feature) |
| bincode | 1.3 | Binary serialization for IPC protocol |
| async-trait | 0.1 | Async trait support for `Injector` trait |

**Infrastructure:**

| Package | Version | Purpose |
|---------|---------|---------|
| tokio | 1.x | Async runtime, Unix sockets, processes, signals |
| libc | 0.2 | Raw FFI bindings for syscalls (ioctl, prctl, setgroups, gettimeofday) |
| nix | 0.29 | Rustic Unix wrapper (user, ioctl, fs features) |
| udev | 0.9 | Device discovery and udev integration |
| tracing | 0.1 | Structured logging framework |
| tracing-subscriber | 0.3 | Log collection and formatting |
| serde_yaml | 0.9 | YAML config/profile file parsing |
| tempfile | 3.0 | Temporary test fixtures |
| thiserror | 1.0 | Error derivation macros |

**GUI-specific:**

| Package | Version | Purpose |
|---------|---------|---------|
| iced | 0.12 | Cross-platform GUI ( Elm-inspired architecture) |

## Configuration

**Environment:**
- Config files stored in `/etc/razermapperd/`
- Cache stored in `/var/cache/razermapperd/`
- Socket at `/run/razermapper/razermapper.sock`
- Runtime directory managed by systemd

**Build:**
- Standard Cargo workspace
- Debian package metadata in `razermapperd/Cargo.toml` (package.metadata.deb)

## Platform Requirements

**Development:**
- Rust 2021 edition compatible compiler
- Linux system for device access testing

**Production:**
- Linux kernel with uinput module loaded
- systemd (for service management)
- udev rules for device permissions
- Root or CAP_SYS_RAWIO capability for device access

## Workspace Structure

```
razermapper/
├── razermapper-common/    # Shared types, IPC protocol, client
├── razermapperd/          # Privileged daemon (runs as root)
├── razermapper-gui/       # Unprivileged GUI client
└── tests/                 # Integration/e2e tests
```

---

*Stack analysis: 2026-02-16*
