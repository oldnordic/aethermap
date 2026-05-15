use crate::gui::{Message, State};
use crate::theme;
use aethermap_common::{DeviceInfo, DeviceType, LayerMode};
use iced::{
    widget::{button, column, container, pick_list, row, text, text_input, Column, Space},
    Alignment, Element, Length,
};

pub fn view_devices_tab(state: &State) -> Element<'_, Message> {
    let header = row![
        text("DEVICES").size(24),
        Space::with_width(Length::Fill),
        button("Reload")
            .on_press(Message::LoadDevices)
            .style(iced::theme::Button::Secondary),
    ]
    .align_items(Alignment::Center);

    // Show auto-switch rules view when open
    if let Some(ref view) = state.auto_switch_view {
        return column![
            header,
            Space::with_height(20),
            row![
                button("← Back to Devices")
                    .on_press(Message::CloseAutoSwitchRules)
                    .style(iced::theme::Button::Text),
                Space::with_width(Length::Fill),
                text(format!("Auto-Switch Rules: {}", view.device_id)).size(18),
            ]
            .align_items(Alignment::Center),
            Space::with_height(20),
            crate::views::auto_switch::view(state),
        ]
        .spacing(10)
        .into();
    }

    // Show hotkey bindings view when open
    if let Some(ref view) = state.hotkey_view {
        return column![
            header,
            Space::with_height(20),
            row![
                button("← Back to Devices")
                    .on_press(Message::CloseHotkeyBindings)
                    .style(iced::theme::Button::Text),
                Space::with_width(Length::Fill),
                text(format!("Hotkey Bindings: {}", view.device_id)).size(18),
            ]
            .align_items(Alignment::Center),
            Space::with_height(20),
            crate::views::hotkeys::view(state),
        ]
        .spacing(10)
        .into();
    }

    // Show keypad view when capabilities are loaded
    if state.device_capabilities.is_some() && !state.keypad_layout.is_empty() {
        let mut keypad_content = vec![
            header.into(),
            Space::with_height(20).into(),
            row![
                button("← Back to Devices")
                    .on_press(Message::ShowKeypadView("".to_string()))
                    .style(iced::theme::Button::Text),
                Space::with_width(Length::Fill),
            ]
            .align_items(Alignment::Center)
            .into(),
            Space::with_height(20).into(),
            crate::views::keypad::view(state),
        ];

        if let Some(ref device_path) = state.keypad_view_device {
            keypad_content.push(Space::with_height(20).into());
            keypad_content.push(
                container(
                    column![
                        text("Quick Profile Switch").size(14),
                        Space::with_height(8),
                        profile_quick_toggles(state, device_path),
                    ]
                    .spacing(4),
                )
                .padding(16)
                .width(Length::Fill)
                .style(theme::styles::card)
                .into(),
            );
        }

        return column(keypad_content).spacing(10).into();
    }

    let device_list = if state.devices.is_empty() {
        column![
            Space::with_height(40),
            text("No devices found").size(16),
            Space::with_height(10),
            text("Click 'Reload' to scan for input devices").size(12),
        ]
        .align_items(Alignment::Center)
        .width(Length::Fill)
    } else {
        let mut list: Column<Message> = column![].spacing(12);
        for (idx, device) in state.devices.iter().enumerate() {
            list = list.push(view_device_card(state, device, idx));
        }
        list
    };

    column![header, Space::with_height(20), device_list,]
        .spacing(10)
        .into()
}

