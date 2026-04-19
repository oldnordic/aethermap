use aethermap_common::{tracing, MacroEntry, Profile};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs;
use tracing::{debug, info, warn};

use crate::remap_engine::RemapProfile;
use crate::analog_calibration::AnalogCalibration;

/// A single key remapping entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemapEntry {
    /// Input key name (e.g., "KEY_A", "a", "capslock")
    pub from: String,
    /// Output key name (e.g., "KEY_B", "KEY_LEFTCTRL")
    pub to: String,
}

/// Global hotkey binding for manual profile switching
///
/// Defines a keyboard shortcut that triggers profile or layer activation.
/// Hotkeys are checked at the daemon level before remap processing.
///
/// # Example
///
/// ```yaml
/// hotkey_bindings:
///   - modifiers: ["ctrl", "alt", "shift"]
///     key: "1"
///     profile_name: "gaming"
///   - modifiers: ["ctrl", "alt", "shift"]
///     key: "2"
///     profile_name: "work"
///     layer_id: 1
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyBinding {
    /// Modifier keys (Ctrl, Alt, Shift, Super)
    ///
    /// Accepted values: "ctrl", "alt", "shift", "super" (case-insensitive)
    #[serde(default)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,

    /// Layer to activate (None = profile default)
    ///
    /// If set, activates the specified layer after switching profiles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer_id: Option<usize>,
}

impl HotkeyBinding {
    /// Create a new hotkey binding
    pub fn new(
        modifiers: Vec<String>,
        key: String,
        profile_name: String,
    ) -> Self {
        Self {
            modifiers,
            key,
            profile_name,
            device_id: None,
            layer_id: None,
        }
    }

    /// Create a new hotkey binding with device restriction
    pub fn with_device(
        modifiers: Vec<String>,
        key: String,
        profile_name: String,
        device_id: String,
    ) -> Self {
        Self {
            modifiers,
            key,
            profile_name,
            device_id: Some(device_id),
            layer_id: None,
        }
    }

    /// Create a new hotkey binding that activates a layer
    pub fn with_layer(
        modifiers: Vec<String>,
        key: String,
        profile_name: String,
        layer_id: usize,
    ) -> Self {
        Self {
            modifiers,
            key,
            profile_name,
            device_id: None,
            layer_id: Some(layer_id),
        }
    }

    /// Normalize modifier names to lowercase for consistent matching
    pub fn normalize_modifiers(&self) -> Vec<String> {
        self.modifiers.iter()
            .map(|m| m.to_lowercase())
            .collect()
    }
}

/// Default hotkey bindings for profile switching
///
/// Returns Ctrl+Alt+Shift+1 through Ctrl+Alt+Shift+9 for profiles 1-9
pub fn default_hotkey_bindings() -> Vec<HotkeyBinding> {
    let modifiers = vec!["ctrl".to_string(), "alt".to_string(), "shift".to_string()];
    let profile_names = vec![
        "profile1", "profile2", "profile3", "profile4", "profile5",
        "profile6", "profile7", "profile8", "profile9",
    ];

    profile_names.iter()
        .enumerate()
        .map(|(i, name)| {
            HotkeyBinding::new(
                modifiers.clone(),
                (i + 1).to_string(),
                name.to_string(),
            )
        })
        .collect()
}

/// Auto-profile switching rule
///
/// Defines when to automatically switch profiles based on focused application.
/// Rules are evaluated in order; the first matching rule wins.
///
/// # Example
///
/// ```yaml
/// auto_switch_rules:
///   - app_id: "org.alacritty"
///     profile_name: "terminal"
///     layer_id: 1
///   - app_id: "steam"
///     profile_name: "gaming"
///   - app_id: "*"
///     profile_name: "default"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,

    /// Layer ID to activate (0 = base, 1+ = additional layers)
    ///
    /// If None, uses profile's default layer (typically base layer 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer_id: Option<usize>,
}

/// Example device configuration with auto-switch rules and hotkeys
///
/// This constant provides a comprehensive example YAML configuration that
/// demonstrates auto-profile switching based on window focus and global
/// hotkey bindings for manual profile switching.
///
/// # Example YAML
///
/// ```yaml
/// devices:
///   "1532:0045":  # Razer Tartarus
///     name: "Razer Tartarus v2"
///     profiles:
///       gaming:
///         remaps:
///           - from: "KEY_1"
///             to: "KEY_F1"
///     # Auto-profile switching based on focused window
///     auto_switch_rules:
///       - app_id: "org.alacritty"     # Terminal emulator
///         profile_name: "terminal"
///         layer_id: 1
///       - app_id: "steam"             # Steam client/games
///         profile_name: "gaming"
///         layer_id: 2
///       - app_id: "firefox"           # Web browser
///         profile_name: "browser"
///       - app_id: "*"                 # Default/fallback
///         profile_name: "default"
///         layer_id: 0
///
///     # Global hotkey bindings (Ctrl+Alt+Shift+Number)
///     hotkey_bindings:
///       - modifiers: ["ctrl", "alt", "shift"]
///         key: "1"
///         profile_name: "gaming"
///         layer_id: 2
///       - modifiers: ["ctrl", "alt", "shift"]
///         key: "2"
///         profile_name: "terminal"
///         layer_id: 1
///       - modifiers: ["ctrl", "alt", "shift"]
///         key: "3"
///         profile_name: "browser"
/// ```
///
/// # Modifier Names
///
/// The following modifier names are supported (case-insensitive):
/// - `"ctrl"` - Control key (both LEFT and RIGHT)
/// - `"alt"` - Alt key (both LEFT and RIGHT)
/// - `"shift"` - Shift key (both LEFT and RIGHT)
/// - `"super"` - Super/Windows/Command key (both LEFT and RIGHT)
///
/// # App ID Patterns
///
/// Auto-switch rules support flexible app_id matching:
/// - `"*"` - Wildcard matching any application (use as fallback/default)
/// - `"org.alacritty"` - Exact match for app_id
/// - `"org.mozilla."` - Prefix match (matches org.mozilla.firefox, org.mozilla.thunderbird, etc.)
/// - `".firefox"` - Suffix match (matches org.mozilla.firefox, com.mozilla.firefox, etc.)
pub const EXAMPLE_CONFIG_WITH_AUTO_SWITCH: &str = r#"
# Example Aethermap device configuration
# with auto-switch rules and global hotkeys

devices:
  # Razer Tartarus v2 (1532:0045)
  "1532:0045":
    name: "Razer Tartarus v2"
    profiles:
      gaming:
        remaps:
          - from: "KEY_1"
            to: "KEY_F1"

    # Auto-profile switching rules
    # Rules are evaluated in order; first match wins
    auto_switch_rules:
      # Terminal emulator - activate terminal profile with layer 1
      - app_id: "org.alacritty"
        profile_name: "terminal"
        layer_id: 1

      # Steam games - activate gaming profile with layer 2
      - app_id: "steam"
        profile_name: "gaming"
        layer_id: 2

      # Web browser - activate browser profile
      - app_id: "firefox"
        profile_name: "browser"

      # Default/fallback - matches any app
      - app_id: "*"
        profile_name: "default"
        layer_id: 0

    # Global hotkey bindings for manual profile switching
    # Ctrl+Alt+Shift+Number switches to profile
    hotkey_bindings:
      # Ctrl+Alt+Shift+1 -> Gaming profile with layer 2
      - modifiers: ["ctrl", "alt", "shift"]
        key: "1"
        profile_name: "gaming"
        layer_id: 2

      # Ctrl+Alt+Shift+2 -> Terminal profile with layer 1
      - modifiers: ["ctrl", "alt", "shift"]
        key: "2"
        profile_name: "terminal"
        layer_id: 1

      # Ctrl+Alt+Shift+3 -> Browser profile
      - modifiers: ["ctrl", "alt", "shift"]
        key: "3"
        profile_name: "browser"
"#;

/// Example configuration showing per-layer analog mode configuration
///
/// This demonstrates how to configure different analog stick behaviors
/// for each layer:
/// - Layer 0 (Base): D-pad mode for menu navigation
/// - Layer 1: Gamepad mode for gaming
/// - Layer 2: Disabled (no analog output)
pub const EXAMPLE_CONFIG_WITH_ANALOG_MODES: &str = r#"
# Example Aethermap device configuration
# demonstrating per-layer analog mode configuration

devices:
  # Razer Tartarus v2 (1532:0045)
  "1532:0045":
    name: "Razer Tartarus v2"
    profiles:
      gaming:
        remaps:
          - from: "KEY_1"
            to: "KEY_F1"

    # Per-layer analog stick configuration
    # Each layer can have its own analog mode
    layers:
      # Layer 0 (Base): D-pad mode for menu navigation
      - layer_id: 0
        name: "Base"
        mode: "Hold"
        analog_mode: "Dpad"  # 8-way directional keys
        led_color: [255, 255, 255]
        led_zone: "Logo"

      # Layer 1: Gamepad mode for gaming
      - layer_id: 1
        name: "Gaming"
        mode: "Toggle"
        analog_mode: "Gamepad"  # Xbox 360 compatible gamepad
        led_color: [0, 0, 255]
        led_zone: "Logo"
        # Optional: per-layer analog calibration
        analog_calibration:
          deadzone: 0.15
          sensitivity_multiplier: 1.5

      # Layer 2: Disabled (no analog output)
      - layer_id: 2
        name: "Keyboard Only"
        mode: "Toggle"
        analog_mode: "Disabled"  # No analog output
        led_color: [0, 255, 0]
        led_zone: "Logo"

# Supported analog_mode values:
# - "Disabled": No output from analog stick
# - "Dpad": 8-way directional keys (hatswitch emulation)
# - "Gamepad": Xbox 360 compatible gamepad axes
# - "Camera": Scroll or key repeat (Phase 15)
# - "Mouse": Velocity-based cursor (Phase 15)
# - "Wasd": Directional keys (Phase 15)
"#;

/// Device capability metadata for Azeron and similar devices
///
/// This structure stores optional capability information that can be used
/// by the GUI for display purposes and profile validation. It is metadata
/// only - actual remapping uses the remaps list in profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// Device has an analog stick
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_analog_stick: Option<bool>,

    /// Device has a hat switch (D-pad)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_hat_switch: Option<bool>,

    /// Number of joystick buttons available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joystick_button_count: Option<usize>,

    /// LED zones available for control
    #[serde(skip_serializing_if = "Option::is_none")]
    pub led_zones: Option<Vec<String>>,

    /// Device type hint for GUI display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_type: Option<String>,

    /// Analog deadzone percentage (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analog_deadzone_percentage: Option<u8>,
}

/// Analog device configuration for stick calibration
///
/// Stores analog-specific settings like deadzone, sensitivity, and response curve.
/// This configuration is persisted per-device in YAML profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalogDeviceConfig {
    /// Deadzone percentage (0-100, default 43%)
    #[serde(default = "default_deadzone")]
    pub deadzone_percentage: u8,

    /// Deadzone percentage for X-axis (0-100, default 43%)
    #[serde(default = "default_deadzone")]
    pub deadzone_percentage_x: u8,

    /// Deadzone percentage for Y-axis (0-100, default 43%)
    #[serde(default = "default_deadzone")]
    pub deadzone_percentage_y: u8,

    /// Outer deadzone percentage (0-100, default 100% = no clamping)
    #[serde(default = "default_outer_deadzone")]
    pub outer_deadzone_percentage: u8,

    /// Outer deadzone percentage for X-axis (0-100, default 100% = no clamping)
    #[serde(default = "default_outer_deadzone")]
    pub outer_deadzone_percentage_x: u8,

    /// Outer deadzone percentage for Y-axis (0-100, default 100% = no clamping)
    #[serde(default = "default_outer_deadzone")]
    pub outer_deadzone_percentage_y: u8,

    /// Sensitivity multiplier (0.1-5.0, default 1.0)
    #[serde(default = "default_sensitivity")]
    pub sensitivity: f32,

    /// Response curve type ("linear" or "exponential" or "exponential(<exponent>)")
    #[serde(default = "default_response_curve")]
    pub response_curve: String,

    /// D-pad mode ("disabled", "eight_way", "four_way")
    #[serde(default = "default_dpad_mode")]
    pub dpad_mode: String,
}

