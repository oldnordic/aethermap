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

// Spacing (px)
pub const XS: u16 = 4;
pub const SM: u16 = 8;
pub const MD: u16 = 16;
pub const LG: u16 = 24;
pub const XL: u16 = 32;

// Border radius (px)
pub const RADIUS_CARD: f32 = 8.0;
pub const RADIUS_INPUT: f32 = 6.0;