pub fn view_device_card<'a>(
    state: &'a State,
    device: &'a DeviceInfo,
    idx: usize,
) -> Element<'a, Message> {
    let device_path = device.path.to_string_lossy().to_string();
    let is_grabbed = state.grabbed_devices.contains(&device_path);
    let is_selected = state.selected_device == Some(idx);

    let icon = match device.device_type {
        DeviceType::Keyboard => "⌨️",
        DeviceType::Mouse => "🖱️",
        DeviceType::Gamepad => "🎮",
        DeviceType::Keypad => "🎹",
        DeviceType::Other => "📱",
    };

    let status_badge = if is_grabbed {
        container(text("GRABBED").size(10))
            .padding([4, 8])
            .style(theme::styles::card)
    } else {
        container(text("").size(10))
    };

    let action_button = if is_grabbed {
        button("Release")
            .on_press(Message::UngrabDevice(device_path.clone()))
            .style(iced::theme::Button::Destructive)
    } else {
        button("Grab Device")
            .on_press(Message::GrabDevice(device_path.clone()))
            .style(iced::theme::Button::Primary)
    };

    let select_indicator = if is_selected { "▶ " } else { "" };
    let device_id = format!("{:04x}:{:04x}", device.vendor_id, device.product_id);

    let keypad_button = if device.device_type == DeviceType::Keypad {
        Some(
            button("Configure Keypad")
                .on_press(Message::ShowKeypadView(device_path.clone()))
                .style(iced::theme::Button::Secondary),
        )
    } else {
        None
    };

    let led_button =
        if device.device_type == DeviceType::Keypad || device.device_type == DeviceType::Gamepad {
            Some(
                button("Configure LEDs")
                    .on_press(Message::OpenLedConfig(device_id.clone()))
                    .style(iced::theme::Button::Secondary),
            )
        } else {
            None
        };

    let auto_switch_button = Some(
        button("Auto-Switch Rules")
            .on_press(Message::ShowAutoSwitchRules(device_id.clone()))
            .style(iced::theme::Button::Secondary),
    );

    let hotkey_button = Some(
        button("Hotkey Bindings")
            .on_press(Message::ShowHotkeyBindings(device_id.clone()))
            .style(iced::theme::Button::Secondary),
    );

    let analog_button =
        if device.device_type == DeviceType::Keypad || device.device_type == DeviceType::Gamepad {
            Some(
                button("Analog Calibration")
                    .on_press(Message::OpenAnalogCalibration {
                        device_id: device_id.clone(),
                        layer_id: state.active_layers.get(&device_id).copied().unwrap_or(0),
                    })
                    .style(iced::theme::Button::Secondary),
            )
        } else {
            None
        };

    let card_content =
        column![
            row![
                text(icon).size(28),
                Space::with_width(12),
                column![
                    row![
                        text(format!("{}{}", select_indicator, device.name)).size(16),
                        Space::with_width(8),
                        text(match device.device_type {
                            DeviceType::Keyboard => "Keyboard",
                            DeviceType::Mouse => "Mouse",
                            DeviceType::Gamepad => "Gamepad",
                            DeviceType::Keypad => "Keypad",
                            DeviceType::Other => "Other",
                        })
                        .size(12)
                        .style(iced::theme::Text::Color(
                            iced::Color::from_rgb(0.6, 0.6, 0.6)
                        )),
                    ],
                    text(format!(
                        "VID:{:04X} PID:{:04X} | {}",
                        device.vendor_id, device.product_id, device_path
                    ))
                    .size(11),
                ],
                Space::with_width(Length::Fill),
                status_badge,
            ]
            .align_items(Alignment::Center),
            Space::with_height(12),
            row![
                button("Select")
                    .on_press(Message::SelectDevice(idx))
                    .style(iced::theme::Button::Text),
                Space::with_width(Length::Fill),
                action_button,
            ],
            Space::with_height(8),
            crate::views::profiles::view_profile_selector(state, device),
            crate::views::profiles::view_remap_profile_switcher(state, &device_path),
            Space::with_height(4),
            container(
                column![
                    text("Profiles").size(11).style(iced::theme::Text::Color(
                        iced::Color::from_rgb(0.5, 0.5, 0.5)
                    )),
                    Space::with_height(4),
                    profile_quick_toggles(state, &device_path),
                ]
                .spacing(4)
            )
            .padding([8, 0])
            .width(Length::Fill),
            Space::with_height(8),
            row![
                text("Layer:").size(12),
                Space::with_width(8),
                layer_indicator(state, &device_id),
                Space::with_width(Length::Fill),
                layer_activation_buttons(state, &device_id),
            ]
            .spacing(4)
            .align_items(Alignment::Center),
        ]
        .spacing(8);

    let mut card_elements: Vec<Element<'_, Message>> = vec![card_content.into()];

    // D-pad mode selector for devices with analog sticks
    if device.device_type == DeviceType::Gamepad || device.device_type == DeviceType::Keypad {
        let current_mode = state
            .analog_dpad_modes
            .get(&device_id)
            .cloned()
            .unwrap_or_else(|| "disabled".to_string());

        card_elements.push(Space::with_height(4).into());
        card_elements.push(
            row![
                text("D-pad:").size(12),
                Space::with_width(4),
                button("Off")
                    .on_press(Message::SetAnalogDpadMode(
                        device_id.clone(),
                        "disabled".to_string()
                    ))
                    .style(if current_mode == "disabled" {
                        iced::theme::Button::Primary
                    } else {
                        iced::theme::Button::Text
                    }),
                button("8-Way")
                    .on_press(Message::SetAnalogDpadMode(
                        device_id.clone(),
                        "eight_way".to_string()
                    ))
                    .style(if current_mode == "eight_way" {
                        iced::theme::Button::Primary
                    } else {
                        iced::theme::Button::Text
                    }),
                button("4-Way")
                    .on_press(Message::SetAnalogDpadMode(
                        device_id.clone(),
                        "four_way".to_string()
                    ))
                    .style(if current_mode == "four_way" {
                        iced::theme::Button::Primary
                    } else {
                        iced::theme::Button::Text
                    }),
            ]
            .spacing(4)
            .align_items(Alignment::Center)
            .into(),
        );

        let (deadzone_x, deadzone_y) = state
            .analog_deadzones_xy
            .get(&device_id)
            .cloned()
            .unwrap_or((43, 43));
        let (outer_deadzone_x, outer_deadzone_y) = state
            .analog_outer_deadzones_xy
            .get(&device_id)
            .cloned()
            .unwrap_or((100, 100));

        card_elements.push(Space::with_height(8).into());

        card_elements.push(
            column![
                text("Deadzone (noise filter)").size(11),
                row![
                    text("X:").size(11),
                    Space::with_width(4),
                    deadzone_buttons(&device_id, false, deadzone_x),
                    Space::with_width(8),
                    text(format!("{}%", deadzone_x)).size(11),
                ]
                .spacing(2)
                .align_items(Alignment::Center),
                row![
                    text("Y:").size(11),
                    Space::with_width(4),
                    deadzone_buttons(&device_id, true, deadzone_y),
                    Space::with_width(8),
                    text(format!("{}%", deadzone_y)).size(11),
                ]
                .spacing(2)
                .align_items(Alignment::Center),
            ]
            .spacing(4)
            .into(),
        );

        card_elements.push(Space::with_height(4).into());
        card_elements.push(
            column![
                text("Max Range (input clamp)").size(11),
                row![
                    text("X:").size(11),
                    Space::with_width(4),
                    outer_deadzone_buttons(&device_id, false, outer_deadzone_x),
                    Space::with_width(8),
                    text(format!("{}%", outer_deadzone_x)).size(11),
                ]
                .spacing(2)
                .align_items(Alignment::Center),
                row![
                    text("Y:").size(11),
                    Space::with_width(4),
                    outer_deadzone_buttons(&device_id, true, outer_deadzone_y),
                    Space::with_width(8),
                    text(format!("{}%", outer_deadzone_y)).size(11),
                ]
                .spacing(2)
                .align_items(Alignment::Center),
            ]
            .spacing(4)
            .into(),
        );
    }

    if let Some(keypad_btn) = keypad_button {
        card_elements.push(Space::with_height(4).into());
        card_elements.push(
            row![Space::with_width(Length::Fill), keypad_btn,]
                .spacing(4)
                .into(),
        );
    }

    if let Some(led_btn) = led_button {
        card_elements.push(Space::with_height(4).into());
        card_elements.push(
            row![Space::with_width(Length::Fill), led_btn,]
                .spacing(4)
                .into(),
        );
    }

    if let Some(auto_btn) = auto_switch_button {
        card_elements.push(Space::with_height(4).into());
        card_elements.push(
            row![Space::with_width(Length::Fill), auto_btn,]
                .spacing(4)
                .into(),
        );
    }

    if let Some(hotkey_btn) = hotkey_button {
        card_elements.push(Space::with_height(4).into());
        card_elements.push(
            row![Space::with_width(Length::Fill), hotkey_btn,]
                .spacing(4)
                .into(),
        );
    }

    if let Some(analog_btn) = analog_button {
        card_elements.push(Space::with_height(4).into());
        card_elements.push(
            row![Space::with_width(Length::Fill), analog_btn,]
                .spacing(4)
                .into(),
        );
    }

    let card_content = column(card_elements).spacing(4);

    container(card_content)
        .padding(16)
        .width(Length::Fill)
        .style(theme::styles::card)
        .into()
}

