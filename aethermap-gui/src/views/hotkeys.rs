use crate::gui::{Message, State};
use crate::theme;
use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Space},
    Alignment, Element, Length,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyBinding {
    pub modifiers: Vec<String>,
    pub key: String,
    pub profile_name: String,
    pub device_id: Option<String>,
    pub layer_id: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct HotkeyBindingsView {
    pub device_id: String,
    pub bindings: Vec<HotkeyBinding>,
    pub editing_binding: Option<usize>,
    pub new_modifiers: Vec<String>,
    pub new_key: String,
    pub new_profile_name: String,
    pub new_layer_id: String,
}

fn modifier_checkbox<'a>(
    label: &str,
    modifier: &str,
    selected: &'a [String],
) -> Element<'a, Message> {
    let is_checked = selected.iter().any(|m| m.to_lowercase() == modifier);
    let btn = if is_checked {
        button(text(format!("[{}] ", label)).size(12))
    } else {
        button(text(format!("[ ] {}", label)).size(12))
    };
    btn.on_press(Message::ToggleHotkeyModifier(modifier.to_string()))
        .style(iced::theme::Button::Text)
        .into()
}

pub fn view(state: &State) -> Element<'_, Message> {
    let view = state.hotkey_view.as_ref().unwrap();

    let bindings_header = row![
        text("Hotkey Bindings").size(18),
        Space::with_width(Length::Fill),
        if view.editing_binding.is_some() {
            button("Cancel")
                .on_press(Message::EditHotkeyBinding(usize::MAX))
                .style(iced::theme::Button::Text)
        } else {
            button("Add Binding")
                .on_press(Message::EditHotkeyBinding(usize::MAX))
                .style(iced::theme::Button::Primary)
        },
    ]
    .align_items(Alignment::Center);

    let bindings_list = if view.bindings.is_empty() {
        column![
            Space::with_height(20),
            text("No bindings configured")
                .size(14)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.6, 0.6, 0.6
                ))),
            Space::with_height(8),
            text("Add a binding to switch profiles using keyboard shortcuts")
                .size(12)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.5, 0.5, 0.5
                ))),
        ]
        .align_items(Alignment::Center)
    } else {
        let mut list = column![].spacing(8);
        for (idx, binding) in view.bindings.iter().enumerate() {
            let is_editing = view.editing_binding == Some(idx);
            let modifiers_str = binding.modifiers.join("+");
            let indicator: Element<'_, Message> = if is_editing {
                container(text("\u{25b6}")).padding([0, 8]).into()
            } else {
                Space::with_width(20).into()
            };
            let rule_row = row![
                indicator,
                column![
                    text(format!(
                        "{}+{} \u{2192} {}",
                        modifiers_str, binding.key, binding.profile_name
                    ))
                    .size(14),
                    text(format!(
                        "Layer: {}",
                        binding
                            .layer_id
                            .map(|l| l.to_string())
                            .unwrap_or_else(|| "default".to_string())
                    ))
                    .size(12),
                ]
                .spacing(2),
                Space::with_width(Length::Fill),
                button("Edit")
                    .on_press(Message::EditHotkeyBinding(idx))
                    .style(iced::theme::Button::Text),
                button("Delete")
                    .on_press(Message::DeleteHotkeyBinding(idx))
                    .style(iced::theme::Button::Destructive),
            ]
            .spacing(8)
            .align_items(Alignment::Center);
            list = list.push(rule_row);
        }
        list
    };

    let edit_form = if view.editing_binding.is_some() {
        Some(
            column![
                Space::with_height(20),
                text(if view.editing_binding.unwrap_or(0) < view.bindings.len() {
                    "Edit Binding"
                } else {
                    "Add New Binding"
                })
                .size(16),
                Space::with_height(12),
                text("Modifiers:").size(14),
                row![
                    modifier_checkbox("Ctrl", "ctrl", &view.new_modifiers),
                    modifier_checkbox("Alt", "alt", &view.new_modifiers),
                    modifier_checkbox("Shift", "shift", &view.new_modifiers),
                    modifier_checkbox("Super", "super", &view.new_modifiers),
                ]
                .spacing(8),
                Space::with_height(8),
                row![
                    text("Key:").size(14),
                    Space::with_width(8),
                    text_input("1", &view.new_key)
                        .on_input(Message::HotkeyKeyChanged)
                        .padding(8)
                        .size(14),
                ]
                .spacing(4)
                .align_items(Alignment::Center),
                Space::with_height(8),
                row![
                    text("Profile:").size(14),
                    Space::with_width(8),
                    text_input("default", &view.new_profile_name)
                        .on_input(Message::HotkeyProfileNameChanged)
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
                        .on_input(Message::HotkeyLayerIdChanged)
                        .padding(8)
                        .size(14),
                ]
                .spacing(4)
                .align_items(Alignment::Center),
                Space::with_height(12),
                row![
                    Space::with_width(Length::Fill),
                    button("Save Binding")
                        .on_press(Message::SaveHotkeyBinding)
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
        bindings_header,
        Space::with_height(12),
        scrollable(bindings_list).height(Length::Fixed(200.0)),
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
