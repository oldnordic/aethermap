# Aethermap User Manual

Welcome to **Aethermap**. This manual covers the setup, configuration, and advanced usage of the remapping system.

## 1. Concepts

### The Daemon (`aethermapd`)
The daemon runs as a system service. It "grabs" your physical devices, preventing their inputs from reaching the OS directly, and creates a virtual device to inject your custom remaps and macros.

### The GUI (`aethermap-gui`)
The unprivileged user interface used to configure the daemon. It communicates via a Unix socket at `/run/aethermap/aethermap.sock`. The GUI is organized into tabs for each feature area.

## 2. Setting Up Your Device

1. Open the **Devices** tab.
2. Locate your device (e.g., "Razer Tartarus V2").
3. Click **Grab** to begin remapping. Note that while grabbed, the device will only output what Aethermap tells it to.

## 3. Keypad Remapping

1. Open the **Keypad** tab.
2. Click any button on the visual keypad layout.
3. Assign a key, modifier combo, or action to that button.
4. Changes apply immediately to the active profile.

## 4. Creating Macros

### Recording
1. Go to the **Macros** tab.
2. Enter a name and click **Start Recording**.
3. Perform the actions on your keyboard or mouse.
4. Click **Stop Recording**.

### Humanization Settings
To avoid "robotic" performance and reduce hand strain:
- **Latency Offset:** Add a buffer (in ms) to account for your internet lag.
- **Jitter Intensity:** Adds a random % variation to every delay. This makes the macro feel like a human is pressing the keys.

## 5. Layers

Layers allow you to create alternate button mappings that can be activated on the fly:
- **Hold Mode:** The layer is active only while a modifier key is held.
- **Toggle Mode:** The layer toggles on/off with each press of the activation key.

To configure layers, use the layer controls within the keypad or devices view.

## 6. Hotkeys

The **Hotkeys** tab lets you create global hotkey bindings that work regardless of which application is focused:
1. Click **Add Hotkey**.
2. Press the key combination you want to bind.
3. Assign an action (key injection, macro, or command).

## 7. Analog Calibration

If your device has an analog stick (e.g., Azeron Cyborg), the **Analog** tab provides:
- **Deadzone:** Adjust the deadzone size and shape (circular or square) to eliminate stick drift.
- **Response Curve:** Choose between linear and exponential response for precise or aggressive control.
- **D-Pad Emulation:** Convert analog stick movement into 4-way or 8-way digital D-pad output.

## 8. LED Configuration

For devices with RGB support, the **LED** tab offers:
- Per-zone color selection.
- Animation patterns (static, breathing, cycling).

## 9. Auto Profile Switching

The **Auto-Switch** tab lets you automatically switch remap profiles when a specific application window is focused:
1. Add a rule with the application name or window class.
2. Select the profile to activate.
3. Aethermap will switch profiles automatically as you change windows.

## 10. Profiles

The **Profiles** tab lets you create, rename, duplicate, and delete remap profiles. Each profile stores its own key mappings, macros, and layer configurations.

## 11. Black Desert Online (BDO) Integration

Aethermap is optimized for BDO skill combos:
- **Mixed Combos:** You can record `Shift + LMB` or `W + F` accurately.
- **Ergonomics:** Use the Humanization sliders to find a speed that is reliable but doesn't require high-APM physical stress.

## 12. Configuration Files

Settings are stored in `/etc/aethermap/`:
- `config.yaml`: Global daemon settings.
- `device_profiles.yaml`: Per-device remapping rules.
- `macros.yaml`: Your recorded macro library.

## 13. Troubleshooting

### No Devices Found
- Ensure you are in the `users` or `input` group.
- Check that the `aethermapd` service is running: `systemctl status aethermapd`.

### Permission Denied
- Aethermap requires access to `/dev/uinput` and `/dev/input/event*`. This is handled automatically by the provided udev rules.

### GUI Not Connecting to Daemon
- Verify the daemon is running: `systemctl status aethermapd`.
- Check that the socket exists: `ls /run/aethermap/aethermap.sock`.
