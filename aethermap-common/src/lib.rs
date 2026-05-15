use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

// Re-export common dependencies
pub use bincode;
pub use serde;
pub use tokio;
pub use tracing;

// IPC client module
pub mod ipc_client;

/// Device type classification based on input capabilities
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceType {
    /// Keyboard device (has EV_KEY with key codes)
    Keyboard,
    /// Mouse or pointing device (has EV_REL or mouse buttons)
    Mouse,
    /// Gamepad or joystick (has gamepad buttons, may have EV_ABS)
    Gamepad,
    /// Keypad device (many keys + possibly analog stick, e.g., Azeron Cyborg)
    Keypad,
    /// Other or unknown input device
    Other,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeviceType::Keyboard => write!(f, "Keyboard"),
            DeviceType::Mouse => write!(f, "Mouse"),
            DeviceType::Gamepad => write!(f, "Gamepad"),
            DeviceType::Keypad => write!(f, "Keypad"),
            DeviceType::Other => write!(f, "Other"),
        }
    }
}

/// Information about a connected input device
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceInfo {
    pub name: String,
    pub path: PathBuf,
    pub vendor_id: u16,
    pub product_id: u16,
    pub phys: String,
    pub device_type: DeviceType,
}

impl fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} (VID: {:04X}, PID: {:04X}, Type: {})",
            self.name, self.vendor_id, self.product_id, self.device_type
        )
    }
}

/// Represents a key combination for macro triggers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub keys: Vec<u16>,      // Key codes
    pub modifiers: Vec<u16>, // Modifier key codes
}

/// Global hotkey binding for manual profile switching
///
/// Defines a keyboard shortcut that triggers profile or layer activation.
/// Hotkeys are checked at the daemon level before remap processing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HotkeyBinding {
    /// Modifier keys (Ctrl, Alt, Shift, Super)
    ///
    /// Accepted values: "ctrl", "alt", "shift", "super" (case-insensitive)
    pub modifiers: Vec<String>,

    /// Trigger key (number 1-9 for profile switching)
    ///
    /// Common values: "1"-"9" for profile slots, or any key name like "f1", "esc"
    pub key: String,

    /// Profile to activate when hotkey pressed
    pub profile_name: String,

    /// Device to apply to (None = all devices)
    ///
    /// If set, only this device_id (vendor:product format) will switch profiles.
    pub device_id: Option<String>,

    /// Layer to activate (None = profile default)
    ///
    /// If set, activates the specified layer after switching profiles.
    pub layer_id: Option<usize>,
}

/// Auto-profile switching rule based on window focus
///
/// Defines automatic profile switching when specific applications gain focus.
/// Rules are evaluated in order with first-match-wins semantics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutoSwitchRule {
    /// Application identifier to match (e.g., "org.alacritty", "firefox", "*")
    ///
    /// - "*" acts as wildcard matching any app (useful for default profile)
    /// - Can match prefix (e.g., "org.mozilla." matches any Firefox window)
    /// - Can match suffix (e.g., ".firefox" matches Firefox app)
    pub app_id: String,

    /// Profile name to activate when this app has focus
    pub profile_name: String,

    /// Device ID to apply profile to (vendor:product format)
    ///
    /// If None, applies to all devices. Use this for per-device auto-switching.
    pub device_id: Option<String>,

    /// Layer ID to activate (0 = base, 1+ = additional layers)
    ///
    /// If None, uses profile's default layer (typically base layer 0).
    pub layer_id: Option<usize>,
}

/// Different actions that can be executed by a macro
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Action {
    /// Key press with optional key code
    KeyPress(u16),
    /// Key release
    KeyRelease(u16),
    /// Delay in milliseconds
    Delay(u32),
    /// Execute a command
    Execute(String),
    /// Type a string
    Type(String),
    /// Mouse button press
    MousePress(u16),
    /// Mouse button release
    MouseRelease(u16),
    /// Mouse move relative
    MouseMove(i32, i32),
    /// Mouse scroll
    MouseScroll(i32),
    /// Analog stick movement with normalized value
    /// axis_code: 61000-61005 (ABS_X, ABS_Y, etc.)
    /// normalized: -1.0 to 1.0 (device-independent)
    AnalogMove { axis_code: u16, normalized: f32 },
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Action::KeyPress(code) => write!(f, "KeyPress({})", code),
            Action::KeyRelease(code) => write!(f, "KeyRelease({})", code),
            Action::Delay(ms) => write!(f, "Delay({}ms)", ms),
            Action::Execute(cmd) => write!(f, "Execute({})", cmd),
            Action::Type(text) => write!(f, "Type({})", text),
            Action::MousePress(btn) => write!(f, "MousePress({})", btn),
            Action::MouseRelease(btn) => write!(f, "MouseRelease({})", btn),
            Action::MouseMove(x, y) => write!(f, "MouseMove({}, {})", x, y),
            Action::MouseScroll(amount) => write!(f, "MouseScroll({})", amount),
            Action::AnalogMove {
                axis_code,
                normalized,
            } => {
                let axis_name = match axis_code {
                    61000 => "X",
                    61001 => "Y",
                    61002 => "Z",
                    61003 => "RX",
                    61004 => "RY",
                    61005 => "RZ",
                    _ => "UNKNOWN",
                };
                write!(f, "Analog({}, {}={:.2})", axis_name, axis_code, normalized)
            }
        }
    }
}

/// Macro definition with name, trigger combo, and actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MacroSettings {
    pub latency_offset_ms: u32,
    pub jitter_pct: f32,
    pub capture_mouse: bool,
}

