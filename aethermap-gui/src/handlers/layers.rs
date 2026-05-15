use crate::gui::{Message, State};
use aethermap_common::{LayerConfigInfo, LayerMode};
use iced::Command;

pub fn layer_state_changed(
    state: &mut State,
    device_id: String,
    layer_id: usize,
) -> Command<Message> {
    state.active_layers.insert(device_id, layer_id);
    Command::none()
}

pub fn layer_config_requested(state: &State, device_id: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let id = device_id.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            (id.clone(), client.list_layers(&id).await)
        },
        |(device_id, result)| match result {
            Ok(layers) => {
                if let Some(active_layer) = layers.first() {
                    Message::LayerStateChanged(device_id, active_layer.layer_id)
                } else {
                    Message::TickAnimations
                }
            }
            Err(e) => Message::ProfileError(format!("Failed to load layers: {}", e)),
        },
    )
}

pub fn layer_activate_requested(
    state: &State,
    device_id: String,
    layer_id: usize,
    mode: LayerMode,
) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let id = device_id.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.activate_layer(&id, layer_id, mode).await
        },
        move |result| match result {
            Ok(()) => Message::LayerStateChanged(device_id, layer_id),
            Err(e) => Message::ProfileError(format!("Failed to activate layer: {}", e)),
        },
    )
}

pub fn layer_config_updated(
    state: &State,
    device_id: String,
    config: LayerConfigInfo,
) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let id = device_id.clone();
    let layer_id = config.layer_id;
    let name = config.name.clone();
    let mode = config.mode;
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.set_layer_config(&id, layer_id, name, mode).await
        },
        move |result| match result {
            Ok(()) => Message::LayerConfigRequested(device_id),
            Err(e) => Message::ProfileError(format!("Failed to update layer config: {}", e)),
        },
    )
}

pub fn open_layer_config_dialog(
    state: &mut State,
    device_id: String,
    layer_id: usize,
) -> Command<Message> {
    let current_name = state
        .layer_configs
        .get(&device_id)
        .and_then(|layers| layers.iter().find(|l| l.layer_id == layer_id))
        .map(|l| l.name.clone())
        .unwrap_or_else(|| format!("Layer {}", layer_id));

    let current_mode = state
        .layer_configs
        .get(&device_id)
        .and_then(|layers| layers.iter().find(|l| l.layer_id == layer_id))
        .map(|l| l.mode)
        .unwrap_or(LayerMode::Hold);

    state.layer_config_dialog = Some((device_id, layer_id, current_name, current_mode));
    Command::none()
}

pub fn layer_config_name_changed(state: &mut State, name: String) -> Command<Message> {
    if let Some((device_id, layer_id, _, mode)) = state.layer_config_dialog.take() {
        state.layer_config_dialog = Some((device_id, layer_id, name, mode));
    }
    Command::none()
}

pub fn layer_config_mode_changed(state: &mut State, mode: LayerMode) -> Command<Message> {
    if let Some((device_id, layer_id, name, _)) = state.layer_config_dialog.take() {
        state.layer_config_dialog = Some((device_id, layer_id, name, mode));
    }
    Command::none()
}

pub fn save_layer_config(state: &mut State) -> Command<Message> {
    if let Some((device_id, layer_id, name, mode)) = state.layer_config_dialog.take() {
        let config = LayerConfigInfo {
            layer_id,
            name: name.clone(),
            mode,
            remap_count: 0,
            led_color: (0, 0, 255),
            led_zone: None,
        };
        Command::perform(async move { (device_id, config) }, |(device_id, config)| {
            Message::LayerConfigUpdated(device_id, config)
        })
    } else {
        Command::none()
    }
}

pub fn cancel_layer_config(state: &mut State) -> Command<Message> {
    state.layer_config_dialog = None;
    Command::none()
}

pub fn refresh_layers(state: &State) -> Command<Message> {
    let mut commands = Vec::new();

    for device_id in state.device_profiles.keys() {
        let device_id = device_id.clone();
        let socket_path = state.socket_path.clone();
        commands.push(Command::perform(
            async move {
                let client = crate::ipc::IpcClient::new(socket_path);
                (device_id.clone(), client.list_layers(&device_id).await)
            },
            |(device_id, result)| match result {
                Ok(layers) => Message::LayerListLoaded(device_id, layers),
                Err(_) => Message::TickAnimations,
            },
        ));
    }

    for device_id in state.active_layers.keys().cloned().collect::<Vec<_>>() {
        let device_id = device_id.clone();
        let socket_path = state.socket_path.clone();
        commands.push(Command::perform(
            async move {
                let client = crate::ipc::IpcClient::new(socket_path);
                (device_id.clone(), client.get_active_layer(&device_id).await)
            },
            |(device_id, result)| match result {
                Ok(Some(layer_id)) => Message::LayerStateChanged(device_id, layer_id),
                _ => Message::TickAnimations,
            },
        ));
    }

    Command::batch(commands)
}

