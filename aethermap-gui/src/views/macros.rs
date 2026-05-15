use crate::gui::{Message, State};
use crate::theme;
use aethermap_common::Action;
use iced::{
    widget::{
        button, checkbox, column, container, row, scrollable, slider, text, text_input, Column,
        Space,
    },
    Alignment, Element, Length,
};

fn format_action_with_icon(action: &Action) -> String {
    match action {
        Action::KeyPress(key) => format!("⌨️ Press Key {}", key),
        Action::KeyRelease(key) => format!("⌨️ Release Key {}", key),
        Action::Delay(ms) => format!("⏱️ Wait {}ms", ms),
        Action::MousePress(btn) => format!("🖱️ Click Button {}", btn),
        Action::MouseRelease(btn) => format!("🖱️ Release Button {}", btn),
        Action::MouseMove(x, y) => format!("↕️ Move X={} Y={}", x, y),
        Action::MouseScroll(amount) => format!("🔄 Scroll {}", amount),
        Action::Execute(cmd) => format!("▶️ Execute {}", cmd),
        Action::Type(text) => format!("⌨️ Type {}", text),
        Action::AnalogMove {
            axis_code,
            normalized,
        } => {
            let axis_name = match axis_code {
                61000 => "X",
                61001 => "Y",
                61002 => "Z",
                61003 => "RX",
                61004 => "RY",
                61005 => "RZ",
                _ => "UNKNOWN",
            };
            format!("🕹️ Analog({}, {:.2})", axis_name, normalized)
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let header = row![
        text("MACROS").size(24),
        Space::with_width(Length::Fill),
        text(format!("{} total", state.macros.len())).size(14),
    ]
    .align_items(Alignment::Center);

    let recording_section = view_recording_panel(state);
    let settings_section = view_macro_settings_panel(state);
    let macro_list = view_macro_list(state);

    column![
        header,
        Space::with_height(20),
        row![recording_section, settings_section,].spacing(20),
        Space::with_height(20),
        text("MACRO LIBRARY").size(18),
        Space::with_height(10),
        macro_list,
    ]
    .spacing(10)
    .into()
}

fn view_recording_panel(state: &State) -> Element<'_, Message> {
    let name_input = text_input(
        "Enter macro name (e.g., 'Quick Reload')",
        &state.new_macro_name,
    )
    .on_input(Message::UpdateMacroName)
    .padding(12)
    .size(14);

    let record_button = if state.recording {
        let indicator = if state.recording_pulse { "●" } else { "○" };
        button(
            row![
                text(indicator).size(18),
                Space::with_width(8),
                text("STOP RECORDING").size(14),
            ]
            .align_items(Alignment::Center),
        )
        .on_press(Message::StopRecording)
        .style(iced::theme::Button::Destructive)
        .padding([14, 24])
    } else {
        button(
            row![
                text("⏺").size(18),
                Space::with_width(8),
                text("START RECORDING").size(14),
            ]
            .align_items(Alignment::Center),
        )
        .on_press(Message::StartRecording)
        .style(iced::theme::Button::Primary)
        .padding([14, 24])
    };

    let instructions = column![
        text("Recording Instructions").size(14),
        Space::with_height(8),
        text("1. Go to Devices tab and grab a device").size(12),
        text("2. Enter a descriptive macro name above").size(12),
        text("3. Click 'Start Recording' and press keys").size(12),
        text("4. Click 'Stop Recording' when finished").size(12),
    ]
    .spacing(4);

    let recording_status = if state.recording {
        container(
            row![
                text("●").size(14),
                Space::with_width(8),
                text(format!(
                    "Recording '{}' - Press keys on grabbed device...",
                    state.recording_macro_name.as_deref().unwrap_or("")
                ))
                .size(13),
            ]
            .align_items(Alignment::Center),
        )
        .padding(12)
        .width(Length::Fill)
        .style(theme::styles::card)
    } else {
        container(text(""))
    };

    let panel_content = column![
        text("MACRO RECORDING").size(16),
        Space::with_height(16),
        name_input,
        Space::with_height(16),
        instructions,
        Space::with_height(16),
        recording_status,
        Space::with_height(16),
        container(record_button).center_x(),
    ];

    container(panel_content)
        .padding(20)
        .width(Length::Fill)
        .style(theme::styles::card)
        .into()
}

fn view_macro_settings_panel(state: &State) -> Element<'_, Message> {
    let latency_label = text(format!(
        "Latency Offset: {}ms",
        state.macro_settings.latency_offset_ms
    ))
    .size(14);
    let latency_slider = slider(
        0..=200,
        state.macro_settings.latency_offset_ms,
        Message::LatencyChanged,
    );

    let jitter_label = text(format!(
        "Jitter: {:.0}%",
        state.macro_settings.jitter_pct * 100.0
    ))
    .size(14);
    let jitter_slider = slider(
        0.0..=0.5,
        state.macro_settings.jitter_pct,
        Message::JitterChanged,
    )
    .step(0.01);

    let capture_mouse_checkbox = checkbox(
        "Capture Mouse (Macro playback moves mouse)",
        state.macro_settings.capture_mouse,
    )
    .on_toggle(Message::CaptureMouseToggled)
    .size(14);

    let content = column![
        text("GLOBAL MACRO SETTINGS").size(16),
        Space::with_height(16),
        latency_label,
        latency_slider,
        Space::with_height(12),
        jitter_label,
        jitter_slider,
        Space::with_height(16),
        capture_mouse_checkbox,
    ]
    .spacing(4);

    container(content)
        .padding(20)
        .width(Length::Fill)
        .style(theme::styles::card)
        .into()
}

