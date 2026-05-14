use iced::{
    widget::{button, column, container, pick_list, row, scrollable, text, text_input, Space},
    Element, Length, Alignment,
};
use aethermap_common::{DeviceInfo, RemapProfileInfo};
use crate::gui::{State, Message};
use crate::theme;

pub fn view_profiles_tab(state: &State) -> Element<'_, Message> {
    let header = text("PROFILES").size(24);

    let profile_input = text_input("Profile name...", &state.profile_name)
        .on_input(Message::UpdateProfileName)
        .padding(12)
        .size(14);

    let save_button = button(
        row![
            text("💾").size(16),
            Space::with_width(8),
            text("Save Profile").size(14),
        ]
        .align_items(Alignment::Center)
    )
    .on_press(Message::SaveProfile)
    .style(iced::theme::Button::Primary)
    .padding([12, 20]);

    let load_button = button(
        row![
            text("📂").size(16),
            Space::with_width(8),
            text("Load Profile").size(14),
        ]
        .align_items(Alignment::Center)
    )
    .on_press(Message::LoadProfile)
    .style(iced::theme::Button::Secondary)
    .padding([12, 20]);

    let profile_info = column![
        text("Current Configuration").size(16),
        Space::with_height(10),
        text(format!("• {} devices detected", state.devices.len())).size(12),
        text(format!("• {} devices grabbed", state.grabbed_devices.len())).size(12),
        text(format!("• {} macros configured", state.macros.len())).size(12),
    ]
    .spacing(4);

    let panel_content = column![
        text("SAVE / LOAD CONFIGURATION").size(16),
        Space::with_height(16),
        profile_input,
        Space::with_height(16),
        row![
            save_button,
            Space::with_width(10),
            load_button,
        ],
        Space::with_height(20),
        profile_info,
    ];

    column![
        header,
        Space::with_height(20),
        container(panel_content)
            .padding(20)
            .width(Length::Fill)
            .style(theme::styles::card),
    ]
    .spacing(10)
    .into()
}

pub fn view_profile_selector<'a>(state: &'a State, device: &'a DeviceInfo) -> Element<'a, Message> {
    let device_id = format!("{:04x}:{:04x}", device.vendor_id, device.product_id);
    let profiles = state.device_profiles.get(&device_id);
    let active_profile = state.active_profiles.get(&device_id);

    let profile_row: Element<'_, Message> = if let Some(profiles) = profiles {
        if profiles.is_empty() {
            row![
                text("Profile: ").size(12),
                text("No profiles configured").size(12),
            ]
            .spacing(10)
            .align_items(Alignment::Center)
            .into()
        } else {
            let device_id_for_closure = device_id.clone();
            let picker = pick_list(
                profiles.clone(),
                active_profile.cloned(),
                move |profile_name| Message::ActivateProfile(device_id_for_closure.clone(), profile_name),
            )
            .placeholder("Select profile")
            .width(Length::Fixed(150.0));

            let mut row_content = row![
                text("Profile: ").size(12),
                picker,
            ]
            .spacing(10)
            .align_items(Alignment::Center);

            if let Some(_active) = active_profile {
                row_content = row_content.push(
                    button(text("Deactivate").size(11))
                        .on_press(Message::DeactivateProfile(device_id.clone()))
                        .padding(5)
                        .style(iced::theme::Button::Text)
                );
            }

            row_content.into()
        }
    } else {
        row![
            text("Profile: ").size(12),
            button(text("Load Profiles").size(11))
                .on_press(Message::LoadDeviceProfiles(device_id.clone()))
                .padding([4, 8])
                .style(iced::theme::Button::Text),
        ]
        .spacing(10)
        .align_items(Alignment::Center)
        .into()
    };

    container(profile_row)
        .padding([4, 0])
        .into()
}

pub fn view_remap_profile_switcher<'a>(state: &'a State, device_path: &str) -> Element<'a, Message> {
    let profiles = state.remap_profiles.get(device_path);
    let active_profile = state.active_remap_profiles.get(device_path);

    let profile_row: Element<'_, Message> = if let Some(profiles) = profiles {
        if profiles.is_empty() {
            row![
                text("Remap: ").size(12),
                text("No remap profiles").size(12),
            ]
            .spacing(10)
            .align_items(Alignment::Center)
            .into()
        } else {
            let profile_names: Vec<String> = profiles.iter().map(|p: &RemapProfileInfo| p.name.clone()).collect();
            let device_path_for_closure = device_path.to_string();
            let picker = pick_list(
                profile_names,
                active_profile.cloned(),
                move |profile_name| Message::ActivateRemapProfile(device_path_for_closure.clone(), profile_name),
            )
            .placeholder("Select remap profile")
            .width(Length::Fixed(150.0));

            let mut row_content = row![
                text("Remap: ").size(12),
                picker,
            ]
            .spacing(10)
            .align_items(Alignment::Center);

            if let Some(_active) = active_profile {
                row_content = row_content.push(
                    button(text("Off").size(11))
                        .on_press(Message::DeactivateRemapProfile(device_path.to_string()))
                        .padding(5)
                        .style(iced::theme::Button::Text)
                );
            }

            row_content = row_content.push(
                button(text("↻").size(11))
                    .on_press(Message::LoadRemapProfiles(device_path.to_string()))
                    .padding(5)
                    .style(iced::theme::Button::Text)
            );

            row_content.into()
        }
    } else {
        row![
            text("Remap: ").size(12),
            button(text("Load Remaps").size(11))
                .on_press(Message::LoadRemapProfiles(device_path.to_string()))
                .padding([4, 8])
                .style(iced::theme::Button::Text),
        ]
        .spacing(10)
        .align_items(Alignment::Center)
        .into()
    };

    let remap_content = column![
        profile_row,
        view_active_remaps_display(state, device_path),
    ]
    .spacing(4);

    container(remap_content)
        .padding([4, 0])
        .into()
}

fn view_active_remaps_display<'a>(state: &'a State, device_path: &str) -> Element<'a, Message> {
    if let Some((profile_name, remaps)) = state.active_remaps.get(device_path) {
        if remaps.is_empty() {
            return text(format!("Profile: {} (no remaps)", profile_name))
                .size(10)
                .into();
        }

        let remap_rows: Vec<Element<'_, Message>> = remaps.iter().map(|remap| {
            row![
                text(format!("{} → {}", remap.from_key, remap.to_key))
                    .size(10)
            ]
            .into()
        }).collect();

        let remap_list = scrollable(
            column(remap_rows).spacing(2)
        )
        .height(Length::Fixed(60.0));

        column![
            text(format!("Active: {} ({} remaps)", profile_name, remaps.len())).size(10),
            remap_list,
        ]
        .spacing(2)
        .into()
    } else {
        text("").size(10).into()
    }
}
