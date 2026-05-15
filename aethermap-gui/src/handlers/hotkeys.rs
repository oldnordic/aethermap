use crate::gui::{Message, State};
use crate::views::hotkeys::HotkeyBinding;
use crate::views::hotkeys::HotkeyBindingsView;
use aethermap_common::ipc_client::IpcClient;
use aethermap_common::HotkeyBinding as CommonHotkeyBinding;
use aethermap_common::{Request, Response};
use iced::Command;

pub fn show(state: &mut State, device_id: String) -> Command<Message> {
    state.hotkey_view = Some(HotkeyBindingsView {
        device_id: device_id.clone(),
        bindings: Vec::new(),
        editing_binding: None,
        new_modifiers: Vec::new(),
        new_key: String::new(),
        new_profile_name: String::new(),
        new_layer_id: String::new(),
    });
    let device_id_clone = device_id.clone();
    Command::perform(async move { device_id_clone }, |id| {
        Message::LoadHotkeyBindings(id)
    })
}

pub fn close(state: &mut State) -> Command<Message> {
    state.hotkey_view = None;
    Command::none()
}

pub fn load(state: &State, device_id: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    Command::perform(
        async move {
            let client = IpcClient::with_socket_path(&socket_path);
            let request = Request::ListHotkeys { device_id };
            match client.send(&request).await {
                Ok(Response::HotkeyList { bindings, .. }) => Ok(bindings
                    .into_iter()
                    .map(|b| HotkeyBinding {
                        modifiers: b.modifiers,
                        key: b.key,
                        profile_name: b.profile_name,
                        device_id: b.device_id,
                        layer_id: b.layer_id,
                    })
                    .collect()),
                Ok(Response::Error(msg)) => Err(msg),
                Err(e) => Err(format!("IPC error: {}", e)),
                _ => Err("Unexpected response".to_string()),
            }
        },
        Message::HotkeyBindingsLoaded,
    )
}

pub fn loaded(state: &mut State, bindings: Vec<HotkeyBinding>) -> Command<Message> {
    if let Some(view) = &mut state.hotkey_view {
        view.bindings = bindings;
    }
    Command::none()
}

pub fn load_error(state: &mut State, error: String) -> Command<Message> {
    state.add_notification(&format!("Failed to load hotkey bindings: {}", error), true);
    Command::none()
}

pub fn edit(state: &mut State, index: usize) -> Command<Message> {
    if let Some(view) = &state.hotkey_view {
        if let Some(binding) = view.bindings.get(index) {
            state.hotkey_view = Some(HotkeyBindingsView {
                device_id: view.device_id.clone(),
                bindings: view.bindings.clone(),
                editing_binding: Some(index),
                new_modifiers: binding.modifiers.clone(),
                new_key: binding.key.clone(),
                new_profile_name: binding.profile_name.clone(),
                new_layer_id: binding
                    .layer_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
            });
        }
    }
    Command::none()
}

pub fn toggle_modifier(state: &mut State, modifier: String) -> Command<Message> {
    if let Some(view) = state.hotkey_view.as_mut() {
        if view.new_modifiers.contains(&modifier) {
            view.new_modifiers.retain(|m| m != &modifier);
        } else {
            view.new_modifiers.push(modifier);
        }
    }
    Command::none()
}

pub fn key_changed(state: &mut State, value: String) -> Command<Message> {
    if let Some(view) = state.hotkey_view.as_mut() {
        view.new_key = value;
    }
    Command::none()
}

pub fn profile_name_changed(state: &mut State, value: String) -> Command<Message> {
    if let Some(view) = state.hotkey_view.as_mut() {
        view.new_profile_name = value;
    }
    Command::none()
}

pub fn layer_id_changed(state: &mut State, value: String) -> Command<Message> {
    if let Some(view) = state.hotkey_view.as_mut() {
        view.new_layer_id = value;
    }
    Command::none()
}

pub fn save(state: &mut State) -> Command<Message> {
    if let Some(view) = &state.hotkey_view {
        let device_id = view.device_id.clone();
        let binding = CommonHotkeyBinding {
            modifiers: view.new_modifiers.clone(),
            key: view.new_key.clone(),
            profile_name: view.new_profile_name.clone(),
            device_id: Some(view.device_id.clone()),
            layer_id: if view.new_layer_id.is_empty() {
                None
            } else {
                view.new_layer_id.parse().ok()
            },
        };
        let socket_path = state.socket_path.clone();

        if let Some(local_view) = &state.hotkey_view {
            let gui_binding = HotkeyBinding {
                modifiers: binding.modifiers.clone(),
                key: binding.key.clone(),
                profile_name: binding.profile_name.clone(),
                device_id: binding.device_id.clone(),
                layer_id: binding.layer_id,
            };
            let mut updated_view = local_view.clone();
            if let Some(editing) = local_view.editing_binding {
                if editing < local_view.bindings.len() {
                    updated_view.bindings[editing] = gui_binding;
                }
            } else {
                updated_view.bindings.push(gui_binding);
            }
            updated_view.editing_binding = None;
            updated_view.new_modifiers = Vec::new();
            updated_view.new_key = String::new();
            updated_view.new_profile_name = String::new();
            updated_view.new_layer_id = String::new();
            state.hotkey_view = Some(updated_view);
        }

        return Command::perform(
            async move {
                let client = IpcClient::with_socket_path(&socket_path);
                let request = Request::RegisterHotkey { device_id, binding };
                match client.send(&request).await {
                    Ok(Response::HotkeyRegistered { .. }) => Ok(()),
                    Ok(Response::Error(msg)) => Err(msg),
                    Err(e) => Err(format!("IPC error: {}", e)),
                    _ => Err("Unexpected response".to_string()),
                }
            },
            |result| match result {
                Ok(()) => Message::ShowNotification("Hotkey saved".to_string(), false),
                Err(e) => Message::ShowNotification(format!("Failed to save hotkey: {}", e), true),
            },
        );
    }
    Command::none()
}

pub fn delete(state: &mut State, index: usize) -> Command<Message> {
    if let Some(view) = &state.hotkey_view {
        if index < view.bindings.len() {
            let device_id = view.device_id.clone();
            let binding = view.bindings[index].clone();
            let socket_path = state.socket_path.clone();

            let updated_bindings = view
                .bindings
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != index)
                .map(|(_, b)| b.clone())
                .collect();

            return Command::perform(
                async move {
                    let client = IpcClient::with_socket_path(&socket_path);
                    let request = Request::RemoveHotkey {
                        device_id,
                        key: binding.key.clone(),
                        modifiers: binding.modifiers.clone(),
                    };
                    match client.send(&request).await {
                        Ok(Response::HotkeyRemoved { .. }) => Ok(()),
                        Ok(Response::Error(msg)) => Err(msg),
                        Err(e) => Err(format!("IPC error: {}", e)),
                        _ => Err("Unexpected response".to_string()),
                    }
                },
                move |result| match result {
                    Ok(_) => Message::HotkeyBindingsUpdated(updated_bindings),
                    Err(err_msg) => Message::ShowNotification(
                        format!("Failed to delete hotkey: {}", err_msg),
                        true,
                    ),
                },
            );
        }
    }
    Command::none()
}

pub fn bindings_updated(state: &mut State, bindings: Vec<HotkeyBinding>) -> Command<Message> {
    if let Some(view) = &mut state.hotkey_view {
        view.bindings = bindings;
    }
    state.add_notification("Hotkey deleted", false);
    Command::none()
}
