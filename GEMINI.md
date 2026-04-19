# Aethermap (remapper_rs) Project Context

## Project Overview
Aethermap is a Wayland-compatible input device remapper and macro system for Linux, primarily targeting gaming keypads (Razer Tartarus V2, Azeron Cyborg 2, etc.) but supporting any evdev-compatible device.

### Core Architecture
- **aethermapd**: A privileged system daemon (running as root) that handles device discovery, exclusive grabbing (evdev), input interception, remapping, macro execution, and virtual device creation (uinput).
- **aethermap-gui**: An unprivileged user-space client built with the Iced framework for managing profiles, macros, and device settings.
- **aethermap-common**: Shared library containing data structures, IPC protocol definitions (Request/Response), and serialization logic (using Bincode).
- **IPC Protocol**: Communicates via a Unix socket located at `/run/aethermap/aethermap.sock`.

### Key Technologies
- **Rust**: Language of choice for both daemon and GUI.
- **Tokio**: Async runtime for handling multiple devices and IPC.
- **evdev / uinput**: Low-level Linux input handling.
- **udev**: Hotplug monitoring.
- **Iced**: GUI framework for the client.

---

## Development Standards (MANDATORY)
Adherence to these rules is non-negotiable. Refer to `CLAUDE.md` for full details.

### Rule #1: NEVER GUESS - ALWAYS VERIFY
- Read source code before changing it.
- Verify changes with `cargo check` and `cargo test`.

### Rule #2: TDD - PROVE IT FIRST
1. Write a failing test.
2. Verify failure.
3. Implement fix.
4. Verify pass.

### Rule #3: CITE YOUR SOURCES
State exactly which files and lines you read before making a change.

### Rule #4: CODE QUALITY
- Max 300 LOC per file (600 with justification).
- No `unwrap()` in production paths.
- Proper error handling is required.
- No `#[allow(dead_code)]` or dirty fixes.

---

## Key Files & Directories
- `aethermapd/src/main.rs`: Daemon entry point and initialization.
- `aethermap-gui/src/main.rs`: GUI entry point.
- `aethermap-common/src/lib.rs`: IPC protocol and shared types.
- `aethermapd/src/config.rs`: Configuration management (YAML).
- `CLAUDE.md`: Non-negotiable development rules.

---

## Building and Running

### Build
```bash
cargo build --release
```

### Test
```bash
cargo test
```

### Run Daemon (Root required)
```bash
sudo ./target/release/aethermapd
```

### Run GUI
```bash
./target/release/aethermap-gui
```

### Configuration Hot-Reload
```bash
sudo systemctl reload aethermapd
# OR
sudo kill -HUP $(pgrep aethermapd)
```

### Arch Linux Packaging
Use `./build-package.sh` to generate a Pacman package using the provided `PKGBUILD`.
