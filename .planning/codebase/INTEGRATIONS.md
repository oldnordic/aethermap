# External Integrations

**Analysis Date:** 2026-02-16

## APIs & External Services

**Linux Kernel Subsystems:**

| Service | Purpose | Integration Point |
|---------|---------|-------------------|
| evdev | Input device event reading | `razermapperd/src/device.rs` via `evdev` crate |
| uinput | Virtual input device creation | `razermapperd/src/injector.rs` via `uinput-sys` crate |
| ioctl | Device control and capability manipulation | `libc::ioctl()` for EVIOCGRAB, UI_SET_EVBIT, UI_DEV_CREATE, etc. |

**OpenRazer (read-only integration):**
- Sysfs device discovery at `/sys/bus/hid/drivers/razerkbd/`, `/sys/bus/hid/drivers/razermouse/`, `/sys/bus/hid/drivers/razerchroma/`
- No SDK - direct sysfs reading in `device.rs::scan_razer_sysfs()`
- VID 1532 detection for Razer devices

## Data Storage

**Configuration:**
- Location: `/etc/razermapperd/config.yaml`
- Format: YAML (serde_yaml)
- Schema: `DaemonConfig` in `razermapperd/src/config.rs`

**Macro Storage:**
- Primary: `/etc/razermapperd/macros.yaml` (YAML, human-editable)
- Cache: `/var/cache/razermapperd/macros.bin` (binary, fast load)
- Profiles: `/etc/razermapperd/profiles/*.yaml`

**File Storage:**
- None - no database used, filesystem only

**Caching:**
- Binary macro cache with magic number header (0xDEADBEEF)
- Cache validated on load, falls back to YAML if invalid

## Authentication & Identity

**Auth Provider:**
- Custom token-based authentication (optional, feature-gated: `token-auth`)
- Implementation: `razermapperd/src/security.rs::SecurityManager`
- Token generation: Hash-based (timestamp + PID + memory address)
- Token expiration: 24 hours

**Privilege Management:**
- Linux capabilities (CAP_SYS_RAWIO) retained after init
- Socket ownership: group "input", mode 0660
- Root requirement for daemon startup only
- Privilege dropping after initialization

## Monitoring & Observability

**Error Tracking:**
- None - logs to journald via systemd

**Logs:**
- Framework: tracing + tracing-subscriber
- Output: stdout/stderr captured by systemd journal
- Level: INFO (configurable)
- Format: Structured (target disabled in main.rs)

## CI/CD & Deployment

**Hosting:**
- Linux distribution packages (systemd integration)

**CI Pipeline:**
- None detected in repo

**Packaging:**
- Debian package via cargo-deb metadata
- Assets: binary + systemd service file
- Installation paths: `/usr/bin/razermapperd`, `/usr/lib/systemd/system/razermapperd.service`

## Environment Configuration

**Required env vars:**
- `DISPLAY` - Passed to executed commands for X11 tools
- `PATH` - Cleared to `/usr/bin:/bin` for command execution

**Runtime directories (systemd-managed):**
- `RuntimeDirectory=razermapper` → `/run/razermapper/`
- `StateDirectory=razermapper` → `/var/lib/razermapper/`
- `CacheDirectory=razermapper` → `/var/cache/razermapper/`
- `ConfigurationDirectory=razermapper` → `/etc/razermapper/`

**Secrets location:**
- Auth tokens stored in-memory (HashMap) only
- No persistent secrets

## Webhooks & Callbacks

**Incoming:**
- Unix socket IPC at `/run/razermapper/razermapper.sock`
- Protocol: Length-prefixed bincode messages (4-byte little-endian length)
- Request types defined in `razermapper-common/src/lib.rs`

**Outgoing:**
- Command execution via `tokio::process::Command` (whitelisted binaries only)
- No network requests

## System Integration

**systemd:**
- Service file: `razermapperd/systemd/razermapperd.service`
- Security hardening: CapabilityBoundingSet, NoNewPrivileges, ProtectSystem, PrivateTmp, etc.
- Signal handling: SIGTERM, SIGINT for graceful shutdown
- Socket activation: Not used (manual socket binding)

**udev:**
- Rules file: `pkg/razermapper/usr/lib/udev/rules.d/99-razermapper.rules`
- Permissions: Sets MODE="0660", GROUP="input" for uinput and Razer devices
- TAG+="uaccess" for automatic ACLs

**Command whitelist (for macro Execute action):**
- `xdotool`, `xrandr`, `amixer`, `notify-send`, `pactl`, `playerctl`, `brightnessctl`, `xbacklight`

## IPC Protocol

**Transport:**
- Unix domain socket (AF_UNIX)

**Serialization:**
- bincode (binary)

**Message flow:**
1. Client sends: `[u32; 4]` length prefix + bincode(Request)
2. Server responds: `[u32; 4]` length prefix + bincode(Response)

**Max message size:**
- 1MB (enforced in both client and server)

---

*Integration audit: 2026-02-16*