fn default_deadzone() -> u8 {
    43  // ~43% of 32767 = ~14000 raw value
}

fn default_sensitivity() -> f32 {
    1.0
}

fn default_response_curve() -> String {
    "linear".to_string()
}

fn default_outer_deadzone() -> u8 {
    100  // 100% = no clamping
}

fn default_dpad_mode() -> String {
    "disabled".to_string()
}

impl Default for AnalogDeviceConfig {
    fn default() -> Self {
        Self {
            deadzone_percentage: default_deadzone(),
            deadzone_percentage_x: default_deadzone(),
            deadzone_percentage_y: default_deadzone(),
            outer_deadzone_percentage: default_outer_deadzone(),
            outer_deadzone_percentage_x: default_outer_deadzone(),
            outer_deadzone_percentage_y: default_outer_deadzone(),
            sensitivity: default_sensitivity(),
            response_curve: default_response_curve(),
            dpad_mode: default_dpad_mode(),
        }
    }
}

/// Device-specific remap configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRemapConfig {
    /// Device identifier (vendor:product format, e.g., "1532:0220")
    pub device_id: String,

    /// Named profiles for this device
    pub profiles: HashMap<String, DeviceProfile>,

    /// Device capabilities (for GUI display and validation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<DeviceCapabilities>,
}

/// A single device profile with remappings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    /// Profile name (matches the key in parent HashMap)
    #[serde(skip)]
    pub name: String,

    /// List of remap entries for this profile
    pub remaps: Vec<RemapEntry>,
}

/// Root structure for device profiles YAML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfilesConfig {
    /// Device configurations keyed by device_id (vendor:product)
    pub devices: HashMap<String, DeviceRemapConfig>,
}

/// Extended device remap configuration with match pattern support
///
/// This structure extends the basic device configuration with support for
/// flexible device matching patterns and profile descriptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedDeviceRemapConfig {
    /// Device match pattern (vendor:product, e.g., "1532:0220" or "1532:*")
    #[serde(default)]
    pub match_pattern: Option<String>,

    /// Named profiles for this device
    #[serde(default)]
    pub profiles: HashMap<String, ProfileRemaps>,

    /// Device capabilities (for GUI display and validation)
    #[serde(default)]
    pub capabilities: Option<DeviceCapabilities>,

    /// Analog device configuration (deadzone, sensitivity, response curve)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analog_config: Option<AnalogDeviceConfig>,

    /// Per-layer analog calibration configuration
    ///
    /// Maps layer_id (0=base, 1, 2...) to calibration settings.
    /// This allows different calibration profiles for different use cases
    /// (e.g., gaming layer with high sensitivity, work layer with precise low sensitivity).
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    pub analog_calibration: HashMap<usize, AnalogCalibration>,

    /// LED configuration (per-zone brightness, colors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub led_config: Option<LedConfig>,

    /// Global hotkey bindings for this device
    ///
    /// Hotkeys defined here trigger profile/layer switches when the key combination
    /// is pressed on any grabbed device. These are checked at the daemon level
    /// before remap processing.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub hotkey_bindings: Vec<HotkeyBinding>,
}

/// Analog calibration YAML format documentation
///
/// Per-layer analog calibration can be configured in device profiles:
///
/// ```yaml
/// devices:
///   "32b6:12f7":  # Azeron Cyborg
///     analog_calibration:
///       0:  # Base layer
///         deadzone: 0.15
///         deadzone_shape: circular
///         sensitivity: linear
///         sensitivity_multiplier: 1.0
///         range_min: -32768
///         range_max: 32767
///         invert_x: false
///         invert_y: false
///       1:  # Gaming layer
///         deadzone: 0.10  # Smaller deadzone for precision
///         sensitivity: quadratic  # More aggressive curve
///         sensitivity_multiplier: 1.5
///       2:  # Work layer
///         deadzone: 0.20  # Larger deadzone to prevent drift
///         sensitivity: linear
///         sensitivity_multiplier: 0.8  # Less sensitive
/// ```
///
/// If analog_calibration is not specified for a layer, AnalogCalibration::default() is used.

/// LED configuration for devices with configurable RGB lighting
///
/// Stores LED-specific settings like per-zone brightness and colors.
/// This configuration is persisted per-device in YAML profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedConfig {
    /// Per-zone brightness (0-100)
    /// Key is zone name: "logo", "keys", "thumbstick", "all", "global"
    #[serde(default)]
    pub zone_brightness: HashMap<String, u8>,

    /// Global brightness (0-100)
    #[serde(default = "default_global_brightness")]
    pub global_brightness: u8,

    /// Per-zone colors (RGB values 0-255)
    /// Key is zone name, value is (R, G, B) tuple
    #[serde(default)]
    pub zone_colors: HashMap<String, (u8, u8, u8)>,

    /// Active LED pattern
    #[serde(default = "default_active_pattern")]
    pub active_pattern: String,  // "static", "breathing", "rainbow", "rainbow_wave"

    /// Pattern animation speed (1-10)
    #[serde(default = "default_pattern_speed")]
    pub pattern_speed: u8,
}

fn default_global_brightness() -> u8 {
    100  // Default to 100% brightness
}

fn default_active_pattern() -> String {
    "static".to_string()
}

fn default_pattern_speed() -> u8 {
    5  // Default to medium speed
}

impl Default for LedConfig {
    fn default() -> Self {
        Self {
            zone_brightness: HashMap::new(),
            global_brightness: 100,
            zone_colors: HashMap::new(),
            active_pattern: default_active_pattern(),
            pattern_speed: default_pattern_speed(),
        }
    }
}

/// Convert LedPattern to string for YAML serialization
pub fn pattern_to_string(pattern: &crate::led_controller::LedPattern) -> &'static str {
    match pattern {
        crate::led_controller::LedPattern::Static => "static",
        crate::led_controller::LedPattern::Breathing => "breathing",
        crate::led_controller::LedPattern::Rainbow => "rainbow",
        crate::led_controller::LedPattern::RainbowWave => "rainbow_wave",
    }
}

/// Convert string to LedPattern
pub fn string_to_pattern(s: &str) -> Option<crate::led_controller::LedPattern> {
    match s {
        "static" => Some(crate::led_controller::LedPattern::Static),
        "breathing" => Some(crate::led_controller::LedPattern::Breathing),
        "rainbow" => Some(crate::led_controller::LedPattern::Rainbow),
        "rainbow_wave" => Some(crate::led_controller::LedPattern::RainbowWave),
        _ => None,
    }
}

/// Remap rules for a named profile with description
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileRemaps {
    /// Human-readable name
    #[serde(default)]
    pub name: String,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Key remappings as flat key:value pairs
    #[serde(default)]
    pub remaps: HashMap<String, String>,
}

/// Complete remap configuration with per-device profiles and default fallback
///
/// This is the top-level structure for the extended YAML configuration file.
/// It supports per-device profiles with optional match patterns and a global
/// default profile that applies when no device-specific profile is found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemapDevicesConfig {
    /// Per-device remap configurations keyed by device_id (vendor:product)
    #[serde(default)]
    pub devices: HashMap<String, ExtendedDeviceRemapConfig>,

    /// Global default remaps (applied when no device-specific profile)
    #[serde(default)]
    pub default: Option<HashMap<String, String>>,
}

/// Error type for remap configuration operations
#[derive(Debug)]
pub enum RemapConfigError {
    /// Error reading the remaps file
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Error parsing the YAML content
    ParseError {
        path: PathBuf,
        source: serde_yaml::Error,
    },

    /// Invalid key name in remaps configuration
    InvalidKey {
        path: PathBuf,
        key: String,
        expected: String,
    },

    /// Error writing the remaps file
    WriteError {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Validation error for configuration values
    Validation {
        field: String,
        message: String,
    },
}

impl std::fmt::Display for RemapConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemapConfigError::ReadError { path, source } => {
                write!(f, "Failed to read remaps file {}: {}", path.display(), source)
            }
            RemapConfigError::ParseError { path, source } => {
                write!(f, "Failed to parse remaps file {}: {}", path.display(), source)
            }
            RemapConfigError::InvalidKey { path, key, expected } => {
                write!(
                    f,
                    "Invalid key name '{}' in {}: expected {}",
                    key,
                    path.display(),
                    expected
                )
            }
            RemapConfigError::WriteError { path, source } => {
                write!(f, "Failed to write remaps file {}: {}", path.display(), source)
            }
            RemapConfigError::Validation { field, message } => {
                write!(f, "Validation error for '{}': {}", field, message)
            }
        }
    }
}

impl std::error::Error for RemapConfigError {}

/// Configuration manager for aethermap daemon
pub struct ConfigManager {
    pub config_path: PathBuf,
    pub macros_path: PathBuf,
    pub cache_path: PathBuf,
    pub profiles_dir: PathBuf,
    pub remaps_path: PathBuf,
    pub device_profiles_path: PathBuf,
    pub layer_state_path: PathBuf,
    /// Main daemon configuration (wrapped in RwLock for runtime updates)
    pub config: Arc<RwLock<DaemonConfig>>,
    pub macros: Arc<RwLock<HashMap<String, MacroEntry>>>,
    pub profiles: Arc<RwLock<HashMap<String, Profile>>>,
    pub remaps: Arc<RwLock<HashMap<String, String>>>,
    pub device_profiles: Arc<RwLock<HashMap<String, HashMap<String, RemapProfile>>>>,
}

/// Daemon configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub daemon: DaemonSettings,
    pub device_discovery: DeviceDiscoverySettings,
    pub macro_engine: MacroEngineSettings,
    pub config: ConfigSettings,
    pub security: SecuritySettings,
    pub led_control: LedControlSettings,
    pub performance: PerformanceSettings,
    /// Auto-profile switching rules (applied when window focus changes)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub auto_switch_rules: Vec<AutoSwitchRule>,
}

/// Daemon-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonSettings {
    pub socket_path: String,
    pub log_level: String,
    pub drop_privileges: bool,
}

/// Device discovery settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDiscoverySettings {
    pub input_devices_path: String,
    pub use_openrazer_db: bool,
    pub fallback_name_pattern: String,
}

/// Macro engine settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroEngineSettings {
    pub max_concurrent_macros: usize,
    pub default_delay: u32,
    pub enable_recording: bool,
    #[serde(default)]
    pub latency_offset_ms: u32,
    #[serde(default)]
    pub jitter_pct: f32,
}

/// Configuration persistence settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSettings {
    pub config_file: String,
    pub macros_file: String,
    pub cache_file: String,
    pub auto_save: bool,
    pub reload_interval: u64,
}

/// Security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub socket_group: String,
    pub socket_permissions: String,
    pub require_auth_token: bool,
    pub retain_capabilities: Vec<String>,
}

/// LED control settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedControlSettings {
    pub enabled: bool,
    pub interface: String,
    pub default_color: [u8; 3],
}

/// Performance settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSettings {
    pub device_poll_interval: u64,
    pub event_queue_size: usize,
    pub thread_pool: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            daemon: DaemonSettings {
                socket_path: "/run/aethermap/aethermap.sock".to_string(),
                log_level: "info".to_string(),
                drop_privileges: true,
            },
            device_discovery: DeviceDiscoverySettings {
                input_devices_path: "/dev/input/by-id".to_string(),
                use_openrazer_db: true,
                fallback_name_pattern: "Razer".to_string(),
            },
            macro_engine: MacroEngineSettings {
                max_concurrent_macros: 10,
                default_delay: 10,
                enable_recording: true,
                latency_offset_ms: 0,
                jitter_pct: 0.0,
            },
            config: ConfigSettings {
                config_file: "/etc/aethermap/config.yaml".to_string(),
                macros_file: "/etc/aethermap/macros.yaml".to_string(),
                cache_file: "/var/cache/aethermap/macros.bin".to_string(),
                auto_save: true,
                reload_interval: 30,
            },
            security: SecuritySettings {
                socket_group: "input".to_string(),
                socket_permissions: "0660".to_string(),
                require_auth_token: false,
                retain_capabilities: vec!["CAP_SYS_RAWIO".to_string()],
            },
            led_control: LedControlSettings {
                enabled: true,
                interface: "dbus".to_string(),
                default_color: [0, 255, 0],
            },
            performance: PerformanceSettings {
                device_poll_interval: 1,
                event_queue_size: 1000,
                thread_pool: true,
            },
            auto_switch_rules: Vec::new(),
        }
    }
}