pub fn layer_list_loaded(
    state: &mut State,
    device_id: String,
    layers: Vec<LayerConfigInfo>,
) -> Command<Message> {
    state.layer_configs.insert(device_id, layers);
    Command::none()
}

pub fn analog_dpad_mode_requested(state: &State, device_id: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let device_id_clone = device_id.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.get_analog_dpad_mode(&device_id_clone).await
        },
        move |result| match result {
            Ok(mode) => Message::AnalogDpadModeLoaded(device_id, mode),
            Err(e) => {
                eprintln!("Failed to get D-pad mode: {}", e);
                Message::TickAnimations
            }
        },
    )
}

pub fn analog_dpad_mode_loaded(
    state: &mut State,
    device_id: String,
    mode: String,
) -> Command<Message> {
    state.analog_dpad_modes.insert(device_id, mode);
    Command::none()
}

pub fn set_analog_dpad_mode(state: &State, device_id: String, mode: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let device_id_clone = device_id.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.set_analog_dpad_mode(&device_id_clone, &mode).await
        },
        |result| match result {
            Ok(_) => Message::AnalogDpadModeSet(Ok(())),
            Err(e) => Message::AnalogDpadModeSet(Err(e)),
        },
    )
}

pub fn analog_dpad_mode_set(state: &mut State, result: Result<(), String>) -> Command<Message> {
    if let Err(e) = result {
        eprintln!("Failed to set D-pad mode: {}", e);
    }
    Command::none()
}

pub fn analog_deadzone_xy_requested(state: &State, device_id: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let device_id_clone = device_id.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.get_analog_deadzone_xy(&device_id_clone).await
        },
        move |result| match result {
            Ok((x_pct, y_pct)) => Message::AnalogDeadzoneXYLoaded(device_id, (x_pct, y_pct)),
            Err(e) => {
                eprintln!("Failed to get per-axis deadzone: {}", e);
                Message::TickAnimations
            }
        },
    )
}

pub fn analog_deadzone_xy_loaded(
    state: &mut State,
    device_id: String,
    xy: (u8, u8),
) -> Command<Message> {
    state.analog_deadzones_xy.insert(device_id, xy);
    Command::none()
}

pub fn set_analog_deadzone_xy(
    state: &State,
    device_id: String,
    x_pct: u8,
    y_pct: u8,
) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client
                .set_analog_deadzone_xy(&device_id, x_pct, y_pct)
                .await
        },
        |result| match result {
            Ok(_) => Message::AnalogDeadzoneXYSet(Ok(())),
            Err(e) => Message::AnalogDeadzoneXYSet(Err(e)),
        },
    )
}

pub fn analog_deadzone_xy_set(state: &mut State, result: Result<(), String>) -> Command<Message> {
    if let Err(e) = result {
        eprintln!("Failed to set per-axis deadzone: {}", e);
        state.add_notification(&format!("Failed to set deadzone: {}", e), true);
    }
    Command::none()
}

pub fn analog_outer_deadzone_xy_requested(state: &State, device_id: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let device_id_clone = device_id.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.get_analog_outer_deadzone_xy(&device_id_clone).await
        },
        move |result| match result {
            Ok((x_pct, y_pct)) => Message::AnalogOuterDeadzoneXYLoaded(device_id, (x_pct, y_pct)),
            Err(e) => {
                eprintln!("Failed to get per-axis outer deadzone: {}", e);
                Message::TickAnimations
            }
        },
    )
}

pub fn analog_outer_deadzone_xy_loaded(
    state: &mut State,
    device_id: String,
    xy: (u8, u8),
) -> Command<Message> {
    state.analog_outer_deadzones_xy.insert(device_id, xy);
    Command::none()
}

pub fn set_analog_outer_deadzone_xy(
    state: &State,
    device_id: String,
    x_pct: u8,
    y_pct: u8,
) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client
                .set_analog_outer_deadzone_xy(&device_id, x_pct, y_pct)
                .await
        },
        |result| match result {
            Ok(_) => Message::AnalogOuterDeadzoneXYSet(Ok(())),
            Err(e) => Message::AnalogOuterDeadzoneXYSet(Err(e)),
        },
    )
}

pub fn analog_outer_deadzone_xy_set(
    state: &mut State,
    result: Result<(), String>,
) -> Command<Message> {
    if let Err(e) = result {
        eprintln!("Failed to set per-axis outer deadzone: {}", e);
        state.add_notification(&format!("Failed to set outer deadzone: {}", e), true);
    }
    Command::none()
}