fn layer_indicator<'a>(state: &'a State, device_id: &str) -> Element<'a, Message> {
    if let Some(&layer_id) = state.active_layers.get(device_id) {
        let layer_name = state
            .layer_configs
            .get(device_id)
            .and_then(|layers| layers.iter().find(|l| l.layer_id == layer_id))
            .map(|l| l.name.as_str())
            .unwrap_or("Unknown");

        container(text(format!("Layer {}: {}", layer_id, layer_name)).size(12))
            .padding([4, 8])
            .style(theme::styles::card)
            .into()
    } else {
        container(text("Layer 0: Base").size(12))
            .padding([4, 8])
            .style(theme::styles::card)
            .into()
    }
}

fn profile_quick_toggles<'a>(state: &'a State, device_path: &str) -> Element<'a, Message> {
    let profiles = state.remap_profiles.get(device_path);
    let active_profile = state.active_remap_profiles.get(device_path);

    if let Some(profile_list) = profiles {
        if profile_list.is_empty() {
            return row![].into();
        }

        let buttons: Vec<Element<'_, Message>> = profile_list
            .iter()
            .map(|profile| {
                let is_active =
                    active_profile.as_ref().map(|s| s.as_str()) == Some(profile.name.as_str());
                let button_style = if is_active {
                    iced::theme::Button::Primary
                } else {
                    iced::theme::Button::Secondary
                };

                button(text(&profile.name).size(11))
                    .on_press(Message::ActivateRemapProfile(
                        device_path.to_string(),
                        profile.name.clone(),
                    ))
                    .style(button_style)
                    .padding([6, 10])
                    .into()
            })
            .collect();

        let mut final_buttons = buttons;
        if active_profile.is_some() {
            final_buttons.push(
                button(text("Off").size(11))
                    .on_press(Message::DeactivateRemapProfile(device_path.to_string()))
                    .style(iced::theme::Button::Text)
                    .padding([6, 10])
                    .into(),
            );
        }

        row(final_buttons).spacing(6).into()
    } else {
        row![].into()
    }
}

