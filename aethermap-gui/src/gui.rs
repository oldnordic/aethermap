use iced::{
    widget::{
        button, column, container, row, text, text_input, scrollable,
        horizontal_rule, vertical_rule, pick_list, Column, Space,
    },
    Element, Length, Subscription, Theme, Application, Command,
    Alignment,
};
use std::sync::Arc;
use crate::theme::{self, aether_dark, aether_light};
use crate::views;

// Import custom widgets
use aethermap_common::{DeviceInfo, DeviceCapabilities, DeviceType, LayerConfigInfo, LayerMode, LedPattern, LedZone, MacroEntry, MacroSettings, RemapProfileInfo, RemapEntry, AnalogMode, CameraOutputMode, Request, Response, AutoSwitchRule as CommonAutoSwitchRule};
use aethermap_common::HotkeyBinding as CommonHotkeyBinding;
use aethermap_common::ipc_client::IpcClient;
use std::path::PathBuf;
use std::collections::{VecDeque, HashMap, HashSet};
use std::time::{Duration, Instant};

// Import focus_tracker types - need to use path from lib.rs root
// Since we're in gui.rs (a module of aethermap_gui library),
// we access sibling modules via super:: or direct path when in closures

// Razer brand colors (for future custom theming)
// const RAZER_GREEN: Color = Color::from_rgb(0.267, 0.839, 0.173); // #44D62C
// const RAZER_GREEN_DIM: Color = Color::from_rgb(0.176, 0.561, 0.118); // #2D8F1E
// const BG_DEEP: Color = Color::from_rgb(0.051, 0.051, 0.051); // #0D0D0D
// const BG_SURFACE: Color = Color::from_rgb(0.102, 0.102, 0.102); // #1A1A1A
// const BG_ELEVATED: Color = Color::from_rgb(0.141, 0.141, 0.141); // #242424
// const TEXT_PRIMARY: Color = Color::WHITE;
// const TEXT_SECONDARY: Color = Color::from_rgb(0.702, 0.702, 0.702); // #B3B3B3
// const TEXT_MUTED: Color = Color::from_rgb(0.400, 0.400, 0.400); // #666666
// const DANGER_RED: Color = Color::from_rgb(1.0, 0.231, 0.188); // #FF3B30
// const WARNING_YELLOW: Color = Color::from_rgb(1.0, 0.722, 0.0); // #FFB800

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Devices,
    Macros,
    Profiles,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub is_error: bool,
    pub timestamp: Instant,
}

pub use views::keypad::{KeypadButton, azeron_keypad_layout};

pub use views::auto_switch::{AutoSwitchRule, AutoSwitchRulesView};

pub use views::hotkeys::{HotkeyBinding, HotkeyBindingsView};

pub use views::analog::{DeadzoneShape, SensitivityCurve, CalibrationConfig, AnalogCalibrationView};

pub use views::led::LedState;

pub struct State {
    pub devices: Vec<DeviceInfo>,
    pub macros: Vec<MacroEntry>,
    pub selected_device: Option<usize>,
    pub status: String,
    pub status_history: VecDeque<String>,
    pub loading: bool,
    pub recording: bool,
    pub recording_macro_name: Option<String>,
    pub daemon_connected: bool,
    pub new_macro_name: String,
    pub socket_path: PathBuf,
    pub recently_updated_macros: HashMap<String, Instant>,
    pub grabbed_devices: HashSet<String>,
    pub profile_name: String,
    pub active_tab: Tab,
    pub notifications: VecDeque<Notification>,
    pub recording_pulse: bool,
    /// Available profiles per device (device_id -> profile names)
    pub device_profiles: HashMap<String, Vec<String>>,
    /// Active profile per device (device_id -> profile name)
    pub active_profiles: HashMap<String, String>,
    /// Available remap profiles per device (device_path -> profile info)
    pub remap_profiles: HashMap<String, Vec<RemapProfileInfo>>,
    /// Active remap profile per device (device_path -> profile name)
    pub active_remap_profiles: HashMap<String, String>,
    /// Active remaps per device (device_path -> remap entries)
    pub active_remaps: HashMap<String, (String, Vec<RemapEntry>)>,
    /// Azeron keypad layout for selected device
    pub keypad_layout: Vec<KeypadButton>,
    /// Current device path being viewed in keypad layout
    pub keypad_view_device: Option<String>,
    /// Selected button for remapping (index into keypad_layout)
    pub selected_button: Option<usize>,
    /// Device capabilities for current selection
    pub device_capabilities: Option<DeviceCapabilities>,
    /// Active layer per device (device_id -> active_layer_id)
    pub active_layers: HashMap<String, usize>,
    /// Layer configurations per device (device_id -> layers)
    pub layer_configs: HashMap<String, Vec<LayerConfigInfo>>,
    /// Layer configuration dialog state (device_id, layer_id, name, mode)
    pub layer_config_dialog: Option<(String, usize, String, LayerMode)>,
    /// D-pad mode per device (device_id -> mode)
    pub analog_dpad_modes: HashMap<String, String>,
    /// Per-axis deadzone values (device_id -> (x_percentage, y_percentage))
    pub analog_deadzones_xy: HashMap<String, (u8, u8)>,
    /// Per-axis outer deadzone values (device_id -> (x_percentage, y_percentage))
    pub analog_outer_deadzones_xy: HashMap<String, (u8, u8)>,
    /// LED configuration state per device (device_id -> LedState)
    pub led_states: HashMap<String, LedState>,
    /// LED configuration dialog open for device
    pub led_config_device: Option<String>,
    /// Currently selected LED zone for color editing
    pub selected_led_zone: Option<LedZone>,
    /// Pending color picker values (r, g, b) before application
    pub pending_led_color: Option<(u8, u8, u8)>,
    /// Current focused application ID (for auto-switch rule creation)
    pub current_focus: Option<String>,
    /// Focus tracking is active
    pub focus_tracking_active: bool,
    /// Auto-switch rules view (open when configuring auto-profile switching)
    pub auto_switch_view: Option<AutoSwitchRulesView>,
    /// Hotkey bindings view (open when configuring hotkeys)
    pub hotkey_view: Option<HotkeyBindingsView>,
    /// Analog calibration view (open when configuring analog stick)
    pub analog_calibration_view: Option<AnalogCalibrationView>,
    /// Global macro timing and jitter settings
    pub macro_settings: MacroSettings,
    /// Current UI theme (Adaptive COSMIC)
    pub current_theme: Theme,
}

