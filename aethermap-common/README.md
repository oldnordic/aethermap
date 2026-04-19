# Aethermap

**Aethermap** is a powerful, low-level input device remapper and macro engine for Linux. Built with Rust for performance and safety, it provides a universal solution for customizing gaming keypads, mice, and keyboards with "Human-Speed" ergonomics.

Originally designed for Razer and Azeron devices, Aethermap now supports any evdev-compatible device, offering deep integration with the COSMIC desktop and full Wayland compatibility.

## Features

- **Ergonomic Macros:** Human-like macro execution with randomized jitter and network latency compensation.
- **Mixed Input Support:** Record and play back complex sequences combining keyboard keys and mouse buttons.
- **Physical Device Merging:** Robust grouping of multiple event nodes into single physical device entries.
- **Adaptive COSMIC Theme:** Modern Iced-based GUI that automatically follows system Light/Dark mode settings.
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

- **Devices Tab:** Manage your connected hardware and toggle exclusive access.
- **Macros Tab:** Record, edit, and assign macros with adjustable humanization settings.
- **Settings:** Tune global latency offsets and jitter intensity to match your playstyle and connection.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
