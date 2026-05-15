use crate::gui::{azeron_keypad_layout, Message, State, Tab};
use aethermap_common::{DeviceCapabilities, DeviceInfo, RemapEntry, RemapProfileInfo};
use iced::Command;

// ---------------------------------------------------------------------------
// Device listing
// ---------------------------------------------------------------------------

pub fn load_devices(state: &mut State) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    state.loading = true;
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.get_devices().await.map_err(|e| e.to_string())
        },
        Message::DevicesLoaded,
    )
}

pub fn devices_loaded(state: &mut State, devices: Vec<DeviceInfo>) -> Command<Message> {
    let count = devices.len();
    state.devices = devices;
    state.loading = false;
    state.add_notification(&format!("Found {} devices", count), false);
    Command::perform(async { Message::LoadMacros }, |msg| msg)
}

pub fn devices_load_error(state: &mut State, e: String) -> Command<Message> {
    state.loading = false;
    state.add_notification(&format!("Error: {}", e), true);
    Command::none()
}

// ---------------------------------------------------------------------------
// Profile save / load (macro profiles)
// ---------------------------------------------------------------------------

pub fn save_profile(state: &mut State) -> Command<Message> {
    if state.profile_name.trim().is_empty() {
        state.add_notification("Enter a profile name", true);
        return Command::none();
    }
    let socket_path = state.socket_path.clone();
    let name = state.profile_name.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.save_profile(&name).await.map_err(|e| e.to_string())
        },
        Message::ProfileSaved,
    )
}

pub fn profile_saved(state: &mut State, name: String, count: usize) -> Command<Message> {
    state.add_notification(&format!("Saved '{}' ({} macros)", name, count), false);
    Command::none()
}

pub fn profile_save_error(state: &mut State, e: String) -> Command<Message> {
    state.add_notification(&format!("Save failed: {}", e), true);
    Command::none()
}

pub fn load_profile(state: &mut State) -> Command<Message> {
    if state.profile_name.trim().is_empty() {
        state.add_notification("Enter a profile name to load", true);
        return Command::none();
    }
    let socket_path = state.socket_path.clone();
    let name = state.profile_name.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.load_profile(&name).await.map_err(|e| e.to_string())
        },
        Message::ProfileLoaded,
    )
}

pub fn profile_loaded(state: &mut State, name: String, count: usize) -> Command<Message> {
    state.add_notification(&format!("Loaded '{}' ({} macros)", name, count), false);
    Command::perform(async { Message::LoadMacros }, |msg| msg)
}

pub fn profile_load_error(state: &mut State, e: String) -> Command<Message> {
    state.add_notification(&format!("Load failed: {}", e), true);
    Command::none()
}

// ---------------------------------------------------------------------------
// Device grab / ungrab
// ---------------------------------------------------------------------------

pub fn grab_device(state: &State, device_path: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let path_clone = device_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client
                .grab_device(&path_clone)
                .await
                .map(|_| path_clone)
                .map_err(|e| e.to_string())
        },
        Message::DeviceGrabbed,
    )
}

pub fn ungrab_device(state: &State, device_path: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let path_clone = device_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client
                .ungrab_device(&path_clone)
                .await
                .map(|_| path_clone)
                .map_err(|e| e.to_string())
        },
        Message::DeviceUngrabbed,
    )
}

pub fn device_grabbed(state: &mut State, device_path: String) -> Command<Message> {
    state.grabbed_devices.insert(device_path.clone());
    if let Some(idx) = state
        .devices
        .iter()
        .position(|d| d.path.to_string_lossy() == device_path)
    {
        state.selected_device = Some(idx);
    }
    state.add_notification("Device grabbed - ready for recording", false);
    Command::none()
}

pub fn device_grab_error(state: &mut State, e: String) -> Command<Message> {
    state.add_notification(&format!("Grab failed: {}", e), true);
    Command::none()
}

pub fn device_ungrabbed(state: &mut State, device_path: String) -> Command<Message> {
    state.grabbed_devices.remove(&device_path);
    state.add_notification("Device released", false);
    Command::none()
}

pub fn device_ungrab_error(state: &mut State, e: String) -> Command<Message> {
    state.add_notification(&format!("Release failed: {}", e), true);
    Command::none()
}

// ---------------------------------------------------------------------------
// Device profiles (auto-switch profiles per device)
// ---------------------------------------------------------------------------