/// Macro definition with name, trigger combo, and actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MacroEntry {
    pub name: String,
    pub trigger: KeyCombo,
    pub actions: Vec<Action>,
    pub device_id: Option<String>, // Optional device restriction
    pub enabled: bool,
    #[serde(default)]
    pub humanize: bool,
    #[serde(default)]
    pub capture_mouse: bool,
}

/// Information about a remap profile for listing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemapProfileInfo {
    /// Profile name
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Number of remaps in this profile
    pub remap_count: usize,
}

/// A single key remapping entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemapEntry {
    /// Source key (the key being remapped)
    pub from_key: String,
    /// Target key (what the source key becomes)
    pub to_key: String,
}

/// Device capability information
///
/// This structure provides detailed capability information for a device,
/// allowing the GUI to enable/disable relevant UI elements based on actual
/// device hardware capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DeviceCapabilities {
    /// Device has analog stick (absolute X/Y axes)
    pub has_analog_stick: bool,

    /// Device has hat switch (D-pad with ABS_HAT0X/ABS_HAT0Y)
    pub has_hat_switch: bool,

    /// Number of joystick buttons (BTN_JOYSTICK range)
    pub joystick_button_count: usize,

    /// LED zones available (empty if none, populated in Phase 12)
    pub led_zones: Vec<String>,
}

/// Layer activation mode
///
/// Determines how a layer becomes active and inactive.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LayerMode {
    /// Layer is active while a modifier key is held
    ///
    /// When the modifier key is released, the layer deactivates.
    /// This is the typical behavior for "layer shift" keys.
    #[default]
    Hold,

    /// Layer toggles on/off with each press
    ///
    /// First press activates the layer, second press deactivates it.
    /// This is useful for "layer lock" functionality.
    Toggle,
}

impl fmt::Display for LayerMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LayerMode::Hold => write!(f, "hold"),
            LayerMode::Toggle => write!(f, "toggle"),
        }
    }
}

/// Common layer configuration for IPC
///
/// This structure provides the complete layer configuration including LED colors
/// for IPC communication between daemon and GUI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommonLayerConfig {
    /// Layer ID (0 = base, 1+ = additional layers)
    #[serde(default)]
    pub layer_id: usize,

    /// Human-readable layer name (e.g., "Base", "Gaming", "Work")
    #[serde(default)]
    pub name: String,

    /// How this layer is activated (hold or toggle)
    #[serde(default)]
    pub mode: LayerMode,

    /// LED color for this layer (RGB)
    #[serde(default = "default_layer_color")]
    pub led_color: (u8, u8, u8),

    /// LED zone to display layer color
    #[serde(default)]
    pub led_zone: Option<LedZone>,
}

/// Default layer color (blue for layer 1, can be customized)
fn default_layer_color() -> (u8, u8, u8) {
    (0, 0, 255) // Blue
}

/// Layer configuration information for IPC
///
/// This structure provides layer configuration details for GUI display
/// and modification via IPC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayerConfigInfo {
    /// Layer ID (0 = base, 1+ = additional layers)
    pub layer_id: usize,

    /// Human-readable layer name (e.g., "Base", "Gaming", "Work")
    pub name: String,

    /// How this layer is activated (hold or toggle)
    pub mode: LayerMode,

    /// Number of remappings configured for this layer
    pub remap_count: usize,

    /// LED color for this layer (RGB)
    #[serde(default = "default_layer_color")]
    pub led_color: (u8, u8, u8),

    /// LED zone to display layer color
    #[serde(default)]
    pub led_zone: Option<LedZone>,
}

/// Analog calibration configuration for IPC
///
/// This structure provides a simplified version of AnalogCalibration for IPC
/// communication between daemon and GUI. It uses string representations for
/// enum values to avoid circular dependencies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalogCalibrationConfig {
    /// Deadzone radius (0.0 to 1.0)
    pub deadzone: f32,

    /// Deadzone shape: "circular" or "square"
    pub deadzone_shape: String,

    /// Sensitivity curve: "linear", "quadratic", or "exponential"
    pub sensitivity: String,

    /// Sensitivity multiplier (0.1 to 5.0)
    pub sensitivity_multiplier: f32,

    /// Minimum output value (typically -32768)
    pub range_min: i32,

    /// Maximum output value (typically 32767)
    pub range_max: i32,

    /// Invert X axis
    pub invert_x: bool,

    /// Invert Y axis
    pub invert_y: bool,

    /// Exponential curve exponent (only used when sensitivity is "exponential")
    #[serde(default = "default_exponent")]
    pub exponent: f32,

    /// Analog output mode (Wasd, Mouse, Camera, etc.)
    #[serde(default)]
    pub analog_mode: AnalogMode,

    /// Camera output sub-mode (only used when analog_mode is Camera)
    #[serde(default)]
    pub camera_output_mode: Option<CameraOutputMode>,
}

fn default_exponent() -> f32 {
    2.0
}

impl Default for AnalogCalibrationConfig {
    fn default() -> Self {
        Self {
            deadzone: 0.15,
            deadzone_shape: "circular".to_string(),
            sensitivity: "linear".to_string(),
            sensitivity_multiplier: 1.0,
            range_min: -32768,
            range_max: 32767,
            invert_x: false,
            invert_y: false,
            exponent: 2.0,
            analog_mode: AnalogMode::Disabled,
            camera_output_mode: None,
        }
    }
}