fn layer_activation_buttons<'a>(state: &'a State, device_id: &str) -> Element<'a, Message> {
    let layers = state.layer_configs.get(device_id);

    if let Some(layer_list) = layers {
        let toggle_layers: Vec<_> = layer_list
            .iter()
            .filter(|l| l.mode == LayerMode::Toggle && l.layer_id > 0)
            .collect();

        if toggle_layers.is_empty() {
            return text("No toggle layers configured").size(11).into();
        }

        let active_layer_id = state.active_layers.get(device_id).copied().unwrap_or(0);

        let buttons: Vec<Element<'_, Message>> = toggle_layers
            .iter()
            .map(|layer| {
                let is_active = active_layer_id == layer.layer_id;
                let button_style = if is_active {
                    iced::theme::Button::Secondary
                } else {
                    iced::theme::Button::Text
                };

                button(text(format!("L{}", layer.layer_id)).size(11))
                    .on_press(Message::LayerActivateRequested(
                        device_id.to_string(),
                        layer.layer_id,
                        LayerMode::Toggle,
                    ))
                    .style(button_style)
                    .padding([4, 8])
                    .into()
            })
            .collect();

        row(buttons).spacing(4).into()
    } else {
        text("Load layers to see toggle buttons").size(11).into()
    }
}

fn deadzone_buttons(device_id: &str, is_y_axis: bool, current: u8) -> Element<'static, Message> {
    let percentages = [0, 10, 20, 30, 40, 50];
    let buttons: Vec<Element<'_, Message>> = percentages
        .iter()
        .map(|&pct| {
            let is_current = current == pct;
            button(text(format!("{}%", pct)).size(10))
                .on_press(if is_y_axis {
                    Message::SetAnalogDeadzoneXY(device_id.to_string(), current, pct)
                } else {
                    Message::SetAnalogDeadzoneXY(device_id.to_string(), pct, current)
                })
                .style(if is_current {
                    iced::theme::Button::Primary
                } else {
                    iced::theme::Button::Text
                })
                .padding([2, 6])
                .into()
        })
        .collect();

    row(buttons).spacing(2).into()
}

fn outer_deadzone_buttons(
    device_id: &str,
    is_y_axis: bool,
    current: u8,
) -> Element<'static, Message> {
    let percentages = [80, 85, 90, 95, 100];
    let buttons: Vec<Element<'_, Message>> = percentages
        .iter()
        .map(|&pct| {
            let is_current = current == pct;
            button(text(format!("{}%", pct)).size(10))
                .on_press(if is_y_axis {
                    Message::SetAnalogOuterDeadzoneXY(device_id.to_string(), current, pct)
                } else {
                    Message::SetAnalogOuterDeadzoneXY(device_id.to_string(), pct, current)
                })
                .style(if is_current {
                    iced::theme::Button::Primary
                } else {
                    iced::theme::Button::Text
                })
                .padding([2, 6])
                .into()
        })
        .collect();

    row(buttons).spacing(2).into()
}