fn view_macro_action(action: &Action) -> Element<'_, Message> {
    let action_text = format_action_with_icon(action);
    text(action_text).size(11).into()
}

fn view_macro_list(state: &State) -> Element<'_, Message> {
    if state.macros.is_empty() {
        return container(
            column![
                text("No macros yet").size(14),
                text("Record your first macro above").size(12),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .padding(20)
        .width(Length::Fill)
        .center_x()
        .into();
    }

    let mut list: Column<Message> = column![].spacing(8);

    for macro_entry in &state.macros {
        let is_recent = state
            .recently_updated_macros
            .contains_key(&macro_entry.name);
        let name_prefix = if is_recent { "★ " } else { "⚡ " };

        let action_preview: Vec<Element<'_, Message>> = macro_entry
            .actions
            .iter()
            .take(3)
            .map(|action| view_macro_action(action))
            .collect();

        let more_indicator = if macro_entry.actions.len() > 3 {
            Some(
                text(format!(
                    "+ {} more actions...",
                    macro_entry.actions.len() - 3
                ))
                .size(10),
            )
        } else {
            None
        };

        let macro_card = container(
            row![
                column![
                    text(format!("{}{}", name_prefix, macro_entry.name)).size(15),
                    text(format!(
                        "{} actions | {} trigger keys | {}",
                        macro_entry.actions.len(),
                        macro_entry.trigger.keys.len(),
                        if macro_entry.enabled {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    ))
                    .size(11),
                    column(action_preview).spacing(2).padding([4, 0]),
                    more_indicator.unwrap_or_else(|| text("").size(10)),
                ]
                .spacing(4),
                Space::with_width(Length::Fill),
                button("▶ Test")
                    .on_press(Message::PlayMacro(macro_entry.name.clone()))
                    .style(iced::theme::Button::Secondary),
                button("🗑")
                    .on_press(Message::DeleteMacro(macro_entry.name.clone()))
                    .style(iced::theme::Button::Destructive),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .padding(12)
        .width(Length::Fill)
        .style(theme::styles::card);

        list = list.push(macro_card);
    }

    scrollable(list).height(300).into()
}