/// IPC Requests from GUI to Daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// List all available devices
    GetDevices,

    /// Set a macro for a device
    SetMacro {
        device_path: String,
        macro_entry: MacroEntry,
    },

    /// List all configured macros
    ListMacros,

    /// Delete a macro by name
    DeleteMacro { name: String },

    /// Reload configuration from disk
    ReloadConfig,

    /// Set LED color for a device
    LedSet {
        device_path: String,
        color: (u8, u8, u8), // RGB
    },

    /// Start recording a macro
    RecordMacro {
        device_path: String,
        name: String,
        capture_mouse: bool,
    },

    /// Stop recording a macro
    StopRecording,

    /// Test a macro execution
    TestMacro { name: String },

    /// Get daemon status and version
    GetStatus,

    /// Save current macros to a profile
    SaveProfile { name: String },

    /// Load macros from a profile
    LoadProfile { name: String },

    /// List available profiles
    ListProfiles,

    /// Delete a profile
    DeleteProfile { name: String },

    /// Generate an authentication token
    GenerateToken { client_id: String },

    /// Authenticate with a token
    Authenticate { token: String },

    /// Execute a macro by name
    ExecuteMacro { name: String },

    /// Grab a device exclusively for input interception
    GrabDevice { device_path: String },

    /// Release exclusive access to a device
    UngrabDevice { device_path: String },

    /// Get available profiles for a specific device
    GetDeviceProfiles {
        device_id: String, // vendor:product format
    },

    /// Activate a remap profile for a device
    ActivateProfile {
        device_id: String, // vendor:product format
        profile_name: String,
    },

    /// Deactivate the current remap profile for a device
    DeactivateProfile {
        device_id: String, // vendor:product format
    },

    /// Get the currently active profile for a device
    GetActiveProfile {
        device_id: String, // vendor:product format
    },

    /// Query active remap configuration for a device
    GetActiveRemaps { device_path: String },

    /// List available remap profiles for a device
    ListRemapProfiles { device_path: String },

    /// Activate a remap profile for a device
    ActivateRemapProfile {
        device_path: String,
        profile_name: String,
    },

    /// Deactivate current remap profile for a device
    DeactivateRemapProfile { device_path: String },

    /// Get device capabilities and features
    GetDeviceCapabilities { device_path: String },

    /// Get the currently active layer for a device
    GetActiveLayer { device_id: String },

    /// Set layer configuration for a device
    SetLayerConfig {
        device_id: String,
        layer_id: usize,
        config: LayerConfigInfo,
    },

    /// Activate a layer for a device with specified mode
    ActivateLayer {
        device_id: String,
        layer_id: usize,
        mode: LayerMode,
    },

    /// List all configured layers for a device
    ListLayers { device_id: String },

    /// Set analog sensitivity for a device
    SetAnalogSensitivity {
        device_id: String,
        sensitivity: f32, // 0.1-5.0 range
    },

    /// Get analog sensitivity for a device
    GetAnalogSensitivity { device_id: String },

    /// Set analog response curve for a device
    SetAnalogResponseCurve {
        device_id: String,
        curve: String, // "linear" or "exponential" or "exponential(<exponent>)"
    },

    /// Get analog response curve for a device
    GetAnalogResponseCurve { device_id: String },

    /// Set analog deadzone for a device (both X and Y axes)
    SetAnalogDeadzone {
        device_id: String,
        percentage: u8, // 0-100
    },

    /// Get analog deadzone for a device (returns X-axis percentage)
    GetAnalogDeadzone { device_id: String },

    /// Set per-axis analog deadzone for a device
    SetAnalogDeadzoneXY {
        device_id: String,
        x_percentage: u8, // 0-100
        y_percentage: u8, // 0-100
    },

    /// Get per-axis analog deadzone for a device
    GetAnalogDeadzoneXY { device_id: String },

    /// Set per-axis outer deadzone (max clamp) for a device
    SetAnalogOuterDeadzoneXY {
        device_id: String,
        x_percentage: u8, // 0-100
        y_percentage: u8, // 0-100
    },

    /// Get per-axis outer deadzone for a device
    GetAnalogOuterDeadzoneXY { device_id: String },

    /// Set D-pad emulation mode for a device
    SetAnalogDpadMode {
        device_id: String,
        mode: String, // "disabled", "eight_way", "four_way"
    },

    /// Get D-pad emulation mode for a device
    GetAnalogDpadMode { device_id: String },

    /// Set LED color for a specific zone
    SetLedColor {
        device_id: String,
        zone: LedZone,
        red: u8,
        green: u8,
        blue: u8,
    },

    /// Get LED color for a specific zone
    GetLedColor { device_id: String, zone: LedZone },

    /// Get all LED colors for a device
    GetAllLedColors { device_id: String },

    /// Set LED brightness for a device (global or per-zone)
    SetLedBrightness {
        device_id: String,
        zone: Option<LedZone>, // None = global brightness
        brightness: u8,        // 0-100
    },

    /// Get LED brightness for a device
    GetLedBrightness {
        device_id: String,
        zone: Option<LedZone>,
    },

    /// Set LED pattern for a device
    SetLedPattern {
        device_id: String,
        pattern: LedPattern,
    },

    /// Get LED pattern for a device
    GetLedPattern { device_id: String },

    /// Notify daemon that window focus changed (for auto-profile switching)
    FocusChanged {
        app_id: String,               // e.g., "org.alacritty", "firefox"
        window_title: Option<String>, // May be empty on some compositors
    },

    /// Register a global hotkey binding
    RegisterHotkey {
        device_id: String,
        binding: HotkeyBinding,
    },

    /// List all registered hotkey bindings for a device
    ListHotkeys { device_id: String },

    /// Remove a hotkey binding
    RemoveHotkey {
        device_id: String,
        key: String,
        modifiers: Vec<String>,
    },

    /// Set global auto-switch rules for profile switching
    SetAutoSwitchRules { rules: Vec<AutoSwitchRule> },

    /// Get all auto-switch rules
    GetAutoSwitchRules,

    /// Get analog calibration for a device and layer
    GetAnalogCalibration { device_id: String, layer_id: usize },

    /// Set analog calibration for a device and layer
    SetAnalogCalibration {
        device_id: String,
        layer_id: usize,
        calibration: AnalogCalibrationConfig,
    },

    /// Subscribe to analog input updates for a device
    SubscribeAnalogInput { device_id: String },

    /// Unsubscribe from analog input updates
    UnsubscribeAnalogInput { device_id: String },

    /// Set global macro timing and jitter settings
    SetMacroSettings(MacroSettings),

    /// Get current global macro settings
    GetMacroSettings,
}

