# External Integrations

**Analysis Date:** 2026-02-20

## System-Level

**Linux evdev:**
- Input device event reading from `/dev/input/event*`
- Key location: `aethermapd/src/device.rs`
- Purpose: Device discovery, grabbing, event reading
- Constants: `evdev::Key`, `InputEventKind`, `AbsoluteAxisType`, `RelativeAxisType`

**uinput:**
- Virtual input device creation
- Key location: `aethermapd/src/injector.rs`, `aethermapd/src/gamepad_device.rs`
- Purpose: Create virtual keyboard, mouse, gamepad devices
- ioctl calls: `UI_DEV_CREATE`, `UI_SET_EVBIT`, `UI_SET_KEYBIT`, `UI_SET_ABSBIT`

**HIDAPI:**
- HID device communication for LED control
- Key location: `aethermapd/src/led_controller.rs`
- Purpose: Azeron Cyborg 2 LED brightness control
- Vendor ID: 0x16d0 (Azeron), Product ID: 0x12f7 (Cyborg 2)
- Interface: Interface 4 (usage_page 0xff01), not keyboard interface

**udev:**
- Device hotplug monitoring
- Key location: `aethermapd/src/hotplug.rs`
- Purpose: Detect device add/remove events via MonitorBuilder
- Subsystem: input devices only

**nix/libc:**
- System calls and ioctl
- Key locations: `aethermapd/src/security.rs`, `aethermapd/src/gamepad_device.rs`
- Purpose: Privilege dropping, ioctl calls, file descriptor operations

## Wayland Integration

**xdg-desktop-portal (ashpd):**
- Window focus tracking for auto-profile switching
- Key location: `aethermap-gui/src/focus_tracker.rs`
- Purpose: Detect focused application for profile switching
- Graceful degradation when portal unavailable
- Environment check: `WAYLAND_DISPLAY`

## IPC & Communication

**Unix Socket Protocol:**
- Location: `/run/aethermap/aethermap.sock`
- Server: `aethermapd/src/ipc.rs`
- Client: `aethermap-common/src/ipc_client.rs`
- Transport: AF_UNIX stream socket
- Serialization: bincode (binary)
- Message format: 4-byte little-endian length prefix + payload
- Max message size: 1MB

**Request types:**
- Device management: GetDevices, GrabDevice, UngrabDevice
- Profile management: SaveProfile, LoadProfile, ListProfiles
- Layer management: SetLayerConfig, ActivateLayer, ListLayers
- Analog calibration: SetAnalogCalibration, GetAnalogCalibration
- LED control: SetLedColor, SetLedBrightness, SetLedPattern
- Focus tracking: FocusChanged (GUI to daemon)
- Hotkey binding: RegisterHotkey, ListHotkeys, RemoveHotkey

## Hardware

**Supported Devices:**
- Razer devices: Tartarus V2, Orbweaver (VID 0x1532)
- Azeron Cyborg 2: Full support including LED control (VID 0x16d0, PID 0x12f7)
- Generic HID gaming devices with configurable modes

**Device Detection:**
- Scan `/proc/bus/input/devices`
- evdev capability detection (EV_KEY, EV_REL, EV_ABS)
- Vendor:Product identification

**Virtual Device Emulation:**
- Virtual gamepad (Xbox 360 controller compatible)
- Location: `aethermapd/src/gamepad_device.rs`
- Vendor ID: 0x045e (Microsoft), Product ID: 0x028e (Xbox 360 Controller)
- Axes: ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ
- Buttons: Standard gamepad button layout

## Config & State

**Configuration Files:**
- `/etc/aethermapd/remaps.yaml` - Global key remappings
- `/etc/aethermapd/device-profiles.yaml` - Per-device remap profiles
- `/etc/aethermapd/config.yaml` - Main configuration
- Format: YAML (serde_yaml)

**State Persistence:**
- LED state persisted per-device in DaemonState
- Restored after hotplug via `DeviceLedState` export/import
- Layer configurations stored in profiles

**Hot-reload:**
- SIGHUP signal handling
- Reloads remaps and device profiles without restart

## System Integration

**systemd:**
- Service file: `aethermapd.service`
- RuntimeDirectory: `/run/aethermap/`
- Signal handling: SIGTERM, SIGINT, SIGHUP
- Security: CapabilityBoundingSet, NoNewPrivileges, ProtectSystem

**udev Rules:**
- File: `99-aethermap.rules`
- Sets MODE="0660", GROUP="input" for devices
- TAG+="uaccess" for ACLs

## Authentication

**Token-based (optional):**
- Feature flag: `token-auth`
- Implementation: `aethermapd/src/security.rs`
- Token generation: Hash-based (timestamp + PID + memory address)
- Expiration: 24 hours
- In-memory storage only (HashMap)

---

*Integration audit: 2026-02-20*