impl Default for State {
    fn default() -> Self {
        let socket_path = if cfg!(target_os = "linux") {
            PathBuf::from("/run/aethermap/aethermap.sock")
        } else if cfg!(target_os = "macos") {
            PathBuf::from("/tmp/aethermap.sock")
        } else {
            std::env::temp_dir().join("aethermap.sock")
        };
        State {
            devices: Vec::new(),
            macros: Vec::new(),
            selected_device: None,
            status: "Initializing...".to_string(),
            status_history: VecDeque::with_capacity(10),
            loading: false,
            recording: false,
            recording_macro_name: None,
            daemon_connected: false,
            new_macro_name: String::new(),
            socket_path,
            recently_updated_macros: HashMap::new(),
            grabbed_devices: HashSet::new(),
            profile_name: "default".to_string(),
            active_tab: Tab::Devices,
            notifications: VecDeque::with_capacity(5),
            recording_pulse: false,
            device_profiles: HashMap::new(),
            active_profiles: HashMap::new(),
            remap_profiles: HashMap::new(),
            active_remap_profiles: HashMap::new(),
            active_remaps: HashMap::new(),
            keypad_layout: Vec::new(),
            keypad_view_device: None,
            selected_button: None,
            device_capabilities: None,
            active_layers: HashMap::new(),
            layer_configs: HashMap::new(),
            layer_config_dialog: None,
            analog_dpad_modes: HashMap::new(),
            analog_deadzones_xy: HashMap::new(),
            analog_outer_deadzones_xy: HashMap::new(),
            led_states: HashMap::new(),
            led_config_device: None,
            selected_led_zone: None,
            pending_led_color: None,
            current_focus: None,
            focus_tracking_active: false,
            auto_switch_view: None,
            hotkey_view: None,
            analog_calibration_view: None,
            macro_settings: MacroSettings {
                latency_offset_ms: 0,
                jitter_pct: 0.0,
                capture_mouse: false,
            },
            current_theme: aether_dark(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    SwitchTab(Tab),
    ThemeChanged(iced::Theme),

    // Device Management
    LoadDevices,
    DevicesLoaded(Result<Vec<DeviceInfo>, String>),
    GrabDevice(String),
    UngrabDevice(String),
    DeviceGrabbed(Result<String, String>),
    DeviceUngrabbed(Result<String, String>),
    SelectDevice(usize),

    // Macro Recording
    UpdateMacroName(String),
    StartRecording,
    StopRecording,
    RecordingStarted(Result<String, String>),
    RecordingStopped(Result<MacroEntry, String>),

    // Macro Management
    LoadMacros,
    MacrosLoaded(Result<Vec<MacroEntry>, String>),
    LoadMacroSettings,
    MacroSettingsLoaded(Result<MacroSettings, String>),
    SetMacroSettings(MacroSettings),
    LatencyChanged(u32),
    JitterChanged(f32),
    CaptureMouseToggled(bool),
    PlayMacro(String),
    MacroPlayed(Result<String, String>),
    DeleteMacro(String),
    MacroDeleted(Result<String, String>),

    // Profile Management
    UpdateProfileName(String),
    SaveProfile,
    ProfileSaved(Result<(String, usize), String>),
    LoadProfile,
    ProfileLoaded(Result<(String, usize), String>),

    // Device Profile Management
    LoadDeviceProfiles(String),
    DeviceProfilesLoaded(String, Result<Vec<String>, String>),
    ActivateProfile(String, String),
    ProfileActivated(String, String),
    DeactivateProfile(String),
    ProfileDeactivated(String),
    ProfileError(String),

    // Remap Profile Management
    LoadRemapProfiles(String),
    RemapProfilesLoaded(String, Result<Vec<RemapProfileInfo>, String>),
    ActivateRemapProfile(String, String),
    RemapProfileActivated(String, String),
    DeactivateRemapProfile(String),
    RemapProfileDeactivated(String),
    LoadActiveRemaps(String),
    ActiveRemapsLoaded(String, Result<Option<(String, Vec<RemapEntry>)>, String>),

    // Status
    CheckDaemonConnection,
    DaemonStatusChanged(bool),

    // UI
    TickAnimations,
    ShowNotification(String, bool), // (message, is_error)

    // Mouse Event Recording
    RecordMouseEvent {
        event_type: String,
        button: Option<u16>,
        x: i32,
        y: i32,
        delta: i32,
    },

    // Keypad Remapping
    /// Show keypad remapping view for a device
    ShowKeypadView(String),
    /// Select a keypad button for remapping
    SelectKeypadButton(String),
    /// Load device capabilities for keypad view
    DeviceCapabilitiesLoaded(String, Result<DeviceCapabilities, String>),

    // Layer Management
    /// Layer state changed (device_id, layer_id)
    LayerStateChanged(String, usize),
    /// Request layer configuration for a device
    LayerConfigRequested(String),
    /// Request activation of a layer (device_id, layer_id, mode)
    LayerActivateRequested(String, usize, LayerMode),
    /// Layer configuration updated (device_id, config)
    LayerConfigUpdated(String, LayerConfigInfo),
    /// Open layer config dialog for editing
    OpenLayerConfigDialog(String, usize),
    /// Update layer name in dialog
    LayerConfigNameChanged(String),
    /// Update layer mode in dialog
    LayerConfigModeChanged(LayerMode),
    /// Save layer config from dialog
    SaveLayerConfig,
    /// Cancel layer config dialog
    CancelLayerConfig,
    /// Periodic refresh of layer states
    RefreshLayers,
    /// Layer list loaded from daemon (device_id, layers)
    LayerListLoaded(String, Vec<LayerConfigInfo>),

    // D-pad Mode Management
    /// Request D-pad mode for a device
    AnalogDpadModeRequested(String),
    /// D-pad mode loaded (device_id, mode)
    AnalogDpadModeLoaded(String, String),
    /// Set D-pad mode (device_id, mode)
    SetAnalogDpadMode(String, String),
    /// D-pad mode set result
    AnalogDpadModeSet(Result<(), String>),

    // Per-Axis Deadzone Management
    /// Request per-axis deadzone for a device
    AnalogDeadzoneXYRequested(String),
    /// Per-axis deadzone loaded (device_id, (x_pct, y_pct))
    AnalogDeadzoneXYLoaded(String, (u8, u8)),
    /// Set per-axis deadzone (device_id, x_pct, y_pct)
    SetAnalogDeadzoneXY(String, u8, u8),
    /// Per-axis deadzone set result
    AnalogDeadzoneXYSet(Result<(), String>),
    /// Request per-axis outer deadzone for a device
    AnalogOuterDeadzoneXYRequested(String),
    /// Per-axis outer deadzone loaded (device_id, (x_pct, y_pct))
    AnalogOuterDeadzoneXYLoaded(String, (u8, u8)),
    /// Set per-axis outer deadzone (device_id, x_pct, y_pct)
    SetAnalogOuterDeadzoneXY(String, u8, u8),
    /// Per-axis outer deadzone set result
    AnalogOuterDeadzoneXYSet(Result<(), String>),

    // LED Configuration Management
    /// Open LED configuration dialog for device
    OpenLedConfig(String),
    /// Close LED configuration dialog
    CloseLedConfig,
    /// Select LED zone for color editing
    SelectLedZone(LedZone),
    /// Set LED color (device_id, zone, red, green, blue)
    SetLedColor(String, LedZone, u8, u8, u8),
    /// LED color set result
    LedColorSet(Result<(), String>),
    /// Set LED brightness (device_id, zone_opt, brightness)
    SetLedBrightness(String, Option<LedZone>, u8),
    /// LED brightness set result
    LedBrightnessSet(Result<(), String>),
    /// Set LED pattern (device_id, pattern)
    SetLedPattern(String, LedPattern),
    /// LED pattern set result
    LedPatternSet(Result<(), String>),
    /// Request LED state refresh for device
    RefreshLedState(String),
    /// LED state loaded (device_id, colors)
    LedStateLoaded(String, Result<HashMap<LedZone, (u8, u8, u8)>, String>),
    /// RGB slider changed (red, green, blue)
    LedSliderChanged(u8, u8, u8),

    // Focus Tracking
    /// Start focus tracking after daemon connection confirmed
    StartFocusTracking,
    /// Focus tracking started successfully
    FocusTrackingStarted(Result<bool, String>),
    /// Focus change event received from tracker
    FocusChanged(String, Option<String>), // (app_id, window_title)

    // Auto-Switch Rules Management
    /// Open auto-switch rules view for a device
    ShowAutoSwitchRules(String),
    /// Close auto-switch rules view
    CloseAutoSwitchRules,
    /// Load auto-switch rules for a device
    LoadAutoSwitchRules(String),
    /// Auto-switch rules loaded (device_id, rules)
    AutoSwitchRulesLoaded(Result<Vec<AutoSwitchRule>, String>),
    /// Start editing a rule (index in list)
    EditAutoSwitchRule(usize),
    /// Update new rule app_id input
    AutoSwitchAppIdChanged(String),
    /// Update new rule profile_name input
    AutoSwitchProfileNameChanged(String),
    /// Update new rule layer_id input
    AutoSwitchLayerIdChanged(String),
    /// Use current focused app as app_id
    AutoSwitchUseCurrentApp,
    /// Save the current rule (add or update)
    SaveAutoSwitchRule,
    /// Delete a rule
    DeleteAutoSwitchRule(usize),

    // Hotkey Bindings Management
    /// Open hotkey bindings view for a device
    ShowHotkeyBindings(String),
    /// Close hotkey bindings view
    CloseHotkeyBindings,
    /// Load hotkey bindings for a device
    LoadHotkeyBindings(String),
    /// Hotkey bindings loaded result
    HotkeyBindingsLoaded(Result<Vec<HotkeyBinding>, String>),
    /// Start editing a binding (index in list)
    EditHotkeyBinding(usize),
    /// Toggle modifier checkbox (modifier_name)
    ToggleHotkeyModifier(String),
    /// Update new binding key input
    HotkeyKeyChanged(String),
    /// Update new binding profile_name input
    HotkeyProfileNameChanged(String),
    /// Update new binding layer_id input
    HotkeyLayerIdChanged(String),
    /// Save the current binding (add or update)
    SaveHotkeyBinding,
    /// Delete a binding
    DeleteHotkeyBinding(usize),
    /// Hotkey bindings updated after delete
    HotkeyBindingsUpdated(Vec<HotkeyBinding>),

    // Analog Calibration Management
    /// Open analog calibration view for a device and layer
    OpenAnalogCalibration {
        device_id: String,
        layer_id: usize,
    },
    /// Analog calibration field changed
    AnalogDeadzoneChanged(f32),
    AnalogDeadzoneShapeChanged(DeadzoneShape),
    AnalogSensitivityChanged(f32),
    AnalogSensitivityCurveChanged(SensitivityCurve),
    AnalogRangeMinChanged(i32),
    AnalogRangeMaxChanged(i32),
    AnalogInvertXToggled(bool),
    AnalogInvertYToggled(bool),
    /// Analog mode changed
    AnalogModeChanged(AnalogMode),
    /// Camera output mode changed
    CameraModeChanged(CameraOutputMode),
    /// Apply calibration changes
    ApplyAnalogCalibration,
    /// Analog calibration loaded
    AnalogCalibrationLoaded(Result<aethermap_common::AnalogCalibrationConfig, String>),
    /// Analog calibration applied
    AnalogCalibrationApplied(Result<(), String>),
    /// Close analog calibration view
    CloseAnalogCalibration,
    /// Analog input updated (streaming from daemon)
    AnalogInputUpdated(f32, f32), // (x, y)
}

// Reserved for future use
#[allow(dead_code)]
pub enum _FutureMessage {
    DismissNotification,
}

impl Application for State {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let initial_state = State::default();
        let initial_commands = Command::batch([
            Command::perform(async { Message::CheckDaemonConnection }, |msg| msg),
            Command::perform(async { Message::LoadDevices }, |msg| msg),
            Command::perform(async { Message::LoadMacroSettings }, |msg| msg),
        ]);
        (initial_state, initial_commands)
    }

    fn title(&self) -> String {
        String::from("Aethermap")
    }

    fn theme(&self) -> Theme {
        self.current_theme.clone()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ThemeChanged(theme) => {
                self.current_theme = theme;
                Command::none()
            }
            Message::SwitchTab(tab) => {
                self.active_tab = tab;
                Command::none()
            }
            Message::SelectDevice(idx) => {
                self.selected_device = Some(idx);
                // Load analog settings for the selected device if it has analog stick
                if let Some(device) = self.devices.get(idx) {
                    let device_id = format!("{:04x}:{:04x}", device.vendor_id, device.product_id);
                    if device.device_type == DeviceType::Gamepad || device.device_type == DeviceType::Keypad {
                        let device_id_clone1 = device_id.clone();
                        let device_id_clone2 = device_id.clone();
                        let device_id_clone3 = device_id.clone();
                        return Command::batch(vec![
                            Command::none(),
                            Command::perform(async move { device_id_clone1 }, |id| Message::AnalogDpadModeRequested(id)),
                            Command::perform(async move { device_id_clone2 }, |id| Message::AnalogDeadzoneXYRequested(id)),
                            Command::perform(async move { device_id_clone3 }, |id| Message::AnalogOuterDeadzoneXYRequested(id)),
                        ]);
                    }
                }
                Command::none()
            }
            Message::CheckDaemonConnection => {
                let socket_path = self.socket_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.connect().await.is_ok()
                    },
                    Message::DaemonStatusChanged,
                )
            }
            Message::DaemonStatusChanged(connected) => {
                self.daemon_connected = connected;
                if connected {
                    self.add_notification("Connected to daemon", false);
                    // Start focus tracking after successful daemon connection
                    Command::perform(async { Message::StartFocusTracking }, |msg| msg)
                } else {
                    self.add_notification("Daemon not running - start aethermapd", true);
                    Command::none()
                }
            }
            Message::StartFocusTracking => {
                // Spawn async task to initialize and start focus tracking
                // We create a simple check for portal availability
                Command::perform(
                    async move {
                        // Check if WAYLAND_DISPLAY is set (basic portal check)
                        let wayland_available = std::env::var("WAYLAND_DISPLAY").is_ok();
                        if wayland_available {
                            tracing::info!("Focus tracking available (Wayland detected)");
                        } else {
                            tracing::warn!("Focus tracking unavailable (not on Wayland)");
                        }
                        wayland_available
                    },
                    |available| Message::FocusTrackingStarted(Ok(available)),
                )
            }
            Message::FocusTrackingStarted(Ok(available)) => {
                self.focus_tracking_active = available;
                if available {
                    self.add_notification("Focus tracking enabled", false);
                } else {
                    self.add_notification("Focus tracking unavailable (portal not connected)", true);
                }
                Command::none()
            }
            Message::FocusTrackingStarted(Err(e)) => {
                self.add_notification(&format!("Focus tracking error: {}", e), true);
                self.focus_tracking_active = false;
                Command::none()
            }
            Message::FocusChanged(app_id, window_title) => {
                // Update current focus for auto-switch rule creation UI
                self.current_focus = Some(app_id.clone());
                // Send focus change to daemon for auto-profile switching
                let socket_path = self.socket_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.send_focus_change(app_id, window_title).await
                    },
                    |result| match result {
                        Ok(()) => Message::TickAnimations, // Silent success
                        Err(e) => Message::ProfileError(format!("Focus change failed: {}", e)),
                    },
                )
            }

            // Auto-Switch Rules Management
            Message::ShowAutoSwitchRules(device_id) => {
                self.auto_switch_view = Some(AutoSwitchRulesView {
                    device_id: device_id.clone(),
                    rules: Vec::new(),
                    editing_rule: None,
                    new_app_id: String::new(),
                    new_profile_name: String::new(),
                    new_layer_id: String::new(),
                });
                // Load rules from daemon
                let device_id_clone = device_id.clone();
                Command::perform(
                    async move { device_id_clone },
                    |id| Message::LoadAutoSwitchRules(id)
                )
            }
            Message::CloseAutoSwitchRules => {
                self.auto_switch_view = None;
                Command::none()
            }
            Message::LoadAutoSwitchRules(_device_id) => {
                let socket_path = self.socket_path.clone();
                Command::perform(
                    async move {
                        let client = IpcClient::with_socket_path(&socket_path);
                        let request = Request::GetAutoSwitchRules;
                        match client.send(&request).await {
                            Ok(Response::AutoSwitchRules { rules }) => {
                                // Convert common::AutoSwitchRule to gui::AutoSwitchRule
                                Ok(rules.into_iter().map(|r| AutoSwitchRule {
                                    app_id: r.app_id,
                                    profile_name: r.profile_name,
                                    device_id: r.device_id,
                                    layer_id: r.layer_id,
                                }).collect())
                            }
                            Ok(Response::Error(msg)) => Err(msg),
                            Err(e) => Err(format!("IPC error: {}", e)),
                            _ => Err("Unexpected response".to_string()),
                        }
                    },
                    Message::AutoSwitchRulesLoaded,
                )
            }
            Message::AutoSwitchRulesLoaded(Ok(rules)) => {
                self.auto_switch_view.as_mut().map(|view| {
                    view.rules = rules;
                });
                Command::none()
            }
            Message::AutoSwitchRulesLoaded(Err(error)) => {
                self.add_notification(&format!("Failed to load auto-switch rules: {}", error), true);
                Command::none()
            }
            Message::EditAutoSwitchRule(index) => {
                if let Some(view) = &self.auto_switch_view {
                    if let Some(rule) = view.rules.get(index) {
                        self.auto_switch_view = Some(AutoSwitchRulesView {
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
            Message::AutoSwitchAppIdChanged(value) => {
                self.auto_switch_view.as_mut().map(|view| {
                    view.new_app_id = value;
                });
                Command::none()
            }
            Message::AutoSwitchProfileNameChanged(value) => {
                self.auto_switch_view.as_mut().map(|view| {
                    view.new_profile_name = value;
                });
                Command::none()
            }
            Message::AutoSwitchLayerIdChanged(value) => {
                self.auto_switch_view.as_mut().map(|view| {
                    view.new_layer_id = value;
                });
                Command::none()
            }
            Message::AutoSwitchUseCurrentApp => {
                if let Some(ref focus) = self.current_focus {
                    self.auto_switch_view.as_mut().map(|view| {
                        view.new_app_id = focus.clone();
                    });
                }
                Command::none()
            }
            Message::SaveAutoSwitchRule => {
                if let Some(mut view) = self.auto_switch_view.clone() {
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
                    let socket_path = self.socket_path.clone();

                    // Update local state immediately
                    self.auto_switch_view = Some(view);

                    // Sync to daemon
                    Command::perform(
                        async move {
                            // Convert GUI AutoSwitchRule to common AutoSwitchRule
                            let common_rules: Vec<CommonAutoSwitchRule> = rules.into_iter()
                                .map(|r| CommonAutoSwitchRule {
                                    app_id: r.app_id,
                                    profile_name: r.profile_name,
                                    device_id: r.device_id,
                                    layer_id: r.layer_id,
                                })
                                .collect();

                            let client = IpcClient::with_socket_path(socket_path);
                            let request = Request::SetAutoSwitchRules { rules: common_rules };
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
                        }
                    )
                } else {
                    Command::none()
                }
            }
            Message::DeleteAutoSwitchRule(index) => {
                if let Some(view) = self.auto_switch_view.clone() {
                    if index < view.rules.len() {
                        let mut rules = view.rules.clone();
                        rules.remove(index);
                        let socket_path = self.socket_path.clone();

                        // Update local state immediately
                        self.auto_switch_view.as_mut().map(|v| v.rules = rules.clone());

                        // Sync to daemon
                        return Command::perform(
                            async move {
                                // Convert GUI AutoSwitchRule to common AutoSwitchRule
                                let common_rules: Vec<CommonAutoSwitchRule> = rules.into_iter()
                                    .map(|r| CommonAutoSwitchRule {
                                        app_id: r.app_id,
                                        profile_name: r.profile_name,
                                        device_id: r.device_id,
                                        layer_id: r.layer_id,
                                    })
                                    .collect();

                                let client = IpcClient::with_socket_path(&socket_path);
                                let request = Request::SetAutoSwitchRules { rules: common_rules };
                                match client.send(&request).await {
                                    Ok(Response::AutoSwitchRulesAck) => Ok(()),
                                    Ok(Response::Error(msg)) => Err(msg),
                                    Err(e) => Err(format!("IPC error: {}", e)),
                                    _ => Err("Unexpected response".to_string()),
                                }
                            },
                            |result| match result {
                                Ok(()) => Message::ShowNotification("Rule deleted".to_string(), false),
                                Err(e) => Message::ShowNotification(format!("Failed to delete rule: {}", e), true),
                            }
                        );
                    }
                }
                Command::none()
            }

            // Hotkey Bindings Management
            Message::ShowHotkeyBindings(device_id) => {
                self.hotkey_view = Some(HotkeyBindingsView {
                    device_id: device_id.clone(),
                    bindings: Vec::new(),
                    editing_binding: None,
                    new_modifiers: Vec::new(),
                    new_key: String::new(),
                    new_profile_name: String::new(),
                    new_layer_id: String::new(),
                });
                // Load bindings from daemon
                let device_id_clone = device_id.clone();
                Command::perform(
                    async move { device_id_clone },
                    |id| Message::LoadHotkeyBindings(id)
                )
            }
            Message::CloseHotkeyBindings => {
                self.hotkey_view = None;
                Command::none()
            }
            Message::LoadHotkeyBindings(device_id) => {
                let socket_path = self.socket_path.clone();
                Command::perform(
                    async move {
                        let client = IpcClient::with_socket_path(&socket_path);
                        let request = Request::ListHotkeys { device_id };
                        match client.send(&request).await {
                            Ok(Response::HotkeyList { bindings, .. }) => {
                                // Convert common::HotkeyBinding to gui::HotkeyBinding
                                Ok(bindings.into_iter().map(|b| HotkeyBinding {
                                    modifiers: b.modifiers,
                                    key: b.key,
                                    profile_name: b.profile_name,
                                    device_id: b.device_id,
                                    layer_id: b.layer_id,
                                }).collect())
                            }
                            Ok(Response::Error(msg)) => Err(msg),
                            Err(e) => Err(format!("IPC error: {}", e)),
                            _ => Err("Unexpected response".to_string()),
                        }
                    },
                    Message::HotkeyBindingsLoaded,
                )
            }
            Message::HotkeyBindingsLoaded(Ok(bindings)) => {
                if let Some(view) = &mut self.hotkey_view {
                    view.bindings = bindings;
                }
                Command::none()
            }
            Message::HotkeyBindingsLoaded(Err(error)) => {
                self.add_notification(&format!("Failed to load hotkey bindings: {}", error), true);
                Command::none()
            }
            Message::EditHotkeyBinding(index) => {
                if let Some(view) = &self.hotkey_view {
                    if let Some(binding) = view.bindings.get(index) {
                        self.hotkey_view = Some(HotkeyBindingsView {
                            device_id: view.device_id.clone(),
                            bindings: view.bindings.clone(),
                            editing_binding: Some(index),
                            new_modifiers: binding.modifiers.clone(),
                            new_key: binding.key.clone(),
                            new_profile_name: binding.profile_name.clone(),
                            new_layer_id: binding.layer_id.map(|id| id.to_string()).unwrap_or_default(),
                        });
                    }
                }
                Command::none()
            }
            Message::ToggleHotkeyModifier(modifier) => {
                self.hotkey_view.as_mut().map(|view| {
                    if view.new_modifiers.contains(&modifier) {
                        view.new_modifiers.retain(|m| m != &modifier);
                    } else {
                        view.new_modifiers.push(modifier);
                    }
                });
                Command::none()
            }
            Message::HotkeyKeyChanged(value) => {
                self.hotkey_view.as_mut().map(|view| {
                    view.new_key = value;
                });
                Command::none()
            }
            Message::HotkeyProfileNameChanged(value) => {
                self.hotkey_view.as_mut().map(|view| {
                    view.new_profile_name = value;
                });
                Command::none()
            }
            Message::HotkeyLayerIdChanged(value) => {
                self.hotkey_view.as_mut().map(|view| {
                    view.new_layer_id = value;
                });
                Command::none()
            }
            Message::SaveHotkeyBinding => {
                if let Some(view) = &self.hotkey_view {
                    let device_id = view.device_id.clone();
                    let binding = CommonHotkeyBinding {
                        modifiers: view.new_modifiers.clone(),
                        key: view.new_key.clone(),
                        profile_name: view.new_profile_name.clone(),
                        device_id: Some(view.device_id.clone()),
                        layer_id: if view.new_layer_id.is_empty() { None } else { view.new_layer_id.parse().ok() },
                    };
                    let socket_path = self.socket_path.clone();

                    // Update local state immediately
                    if let Some(local_view) = &self.hotkey_view {
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
                        self.hotkey_view = Some(updated_view);
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
                        }
                    );
                }
                Command::none()
            }
            Message::DeleteHotkeyBinding(index) => {
                if let Some(view) = &self.hotkey_view {
                    if index < view.bindings.len() {
                        let device_id = view.device_id.clone();
                        let binding = view.bindings[index].clone();
                        let socket_path = self.socket_path.clone();

                        // Update local state immediately
                        let updated_bindings = view.bindings.iter()
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
                            move |result| {
                                if result.is_ok() {
                                    Message::HotkeyBindingsUpdated(updated_bindings)
                                } else {
                                    let err_msg = result.unwrap_err();
                                    Message::ShowNotification(format!("Failed to delete hotkey: {}", err_msg), true)
                                }
                            }
                        );
                    }
                }
                Command::none()
            }
            Message::HotkeyBindingsUpdated(bindings) => {
                if let Some(view) = &mut self.hotkey_view {
                    view.bindings = bindings;
                }
                self.add_notification("Hotkey deleted", false);
                Command::none()
            }

            // Analog Calibration Management
            Message::OpenAnalogCalibration { device_id, layer_id } => {
                // Create the view with loading state
                self.analog_calibration_view = Some(AnalogCalibrationView {
                    device_id: device_id.clone(),
                    layer_id,
                    calibration: CalibrationConfig::default(),
                    deadzone_shape_selected: DeadzoneShape::Circular,
                    sensitivity_curve_selected: SensitivityCurve::Linear,
                    analog_mode_selected: AnalogMode::Disabled,
                    camera_mode_selected: CameraOutputMode::Scroll,
                    invert_x_checked: false,
                    invert_y_checked: false,
                    stick_x: 0.0,
                    stick_y: 0.0,
                    loading: true,
                    error: None,
                    last_visualizer_update: Instant::now(),
                    visualizer_cache: Arc::new(iced::widget::canvas::Cache::default()),
                });

                // Load calibration from daemon
                let device_id_clone = device_id.clone();
                let socket_path = self.socket_path.clone();

                // Subscribe to analog input updates
                let device_id_subscribe = device_id.clone();
                let socket_path_subscribe = self.socket_path.clone();

                Command::batch(vec![
                    // Subscribe to analog input updates
                    Command::perform(
                        async move {
                            let client = crate::ipc::IpcClient::new(socket_path_subscribe);
                            client.subscribe_analog_input(&device_id_subscribe).await
                        },
                        |result| match result {
                            Ok(_) => Message::ShowNotification("Subscribed to analog input".to_string(), false),
                            Err(e) => Message::ShowNotification(format!("Subscription failed: {}", e), true),
                        },
                    ),
                    // Load calibration data
                    Command::perform(
                        async move {
                            let client = crate::ipc::IpcClient::new(socket_path);
                            client.get_analog_calibration(&device_id_clone, layer_id).await
                        },
                        Message::AnalogCalibrationLoaded,
                    ),
                ])
            }
            Message::AnalogCalibrationLoaded(Ok(calibration)) => {
                if let Some(view) = &mut self.analog_calibration_view {
                    // Convert common config to local CalibrationConfig
                    view.calibration = CalibrationConfig {
                        deadzone: calibration.deadzone,
                        deadzone_shape: calibration.deadzone_shape.clone(),
                        sensitivity: calibration.sensitivity.clone(),
                        sensitivity_multiplier: calibration.sensitivity_multiplier,
                        range_min: calibration.range_min,
                        range_max: calibration.range_max,
                        invert_x: calibration.invert_x,
                        invert_y: calibration.invert_y,
                        exponent: calibration.exponent,
                    };
                    view.loading = false;

                    // Update selections from loaded calibration
                    view.deadzone_shape_selected = match calibration.deadzone_shape.as_str() {
                        "circular" => DeadzoneShape::Circular,
                        "square" => DeadzoneShape::Square,
                        _ => DeadzoneShape::Circular,
                    };
                    view.sensitivity_curve_selected = match calibration.sensitivity.as_str() {
                        "linear" => SensitivityCurve::Linear,
                        "quadratic" => SensitivityCurve::Quadratic,
                        "exponential" => SensitivityCurve::Exponential,
                        _ => SensitivityCurve::Linear,
                    };
                    view.invert_x_checked = calibration.invert_x;
                    view.invert_y_checked = calibration.invert_y;
                }
                Command::none()
            }
            Message::AnalogCalibrationLoaded(Err(error)) => {
                if let Some(view) = &mut self.analog_calibration_view {
                    view.error = Some(error);
                    view.loading = false;
                }
                Command::none()
            }
            Message::AnalogDeadzoneChanged(value) => {
                if let Some(view) = &mut self.analog_calibration_view {
                    view.calibration.deadzone = value;
                    // Clear cache so deadzone redraws with new size
                    view.visualizer_cache.clear();
                }
                Command::none()
            }
            Message::AnalogDeadzoneShapeChanged(shape) => {
                if let Some(view) = &mut self.analog_calibration_view {
                    view.deadzone_shape_selected = shape;
                    view.calibration.deadzone_shape = shape.to_string().to_lowercase();
                    // Clear cache so deadzone redraws with new shape
                    view.visualizer_cache.clear();
                }
                Command::none()
            }
            Message::AnalogSensitivityChanged(value) => {
                if let Some(view) = &mut self.analog_calibration_view {
                    view.calibration.sensitivity_multiplier = value;
                }
                Command::none()
            }
            Message::AnalogSensitivityCurveChanged(curve) => {
                if let Some(view) = &mut self.analog_calibration_view {
                    view.sensitivity_curve_selected = curve;
                    view.calibration.sensitivity = curve.to_string().to_lowercase();
                }
                Command::none()
            }
            Message::AnalogRangeMinChanged(value) => {
                if let Some(view) = &mut self.analog_calibration_view {
                    view.calibration.range_min = value;
                }
                Command::none()
            }
            Message::AnalogRangeMaxChanged(value) => {
                if let Some(view) = &mut self.analog_calibration_view {
                    view.calibration.range_max = value;
                }
                Command::none()
            }
            Message::AnalogInvertXToggled(checked) => {
                if let Some(view) = &mut self.analog_calibration_view {
                    view.invert_x_checked = checked;
                    view.calibration.invert_x = checked;
                }
                Command::none()
            }
            Message::AnalogInvertYToggled(checked) => {
                if let Some(view) = &mut self.analog_calibration_view {
                    view.invert_y_checked = checked;
                    view.calibration.invert_y = checked;
                }
                Command::none()
            }
            Message::AnalogModeChanged(mode) => {
                if let Some(view) = &mut self.analog_calibration_view {
                    view.analog_mode_selected = mode;
                }
                Command::none()
            }
            Message::CameraModeChanged(mode) => {
                if let Some(view) = &mut self.analog_calibration_view {
                    view.camera_mode_selected = mode;
                }
                Command::none()
            }
            Message::ApplyAnalogCalibration => {
                if let Some(view) = self.analog_calibration_view.clone() {
                    let device_id = view.device_id.clone();
                    let layer_id = view.layer_id;
                    let calibration = aethermap_common::AnalogCalibrationConfig {
                        deadzone: view.calibration.deadzone,
                        deadzone_shape: view.calibration.deadzone_shape.clone(),
                        sensitivity: view.calibration.sensitivity.clone(),
                        sensitivity_multiplier: view.calibration.sensitivity_multiplier,
                        range_min: view.calibration.range_min,
                        range_max: view.calibration.range_max,
                        invert_x: view.calibration.invert_x,
                        invert_y: view.calibration.invert_y,
                        exponent: view.calibration.exponent,
                        analog_mode: view.analog_mode_selected,
                        camera_output_mode: if view.analog_mode_selected == aethermap_common::AnalogMode::Camera {
                            Some(view.camera_mode_selected)
                        } else {
                            None
                        },
                    };
                    let socket_path = self.socket_path.clone();

                    return Command::perform(
                        async move {
                            let client = crate::ipc::IpcClient::new(socket_path);
                            client.set_analog_calibration(&device_id, layer_id, calibration).await
                                .map_err(|e| e.to_string())
                        },
                        Message::AnalogCalibrationApplied,
                    );
                }
                Command::none()
            }
            Message::AnalogCalibrationApplied(Ok(())) => {
                self.add_notification("Calibration saved successfully", false);
                Command::none()
            }
            Message::AnalogCalibrationApplied(Err(error)) => {
                self.add_notification(&format!("Failed to save calibration: {}", error), true);
                if let Some(view) = &mut self.analog_calibration_view {
                    let mut view = view.clone();
                    view.error = Some(error);
                    self.analog_calibration_view = Some(view);
                }
                Command::none()
            }
            Message::CloseAnalogCalibration => {
                // Unsubscribe from analog input updates
                let device_id = self.analog_calibration_view.as_ref()
                    .map(|v| v.device_id.clone())
                    .unwrap_or_default();
                let socket_path = self.socket_path.clone();

                self.analog_calibration_view = None;

                // Unsubscribe is fire-and-forget - we don't need to wait for result
                // Spawn a background task to handle it
                let _ = std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        if let Err(e) = client.unsubscribe_analog_input(&device_id).await {
                            eprintln!("Failed to unsubscribe: {}", e);
                        }
                    });
                });

                Command::none()
            }
            Message::AnalogInputUpdated(x, y) => {
                // Update analog calibration view stick position with throttling
                // Throttle to ~30 FPS (33ms between updates) to prevent overwhelming the GUI
                if let Some(view) = &mut self.analog_calibration_view {
                    if view.last_visualizer_update.elapsed() >= Duration::from_millis(33) {
                        view.stick_x = x;
                        view.stick_y = y;
                        view.last_visualizer_update = Instant::now();
                        Command::none() // Triggers redraw
                    } else {
                        Command::none() // Skip redraw, no state change
                    }
                } else {
                    Command::none()
                }
            }

            Message::LoadDevices => {
                let socket_path = self.socket_path.clone();
                self.loading = true;
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.get_devices().await.map_err(|e| e.to_string())
                    },
                    Message::DevicesLoaded,
                )
            }
            Message::DevicesLoaded(Ok(devices)) => {
                let count = devices.len();
                self.devices = devices;
                self.loading = false;
                self.add_notification(&format!("Found {} devices", count), false);
                Command::perform(async { Message::LoadMacros }, |msg| msg)
            }
            Message::DevicesLoaded(Err(e)) => {
                self.loading = false;
                self.add_notification(&format!("Error: {}", e), true);
                Command::none()
            }
            Message::LoadMacros => {
                let socket_path = self.socket_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.list_macros().await.map_err(|e| e.to_string())
                    },
                    Message::MacrosLoaded,
                )
            }
            Message::MacrosLoaded(Ok(macros)) => {
                let count = macros.len();
                self.macros = macros;
                self.add_notification(&format!("Loaded {} macros", count), false);
                Command::none()
            }
            Message::MacrosLoaded(Err(e)) => {
                self.add_notification(&format!("Error loading macros: {}", e), true);
                Command::none()
            }
            Message::LoadMacroSettings => {
                let socket_path = self.socket_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.get_macro_settings().await.map_err(|e| e.to_string())
                    },
                    Message::MacroSettingsLoaded,
                )
            }
            Message::MacroSettingsLoaded(Ok(settings)) => {
                self.macro_settings = settings;
                Command::none()
            }
            Message::MacroSettingsLoaded(Err(e)) => {
                self.add_notification(&format!("Error loading macro settings: {}", e), true);
                Command::none()
            }
            Message::SetMacroSettings(settings) => {
                let socket_path = self.socket_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.set_macro_settings(settings).await.map_err(|e| e.to_string())
                    },
                    |result| match result {
                        Ok(_) => Message::TickAnimations, // Silent success
                        Err(e) => Message::ShowNotification(format!("Failed to save settings: {}", e), true),
                    }
                )
            }
            Message::LatencyChanged(ms) => {
                self.macro_settings.latency_offset_ms = ms;
                let settings = self.macro_settings.clone();
                Command::perform(async move { Message::SetMacroSettings(settings) }, |msg| msg)
            }
            Message::JitterChanged(pct) => {
                self.macro_settings.jitter_pct = pct;
                let settings = self.macro_settings.clone();
                Command::perform(async move { Message::SetMacroSettings(settings) }, |msg| msg)
            }
            Message::CaptureMouseToggled(enabled) => {
                self.macro_settings.capture_mouse = enabled;
                let settings = self.macro_settings.clone();
                Command::perform(async move { Message::SetMacroSettings(settings) }, |msg| msg)
            }
            Message::PlayMacro(macro_name) => {
                let socket_path = self.socket_path.clone();
                let name = macro_name.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.test_macro(&name).await.map(|_| name).map_err(|e| e.to_string())
                    },
                    Message::MacroPlayed,
                )
            }
            Message::MacroPlayed(Ok(name)) => {
                self.add_notification(&format!("Played macro: {}", name), false);
                Command::none()
            }
            Message::MacroPlayed(Err(e)) => {
                self.add_notification(&format!("Failed to play: {}", e), true);
                Command::none()
            }
            Message::UpdateMacroName(name) => {
                self.new_macro_name = name;
                Command::none()
            }
            Message::UpdateProfileName(name) => {
                self.profile_name = name;
                Command::none()
            }
            Message::StartRecording => {
                if self.new_macro_name.trim().is_empty() {
                    self.add_notification("Enter a macro name first", true);
                    return Command::none();
                }
                if self.grabbed_devices.is_empty() {
                    self.add_notification("Grab a device first", true);
                    return Command::none();
                }

                let device_path = self.grabbed_devices.iter().next().unwrap().clone();
                let socket_path = self.socket_path.clone();
                let macro_name = self.new_macro_name.clone();
                let capture_mouse = self.macro_settings.capture_mouse;
                self.recording = true;
                self.recording_macro_name = Some(macro_name.clone());

                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.start_recording_macro(&device_path, &macro_name, capture_mouse)
                            .await
                            .map(|_| macro_name)
                            .map_err(|e| e.to_string())
                    },
                    Message::RecordingStarted,
                )
            }
            Message::RecordingStarted(Ok(name)) => {
                self.add_notification(&format!("Recording '{}' - Press keys now!", name), false);
                Command::none()
            }
            Message::RecordingStarted(Err(e)) => {
                self.recording = false;
                self.recording_macro_name = None;
                self.add_notification(&format!("Failed to start recording: {}", e), true);
                Command::none()
            }
            Message::StopRecording => {
                let socket_path = self.socket_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.stop_recording_macro().await.map_err(|e| e.to_string())
                    },
                    Message::RecordingStopped,
                )
            }
            Message::RecordingStopped(Ok(macro_entry)) => {
                let name = macro_entry.name.clone();
                self.macros.push(macro_entry);
                self.recording = false;
                self.recording_macro_name = None;
                self.recently_updated_macros.insert(name.clone(), Instant::now());
                self.new_macro_name.clear();
                self.add_notification(&format!("Recorded macro: {}", name), false);
                Command::none()
            }
            Message::RecordingStopped(Err(e)) => {
                self.recording = false;
                self.recording_macro_name = None;
                self.add_notification(&format!("Recording failed: {}", e), true);
                Command::none()
            }
            Message::DeleteMacro(macro_name) => {
                let socket_path = self.socket_path.clone();
                let name = macro_name.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.delete_macro(&name).await.map(|_| name).map_err(|e| e.to_string())
                    },
                    Message::MacroDeleted,
                )
            }
            Message::MacroDeleted(Ok(name)) => {
                self.macros.retain(|m| m.name != name);
                self.add_notification(&format!("Deleted: {}", name), false);
                Command::none()
            }
            Message::MacroDeleted(Err(e)) => {
                self.add_notification(&format!("Delete failed: {}", e), true);
                Command::none()
            }
            Message::SaveProfile => {
                if self.profile_name.trim().is_empty() {
                    self.add_notification("Enter a profile name", true);
                    return Command::none();
                }
                let socket_path = self.socket_path.clone();
                let name = self.profile_name.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.save_profile(&name).await.map_err(|e| e.to_string())
                    },
                    Message::ProfileSaved,
                )
            }
            Message::ProfileSaved(Ok((name, count))) => {
                self.add_notification(&format!("Saved '{}' ({} macros)", name, count), false);
                Command::none()
            }
            Message::ProfileSaved(Err(e)) => {
                self.add_notification(&format!("Save failed: {}", e), true);
                Command::none()
            }
            Message::LoadProfile => {
                if self.profile_name.trim().is_empty() {
                    self.add_notification("Enter a profile name to load", true);
                    return Command::none();
                }
                let socket_path = self.socket_path.clone();
                let name = self.profile_name.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.load_profile(&name).await.map_err(|e| e.to_string())
                    },
                    Message::ProfileLoaded,
                )
            }
            Message::ProfileLoaded(Ok((name, count))) => {
                self.add_notification(&format!("Loaded '{}' ({} macros)", name, count), false);
                Command::perform(async { Message::LoadMacros }, |msg| msg)
            }
            Message::ProfileLoaded(Err(e)) => {
                self.add_notification(&format!("Load failed: {}", e), true);
                Command::none()
            }
            Message::TickAnimations => {
                let now = Instant::now();
                self.recently_updated_macros.retain(|_, timestamp| {
                    now.duration_since(*timestamp) < Duration::from_secs(3)
                });
                self.recording_pulse = !self.recording_pulse;
                // Auto-dismiss old notifications
                while let Some(notif) = self.notifications.front() {
                    if now.duration_since(notif.timestamp) > Duration::from_secs(5) {
                        self.notifications.pop_front();
                    } else {
                        break;
                    }
                }
                Command::none()
            }
            Message::ShowNotification(message, is_error) => {
                self.add_notification(&message, is_error);
                Command::none()
            }
            Message::GrabDevice(device_path) => {
                let socket_path = self.socket_path.clone();
                let path_clone = device_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.grab_device(&path_clone).await.map(|_| path_clone).map_err(|e| e.to_string())
                    },
                    Message::DeviceGrabbed,
                )
            }
            Message::UngrabDevice(device_path) => {
                let socket_path = self.socket_path.clone();
                let path_clone = device_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.ungrab_device(&path_clone).await.map(|_| path_clone).map_err(|e| e.to_string())
                    },
                    Message::DeviceUngrabbed,
                )
            }
            Message::DeviceGrabbed(Ok(device_path)) => {
                self.grabbed_devices.insert(device_path.clone());
                if let Some(idx) = self.devices.iter().position(|d| d.path.to_string_lossy() == device_path) {
                    self.selected_device = Some(idx);
                }
                self.add_notification("Device grabbed - ready for recording", false);
                Command::none()
            }
            Message::DeviceGrabbed(Err(e)) => {
                self.add_notification(&format!("Grab failed: {}", e), true);
                Command::none()
            }
            Message::DeviceUngrabbed(Ok(device_path)) => {
                self.grabbed_devices.remove(&device_path);
                self.add_notification("Device released", false);
                Command::none()
            }
            Message::DeviceUngrabbed(Err(e)) => {
                self.add_notification(&format!("Release failed: {}", e), true);
                Command::none()
            }
            Message::LoadDeviceProfiles(device_id) => {
                let socket_path = self.socket_path.clone();
                let id = device_id.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        (id.clone(), client.get_device_profiles(id).await)
                    },
                    |(device_id, result)| Message::DeviceProfilesLoaded(
                        device_id,
                        result.map_err(|e| e.to_string())
                    )
                )
            }
            Message::DeviceProfilesLoaded(device_id, Ok(profiles)) => {
                self.device_profiles.insert(device_id.clone(), profiles);
                self.add_notification(&format!("Loaded {} profiles for {}", self.device_profiles.get(&device_id).map(|p| p.len()).unwrap_or(0), device_id), false);
                Command::none()
            }
            Message::DeviceProfilesLoaded(_device_id, Err(e)) => {
                self.add_notification(&format!("Failed to load device profiles: {}", e), true);
                Command::none()
            }
            Message::ActivateProfile(device_id, profile_name) => {
                let socket_path = self.socket_path.clone();
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
                    }
                )
            }
            Message::ProfileActivated(device_id, profile_name) => {
                self.active_profiles.insert(device_id.clone(), profile_name.clone());
                self.add_notification(&format!("Activated profile '{}' on {}", profile_name, device_id), false);
                Command::none()
            }
            Message::DeactivateProfile(device_id) => {
                let socket_path = self.socket_path.clone();
                let id = device_id.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.deactivate_profile(id.clone()).await
                    },
                    move |result| match result {
                        Ok(()) => Message::ProfileDeactivated(device_id),
                        Err(e) => Message::ProfileError(format!("Failed to deactivate profile: {}", e)),
                    }
                )
            }
            Message::ProfileDeactivated(device_id) => {
                self.active_profiles.remove(&device_id);
                self.add_notification(&format!("Deactivated profile on {}", device_id), false);
                Command::none()
            }
            Message::ProfileError(msg) => {
                self.add_notification(&msg, true);
                Command::none()
            }
            Message::LoadRemapProfiles(device_path) => {
                let socket_path = self.socket_path.clone();
                let path = device_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        (path.clone(), client.list_remap_profiles(&path).await)
                    },
                    |(device_path, result)| Message::RemapProfilesLoaded(
                        device_path,
                        result.map_err(|e| e.to_string())
                    )
                )
            }
            Message::RemapProfilesLoaded(device_path, Ok(profiles)) => {
                self.remap_profiles.insert(device_path.clone(), profiles);
                self.add_notification(&format!("Loaded {} remap profiles for {}", self.remap_profiles.get(&device_path).map(|p| p.len()).unwrap_or(0), device_path), false);
                Command::none()
            }
            Message::RemapProfilesLoaded(_device_path, Err(e)) => {
                self.add_notification(&format!("Failed to load remap profiles: {}", e), true);
                Command::none()
            }
            Message::ActivateRemapProfile(device_path, profile_name) => {
                let socket_path = self.socket_path.clone();
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
                    }
                )
            }
            Message::RemapProfileActivated(device_path, profile_name) => {
                self.active_remap_profiles.insert(device_path.clone(), profile_name.clone());
                self.add_notification(&format!("Activated remap profile '{}' on {}", profile_name, device_path), false);
                // Refresh active remaps after activation
                Command::perform(
                    async move { device_path.clone() },
                    |path| Message::LoadActiveRemaps(path)
                )
            }
            Message::DeactivateRemapProfile(device_path) => {
                let socket_path = self.socket_path.clone();
                let path = device_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.deactivate_remap_profile(&path).await
                    },
                    move |result| match result {
                        Ok(()) => Message::RemapProfileDeactivated(device_path),
                        Err(e) => Message::ProfileError(format!("Failed to deactivate remap profile: {}", e)),
                    }
                )
            }
            Message::RemapProfileDeactivated(device_path) => {
                self.active_remap_profiles.remove(&device_path);
                self.active_remaps.remove(&device_path);
                self.add_notification(&format!("Deactivated remap profile on {}", device_path), false);
                Command::none()
            }
            Message::LoadActiveRemaps(device_path) => {
                let socket_path = self.socket_path.clone();
                let path = device_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        (path.clone(), client.get_active_remaps(&path).await)
                    },
                    |(device_path, result)| Message::ActiveRemapsLoaded(
                        device_path,
                        result.map_err(|e| e.to_string())
                    )
                )
            }
            Message::ActiveRemapsLoaded(device_path, Ok(Some((profile_name, remaps)))) => {
                self.active_remaps.insert(device_path.clone(), (profile_name, remaps));
                Command::none()
            }
            Message::ActiveRemapsLoaded(device_path, Ok(None)) => {
                self.active_remaps.remove(&device_path);
                Command::none()
            }
            Message::ActiveRemapsLoaded(_device_path, Err(e)) => {
                self.add_notification(&format!("Failed to load active remaps: {}", e), true);
                Command::none()
            }
            Message::RecordMouseEvent { event_type, button, x, y, delta } => {
                // Mouse events are captured by daemon during recording via device grab
                // This handler is for GUI-side mouse event logging
                if self.recording {
                    // Log the mouse event for debugging/confirmation
                    let event_desc = match event_type.as_str() {
                        "button_press" => format!("Mouse button {} pressed", button.unwrap_or(0)),
                        "button_release" => format!("Mouse button {} released", button.unwrap_or(0)),
                        "movement" => format!("Mouse moved to ({}, {})", x, y),
                        "scroll" => format!("Mouse scrolled {}", delta),
                        _ => format!("Unknown mouse event: {}", event_type),
                    };
                    // Update status to show mouse event was captured
                    self.status = event_desc;
                }
                Command::none()
            }
            Message::ShowKeypadView(device_path) => {
                // Empty string means back button was pressed - clear keypad view
                if device_path.is_empty() {
                    self.device_capabilities = None;
                    self.keypad_layout.clear();
                    self.keypad_view_device = None;
                    self.selected_button = None;
                    return Command::none();
                }
                // Store the device path for keypad view
                self.keypad_view_device = Some(device_path.clone());
                // Query device capabilities and load keypad layout
                let socket_path = self.socket_path.clone();
                let path_clone = device_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        (path_clone.clone(), client.get_device_capabilities(&path_clone).await)
                    },
                    |(device_path, result)| Message::DeviceCapabilitiesLoaded(
                        device_path,
                        result.map_err(|e| e.to_string())
                    )
                )
            }
            Message::DeviceCapabilitiesLoaded(device_path, Ok(capabilities)) => {
                self.device_capabilities = Some(capabilities);
                self.keypad_layout = azeron_keypad_layout();
                // Load current remappings and update button.current_remap
                if let Some((profile_name, remaps)) = self.active_remaps.get(&device_path) {
                    for remap in remaps {
                        if let Some(button) = self.keypad_layout.iter_mut().find(|b| b.id == remap.from_key) {
                            button.current_remap = Some(remap.to_key.clone());
                        }
                    }
                    self.add_notification(&format!("Loaded remaps from profile '{}'", profile_name), false);
                }
                // Switch to Devices tab to show keypad view
                self.active_tab = Tab::Devices;
                Command::none()
            }
            Message::DeviceCapabilitiesLoaded(_device_path, Err(e)) => {
                self.add_notification(&format!("Failed to load device capabilities: {}", e), true);
                Command::none()
            }
            Message::SelectKeypadButton(button_id) => {
                self.selected_button = self.keypad_layout.iter().position(|b| b.id == button_id);
                self.status = format!("Selected button: {} - Configure remapping in device profile", button_id);
                Command::none()
            }
            Message::LayerStateChanged(device_id, layer_id) => {
                self.active_layers.insert(device_id, layer_id);
                Command::none()
            }
            Message::LayerConfigRequested(device_id) => {
                let socket_path = self.socket_path.clone();
                let id = device_id.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        (id.clone(), client.list_layers(&id).await)
                    },
                    |(device_id, result)| match result {
                        Ok(layers) => {
                            // Store layers and trigger UI refresh
                            // We'll emit LayerStateChanged for the active layer
                            if let Some(active_layer) = layers.first() {
                                Message::LayerStateChanged(device_id, active_layer.layer_id)
                            } else {
                                Message::TickAnimations // No-op refresh
                            }
                        }
                        Err(e) => Message::ProfileError(format!("Failed to load layers: {}", e)),
                    }
                )
            }
            Message::LayerActivateRequested(device_id, layer_id, mode) => {
                let socket_path = self.socket_path.clone();
                let id = device_id.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.activate_layer(&id, layer_id, mode).await
                    },
                    move |result| match result {
                        Ok(()) => Message::LayerStateChanged(device_id, layer_id),
                        Err(e) => Message::ProfileError(format!("Failed to activate layer: {}", e)),
                    }
                )
            }
            Message::LayerConfigUpdated(device_id, config) => {
                let socket_path = self.socket_path.clone();
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
                        Ok(()) => {
                            // Refresh layer list after config update
                            Message::LayerConfigRequested(device_id)
                        }
                        Err(e) => Message::ProfileError(format!("Failed to update layer config: {}", e)),
                    }
                )
            }
            Message::OpenLayerConfigDialog(device_id, layer_id) => {
                // Get current layer config if available
                let current_name = self.layer_configs
                    .get(&device_id)
                    .and_then(|layers| layers.iter().find(|l| l.layer_id == layer_id))
                    .map(|l| l.name.clone())
                    .unwrap_or_else(|| format!("Layer {}", layer_id));

                let current_mode = self.layer_configs
                    .get(&device_id)
                    .and_then(|layers| layers.iter().find(|l| l.layer_id == layer_id))
                    .map(|l| l.mode)
                    .unwrap_or(LayerMode::Hold);

                self.layer_config_dialog = Some((device_id, layer_id, current_name, current_mode));
                Command::none()
            }
            Message::LayerConfigNameChanged(name) => {
                if let Some((device_id, layer_id, _, mode)) = self.layer_config_dialog.take() {
                    self.layer_config_dialog = Some((device_id, layer_id, name, mode));
                }
                Command::none()
            }
            Message::LayerConfigModeChanged(mode) => {
                if let Some((device_id, layer_id, name, _)) = self.layer_config_dialog.take() {
                    self.layer_config_dialog = Some((device_id, layer_id, name, mode));
                }
                Command::none()
            }
            Message::SaveLayerConfig => {
                if let Some((device_id, layer_id, name, mode)) = self.layer_config_dialog.take() {
                    let config = LayerConfigInfo {
                        layer_id,
                        name: name.clone(),
                        mode,
                        remap_count: 0,
                        led_color: (0, 0, 255), // Default blue - TODO: allow GUI configuration
                        led_zone: None, // Default zone - TODO: allow GUI configuration
                    };
                    // Return LayerConfigUpdated message to handle the async save
                    Command::perform(
                        async move { (device_id, config) },
                        |(device_id, config)| Message::LayerConfigUpdated(device_id, config)
                    )
                } else {
                    Command::none()
                }
            }
            Message::CancelLayerConfig => {
                self.layer_config_dialog = None;
                Command::none()
            }
            Message::RefreshLayers => {
                // Periodic refresh of layer states for all devices
                let mut commands = Vec::new();

                // Request layer configuration refresh for devices that have profiles loaded
                for device_id in self.device_profiles.keys() {
                    let device_id = device_id.clone();
                    let socket_path = self.socket_path.clone();
                    commands.push(Command::perform(
                        async move {
                            let client = crate::ipc::IpcClient::new(socket_path);
                            (device_id.clone(), client.list_layers(&device_id).await)
                        },
                        |(device_id, result)| match result {
                            Ok(layers) => {
                                // Store layers and update active layer
                                Message::LayerListLoaded(device_id, layers)
                            }
                            Err(_) => Message::TickAnimations, // Silent fail on refresh
                        }
                    ));
                }

                // Also refresh active layer states
                for device_id in self.active_layers.keys().cloned().collect::<Vec<_>>() {
                    let device_id = device_id.clone();
                    let socket_path = self.socket_path.clone();
                    commands.push(Command::perform(
                        async move {
                            let client = crate::ipc::IpcClient::new(socket_path);
                            (device_id.clone(), client.get_active_layer(&device_id).await)
                        },
                        |(device_id, result)| match result {
                            Ok(Some(layer_id)) => {
                                Message::LayerStateChanged(device_id, layer_id)
                            }
                            _ => Message::TickAnimations,
                        }
                    ));
                }

                Command::batch(commands)
            }
            Message::LayerListLoaded(device_id, layers) => {
                self.layer_configs.insert(device_id.clone(), layers);
                Command::none()
            }

            Message::AnalogDpadModeRequested(device_id) => {
                let socket_path = self.socket_path.clone();
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
                            Message::TickAnimations // Silent fail
                        }
                    },
                )
            }

            Message::AnalogDpadModeLoaded(device_id, mode) => {
                self.analog_dpad_modes.insert(device_id, mode);
                Command::none()
            }

            Message::SetAnalogDpadMode(device_id, mode) => {
                let socket_path = self.socket_path.clone();
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

            Message::AnalogDpadModeSet(result) => {
                match result {
                    Ok(_) => {
                        // Success - D-pad mode updated
                        Command::none()
                    }
                    Err(e) => {
                        eprintln!("Failed to set D-pad mode: {}", e);
                        // Could show a toast notification here
                        Command::none()
                    }
                }
            }

            // Per-Axis Deadzone handlers
            Message::AnalogDeadzoneXYRequested(device_id) => {
                let socket_path = self.socket_path.clone();
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
                            Message::TickAnimations // Silent fail
                        }
                    },
                )
            }

            Message::AnalogDeadzoneXYLoaded(device_id, (x_pct, y_pct)) => {
                self.analog_deadzones_xy.insert(device_id, (x_pct, y_pct));
                Command::none()
            }

            Message::SetAnalogDeadzoneXY(device_id, x_pct, y_pct) => {
                let socket_path = self.socket_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.set_analog_deadzone_xy(&device_id, x_pct, y_pct).await
                    },
                    |result| match result {
                        Ok(_) => Message::AnalogDeadzoneXYSet(Ok(())),
                        Err(e) => Message::AnalogDeadzoneXYSet(Err(e)),
                    },
                )
            }

            Message::AnalogDeadzoneXYSet(result) => {
                match result {
                    Ok(_) => {
                        // Success - per-axis deadzone updated
                        Command::none()
                    }
                    Err(e) => {
                        eprintln!("Failed to set per-axis deadzone: {}", e);
                        self.add_notification(&format!("Failed to set deadzone: {}", e), true);
                        Command::none()
                    }
                }
            }

            // Per-Axis Outer Deadzone handlers
            Message::AnalogOuterDeadzoneXYRequested(device_id) => {
                let socket_path = self.socket_path.clone();
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
                            Message::TickAnimations // Silent fail
                        }
                    },
                )
            }

            Message::AnalogOuterDeadzoneXYLoaded(device_id, (x_pct, y_pct)) => {
                self.analog_outer_deadzones_xy.insert(device_id, (x_pct, y_pct));
                Command::none()
            }

            Message::SetAnalogOuterDeadzoneXY(device_id, x_pct, y_pct) => {
                let socket_path = self.socket_path.clone();
                Command::perform(
                    async move {
                        let client = crate::ipc::IpcClient::new(socket_path);
                        client.set_analog_outer_deadzone_xy(&device_id, x_pct, y_pct).await
                    },
                    |result| match result {
                        Ok(_) => Message::AnalogOuterDeadzoneXYSet(Ok(())),
                        Err(e) => Message::AnalogOuterDeadzoneXYSet(Err(e)),
                    },
                )
            }

            Message::AnalogOuterDeadzoneXYSet(result) => {
                match result {
                    Ok(_) => {
                        // Success - per-axis outer deadzone updated
                        Command::none()
                    }
                    Err(e) => {
                        eprintln!("Failed to set per-axis outer deadzone: {}", e);
                        self.add_notification(&format!("Failed to set outer deadzone: {}", e), true);
                        Command::none()
                    }
                }
            }

            // LED Configuration handlers
            Message::OpenLedConfig(device_id) => {
                self.led_config_device = Some(device_id.clone());
                self.selected_led_zone = Some(LedZone::Logo); // Default to Logo zone
                return Command::batch([
                    Command::none(),
                    Command::perform(
                        async move { device_id },
                        |device_id| Message::RefreshLedState(device_id)
                    ),
                ]);
            }

            Message::CloseLedConfig => {
                self.led_config_device = None;
                self.selected_led_zone = None;
                self.pending_led_color = None;
                Command::none()
            }

            Message::SelectLedZone(zone) => {
                self.selected_led_zone = Some(zone);
                Command::none()
            }

            Message::RefreshLedState(device_id) => {
                let socket_path = self.socket_path.clone();
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

            Message::LedStateLoaded(device_id, result) => {
                match result {
                    Ok(colors) => {
                        // Initialize LED state for device if not exists
                        let led_state = self.led_states.entry(device_id.clone()).or_default();
                        led_state.zone_colors = colors;
                        Command::none()
                    }
                    Err(e) => {
                        eprintln!("Failed to load LED state: {}", e);
                        // Silent fail - LED may not be supported
                        Command::none()
                    }
                }
            }

            Message::SetLedColor(device_id, zone, red, green, blue) => {
                let socket_path = self.socket_path.clone();
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

            Message::LedColorSet(result) => {
                match result {
                    Ok(_) => {
                        // Success - color updated
                        Command::none()
                    }
                    Err(e) => {
                        eprintln!("Failed to set LED color: {}", e);
                        self.add_notification(&format!("Failed to set LED color: {}", e), true);
                        Command::none()
                    }
                }
            }

            Message::SetLedBrightness(device_id, zone, brightness) => {
                let socket_path = self.socket_path.clone();
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

            Message::LedBrightnessSet(result) => {
                match result {
                    Ok(_) => {
                        // Success - brightness updated
                        Command::none()
                    }
                    Err(e) => {
                        eprintln!("Failed to set LED brightness: {}", e);
                        self.add_notification(&format!("Failed to set LED brightness: {}", e), true);
                        Command::none()
                    }
                }
            }

            Message::SetLedPattern(device_id, pattern) => {
                let socket_path = self.socket_path.clone();
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

            Message::LedPatternSet(result) => {
                match result {
                    Ok(_) => {
                        // Success - pattern updated
                        Command::none()
                    }
                    Err(e) => {
                        eprintln!("Failed to set LED pattern: {}", e);
                        self.add_notification(&format!("Failed to set LED pattern: {}", e), true);
                        Command::none()
                    }
                }
            }

            Message::LedSliderChanged(red, green, blue) => {
                self.pending_led_color = Some((red, green, blue));
                // If a device and zone are selected, apply the color immediately
                if let (Some(ref device_id), Some(zone)) = (&self.led_config_device, self.selected_led_zone) {
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
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = self.view_sidebar();
        let main_content = self.view_main_content();
        let status_bar = self.view_status_bar();

        let main_layout = row![
            sidebar,
            vertical_rule(1),
            column![
                main_content,
                horizontal_rule(1),
                status_bar,
            ]
            .height(Length::Fill)
        ];

        let base: Element<'_, Message> = container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        // Show layer config dialog overlay if active
        if let Some(dialog) = self.layer_config_dialog() {
            container(
                column![
                    base,
                    dialog,
                ]
                .height(Length::Fill)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else if let Some(led_dialog) = self.view_led_config() {
            // Show LED config dialog overlay if active
            container(
                column![
                    base,
                    led_dialog,
                ]
                .height(Length::Fill)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else if let Some(calib_dialog) = self.view_analog_calibration() {
            // Show analog calibration dialog overlay if active
            container(
                column![
                    base,
                    calib_dialog,
                ]
                .height(Length::Fill)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            base
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let timer = iced::time::every(Duration::from_millis(500)).map(|_| Message::TickAnimations);

        // Periodic layer state refresh (every 2 seconds)
        let layer_refresh = iced::time::every(Duration::from_secs(2))
            .map(|_| Message::RefreshLayers);

        // Subscribe to mouse events only when recording
        // Note: In iced 0.12, mouse events are handled via the runtime event stream
        // The actual mouse event capture for macros happens at the daemon level via evdev
        // This subscription tracks recording state for UI updates only
        let mouse_events = iced::event::listen_with(|event, _status| {
            match event {
                iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                    Some(Message::RecordMouseEvent {
                        event_type: "button_press".to_string(),
                        button: Some(0x110), // BTN_LEFT in evdev
                        x: 0,
                        y: 0,
                        delta: 0,
                    })
                }
                iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Right)) => {
                    Some(Message::RecordMouseEvent {
                        event_type: "button_press".to_string(),
                        button: Some(0x111), // BTN_RIGHT in evdev
                        x: 0,
                        y: 0,
                        delta: 0,
                    })
                }
                iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Middle)) => {
                    Some(Message::RecordMouseEvent {
                        event_type: "button_press".to_string(),
                        button: Some(0x112), // BTN_MIDDLE in evdev
                        x: 0,
                        y: 0,
                        delta: 0,
                    })
                }
                iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => {
                    Some(Message::RecordMouseEvent {
                        event_type: "button_release".to_string(),
                        button: Some(0),
                        x: 0,
                        y: 0,
                        delta: 0,
                    })
                }
                iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                    let scroll_delta = match delta {
                        iced::mouse::ScrollDelta::Lines { y, .. } => y as i32,
                        iced::mouse::ScrollDelta::Pixels { y, .. } => y as i32,
                    };
                    Some(Message::RecordMouseEvent {
                        event_type: "scroll".to_string(),
                        button: None,
                        x: 0,
                        y: 0,
                        delta: scroll_delta,
                    })
                }
                iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
                    // Note: Cursor movement is tracked but may be sampled at reduced rate
                    Some(Message::RecordMouseEvent {
                        event_type: "movement".to_string(),
                        button: None,
                        x: 0,
                        y: 0,
                        delta: 0,
                    })
                }
                _ => None,
            }
        });

        // Only enable mouse event subscription during recording
        let mouse_subscription = if self.recording {
            mouse_events
        } else {
            Subscription::none()
        };

        let theme_subscription = iced::subscription::unfold(
            "ashpd-theme",
            None,
            |state: Option<iced::futures::stream::BoxStream<'static, ashpd::desktop::settings::ColorScheme>>| async move {
                use ashpd::desktop::settings::{ColorScheme, Settings};
                use iced::futures::StreamExt;

                let mut stream = match state {
                    Some(s) => s,
                    None => {
                        let settings = match Settings::new().await {
                            Ok(s) => s,
                            Err(_) => return iced::futures::future::pending().await,
                        };
                        let initial = settings.color_scheme().await.unwrap_or(ColorScheme::NoPreference);
                        let theme = match initial {
                            ColorScheme::PreferDark => aether_dark(),
                            ColorScheme::PreferLight => aether_light(),
                            ColorScheme::NoPreference => aether_dark(),
                        };
                        
                        let s = match settings.receive_color_scheme_changed().await {
                            Ok(s) => s,
                            Err(_) => return (Message::ThemeChanged(theme), None),
                        };
                        return (Message::ThemeChanged(theme), Some(s.boxed()));
                    }
                };

                if let Some(scheme) = stream.next().await {
                    let theme = match scheme {
                        ColorScheme::PreferDark => aether_dark(),
                        ColorScheme::PreferLight => aether_light(),
                        ColorScheme::NoPreference => aether_dark(),
                    };
                    (Message::ThemeChanged(theme), Some(stream))
                } else {
                    iced::futures::future::pending().await
                }
            }
        );

        Subscription::batch(vec![timer, layer_refresh, mouse_subscription, theme_subscription])
    }
}

impl State {
    fn add_notification(&mut self, message: &str, is_error: bool) {
        self.notifications.push_back(Notification {
            message: message.to_string(),
            is_error,
            timestamp: Instant::now(),
        });
        self.status = message.to_string();
        self.status_history.push_back(message.to_string());
        if self.status_history.len() > 10 {
            self.status_history.pop_front();
        }
        if self.notifications.len() > 5 {
            self.notifications.pop_front();
        }
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        views::sidebar::view(self)
    }

    fn view_main_content(&self) -> Element<'_, Message> {
        let content = match self.active_tab {
            Tab::Devices => self.view_devices_tab(),
            Tab::Macros => self.view_macros_tab(),
            Tab::Profiles => self.view_profiles_tab(),
        };

        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(24)
            .into()
    }

    fn view_devices_tab(&self) -> Element<'_, Message> {
        let header = row![
            text("DEVICES").size(24),
            Space::with_width(Length::Fill),
            button("Reload")
                .on_press(Message::LoadDevices)
                .style(iced::theme::Button::Secondary),
        ]
        .align_items(Alignment::Center);

        // Show auto-switch rules view when open
        if let Some(ref view) = self.auto_switch_view {
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
                self.view_auto_switch_rules(),
            ]
            .spacing(10)
            .into();
        }

        // Show hotkey bindings view when open
        if let Some(ref view) = self.hotkey_view {
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
                self.view_hotkey_bindings(),
            ]
            .spacing(10)
            .into();
        }

        // Show keypad view when capabilities are loaded
        if self.device_capabilities.is_some() && !self.keypad_layout.is_empty() {
            // Build keypad view content
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
                self.view_azeron_keypad().into(),
            ];

            // Add profile quick toggles at the bottom if device path is available
            if let Some(ref device_path) = self.keypad_view_device {
                keypad_content.push(Space::with_height(20).into());
                keypad_content.push(
                    container(
                        column![
                            text("Quick Profile Switch").size(14),
                            Space::with_height(8),
                            self.profile_quick_toggles(device_path),
                        ]
                        .spacing(4)
                    )
                    .padding(16)
                    .width(Length::Fill)
                    .style(theme::styles::card)
                    .into()
                );
            }

            return column(keypad_content)
                .spacing(10)
                .into();
        }

        let device_list = if self.devices.is_empty() {
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
            for (idx, device) in self.devices.iter().enumerate() {
                list = list.push(self.view_device_card(device, idx));
            }
            list
        };

        column![
            header,
            Space::with_height(20),
            device_list,
        ]
        .spacing(10)
        .into()
    }

    fn view_device_card<'a>(&'a self, device: &'a DeviceInfo, idx: usize) -> Element<'a, Message> {
        let device_path = device.path.to_string_lossy().to_string();
        let is_grabbed = self.grabbed_devices.contains(&device_path);
        let is_selected = self.selected_device == Some(idx);

        // Use device_type from capability detection (not name heuristics)
        let icon = match device.device_type {
            DeviceType::Keyboard => "⌨️",
            DeviceType::Mouse => "🖱️",
            DeviceType::Gamepad => "🎮",
            DeviceType::Keypad => "🎹",
            DeviceType::Other => "📱",
        };

        let status_badge = if is_grabbed {
            container(
                text("GRABBED").size(10)
            )
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

        // Get device_id for layer operations
        let device_id = format!("{:04x}:{:04x}", device.vendor_id, device.product_id);

        // Add "Configure Keypad" button for keypad devices
        let keypad_button = if device.device_type == DeviceType::Keypad {
            Some(
                button("Configure Keypad")
                    .on_press(Message::ShowKeypadView(device_path.clone()))
                    .style(iced::theme::Button::Secondary)
            )
        } else {
            None
        };

        // Add "Configure LEDs" button for LED-capable devices (keypad/gamepad)
        let led_button = if device.device_type == DeviceType::Keypad || device.device_type == DeviceType::Gamepad {
            Some(
                button("Configure LEDs")
                    .on_press(Message::OpenLedConfig(device_id.clone()))
                    .style(iced::theme::Button::Secondary)
            )
        } else {
            None
        };

        // Add "Auto-Switch Rules" button for all devices
        let auto_switch_button = Some(
            button("Auto-Switch Rules")
                .on_press(Message::ShowAutoSwitchRules(device_id.clone()))
                .style(iced::theme::Button::Secondary)
        );

        // Add "Hotkey Bindings" button for all devices
        let hotkey_button = Some(
            button("Hotkey Bindings")
                .on_press(Message::ShowHotkeyBindings(device_id.clone()))
                .style(iced::theme::Button::Secondary)
        );

        // Add "Analog Calibration" button for devices with analog support
        let analog_button = if device.device_type == DeviceType::Keypad ||
                             device.device_type == DeviceType::Gamepad {
            Some(
                button("Analog Calibration")
                    .on_press(Message::OpenAnalogCalibration {
                        device_id: device_id.clone(),
                        layer_id: self.active_layers.get(&device_id).copied().unwrap_or(0),
                    })
                    .style(iced::theme::Button::Secondary)
            )
        } else {
            None
        };

        let card_content = column![
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
                        }).size(12).style(iced::theme::Text::Color(iced::Color::from_rgb(0.6, 0.6, 0.6))),
                    ],
                    text(format!(
                        "VID:{:04X} PID:{:04X} | {}",
                        device.vendor_id, device.product_id, device_path
                    )).size(11),
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
            views::profiles::view_profile_selector(self, device),
            views::profiles::view_remap_profile_switcher(self, &device_path),
            Space::with_height(4),
            // Profile quick toggles - horizontal row of profile buttons
            container(
                column![
                    text("Profiles").size(11).style(iced::theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5))),
                    Space::with_height(4),
                    self.profile_quick_toggles(&device_path),
                ]
                .spacing(4)
            )
            .padding([8, 0])
            .width(Length::Fill),
            Space::with_height(8),
            row![
                text("Layer:").size(12),
                Space::with_width(8),
                self.layer_indicator(&device_id),
                Space::with_width(Length::Fill),
                self.layer_activation_buttons(&device_id),
            ]
            .spacing(4)
            .align_items(Alignment::Center),
        ]
        .spacing(8);

        // Build card content with optional D-pad mode selector
        let mut card_elements: Vec<Element<'_, Message>> = vec![card_content.into()];

        // Add D-pad mode selector for devices with analog sticks
        if device.device_type == DeviceType::Gamepad || device.device_type == DeviceType::Keypad {
            let current_mode = self.analog_dpad_modes.get(&device_id).cloned().unwrap_or_else(|| "disabled".to_string());

            card_elements.push(Space::with_height(4).into());
            card_elements.push(
                row![
                    text("D-pad:").size(12),
                    Space::with_width(4),
                    button("Off")
                        .on_press(Message::SetAnalogDpadMode(device_id.clone(), "disabled".to_string()))
                        .style(if current_mode == "disabled" {
                            iced::theme::Button::Primary
                        } else {
                            iced::theme::Button::Text
                        }),
                    button("8-Way")
                        .on_press(Message::SetAnalogDpadMode(device_id.clone(), "eight_way".to_string()))
                        .style(if current_mode == "eight_way" {
                            iced::theme::Button::Primary
                        } else {
                            iced::theme::Button::Text
                        }),
                    button("4-Way")
                        .on_press(Message::SetAnalogDpadMode(device_id.clone(), "four_way".to_string()))
                        .style(if current_mode == "four_way" {
                            iced::theme::Button::Primary
                        } else {
                            iced::theme::Button::Text
                        }),
                ]
                .spacing(4)
                .align_items(Alignment::Center)
                .into()
            );

            // Add per-axis deadzone controls
            let (deadzone_x, deadzone_y) = self.analog_deadzones_xy.get(&device_id).cloned().unwrap_or((43, 43));
            let (outer_deadzone_x, outer_deadzone_y) = self.analog_outer_deadzones_xy.get(&device_id).cloned().unwrap_or((100, 100));

            card_elements.push(Space::with_height(8).into());

            // Inner deadzone controls
            card_elements.push(
                column![
                    text("Deadzone (noise filter)").size(11),
                    row![
                        text("X:").size(11),
                        Space::with_width(4),
                        self.deadzone_buttons(&device_id, false, deadzone_x),
                        Space::with_width(8),
                        text(format!("{}%", deadzone_x)).size(11),
                    ]
                    .spacing(2)
                    .align_items(Alignment::Center),
                    row![
                        text("Y:").size(11),
                        Space::with_width(4),
                        self.deadzone_buttons(&device_id, true, deadzone_y),
                        Space::with_width(8),
                        text(format!("{}%", deadzone_y)).size(11),
                    ]
                    .spacing(2)
                    .align_items(Alignment::Center),
                ]
                .spacing(4)
                .into()
            );

            // Outer deadzone controls
            card_elements.push(Space::with_height(4).into());
            card_elements.push(
                column![
                    text("Max Range (input clamp)").size(11),
                    row![
                        text("X:").size(11),
                        Space::with_width(4),
                        self.outer_deadzone_buttons(&device_id, false, outer_deadzone_x),
                        Space::with_width(8),
                        text(format!("{}%", outer_deadzone_x)).size(11),
                    ]
                    .spacing(2)
                    .align_items(Alignment::Center),
                    row![
                        text("Y:").size(11),
                        Space::with_width(4),
                        self.outer_deadzone_buttons(&device_id, true, outer_deadzone_y),
                        Space::with_width(8),
                        text(format!("{}%", outer_deadzone_y)).size(11),
                    ]
                    .spacing(2)
                    .align_items(Alignment::Center),
                ]
                .spacing(4)
                .into()
            );
        }

        // Add keypad button if applicable
        if let Some(keypad_btn) = keypad_button {
            card_elements.push(Space::with_height(4).into());
            card_elements.push(
                row![Space::with_width(Length::Fill), keypad_btn,]
                    .spacing(4)
                    .into()
            );
        }

        // Add LED configuration button if applicable
        if let Some(led_btn) = led_button {
            card_elements.push(Space::with_height(4).into());
            card_elements.push(
                row![Space::with_width(Length::Fill), led_btn,]
                    .spacing(4)
                    .into()
            );
        }

        // Add auto-switch rules button
        if let Some(auto_btn) = auto_switch_button {
            card_elements.push(Space::with_height(4).into());
            card_elements.push(
                row![Space::with_width(Length::Fill), auto_btn,]
                    .spacing(4)
                    .into()
            );
        }

        // Add hotkey bindings button
        if let Some(hotkey_btn) = hotkey_button {
            card_elements.push(Space::with_height(4).into());
            card_elements.push(
                row![Space::with_width(Length::Fill), hotkey_btn,]
                    .spacing(4)
                    .into()
            );
        }

        // Add analog calibration button
        if let Some(analog_btn) = analog_button {
            card_elements.push(Space::with_height(4).into());
            card_elements.push(
                row![Space::with_width(Length::Fill), analog_btn,]
                    .spacing(4)
                    .into()
            );
        }

        let card_content = column(card_elements).spacing(4);

        container(card_content)
            .padding(16)
            .width(Length::Fill)
            .style(theme::styles::card)
            .into()
    }

    fn view_macros_tab(&self) -> Element<'_, Message> {
        views::macros::view(self)
    }

    fn view_profiles_tab(&self) -> Element<'_, Message> {
        views::profiles::view_profiles_tab(self)
    }

    /// View for auto-switch rules configuration
    ///
    /// Displays the current focus, list of rules, and controls for adding/editing rules.
    fn view_auto_switch_rules(&self) -> Element<'_, Message> {
        views::auto_switch::view(self)
    }

    fn view_hotkey_bindings(&self) -> Element<'_, Message> {
        views::hotkeys::view(self)
    }

    fn view_azeron_keypad(&self) -> Element<'_, Message> {
        views::keypad::view(self)
    }

    fn view_status_bar(&self) -> Element<'_, Message> {
        views::status_bar::view(self)
    }

    /// View layer indicator for a device
    ///
    /// Displays the active layer name/ID for the given device.
    /// Shows "Layer N: {name}" format with Primary style for visibility.
    fn layer_indicator(&self, device_id: &str) -> Element<'_, Message> {
        if let Some(&layer_id) = self.active_layers.get(device_id) {
            // Get layer name from configs if available
            let layer_name = self.layer_configs
                .get(device_id)
                .and_then(|layers| layers.iter().find(|l| l.layer_id == layer_id))
                .map(|l| l.name.as_str())
                .unwrap_or("Unknown");

            container(
                text(format!("Layer {}: {}", layer_id, layer_name))
                    .size(12)
            )
            .padding([4, 8])
            .style(theme::styles::card)
            .into()
        } else {
            // No active layer - show default base layer
            container(
                text("Layer 0: Base").size(12)
            )
            .padding([4, 8])
            .style(theme::styles::card)
            .into()
        }
    }

    /// View profile quick toggle buttons for a device
    ///
    /// Shows horizontal row of toggle buttons for each available remap profile.
    /// Highlights the active profile with Primary style.
    /// Similar to the official Azeron software's profile toggle interface.
    fn profile_quick_toggles(&self, device_path: &str) -> Element<'_, Message> {
        let profiles = self.remap_profiles.get(device_path);
        let active_profile = self.active_remap_profiles.get(device_path);

        if let Some(profile_list) = profiles {
            if profile_list.is_empty() {
                return row![].into(); // Empty row when no profiles
            }

            let buttons: Vec<Element<'_, Message>> = profile_list
                .iter()
                .map(|profile| {
                    let is_active = active_profile.as_ref().map(|s| s.as_str()) == Some(profile.name.as_str());
                    let button_style = if is_active {
                        iced::theme::Button::Primary
                    } else {
                        iced::theme::Button::Secondary
                    };

                    button(
                        text(&profile.name).size(11)
                    )
                    .on_press(Message::ActivateRemapProfile(device_path.to_string(), profile.name.clone()))
                    .style(button_style)
                    .padding([6, 10])
                    .into()
                })
                .collect();

            // If there's an active profile, add a deactivate button at the end
            let mut final_buttons = buttons;
            if active_profile.is_some() {
                final_buttons.push(
                    button(
                        text("Off").size(11)
                    )
                    .on_press(Message::DeactivateRemapProfile(device_path.to_string()))
                    .style(iced::theme::Button::Text)
                    .padding([6, 10])
                    .into()
                );
            }

            row(final_buttons).spacing(6).into()
        } else {
            row![].into() // Empty row when profiles not loaded
        }
    }

    /// View layer activation buttons for a device
    ///
    /// Shows buttons for each toggle layer available for the device.
    /// Highlights active toggle layers with Secondary style.
    fn layer_activation_buttons(&self, device_id: &str) -> Element<'_, Message> {
        let layers = self.layer_configs.get(device_id);

        if let Some(layer_list) = layers {
            // Filter for toggle layers only
            let toggle_layers: Vec<_> = layer_list
                .iter()
                .filter(|l| l.mode == LayerMode::Toggle && l.layer_id > 0)
                .collect();

            if toggle_layers.is_empty() {
                return text("No toggle layers configured").size(11).into();
            }

            let active_layer_id = self.active_layers.get(device_id).copied().unwrap_or(0);

            let buttons: Vec<Element<'_, Message>> = toggle_layers
                .iter()
                .map(|layer| {
                    let is_active = active_layer_id == layer.layer_id;
                    let button_style = if is_active {
                        iced::theme::Button::Secondary
                    } else {
                        iced::theme::Button::Text
                    };

                    button(
                        text(format!("L{}", layer.layer_id)).size(11)
                    )
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

    /// Deadzone quick-select buttons
    ///
    /// Provides buttons for common deadzone percentages.
    fn deadzone_buttons(&self, device_id: &str, is_y_axis: bool, current: u8) -> Element<'_, Message> {
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

    /// Outer deadzone quick-select buttons
    ///
    /// Provides buttons for common outer deadzone percentages.
    fn outer_deadzone_buttons(&self, device_id: &str, is_y_axis: bool, current: u8) -> Element<'_, Message> {
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

    /// View layer settings for a device
    ///
    /// Displays a table/list of all layers for the device with edit buttons.
    fn layer_settings_view(&self, device_id: &str) -> Element<'_, Message> {
        let layers = self.layer_configs.get(device_id);

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
                        text(format!("L{}", layer.layer_id)).size(12).width(Length::Fixed(30.0)),
                        text(&layer.name).size(12).width(Length::Fixed(100.0)),
                        text(mode_text).size(12).width(Length::Fixed(60.0)),
                        text(format!("{} remaps", layer.remap_count)).size(11),
                        Space::with_width(Length::Fill),
                        button(text("Edit").size(11))
                            .on_press(Message::OpenLayerConfigDialog(device_id.to_string(), layer.layer_id))
                            .style(iced::theme::Button::Text)
                            .padding([4, 8]),
                    ]
                    .spacing(8)
                    .align_items(Alignment::Center)
                    .into()
                })
                .collect();

            // Add "Add Layer" button if less than 8 layers
            let add_button = if layer_list.len() < 8 {
                Some(
                    button(
                        row![
                            text("+").size(14),
                            text("Add Layer").size(12),
                        ]
                        .spacing(4)
                    )
                    .on_press(Message::OpenLayerConfigDialog(
                        device_id.to_string(),
                        layer_list.len(),
                    ))
                    .style(iced::theme::Button::Secondary)
                    .padding([6, 12])
                    .into()
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

    /// View layer configuration dialog
    ///
    /// Modal dialog for editing layer name and mode.
    fn layer_config_dialog(&self) -> Option<Element<'_, Message>> {
        if let Some((_device_id, layer_id, name, mode)) = &self.layer_config_dialog {
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
                    pick_list(mode_options, Some(current_mode_str.to_string()), |selected| {
                        let new_mode = match selected.as_str() {
                            "Toggle" => LayerMode::Toggle,
                            _ => LayerMode::Hold,
                        };
                        Message::LayerConfigModeChanged(new_mode)
                    })
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
                .spacing(4)
            )
            .padding(24)
            .width(Length::Fixed(300.0))
            .style(theme::styles::card);

            // Overlay dialog on semi-transparent background
            Some(
                container(
                    container(dialog)
                        .width(Length::Fill)
                        .center_x()
                        .center_y()
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .style(iced::theme::Container::Transparent)
                .into()
            )
        } else {
            None
        }
    }

    /// Get current color for a zone, with default fallback
    pub fn view_led_config(&self) -> Option<Element<'_, Message>> {
        views::led::view(self)
    }

    /// View analog calibration dialog
    ///
    /// Displays modal dialog for analog stick calibration with deadzone,
    /// sensitivity, range, and inversion controls.
    pub fn view_analog_calibration(&self) -> Option<Element<'_, Message>> {
        views::analog::overlay_view(self)
    }
}