impl ConfigManager {
    /// Create a new configuration manager with default paths
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = PathBuf::from("/etc/aethermap/config.yaml");
        let macros_path = PathBuf::from("/etc/aethermap/macros.yaml");
        let cache_path = PathBuf::from("/var/cache/aethermap/macros.bin");
        let profiles_dir = PathBuf::from("/etc/aethermap/profiles");
        let remaps_path = PathBuf::from("/etc/aethermap/remaps.yaml");
        let device_profiles_path = PathBuf::from("/etc/aethermap/device_profiles.yaml");
        let layer_state_path = PathBuf::from("/etc/aethermap/layer_state.yaml");

        let manager = Self {
            config_path,
            macros_path,
            cache_path,
            profiles_dir,
            remaps_path,
            device_profiles_path,
            layer_state_path,
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Ensure directories exist
        if let Some(parent) = manager.config_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        if let Some(parent) = manager.macros_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        if let Some(parent) = manager.cache_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::create_dir_all(&manager.profiles_dir).await?;
        // Ensure remaps directory exists
        if let Some(parent) = manager.remaps_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        // Ensure device_profiles directory exists
        if let Some(parent) = manager.device_profiles_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        // Ensure layer_state directory exists
        if let Some(parent) = manager.layer_state_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        Ok(manager)
    }

    /// Load configuration from disk
    pub async fn load_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Loading configuration from {}", self.config_path.display());

        if self.config_path.exists() {
            let content = fs::read_to_string(&self.config_path).await?;
            let new_config: DaemonConfig = serde_yaml::from_str(&content)?;
            *self.config.write().await = new_config;
            debug!("Loaded configuration from disk");
        } else {
            warn!("Configuration file not found, using defaults");
            self.save_config().await?;
        }

        // Try to load macros from cache first, then from YAML
        if self.cache_path.exists() {
            match self.load_macros_from_cache().await {
                Ok(()) => {
                    debug!("Loaded macros from cache");
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed to load macros from cache: {}", e);
                    // Fall back to YAML
                }
            }
        }

        if self.macros_path.exists() {
            self.load_macros_from_yaml().await?;
        } else {
            info!("No macros file found, creating empty macros");
            self.save_macros().await?;
        }

