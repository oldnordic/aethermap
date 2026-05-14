use iced::{
    widget::{
        column, container, row, scrollable,
        horizontal_rule, vertical_rule,
    },
    Element, Length, Subscription, Theme, Application, Command,
};
use std::sync::Arc;
use crate::theme::{aether_dark, aether_light};
use crate::views;

// Import custom widgets
use aethermap_common::{DeviceInfo, DeviceCapabilities, DeviceType, LayerConfigInfo, LayerMode, LedPattern, LedZone, MacroEntry, MacroSettings, RemapProfileInfo, RemapEntry, AnalogMode, CameraOutputMode};
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
                crate::handlers::auto_switch::show(self, device_id)
            }
            Message::CloseAutoSwitchRules => {
                crate::handlers::auto_switch::close(self)
            }
            Message::LoadAutoSwitchRules(_device_id) => {
                crate::handlers::auto_switch::load(self)
            }
            Message::AutoSwitchRulesLoaded(Ok(rules)) => {
                crate::handlers::auto_switch::loaded(self, rules)
            }
            Message::AutoSwitchRulesLoaded(Err(error)) => {
                crate::handlers::auto_switch::load_error(self, error)
            }
            Message::EditAutoSwitchRule(index) => {
                crate::handlers::auto_switch::edit(self, index)
            }
            Message::AutoSwitchAppIdChanged(value) => {
                crate::handlers::auto_switch::app_id_changed(self, value)
            }
            Message::AutoSwitchProfileNameChanged(value) => {
                crate::handlers::auto_switch::profile_name_changed(self, value)
            }
            Message::AutoSwitchLayerIdChanged(value) => {
                crate::handlers::auto_switch::layer_id_changed(self, value)
            }
            Message::AutoSwitchUseCurrentApp => {
                crate::handlers::auto_switch::use_current_app(self)
            }
            Message::SaveAutoSwitchRule => {
                crate::handlers::auto_switch::save(self)
            }
            Message::DeleteAutoSwitchRule(index) => {
                crate::handlers::auto_switch::delete(self, index)
            }

            // Hotkey Bindings Management
            Message::ShowHotkeyBindings(device_id) => {
                crate::handlers::hotkeys::show(self, device_id)
            }
            Message::CloseHotkeyBindings => {
                crate::handlers::hotkeys::close(self)
            }
            Message::LoadHotkeyBindings(device_id) => {
                crate::handlers::hotkeys::load(self, device_id)
            }
            Message::HotkeyBindingsLoaded(Ok(bindings)) => {
                crate::handlers::hotkeys::loaded(self, bindings)
            }
            Message::HotkeyBindingsLoaded(Err(error)) => {
                crate::handlers::hotkeys::load_error(self, error)
            }
            Message::EditHotkeyBinding(index) => {
                crate::handlers::hotkeys::edit(self, index)
            }
            Message::ToggleHotkeyModifier(modifier) => {
                crate::handlers::hotkeys::toggle_modifier(self, modifier)
            }
            Message::HotkeyKeyChanged(value) => {
                crate::handlers::hotkeys::key_changed(self, value)
            }
            Message::HotkeyProfileNameChanged(value) => {
                crate::handlers::hotkeys::profile_name_changed(self, value)
            }
            Message::HotkeyLayerIdChanged(value) => {
                crate::handlers::hotkeys::layer_id_changed(self, value)
            }
            Message::SaveHotkeyBinding => {
                crate::handlers::hotkeys::save(self)
            }
            Message::DeleteHotkeyBinding(index) => {
                crate::handlers::hotkeys::delete(self, index)
            }
            Message::HotkeyBindingsUpdated(bindings) => {
                crate::handlers::hotkeys::bindings_updated(self, bindings)
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
        if let Some(dialog) = views::devices::layer_config_dialog(self) {
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
    pub(crate) fn add_notification(&mut self, message: &str, is_error: bool) {
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
        views::devices::view_devices_tab(self)
    }

    fn view_macros_tab(&self) -> Element<'_, Message> {
        views::macros::view(self)
    }

    fn view_profiles_tab(&self) -> Element<'_, Message> {
        views::profiles::view_profiles_tab(self)
    }

    fn view_status_bar(&self) -> Element<'_, Message> {
        views::status_bar::view(self)
    }

    /// View layer indicator for a device
    ///
    /// Displays the active layer name/ID for the given device.
    /// Shows "Layer N: {name}" format with Primary style for visibility.
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