/// Analog output mode for stick behavior
///
/// Determines how analog stick input is converted to output events.
/// Used in LayerConfig to specify per-layer analog mode selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AnalogMode {
    /// No output (analog disabled)
    #[default]
    Disabled,
    /// D-pad mode - 8-way directional keys (arrows)
    Dpad,
    /// Gamepad mode - Xbox 360 compatible axis output
    Gamepad,
    /// Camera mode - scroll or key repeat
    Camera,
    /// Mouse mode - velocity-based cursor movement
    Mouse,
    /// WASD mode - directional keys (WASD)
    Wasd,
}

impl fmt::Display for AnalogMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalogMode::Disabled => write!(f, "Disabled"),
            AnalogMode::Dpad => write!(f, "D-pad (Arrows)"),
            AnalogMode::Gamepad => write!(f, "Gamepad"),
            AnalogMode::Camera => write!(f, "Camera"),
            AnalogMode::Mouse => write!(f, "Mouse"),
            AnalogMode::Wasd => write!(f, "WASD"),
        }
    }
}

impl AnalogMode {
    /// All analog modes for pick_list widget
    pub const ALL: [AnalogMode; 6] = [
        AnalogMode::Disabled,
        AnalogMode::Dpad,
        AnalogMode::Gamepad,
        AnalogMode::Wasd,
        AnalogMode::Mouse,
        AnalogMode::Camera,
    ];
}

/// Camera mode output type
///
/// Controls how analog stick input is converted in Camera mode.
/// Used in LayerConfig to specify camera output behavior per layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CameraOutputMode {
    /// Emit REL_WHEEL events for scrolling
    #[default]
    Scroll,
    /// Emit key repeat events (PageUp/PageDown/arrows)
    Keys,
}

impl fmt::Display for CameraOutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CameraOutputMode::Scroll => write!(f, "Scroll"),
            CameraOutputMode::Keys => write!(f, "Key Repeat"),
        }
    }
}

impl CameraOutputMode {
    /// All camera output modes for pick_list widget
    pub const ALL: [CameraOutputMode; 2] = [CameraOutputMode::Scroll, CameraOutputMode::Keys];
}

/// LED pattern types for visual effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LedPattern {
    /// Static solid colors (no animation)
    Static,
    /// Breathing pattern - fades colors in/out
    Breathing,
    /// Rainbow pattern - cycles through colors
    Rainbow,
    /// Rainbow wave - wave effect across zones
    RainbowWave,
}

/// LED zones on devices with configurable RGB lighting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LedZone {
    /// Side LED (single LED on Azeron Cyborg 2)
    Side,
    /// Logo LED (top of device) - legacy, may map to Side on some devices
    Logo,
    /// Main key cluster LEDs - legacy, may map to Side on some devices
    Keys,
    /// Thumbstick or analog stick LED ring - legacy, may map to Side on some devices
    Thumbstick,
    /// All zones at once
    All,
    /// Global setting
    Global,
}

/// Status information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusInfo {
    pub version: String,
    pub uptime_seconds: u64,
    pub devices_count: usize,
    pub macros_count: usize,
}

