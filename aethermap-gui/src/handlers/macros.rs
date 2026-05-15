use crate::gui::{Message, State};
use aethermap_common::MacroSettings;
use iced::Command;
use std::time::Instant;

pub fn load(state: &State) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.list_macros().await.map_err(|e| e.to_string())
        },
        Message::MacrosLoaded,
    )
}

pub fn loaded(state: &mut State, macros: Vec<aethermap_common::MacroEntry>) -> Command<Message> {
    let count = macros.len();
    state.macros = macros;
    state.add_notification(&format!("Loaded {} macros", count), false);
    Command::none()
}

pub fn load_error(state: &mut State, e: String) -> Command<Message> {
    state.add_notification(&format!("Error loading macros: {}", e), true);
    Command::none()
}

pub fn load_settings(state: &State) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client.get_macro_settings().await.map_err(|e| e.to_string())
        },
        Message::MacroSettingsLoaded,
    )
}

pub fn settings_loaded(state: &mut State, settings: MacroSettings) -> Command<Message> {
    state.macro_settings = settings;
    Command::none()
}

pub fn settings_load_error(state: &mut State, e: String) -> Command<Message> {
    state.add_notification(&format!("Error loading macro settings: {}", e), true);
    Command::none()
}

pub fn set_settings(state: &State, settings: MacroSettings) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client
                .set_macro_settings(settings)
                .await
                .map_err(|e| e.to_string())
        },
        |result| match result {
            Ok(_) => Message::TickAnimations,
            Err(e) => Message::ShowNotification(format!("Failed to save settings: {}", e), true),
        },
    )
}

pub fn latency_changed(state: &mut State, ms: u32) -> Command<Message> {
    state.macro_settings.latency_offset_ms = ms;
    let settings = state.macro_settings.clone();
    Command::perform(async move { Message::SetMacroSettings(settings) }, |msg| {
        msg
    })
}

pub fn jitter_changed(state: &mut State, pct: f32) -> Command<Message> {
    state.macro_settings.jitter_pct = pct;
    let settings = state.macro_settings.clone();
    Command::perform(async move { Message::SetMacroSettings(settings) }, |msg| {
        msg
    })
}

pub fn capture_mouse_toggled(state: &mut State, enabled: bool) -> Command<Message> {
    state.macro_settings.capture_mouse = enabled;
    let settings = state.macro_settings.clone();
    Command::perform(async move { Message::SetMacroSettings(settings) }, |msg| {
        msg
    })
}

pub fn play(state: &State, macro_name: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let name = macro_name.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client
                .test_macro(&name)
                .await
                .map(|_| name)
                .map_err(|e| e.to_string())
        },
        Message::MacroPlayed,
    )
}

pub fn played_ok(state: &mut State, name: String) -> Command<Message> {
    state.add_notification(&format!("Played macro: {}", name), false);
    Command::none()
}

pub fn played_error(state: &mut State, e: String) -> Command<Message> {
    state.add_notification(&format!("Failed to play: {}", e), true);
    Command::none()
}

pub fn update_name(state: &mut State, name: String) -> Command<Message> {
    state.new_macro_name = name;
    Command::none()
}

pub fn update_profile_name(state: &mut State, name: String) -> Command<Message> {
    state.profile_name = name;
    Command::none()
}

pub fn start_recording(state: &mut State) -> Command<Message> {
    if state.new_macro_name.trim().is_empty() {
        state.add_notification("Enter a macro name first", true);
        return Command::none();
    }
    if state.grabbed_devices.is_empty() {
        state.add_notification("Grab a device first", true);
        return Command::none();
    }

    let device_path = state.grabbed_devices.iter().next().unwrap().clone();
    let socket_path = state.socket_path.clone();
    let macro_name = state.new_macro_name.clone();
    let capture_mouse = state.macro_settings.capture_mouse;
    state.recording = true;
    state.recording_macro_name = Some(macro_name.clone());

    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client
                .start_recording_macro(&device_path, &macro_name, capture_mouse)
                .await
                .map(|_| macro_name)
                .map_err(|e| e.to_string())
        },
        Message::RecordingStarted,
    )
}

pub fn recording_started_ok(state: &mut State, name: String) -> Command<Message> {
    state.add_notification(&format!("Recording '{}' - Press keys now!", name), false);
    Command::none()
}

pub fn recording_started_error(state: &mut State, e: String) -> Command<Message> {
    state.recording = false;
    state.recording_macro_name = None;
    state.add_notification(&format!("Failed to start recording: {}", e), true);
    Command::none()
}

pub fn stop_recording(state: &State) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client
                .stop_recording_macro()
                .await
                .map_err(|e| e.to_string())
        },
        Message::RecordingStopped,
    )
}

pub fn recording_stopped_ok(
    state: &mut State,
    macro_entry: aethermap_common::MacroEntry,
) -> Command<Message> {
    let name = macro_entry.name.clone();
    state.macros.push(macro_entry);
    state.recording = false;
    state.recording_macro_name = None;
    state
        .recently_updated_macros
        .insert(name.clone(), Instant::now());
    state.new_macro_name.clear();
    state.add_notification(&format!("Recorded macro: {}", name), false);
    Command::none()
}

pub fn recording_stopped_error(state: &mut State, e: String) -> Command<Message> {
    state.recording = false;
    state.recording_macro_name = None;
    state.add_notification(&format!("Recording failed: {}", e), true);
    Command::none()
}

pub fn delete(state: &State, macro_name: String) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    let name = macro_name.clone();
    Command::perform(
        async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            client
                .delete_macro(&name)
                .await
                .map(|_| name)
                .map_err(|e| e.to_string())
        },
        Message::MacroDeleted,
    )
}

pub fn deleted_ok(state: &mut State, name: String) -> Command<Message> {
    state.macros.retain(|m| m.name != name);
    state.add_notification(&format!("Deleted: {}", name), false);
    Command::none()
}

pub fn deleted_error(state: &mut State, e: String) -> Command<Message> {
    state.add_notification(&format!("Delete failed: {}", e), true);
    Command::none()
}
