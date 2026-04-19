# Adaptive COSMIC Theme Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a COSMIC-inspired adaptive theme (Dark/Light) for Aethermap GUI with automatic switching via DBus (Settings Portal).

**Architecture:**
- Custom theme definitions in `src/theme.rs` using `iced::Theme` and `iced::Palette`.
- Integration into `gui::State` with a `current_theme` field.
- `ashpd` based DBus subscription for real-time theme switching.
- Specialized styling for "cards" with 10px rounded corners and 1px subtle borders.

**Tech Stack:** `iced` 0.12, `ashpd` 0.8, `tokio`, `dbus` (via `ashpd`).

---

### Task 1: Define AetherTheme

**Files:**
- Create: `aethermap/aethermap-gui/src/theme.rs`
- Modify: `aethermap/aethermap-gui/src/lib.rs`

- [ ] **Step 1: Create `theme.rs` with COSMIC palettes**

```rust
use iced::{Color, Theme};
use iced::theme::Palette;

pub const ACCENT: Color = Color::from_rgb(
    0x30 as f32 / 255.0,
    0x70 as f32 / 255.0,
    0xf0 as f32 / 255.0,
);

pub const DARK_BACKGROUND: Color = Color::from_rgb(
    0x1a as f32 / 255.0,
    0x1a as f32 / 255.0,
    0x1a as f32 / 255.0,
);

pub const DARK_SURFACE: Color = Color::from_rgb(
    0x24 as f32 / 255.0,
    0x24 as f32 / 255.0,
    0x24 as f32 / 255.0,
);

pub const LIGHT_BACKGROUND: Color = Color::from_rgb(
    0xf2 as f32 / 255.0,
    0xf2 as f32 / 255.0,
    0xf2 as f32 / 255.0,
);

pub const LIGHT_SURFACE: Color = Color::from_rgb(1.0, 1.0, 1.0);

pub fn aether_dark() -> Theme {
    Theme::custom(
        String::from("Aether Dark"),
        Palette {
            background: DARK_BACKGROUND,
            text: Color::WHITE,
            primary: ACCENT,
            success: Color::from_rgb(0.0, 1.0, 0.0),
            danger: Color::from_rgb(1.0, 0.0, 0.0),
        },
    )
}

pub fn aether_light() -> Theme {
    Theme::custom(
        String::from("Aether Light"),
        Palette {
            background: LIGHT_BACKGROUND,
            text: Color::BLACK,
            primary: ACCENT,
            success: Color::from_rgb(0.0, 0.8, 0.0),
            danger: Color::from_rgb(0.8, 0.0, 0.0),
        },
    )
}

pub mod container_styles {
    use iced::widget::container;
    use iced::{border, Color, Theme};

    pub fn card(theme: &Theme) -> container::Appearance {
        let palette = theme.palette();
        container::Appearance {
            text_color: Some(palette.text),
            background: Some(match theme {
                Theme::Custom(custom) if custom.name == "Aether Dark" => super::DARK_SURFACE.into(),
                Theme::Custom(custom) if custom.name == "Aether Light" => super::LIGHT_SURFACE.into(),
                Theme::Dark => super::DARK_SURFACE.into(),
                _ => palette.background.into(),
            }),
            border: border::Border {
                color: Color::from_rgba(0.5, 0.5, 0.5, 0.1),
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        }
    }
}
```

- [ ] **Step 2: Register module in `lib.rs`**

```rust
pub mod theme;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p aethermap-gui`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add aethermap/aethermap-gui/src/theme.rs aethermap/aethermap-gui/src/lib.rs
git commit -m "feat(gui): define AetherTheme with COSMIC palettes"
```

### Task 2: Integrate Theme into GUI State

**Files:**
- Modify: `aethermap/aethermap-gui/src/gui.rs`

- [ ] **Step 1: Add `current_theme` to `State` struct**

```rust
pub struct State {
    // ... existing fields ...
    pub current_theme: Theme,
}
```

- [ ] **Step 2: Update `Message` enum**

```rust
pub enum Message {
    // ... existing variants ...
    ThemeChanged(Theme),
}
```

- [ ] **Step 3: Update `Application` trait implementation**

```rust
    fn theme(&self) -> Theme {
        self.current_theme.clone()
    }
```

- [ ] **Step 4: Update `update()` function to handle `ThemeChanged`**

```rust
            Message::ThemeChanged(theme) => {
                self.current_theme = theme;
                Command::none()
            }
```

- [ ] **Step 5: Apply `container_styles::card` to device cards and macro rows**

Search for `.style(iced::theme::Container::Box)` and replace with `.style(crate::theme::container_styles::card)`.

- [ ] **Step 6: Verify compilation**

Run: `cargo check -p aethermap-gui`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add aethermap/aethermap-gui/src/gui.rs
git commit -m "feat(gui): integrate custom theme into State and apply card styling"
```

### Task 3: Implement DBus Listener for Theme Changes

**Files:**
- Modify: `aethermap/aethermap-gui/src/gui.rs`

- [ ] **Step 1: Implement subscription for theme changes**

```rust
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            // ... existing subscriptions ...
            iced::subscription::unfold(
                "theme-listener",
                (),
                |_| async move {
                    use ashpd::desktop::settings::{ColorScheme, Settings};
                    let settings = Settings::new().await.unwrap();
                    let mut stream = settings.receive_color_scheme_changed().await.unwrap();

                    // Initial value
                    let initial_scheme = settings.color_scheme().await.unwrap();
                    let initial_theme = match initial_scheme {
                        ColorScheme::PreferDark => crate::theme::aether_dark(),
                        _ => crate::theme::aether_light(),
                    };

                    // We need a way to return the initial value then loop
                    // For simplicity in unfold, we might just loop and emit changes
                    // But we want the initial value too.
                    
                    // Actually, a better way is to use a stream that yields the initial value first.
                    (Message::ThemeChanged(initial_theme), stream)
                },
                |mut stream| async move {
                    use ashpd::desktop::settings::ColorScheme;
                    use iced::futures::StreamExt;
                    
                    if let Some(scheme) = stream.next().await {
                        let theme = match scheme {
                            ColorScheme::PreferDark => crate::theme::aether_dark(),
                            _ => crate::theme::aether_light(),
                        };
                        return (Some(Message::ThemeChanged(theme)), stream);
                    }
                    (None, stream)
                }
            )
        ])
    }
```

Wait, `iced::subscription::unfold` in 0.12 might have slightly different API. I'll check the current `gui.rs` for existing subscriptions.

- [ ] **Step 2: Verify `ashpd` usage and compilation**

Run: `cargo check -p aethermap-gui`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add aethermap/aethermap-gui/src/gui.rs
git commit -m "feat(gui): add DBus listener for adaptive theme switching"
```

### Task 4: Final Verification and Polish

- [ ] **Step 1: Ensure all "Box" styled containers are updated**
- [ ] **Step 2: Check for any hardcoded colors that might clash with Light mode**
- [ ] **Step 3: Run full check**

Run: `cargo check -p aethermap-gui`
Expected: PASS