pub fn load_device_profiles(state: &State, device_id: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let id = device_id.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            (id.clone(), client.get_device_profiles(id).await)
        },
        |(device_id, result)| {
            Message::DeviceProfilesLoaded(device_id, result.map_err(|e| e.to_string()))
        },
    )
}

pub fn device_profiles_loaded(
    state: &mut State,
    device_id: String,
    profiles: Vec<String>,
) -> Command<Message> {
    state.device_profiles.insert(device_id.clone(), profiles);
    state.add_notification(
        &format!(
            "Loaded {} profiles for {}",
            state
                .device_profiles
                .get(&device_id)
                .map(|p| p.len())
                .unwrap_or(0),
            device_id,
        ),
        false,
    );
    Command::none()
}

pub fn device_profiles_load_error(
    state: &mut State,
    device_id: String,
    e: String,
) -> Command<Message> {
    let _ = device_id;
    state.add_notification(&format!("Failed to load device profiles: {}", e), true);
    Command::none()
}

pub fn activate_profile(
    state: &State,
    device_id: String,
    profile_name: String,
) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let id = device_id.clone();
    let name = profile_name.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.activate_profile(id.clone(), name.clone()).await
        },
        move |result| match result {
            Ok(()) => Message::ProfileActivated(device_id, profile_name),
            Err(e) => Message::ProfileError(format!("Failed to activate profile: {}", e)),
        },
    )
}

pub fn profile_activated(
    state: &mut State,
    device_id: String,
    profile_name: String,
) -> Command<Message> {
    state
        .active_profiles
        .insert(device_id.clone(), profile_name.clone());
    state.add_notification(
        &format!("Activated profile '{}' on {}", profile_name, device_id),
        false,
    );
    Command::none()
}

pub fn deactivate_profile(state: &State, device_id: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let id = device_id.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.deactivate_profile(id.clone()).await
        },
        move |result| match result {
            Ok(()) => Message::ProfileDeactivated(device_id),
            Err(e) => Message::ProfileError(format!("Failed to deactivate profile: {}", e)),
        },
    )
}

pub fn profile_deactivated(state: &mut State, device_id: String) -> Command<Message> {
    state.active_profiles.remove(&device_id);
    state.add_notification(&format!("Deactivated profile on {}", device_id), false);
    Command::none()
}

pub fn profile_error(state: &mut State, msg: String) -> Command<Message> {
    state.add_notification(&msg, true);
    Command::none()
}

// ---------------------------------------------------------------------------
// Remap profiles
// ---------------------------------------------------------------------------

pub fn load_remap_profiles(state: &State, device_path: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let path = device_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            (path.clone(), client.list_remap_profiles(&path).await)
        },
        |(device_path, result)| {
            Message::RemapProfilesLoaded(device_path, result.map_err(|e| e.to_string()))
        },
    )
}

pub fn remap_profiles_loaded(
    state: &mut State,
    device_path: String,
    profiles: Vec<RemapProfileInfo>,
) -> Command<Message> {
    state.remap_profiles.insert(device_path.clone(), profiles);
    state.add_notification(
        &format!(
            "Loaded {} remap profiles for {}",
            state
                .remap_profiles
                .get(&device_path)
                .map(|p| p.len())
                .unwrap_or(0),
            device_path,
        ),
        false,
    );
    Command::none()
}

pub fn remap_profiles_load_error(
    state: &mut State,
    device_path: String,
    e: String,
) -> Command<Message> {
    let _ = device_path;
    state.add_notification(&format!("Failed to load remap profiles: {}", e), true);
    Command::none()
}

pub fn activate_remap_profile(
    state: &State,
    device_path: String,
    profile_name: String,
) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let path = device_path.clone();
    let name = profile_name.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.activate_remap_profile(&path, &name).await
        },
        move |result| match result {
            Ok(()) => Message::RemapProfileActivated(device_path, profile_name),
            Err(e) => Message::ProfileError(format!("Failed to activate remap profile: {}", e)),
        },
    )
}

pub fn remap_profile_activated(
    state: &mut State,
    device_path: String,
    profile_name: String,
) -> Command<Message> {
    state
        .active_remap_profiles
        .insert(device_path.clone(), profile_name.clone());
    state.add_notification(
        &format!(
            "Activated remap profile '{}' on {}",
            profile_name, device_path
        ),
        false,
    );
    Command::perform(async move { device_path.clone() }, |path| {
        Message::LoadActiveRemaps(path)
    })
}

