# Azeron Cyborg 2 Support for Aethermap

## Overview

The Azeron Cyborg 2 is now supported in aethermap with the following features:

### Device Identification
- **Vendor ID:** `0x16d0` (MCS / Azeron LTD)
- **Product ID:** `0x12f7` (Azeron Cyborg 2 Keypad)

### Supported Features

#### ✅ Implemented
1. **Full Button Remapping** - All 26 programmable buttons
2. **Dual Joystick Support** - Left thumbstick + Right adjustable joystick
3. **Hat Switch (8-way)** - D-pad on main joystick
4. **LED Brightness Control** - Side LED brightness (0-100%)
5. **Profile Management** - Save and load keymap profiles
6. **Layer Switching** - Multiple remap layers

#### ⏳ Future Work
1. **OLED Display Control** - Show current profile/mode on device display
2. **RGB Color Control** - Hardware only supports brightness; RGB requires further protocol discovery
3. **Profile Switching via HID** - Currently profiles switch via software
4. **Thumbstick Calibration** - GUI for deadzone/sensitivity adjustment

## Hardware Layout

### Button Map (26 buttons)
```
D-Pad (Surrounding Left Joystick):
  joy_btn_0 - D-Pad Up
  joy_btn_1 - D-Pad Right
  joy_btn_2 - D-Pad Down
  joy_btn_3 - D-Pad Left

Thumb Cluster (Right side, below LCD):
  joy_btn_4  - Thumb Top
  joy_btn_5  - Thumb Middle
  joy_btn_6  - Thumb Bottom

Left Cluster (Left side):
  joy_btn_7  - Left Cluster 1
  joy_btn_8  - Left Cluster 2
  joy_btn_9  - Left Cluster 3
  joy_btn_10 - Left Cluster 4

Main Keypad:
  joy_btn_11 - Key 01
  joy_btn_12 - Key 02
  joy_btn_13 - Key 03
  joy_btn_14 - Key 04
  joy_btn_15 - Key 05
  joy_btn_16 - Key 06

Right Cluster:
  joy_btn_17 - Right Cluster 1
  joy_btn_18 - Right Cluster 2
  joy_btn_19 - Right Cluster 3

Modifiers:
  joy_btn_20 - Modifier 1 (Shift)
  joy_btn_21 - Modifier 2 (Ctrl)
  joy_btn_22 - Modifier 3 (Alt)
  joy_btn_23 - Modifier 4 (Space)

Actions:
  joy_btn_24 - Action 1 (Tab)
  joy_btn_25 - Action 2 (Enter)
  joy_btn_26 - Extra (ESC)
```

### Joysticks
- **Left Stick (js0):** ABS_X (0), ABS_Y (1)
- **Right Stick (js1):** ABS_X (0), ABS_Y (1)
- **Hat Switch:** 8-way on js0

### LED
- **Single Zone:** Side LED bar (blue only)
- **Control:** Brightness 0-100%
- **Protocol:** Discovered via Wireshark USB capture

## Setup Instructions

### 1. Install aethermap v1.3.0+
```bash
sudo pacman -U aethermap-1.3.0-1-x86_64.pkg.tar.zst
```

### 2. Configure Azeron Device Profile
Copy the device profile to the config directory:
```bash
sudo cp device_profiles.yaml /etc/aethermap/device_profiles.yaml
```

Or use the GUI to load the profile:
1. Open aethermap-gui
2. Select "Azeron Cyborg 2" from device list
3. Choose a profile (Gaming, MOBA, Productivity, or Media)
4. Customize as needed

### 3. Configure Azeron Hardware Settings
Before using aethermap, configure the device in the official Azeron software:
1. **Set all keys to "Joystick Button"** mode
2. **Set joystick to "Analog Joystick"** mode
3. **Set hat to "Joystick D-Pad"** with up/down/left/right mappings
4. **Set mouse emulation OFF** (aethermap handles this via uinput)
5. **Optional:** Adjust thumbstick position/angle

## Pre-configured Profiles

### Gaming
WASD-style layout with gaming keys:
- D-Pad: W/A/S/D
- Thumb: E/R/F (Item, Reload, Flashlight)
- Cluster: 1-4 number keys
- Main: 5-0 and spell keys
- Right: Q/G/C

### MOBA
MOBA game optimized:
- D-Pad: Camera controls
- Thumb: Q/W/E ability keys
- Main: Q/W/E/R/D/F spell keys
- Center: B (back), G (ping), Space (stop)

### Productivity
Office work:
- D-Pad: Arrow keys
- Thumb: Delete/Home/End
- Cluster: Number keys
- Right: Insert/PageUp/PageDown

### Media
Media player controls:
- D-Pad: Navigation arrows
- Thumb: Mute/Volume Down/Volume Up
- Cluster: Previous/Next/Play/Pause

## Technical Details

### Input Interfaces
The Azeron Cyborg 2 exposes 3 input interfaces:

1. **Keyboard + Joystick (event27, js0)**
   - Keyboard events
   - Main joystick (X/Y)
   - Hat switch
   - All joystick buttons

2. **Mouse Emulation (event28, mouse4)**
   - Can emulate mouse movement
   - NOT recommended - use aethermap's uinput instead

3. **Joystick Only (event29, js1)**
   - Second joystick only
   - For adjustable right stick

### LED Protocol (Discovered)
**Packet Format (64 bytes):**
```
Offset  Size    Description
------  ------  -----------
0       2 bytes Sequence/type (0x0001 for LED, 0x0000 for keepalive)
2       2 bytes Command counter (little-endian, increments)
4       1 byte  Always 0x01
5       1 byte  Always 0x01
6       2 bytes Brightness value (little-endian: 256-406)
8       1 byte  Always 0x00
9       1 byte  Always 0x00
10-63   54 bytes Padding (0x00)
```

**Brightness Encoding:**
- 0% = 0x0100 (256)
- 100% = 0x0196 (406)
- Formula: `brightness = 256 + (percent * 150 / 100)`

## Troubleshooting

### Device Not Detected
```bash
# Check device is connected
lsusb | grep 16d0

# Check input devices
cat /proc/bus/input/devices | grep -A 5 Azeron

# Check permissions
ls -l /dev/input/event*
```

### Buttons Not Responding
1. Verify Azeron is configured for Joystick mode (not Keyboard mode)
2. Check aethermapd is running: `systemctl status aethermapd`
3. Check logs: `journalctl -u aethermapd -f`

### LED Not Working
1. Verify interface 4 is accessible: `ls -l /dev/hidraw*`
2. Check LED controller is enabled in config
3. Test with: `journalctl -u aethermapd | grep -i led`

## References
- [Azeron Linux](https://github.com/renatoi/azeron-linux) - Official software for Linux
- [Noreza](https://github.com/Caedis/noreza) - Alternative Azeron mapper
- [azeron-cli](https://github.com/cozyGalvinism/azeron-cli) - Protocol documentation
