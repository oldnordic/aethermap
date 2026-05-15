use crate::gui::{Message, State};
use crate::theme;
use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Space},
    Alignment, Element, Length,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSwitchRule {
    pub app_id: String,
    pub profile_name: String,
    pub device_id: Option<String>,
    pub layer_id: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct AutoSwitchRulesView {
    pub device_id: String,
    pub rules: Vec<AutoSwitchRule>,
    pub editing_rule: Option<usize>,
    pub new_app_id: String,
    pub new_profile_name: String,
    pub new_layer_id: String,
}

pub fn view(state: &State) -> Element<'_, Message> {
    let view = state.auto_switch_view.as_ref().unwrap();

    let focus_display = row![
        text("Current Focus:").size(14),
        Space::with_width(8),
        if let Some(ref focus) = state.current_focus {
            container(text(focus).size(14))
                .padding([4, 12])
                .style(theme::styles::card)
        } else {
            container(text("Unknown").size(14).style(iced::theme::Text::Color(
                iced::Color::from_rgb(0.6, 0.6, 0.6),
            )))
            .padding([4, 12])
        },
    ]
    .spacing(4)
    .align_items(Alignment::Center);

    let rules_header = row![
        text("Auto-Switch Rules").size(18),
        Space::with_width(Length::Fill),
        if view.editing_rule.is_some() {
            button("Cancel")
                .on_press(Message::EditAutoSwitchRule(usize::MAX))
                .style(iced::theme::Button::Text)
        } else {
            button("Add Rule")
                .on_press(Message::EditAutoSwitchRule(usize::MAX))
                .style(iced::theme::Button::Primary)
        },
    ]
    .align_items(Alignment::Center);

    let rules_list = if view.rules.is_empty() {
        column![
            Space::with_height(20),
            text("No rules configured")
                .size(14)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.6, 0.6, 0.6
                ))),
            Space::with_height(8),
            text("Add a rule to automatically switch profiles when windows gain focus")
                .size(12)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.5, 0.5, 0.5
                ))),
        ]
        .align_items(Alignment::Center)
    } else {
        let mut list = column![].spacing(8);
        for (idx, rule) in view.rules.iter().enumerate() {
            let is_editing = view.editing_rule == Some(idx);
            let indicator: Element<'_, Message> = if is_editing {
                container(text("\u{25b6}")).padding([0, 8]).into()
            } else {
                Space::with_width(20).into()
            };
            let rule_row = row![
                indicator,
                column![
                    text(format!("App: {}", rule.app_id)).size(14),
                    text(format!(
                        "Profile: {}{}",
                        rule.profile_name,
                        rule.layer_id
                            .map(|l| format!(" + Layer {}", l))
                            .unwrap_or_default()
                    ))
                    .size(12),
                ]
                .spacing(2),
                Space::with_width(Length::Fill),
                button("Edit")
                    .on_press(Message::EditAutoSwitchRule(idx))
                    .style(iced::theme::Button::Text),
                button("Delete")
                    .on_press(Message::DeleteAutoSwitchRule(idx))
                    .style(iced::theme::Button::Destructive),
            ]
            .spacing(8)
            .align_items(Alignment::Center);
            list = list.push(rule_row);
        }
        list
    };

    let edit_form = if view.editing_rule.is_some() {
        Some(
            column![
                Space::with_height(20),
                text(if view.editing_rule.unwrap_or(0) < view.rules.len() {
                    "Edit Rule"
                } else {
                    "Add New Rule"
                })
                .size(16),
                Space::with_height(12),
                row![
                    text("App ID:").size(14),
                    Space::with_width(8),
                    text_input("org.alacritty", &view.new_app_id)
                        .on_input(Message::AutoSwitchAppIdChanged)
                        .padding(8)
                        .size(14),
                    Space::with_width(8),
                    button("Use Current")
                        .on_press(Message::AutoSwitchUseCurrentApp)
                        .style(iced::theme::Button::Secondary),
                ]
                .spacing(4)
                .align_items(Alignment::Center),
                Space::with_height(8),
                row![
                    text("Profile:").size(14),
                    Space::with_width(8),
                    text_input("default", &view.new_profile_name)
                        .on_input(Message::AutoSwitchProfileNameChanged)
                        .padding(8)
                        .size(14),
                ]
                .spacing(4)
                .align_items(Alignment::Center),
                Space::with_height(8),
                row![
                    text("Layer (optional):").size(14),
                    Space::with_width(8),
                    text_input("0", &view.new_layer_id)
                        .on_input(Message::AutoSwitchLayerIdChanged)
                        .padding(8)
                        .size(14),
                ]
                .spacing(4)
                .align_items(Alignment::Center),
                Space::with_height(12),
                row![
                    Space::with_width(Length::Fill),
                    button("Save Rule")
                        .on_press(Message::SaveAutoSwitchRule)
                        .style(iced::theme::Button::Primary),
                ]
                .align_items(Alignment::Center),
            ]
            .spacing(4),
        )
    } else {
        None
    };

    let mut content = column![
        focus_display,
        Space::with_height(20),
        rules_header,
        Space::with_height(12),
        scrollable(rules_list).height(Length::Fixed(200.0)),
    ]
    .spacing(4);

    if let Some(form) = edit_form {
        content = content.push(form);
    }

    container(content)
        .padding(20)
        .width(Length::Fill)
        .style(theme::styles::card)
        .into()
}
