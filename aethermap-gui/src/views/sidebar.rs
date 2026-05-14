use iced::{
    widget::{button, column, container, horizontal_rule, row, text, Space},
    Element, Length, Alignment,
};
use crate::gui::{State, Message, Tab};

pub fn view(state: &State) -> Element<'_, Message> {
    let logo = column![
        text("\u{25e2}").size(40),
        text("AETHERMAP").size(16),
        text("v1.4.1").size(10),
    ]
    .spacing(2)
    .align_items(Alignment::Center)
    .width(Length::Fill);

    let nav_button = |label: &str, icon: &str, tab: Tab| {
        let is_active = state.active_tab == tab;
        let btn_style = if is_active {
            iced::theme::Button::Primary
        } else {
            iced::theme::Button::Text
        };

        button(
            row![
                text(icon).size(18),
                Space::with_width(10),
                text(label).size(14),
            ]
            .align_items(Alignment::Center)
        )
        .on_press(Message::SwitchTab(tab))
        .style(btn_style)
        .padding([12, 20])
        .width(Length::Fill)
    };

    let connection_status = if state.daemon_connected {
        row![
            text("\u{25cf}").size(12),
            Space::with_width(8),
            text("Connected").size(11),
        ]
    } else {
        row![
            text("\u{25cb}").size(12),
            Space::with_width(8),
            text("Disconnected").size(11),
        ]
    }
    .align_items(Alignment::Center);

    let sidebar_content = column![
        logo,
        Space::with_height(30),
        nav_button("Devices", "\u{1f3ae}", Tab::Devices),
        nav_button("Macros", "\u{26a1}", Tab::Macros),
        nav_button("Profiles", "\u{1f4c1}", Tab::Profiles),
        Space::with_height(Length::Fill),
        horizontal_rule(1),
        Space::with_height(10),
        connection_status,
        Space::with_height(5),
        button("Refresh")
            .on_press(Message::CheckDaemonConnection)
            .style(iced::theme::Button::Text)
            .width(Length::Fill),
    ]
    .spacing(4)
    .padding(16)
    .align_items(Alignment::Center);

    container(sidebar_content)
        .width(180)
        .height(Length::Fill)
        .into()
}
