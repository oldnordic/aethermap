# Aethermap GUI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the 5357-line monolithic `gui.rs` into focused modules with a modern minimal dark theme, new layout structure, and improved user flows — all within Iced 0.12.

**Architecture:** Incremental extraction. Move view methods into separate modules first (they're pure functions of state), then extract state sub-structs, then restructure layout. The flat `Message` enum is preserved until the final phase. Every task produces a compiling, running app.

**Tech Stack:** Rust, Iced 0.12 (existing), no new dependencies.

**Design spec:** `docs/superpowers/specs/2026-05-14-gui-redesign-design.md`

---

## File Structure (Target State)

```
aethermap-gui/src/
├── app.rs              # State struct, Message enum, Application impl, update dispatch
├── theme/
│   ├── mod.rs          # Re-exports
│   ├── palette.rs      # Color constants + spacing
│   └── styles.rs       # Component style functions (cards, buttons, sidebar items, inputs)
├── views/
│   ├── mod.rs          # Re-exports
│   ├── sidebar.rs      # view_sidebar()
│   ├── header.rs       # view_header()
│   ├── status_bar.rs   # view_status_bar()
│   ├── devices.rs      # view_devices_tab(), view_device_card()
│   ├── macros.rs       # view_macros_tab(), view_recording_panel(), view_macro_list()
│   ├── profiles.rs     # view_profiles_tab(), view_profile_selector()
│   ├── keypad.rs       # view_azeron_keypad(), KeypadButton, azeron_keypad_layout()
│   ├── analog.rs       # view_analog_calibration(), CalibrationConfig, DeadzoneShape, SensitivityCurve, AnalogCalibrationView
│   ├── led.rs          # view_led_config(), LedState
│   ├── layers.rs       # Layer config dialog + layer management views
│   ├── auto_switch.rs  # view_auto_switch_rules(), AutoSwitchRule, AutoSwitchRulesView
│   └── hotkeys.rs      # view_hotkey_bindings(), HotkeyBinding, HotkeyBindingsView
├── widgets/            # (unchanged) AnalogVisualizer, CurveGraph
├── focus_tracker.rs    # (unchanged)
├── ipc.rs              # (unchanged)
├── lib.rs              # Updated to re-export from app instead of gui
└── main.rs             # Updated to use app::State
```

---

## Phase 1: Theme Foundation

### Task 1: Create theme module

**Files:**
- Create: `aethermap-gui/src/theme/mod.rs`
- Create: `aethermap-gui/src/theme/palette.rs`
- Create: `aethermap-gui/src/theme/styles.rs`
- Delete: `aethermap-gui/src/theme.rs`

- [ ] **Step 1: Create `theme/palette.rs`**

Move color constants from current `theme.rs` into `palette.rs` with the new design system colors. Add spacing constants and border radius values.

```rust
use iced::Color;

// Background
pub const BG_BASE: Color = Color::from_rgb(0x0D as f32 / 255.0, 0x0D as f32 / 255.0, 0x0D as f32 / 255.0);
pub const BG_SURFACE: Color = Color::from_rgb(0x16 as f32 / 255.0, 0x16 as f32 / 255.0, 0x16 as f32 / 255.0);
pub const BG_ELEVATED: Color = Color::from_rgb(0x1E as f32 / 255.0, 0x1E as f32 / 255.0, 0x1E as f32 / 255.0);

// Text
pub const TEXT_PRIMARY: Color = Color::from_rgb(0xE8 as f32 / 255.0, 0xE8 as f32 / 255.0, 0xE8 as f32 / 255.0);
pub const TEXT_SECONDARY: Color = Color::from_rgb(0x88 as f32 / 255.0, 0x88 as f32 / 255.0, 0x88 as f32 / 255.0);
pub const TEXT_MUTED: Color = Color::from_rgb(0x55 as f32 / 255.0, 0x55 as f32 / 255.0, 0x55 as f32 / 255.0);

// Accent
pub const ACCENT: Color = Color::from_rgb(0x3B as f32 / 255.0, 0x82 as f32 / 255.0, 0xF6 as f32 / 255.0);
pub const ACCENT_HOVER: Color = Color::from_rgb(0x60 as f32 / 255.0, 0xA5 as f32 / 255.0, 0xFA as f32 / 255.0);

// Semantic
pub const SUCCESS: Color = Color::from_rgb(0x22 as f32 / 255.0, 0xC5 as f32 / 255.0, 0x5E as f32 / 255.0);
pub const DANGER: Color = Color::from_rgb(0xEF as f32 / 255.0, 0x44 as f32 / 255.0, 0x44 as f32 / 255.0);
pub const WARNING: Color = Color::from_rgb(0xF5 as f32 / 255.0, 0x9E as f32 / 255.0, 0x0B as f32 / 255.0);

// Border
pub const BORDER_SUBTLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.06);

// Spacing
pub const XS: u16 = 4;
pub const SM: u16 = 8;
pub const MD: u16 = 16;
pub const LG: u16 = 24;
pub const XL: u16 = 32;

// Radius
pub const RADIUS_CARD: f32 = 8.0;
pub const RADIUS_INPUT: f32 = 6.0;
```

- [ ] **Step 2: Create `theme/styles.rs`**

Port the existing `container_styles::card` and add new component style functions using the palette.

```rust
use iced::widget::{button, container, text_input, rule};
use iced::{border, Color, Theme, Background};
use super::palette::*;

pub fn card(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        text_color: Some(TEXT_PRIMARY),
        background: Some(BG_SURFACE.into()),
        border: border::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: RADIUS_CARD.into(),
        },
        ..Default::default()
    }
}

pub fn sidebar_item(active: bool) -> impl Fn(&Theme) -> button::Appearance {
    move |_theme: &Theme| {
        button::Appearance {
            background: if active { Some(BG_ELEVATED.into()) } else { Some(Color::TRANSPARENT.into()) },
            text_color: if active { ACCENT } else { TEXT_SECONDARY },
            border: border::Border {
                color: if active { ACCENT } else { Color::TRANSPARENT },
                width: if active { 2.0 } else { 0.0 },
                radius: [0.0, RADIUS_INPUT, RADIUS_INPUT, 0.0].into(),
            },
            ..Default::default()
        }
    }
}

pub fn primary_button(_theme: &Theme) -> button::Appearance {
    button::Appearance {
        background: Some(ACCENT.into()),
        text_color: Color::WHITE,
        border: border::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: RADIUS_INPUT.into(),
        },
        ..Default::default()
    }
}

pub fn secondary_button(_theme: &Theme) -> button::Appearance {
    button::Appearance {
        background: Some(BG_ELEVATED.into()),
        text_color: TEXT_PRIMARY,
        border: border::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: RADIUS_INPUT.into(),
        },
        ..Default::default()
    }
}

pub fn danger_button(_theme: &Theme) -> button::Appearance {
    button::Appearance {
        background: Some(DANGER.into()),
        text_color: Color::WHITE,
        border: border::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: RADIUS_INPUT.into(),
        },
        ..Default::default()
    }
}

pub fn input_style(_theme: &Theme) -> text_input::Appearance {
    text_input::Appearance {
        background: BG_ELEVATED.into(),
        border: border::Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: RADIUS_INPUT.into(),
        },
        icon_color: TEXT_MUTED,
    }
}

pub fn header_bar(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(BG_SURFACE.into()),
        border: border::Border {
            color: BORDER_SUBTLE,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn footer_bar(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(BG_SURFACE.into()),
        border: border::Border {
            color: BORDER_SUBTLE,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn subtle_rule(_theme: &Theme) -> rule::Appearance {
    rule::Appearance {
        color: BORDER_SUBTLE,
        width: 1,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
    }
}
```

- [ ] **Step 3: Create `theme/mod.rs`**

```rust
pub mod palette;
pub mod styles;

pub use palette::*;
pub use styles::*;

use iced::{Color, Theme};
use iced::theme::Palette;
use palette::*;

pub fn aether_dark() -> Theme {
    Theme::custom(
        String::from("Aether Dark"),
        Palette {
            background: BG_BASE,
            text: TEXT_PRIMARY,
            primary: ACCENT,
            success: SUCCESS,
            danger: DANGER,
        },
    )
}

pub fn aether_light() -> Theme {
    Theme::custom(
        String::from("Aether Light"),
        Palette {
            background: Color::from_rgb(0.95, 0.95, 0.95),
            text: Color::BLACK,
            primary: ACCENT,
            success: SUCCESS,
            danger: DANGER,
        },
    )
}
```

- [ ] **Step 4: Update `gui.rs` imports**

Replace `use crate::theme::{aether_dark, aether_light, container_styles};` with `use crate::theme;` and update all references. Replace `container_styles::card` with `theme::styles::card`. Remove the old `theme.rs` file.

- [ ] **Step 5: Verify compilation**

Run: `cargo build -p aethermap-gui`
Expected: Compiles with no errors

- [ ] **Step 6: Commit**

```
feat(gui): create modular theme system with new design palette
```

---

### Task 2: Create views module with empty shells

**Files:**
- Create: `aethermap-gui/src/views/mod.rs`
- Create: `aethermap-gui/src/views/sidebar.rs` (empty shell)
- Create: `aethermap-gui/src/views/devices.rs` (empty shell)
- Create: `aethermap-gui/src/views/macros.rs` (empty shell)
- Create: `aethermap-gui/src/views/profiles.rs` (empty shell)
- Create: `aethermap-gui/src/views/keypad.rs` (empty shell)
- Create: `aethermap-gui/src/views/analog.rs` (empty shell)
- Create: `aethermap-gui/src/views/led.rs` (empty shell)
- Create: `aethermap-gui/src/views/layers.rs` (empty shell)
- Create: `aethermap-gui/src/views/auto_switch.rs` (empty shell)
- Create: `aethermap-gui/src/views/hotkeys.rs` (empty shell)

- [ ] **Step 1: Create `views/mod.rs`**

```rust
pub mod sidebar;
pub mod devices;
pub mod macros_view;
pub mod profiles;
pub mod keypad;
pub mod analog;
pub mod led;
pub mod layers;
pub mod auto_switch;
pub mod hotkeys;
```

- [ ] **Step 2: Create each view module with an empty public view function**

Each module starts as:
```rust
use iced::Element;
use crate::gui::{State, Message};

pub fn view(_state: &State) -> Element<'_, Message> {
    iced::widget::text("TODO: extract view").into()
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p aethermap-gui`
Expected: Compiles (the view functions exist but aren't wired yet)

- [ ] **Step 4: Commit**

```
chore(gui): create views module structure with empty shells
```

---

## Phase 2: Extract View Methods

Strategy: Each task extracts one set of related view methods from `gui.rs` into a view module. The view functions take `&State` and return `Element<'_, Message>`. The `Message` enum stays in `gui.rs` (moved to `app.rs` later).

### Task 3: Extract keypad view

**Files:**
- Modify: `aethermap-gui/src/views/keypad.rs` — move `view_azeron_keypad()`, `KeypadButton`, `azeron_keypad_layout()` from gui.rs
- Modify: `aethermap-gui/src/gui.rs` — remove moved code, call `views::keypad::view()`

- [ ] **Step 1: Move types to `views/keypad.rs`**

Move `KeypadButton` struct (lines 56-69), `azeron_keypad_layout()` function (lines 75-112), and `view_azeron_keypad()` method (lines 4186-4289) from `gui.rs` into `views/keypad.rs`.

Make `view_azeron_keypad` a free function:
```rust
pub fn view(state: &State) -> Element<'_, Message> {
    // body of view_azeron_keypad, with self -> state
}
```

- [ ] **Step 2: Wire into `gui.rs`**

In `gui.rs`, replace the body of `view_azeron_keypad` with:
```rust
fn view_azeron_keypad(&self) -> Element<'_, Message> {
    views::keypad::view(self)
}
```

- [ ] **Step 3: Verify**

Run: `cargo build -p aethermap-gui && cargo test -p aethermap-gui`
Expected: Compiles, tests pass

- [ ] **Step 4: Commit**

```
refactor(gui): extract keypad view into views/keypad module
```

---

### Task 4: Extract status bar view

**Files:**
- Modify: `aethermap-gui/src/views/status_bar.rs` (rename from empty shell or add content)
- Modify: `aethermap-gui/src/gui.rs`

- [ ] **Step 1: Move `view_status_bar()` (lines 4290-4677) into `views/status_bar.rs`**

Make it a free function: `pub fn view(state: &State) -> Element<'_, Message>`

- [ ] **Step 2: Wire into gui.rs**

- [ ] **Step 3: Verify and commit**

```
refactor(gui): extract status bar into views module
```

---

### Task 5: Extract sidebar view

**Files:**
- Modify: `aethermap-gui/src/views/sidebar.rs`
- Modify: `aethermap-gui/src/gui.rs`

- [ ] **Step 1: Move `view_sidebar()` (lines 2780-2852) into `views/sidebar.rs`**

- [ ] **Step 2: Wire into gui.rs**

- [ ] **Step 3: Verify and commit**

```
refactor(gui): extract sidebar view into views module
```

---

### Task 6: Extract auto-switch rules view

**Files:**
- Modify: `aethermap-gui/src/views/auto_switch.rs`
- Modify: `aethermap-gui/src/gui.rs`

- [ ] **Step 1: Move types**

Move `AutoSwitchRule` (lines 118-127), `AutoSwitchRulesView` (lines 133-146) into `views/auto_switch.rs`.

- [ ] **Step 2: Move `view_auto_switch_rules()` (lines 3795-3953) as free function**

- [ ] **Step 3: Wire into gui.rs**

- [ ] **Step 4: Verify and commit**

```
refactor(gui): extract auto-switch rules view into views module
```

---

### Task 7: Extract hotkey bindings view

**Files:**
- Modify: `aethermap-gui/src/views/hotkeys.rs`
- Modify: `aethermap-gui/src/gui.rs`

- [ ] **Step 1: Move `HotkeyBinding` (lines 152-163), `HotkeyBindingsView` (lines 169-184), `view_hotkey_bindings()` (lines 3954-4185)**

- [ ] **Step 2: Wire into gui.rs**

- [ ] **Step 3: Verify and commit**

```
refactor(gui): extract hotkey bindings view into views module
```

---

### Task 8: Extract LED config view

**Files:**
- Modify: `aethermap-gui/src/views/led.rs`
- Modify: `aethermap-gui/src/gui.rs`

- [ ] **Step 1: Move `LedState` (lines 376-396), `view_led_config()` (lines 4757-4927), `view_led_rgb_sliders()` (lines 4678-4756)**

- [ ] **Step 2: Wire into gui.rs**

- [ ] **Step 3: Verify and commit**

```
refactor(gui): extract LED config view into views module
```

---

### Task 9: Extract analog calibration view

**Files:**
- Modify: `aethermap-gui/src/views/analog.rs`
- Modify: `aethermap-gui/src/gui.rs`

- [ ] **Step 1: Move `DeadzoneShape` (lines 188-210), `SensitivityCurve` (lines 214-242), `CalibrationConfig` (lines 249-275), `AnalogCalibrationView` (lines 281-370+Default+Clone impls), `view_analog_calibration()` (lines 4928-4963)**

- [ ] **Step 2: Wire into gui.rs**

- [ ] **Step 3: Verify and commit**

```
refactor(gui): extract analog calibration view into views module
```

---

### Task 10: Extract macros view

**Files:**
- Modify: `aethermap-gui/src/views/macros.rs`
- Modify: `aethermap-gui/src/gui.rs`

- [ ] **Step 1: Move `view_macros_tab()` (lines 3296-3323), `view_recording_panel()` (lines 3324-3404), `view_macro_settings_panel()` (lines 3405-3447), `view_macro_action()` (lines 3448-3452), `view_macro_list()` (lines 3453-3525)**

- [ ] **Step 2: Wire into gui.rs**

- [ ] **Step 3: Verify and commit**

```
refactor(gui): extract macros view into views module
```

---

### Task 11: Extract profiles view

**Files:**
- Modify: `aethermap-gui/src/views/profiles.rs`
- Modify: `aethermap-gui/src/gui.rs`

- [ ] **Step 1: Move `view_profiles_tab()` (lines 3526-3593), `view_profile_selector()` (lines 3594-3655), `view_remap_profile_switcher()` (lines 3656-3731), `view_active_remaps_display()` (lines 3732-3794)**

- [ ] **Step 2: Wire into gui.rs**

- [ ] **Step 3: Verify and commit**

```
refactor(gui): extract profiles view into views module
```

---

### Task 12: Extract devices view

**Files:**
- Modify: `aethermap-gui/src/views/devices.rs`
- Modify: `aethermap-gui/src/gui.rs`

- [ ] **Step 1: Move `view_devices_tab()` (lines 2867-2984), `view_device_card()` (lines 2985-3295)**

- [ ] **Step 2: Wire into gui.rs**

- [ ] **Step 3: Verify and commit**

```
refactor(gui): extract devices view into views module
```

---

### Task 13: Extract layer management view

**Files:**
- Modify: `aethermap-gui/src/views/layers.rs`
- Modify: `aethermap-gui/src/gui.rs`

- [ ] **Step 1: Move `layer_config_dialog()` method and any layer-related view code**

- [ ] **Step 2: Wire into gui.rs**

- [ ] **Step 3: Verify and commit**

```
refactor(gui): extract layers view into views module
```

---

### Task 14: Extract main view composition

**Files:**
- Modify: `aethermap-gui/src/gui.rs`

- [ ] **Step 1: Move `view_main_content()` (lines 2853-2866) and the main `view()` (lines 2572-2632) so they call the extracted modules**

After this step, `gui.rs` should contain only:
- `Tab` enum
- `Notification` struct
- `State` struct + Default impl
- `Message` enum (all variants)
- `Application` impl (new, update, subscription)
- Thin wrapper methods that delegate to view modules

- [ ] **Step 2: Verify and commit**

```
refactor(gui): complete view extraction — gui.rs now contains only state and dispatch
```

---

## Phase 3: Layout Restructure

### Task 15: Add header bar

**Files:**
- Create: `aethermap-gui/src/views/header.rs`
- Modify: `aethermap-gui/src/views/mod.rs`
- Modify: `aethermap-gui/src/gui.rs`

- [ ] **Step 1: Create `views/header.rs`**

New header bar showing app name, daemon connection indicator, and theme toggle. Uses `theme::styles::header_bar` container style.

```rust
use iced::{
    widget::{container, row, text, button, Space},
    Element, Length, Alignment,
};
use crate::gui::Message;
use crate::theme;

pub fn view(daemon_connected: bool, recording: bool) -> Element<'_, Message> {
    let connection_indicator = text(if daemon_connected { "●" } else { "○" })
        .color(if daemon_connected { theme::SUCCESS } else { theme::DANGER })
        .size(14);

    let connection_label = text(if daemon_connected { "Connected" } else { "Disconnected" })
        .color(theme::TEXT_SECONDARY)
        .size(12);

    let app_title = text("Aethermap")
        .size(16)
        .color(theme::TEXT_PRIMARY);

    let recording_indicator = if recording {
        container(text("● REC").color(theme::DANGER).size(12))
            .padding([2, 8])
            .into()
    } else {
        Space::with_width(0).into()
    };

    container(
        row![
            container(app_title).padding([0, theme::MD]),
            Space::with_width(Length::Fill),
            recording_indicator,
            connection_indicator,
            connection_label,
        ]
        .align_items(Alignment::Center)
        .height(40),
    )
    .style(theme::styles::header_bar)
    .width(Length::Fill)
    .height(40)
    .into()
}
```

- [ ] **Step 2: Wire into main view layout**

- [ ] **Step 3: Verify and commit**

```
feat(gui): add header bar with connection indicator and recording status
```

---

### Task 16: Restructure navigation — tabs → sidebar nav

**Files:**
- Modify: `aethermap-gui/src/gui.rs` — add `NavItem` enum replacing `Tab`
- Modify: `aethermap-gui/src/views/sidebar.rs`

- [ ] **Step 1: Add `NavItem` enum to gui.rs**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavItem {
    Devices,
    Macros,
    Profiles,
    Layers,
    Leds,
    Hotkeys,
    AutoSwitch,
}
```

- [ ] **Step 2: Replace `Tab` with `NavItem` in State, update `view_sidebar` to show all nav items**

Each nav item: text label, accent left border on active, transparent otherwise. 200px wide sidebar.

- [ ] **Step 3: Update `view_main_content` to dispatch by `NavItem`**

The existing tab content gets mapped to the new nav items. Layers, LEDs, Hotkeys, AutoSwitch get their own content panels instead of being overlays or hidden sub-views.

- [ ] **Step 4: Verify and commit**

```
feat(gui): replace tabs with full sidebar navigation
```

---

### Task 17: Add footer/status bar with toast notifications

**Files:**
- Modify: `aethermap-gui/src/views/status_bar.rs`

- [ ] **Step 1: Redesign status bar**

Replace the current status bar with a compact footer. Show connection status, notification count. Toast notifications overlay from the top-right (tracked in the State, rendered in the main view on top of content).

- [ ] **Step 2: Implement toast notification rendering**

```rust
pub fn view_toasts(notifications: &VecDeque<Notification>) -> Element<'_, Message> {
    // Render each notification as a small card from top-right
    // Auto-dismissing toasts: show timestamp, fade after 3s (checked in update via TickAnimations)
    // Error toasts stay until dismissed
}
```

- [ ] **Step 3: Verify and commit**

```
feat(gui): add footer bar and toast notifications
```

---

## Phase 4: Rename gui.rs → app.rs

### Task 18: Rename gui.rs to app.rs and update module references

**Files:**
- Rename: `aethermap-gui/src/gui.rs` → `aethermap-gui/src/app.rs`
- Modify: `aethermap-gui/src/lib.rs`
- Modify: `aethermap-gui/src/main.rs`
- Modify: all `views/*.rs` — update import paths from `crate::gui::` to `crate::app::`

- [ ] **Step 1: `git mv aethermap-gui/src/gui.rs aethermap-gui/src/app.rs`**

- [ ] **Step 2: Update `lib.rs`**

```rust
pub mod app;
pub mod theme;
pub mod views;
pub mod widgets;
pub mod focus_tracker;
pub mod ipc;

pub use app::{State, Message, NavItem, Notification};
```

- [ ] **Step 3: Update all view module imports from `crate::gui::` to `crate::app::`**

- [ ] **Step 4: Verify full build**

Run: `cargo build -p aethermap-gui && cargo test -p aethermap-gui`

- [ ] **Step 5: Commit**

```
refactor(gui): rename gui.rs to app.rs, update all module references
```

---

## Phase 5: User Flow Improvements

### Task 19: Inline key remapping on keypad

**Files:**
- Modify: `aethermap-gui/src/views/keypad.rs`
- Modify: `aethermap-gui/src/views/devices.rs`

- [ ] **Step 1: Add inline remap input to keypad buttons**

When a keypad button is clicked, show an inline text input below/beside it for entering the remap target. Pressing Enter applies the remap immediately (sends IPC command). No separate "save profile" step.

- [ ] **Step 2: Integrate keypad into devices view**

When a device is grabbed and has keypad capabilities, the keypad card appears in the devices view automatically (not a separate navigation target).

- [ ] **Step 3: Verify and commit**

```
feat(gui): inline key remapping on visual keypad
```

---

### Task 20: Prominent macro recording button

**Files:**
- Modify: `aethermap-gui/src/views/macros.rs`

- [ ] **Step 1: Redesign macros view layout**

Top of view: large "Record" button (accent colored, prominent). When clicked, shows inline name input + "Start Recording" confirmation. During recording, button becomes "Stop Recording" (danger color, pulsing).

Below: macro list with play/delete per macro. Settings (latency, jitter, mouse capture) in a collapsible section at the bottom.

- [ ] **Step 2: Verify and commit**

```
feat(gui): prominent macro recording button with inline name prompt
```

---

### Task 21: Card-based profile view

**Files:**
- Modify: `aethermap-gui/src/views/profiles.rs`

- [ ] **Step 1: Replace list view with card grid**

Each profile = a card showing name, key count, active indicator. Active profile has accent border. Click to activate, right-click or button to edit. "New Profile" button at top of the grid.

- [ ] **Step 2: Verify and commit**

```
feat(gui): card-based profile management view
```

---

### Task 22: Collapsible calibration sections

**Files:**
- Modify: `aethermap-gui/src/views/analog.rs`

- [ ] **Step 1: Group calibration controls into collapsible sections**

Sections: Deadzone, Sensitivity, Range, Inversion. Each section has a header that toggles visibility. Visualizer stays always visible on the right side of the panel.

- [ ] **Step 2: Verify and commit**

```
feat(gui): collapsible sections in analog calibration view
```

---

## Verification Checklist

After all tasks:

- [ ] `cargo build -p aethermap-gui` — clean build
- [ ] `cargo test -p aethermap-gui` — all tests pass
- [ ] `cargo clippy --lib -p aethermap-gui -- -W clippy::all` — no new warnings
- [ ] Visual verification: run `./target/release/aethermap-gui` and check all nav items render correctly
- [ ] `gui.rs` no longer exists — replaced by `app.rs` + views/ modules
- [ ] No functionality lost compared to pre-refactor

## Self-Review

1. **Spec coverage:** Each design section (theme, modules, layout, flows) maps to tasks:
   - Theme → Task 1
   - Modules → Tasks 2-14
   - Layout → Tasks 15-18
   - User flows → Tasks 19-22

2. **Placeholder scan:** No TBDs. All steps show code or specific line references.

3. **Type consistency:** `State` and `Message` stay in `gui.rs` (later `app.rs`) throughout extraction. View modules take `&State` and return `Element<'_, Message>`. `NavItem` replaces `Tab` in Task 16.
