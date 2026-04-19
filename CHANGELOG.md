# Changelog

All notable changes to this project will be documented in this file.

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