#[cfg(test)]
mod calibration_tests {
    use super::*;
    use aethermap_common::{AnalogMode, CameraOutputMode};

    #[test]
    fn test_analog_calibration_view_default() {
        let view = AnalogCalibrationView::default();

        assert_eq!(view.device_id, "");
        assert_eq!(view.layer_id, 0);
        assert_eq!(view.calibration.deadzone, 0.15);
        assert_eq!(view.stick_x, 0.0);
        assert_eq!(view.stick_y, 0.0);
        assert_eq!(view.loading, false);
        assert!(view.error.is_none());
    }

    #[test]
    fn test_analog_calibration_view_with_values() {
        let view = AnalogCalibrationView {
            device_id: "test_device".to_string(),
            layer_id: 1,
            calibration: CalibrationConfig {
                deadzone: 0.2,
                deadzone_shape: "circular".to_string(),
                sensitivity: "quadratic".to_string(),
                sensitivity_multiplier: 1.5,
                range_min: -16384,
                range_max: 16383,
                invert_x: true,
                invert_y: false,
                exponent: 2.0,
            },
            deadzone_shape_selected: DeadzoneShape::Square,
            sensitivity_curve_selected: SensitivityCurve::Quadratic,
            analog_mode_selected: AnalogMode::Mouse,
            camera_mode_selected: CameraOutputMode::Keys,
            invert_x_checked: true,
            invert_y_checked: false,
            stick_x: 0.5,
            stick_y: -0.3,
            loading: false,
            error: None,
            last_visualizer_update: Instant::now(),
            visualizer_cache: Arc::new(iced::widget::canvas::Cache::default()),
        };

        assert_eq!(view.device_id, "test_device");
        assert_eq!(view.layer_id, 1);
        assert_eq!(view.calibration.deadzone, 0.2);
        assert_eq!(view.stick_x, 0.5);
        assert_eq!(view.stick_y, -0.3);
        assert_eq!(view.analog_mode_selected, AnalogMode::Mouse);
        assert_eq!(view.camera_mode_selected, CameraOutputMode::Keys);
        assert_eq!(view.invert_x_checked, true);
        assert_eq!(view.invert_y_checked, false);
    }

