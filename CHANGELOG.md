# Changelog

All notable changes to this project will be documented in this file.

## [1.5.1] - 2026-05-29

### Fixed
- **Deadlock fix**: Replaced nested `rt.block_on()` calls in the evdev event loop with `sender.try_send()` and `tokio::task::block_in_place()` to prevent runtime starvation under rapid analog stick input.
- **Async mutex safety**: Replaced `std::sync::Mutex` with `tokio::sync::Mutex` in `DaemonState` to eliminate `.lock().unwrap()` panics in async contexts.
- **ABS axis value range**: Added normalization for unsigned 0-65535 device values (Azeron Cyborg 2) to signed -32768..32767 standard evdev range before injection.
- **Error handling**: Replaced all `let _ = ` error discards in the event loop with proper `if let Err(e)` logging for key, relative axis, and absolute axis events.

## [1.5.0] - 2026-05-15

### Added
- **Modular Theme System:** New design palette with a structured theme module (`theme/palette`, `theme/styles`) for consistent, maintainable styling.
- **Keypad Remapping View:** Dedicated keypad view with key-by-key remapping interface.
- **Hotkey Binding Management:** Dedicated hotkey view for creating and managing global hotkey bindings.
- **LED Configuration UI:** LED view with per-zone color and pattern configuration.
- **Analog Calibration UI:** Analog view with live visualizer, deadzone tuning, response curve selection, and D-pad emulation settings.
- **Auto-Switch Rules View:** UI for configuring automatic profile switching based on focused application.
- **Profile Management View:** Dedicated profiles view for creating, switching, and organizing remap profiles.
- **Device & Layer Views:** Separated device management and layer configuration into focused views.
- **Sidebar & Status Bar:** Extracted sidebar navigation and status bar into standalone components.

### Changed
- **GUI Architecture Refactor:** Monolithic `gui.rs` decomposed into modular `views/`, `handlers/`, `widgets/`, and `theme/` modules.
- Improved code formatting and clippy compliance across all workspace members.
- IPC protocol types now derive `Default` where appropriate for cleaner serialization.

### Fixed
- Fixed macro engine cleanup task not aborting on drop, which could cause resource leaks.
- Fixed macro combo update race condition by passing macro map directly to `update_active_combos_from`.

## [1.4.3] - 2026-05-04

### Changed
- Changed license to GPL-2.0-only.

## [1.4.2] - 2026-05-04

### Changed
- Included README.md in all crate manifests for crates.io display.
- Updated crate metadata for crates.io publishing.

## [1.4.1] - 2026-04-19

### Added
- **Aethermap Rebranding:** Complete project transformation from Razermapper to Aethermap.
- **Human-Speed Macros:** Added randomized jitter and latency compensation to macro execution for better ergonomics and anti-detection.
- **Mouse Recording:** Support for capturing mouse button clicks and relative movements in the macro recorder.
- **Adaptive COSMIC Theme:** New GUI theme that responds to system color-scheme changes via DBus/Ashpd.
- **Physical Device Merging:** Improved discovery logic that groups multiple event nodes (e.g. Naga mouse + keyboard nodes) into a single UI entry.
- **Pacman Packaging:** Updated build scripts and PKGBUILD for Aethermap.

### Fixed
- Fixed duplicated device entries in the GUI.
- Fixed incorrect device classification for hybrid gaming mice.
- Corrected configuration and socket permission issues for standard users.

## [1.3.0] - 2025-11-16
- Initial release of the Rust-based remapper core.
- Basic macro recording and playback.
- Razer Tartarus V2 and Azeron Cyborg 2 support.
