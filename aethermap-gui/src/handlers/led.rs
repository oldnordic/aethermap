use iced::Command;
use aethermap_common::LedZone;
use crate::gui::{State, Message};

pub fn open(state: &mut State, device_id: String) -> Command<Message> {
    state.led_config_device = Some(device_id.clone());
    state.selected_led_zone = Some(LedZone::Logo);
    Command::batch([
        Command::none(),
        Command::perform(
            async move { device_id },
            |device_id| Message::RefreshLedState(device_id)
        ),
    ])
}

pub fn close(state: &mut State) -> Command<Message> {
    state.led_config_device = None;
    state.selected_led_zone = None;
    state.pending_led_color = None;
    Command::none()
}

pub fn select_zone(state: &mut State, zone: LedZone) -> Command<Message> {
    state.selected_led_zone = Some(zone);
    Command::none()
}

pub fn refresh(state: &State, device_id: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let device_id_clone = device_id.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.get_all_led_colors(&device_id_clone).await
        },
        move |result| match result {
            Ok(colors) => Message::LedStateLoaded(device_id, Ok(colors)),
            Err(e) => Message::LedStateLoaded(device_id, Err(e)),
        },
    )
}

pub fn state_loaded(state: &mut State, device_id: String, result: Result<std::collections::HashMap<LedZone, (u8, u8, u8)>, String>) -> Command<Message> {
    match result {
        Ok(colors) => {
            let led_state = state.led_states.entry(device_id).or_default();
            led_state.zone_colors = colors;
            Command::none()
        }
        Err(e) => {
            eprintln!("Failed to load LED state: {}", e);
            Command::none()
        }
    }
}

pub fn set_color(state: &State, device_id: String, zone: LedZone, red: u8, green: u8, blue: u8) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let device_id_clone = device_id.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.set_led_color(&device_id_clone, zone, red, green, blue).await
        },
        move |result| match result {
            Ok(_) => Message::LedColorSet(Ok(())),
            Err(e) => Message::LedColorSet(Err(e)),
        },
    )
}

pub fn color_set(state: &mut State, result: Result<(), String>) -> Command<Message> {
    if let Err(e) = result {
        eprintln!("Failed to set LED color: {}", e);
        state.add_notification(&format!("Failed to set LED color: {}", e), true);
    }
    Command::none()
}

pub fn set_brightness(state: &State, device_id: String, zone: Option<LedZone>, brightness: u8) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.set_led_brightness(&device_id, zone, brightness).await
        },
        |result| match result {
            Ok(_) => Message::LedBrightnessSet(Ok(())),
            Err(e) => Message::LedBrightnessSet(Err(e)),
        },
    )
}

pub fn brightness_set(state: &mut State, result: Result<(), String>) -> Command<Message> {
    if let Err(e) = result {
        eprintln!("Failed to set LED brightness: {}", e);
        state.add_notification(&format!("Failed to set LED brightness: {}", e), true);
    }
    Command::none()
}

pub fn set_pattern(state: &State, device_id: String, pattern: aethermap_common::LedPattern) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.set_led_pattern(&device_id, pattern).await
        },
        |result| match result {
            Ok(_) => Message::LedPatternSet(Ok(())),
            Err(e) => Message::LedPatternSet(Err(e)),
        },
    )
}

pub fn pattern_set(state: &mut State, result: Result<(), String>) -> Command<Message> {
    if let Err(e) = result {
        eprintln!("Failed to set LED pattern: {}", e);
        state.add_notification(&format!("Failed to set LED pattern: {}", e), true);
    }
    Command::none()
}

pub fn slider_changed(state: &mut State, red: u8, green: u8, blue: u8) -> Command<Message> {
    state.pending_led_color = Some((red, green, blue));
    if let (Some(ref device_id), Some(zone)) = (&state.led_config_device, state.selected_led_zone) {
        let device_id = device_id.clone();
        return Command::perform(
            async move { (device_id, zone, red, green, blue) },
            |(device_id, zone, red, green, blue)| {
                Message::SetLedColor(device_id, zone, red, green, blue)
            },
        );
    }
    Command::none()
}