pub fn deactivate_remap_profile(state: &State, device_path: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let path = device_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.deactivate_remap_profile(&path).await
        },
        move |result| match result {
            Ok(()) => Message::RemapProfileDeactivated(device_path),
            Err(e) => Message::ProfileError(format!("Failed to deactivate remap profile: {}", e)),
        },
    )
}

pub fn remap_profile_deactivated(state: &mut State, device_path: String) -> Command<Message> {
    state.active_remap_profiles.remove(&device_path);
    state.active_remaps.remove(&device_path);
    state.add_notification(
        &format!("Deactivated remap profile on {}", device_path),
        false,
    );
    Command::none()
}

// ---------------------------------------------------------------------------
// Active remaps
// ---------------------------------------------------------------------------

pub fn load_active_remaps(state: &State, device_path: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let path = device_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            (path.clone(), client.get_active_remaps(&path).await)
        },
        |(device_path, result)| {
            Message::ActiveRemapsLoaded(device_path, result.map_err(|e| e.to_string()))
        },
    )
}

pub fn active_remaps_loaded(
    state: &mut State,
    device_path: String,
    result: Option<(String, Vec<RemapEntry>)>,
) -> Command<Message> {
    match result {
        Some((profile_name, remaps)) => {
            state
                .active_remaps
                .insert(device_path, (profile_name, remaps));
        }
        None => {
            state.active_remaps.remove(&device_path);
        }
    }
    Command::none()
}

pub fn active_remaps_load_error(state: &mut State, e: String) -> Command<Message> {
    state.add_notification(&format!("Failed to load active remaps: {}", e), true);
    Command::none()
}

// ---------------------------------------------------------------------------
// Mouse event recording (GUI-side)
// ---------------------------------------------------------------------------

pub fn record_mouse_event(
    state: &mut State,
    event_type: String,
    button: Option<u16>,
    x: i32,
    y: i32,
    delta: i32,
) -> Command<Message> {
    if state.recording {
        let event_desc = match event_type.as_str() {
            "button_press" => format!("Mouse button {} pressed", button.unwrap_or(0)),
            "button_release" => format!("Mouse button {} released", button.unwrap_or(0)),
            "movement" => format!("Mouse moved to ({}, {})", x, y),
            "scroll" => format!("Mouse scrolled {}", delta),
            _ => format!("Unknown mouse event: {}", event_type),
        };
        state.status = event_desc;
    }
    Command::none()
}

// ---------------------------------------------------------------------------
// Keypad view
// ---------------------------------------------------------------------------

pub fn show_keypad_view(state: &mut State, device_path: String) -> Command<Message> {
    if device_path.is_empty() {
        state.device_capabilities = None;
        state.keypad_layout.clear();
        state.keypad_view_device = None;
        state.selected_button = None;
        return Command::none();
    }
    state.keypad_view_device = Some(device_path.clone());
    let socket_path = state.socket_path.clone();
    let path_clone = device_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            (
                path_clone.clone(),
                client.get_device_capabilities(&path_clone).await,
            )
        },
        |(device_path, result)| {
            Message::DeviceCapabilitiesLoaded(device_path, result.map_err(|e| e.to_string()))
        },
    )
}

pub fn device_capabilities_loaded(
    state: &mut State,
    device_path: String,
    capabilities: DeviceCapabilities,
) -> Command<Message> {
    state.device_capabilities = Some(capabilities);
    state.keypad_layout = azeron_keypad_layout();
    if let Some((profile_name, remaps)) = state.active_remaps.get(&device_path) {
        for remap in remaps {
            if let Some(button) = state
                .keypad_layout
                .iter_mut()
                .find(|b| b.id == remap.from_key)
            {
                button.current_remap = Some(remap.to_key.clone());
            }
        }
        state.add_notification(
            &format!("Loaded remaps from profile '{}'", profile_name),
            false,
        );
    }
    state.active_tab = Tab::Devices;
    Command::none()
}

pub fn device_capabilities_load_error(state: &mut State, e: String) -> Command<Message> {
    state.add_notification(&format!("Failed to load device capabilities: {}", e), true);
    Command::none()
}

pub fn select_keypad_button(state: &mut State, button_id: String) -> Command<Message> {
    state.selected_button = state.keypad_layout.iter().position(|b| b.id == button_id);
    state.status = format!(
        "Selected button: {} - Configure remapping in device profile",
        button_id
    );
    Command::none()
}
