use super::palette::*;
use iced::widget::{button, container, rule, text_input};
use iced::{border, Color, Theme};

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
    move |_theme: &Theme| button::Appearance {
        background: if active {
            Some(BG_ELEVATED.into())
        } else {
            Some(Color::TRANSPARENT.into())
        },
        text_color: if active { ACCENT } else { TEXT_SECONDARY },
        border: border::Border {
            color: if active { ACCENT } else { Color::TRANSPARENT },
            width: if active { 2.0 } else { 0.0 },
            radius: [0.0, RADIUS_INPUT, RADIUS_INPUT, 0.0].into(),
        },
        ..Default::default()
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
