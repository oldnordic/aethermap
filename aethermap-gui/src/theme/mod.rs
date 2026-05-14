pub mod palette;
pub mod styles;

pub use palette::*;
pub use styles::*;

use iced::{Color, Theme};
use iced::theme::Palette;

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
