use std::collections::HashMap;
use iced::{
    widget::{button, column, container, horizontal_rule, row, slider, text, Column, Space},
    Element, Length, Alignment, Color, Theme,
};
use aethermap_common::LedZone;
use aethermap_common::LedPattern;
use crate::gui::{State, Message};
use crate::theme;

#[derive(Debug, Clone)]
pub struct LedState {
    pub zone_colors: HashMap<LedZone, (u8, u8, u8)>,
    pub global_brightness: u8,
    pub zone_brightness: HashMap<LedZone, u8>,
    pub active_pattern: LedPattern,
}

impl Default for LedState {
    fn default() -> Self {
        Self {
            zone_colors: HashMap::new(),
            global_brightness: 100,
            zone_brightness: HashMap::new(),
            active_pattern: LedPattern::Static,
        }
    }
}

fn get_zone_color(state: &State, zone: LedZone) -> (u8, u8, u8) {
    if let Some(device_id) = &state.led_config_device {
        if let Some(led_state) = state.led_states.get(device_id) {
            if let Some(&color) = led_state.zone_colors.get(&zone) {
                return color;
            }
        }
    }
    (255, 255, 255)
}

fn led_color_style(zone: Option<LedZone>, zone_colors: &HashMap<LedZone, (u8, u8, u8)>) -> iced::theme::Container {
    let (r, g, b) = zone
        .and_then(|z| zone_colors.get(&z))
        .copied()
        .unwrap_or((255, 255, 255));

    struct LedColorStyle { r: u8, g: u8, b: u8 }

    impl iced::widget::container::StyleSheet for LedColorStyle {
        type Style = Theme;
        fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
            iced::widget::container::Appearance {
                background: Some(Color::from_rgb8(self.r, self.g, self.b).into()),
                ..Default::default()
            }
        }
    }

    iced::theme::Container::Custom(Box::new(LedColorStyle { r, g, b }))
}

fn view_led_rgb_sliders(state: &State) -> Element<'_, Message> {
    let zone = state.selected_led_zone.unwrap_or(LedZone::Logo);
    let (r, g, b) = state.pending_led_color.unwrap_or_else(|| get_zone_color(state, zone));

    Column::new()
        .spacing(8)
        .push(
            row![
                text("Red:").size(12).width(Length::Fixed(40.0)),
                text(format!("{}", r)).size(12).width(Length::Fixed(30.0)),
                slider(0..=255, r, move |v| {
                    Message::LedSliderChanged(v as u8, g, b)
                })
                .width(Length::Fill)
            ]
            .spacing(8)
            .align_items(Alignment::Center)
        )
        .push(
            row![
                text("Green:").size(12).width(Length::Fixed(40.0)),
                text(format!("{}", g)).size(12).width(Length::Fixed(30.0)),
                slider(0..=255, g, move |v| {
                    Message::LedSliderChanged(r, v as u8, b)
                })
                .width(Length::Fill)
            ]
            .spacing(8)
            .align_items(Alignment::Center)
        )
        .push(
            row![
                text("Blue:").size(12).width(Length::Fixed(40.0)),
                text(format!("{}", b)).size(12).width(Length::Fixed(30.0)),
                slider(0..=255, b, move |v| {
                    Message::LedSliderChanged(r, g, v as u8)
                })
                .width(Length::Fill)
            ]
            .spacing(8)
            .align_items(Alignment::Center)
        )
        .into()
}

pub fn view(state: &State) -> Option<Element<'_, Message>> {
    if let Some(ref device_id) = state.led_config_device {
        let selected_zone = state.selected_led_zone.unwrap_or(LedZone::Logo);
        let led_state = state.led_states.get(device_id);
        let zone_colors = led_state.map(|s| &s.zone_colors);
        let current_color = get_zone_color(state, selected_zone);

        let zones = vec![
            (LedZone::Logo, "Logo"),
            (LedZone::Keys, "Keys"),
            (LedZone::Thumbstick, "Thumbstick"),
        ];

        let zone_buttons: Vec<Element<'_, Message>> = zones
            .into_iter()
            .map(|(zone, label)| {
                let is_selected = state.selected_led_zone == Some(zone);
                button(text(label).size(12))
                    .on_press(Message::SelectLedZone(zone))
                    .style(if is_selected {
                        iced::theme::Button::Primary
                    } else {
                        iced::theme::Button::Secondary
                    })
                    .padding([6, 12])
                    .into()
            })
            .collect();

        let preview = container(
            container(
                text(format!("RGB({}, {}, {})", current_color.0, current_color.1, current_color.2))
                    .size(11)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
        )
        .width(Length::Fixed(120.0))
        .height(Length::Fixed(60.0))
        .style(if let Some(colors) = zone_colors {
            led_color_style(state.selected_led_zone, colors)
        } else {
            iced::theme::Container::Transparent
        });

        let patterns = vec![
            (LedPattern::Static, "Static"),
            (LedPattern::Breathing, "Breathing"),
            (LedPattern::Rainbow, "Rainbow"),
        ];

        let current_pattern = led_state.map(|s| s.active_pattern).unwrap_or(LedPattern::Static);

        let pattern_buttons: Vec<Element<'_, Message>> = patterns
            .into_iter()
            .map(|(pattern, label)| {
                let is_active = current_pattern == pattern;
                button(text(label).size(11))
                    .on_press(Message::SetLedPattern(device_id.clone(), pattern))
                    .style(if is_active {
                        iced::theme::Button::Primary
                    } else {
                        iced::theme::Button::Secondary
                    })
                    .padding([4, 10])
                    .into()
            })
            .collect();

        let brightness = led_state.map(|s| s.global_brightness as f32).unwrap_or(100.0);

        let dialog = container(
            column![
                row![
                    text("LED Configuration").size(18),
                    Space::with_width(Length::Fill),
                    button(text("\u{00d7}").size(20))
                        .on_press(Message::CloseLedConfig)
                        .style(iced::theme::Button::Text)
                        .padding([0, 8])
                ]
                .spacing(8)
                .align_items(Alignment::Center),
                horizontal_rule(1),
                text(device_id).size(11).width(Length::Fill),
                text("Zone:").size(13),
                row(zone_buttons).spacing(8),
                horizontal_rule(1),
                text("Color:").size(13),
                row![
                    preview,
                    column![
                        text("Adjust RGB sliders below").size(11),
                        text("to change color").size(11),
                    ]
                    .spacing(4)
                ]
                .spacing(12)
                .align_items(Alignment::Center),
                view_led_rgb_sliders(state),
                horizontal_rule(1),
                text(format!("Brightness: {}%", brightness as u8)).size(13),
                slider(0.0..=100.0, brightness, move |v| {
                    Message::SetLedBrightness(device_id.clone(), None, v as u8)
                })
                .width(Length::Fill),
                horizontal_rule(1),
                text("Pattern:").size(13),
                row(pattern_buttons).spacing(8),
                horizontal_rule(1),
                row![
                    Space::with_width(Length::Fill),
                    button(text("Close").size(13))
                        .on_press(Message::CloseLedConfig)
                        .style(iced::theme::Button::Secondary)
                        .padding([6, 16])
                ]
                .spacing(8)
            ]
            .spacing(12)
            .padding(20)
        )
        .max_width(500)
        .style(theme::styles::card);

        Some(
            container(dialog)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .padding(40)
                .style(iced::theme::Container::Transparent)
                .into(),
        )
    } else {
        None
    }
}
