use crate::gui::{AutoSwitchRule, AutoSwitchRulesView, Message, State};
use aethermap_common::ipc_client::IpcClient;
use aethermap_common::AutoSwitchRule as CommonAutoSwitchRule;
use aethermap_common::{Request, Response};
use iced::Command;

pub fn show(state: &mut State, device_id: String) -> Command<Message> {
    state.auto_switch_view = Some(AutoSwitchRulesView {
        device_id: device_id.clone(),
        rules: Vec::new(),
        editing_rule: None,
        new_app_id: String::new(),
        new_profile_name: String::new(),
        new_layer_id: String::new(),
    });
    let device_id_clone = device_id.clone();
    Command::perform(async move { device_id_clone }, |id| {
        Message::LoadAutoSwitchRules(id)
    })
}

pub fn close(state: &mut State) -> Command<Message> {
    state.auto_switch_view = None;
    Command::none()
}

pub fn load(state: &State) -> Command<Message> {
    let socket_path = state.socket_path.clone();
    Command::perform(
        async move {
            let client = IpcClient::with_socket_path(&socket_path);
            let request = Request::GetAutoSwitchRules;
            match client.send(&request).await {
                Ok(Response::AutoSwitchRules { rules }) => Ok(rules
                    .into_iter()
                    .map(|r| AutoSwitchRule {
                        app_id: r.app_id,
                        profile_name: r.profile_name,
                        device_id: r.device_id,
                        layer_id: r.layer_id,
                    })
                    .collect()),
                Ok(Response::Error(msg)) => Err(msg),
                Err(e) => Err(format!("IPC error: {}", e)),
                _ => Err("Unexpected response".to_string()),
            }
        },
        Message::AutoSwitchRulesLoaded,
    )
}

pub fn loaded(state: &mut State, rules: Vec<AutoSwitchRule>) -> Command<Message> {
    if let Some(view) = state.auto_switch_view.as_mut() {
        view.rules = rules;
    }
    Command::none()
}

pub fn load_error(state: &mut State, error: String) -> Command<Message> {
    state.add_notification(
        &format!("Failed to load auto-switch rules: {}", error),
        true,
    );
    Command::none()
}

pub fn edit(state: &mut State, index: usize) -> Command<Message> {
    if let Some(view) = &state.auto_switch_view {
        if let Some(rule) = view.rules.get(index) {
            state.auto_switch_view = Some(AutoSwitchRulesView {
                device_id: view.device_id.clone(),
                rules: view.rules.clone(),
                editing_rule: Some(index),
                new_app_id: rule.app_id.clone(),
                new_profile_name: rule.profile_name.clone(),
                new_layer_id: rule.layer_id.map(|id| id.to_string()).unwrap_or_default(),
            });
        }
    }
    Command::none()
}

pub fn app_id_changed(state: &mut State, value: String) -> Command<Message> {
    if let Some(view) = state.auto_switch_view.as_mut() {
        view.new_app_id = value;
    }
    Command::none()
}

pub fn profile_name_changed(state: &mut State, value: String) -> Command<Message> {
    if let Some(view) = state.auto_switch_view.as_mut() {
        view.new_profile_name = value;
    }
    Command::none()
}

pub fn layer_id_changed(state: &mut State, value: String) -> Command<Message> {
    if let Some(view) = state.auto_switch_view.as_mut() {
        view.new_layer_id = value;
    }
    Command::none()
}

pub fn use_current_app(state: &mut State) -> Command<Message> {
    if let Some(ref focus) = state.current_focus {
        if let Some(view) = state.auto_switch_view.as_mut() {
            view.new_app_id = focus.clone();
        }
    }
    Command::none()
}

pub fn save(state: &mut State) -> Command<Message> {
    if let Some(mut view) = state.auto_switch_view.clone() {
        let rule = AutoSwitchRule {
            app_id: view.new_app_id.clone(),
            profile_name: view.new_profile_name.clone(),
            device_id: Some(view.device_id.clone()),
            layer_id: view.new_layer_id.parse().ok(),
        };

        if let Some(editing) = view.editing_rule {
            if editing < view.rules.len() {
                view.rules[editing] = rule.clone();
            }
        } else {
            view.rules.push(rule.clone());
        }

        view.editing_rule = None;
        view.new_app_id = String::new();
        view.new_profile_name = String::new();
        view.new_layer_id = String::new();

        let rules = view.rules.clone();
        let socket_path = state.socket_path.clone();

        state.auto_switch_view = Some(view);

        Command::perform(
            async move {
                let common_rules: Vec<CommonAutoSwitchRule> = rules
                    .into_iter()
                    .map(|r| CommonAutoSwitchRule {
                        app_id: r.app_id,
                        profile_name: r.profile_name,
                        device_id: r.device_id,
                        layer_id: r.layer_id,
                    })
                    .collect();

                let client = IpcClient::with_socket_path(socket_path);
                let request = Request::SetAutoSwitchRules {
                    rules: common_rules,
                };
                match client.send(&request).await {
                    Ok(Response::AutoSwitchRulesAck) => Ok(()),
                    Ok(Response::Error(msg)) => Err(msg),
                    Err(e) => Err(format!("IPC error: {}", e)),
                    _ => Err("Unexpected response".to_string()),
                }
            },
            |result| match result {
                Ok(()) => Message::ShowNotification("Auto-switch rules saved".to_string(), false),
                Err(e) => Message::ShowNotification(format!("Failed to save rules: {}", e), true),
            },
        )
    } else {
        Command::none()
    }
}

pub fn delete(state: &mut State, index: usize) -> Command<Message> {
    if let Some(view) = state.auto_switch_view.clone() {
        if index < view.rules.len() {
            let mut rules = view.rules.clone();
            rules.remove(index);
            let socket_path = state.socket_path.clone();

            if let Some(v) = state.auto_switch_view.as_mut() {
                v.rules = rules.clone();
            }

            return Command::perform(
                async move {
                    let common_rules: Vec<CommonAutoSwitchRule> = rules
                        .into_iter()
                        .map(|r| CommonAutoSwitchRule {
                            app_id: r.app_id,
                            profile_name: r.profile_name,
                            device_id: r.device_id,
                            layer_id: r.layer_id,
                        })
                        .collect();

                    let client = IpcClient::with_socket_path(&socket_path);
                    let request = Request::SetAutoSwitchRules {
                        rules: common_rules,
                    };
                    match client.send(&request).await {
                        Ok(Response::AutoSwitchRulesAck) => Ok(()),
                        Ok(Response::Error(msg)) => Err(msg),
                        Err(e) => Err(format!("IPC error: {}", e)),
                        _ => Err("Unexpected response".to_string()),
                    }
                },
                |result| match result {
                    Ok(()) => Message::ShowNotification("Rule deleted".to_string(), false),
                    Err(e) => {
                        Message::ShowNotification(format!("Failed to delete rule: {}", e), true)
                    }
                },
            );
        }
    }
    Command::none()
}
