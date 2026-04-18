# Razermapper (remapper_rs) Project Context

## Project Overview
Razermapper is a Wayland-compatible input device remapper and macro system for Linux, primarily targeting gaming keypads (Razer Tartarus V2, Azeron Cyborg 2, etc.) but supporting any evdev-compatible device.

### Core Architecture
- **razermapperd**: A privileged system daemon (running as root) that handles device discovery, exclusive grabbing (evdev), input interception, remapping, macro execution, and virtual device creation (uinput).
- **razermapper-gui**: An unprivileged user-space client built with the Iced framework for managing profiles, macros, and device settings.
- **razermapper-common**: Shared library containing data structures, IPC protocol definitions (Request/Response), and serialization logic (using Bincode).
- **IPC Protocol**: Communicates via a Unix socket located at `/run/razermapper/razermapper.sock`.

### Key Technologies
- **Rust**: Language of choice for both daemon and GUI.
- **Tokio**: Async runtime for handling multiple devices and IPC.
- **evdev / uinput**: Low-level Linux input handling.
- **udev**: Hotplug monitoring.
- **Iced**: GUI framework for the client.
- **SQLite**: Used for tracking tasks, steps, and architectural decisions (see `syncore.db`).

---

## Development Standards (MANDATORY)
Adherence to these rules is non-negotiable. Refer to `CLAUDE.md` for full details.

### Rule #1: NEVER GUESS - ALWAYS VERIFY
- Read source code before changing it.
- Check actual database schema via `sqlite3`.
- Verify changes with `cargo check` and `cargo test`.

### Rule #2: STORE ARCHITECTURAL DECISIONS
Before any code change, document the decision in the `syncore.db` database.
```sql
INSERT INTO architectural_decisions (project, decision, reasoning, ...) VALUES (...);
```

### Rule #3: TDD - PROVE IT FIRST
1. Write a failing test.
2. Verify failure.
3. Implement fix.
4. Verify pass.

### Rule #4: CITE YOUR SOURCES
State exactly which files and lines you read before making a change.

### Rule #5: CODE QUALITY
- Max 300 LOC per file (600 with justification).
- No `unwrap()` in production paths.
- Proper error handling is required.
- No `#[allow(dead_code)]` or dirty fixes.

---

## Key Files & Directories
- `razermapperd/src/main.rs`: Daemon entry point and initialization.
- `razermapper-gui/src/main.rs`: GUI entry point.
- `razermapper-common/src/lib.rs`: IPC protocol and shared types.
- `razermapperd/src/config.rs`: Configuration management (YAML).
- `CLAUDE.md`: Non-negotiable development rules.
- `syncore.db`: SQLite database for tracking development state and decisions.

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
sudo ./target/release/razermapperd
```

### Run GUI
```bash
./target/release/razermapper-gui
```

### Configuration Hot-Reload
```bash
sudo systemctl reload razermapperd
# OR
sudo kill -HUP $(pgrep razermapperd)
```

### Arch Linux Packaging
Use `./build-package.sh` to generate a Pacman package using the provided `PKGBUILD`.