    #[test]
    fn test_calibration_config_default() {
        let config = CalibrationConfig::default();

        assert_eq!(config.deadzone, 0.15);
        assert_eq!(config.deadzone_shape, "circular");
        assert_eq!(config.sensitivity, "linear");
        assert_eq!(config.sensitivity_multiplier, 1.0);
        assert_eq!(config.range_min, -32768);
        assert_eq!(config.range_max, 32767);
        assert_eq!(config.invert_x, false);
        assert_eq!(config.invert_y, false);
        assert_eq!(config.exponent, 2.0);
    }

    #[test]
    fn test_deadzone_shape_display() {
        assert_eq!(DeadzoneShape::Circular.to_string(), "Circular");
        assert_eq!(DeadzoneShape::Square.to_string(), "Square");
    }

    #[test]
    fn test_sensitivity_curve_display() {
        assert_eq!(SensitivityCurve::Linear.to_string(), "Linear");
        assert_eq!(SensitivityCurve::Quadratic.to_string(), "Quadratic");
        assert_eq!(SensitivityCurve::Exponential.to_string(), "Exponential");
    }

    #[test]
    fn test_deadzone_shape_default() {
        assert_eq!(DeadzoneShape::default(), DeadzoneShape::Circular);
    }

    #[test]
    fn test_sensitivity_curve_default() {
        assert_eq!(SensitivityCurve::default(), SensitivityCurve::Linear);
    }