/// IPC Responses from Daemon to GUI
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Response {
    /// List of discovered devices
    Devices(Vec<DeviceInfo>),

    /// List of configured macros
    Macros(Vec<MacroEntry>),

    /// Acknowledgment of successful operation
    Ack,

    /// Status information
    Status {
        version: String,
        uptime_seconds: u64,
        devices_count: usize,
        macros_count: usize,
    },

    /// Notification that recording has started
    RecordingStarted { device_path: String, name: String },

    /// Notification that recording has stopped
    RecordingStopped { macro_entry: MacroEntry },

    /// List of available profiles
    Profiles(Vec<String>),

    /// Profile load confirmation
    ProfileLoaded { name: String, macros_count: usize },

    /// Profile save confirmation
    ProfileSaved { name: String, macros_count: usize },

    /// Error response
    Error(String),

    /// Authentication token
    Token(String),

    /// Authentication successful
    Authenticated,

    /// List of available profiles for a device
    DeviceProfiles {
        device_id: String,
        profiles: Vec<String>,
    },

    /// Profile activation confirmation
    ProfileActivated {
        device_id: String,
        profile_name: String,
    },

    /// Profile deactivation confirmation
    ProfileDeactivated { device_id: String },

    /// Current active profile for a device
    ActiveProfile {
        device_id: String,
        profile_name: Option<String>,
    },

    /// Active remap configuration
    ActiveRemaps {
        device_path: String,
        profile_name: Option<String>,
        remaps: Vec<RemapEntry>,
    },

    /// List of available profiles
    RemapProfiles {
        device_path: String,
        profiles: Vec<RemapProfileInfo>,
    },

    /// Remap profile activation confirmation
    RemapProfileActivated {
        device_path: String,
        profile_name: String,
    },

    /// Remap profile deactivation confirmation
    RemapProfileDeactivated { device_path: String },

    /// Device capability information
    DeviceCapabilities {
        device_path: String,
        capabilities: DeviceCapabilities,
    },

    /// Current active layer for a device
    ActiveLayer {
        device_id: String,
        layer_id: usize,
        layer_name: String,
    },

    /// Layer configuration confirmation
    LayerConfigured { device_id: String, layer_id: usize },

    /// List of configured layers for a device
    LayerList {
        device_id: String,
        layers: Vec<LayerConfigInfo>,
    },

    /// Analog sensitivity set confirmation
    AnalogSensitivitySet { device_id: String, sensitivity: f32 },

    /// Analog sensitivity response
    AnalogSensitivity { device_id: String, sensitivity: f32 },

    /// Analog response curve set confirmation
    AnalogResponseCurveSet { device_id: String, curve: String },

    /// Analog response curve response
    AnalogResponseCurve { device_id: String, curve: String },

    /// Analog deadzone set confirmation
    AnalogDeadzoneSet { device_id: String, percentage: u8 },

    /// Analog deadzone response
    AnalogDeadzone { device_id: String, percentage: u8 },

    /// Per-axis deadzone set confirmation
    AnalogDeadzoneXYSet {
        device_id: String,
        x_percentage: u8,
        y_percentage: u8,
    },

    /// Per-axis deadzone response
    AnalogDeadzoneXY {
        device_id: String,
        x_percentage: u8,
        y_percentage: u8,
    },

    /// Per-axis outer deadzone set confirmation
    AnalogOuterDeadzoneXYSet {
        device_id: String,
        x_percentage: u8,
        y_percentage: u8,
    },

    /// Per-axis outer deadzone response
    AnalogOuterDeadzoneXY {
        device_id: String,
        x_percentage: u8,
        y_percentage: u8,
    },

    /// D-pad mode set confirmation
    AnalogDpadModeSet { device_id: String, mode: String },

    /// D-pad mode response
    AnalogDpadMode { device_id: String, mode: String },

    /// LED color set confirmation
    LedColorSet {
        device_id: String,
        zone: LedZone,
        color: (u8, u8, u8),
    },

    /// LED color response
    LedColor {
        device_id: String,
        zone: LedZone,
        color: Option<(u8, u8, u8)>,
    },

    /// All LED colors response
    AllLedColors {
        device_id: String,
        colors: std::collections::HashMap<LedZone, (u8, u8, u8)>,
    },

    /// LED brightness set confirmation
    LedBrightnessSet {
        device_id: String,
        zone: Option<LedZone>,
        brightness: u8,
    },

    /// LED brightness response
    LedBrightness {
        device_id: String,
        zone: Option<LedZone>,
        brightness: u8,
    },

    /// LED pattern set confirmation
    LedPatternSet {
        device_id: String,
        pattern: LedPattern,
    },

    /// LED pattern response
    LedPattern {
        device_id: String,
        pattern: LedPattern,
    },

    /// Acknowledgment of focus change event
    FocusChangedAck { app_id: String },

    /// Hotkey registration successful
    HotkeyRegistered {
        device_id: String,
        key: String,
        modifiers: Vec<String>,
    },

    /// List of hotkey bindings for a device
    HotkeyList {
        device_id: String,
        bindings: Vec<HotkeyBinding>,
    },

    /// Hotkey removal successful
    HotkeyRemoved {
        device_id: String,
        key: String,
        modifiers: Vec<String>,
    },

    /// Auto-switch rules acknowledgment
    AutoSwitchRulesAck,

    /// Auto-switch rules response
    AutoSwitchRules { rules: Vec<AutoSwitchRule> },

    /// Analog calibration response
    AnalogCalibration {
        device_id: String,
        layer_id: usize,
        calibration: Option<AnalogCalibrationConfig>,
    },

    /// Analog calibration acknowledgment
    AnalogCalibrationAck,

    /// Analog input update (streamed to subscribers)
    AnalogInputUpdate {
        device_id: String,
        axis_x: f32, // -1.0 to 1.0
        axis_y: f32, // -1.0 to 1.0
    },

    /// Analog subscription acknowledgment
    AnalogInputSubscribed,

    /// Global macro settings response
    MacroSettings(MacroSettings),
}

/// Profile structure for organizing macros
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub macros: std::collections::HashMap<String, MacroEntry>,
}

/// Serialization helpers for the IPC protocol
pub fn serialize<T: Serialize>(msg: &T) -> Vec<u8> {
    bincode::serialize(msg).unwrap_or_else(|e| {
        tracing::error!("Failed to serialize message: {:?}", e);
        Vec::new()
    })
}

