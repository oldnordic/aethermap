use iced::{
    widget::{container, row, text, Space},
    Element, Length, Alignment,
};
use crate::gui::{State, Message};

pub fn view(state: &State) -> Element<'_, Message> {
    let connection_indicator = if state.daemon_connected {
        text("\u{25cf} Connected").size(12)
    } else {
        text("\u{25cb} Disconnected").size(12)
    };

    let latest_notification = if let Some(notif) = state.notifications.back() {
        text(&notif.message).size(12)
    } else {
        text("Ready").size(12)
    };

    container(
        row![
            connection_indicator,
            text(" | ").size(12),
            latest_notification,
            Space::with_width(Length::Fill),
            text(format!("{} macros", state.macros.len())).size(12),
        ]
        .spacing(5)
        .align_items(Alignment::Center)
    )
    .padding([8, 16])
    .width(Length::Fill)
    .into()
}