pub fn layer_settings_view<'a>(state: &'a State, device_id: &str) -> Element<'a, Message> {
    let layers = state.layer_configs.get(device_id);

    if let Some(layer_list) = layers {
        if layer_list.is_empty() {
            return column![
                text("No layers configured").size(14),
                text("Default base layer will be created automatically").size(11),
            ]
            .spacing(4)
            .into();
        }

        let mut rows: Vec<Element<'_, Message>> = layer_list
            .iter()
            .map(|layer| {
                let mode_text = match layer.mode {
                    LayerMode::Hold => "Hold",
                    LayerMode::Toggle => "Toggle",
                };

                row![
                    text(format!("L{}", layer.layer_id))
                        .size(12)
                        .width(Length::Fixed(30.0)),
                    text(&layer.name).size(12).width(Length::Fixed(100.0)),
                    text(mode_text).size(12).width(Length::Fixed(60.0)),
                    text(format!("{} remaps", layer.remap_count)).size(11),
                    Space::with_width(Length::Fill),
                    button(text("Edit").size(11))
                        .on_press(Message::OpenLayerConfigDialog(
                            device_id.to_string(),
                            layer.layer_id
                        ))
                        .style(iced::theme::Button::Text)
                        .padding([4, 8]),
                ]
                .spacing(8)
                .align_items(Alignment::Center)
                .into()
            })
            .collect();

        let add_button = if layer_list.len() < 8 {
            Some(
                button(row![text("+").size(14), text("Add Layer").size(12),].spacing(4))
                    .on_press(Message::OpenLayerConfigDialog(
                        device_id.to_string(),
                        layer_list.len(),
                    ))
                    .style(iced::theme::Button::Secondary)
                    .padding([6, 12])
                    .into(),
            )
        } else {
            None
        };

        if let Some(btn) = add_button {
            rows.push(btn);
        }

        column(rows).spacing(8).into()
    } else {
        column![
            text("Load layers to see settings").size(12),
            button("Load Layers")
                .on_press(Message::LayerConfigRequested(device_id.to_string()))
                .style(iced::theme::Button::Secondary),
        ]
        .spacing(8)
        .into()
    }
}

pub fn layer_config_dialog(state: &State) -> Option<Element<'_, Message>> {
    if let Some((_device_id, layer_id, name, mode)) = &state.layer_config_dialog {
        let mode_options = vec!["Hold".to_string(), "Toggle".to_string()];
        let current_mode_str = match mode {
            LayerMode::Hold => "Hold",
            LayerMode::Toggle => "Toggle",
        };

        let dialog = container(
            column![
                text(format!("Configure Layer {}", layer_id)).size(18),
                Space::with_height(20),
                text("Layer Name:").size(12),
                text_input("Enter layer name...", name)
                    .on_input(Message::LayerConfigNameChanged)
                    .padding(8)
                    .size(14)
                    .width(Length::Fixed(250.0)),
                Space::with_height(12),
                text("Activation Mode:").size(12),
                pick_list(
                    mode_options,
                    Some(current_mode_str.to_string()),
                    |selected| {
                        let new_mode = match selected.as_str() {
                            "Toggle" => LayerMode::Toggle,
                            _ => LayerMode::Hold,
                        };
                        Message::LayerConfigModeChanged(new_mode)
                    }
                )
                .width(Length::Fixed(250.0))
                .padding(8),
                Space::with_height(20),
                row![
                    button("Cancel")
                        .on_press(Message::CancelLayerConfig)
                        .style(iced::theme::Button::Text)
                        .padding([8, 16]),
                    Space::with_width(Length::Fill),
                    button("Save")
                        .on_press(Message::SaveLayerConfig)
                        .style(iced::theme::Button::Primary)
                        .padding([8, 16]),
                ]
                .spacing(8),
            ]
            .spacing(4),
        )
        .padding(24)
        .width(Length::Fixed(300.0))
        .style(theme::styles::card);

        Some(
            container(container(dialog).width(Length::Fill).center_x().center_y())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(iced::theme::Container::Transparent)
                .into(),
        )
    } else {
        None
    }
}
