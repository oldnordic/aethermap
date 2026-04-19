# Aethermap User Manual

Welcome to **Aethermap**. This manual covers the setup, configuration, and advanced usage of the remapping system.

## 1. Concepts

### The Daemon (`aethermapd`)
The daemon runs as a system service. It "grabs" your physical devices, preventing their inputs from reaching the OS directly, and creates a virtual device to inject your custom remaps and macros.

### The GUI (`aethermap-gui`)
The unprivileged user interface used to configure the daemon. It communicates via a Unix socket at `/run/aethermap/aethermap.sock`.

## 2. Setting Up Your Device

1. Open the **Devices** tab.
2. Locate your device (e.g., "Razer Tartarus V2").
3. Click **Grab** to begin remapping. Note that while grabbed, the device will only output what Aethermap tells it to.

## 3. Creating Macros

### Recording
1. Go to the **Macros** tab.
2. Enter a name and click **Start Recording**.
3. Perform the actions on your keyboard or mouse.
4. Click **Stop Recording**.

### Humanization Settings
To avoid "robotic" performance and reduce hand strain:
- **Latency Offset:** Add a buffer (in ms) to account for your internet lag.
- **Jitter Intensity:** Adds a random % variation to every delay. This makes the macro feel like a human is pressing the keys.

## 4. Black Desert Online (BDO) Integration

Aethermap is optimized for BDO skill combos:
- **Mixed Combos:** You can record `Shift + LMB` or `W + F` accurately.
- **Ergonomics:** Use the Humanization sliders to find a speed that is reliable but doesn't require high-APM physical stress.

## 5. Configuration Files

Settings are stored in `/etc/aethermap/`:
- `config.yaml`: Global daemon settings.
- `device_profiles.yaml`: Per-device remapping rules.
- `macros.yaml`: Your recorded macro library.

## 6. Troubleshooting

### No Devices Found
- Ensure you are in the `users` or `input` group.
- Check that the `aethermapd` service is running: `systemctl status aethermapd`.

### Permission Denied
- Aethermap requires access to `/dev/uinput` and `/dev/input/event*`. This is handled automatically by the provided udev rules.