pub fn deserialize<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, bincode::Error> {
    bincode::deserialize(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_settings_serialization() {
        let settings = MacroSettings {
            latency_offset_ms: 10,
            jitter_pct: 0.05,
            capture_mouse: false,
        };

        let serialized = serialize(&settings);
        let deserialized: MacroSettings = deserialize(&serialized).unwrap();
        assert_eq!(deserialized, settings);
    }

    #[test]
    fn test_ipc_serialization() {
        let request = Request::GetDevices;
        let serialized = serialize(&request);
        let deserialized: Request = deserialize(&serialized).unwrap();
        assert!(matches!(deserialized, Request::GetDevices));
    }

    #[test]
    fn test_macro_entry_serialization() {
        let macro_entry = MacroEntry {
            name: "Test Macro".to_string(),
            trigger: KeyCombo {
                keys: vec![30, 40],  // A and D keys
                modifiers: vec![29], // Ctrl key
            },
            actions: vec![
                Action::KeyPress(30),
                Action::Delay(100),
                Action::KeyRelease(30),
            ],
            device_id: Some("test_device".to_string()),
            enabled: true,
            humanize: false,
            capture_mouse: false,
        };

        let serialized = serialize(&macro_entry);
        let deserialized: MacroEntry = deserialize(&serialized).unwrap();
        assert_eq!(deserialized.name, "Test Macro");
        assert_eq!(deserialized.trigger.keys, vec![30, 40]);
    }

    #[test]
    fn test_profile_ipc_serialization() {
        let request = Request::GetDeviceProfiles {
            device_id: "1532:0220".to_string(),
        };
        let serialized = serialize(&request);
        let deserialized: Request = deserialize(&serialized).unwrap();
        assert!(matches!(deserialized, Request::GetDeviceProfiles { .. }));

        let response = Response::DeviceProfiles {
            device_id: "1532:0220".to_string(),
            profiles: vec!["gaming".to_string(), "work".to_string()],
        };
        let serialized = serialize(&response);
        let deserialized: Response = deserialize(&serialized).unwrap();
        assert!(matches!(deserialized, Response::DeviceProfiles { .. }));
    }

    #[test]
    fn test_analog_deadzone_ipc_serialization() {
        // Test SetAnalogDeadzone request
        let request = Request::SetAnalogDeadzone {
            device_id: "1532:0220".to_string(),
            percentage: 50,
        };
        let serialized = serialize(&request);
        let deserialized: Request = deserialize(&serialized).unwrap();
        assert!(matches!(deserialized, Request::SetAnalogDeadzone { .. }));

        // Test GetAnalogDeadzone request
        let request = Request::GetAnalogDeadzone {
            device_id: "1532:0220".to_string(),
        };
        let serialized = serialize(&request);
        let deserialized: Request = deserialize(&serialized).unwrap();
        assert!(matches!(deserialized, Request::GetAnalogDeadzone { .. }));

        // Test AnalogDeadzoneSet response
        let response = Response::AnalogDeadzoneSet {
            device_id: "1532:0220".to_string(),
            percentage: 50,
        };
        let serialized = serialize(&response);
        let deserialized: Response = deserialize(&serialized).unwrap();
        assert_eq!(deserialized, response);

        // Test AnalogDeadzone response
        let response = Response::AnalogDeadzone {
            device_id: "1532:0220".to_string(),
            percentage: 43,
        };
        let serialized = serialize(&response);
        let deserialized: Response = deserialize(&serialized).unwrap();
        assert_eq!(deserialized, response);
    }

    #[test]
    fn test_mouse_action_serialization() {
        // Test individual mouse action variants serialize correctly
        let actions = vec![
            Action::MousePress(0x110), // BTN_LEFT
            Action::MouseRelease(0x110),
            Action::MouseMove(10, 20),
            Action::MouseScroll(5),
        ];

        for action in &actions {
            let serialized = serialize(action);
            let deserialized: Action = deserialize(&serialized).unwrap();
            assert_eq!(action, &deserialized);
        }

        // Test macro entry with mixed keyboard and mouse actions
        let macro_entry = MacroEntry {
            name: "Mixed Macro".to_string(),
            trigger: KeyCombo {
                keys: vec![30],
                modifiers: vec![],
            },
            actions: vec![
                Action::KeyPress(30),
                Action::MousePress(0x110),
                Action::Delay(50),
                Action::MouseRelease(0x110),
                Action::MouseMove(100, 200),
                Action::MouseScroll(3),
                Action::KeyRelease(30),
            ],
            device_id: Some("1532:0220".to_string()),
            enabled: true,
            humanize: false,
            capture_mouse: false,
        };

        let serialized = serialize(&macro_entry);
        let deserialized: MacroEntry = deserialize(&serialized).unwrap();
        assert_eq!(deserialized.name, "Mixed Macro");
        assert_eq!(deserialized.actions.len(), 7);

        // Verify each action type survived round-trip
        assert!(matches!(deserialized.actions[0], Action::KeyPress(30)));
        assert!(matches!(deserialized.actions[1], Action::MousePress(0x110)));
        assert!(matches!(deserialized.actions[2], Action::Delay(50)));
        assert!(matches!(
            deserialized.actions[3],
            Action::MouseRelease(0x110)
        ));
        assert!(matches!(
            deserialized.actions[4],
            Action::MouseMove(100, 200)
        ));
        assert!(matches!(deserialized.actions[5], Action::MouseScroll(3)));
        assert!(matches!(deserialized.actions[6], Action::KeyRelease(30)));
    }

    #[test]
    fn test_device_capabilities_serialization() {
        let caps = DeviceCapabilities {
            has_analog_stick: true,
            has_hat_switch: true,
            joystick_button_count: 26,
            led_zones: vec!["logo".to_string(), "keys".to_string()],
        };
        let serialized = serialize(&caps);
        let deserialized: DeviceCapabilities = deserialize(&serialized).unwrap();
        assert!(deserialized.has_analog_stick);
        assert!(deserialized.has_hat_switch);
        assert_eq!(deserialized.joystick_button_count, 26);
        assert_eq!(deserialized.led_zones.len(), 2);
    }

    #[test]
    fn test_get_device_capabilities_request() {
        let request = Request::GetDeviceCapabilities {
            device_path: "/dev/input/event0".to_string(),
        };
        let serialized = serialize(&request);
        let deserialized: Request = deserialize(&serialized).unwrap();
        assert!(matches!(
            deserialized,
            Request::GetDeviceCapabilities { .. }
        ));
        if let Request::GetDeviceCapabilities { device_path } = deserialized {
            assert_eq!(device_path, "/dev/input/event0");
        }
    }

    #[test]
    fn test_device_capabilities_response() {
        let response = Response::DeviceCapabilities {
            device_path: "/dev/input/event0".to_string(),
            capabilities: DeviceCapabilities {
                has_analog_stick: true,
                has_hat_switch: true,
                joystick_button_count: 26,
                led_zones: vec![],
            },
        };
        let serialized = serialize(&response);
        let deserialized: Response = deserialize(&serialized).unwrap();
        assert!(matches!(deserialized, Response::DeviceCapabilities { .. }));
    }

    #[test]
    fn test_layer_mode_serialization() {
        // Test Hold variant
        let hold_mode = LayerMode::Hold;
        let serialized = serialize(&hold_mode);
        let deserialized: LayerMode = deserialize(&serialized).unwrap();
        assert_eq!(deserialized, LayerMode::Hold);

        // Test Toggle variant
        let toggle_mode = LayerMode::Toggle;
        let serialized = serialize(&toggle_mode);
        let deserialized: LayerMode = deserialize(&serialized).unwrap();
        assert_eq!(deserialized, LayerMode::Toggle);
    }

    #[test]
    fn test_layer_config_info_serialization() {
        let config = LayerConfigInfo {
            layer_id: 1,
            name: "Gaming".to_string(),
            mode: LayerMode::Toggle,
            remap_count: 5,
            led_color: (0, 0, 255), // Default blue
            led_zone: None,
        };

        let serialized = serialize(&config);
        let deserialized: LayerConfigInfo = deserialize(&serialized).unwrap();

        assert_eq!(deserialized.layer_id, 1);
        assert_eq!(deserialized.name, "Gaming");
        assert_eq!(deserialized.mode, LayerMode::Toggle);
        assert_eq!(deserialized.remap_count, 5);
    }

    #[test]
    fn test_get_active_layer_request() {
        let request = Request::GetActiveLayer {
            device_id: "1532:0220".to_string(),
        };

        let serialized = serialize(&request);
        let deserialized: Request = deserialize(&serialized).unwrap();

        assert!(matches!(deserialized, Request::GetActiveLayer { .. }));
        if let Request::GetActiveLayer { device_id } = deserialized {
            assert_eq!(device_id, "1532:0220");
        }
    }

    #[test]
    fn test_active_layer_response() {
        let response = Response::ActiveLayer {
            device_id: "1532:0220".to_string(),
            layer_id: 2,
            layer_name: "Gaming".to_string(),
        };

        let serialized = serialize(&response);
        let deserialized: Response = deserialize(&serialized).unwrap();

        assert!(matches!(deserialized, Response::ActiveLayer { .. }));
        if let Response::ActiveLayer {
            device_id,
            layer_id,
            layer_name,
        } = deserialized
        {
            assert_eq!(device_id, "1532:0220");
            assert_eq!(layer_id, 2);
            assert_eq!(layer_name, "Gaming");
        }
    }

    #[test]
    fn test_set_layer_config_request() {
        let request = Request::SetLayerConfig {
            device_id: "1532:0220".to_string(),
            layer_id: 1,
            config: LayerConfigInfo {
                layer_id: 1,
                name: "Work".to_string(),
                mode: LayerMode::Hold,
                remap_count: 0,
                led_color: (0, 0, 255),
                led_zone: None,
            },
        };

        let serialized = serialize(&request);
        let deserialized: Request = deserialize(&serialized).unwrap();

        assert!(matches!(deserialized, Request::SetLayerConfig { .. }));
        if let Request::SetLayerConfig {
            device_id,
            layer_id,
            config,
        } = deserialized
        {
            assert_eq!(device_id, "1532:0220");
            assert_eq!(layer_id, 1);
            assert_eq!(config.name, "Work");
            assert_eq!(config.mode, LayerMode::Hold);
        }
    }

    #[test]
    fn test_activate_layer_request() {
        let request = Request::ActivateLayer {
            device_id: "1532:0220".to_string(),
            layer_id: 2,
            mode: LayerMode::Toggle,
        };

        let serialized = serialize(&request);
        let deserialized: Request = deserialize(&serialized).unwrap();

        assert!(matches!(deserialized, Request::ActivateLayer { .. }));
        if let Request::ActivateLayer {
            device_id,
            layer_id,
            mode,
        } = deserialized
        {
            assert_eq!(device_id, "1532:0220");
            assert_eq!(layer_id, 2);
            assert_eq!(mode, LayerMode::Toggle);
        }
    }

    #[test]
    fn test_list_layers_request() {
        let request = Request::ListLayers {
            device_id: "1532:0220".to_string(),
        };

        let serialized = serialize(&request);
        let deserialized: Request = deserialize(&serialized).unwrap();

        assert!(matches!(deserialized, Request::ListLayers { .. }));
        if let Request::ListLayers { device_id } = deserialized {
            assert_eq!(device_id, "1532:0220");
        }
    }

    #[test]
    fn test_layer_list_response() {
        let response = Response::LayerList {
            device_id: "1532:0220".to_string(),
            layers: vec![
                LayerConfigInfo {
                    layer_id: 0,
                    name: "Base".to_string(),
                    mode: LayerMode::Hold,
                    remap_count: 10,
                    led_color: (0, 255, 0),
                    led_zone: None,
                },
                LayerConfigInfo {
                    layer_id: 1,
                    name: "Gaming".to_string(),
                    mode: LayerMode::Toggle,
                    remap_count: 5,
                    led_color: (255, 0, 0),
                    led_zone: Some(LedZone::Side),
                },
            ],
        };

        let serialized = serialize(&response);
        let deserialized: Response = deserialize(&serialized).unwrap();

        assert!(matches!(deserialized, Response::LayerList { .. }));
        if let Response::LayerList { device_id, layers } = deserialized {
            assert_eq!(device_id, "1532:0220");
            assert_eq!(layers.len(), 2);
            assert_eq!(layers[0].name, "Base");
            assert_eq!(layers[1].name, "Gaming");
        }
    }

    #[test]
    fn test_layer_configured_response() {
        let response = Response::LayerConfigured {
            device_id: "1532:0220".to_string(),
            layer_id: 1,
        };

        let serialized = serialize(&response);
        let deserialized: Response = deserialize(&serialized).unwrap();

        assert!(matches!(deserialized, Response::LayerConfigured { .. }));
        if let Response::LayerConfigured {
            device_id,
            layer_id,
        } = deserialized
        {
            assert_eq!(device_id, "1532:0220");
            assert_eq!(layer_id, 1);
        }
    }

    #[test]
    fn test_focus_changed_request_serialization() {
        // Test with full app_id and window title
        let request = Request::FocusChanged {
            app_id: "org.alacritty".to_string(),
            window_title: Some("Alacritty: ~/Projects".to_string()),
        };

        let serialized = serialize(&request);
        let deserialized: Request = deserialize(&serialized).unwrap();

        assert!(matches!(deserialized, Request::FocusChanged { .. }));
        if let Request::FocusChanged {
            app_id,
            window_title,
        } = deserialized
        {
            assert_eq!(app_id, "org.alacritty");
            assert_eq!(window_title, Some("Alacritty: ~/Projects".to_string()));
        }

        // Test with flatpak-style app_id and no title
        let request = Request::FocusChanged {
            app_id: "org.mozilla.firefox".to_string(),
            window_title: None,
        };

        let serialized = serialize(&request);
        let deserialized: Request = deserialize(&serialized).unwrap();

        assert!(matches!(deserialized, Request::FocusChanged { .. }));
        if let Request::FocusChanged {
            app_id,
            window_title,
        } = deserialized
        {
            assert_eq!(app_id, "org.mozilla.firefox");
            assert_eq!(window_title, None);
        }

        // Test with simple app_id (suffix format)
        let request = Request::FocusChanged {
            app_id: "firefox".to_string(),
            window_title: Some("Mozilla Firefox".to_string()),
        };

        let serialized = serialize(&request);
        let deserialized: Request = deserialize(&serialized).unwrap();

        assert!(matches!(deserialized, Request::FocusChanged { .. }));
        if let Request::FocusChanged {
            app_id,
            window_title,
        } = deserialized
        {
            assert_eq!(app_id, "firefox");
            assert_eq!(window_title, Some("Mozilla Firefox".to_string()));
        }
    }

    #[test]
    fn test_focus_changed_ack_response_serialization() {
        let response = Response::FocusChangedAck {
            app_id: "org.alacritty".to_string(),
        };

        let serialized = serialize(&response);
        let deserialized: Response = deserialize(&serialized).unwrap();

        assert!(matches!(deserialized, Response::FocusChangedAck { .. }));
        if let Response::FocusChangedAck { ref app_id } = deserialized {
            assert_eq!(app_id, "org.alacritty");
        }

        // Test round-trip equality
        assert_eq!(deserialized, response);
    }

    #[test]
    fn test_analog_calibration_config_with_mode() {
        let config = AnalogCalibrationConfig {
            deadzone: 0.2,
            deadzone_shape: "circular".to_string(),
            sensitivity: "linear".to_string(),
            sensitivity_multiplier: 1.5,
            range_min: -32768,
            range_max: 32767,
            invert_x: false,
            invert_y: true,
            exponent: 2.0,
            analog_mode: AnalogMode::Wasd,
            camera_output_mode: None,
        };

        let serialized = serialize(&config);
        let deserialized: AnalogCalibrationConfig = deserialize(&serialized).unwrap();

        assert_eq!(deserialized.analog_mode, AnalogMode::Wasd);
        assert_eq!(deserialized.camera_output_mode, None);

        // Test Camera mode with output mode
        let camera_config = AnalogCalibrationConfig {
            analog_mode: AnalogMode::Camera,
            camera_output_mode: Some(CameraOutputMode::Keys),
            ..config
        };

        let serialized = serialize(&camera_config);
        let deserialized: AnalogCalibrationConfig = deserialize(&serialized).unwrap();

        assert_eq!(deserialized.analog_mode, AnalogMode::Camera);
        assert_eq!(
            deserialized.camera_output_mode,
            Some(CameraOutputMode::Keys)
        );
    }
}