        Ok(())
    }

    /// Save configuration to disk
    pub async fn save_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Saving configuration to {}", self.config_path.display());

        let config = self.config.read().await;
        let content = serde_yaml::to_string(&*config)?;
        drop(config);
        fs::write(&self.config_path, content).await?;

        debug!("Configuration saved");
        Ok(())
    }

    /// Set global auto-switch rules and persist to config.yaml
    ///
    /// Updates the in-memory configuration and writes the complete daemon
    /// configuration to config.yaml. Auto-switch rules are global (not per-device)
    /// and control automatic profile switching based on window focus.
    ///
    /// # Arguments
    ///
    /// * `rules` - New auto-switch rules to set
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Rules updated and saved successfully
    /// * `Err(_)` - Serialization or write error
    ///
    /// # Example
    ///
    /// ```ignore
    /// let rules = vec![
    ///     AutoSwitchRule {
    ///         app_id: "steam".to_string(),
    ///         profile_name: "gaming".to_string(),
    ///         device_id: None,
    ///         layer_id: Some(2),
    ///     },
    /// ];
    /// config_manager.set_auto_switch_rules(rules).await?;
    /// ```
    pub async fn set_auto_switch_rules(
        &self,
        rules: Vec<AutoSwitchRule>,
    ) -> Result<(), RemapConfigError> {
        // Update in-memory config
        {
            let mut config = self.config.write().await;
            config.auto_switch_rules = rules.clone();
        }

        // Serialize the full config and write to config.yaml
        let config = self.config.read().await;
        let content = serde_yaml::to_string(&*config)
            .map_err(|e| RemapConfigError::WriteError {
                path: self.config_path.clone(),
                source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
            })?;
        drop(config);

        fs::write(&self.config_path, content)
            .await
            .map_err(|e| RemapConfigError::WriteError {
                path: self.config_path.clone(),
                source: e,
            })?;

        info!("Saved {} auto-switch rules to {}", rules.len(), self.config_path.display());
        Ok(())
    }

    /// Get global auto-switch rules from in-memory configuration
    ///
    /// Returns the current auto-switch rules without reading from disk.
    /// This is efficient for repeated queries as it reads from the
    /// cached in-memory configuration.
    ///
    /// # Returns
    ///
    /// Vector of auto-switch rules (empty if none configured)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let rules = config_manager.get_auto_switch_rules().await;
    /// for rule in rules {
    ///     println!("App '{}' -> Profile '{}'", rule.app_id, rule.profile_name);
    /// }
    /// ```
    pub async fn get_auto_switch_rules(&self) -> Vec<AutoSwitchRule> {
        let config = self.config.read().await;
        config.auto_switch_rules.clone()
    }

    /// Load macros from binary cache
    async fn load_macros_from_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Loading macros from cache {}", self.cache_path.display());

        let content = fs::read(&self.cache_path).await?;

        // First 4 bytes should be a magic number for verification
        if content.len() < 4 {
            return Err("Cache file too short".into());
        }

        let magic = u32::from_le_bytes([content[0], content[1], content[2], content[3]]);
        if magic != 0xDEADBEEF {
            return Err("Invalid cache file magic number".into());
        }

        let macros: HashMap<String, MacroEntry> = aethermap_common::deserialize(&content[4..])?;
        *self.macros.write().await = macros;

        debug!("Loaded macros from cache");
        Ok(())
    }

    /// Load macros from YAML file
    async fn load_macros_from_yaml(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Loading macros from {}", self.macros_path.display());

        let content = fs::read_to_string(&self.macros_path).await?;
        let macros: HashMap<String, MacroEntry> = serde_yaml::from_str(&content)?;
        *self.macros.write().await = macros;

        debug!("Loaded macros from YAML");
        Ok(())
    }

    /// Save macros to both cache and YAML
    pub async fn save_macros(&self) -> Result<(), Box<dyn std::error::Error>> {
        let macros = self.macros.read().await;

        // Save to cache
        self.save_macros_to_cache(&macros).await?;

        // Save to YAML
        self.save_macros_to_yaml(&macros).await?;

        debug!("Saved macros to both cache and YAML");
        Ok(())
    }

    /// Save macros to binary cache
    async fn save_macros_to_cache(&self, macros: &HashMap<String, MacroEntry>) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = Vec::new();

        // Add magic number
        data.extend_from_slice(&0xDEADBEEFu32.to_le_bytes());

        // Add serialized data
        let serialized = aethermap_common::serialize(macros);
        data.extend_from_slice(&serialized);

        fs::write(&self.cache_path, data).await?;
        debug!("Saved macros to cache");
        Ok(())
    }

    /// Save macros to YAML file
    async fn save_macros_to_yaml(&self, macros: &HashMap<String, MacroEntry>) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_yaml::to_string(macros)?;
        fs::write(&self.macros_path, content).await?;
        debug!("Saved macros to YAML");
        Ok(())
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &Arc<RwLock<DaemonConfig>> {
        &self.config
    }

    /// Get the layer state file path
    ///
    /// Returns the path where layer state snapshots are persisted.
    ///
    /// # Returns
    ///
    /// Reference to the layer_state_path
    pub fn layer_state_path(&self) -> &PathBuf {
        &self.layer_state_path
    }

    /// Get analog calibration for a specific device and layer
    ///
    /// Retrieves the per-layer analog calibration from the extended device profiles.
    /// This allows different calibration profiles for different layers (e.g., gaming
    /// vs work layer with different sensitivity settings).
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (e.g., "32b6:12f7")
    /// * `layer_id` - Layer ID (0=base, 1, 2, ...)
    ///
    /// # Returns
    ///
    /// * `Some(AnalogCalibration)` - Calibration configured for this device/layer
    /// * `None` - No layer-specific calibration (use AnalogCalibration::default())
    pub async fn get_analog_calibration(
        &self,
        device_id: &str,
        layer_id: usize,
    ) -> Option<AnalogCalibration> {
        // Read the extended device profiles
        if !self.device_profiles_path.exists() {
            return None;
        }

        let content = fs::read_to_string(&self.device_profiles_path).await.ok()?;
        let config: RemapDevicesConfig = serde_yaml::from_str(&content).ok()?;

        // Find the device configuration
        config.devices.get(device_id)
            .and_then(|device_config| device_config.analog_calibration.get(&layer_id))
            .cloned()
    }

    /// Get all analog calibrations for a device (all layers)
    ///
    /// Returns a HashMap of layer_id to AnalogCalibration for all layers
    /// configured for the specified device.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (e.g., "32b6:12f7")
    ///
    /// # Returns
    ///
    /// HashMap of layer_id -> AnalogCalibration (empty if none configured)
    pub async fn get_all_analog_calibrations(
        &self,
        device_id: &str,
    ) -> HashMap<usize, AnalogCalibration> {
        let mut calibrations = HashMap::new();

        if !self.device_profiles_path.exists() {
            return calibrations;
        }

        let Ok(content) = fs::read_to_string(&self.device_profiles_path).await else {
            return calibrations;
        };

        let Ok(config) = serde_yaml::from_str::<RemapDevicesConfig>(&content) else {
            return calibrations;
        };

        if let Some(device_config) = config.devices.get(device_id) {
            for (layer_id, calibration) in &device_config.analog_calibration {
                calibrations.insert(*layer_id, calibration.clone());
            }
        }

        calibrations
    }

    /// Save analog calibration for a device and layer
    ///
    /// This method updates the device_profiles.yaml file with the new calibration
    /// settings for a specific device and layer.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (e.g., "32b6:12f7")
    /// * `layer_id` - Layer ID (0=base, 1, 2, ...)
    /// * `calibration` - Calibration settings to save
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Calibration saved successfully
    /// * `Err(String)` - Error message if save failed
    pub async fn save_analog_calibration(
        &self,
        device_id: &str,
        layer_id: usize,
        calibration: AnalogCalibration,
    ) -> Result<(), String> {
        // Read existing config or create new
        let mut config = if self.device_profiles_path.exists() {
            let content = fs::read_to_string(&self.device_profiles_path)
                .await
                .map_err(|e| format!("Failed to read device profiles: {}", e))?;

            serde_yaml::from_str(&content)
                .map_err(|e| format!("Failed to parse device profiles: {}", e))?
        } else {
            RemapDevicesConfig {
                devices: HashMap::new(),
                default: None,
            }
        };

        // Ensure device entry exists
        if !config.devices.contains_key(device_id) {
            config.devices.insert(
                device_id.to_string(),
                ExtendedDeviceRemapConfig {
                    match_pattern: None,
                    profiles: HashMap::new(),
                    capabilities: None,
                    analog_config: None,
                    analog_calibration: HashMap::new(),
                    led_config: None,
                    hotkey_bindings: Vec::new(),
                }
            );
        }

        // Update calibration for the layer
        let device_config = config.devices.get_mut(device_id)
            .ok_or_else(|| format!("Device not found: {}", device_id))?;

        device_config.analog_calibration.insert(layer_id, calibration);

        // Write back to file
        let yaml = serde_yaml::to_string(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        // Ensure parent directory exists
        if let Some(parent) = self.device_profiles_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        fs::write(&self.device_profiles_path, yaml)
            .await
            .map_err(|e| format!("Failed to write device profiles: {}", e))?;

        info!(
            "Saved analog calibration for device {} layer {}",
            device_id, layer_id
        );

        Ok(())
    }

    /// Load configuration from disk (mutable version for use with Arc)
    pub async fn load_config_mut(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Loading configuration from {}", self.config_path.display());

        if self.config_path.exists() {
            let content = fs::read_to_string(&self.config_path).await?;
            let _config: DaemonConfig = serde_yaml::from_str(&content)?;
            // We can't replace self.config directly, so we'll update the fields
            // This is a limitation of using Arc<ConfigManager> without interior mutability
            debug!("Loaded configuration from disk");
        } else {
            warn!("Configuration file not found, using defaults");
            self.save_config().await?;
        }

        // Try to load macros from cache first, then from YAML
        if self.cache_path.exists() {
            match self.load_macros_from_cache().await {
                Ok(()) => {
                    debug!("Loaded macros from cache");
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed to load macros from cache: {}", e);
                    // Fall back to YAML
                }
            }
        }

        if self.macros_path.exists() {
            self.load_macros_from_yaml().await?;
        } else {
            info!("No macros file found, creating empty macros");
            self.save_macros().await?;
        }

        Ok(())
    }

    /// Get a reference to the macros
    pub fn macros(&self) -> &Arc<RwLock<HashMap<String, MacroEntry>>> {
        &self.macros
    }

    /// Get a profile by name
    pub async fn get_profile(&self, name: &str) -> Option<Profile> {
        let profiles = self.profiles.read().await;
        profiles.get(name).cloned()
    }

    /// Get all profiles
    pub async fn get_profiles(&self) -> std::collections::HashMap<String, Profile> {
        let profiles = self.profiles.read().await;
        profiles.clone()
    }

    /// Save a profile
    pub async fn save_profile(&self, profile: &Profile) -> Result<(), Box<dyn std::error::Error>> {
        let profile_path = self.profiles_dir.join(format!("{}.yaml", profile.name));

        // Save to YAML
        let yaml = serde_yaml::to_string(profile)?;
        fs::write(&profile_path, yaml).await?;

        // Update in-memory profiles
        let mut profiles = self.profiles.write().await;
        profiles.insert(profile.name.clone(), profile.clone());

        info!("Profile {} saved to {}", profile.name, profile_path.display());
        Ok(())
    }

    /// Load a profile by name
    pub async fn load_profile(&self, name: &str) -> Result<Profile, Box<dyn std::error::Error>> {
        let profile_path = self.profiles_dir.join(format!("{}.yaml", name));

        if !profile_path.exists() {
            return Err(format!("Profile {} not found", name).into());
        }

        let yaml = fs::read_to_string(&profile_path).await?;
        let profile: Profile = serde_yaml::from_str(&yaml)?;

        // Update in-memory profiles
        let mut profiles = self.profiles.write().await;
        profiles.insert(name.to_string(), profile.clone());

        // Load macros from profile into current macros
        let mut macros = self.macros.write().await;
        for (name, macro_entry) in &profile.macros {
            macros.insert(name.clone(), macro_entry.clone());
        }

        info!("Profile {} loaded from {}", name, profile_path.display());
        Ok(profile)
    }

    /// List all available profiles
    pub async fn list_profiles(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut entries = match fs::read_dir(&self.profiles_dir).await {
            Ok(entries) => entries,
            Err(e) => return Err(e.into()),
        };

        let mut profiles = Vec::new();
        while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    profiles.push(name.to_string());
                }
            }
        }

        profiles.sort();
        Ok(profiles)
    }

    /// Delete a profile
    pub async fn delete_profile(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let profile_path = self.profiles_dir.join(format!("{}.yaml", name));

        if profile_path.exists() {
            fs::remove_file(&profile_path).await?;
        }

        // Remove from in-memory profiles
        let mut profiles = self.profiles.write().await;
        profiles.remove(name);

        info!("Profile {} deleted", name);
        Ok(())
    }

    /// Save current macros as a new profile
    pub async fn save_current_macros_as_profile(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let macros = self.macros.read().await;
        let profile = Profile {
            name: name.to_string(),
            macros: macros.clone(),
        };

        self.save_profile(&profile).await
    }

    /// Get a specific profile for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format (e.g., "1532:0220")
    /// * `profile_name` - Name of the profile to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(RemapProfile)` - Profile if found
    /// * `None` - Profile not found
    pub async fn get_device_profile(&self, device_id: &str, profile_name: &str) -> Option<RemapProfile> {
        let profiles = self.device_profiles.read().await;
        profiles.get(device_id)?.get(profile_name).cloned()
    }

    /// List all available profiles for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format (e.g., "1532:0220")
    ///
    /// # Returns
    ///
    /// Vector of profile names available for the device
    pub async fn list_device_profiles(&self, device_id: &str) -> Vec<String> {
        let profiles = self.device_profiles.read().await;
        match profiles.get(device_id) {
            Some(device_profiles) => device_profiles.keys().cloned().collect(),
            None => Vec::new(),
        }
    }

    /// List all devices that have profiles configured
    pub async fn list_profile_devices(&self) -> Vec<String> {
        let profiles = self.device_profiles.read().await;
        profiles.keys().cloned().collect()
    }

    /// Load remaps from YAML configuration file
    ///
    /// This method loads key remappings from a flat YAML file where each line
    /// is a key:value pair. It validates all key names before returning any remaps.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<RemapEntry>)` - List of validated remapping entries
    /// * `Err(RemapConfigError)` - Error reading, parsing, or validating the remaps file
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let entries = manager.load_remaps().await?;
    /// for entry in entries {
    ///     println!("{} -> {}", entry.from, entry.to);
    /// }
    /// ```
    pub async fn load_remaps(&self) -> Result<Vec<RemapEntry>, RemapConfigError> {
        // Create empty file if it doesn't exist
        if !self.remaps_path.exists() {
            warn!(
                "Remaps file not found at {}, creating empty file",
                self.remaps_path.display()
            );
            let empty = HashMap::<String, String>::new();
            let yaml = serde_yaml::to_string(&empty).map_err(|e| RemapConfigError::WriteError {
                path: self.remaps_path.clone(),
                source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
            })?;
            fs::write(&self.remaps_path, yaml)
                .await
                .map_err(|e| RemapConfigError::WriteError {
                    path: self.remaps_path.clone(),
                    source: e,
                })?;
            return Ok(Vec::new());
        }

        // Read the file
        let content = fs::read_to_string(&self.remaps_path)
            .await
            .map_err(|e| RemapConfigError::ReadError {
                path: self.remaps_path.clone(),
                source: e,
            })?;

        // Parse flat YAML key:value pairs
        let remap_hash: HashMap<String, String> =
            serde_yaml::from_str(&content).map_err(|e| RemapConfigError::ParseError {
                path: self.remaps_path.clone(),
                source: e,
            })?;

        // Validate all key names exist (eager validation - fail-fast)
        let key_parser = crate::key_parser::KeyParser::new();
        let mut entries = Vec::new();

        for (from, to) in &remap_hash {
            // Validate input key
            if let Err(_e) = key_parser.parse(from) {
                return Err(RemapConfigError::InvalidKey {
                    path: self.remaps_path.clone(),
                    key: from.clone(),
                    expected: "valid evdev key name (e.g., KEY_A, a, capslock)".to_string(),
                });
            }

            // Validate output key
            if let Err(_e) = key_parser.parse(to) {
                return Err(RemapConfigError::InvalidKey {
                    path: self.remaps_path.clone(),
                    key: to.clone(),
                    expected: "valid evdev key name (e.g., KEY_A, a, capslock)".to_string(),
                });
            }

            entries.push(RemapEntry {
                from: from.clone(),
                to: to.clone(),
            });
        }

        // Store for later use
        *self.remaps.write().await = remap_hash;

        info!(
            "Loaded {} remaps from {}",
            entries.len(),
            self.remaps_path.display()
        );
        Ok(entries)
    }

    /// Reload device profiles from YAML configuration file with atomic swap
    ///
    /// This method reloads per-device remap profiles and atomically swaps the
    /// configuration. If validation fails, the existing configuration remains unchanged.
    ///
    /// This is the hot-reload variant of `load_device_profiles()` that performs an
    /// atomic swap using Arc pointer exchange. The operation is O(1) for the swap
    /// after validation completes.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Profiles reloaded successfully
    /// * `Err(RemapConfigError)` - Error reading, parsing, or validating the file
    ///
    /// # Hot-Reload Behavior
    ///
    /// - Configuration is validated before any changes are made
    /// - On validation success, the new config is atomically swapped
    /// - On validation failure, the existing config remains unchanged
    /// - Existing device remappings continue working during reload
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Trigger hot-reload via SIGHUP
    /// if let Err(e) = config_manager.reload_device_profiles().await {
    ///     error!("Failed to reload device profiles: {}", e);
    ///     // Daemon continues with old configuration
    /// }
    /// ```
    pub async fn reload_device_profiles(&self) -> Result<(), RemapConfigError> {
        info!(
            "Reloading device profiles from {}",
            self.device_profiles_path.display()
        );

        // Read the file
        let content = fs::read_to_string(&self.device_profiles_path)
            .await
            .map_err(|e| RemapConfigError::ReadError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        // Parse YAML
        let config: DeviceProfilesConfig = serde_yaml::from_str(&content)
            .map_err(|e| RemapConfigError::ParseError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        // Create a shared key parser for all profiles
        let key_parser = Arc::new(crate::key_parser::KeyParser::new());
        let mut all_profiles = HashMap::new();
        let mut total_profiles = 0usize;

        // Process each device
        for (device_id, device_config) in &config.devices {
            let mut device_profiles = HashMap::new();

            // Process each profile for this device
            for (profile_name, profile) in &device_config.profiles {
                // Convert remap entries to HashMap for RemapProfile
                let remap_config: HashMap<String, String> = profile.remaps.iter()
                    .map(|r| (r.from.clone(), r.to.clone()))
                    .collect();

                // Create RemapProfile with validation
                match RemapProfile::with_key_parser(
                    profile_name.clone(),
                    &remap_config,
                    key_parser.clone(),
                ) {
                    Ok(remap_profile) => {
                        debug!(
                            "Validated profile '{}' for device {} with {} remappings",
                            profile_name,
                            device_id,
                            remap_profile.remap_count().await
                        );
                        device_profiles.insert(profile_name.clone(), remap_profile);
                        total_profiles += 1;
                    }
                    Err(e) => {
                        return Err(RemapConfigError::InvalidKey {
                            path: self.device_profiles_path.clone(),
                            key: profile_name.clone(),
                            expected: format!("valid profile configuration: {}", e),
                        });
                    }
                }
            }

            all_profiles.insert(device_id.clone(), device_profiles);
        }

        // All validation passed - perform atomic swap
        // The Arc<RwLock<>> wrapper allows atomic swap by replacing the inner HashMap
        let mut config_guard = self.device_profiles.write().await;
        *config_guard = all_profiles;

        info!(
            "Reloaded {} profiles for {} devices from {}",
            total_profiles,
            config.devices.len(),
            self.device_profiles_path.display()
        );
        Ok(())
    }

    /// Reload global remappings from YAML configuration file
    ///
    /// This method reloads the global key remappings and applies them to the
    /// RemapEngine. If validation fails, the RemapEngine state remains unchanged.
    ///
    /// # Arguments
    ///
    /// * `remap_engine` - The RemapEngine to apply the new remappings to
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Remappings reloaded successfully
    /// * `Err(RemapConfigError)` - Error reading, parsing, or validating the file
    ///
    /// # Hot-Reload Behavior
    ///
    /// - Configuration is validated before any changes are made
    /// - On validation success, the new remappings are applied to RemapEngine
    /// - On validation failure, the existing remappings remain unchanged
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Trigger hot-reload via SIGHUP
    /// if let Err(e) = config_manager.reload_remaps(remap_engine).await {
    ///     error!("Failed to reload global remaps: {}", e);
    ///     // Daemon continues with old remappings
    /// }
    /// ```
    pub async fn reload_remaps(
        &self,
        remap_engine: Arc<crate::remap_engine::RemapEngine>,
    ) -> Result<(), RemapConfigError> {
        info!(
            "Reloading global remappings from {}",
            self.remaps_path.display()
        );

        // Load and validate remaps using existing method
        let entries = self.load_remaps().await?;

        // Convert to HashMap for RemapEngine
        let remap_hash: HashMap<String, String> = entries
            .iter()
            .map(|e| (e.from.clone(), e.to.clone()))
            .collect();

        // Apply to RemapEngine (validates before applying)
        remap_engine
            .load_config(&remap_hash)
            .await
            .map_err(|e| RemapConfigError::InvalidKey {
                path: self.remaps_path.clone(),
                key: "remap_config".to_string(),
                expected: format!("valid remap configuration: {}", e),
            })?;

        info!(
            "Reloaded {} global remappings from {}",
            remap_hash.len(),
            self.remaps_path.display()
        );
        Ok(())
    }

    /// Load device profiles from YAML configuration file
    ///
    /// This method loads per-device remap profiles from a hierarchical YAML file.
    /// The structure is:
    /// ```yaml
    /// devices:
    ///   "1532:0220":  # vendor:product
    ///     profiles:
    ///       gaming:
    ///         remaps:
    ///           - from: capslock
    ///             to: leftctrl
    ///       work:
    ///         remaps:
    ///           - from: a
    ///             to: b
    /// ```
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Profiles loaded successfully
    /// * `Err(RemapConfigError)` - Error reading, parsing, or validating the file
    pub async fn load_device_profiles(&self) -> Result<(), RemapConfigError> {
        // Create empty file if it doesn't exist
        if !self.device_profiles_path.exists() {
            warn!(
                "Device profiles file not found at {}, creating empty file",
                self.device_profiles_path.display()
            );
            let empty = DeviceProfilesConfig {
                devices: HashMap::new(),
            };
            let yaml = serde_yaml::to_string(&empty).map_err(|e| RemapConfigError::WriteError {
                path: self.device_profiles_path.clone(),
                source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
            })?;
            fs::write(&self.device_profiles_path, yaml)
                .await
                .map_err(|e| RemapConfigError::WriteError {
                    path: self.device_profiles_path.clone(),
                    source: e,
                })?;
            return Ok(());
        }

        // Read the file
        let content = fs::read_to_string(&self.device_profiles_path)
            .await
            .map_err(|e| RemapConfigError::ReadError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        // Parse YAML
        let config: DeviceProfilesConfig = serde_yaml::from_str(&content)
            .map_err(|e| RemapConfigError::ParseError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        // Create a shared key parser for all profiles
        let key_parser = Arc::new(crate::key_parser::KeyParser::new());
        let mut all_profiles = HashMap::new();

        // Process each device
        for (device_id, device_config) in &config.devices {
            let mut device_profiles = HashMap::new();

            // Process each profile for this device
            for (profile_name, profile) in &device_config.profiles {
                // Convert remap entries to HashMap for RemapProfile
                let remap_config: HashMap<String, String> = profile.remaps.iter()
                    .map(|r| (r.from.clone(), r.to.clone()))
                    .collect();

                // Create RemapProfile with validation
                match RemapProfile::with_key_parser(
                    profile_name.clone(),
                    &remap_config,
                    key_parser.clone(),
                ) {
                    Ok(remap_profile) => {
                        info!(
                            "Loaded profile '{}' for device {} with {} remappings",
                            profile_name,
                            device_id,
                            remap_profile.remap_count().await
                        );
                        device_profiles.insert(profile_name.clone(), remap_profile);
                    }
                    Err(e) => {
                        return Err(RemapConfigError::InvalidKey {
                            path: self.device_profiles_path.clone(),
                            key: profile_name.clone(),
                            expected: format!("valid profile configuration: {}", e),
                        });
                    }
                }
            }

            all_profiles.insert(device_id.clone(), device_profiles);
        }

        // Store all profiles
        *self.device_profiles.write().await = all_profiles;

        info!(
            "Loaded profiles for {} devices from {}",
            config.devices.len(),
            self.device_profiles_path.display()
        );
        Ok(())
    }

    /// Load per-device remap profiles from extended YAML configuration
    ///
    /// This method loads per-device remap profiles from a hierarchical YAML file
    /// with support for match patterns and profile descriptions. The structure is:
    /// ```yaml
    /// devices:
    ///   "1532:0220":  # vendor:product
    ///     match_pattern: "1532:0220"
    ///     profiles:
    ///       gaming:
    ///         name: "Gaming Profile"
    ///         description: "Profile for gaming"
    ///         remaps:
    ///           capslock: leftctrl
    ///           a: b
    /// default:
    ///   capslock: leftctrl
    /// ```
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<String, Vec<RemapProfile>>)` - Map of device_id to list of profiles
    /// * `Err(RemapConfigError)` - Error reading, parsing, or validating the file
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let profiles = manager.load_device_profiles_extended().await?;
    /// for (device_id, profile_list) in profiles {
    ///     for profile in profile_list {
    ///         println!("Device {} has profile {}", device_id, profile.name());
    ///     }
    /// }
    /// ```
    pub async fn load_device_profiles_extended(
        &self,
    ) -> Result<HashMap<String, Vec<RemapProfile>>, RemapConfigError> {
        use std::sync::Arc;

        // Create empty file if it doesn't exist
        if !self.device_profiles_path.exists() {
            warn!(
                "Device profiles file not found at {}, creating empty file",
                self.device_profiles_path.display()
            );
            let empty = RemapDevicesConfig {
                devices: HashMap::new(),
                default: None,
            };
            let yaml = serde_yaml::to_string(&empty).map_err(|e| RemapConfigError::WriteError {
                path: self.device_profiles_path.clone(),
                source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
            })?;
            fs::write(&self.device_profiles_path, yaml)
                .await
                .map_err(|e| RemapConfigError::WriteError {
                    path: self.device_profiles_path.clone(),
                    source: e,
                })?;
            return Ok(HashMap::new());
        }

        // Read the file
        let content = fs::read_to_string(&self.device_profiles_path)
            .await
            .map_err(|e| RemapConfigError::ReadError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        // Parse YAML
        let config: RemapDevicesConfig = serde_yaml::from_str(&content)
            .map_err(|e| RemapConfigError::ParseError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        // Create a shared key parser for all profiles
        let key_parser = Arc::new(crate::key_parser::KeyParser::new());
        let mut result: HashMap<String, Vec<RemapProfile>> = HashMap::new();

        // Process each device
        for (device_id, device_config) in &config.devices {
            let mut profiles = Vec::new();

            // Process each profile for this device
            for (profile_name, profile_config) in &device_config.profiles {
                // Create RemapProfile with validation
                match RemapProfile::with_key_parser(
                    profile_name.clone(),
                    &profile_config.remaps,
                    key_parser.clone(),
                ) {
                    Ok(remap_profile) => {
                        info!(
                            "Loaded profile '{}' for device {} with {} remappings",
                            profile_name,
                            device_id,
                            remap_profile.remap_count().await
                        );
                        profiles.push(remap_profile);
                    }
                    Err(e) => {
                        return Err(RemapConfigError::InvalidKey {
                            path: self.device_profiles_path.clone(),
                            key: profile_name.clone(),
                            expected: format!("valid profile configuration: {}", e),
                        });
                    }
                }
            }

            // Log analog calibration entries for this device
            if !device_config.analog_calibration.is_empty() {
                debug!(
                    "Loaded {} analog calibration entries for device {}",
                    device_config.analog_calibration.len(),
                    device_id
                );
            }

            result.insert(device_id.clone(), profiles);
        }

        info!(
            "Loaded {} device profiles from {}",
            result.len(),
            self.device_profiles_path.display()
        );
        Ok(result)
    }

    /// Load analog configuration for devices from extended profiles
    ///
    /// Returns a map of device_id to AnalogDeviceConfig, parsed from the
    /// extended device profiles YAML file. This configuration can be applied
    /// to the AnalogProcessor when devices are grabbed.
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap)` - Map of device_id to analog configuration
    /// * `Err(RemapConfigError)` - Error reading or parsing the file
    pub async fn load_analog_configs(
        &self,
    ) -> Result<HashMap<String, AnalogDeviceConfig>, RemapConfigError> {
        // Create empty file if it doesn't exist
        if !self.device_profiles_path.exists() {
            return Ok(HashMap::new());
        }

        // Read the file
        let content = fs::read_to_string(&self.device_profiles_path)
            .await
            .map_err(|e| RemapConfigError::ReadError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        // Parse YAML
        let config: RemapDevicesConfig = serde_yaml::from_str(&content)
            .map_err(|e| RemapConfigError::ParseError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        let mut result: HashMap<String, AnalogDeviceConfig> = HashMap::new();

        // Process each device and extract analog_config if present
        for (device_id, device_config) in &config.devices {
            if let Some(analog_config) = &device_config.analog_config {
                info!(
                    "Loaded analog config for device {}: deadzone={}%, sensitivity={:.2}, curve={}",
                    device_id,
                    analog_config.deadzone_percentage,
                    analog_config.sensitivity,
                    analog_config.response_curve
                );
                result.insert(device_id.clone(), analog_config.clone());
            }
        }

        info!(
            "Loaded analog configs for {} devices from {}",
            result.len(),
            self.device_profiles_path.display()
        );
        Ok(result)
    }

    /// Save analog configuration for a device to the YAML profile
    ///
    /// Updates the analog_config section for a device in device_profiles.yaml.
    /// If the device doesn't exist in the config, it will be added with an empty profiles list.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    /// * `analog_config` - Analog configuration to save
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Configuration saved successfully
    /// * `Err(RemapConfigError)` - Error reading, parsing, or writing the file
    pub async fn save_analog_config(
        &self,
        device_id: &str,
        analog_config: &AnalogDeviceConfig,
    ) -> Result<(), RemapConfigError> {
        info!(
            "Saving analog config for device {}: deadzone={}%, sensitivity={:.2}, curve={}, dpad={}",
            device_id,
            analog_config.deadzone_percentage,
            analog_config.sensitivity,
            analog_config.response_curve,
            analog_config.dpad_mode
        );

        // Read existing config or create new one
        let mut config: RemapDevicesConfig = if self.device_profiles_path.exists() {
            let content = fs::read_to_string(&self.device_profiles_path)
                .await
                .map_err(|e| RemapConfigError::ReadError {
                    path: self.device_profiles_path.clone(),
                    source: e,
                })?;

            serde_yaml::from_str(&content).map_err(|e| RemapConfigError::ParseError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?
        } else {
            RemapDevicesConfig {
                devices: HashMap::new(),
                default: None,
            }
        };

        // Update or insert device config with analog_config
        let device_entry = config.devices.entry(device_id.to_string()).or_insert_with(|| {
            ExtendedDeviceRemapConfig {
                match_pattern: Some(device_id.to_string()),
                profiles: HashMap::new(),
                capabilities: None,
                analog_config: None,
                analog_calibration: HashMap::new(),
                led_config: None,
                hotkey_bindings: Vec::new(),
            }
        });

        // Update analog_config
        device_entry.analog_config = Some(analog_config.clone());

        // Write back to file
        let yaml = serde_yaml::to_string(&config).map_err(|e| RemapConfigError::WriteError {
            path: self.device_profiles_path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
        })?;

        fs::write(&self.device_profiles_path, yaml)
            .await
            .map_err(|e| RemapConfigError::WriteError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        info!(
            "Analog config saved for device {} to {}",
            device_id,
            self.device_profiles_path.display()
        );
        Ok(())
    }

    /// Save LED configuration for a device to the YAML profile
    ///
    /// Updates the led_config section for a device in device_profiles.yaml.
    /// If the device doesn't exist in the config, it will be added with an empty profiles list.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    /// * `led_config` - LED configuration to save
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Configuration saved successfully
    /// * `Err(RemapConfigError)` - Error reading, parsing, or writing the file
    pub async fn save_led_config(
        &self,
        device_id: &str,
        led_config: &LedConfig,
    ) -> Result<(), RemapConfigError> {
        info!(
            "Saving LED config for device {}: global_brightness={}%, zone_brightness={:?}",
            device_id,
            led_config.global_brightness,
            led_config.zone_brightness
        );

        // Read existing config or create new one
        let mut config: RemapDevicesConfig = if self.device_profiles_path.exists() {
            let content = fs::read_to_string(&self.device_profiles_path)
                .await
                .map_err(|e| RemapConfigError::ReadError {
                    path: self.device_profiles_path.clone(),
                    source: e,
                })?;

            serde_yaml::from_str(&content).map_err(|e| RemapConfigError::ParseError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?
        } else {
            RemapDevicesConfig {
                devices: HashMap::new(),
                default: None,
            }
        };

        // Update or insert device config with led_config
        let device_entry = config.devices.entry(device_id.to_string()).or_insert_with(|| {
            ExtendedDeviceRemapConfig {
                match_pattern: Some(device_id.to_string()),
                profiles: HashMap::new(),
                capabilities: None,
                analog_config: None,
                analog_calibration: HashMap::new(),
                led_config: None,
                hotkey_bindings: Vec::new(),
            }
        });

        // Update led_config
        device_entry.led_config = Some(led_config.clone());

        // Write back to file
        let yaml = serde_yaml::to_string(&config).map_err(|e| RemapConfigError::WriteError {
            path: self.device_profiles_path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
        })?;

        fs::write(&self.device_profiles_path, yaml)
            .await
            .map_err(|e| RemapConfigError::WriteError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        info!(
            "LED config saved for device {} to {}",
            device_id,
            self.device_profiles_path.display()
        );
        Ok(())
    }

    /// Load LED configuration for devices from extended profiles
    ///
    /// Returns a map of device_id to LedConfig, parsed from the
    /// extended device profiles YAML file. This configuration can be applied
    /// to the LedController when devices are initialized.
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap)` - Map of device_id to LED configuration
    /// * `Err(RemapConfigError)` - Error reading or parsing the file
    pub async fn load_led_configs(
        &self,
    ) -> Result<HashMap<String, LedConfig>, RemapConfigError> {
        // Create empty file if it doesn't exist
        if !self.device_profiles_path.exists() {
            return Ok(HashMap::new());
        }

        // Read the file
        let content = fs::read_to_string(&self.device_profiles_path)
            .await
            .map_err(|e| RemapConfigError::ReadError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        // Parse YAML
        let config: RemapDevicesConfig = serde_yaml::from_str(&content)
            .map_err(|e| RemapConfigError::ParseError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        let mut result: HashMap<String, LedConfig> = HashMap::new();

        // Process each device and extract led_config if present
        for (device_id, device_config) in &config.devices {
            if let Some(led_config) = &device_config.led_config {
                info!(
                    "Loaded LED config for device {}: global_brightness={}%, zones={}",
                    device_id,
                    led_config.global_brightness,
                    led_config.zone_brightness.len()
                );
                result.insert(device_id.clone(), led_config.clone());
            }
        }

        info!(
            "Loaded LED configs for {} devices from {}",
            result.len(),
            self.device_profiles_path.display()
        );
        Ok(result)
    }

    /// Get LED configuration for a specific device
    ///
    /// Returns the LED configuration for a specific device, or a default
    /// configuration if the device has no LED configuration saved.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    ///
    /// # Returns
    ///
    /// * `LedConfig` - The LED configuration for the device
    pub async fn get_led_config(&self, device_id: &str) -> LedConfig {
        // Load all LED configs and return the specific device's config
        match self.load_led_configs().await {
            Ok(configs) => configs.get(device_id).cloned().unwrap_or_default(),
            Err(_) => LedConfig::default(),
        }
    }

    /// Helper function to convert brightness percentage to raw value
    ///
    /// Converts a 0-100 percentage to a raw value used by the HID protocol.
    /// For many devices, raw values are 0-255.
    ///
    /// # Arguments
    ///
    /// * `brightness` - Brightness percentage (0-100)
    ///
    /// # Returns
    ///
    /// Raw brightness value (0-255)
    pub fn brightness_to_raw(brightness: u8) -> u8 {
        // Convert 0-100 percentage to raw value (0-255)
        // Formula: (brightness * 255) / 100
        (brightness as u16 * 255 / 100) as u8
    }

    /// Helper function to convert raw brightness to percentage
    ///
    /// Converts a raw value to a 0-100 percentage for display/storage.
    ///
    /// # Arguments
    ///
    /// * `raw` - Raw brightness value (0-255)
    ///
    /// # Returns
    ///
    /// Brightness percentage (0-100)
    pub fn raw_to_brightness(raw: u8) -> u8 {
        // Convert raw value (0-255) to percentage (0-100)
        // Formula: (raw * 100) / 255
        (raw as u16 * 100 / 255) as u8
    }

    /// Convert zone name string to LedZone
    ///
    /// Parses a zone name string ("logo", "keys", "thumbstick", "all", "global")
    /// and returns the corresponding LedZone variant.
    ///
    /// # Arguments
    ///
    /// * `zone_name` - Zone name as a string
    ///
    /// # Returns
    ///
    /// * `Some(LedZone)` - If the zone name is valid
    /// * `None` - If the zone name is not recognized
    pub fn parse_led_zone(zone_name: &str) -> Option<aethermap_common::LedZone> {
        match zone_name.to_lowercase().as_str() {
            "side" => Some(aethermap_common::LedZone::Side),
            "logo" => Some(aethermap_common::LedZone::Logo),
            "keys" => Some(aethermap_common::LedZone::Keys),
            "thumbstick" => Some(aethermap_common::LedZone::Thumbstick),
            "all" => Some(aethermap_common::LedZone::All),
            "global" => Some(aethermap_common::LedZone::Global),
            _ => None,
        }
    }

    /// Convert LedZone to zone name string
    ///
    /// Converts a LedZone variant to its string representation.
    ///
    /// # Arguments
    ///
    /// * `zone` - The LedZone to convert
    ///
    /// # Returns
    ///
    /// Zone name as a string
    pub fn led_zone_to_string(zone: aethermap_common::LedZone) -> String {
        match zone {
            aethermap_common::LedZone::Side => "side".to_string(),
            aethermap_common::LedZone::Logo => "logo".to_string(),
            aethermap_common::LedZone::Keys => "keys".to_string(),
            aethermap_common::LedZone::Thumbstick => "thumbstick".to_string(),
            aethermap_common::LedZone::All => "all".to_string(),
            aethermap_common::LedZone::Global => "global".to_string(),
        }
    }

    /// Get default LED color for a layer
    ///
    /// Returns a default RGB color for each layer to provide visual distinction.
    /// Base layer is white, other layers have distinct colors.
    ///
    /// # Arguments
    ///
    /// * `layer_id` - Layer ID (0 = base, 1+ = additional layers)
    ///
    /// # Returns
    ///
    /// RGB color tuple (red, green, blue) with values 0-255
    pub fn get_default_layer_color(layer_id: usize) -> (u8, u8, u8) {
        match layer_id {
            0 => (255, 255, 255),  // Base layer: white
            1 => (0, 0, 255),      // Layer 1: blue
            2 => (0, 255, 0),      // Layer 2: green
            3 => (255, 0, 0),      // Layer 3: red
            4 => (255, 255, 0),    // Layer 4: yellow
            5 => (255, 0, 255),    // Layer 5: magenta
            6 => (0, 255, 255),    // Layer 6: cyan
            7 => (255, 128, 0),    // Layer 7: orange
            8 => (128, 0, 255),    // Layer 8: purple
            9 => (0, 128, 255),    // Layer 9: light blue
            _ => (128, 128, 128),  // Others: gray
        }
    }

    /// Add a hotkey binding to a device's configuration
    ///
    /// Adds a hotkey binding to the device's extended configuration in device_profiles.yaml.
    /// Follows the same pattern as set_analog_calibration for YAML persistence.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    /// * `binding` - HotkeyBinding to add
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Binding added successfully
    /// * `Err(RemapConfigError::Validation)` - Duplicate binding (same key + modifiers)
    /// * `Err(RemapConfigError)` - Error reading, parsing, or writing the file
    pub async fn add_hotkey_binding(
        &self,
        device_id: &str,
        binding: HotkeyBinding,
    ) -> Result<(), RemapConfigError> {
        info!(
            "Adding hotkey binding for device {}: key={:?}, modifiers={:?}, profile={}",
            device_id, binding.key, binding.modifiers, binding.profile_name
        );

        // Read existing config or create new one
        let mut config: RemapDevicesConfig = if self.device_profiles_path.exists() {
            let content = fs::read_to_string(&self.device_profiles_path)
                .await
                .map_err(|e| RemapConfigError::ReadError {
                    path: self.device_profiles_path.clone(),
                    source: e,
                })?;

            serde_yaml::from_str(&content).map_err(|e| RemapConfigError::ParseError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?
        } else {
            RemapDevicesConfig {
                devices: HashMap::new(),
                default: None,
            }
        };

        // Get or create device entry with full ExtendedDeviceRemapConfig default
        let device_entry = config.devices.entry(device_id.to_string()).or_insert_with(|| {
            ExtendedDeviceRemapConfig {
                match_pattern: Some(device_id.to_string()),
                profiles: HashMap::new(),
                capabilities: None,
                analog_config: None,
                analog_calibration: HashMap::new(),
                led_config: None,
                hotkey_bindings: Vec::new(),
            }
        });

        // Check for duplicate (same key + modifiers)
        let normalized_binding_modifiers: Vec<String> = binding.modifiers.iter()
            .map(|m| m.to_lowercase())
            .collect();

        let is_duplicate = device_entry.hotkey_bindings.iter().any(|existing| {
            let normalized_existing: Vec<String> = existing.modifiers.iter()
                .map(|m| m.to_lowercase())
                .collect();
            existing.key == binding.key && normalized_existing == normalized_binding_modifiers
        });

        if is_duplicate {
            return Err(RemapConfigError::Validation {
                field: "hotkey".to_string(),
                message: "Hotkey with this key and modifiers already exists".to_string(),
            });
        }

        // Push binding to device_entry.hotkey_bindings
        device_entry.hotkey_bindings.push(binding);

        // Serialize to YAML
        let yaml = serde_yaml::to_string(&config).map_err(|e| RemapConfigError::WriteError {
            path: self.device_profiles_path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
        })?;

        // Write to device_profiles_path
        fs::write(&self.device_profiles_path, yaml)
            .await
            .map_err(|e| RemapConfigError::WriteError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        info!(
            "Added hotkey binding for device {} to {}",
            device_id,
            self.device_profiles_path.display()
        );
        Ok(())
    }

    /// Remove a hotkey binding from a device's configuration
    ///
    /// Removes a hotkey binding matching the specified key and modifiers.
    /// Uses retain() to filter out matching bindings.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    /// * `key` - Key name to match (e.g., "1", "f1")
    /// * `modifiers` - Modifiers to match (e.g., ["ctrl", "alt", "shift"])
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Binding removed successfully
    /// * `Err(RemapConfigError::Validation)` - Binding not found
    /// * `Err(RemapConfigError)` - Error reading, parsing, or writing the file
    pub async fn remove_hotkey_binding(
        &self,
        device_id: &str,
        key: &str,
        modifiers: &[String],
    ) -> Result<(), RemapConfigError> {
        info!(
            "Removing hotkey binding for device {}: key={:?}, modifiers={:?}",
            device_id, key, modifiers
        );

        // Return error if file doesn't exist
        if !self.device_profiles_path.exists() {
            return Err(RemapConfigError::Validation {
                field: "hotkey".to_string(),
                message: "Hotkey not found".to_string(),
            });
        }

        // Read existing config
        let content = fs::read_to_string(&self.device_profiles_path)
            .await
            .map_err(|e| RemapConfigError::ReadError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        let mut config: RemapDevicesConfig = serde_yaml::from_str(&content)
            .map_err(|e| RemapConfigError::ParseError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        // Get device entry, return error if device_id not found
        let device_entry = config.devices.get_mut(device_id)
            .ok_or_else(|| RemapConfigError::Validation {
                field: "hotkey".to_string(),
                message: "Hotkey not found".to_string(),
            })?;

        // Get original length of hotkey_bindings
        let original_len = device_entry.hotkey_bindings.len();

        // Normalize input modifiers for comparison
        let normalized_modifiers: Vec<String> = modifiers.iter()
            .map(|m| m.to_lowercase())
            .collect();

        // Use retain() to keep bindings that DON'T match
        device_entry.hotkey_bindings.retain(|binding| {
            let normalized_binding: Vec<String> = binding.modifiers.iter()
                .map(|m| m.to_lowercase())
                .collect();
            !(binding.key == key && normalized_binding == normalized_modifiers)
        });

        // If length unchanged (no binding removed), return error
        if device_entry.hotkey_bindings.len() == original_len {
            return Err(RemapConfigError::Validation {
                field: "hotkey".to_string(),
                message: "Hotkey not found".to_string(),
            });
        }

        // Serialize to YAML
        let yaml = serde_yaml::to_string(&config).map_err(|e| RemapConfigError::WriteError {
            path: self.device_profiles_path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
        })?;

        // Write to device_profiles_path
        fs::write(&self.device_profiles_path, yaml)
            .await
            .map_err(|e| RemapConfigError::WriteError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        info!(
            "Removed hotkey binding for device {}",
            device_id
        );
        Ok(())
    }

    /// Get all hotkey bindings for a device
    ///
    /// Returns all hotkey bindings configured for a device.
    /// Uses graceful degradation: missing file or device returns empty Vec.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<HotkeyBinding>)` - Vector of hotkey bindings (empty if none configured)
    pub async fn get_hotkey_bindings(
        &self,
        device_id: &str,
    ) -> Result<Vec<HotkeyBinding>, RemapConfigError> {
        // Return empty Vec if file doesn't exist (not an error)
        if !self.device_profiles_path.exists() {
            return Ok(Vec::new());
        }

        // Read the file
        let content = fs::read_to_string(&self.device_profiles_path)
            .await
            .map_err(|e| RemapConfigError::ReadError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        // Parse YAML
        let config: RemapDevicesConfig = serde_yaml::from_str(&content)
            .map_err(|e| RemapConfigError::ParseError {
                path: self.device_profiles_path.clone(),
                source: e,
            })?;

        // Get device entry, return empty Vec if device_id not found (not an error)
        match config.devices.get(device_id) {
            Some(device_entry) => Ok(device_entry.hotkey_bindings.clone()),
            None => Ok(Vec::new()),
        }
    }

    /// Get all hotkey bindings from all devices
    ///
    /// Reads the device_profiles.yaml file and returns hotkey bindings from
    /// all configured devices. This is used by GlobalHotkeyManager to register
    /// global hotkeys at startup.
    ///
    /// # Returns
    ///
    /// * `Vec<HotkeyBinding>` - All hotkey bindings from all devices (empty if none configured)
    ///
    /// # Behavior
    ///
    /// - Returns empty Vec if file doesn't exist (graceful degradation)
    /// - Returns empty Vec if file cannot be parsed (graceful degradation)
    /// - Aggregates bindings from all devices in the config
    pub async fn get_all_hotkey_bindings(&self) -> Vec<HotkeyBinding> {
        let mut bindings = Vec::new();

        if !self.device_profiles_path.exists() {
            return bindings;
        }

        let Ok(content) = fs::read_to_string(&self.device_profiles_path).await else {
            return bindings;
        };

        let Ok(config) = serde_yaml::from_str::<RemapDevicesConfig>(&content) else {
            return bindings;
        };

        for device_config in config.devices.values() {
            bindings.extend(device_config.hotkey_bindings.clone());
        }

        bindings
    }
}

/// Helper function to convert brightness percentage to raw value
///
/// Standalone version that can be used without ConfigManager instance.
pub fn brightness_to_raw(brightness: u8) -> u8 {
    ConfigManager::brightness_to_raw(brightness)
}

/// Helper function to convert raw brightness to percentage
///
/// Standalone version that can be used without ConfigManager instance.
pub fn raw_to_brightness(raw: u8) -> u8 {
    ConfigManager::raw_to_brightness(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        let macros_path = temp_dir.path().join("macros.yaml");
        let cache_path = temp_dir.path().join("macros.bin");

        let mut manager = ConfigManager {
            config_path,
            macros_path,
            cache_path,
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path: temp_dir.path().join("device_profiles.yaml"),
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Should be able to save and load without errors
        manager.save_config().await.unwrap();
        manager.load_config().await.unwrap();
    }

    #[tokio::test]
    async fn test_macro_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        let macros_path = temp_dir.path().join("macros.yaml");
        let cache_path = temp_dir.path().join("macros.bin");

        let manager = ConfigManager {
            config_path: config_path.clone(),
            macros_path: macros_path.clone(),
            cache_path: cache_path.clone(),
            profiles_dir: temp_dir.path().to_path_buf(),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path: temp_dir.path().join("device_profiles.yaml"),
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Add a test macro
        let test_macro = MacroEntry {
            name: "Test Macro".to_string(),
            trigger: aethermap_common::KeyCombo {
                keys: vec![30, 40], // A and D keys
                modifiers: vec![29], // Ctrl key
            },
            actions: vec![
                aethermap_common::Action::KeyPress(30),
                aethermap_common::Action::Delay(100),
                aethermap_common::Action::KeyRelease(30),
            ],
            device_id: None,
            enabled: true,
            humanize: false,
            capture_mouse: false,
        };

        manager.macros.write().await.insert("test_macro".to_string(), test_macro.clone());

        // Save and reload
        manager.save_macros().await.unwrap();

        let manager2 = ConfigManager {
            config_path: config_path.clone(),
            macros_path,
            cache_path: temp_dir.path().join("macros2.bin"),
            profiles_dir: temp_dir.path().to_path_buf(),
            remaps_path: temp_dir.path().join("remaps2.yaml"),
            device_profiles_path: temp_dir.path().join("device_profiles2.yaml"),
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        manager2.load_macros_from_yaml().await.unwrap();

        let loaded_macros = manager2.macros.read().await;
        let loaded_macro = loaded_macros.get("test_macro").unwrap();

        assert_eq!(loaded_macro.name, test_macro.name);
        assert_eq!(loaded_macro.trigger.keys, test_macro.trigger.keys);
    }

    #[tokio::test]
    async fn test_device_profile_config_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        let macros_path = temp_dir.path().join("macros.yaml");
        let cache_path = temp_dir.path().join("macros.bin");
        let device_profiles_path = temp_dir.path().join("device_profiles.yaml");

        let manager = ConfigManager {
            config_path,
            macros_path,
            cache_path,
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Test empty state
        assert_eq!(manager.list_profile_devices().await.len(), 0);
    }

    #[tokio::test]
    async fn test_device_profile_loading() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("device_profiles.yaml");

        // Create test YAML - device_id is the key in the HashMap, not a field
        let yaml_content = r#"
devices:
  "1532:0220":
    device_id: "1532:0220"
    profiles:
      gaming:
        name: "gaming"
        remaps:
          - from: capslock
            to: leftctrl
          - from: a
            to: b
      work:
        name: "work"
        remaps:
          - from: esc
            to: grave
  "1532:0221":
    device_id: "1532:0221"
    profiles:
      default:
        name: "default"
        remaps:
          - from: KEY_1
            to: KEY_2
"#;

        let mut file = std::fs::File::create(&device_profiles_path).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();

        let manager = ConfigManager {
            config_path: temp_dir.path().join("config.yaml"),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load profiles
        let result = manager.load_device_profiles().await;
        if let Err(e) = &result {
            eprintln!("Error loading device profiles: {}", e);
        }
        assert!(result.is_ok());

        // Verify devices
        let devices = manager.list_profile_devices().await;
        assert_eq!(devices.len(), 2);
        assert!(devices.contains(&"1532:0220".to_string()));
        assert!(devices.contains(&"1532:0221".to_string()));

        // Verify profiles for first device
        let profiles = manager.list_device_profiles("1532:0220").await;
        assert_eq!(profiles.len(), 2);
        assert!(profiles.contains(&"gaming".to_string()));
        assert!(profiles.contains(&"work".to_string()));

        // Verify profile retrieval
        let gaming_profile = manager.get_device_profile("1532:0220", "gaming").await;
        assert!(gaming_profile.is_some());
        let profile = gaming_profile.unwrap();
        assert_eq!(profile.name(), "gaming");
        assert_eq!(profile.remap_count().await, 2);
    }

    #[tokio::test]
    async fn test_device_profile_invalid_key_fails() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("device_profiles.yaml");

        // Create invalid YAML
        let yaml_content = r#"
devices:
  "1532:0220":
    profiles:
      bad_profile:
        remaps:
          - from: invalid_key_name
            to: leftctrl
"#;

        let mut file = std::fs::File::create(&device_profiles_path).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();

        let manager = ConfigManager {
            config_path: temp_dir.path().join("config.yaml"),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Should fail due to invalid key
        let result = manager.load_device_profiles().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extended_device_profile_loading() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("device_profiles_extended.yaml");

        // Create extended YAML with flat remaps HashMap
        let yaml_content = r#"
devices:
  "1532:0220":
    match_pattern: "1532:0220"
    profiles:
      gaming:
        name: "Gaming Profile"
        description: "Optimized for gaming"
        remaps:
          capslock: leftctrl
          a: b
      work:
        name: "Work Profile"
        remaps:
          esc: grave
default:
  KEY_1: KEY_2
"#;

        let mut file = std::fs::File::create(&device_profiles_path).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();

        let manager = ConfigManager {
            config_path: temp_dir.path().join("config.yaml"),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load profiles using extended method
        let result = manager.load_device_profiles_extended().await;
        if let Err(e) = &result {
            eprintln!("Error loading extended device profiles: {}", e);
        }
        assert!(result.is_ok());

        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 1);
        assert!(profiles.contains_key("1532:0220"));

        let device_profiles = &profiles["1532:0220"];
        assert_eq!(device_profiles.len(), 2);

        // Check profiles by finding them by name (HashMap order is not deterministic)
        let gaming_profile = device_profiles
            .iter()
            .find(|p| p.name() == "gaming")
            .expect("gaming profile not found");
        assert_eq!(gaming_profile.remap_count().await, 2);

        let work_profile = device_profiles
            .iter()
            .find(|p| p.name() == "work")
            .expect("work profile not found");
        assert_eq!(work_profile.remap_count().await, 1);
    }

    #[tokio::test]
    async fn test_extended_device_profile_invalid_key_fails() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("device_profiles_extended.yaml");

        // Create extended YAML with invalid key
        let yaml_content = r#"
devices:
  "1532:0220":
    profiles:
      bad_profile:
        remaps:
          invalid_key_name: leftctrl
"#;

        let mut file = std::fs::File::create(&device_profiles_path).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();

        let manager = ConfigManager {
            config_path: temp_dir.path().join("config.yaml"),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Should fail due to invalid key
        let result = manager.load_device_profiles_extended().await;
        assert!(result.is_err());

        match result {
            Err(RemapConfigError::InvalidKey { key, .. }) => {
                assert_eq!(key, "bad_profile");
            }
            _ => panic!("Expected InvalidKey error"),
        }
    }

    #[tokio::test]
    async fn test_extended_device_profile_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("device_profiles_empty.yaml");

        let manager = ConfigManager {
            config_path: temp_dir.path().join("config.yaml"),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load profiles - should create empty file and return empty HashMap
        let result = manager.load_device_profiles_extended().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_device_capabilities_serialization() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("device_profiles_caps.yaml");

        // Create YAML with device capabilities (Azeron keypad example)
        let yaml_content = r#"
devices:
  "32b6:12f7":
    profiles:
      gaming:
        name: "Gaming Profile"
        remaps:
          joy_btn_0: KEY_A
          joy_btn_1: KEY_B
    capabilities:
      has_analog_stick: true
      has_hat_switch: true
      joystick_button_count: 26
      led_zones:
        - "thumbstick"
        - "wrist_rest"
        - "logo"
      device_type: "Keypad"
"#;

        let mut file = std::fs::File::create(&device_profiles_path).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();

        let manager = ConfigManager {
            config_path: temp_dir.path().join("config.yaml"),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load profiles with capabilities
        let result = manager.load_device_profiles_extended().await;
        assert!(result.is_ok());

        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 1);

        // Read the file back and verify capabilities deserialize
        let content = std::fs::read_to_string(manager.device_profiles_path).unwrap();
        let config: RemapDevicesConfig = serde_yaml::from_str(&content).unwrap();

        let azeron_config = &config.devices["32b6:12f7"];
        assert!(azeron_config.capabilities.is_some());

        let caps = azeron_config.capabilities.as_ref().unwrap();
        assert_eq!(caps.has_analog_stick, Some(true));
        assert_eq!(caps.has_hat_switch, Some(true));
        assert_eq!(caps.joystick_button_count, Some(26));
        assert_eq!(caps.device_type, Some("Keypad".to_string()));
        assert!(caps.led_zones.is_some());
        let zones = caps.led_zones.as_ref().unwrap();
        assert_eq!(zones.len(), 3);
        assert!(zones.contains(&"thumbstick".to_string()));
    }

    #[tokio::test]
    async fn test_device_capabilities_optional() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("device_profiles_no_caps.yaml");

        // Create YAML without device capabilities - should still work
        let yaml_content = r#"
devices:
  "1532:0220":
    profiles:
      default:
        name: "Default"
        remaps:
          capslock: leftctrl
"#;

        let mut file = std::fs::File::create(&device_profiles_path).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();

        let manager = ConfigManager {
            config_path: temp_dir.path().join("config.yaml"),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load profiles - should work without capabilities
        let result = manager.load_device_profiles_extended().await;
        assert!(result.is_ok());

        // Verify capabilities are None
        let content = std::fs::read_to_string(manager.device_profiles_path).unwrap();
        let config: RemapDevicesConfig = serde_yaml::from_str(&content).unwrap();

        let device_config = &config.devices["1532:0220"];
        assert!(device_config.capabilities.is_none());
    }

    #[tokio::test]
    async fn test_analog_config_persistence() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("device_profiles_analog.yaml");

        // Create YAML with analog configuration
        let yaml_content = r#"
devices:
  "1532:0220":
    match_pattern: "1532:0220"
    profiles:
      gaming:
        name: "Gaming Profile"
        remaps:
          capslock: leftctrl
    analog_config:
      deadzone_percentage: 50
      sensitivity: 1.5
      response_curve: "exponential"
  "32b6:12f7":
    profiles:
      default:
        name: "Default"
        remaps: {}
    analog_config:
      deadzone_percentage: 43
      sensitivity: 1.0
      response_curve: "linear"
"#;

        let mut file = std::fs::File::create(&device_profiles_path).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();

        let manager = ConfigManager {
            config_path: temp_dir.path().join("config.yaml"),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load analog configs
        let result = manager.load_analog_configs().await;
        assert!(result.is_ok());

        let configs = result.unwrap();
        assert_eq!(configs.len(), 2);

        // Check first device config
        assert!(configs.contains_key("1532:0220"));
        let config1 = &configs["1532:0220"];
        assert_eq!(config1.deadzone_percentage, 50);
        assert_eq!(config1.sensitivity, 1.5);
        assert_eq!(config1.response_curve, "exponential");

        // Check second device config
        assert!(configs.contains_key("32b6:12f7"));
        let config2 = &configs["32b6:12f7"];
        assert_eq!(config2.deadzone_percentage, 43);
        assert_eq!(config2.sensitivity, 1.0);
        assert_eq!(config2.response_curve, "linear");
    }

    #[tokio::test]
    async fn test_analog_config_default_values() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("device_profiles_analog_defaults.yaml");

        // Create YAML with minimal analog config (should use defaults)
        let yaml_content = r#"
devices:
  "1532:0220":
    profiles:
      default:
        name: "Default"
        remaps: {}
    analog_config:
      response_curve: "exponential(2.5)"
"#;

        let mut file = std::fs::File::create(&device_profiles_path).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();

        let manager = ConfigManager {
            config_path: temp_dir.path().join("config.yaml"),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load analog configs
        let result = manager.load_analog_configs().await;
        assert!(result.is_ok());

        let configs = result.unwrap();
        assert_eq!(configs.len(), 1);

        let config = &configs["1532:0220"];
        // Should use default values for fields not specified
        assert_eq!(config.deadzone_percentage, 43);  // Default
        assert_eq!(config.sensitivity, 1.0);  // Default
        assert_eq!(config.response_curve, "exponential(2.5)");  // Specified
    }

    #[tokio::test]
    async fn test_analog_calibration_loading() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("test_analog_calibration.yaml");

        // Create test config with analog calibration per layer
        let yaml = r#"
devices:
  "32b6:12f7":
    analog_calibration:
      0:
        deadzone: 0.15
        sensitivity: linear
      1:
        deadzone: 0.10
        sensitivity: quadratic
"#;

        fs::write(&device_profiles_path, yaml).await.unwrap();

        let manager = ConfigManager {
            config_path: temp_dir.path().join("config.yaml"),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Verify base layer calibration
        let cal0 = manager.get_analog_calibration("32b6:12f7", 0).await;
        assert!(cal0.is_some());
        assert_eq!(cal0.unwrap().deadzone, 0.15);

        // Verify layer 1 calibration
        let cal1 = manager.get_analog_calibration("32b6:12f7", 1).await;
        assert!(cal1.is_some());
        assert_eq!(cal1.unwrap().deadzone, 0.10);

        // Verify non-existent layer returns None
        let cal2 = manager.get_analog_calibration("32b6:12f7", 2).await;
        assert!(cal2.is_none());
    }

    #[tokio::test]
    async fn test_get_all_analog_calibrations() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("test_all_analog_calibrations.yaml");

        // Create test config with multiple layer calibrations
        let yaml = r#"
devices:
  "32b6:12f7":
    analog_calibration:
      0:
        deadzone: 0.15
        sensitivity_multiplier: 1.0
      1:
        deadzone: 0.10
        sensitivity_multiplier: 1.5
      2:
        deadzone: 0.20
        sensitivity_multiplier: 0.8
"#;

        fs::write(&device_profiles_path, yaml).await.unwrap();

        let manager = ConfigManager {
            config_path: temp_dir.path().join("config.yaml"),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Get all calibrations
        let calibrations = manager.get_all_analog_calibrations("32b6:12f7").await;
        assert_eq!(calibrations.len(), 3);

        // Verify each layer
        assert_eq!(calibrations.get(&0).unwrap().deadzone, 0.15);
        assert_eq!(calibrations.get(&1).unwrap().deadzone, 0.10);
        assert_eq!(calibrations.get(&2).unwrap().deadzone, 0.20);

        // Non-existent device returns empty map
        let empty = manager.get_all_analog_calibrations("nonexistent").await;
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_add_hotkey_binding_persists_to_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("test_profiles.yaml");
        let config_path = temp_dir.path().join("test_config.yaml");

        // Create initial config files
        let config = ConfigManager {
            config_path: config_path.clone(),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path: device_profiles_path.clone(),
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        let binding = HotkeyBinding {
            modifiers: vec!["ctrl".to_string(), "shift".to_string()],
            key: "1".to_string(),
            profile_name: "gaming".to_string(),
            device_id: Some("32b6:12f7".to_string()),
            layer_id: Some(1),
        };

        config.add_hotkey_binding("32b6:12f7", binding).await.unwrap();

        // Verify YAML was written
        let content = fs::read_to_string(&device_profiles_path).await.unwrap();
        assert!(content.contains("hotkey_bindings"));
        assert!(content.contains("ctrl"));
        assert!(content.contains("profile_name: gaming"));

        // Verify we can read it back
        let loaded = config.get_hotkey_bindings("32b6:12f7").await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].key, "1");
    }

    #[tokio::test]
    async fn test_add_hotkey_binding_rejects_duplicate() {
        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("test_profiles.yaml");
        let config_path = temp_dir.path().join("test_config.yaml");

        let config = ConfigManager {
            config_path: config_path.clone(),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path: device_profiles_path.clone(),
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        let binding = HotkeyBinding {
            modifiers: vec!["ctrl".to_string()],
            key: "1".to_string(),
            profile_name: "gaming".to_string(),
            device_id: Some("32b6:12f7".to_string()),
            layer_id: None,
        };

        config.add_hotkey_binding("32b6:12f7", binding.clone()).await.unwrap();

        // Try to add duplicate
        let result = config.add_hotkey_binding("32b6:12f7", binding).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RemapConfigError::Validation { field, message } => {
                assert_eq!(field, "hotkey");
                assert!(message.contains("already exists"));
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[tokio::test]
    async fn test_remove_hotkey_binding() {
        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("test_profiles.yaml");
        let config_path = temp_dir.path().join("test_config.yaml");

        let config = ConfigManager {
            config_path: config_path.clone(),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path: device_profiles_path.clone(),
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Add two bindings
        let binding1 = HotkeyBinding {
            modifiers: vec!["ctrl".to_string()],
            key: "1".to_string(),
            profile_name: "gaming".to_string(),
            device_id: Some("32b6:12f7".to_string()),
            layer_id: None,
        };
        let binding2 = HotkeyBinding {
            modifiers: vec!["alt".to_string()],
            key: "2".to_string(),
            profile_name: "work".to_string(),
            device_id: Some("32b6:12f7".to_string()),
            layer_id: None,
        };

        config.add_hotkey_binding("32b6:12f7", binding1).await.unwrap();
        config.add_hotkey_binding("32b6:12f7", binding2).await.unwrap();

        // Verify both exist
        let bindings = config.get_hotkey_bindings("32b6:12f7").await.unwrap();
        assert_eq!(bindings.len(), 2);

        // Remove first
        config.remove_hotkey_binding("32b6:12f7", "1", &["ctrl".to_string()]).await.unwrap();

        // Verify only second remains
        let bindings = config.get_hotkey_bindings("32b6:12f7").await.unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].key, "2");
    }

    #[tokio::test]
    async fn test_get_hotkey_bindings_returns_empty_for_missing_device() {
        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("test_profiles.yaml");
        let config_path = temp_dir.path().join("test_config.yaml");

        let config = ConfigManager {
            config_path: config_path.clone(),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path: device_profiles_path.clone(),
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        let bindings = config.get_hotkey_bindings("nonexistent").await.unwrap();
        assert_eq!(bindings.len(), 0);
    }

    #[tokio::test]
    async fn test_set_get_auto_switch_rules() {
        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("test_profiles.yaml");
        let config_path = temp_dir.path().join("test_config.yaml");

        let config = ConfigManager {
            config_path: config_path.clone(),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path: device_profiles_path.clone(),
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        let rules = vec![
            AutoSwitchRule {
                app_id: "org.alacritty".to_string(),
                profile_name: "terminal".to_string(),
                device_id: Some("32b6:12f7".to_string()),
                layer_id: Some(0),
            },
            AutoSwitchRule {
                app_id: "*".to_string(),
                profile_name: "default".to_string(),
                device_id: None,
                layer_id: None,
            },
        ];

        config.set_auto_switch_rules(rules.clone()).await.unwrap();

        let loaded = config.get_auto_switch_rules().await;
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].app_id, "org.alacritty");
        assert_eq!(loaded[1].app_id, "*");

        // Verify YAML was written
        let content = fs::read_to_string(&config_path).await.unwrap();
        assert!(content.contains("auto_switch_rules"));
        assert!(content.contains("org.alacritty"));
    }

    #[tokio::test]
    async fn test_get_all_hotkey_bindings_aggregates_devices() {
        let temp_dir = TempDir::new().unwrap();
        let device_profiles_path = temp_dir.path().join("test_profiles.yaml");
        let config_path = temp_dir.path().join("test_config.yaml");

        let config = ConfigManager {
            config_path: config_path.clone(),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path: device_profiles_path.clone(),
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Add bindings for two devices
        let binding1 = HotkeyBinding {
            modifiers: vec!["ctrl".to_string()],
            key: "1".to_string(),
            profile_name: "gaming".to_string(),
            device_id: Some("32b6:12f7".to_string()),
            layer_id: None,
        };
        let binding2 = HotkeyBinding {
            modifiers: vec!["alt".to_string()],
            key: "2".to_string(),
            profile_name: "work".to_string(),
            device_id: Some("1532:0220".to_string()),
            layer_id: None,
        };

        config.add_hotkey_binding("32b6:12f7", binding1).await.unwrap();
        config.add_hotkey_binding("1532:0220", binding2).await.unwrap();

        let all = config.get_all_hotkey_bindings().await;
        assert_eq!(all.len(), 2);
    }
}
