# Design Spec: Aethermap Project Transformation

## 1. Overview
The goal is to rebrand the project from "Aethermap" to **Aethermap** to reflect its universal device support and to implement a beautiful, native-feeling "Cosmic" UI that follows system themes (Light/Dark) automatically.

## 2. Success Criteria
- [ ] Entire project renamed to **Aethermap** (directories, binaries, services, socket paths, source code).
- [ ] Project continues to compile and pass all tests after renaming.
- [ ] GUI implements an "Adaptive" theme that responds to system Light/Dark mode changes.
- [ ] GUI aesthetics match the COSMIC desktop (rounded corners, 1px borders, specific palette).
- [ ] Backward compatibility: Migrates `~/.config/aethermap` to `~/.config/aethermap` automatically.

## 3. Architecture & Refactoring

### 3.1 Renaming Strategy (The "Big Sweep")
- **Directory Structure**: 
    - `aethermap/` -> `aethermap/`
    - `aethermapd/` -> `aethermapd/`
    - `aethermap-gui/` -> `aethermap-gui/`
    - `aethermap-common/` -> `aethermap-common/`
- **Metadata**: Update `Cargo.toml` `[package]` names and workspace `members`.
- **System Integration**: 
    - Socket: `/run/aethermap/aethermap.sock`
    - Service: `aethermapd.service`
    - Desktop: `aethermap-gui.desktop`
- **Source Code**: Case-sensitive search-and-replace:
    - `aethermap` -> `aethermap`
    - `Aethermap` -> `Aethermap`
    - `AETHERMAP` -> `AETHERMAP`

### 3.2 Custom Adaptive Theme
- **Theme Engine**: Implement `AetherTheme` in the GUI.
- **System Integration**: Use `ashpd` (Settings portal) to listen for `color-scheme` changes via DBus.
- **Cosmic Palette**:
    - **Dark**: Bg: `#1a1a1a`, Card: `#242424`, Accent: `#3070f0` (Cosmic Blue).
    - **Light**: Bg: `#f2f2f2`, Card: `#ffffff`, Accent: `#3070f0`.
- **Aesthetics**:
    - Border Radius: 10px for all primary containers.
    - Spacing: Unified 12px/16px grid.
    - Icons: Modernize navigation icons.

## 4. Safety & Verification Plan
1. **Incremental Rename**: Rename one sub-crate at a time and run `cargo check`.
2. **IPC Validation**: Test that `aethermap-gui` can still connect to `aethermapd`.
3. **Migration Test**: Verify that old config files are moved and used correctly.
4. **Theme Test**: Manually toggle system theme and verify GUI response.

## 5. Implementation Roadmap
- **Task 1**: Rename common library and daemon components.
- **Task 2**: Rename GUI and update systemd/socket files.
- **Task 3**: Implement the Adaptive Theme and COSMIC styling.
- **Task 4**: Final workspace cleanup and verification.