    #[test]
    fn test_analog_calibration_view_clone() {
        let view = AnalogCalibrationView {
            device_id: "test_device".to_string(),
            layer_id: 1,
            calibration: CalibrationConfig {
                deadzone: 0.2,
                ..Default::default()
            },
            ..Default::default()
        };

        let cloned = view.clone();
        assert_eq!(cloned.device_id, "test_device");
        assert_eq!(cloned.layer_id, 1);
        assert_eq!(cloned.calibration.deadzone, 0.2);
        // Clone resets last_visualizer_update to Instant::now()
        assert!(cloned.last_visualizer_update.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn test_throttling_threshold() {
        // Verify the 30 FPS throttling threshold (33ms)
        let view = AnalogCalibrationView {
            device_id: "test".to_string(),
            layer_id: 0,
            calibration: CalibrationConfig::default(),
            deadzone_shape_selected: DeadzoneShape::Circular,
            sensitivity_curve_selected: SensitivityCurve::Linear,
            analog_mode_selected: AnalogMode::Disabled,
            camera_mode_selected: CameraOutputMode::Scroll,
            invert_x_checked: false,
            invert_y_checked: false,
            stick_x: 0.0,
            stick_y: 0.0,
            loading: false,
            error: None,
            last_visualizer_update: Instant::now(),
            visualizer_cache: Arc::new(iced::widget::canvas::Cache::default()),
        };

        // Immediately after update, elapsed time should be small
        assert!(view.last_visualizer_update.elapsed() < Duration::from_millis(33));

        // After 40ms, should definitely exceed the threshold
        std::thread::sleep(Duration::from_millis(40));
        assert!(view.last_visualizer_update.elapsed() >= Duration::from_millis(33));
    }

    #[test]
    fn test_visualizer_cache_arc_sharing() {
        // Verify that Arc<Cache> can be cloned and shared
        let cache = Arc::new(iced::widget::canvas::Cache::default());
        let cache_clone = Arc::clone(&cache);

        // Both Arcs point to the same Cache
        assert!(Arc::ptr_eq(&cache, &cache_clone));
    }

    #[test]
    fn test_analog_mode_selection_states() {
        // Test that all analog modes can be selected
        let modes = [
            AnalogMode::Disabled,
            AnalogMode::Dpad,
            AnalogMode::Gamepad,
            AnalogMode::Camera,
            AnalogMode::Mouse,
            AnalogMode::Wasd,
        ];

        for mode in modes {
            let view = AnalogCalibrationView {
                analog_mode_selected: mode,
                ..Default::default()
            };
            assert_eq!(view.analog_mode_selected, mode);
        }
    }

    #[test]
    fn test_camera_mode_selection_states() {
        // Test that all camera output modes can be selected
        let modes = [CameraOutputMode::Scroll, CameraOutputMode::Keys];

        for mode in modes {
            let view = AnalogCalibrationView {
                camera_mode_selected: mode,
                ..Default::default()
            };
            assert_eq!(view.camera_mode_selected, mode);
        }
    }
}
