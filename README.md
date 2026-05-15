# Aethermap

**Aethermap** is a powerful, low-level input device remapper and macro engine for Linux. Built with Rust for performance and safety, it provides a universal solution for customizing gaming keypads, mice, and keyboards with "Human-Speed" ergonomics.

Originally designed for Razer and Azeron devices, Aethermap now supports any evdev-compatible device, offering deep integration with the COSMIC desktop and full Wayland compatibility.

## Features

- **Ergonomic Macros:** Human-like macro execution with randomized jitter and network latency compensation.
- **Mixed Input Support:** Record and play back complex sequences combining keyboard keys and mouse buttons.
- **Physical Device Merging:** Robust grouping of multiple event nodes into single physical device entries.
- **Adaptive COSMIC Theme:** Modern Iced-based GUI with a modular theme system that automatically follows system Light/Dark mode settings.
- **Analog Stick Processing:** Deadzone calibration, response curves (linear, exponential), and D-pad emulation for analog sticks.
- **Layer System:** Multiple input layers with hold and toggle activation modes.
- **Auto Profile Switching:** Automatically switch remap profiles based on the focused application window.
- **Global Hotkeys:** System-wide hotkey bindings that work regardless of which application is focused.
- **LED Control:** Per-zone LED color and pattern configuration for devices with RGB support.
- **Low-Level Control:** Privileged daemon for exclusive device grabbing and uinput-based virtual injection.
- **Device Support:** 
    - **Azeron:** Cyborg, Compact, Classic (including analog stick support).
    - **Razer:** Tartarus, Naga, and all other keyboards/mice.
    - **Universal:** Any standard Linux input device.

## Installation (Arch Linux)

1. Build the package:
   ```bash
   ./build-package.sh
   ```
2. Install:
   ```bash
   sudo pacman -U aethermap-*.pkg.tar.zst
   ```
3. Start the daemon:
   ```bash
   sudo systemctl enable --now aethermapd
   ```

## Usage

Launch the GUI from your application menu or run `aethermap-gui`.

- **Keypad Tab:** Key-by-key remapping for your device's buttons.
- **Macros Tab:** Record, edit, and assign macros with adjustable humanization settings.
- **Hotkeys Tab:** Create and manage global hotkey bindings.
- **LED Tab:** Configure per-zone LED colors and animation patterns.
- **Analog Tab:** Calibrate analog sticks with deadzone, response curve, and D-pad emulation settings.
- **Auto-Switch Tab:** Set rules for automatic profile switching based on focused application.
- **Profiles Tab:** Create, switch, and organize remap profiles.
- **Devices Tab:** Manage connected hardware and toggle exclusive access.
- **Settings:** Tune global latency offsets and jitter intensity to match your playstyle and connection.

## Documentation

- [User Manual](MANUAL.md) — Setup, configuration, and troubleshooting.
- [Azeron Setup Guide](AZERON_SETUP.md) — Azeron Cyborg 2 specific setup.
- [Changelog](CHANGELOG.md) — Version history and release notes.

## License

This project is licensed under the GPL-2.0-only License - see the [LICENSE](LICENSE) file for details.
