# Aethermap GUI Redesign

**Date:** 2026-05-14
**Status:** Approved
**Scope:** Full visual and structural redesign of aethermap-gui, staying within Iced 0.12

---

## Problem

The current GUI is a single 5357-line `gui.rs` file with a flat, generic visual style. It has:
- No visual identity or personality
- Dense, cluttered layout with no breathing room
- Monolithic code that makes iteration difficult
- Complex user flows for common tasks (remapping, macro recording)

## Approach

Incremental refactor: split gui.rs into focused modules first, then apply a new design system and layout to each module. Every step compiles and runs.

## Design System

### Color Palette

| Token | Hex | Use |
|-------|-----|-----|
| `bg-base` | `#0D0D0D` | Main background |
| `bg-surface` | `#161616` | Cards, panels |
| `bg-elevated` | `#1E1E1E` | Hover states, active items |
| `bg-overlay` | `#000000CC` | Dialog overlays |
| `text-primary` | `#E8E8E8` | Main text |
| `text-secondary` | `#888888` | Labels, descriptions |
| `text-muted` | `#555555` | Disabled, hints |
| `accent` | `#3B82F6` | Primary actions, active states |
| `accent-hover` | `#60A5FA` | Hover on accent elements |
| `success` | `#22C55E` | Connected, saved, success |
| `danger` | `#EF4444` | Errors, destructive actions |
| `warning` | `#F59E0B` | Warnings |
| `border-subtle` | `#FFFFFF 0.06` | Panel separators |

### Spacing Scale

- `xs` = 4px
- `sm` = 8px
- `md` = 16px
- `lg` = 24px
- `xl` = 32px

### Border Radius

- Cards: 8px
- Buttons: 6px
- Inputs: 6px

### Component Styles

- **Buttons:** Flat fill, no borders. Primary = accent background. Secondary = bg-elevated.
- **Cards:** bg-surface background, no border, 8px radius. Visual separation via background contrast only.
- **Text inputs:** bg-elevated background, no border. Accent color focus indicator.
- **Sidebar items:** Transparent background. Active = accent left border + bg-elevated. Text labels only.

## Module Structure

```
aethermap-gui/src/
├── app.rs              # AppState, Application impl, Message enum, update dispatch
├── theme/
│   ├── mod.rs          # Module root, re-exports
│   ├── palette.rs      # Color constants
│   └── styles.rs       # Reusable component style functions (cards, buttons, inputs, sidebar items)
├── views/
│   ├── mod.rs          # Module root, re-exports
│   ├── sidebar.rs      # Navigation sidebar view
│   ├── header.rs       # Top header bar
│   ├── status_bar.rs   # Bottom footer bar
│   ├── devices.rs      # Device management + keypad integration
│   ├── macros.rs       # Macro recording and management
│   ├── profiles.rs     # Profile management (card grid)
│   ├── keypad.rs       # Visual keypad layout (used within devices view)
│   ├── analog.rs       # Analog calibration overlay
│   ├── led.rs          # LED configuration overlay
│   ├── layers.rs       # Layer management
│   ├── auto_switch.rs  # Auto-profile switching rules
│   └── hotkeys.rs      # Global hotkey bindings
├── widgets/            # (existing) AnalogVisualizer, CurveGraph
├── focus_tracker.rs    # (existing)
├── ipc.rs              # (existing)
├── lib.rs              # (existing)
└── main.rs             # (existing)
```

### State Architecture

```
AppState (app.rs)
├── socket_path, current_theme, notifications, daemon_connected
├── active_nav: NavItem
├── devices: DevicesState
├── macros: MacrosState
├── profiles: ProfilesState
├── keypad: KeypadState
├── analog: Option<AnalogCalibrationView>
├── led: Option<LedConfigState>
├── layers: LayersState
├── auto_switch: Option<AutoSwitchView>
├── hotkeys: Option<HotkeyView>
└── focus_tracker: FocusTracker
```

### Message Dispatch

Top-level `Message` wraps sub-view `Msg` enums:

```rust
pub enum Message {
    // App-level
    Navigate(NavItem),
    ThemeToggled,
    NotificationDismissed(usize),
    TickAnimations,

    // Delegated to sub-views
    Device(views::devices::Msg),
    Macro(views::macros::Msg),
    Profile(views::profiles::Msg),
    Keypad(views::keypad::Msg),
    Analog(views::analog::Msg),
    Led(views::led::Msg),
    Layer(views::layers::Msg),
    AutoSwitch(views::auto_switch::Msg),
    Hotkey(views::hotkeys::Msg),
}
```

Each view module owns its `update()` logic. The app-level `update()` dispatches to the appropriate module.

## Layout

```
┌─────────────────────────────────────────────────┐
│  [Aethermap]          ○ Connected    ◐ Dark/Light │  ← Header (40px)
├────────┬────────────────────────────────────────┤
│        │                                         │
│  Nav   │          Content Area                   │
│  (200px│                                         │
│  wide) │   (changes based on selected nav item)  │
│        │                                         │
│ Devices│                                         │
│ Macros │                                         │
│ Profil │                                         │
│ Layers │                                         │
│ LEDs   │                                         │
│ Hotkeys│                                         │
│ Auto   │                                         │
│        │                                         │
│        │─────────────────────────────────────────│
│        │  [Status indicators / notifications]    │  ← Footer (32px)
└────────┴─────────────────────────────────────────┘
```

### Navigation Items

Promote all features to top-level nav (no hidden sub-panels):
1. Devices
2. Macros
3. Profiles
4. Layers
5. LEDs
6. Hotkeys
7. Auto-Switch

### Overlays

Analog calibration and per-device LED config remain as overlays with bg-overlay backdrop.

## User Flow Improvements

### Key Remapping

- **Current:** Grab device → profile tab → type key name → save
- **New:** Grab device → keypad card appears in devices view → click button → inline remap input → Enter to apply → auto-saved

### Macro Recording

- **Current:** Macros tab → type name → start → stop → appears in list
- **New:** Macros view → prominent "Record" button at top → click → inline name prompt → recording starts → button becomes "Stop Recording" → macro in list below. Settings in collapsible panel.

### Profile Management

- **Current:** Profiles mixed with device management, activation unclear
- **New:** Profiles view shows card grid. Active profile = accent border. Click to activate. Double-click to edit. "New Profile" button at top.

### Analog Calibration

- **Current:** All sliders always visible in dense form
- **New:** Grouped into collapsible sections (Deadzone, Sensitivity, Range, Inversion). Visualizer always visible on right. Changes apply live.

### Notifications

- **Current:** Inline notifications that push content down
- **New:** Toast notifications from top-right. Auto-dismiss after 3s. Stack vertically. Errors stay until dismissed.

## Preserved Functionality

All existing features preserved:
- Device grab/ungrab
- Macro record/play/delete with settings (latency, jitter, mouse capture)
- Profile save/load/activate/deactivate
- Per-device profiles
- Remap profiles with keypad layout
- Layer management (toggle, hold, config)
- Analog calibration (deadzone, sensitivity, range, inversion)
- LED configuration (zones, colors, patterns)
- Global hotkey bindings
- Auto-profile switching rules
- Focus tracking via ashpd/Wayland portal
- Dark/light theme toggle
- Daemon connection status

## Constraints

- Stay on Iced 0.12 (no framework changes)
- Keep existing `ipc.rs` and `focus_tracker.rs` unchanged
- Keep existing custom widgets (`AnalogVisualizer`, `CurveGraph`)
- No new external dependencies for styling (Iced handles this natively)
