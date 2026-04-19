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
            background: Some(if palette.background == super::DARK_BACKGROUND {
                super::DARK_SURFACE.into()
            } else if palette.background == super::LIGHT_BACKGROUND {
                super::LIGHT_SURFACE.into()
            } else {
                palette.background.into()
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
