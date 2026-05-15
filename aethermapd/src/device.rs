use aethermap_common::{tracing, DeviceInfo};
use evdev::{AbsoluteAxisType, Device as EvdevDevice, InputEventKind, Key, RelativeAxisType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Event type classification for passthrough decisions
#[derive(Debug, Clone, Copy)]
pub enum DeviceEventType {
    /// Key event (keyboard, mouse button, joystick button)
    Key { original_code: u16, value: i32 },
    /// Relative axis (mouse movement, scroll)
    RelAxis { axis: RelativeAxisType, value: i32 },
}

/// Message sent from device event reader to main event loop
#[derive(Debug, Clone)]
pub struct DeviceEventMessage {
    /// Device path
    pub device_path: String,
    /// Event type and data
    pub event_type: DeviceEventType,
    /// Processed key code (may be mapped from joystick buttons)
    pub key_code: Option<u16>,
}

impl DeviceEventMessage {
    /// Create a key event message
    pub fn key_event(device_path: String, original_code: u16, key_code: u16, value: i32) -> Self {
        Self {
            device_path,
            event_type: DeviceEventType::Key {
                original_code,
                value,
            },
            key_code: Some(key_code),
        }
    }

    /// Create a relative axis event message
    pub fn rel_axis_event(device_path: String, axis: RelativeAxisType, value: i32) -> Self {
        Self {
            device_path,
            event_type: DeviceEventType::RelAxis { axis, value },
            key_code: None,
        }
    }
}

use crate::gamepad_device::GamepadVirtualDevice;
use crate::remap_engine::{RemapProfile, RemapTable};

// EVIOCGRAB ioctl number for exclusive device access
const EVIOCGRAB: u64 = 0x40044590;

// Azeron Cyborg vendor ID (MCS / Azeron LTD)
// Discovered via lsusb: ID 16d0:12f7
const AZERON_VENDOR_ID: u16 = 0x16d0;

// Joystick button event codes (for internal mapping)
// Maps joystick buttons to JOY_BTN_0 through JOY_BTN_25 codes (50000-50025)
const JOY_BTN_BASE: u16 = 50000;

/// Convert evdev joystick button key to JOY_BTN_N code
///
/// Maps BTN_JOYSTICK, BTN_THUMB, etc. to sequential JOY_BTN_0-25 codes.
/// This provides a consistent remappable interface for joystick buttons.
///
/// # Arguments
///
/// * `key` - The evdev Key code
///
/// # Returns
///
/// * `Some(u16)` - JOY_BTN_N code (50000-50025) if key is a joystick button
/// * `None` - Key is not a joystick button
fn map_joystick_button(key: Key) -> Option<u16> {
    // Joystick button range: BTN_JOYSTICK (0x120) to BTN_DEAD (0x12f)
    // Map to JOY_BTN_0 (50000) through JOY_BTN_25 (50025)
    const BTN_JOYSTICK_START: u16 = 0x120;
    const BTN_JOYSTICK_END: u16 = 0x12f;

    let code = key.0;
    if (BTN_JOYSTICK_START..=BTN_JOYSTICK_END).contains(&code) {
        Some(JOY_BTN_BASE + (code - BTN_JOYSTICK_START))
    } else {
        None
    }
}

/// Hat switch direction key codes (default to arrow keys and diagonals)
const HAT_UP: u16 = 103; // KEY_UP
const HAT_DOWN: u16 = 108; // KEY_DOWN
const HAT_LEFT: u16 = 105; // KEY_LEFT
const HAT_RIGHT: u16 = 106; // KEY_RIGHT
                            // Diagonal directions map to two key presses (composable)
#[allow(dead_code)]
const HAT_UP_LEFT: u16 = 111; // KEY_UPLEFT (or map to custom)
#[allow(dead_code)]
const HAT_UP_RIGHT: u16 = 114; // KEY_UPRIGHT (or map to custom)
#[allow(dead_code)]
const HAT_DOWN_LEFT: u16 = 116; // KEY_DOWNLEFT (or map to custom)
#[allow(dead_code)]
const HAT_DOWN_RIGHT: u16 = 113; // KEY_DOWNRIGHT (or map to custom)

/// Hat switch state for tracking 8-way position
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct HatSwitchState {
    /// X axis: -1 (left), 0 (center), 1 (right)
    x: i32,
    /// Y axis: -1 (up), 0 (center), 1 (down)
    y: i32,
}

/// D-pad state for tracking analog stick position when in D-pad mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct DpadState {
    /// X position: -32768 to 32767 (normalized later)
    x: i32,
    /// Y position: -32768 to 32767 (normalized later)
    y: i32,
}

/// Convert hat switch position to keyboard key codes
///
/// Hat switches use ABS_HAT0X and ABS_HAT0Y axes with values -1, 0, 1.
/// This function maps the 8-way position to keyboard key codes.
/// Diagonal directions return two key codes (composable from cardinal keys).
///
/// # Arguments
///
/// * `state` - Current hat switch position
///
/// # Returns
///
/// Vec of key codes to simulate (0 for centered, 1 for cardinal, 2 for diagonal)
fn map_hat_switch_to_keys(state: HatSwitchState) -> Vec<u16> {
    match (state.x, state.y) {
        (0, 0) => vec![],                    // Centered - no keys
        (0, -1) => vec![HAT_UP],             // Up
        (1, -1) => vec![HAT_UP, HAT_RIGHT],  // Up-Right
        (1, 0) => vec![HAT_RIGHT],           // Right
        (1, 1) => vec![HAT_DOWN, HAT_RIGHT], // Down-Right
        (0, 1) => vec![HAT_DOWN],            // Down
        (-1, 1) => vec![HAT_DOWN, HAT_LEFT], // Down-Left
        (-1, 0) => vec![HAT_LEFT],           // Left
        (-1, -1) => vec![HAT_UP, HAT_LEFT],  // Up-Left
        _ => vec![],
    }
}

/// Information about a grabbed device
pub struct GrabbedDevice {
    pub info: DeviceInfo,
    pub evdev: EvdevDevice,
    pub fd: RawFd,
    pub grabbed: bool,
    pub remap_engine: Option<Arc<crate::remap_engine::RemapEngine>>,
    pub injector: Option<Arc<RwLock<dyn crate::injector::Injector + Send + Sync>>>,
    /// Active profile name for this device
    pub active_profile: Option<String>,
    /// Current active remappings for this device
    pub active_remappings: Option<Arc<RwLock<HashMap<Key, Key>>>>,
    /// Pre-compiled remap tables for each profile (profile_name -> remap_table)
    /// This enables O(1) profile switching without recompiling remaps
    pub profile_remaps: HashMap<String, Arc<RwLock<RemapTable>>>,
}

/// Ensure device is released on drop (panic or shutdown)
impl Drop for GrabbedDevice {
    fn drop(&mut self) {
        if self.grabbed {
            info!("Releasing device grab for {} via Drop", self.info.name);
            // Ensure ungrab happens even on panic
            let _ = unsafe { libc::ioctl(self.fd, EVIOCGRAB, 0 as libc::c_int) };
        }
    }
}

/// Information about available profiles for a device
///
/// This type is used in IPC responses to provide profile metadata
/// to clients for device profile listing and selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfileInfo {
    /// Device path (e.g., "/dev/input/event0")
    pub device_path: String,
    /// Available profile names for this device
    pub profiles: Vec<String>,
}

/// Manages the discovery and monitoring of input devices
pub struct DeviceManager {
    devices: HashMap<String, DeviceInfo>,
    grabbed_devices: HashMap<String, GrabbedDevice>,
    event_sender: mpsc::Sender<DeviceEventMessage>,
    event_receiver: Option<mpsc::Receiver<DeviceEventMessage>>,
    remap_engine: Option<Arc<crate::remap_engine::RemapEngine>>,
    injector: Option<Arc<RwLock<dyn crate::injector::Injector + Send + Sync>>>,
    config_manager: Option<Arc<crate::config::ConfigManager>>,
    /// Per-device profile lists loaded from config
    device_profiles: HashMap<String, Vec<RemapProfile>>,
    /// Analog processor for D-pad emulation and analog processing
    analog_processor: Option<Arc<crate::analog_processor::AnalogProcessor>>,
    /// Global hotkey manager for profile switching
    hotkey_manager:
        Option<Arc<tokio::sync::Mutex<crate::global_hotkey_manager::GlobalHotkeyManager>>>,
    /// Virtual gamepad device for analog stick output
    gamepad_device: Arc<GamepadVirtualDevice>,
    /// Layer manager for accessing active layer configuration
    layer_manager: Option<Arc<tokio::sync::RwLock<crate::layer_manager::LayerManager>>>,
    /// Macro engine for recording mouse events
    macro_engine: Option<Arc<crate::macro_engine::MacroEngine>>,
    /// Daemon state for broadcasting analog input updates
    daemon_state: Option<Arc<tokio::sync::RwLock<crate::DaemonState>>>,
}

impl DeviceManager {
    /// Create a new device manager
    /// Create a new device manager
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::channel(1000);
        let gamepad_device = Arc::new(GamepadVirtualDevice::new());
        Self {
            devices: HashMap::new(),
            grabbed_devices: HashMap::new(),
            event_sender,
            event_receiver: Some(event_receiver),
            remap_engine: None,
            injector: None,
            config_manager: None,
            device_profiles: HashMap::new(),
            analog_processor: None,
            hotkey_manager: None,
            gamepad_device,
            layer_manager: None,
            macro_engine: None,
            daemon_state: None,
        }
    }

    /// Set the remap engine for device event processing
    pub fn set_remap_engine(
        &mut self,
        remap_engine: Option<Arc<crate::remap_engine::RemapEngine>>,
    ) {
        self.remap_engine = remap_engine;
    }

    /// Set the macro engine for recording
    pub fn set_macro_engine(
        &mut self,
        macro_engine: Option<Arc<crate::macro_engine::MacroEngine>>,
    ) {
        self.macro_engine = macro_engine;
    }

    /// Set the injector for remapped key injection
    pub fn set_injector(
        &mut self,
        injector: Option<Arc<RwLock<dyn crate::injector::Injector + Send + Sync>>>,
    ) {
        self.injector = injector.clone();
    }

    /// Get a clone of the injector reference
    ///
    /// Returns a clone of the injector Arc if set, for use in event passthrough.
    pub fn get_injector(&self) -> Option<Arc<RwLock<dyn crate::injector::Injector + Send + Sync>>> {
        self.injector.clone()
    }

    /// Set the analog processor for D-pad emulation
    pub fn set_analog_processor(
        &mut self,
        analog_processor: Option<Arc<crate::analog_processor::AnalogProcessor>>,
    ) {
        self.analog_processor = analog_processor;
    }

    /// Set the global hotkey manager for profile switching
    pub fn set_hotkey_manager(
        &mut self,
        hotkey_manager: Option<
            Arc<tokio::sync::Mutex<crate::global_hotkey_manager::GlobalHotkeyManager>>,
        >,
    ) {
        self.hotkey_manager = hotkey_manager;
    }

    /// Get the global hotkey manager reference
    ///
    /// Returns a reference to the hotkey manager for profile switching.
    /// This is used by IPC handlers to reload bindings after configuration changes.
    pub fn hotkey_manager(
        &self,
    ) -> Option<&Arc<tokio::sync::Mutex<crate::global_hotkey_manager::GlobalHotkeyManager>>> {
        self.hotkey_manager.as_ref()
    }

    /// Set the layer manager for accessing active layer configuration
    pub fn set_layer_manager(
        &mut self,
        layer_manager: Option<Arc<tokio::sync::RwLock<crate::layer_manager::LayerManager>>>,
    ) {
        self.layer_manager = layer_manager;
    }

    /// Set the daemon state for broadcasting analog input updates
    pub fn set_daemon_state(
        &mut self,
        daemon_state: Option<Arc<tokio::sync::RwLock<crate::DaemonState>>>,
    ) {
        self.daemon_state = daemon_state;
    }

    /// Get the virtual gamepad device reference
    ///
    /// Returns a reference to the gamepad device for analog stick output.
    /// This can be used to emit axis events directly if needed.
    pub fn gamepad_device(&self) -> &Arc<GamepadVirtualDevice> {
        &self.gamepad_device
    }

    /// Set the config manager for device profile access
    pub fn set_config_manager(
        &mut self,
        config_manager: Option<Arc<crate::config::ConfigManager>>,
    ) {
        self.config_manager = config_manager;
    }

    /// Store device profiles loaded from config
    ///
    /// This method stores the per-device profile lists for runtime profile lookup.
    /// Profiles are typically loaded from the extended YAML configuration format.
    ///
    /// # Arguments
    ///
    /// * `profiles` - HashMap mapping device_id to Vec of RemapProfile
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let profiles = config_manager.load_device_profiles_extended().await?;
    /// device_manager.set_device_profiles(profiles);
    /// ```
    pub fn set_device_profiles(&mut self, profiles: HashMap<String, Vec<RemapProfile>>) {
        self.device_profiles = profiles;
        info!("Stored profiles for {} devices", self.device_profiles.len());
    }

    /// Get available profiles for a device
    ///
    /// Returns the list of profiles configured for a specific device.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    ///
    /// # Returns
    ///
    /// * `Some(&[RemapProfile])` - Slice of profiles for the device
    /// * `None` - No profiles configured for this device
    pub fn get_device_profiles(&self, device_id: &str) -> Option<&[RemapProfile]> {
        self.device_profiles.get(device_id).map(|v| v.as_slice())
    }

    /// Format a device identifier from vendor and product IDs
    ///
    /// Device IDs use the format "vendor:product" with hexadecimal values.
    /// This matches the YAML configuration format.
    ///
    /// # Arguments
    ///
    /// * `vendor_id` - Vendor ID (e.g., 0x1532 for Razer)
    /// * `product_id` - Product ID (e.g., 0x0220)
    ///
    /// # Returns
    ///
    /// Device identifier string (e.g., "1532:0220")
    pub fn format_device_id(vendor_id: u16, product_id: u16) -> String {
        format!("{:04x}:{:04x}", vendor_id, product_id)
    }

    /// Parse a device path to find its device_id
    ///
    /// This method looks up the device info and formats its ID.
    ///
    /// # Arguments
    ///
    /// * `device_path` - Path to the device (e.g., "/dev/input/event0")
    ///
    /// # Returns
    ///
    /// * `Some(String)` - Device identifier if device found
    /// * `None` - Device not in discovered devices
    pub fn get_device_id(&self, device_path: &str) -> Option<String> {
        self.devices
            .get(device_path)
            .map(|info| Self::format_device_id(info.vendor_id, info.product_id))
    }

    /// Start device discovery and monitoring
    pub async fn start_discovery(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting device discovery");

        // Get list of devices
        let discovered_devices = self.scan_devices().await?;

        // Add devices to our collection
        for device in discovered_devices {
            info!("Found device: {} at {}", device.name, device.path.display());
            self.devices
                .insert(device.path.to_string_lossy().to_string(), device);
        }

        info!("Discovered {} input devices", self.devices.len());
        Ok(())
    }

    /// Get all discovered devices
    pub fn get_devices(&self) -> Vec<DeviceInfo> {
        self.devices.values().cloned().collect()
    }

    /// Get a specific device by path
    pub fn get_device(&self, path: &str) -> Option<DeviceInfo> {
        self.devices.get(path).cloned()
    }

    /// Get event receiver for new device events
    ///
    /// # Panics
    /// If the receiver has already been taken (programming error - should only be called once)
    pub fn get_event_receiver(&mut self) -> mpsc::Receiver<DeviceEventMessage> {
        self.event_receiver.take()
            .expect("Event receiver can only be taken once. This is a programming error - get_event_receiver() should only be called once during daemon initialization.")
    }

    /// Grab a device exclusively (EVIOCGRAB) for input interception
    pub async fn grab_device(
        &mut self,
        device_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.grabbed_devices.contains_key(device_path) {
            info!("Device {} already grabbed", device_path);
            return Ok(());
        }

        let device_info = self
            .devices
            .get(device_path)
            .ok_or_else(|| format!("Device not found: {}", device_path))?
            .clone();

        info!("Grabbing device: {} ({})", device_info.name, device_path);

        // Open the evdev device
        let evdev = EvdevDevice::open(device_path)
            .map_err(|e| format!("Failed to open device {}: {}", device_path, e))?;

        let fd = evdev.as_raw_fd();

        // Grab the device exclusively with EVIOCGRAB
        let result = unsafe { libc::ioctl(fd, EVIOCGRAB, 1 as libc::c_int) };

        if result < 0 {
            let err = std::io::Error::last_os_error();
            error!("Failed to grab device {}: {}", device_path, err);
            return Err(format!("EVIOCGRAB failed: {}", err).into());
        }

        info!("Successfully grabbed device {} (fd={})", device_path, fd);
        info!("Note: Release all held keys after grab to prevent stuck modifiers");

        // Store the grabbed device with remap engine and injector
        let remap_engine = self.remap_engine.clone();
        let injector = self.injector.clone();

        // Get device_id for analog processing (before device_info is moved)
        let device_id = Self::format_device_id(device_info.vendor_id, device_info.product_id);

        self.grabbed_devices.insert(
            device_path.to_string(),
            GrabbedDevice {
                info: device_info,
                evdev,
                fd,
                grabbed: true,
                remap_engine: remap_engine.clone(),
                injector: injector.clone(),
                active_profile: None,
                active_remappings: None,
                profile_remaps: HashMap::new(),
            },
        );

        // Start event reading loop for this device
        let analog_processor = self.analog_processor.clone();
        let hotkey_manager = self.hotkey_manager.clone();
        let layer_manager = self.layer_manager.clone();
        let macro_engine = self.macro_engine.clone();
        let gamepad_device = Arc::clone(&self.gamepad_device);
        let daemon_state = self.daemon_state.clone();
        self.start_event_reader(
            device_path.to_string(),
            device_id.clone(),
            remap_engine,
            injector,
            analog_processor,
            hotkey_manager,
            layer_manager,
            macro_engine,
            gamepad_device,
            daemon_state,
        )
        .await?;

        // Load analog configuration from profile if available
        if let Some(config_manager) = &self.config_manager {
            // Load analog configs for all devices
            if let Ok(analog_configs) = config_manager.load_analog_configs().await {
                if let Some(analog_config) = analog_configs.get(&device_id) {
                    if let Some(processor) = &self.analog_processor {
                        if let Err(e) = processor.load_config(&device_id, analog_config).await {
                            warn!(
                                "Failed to load analog config for device {}: {}",
                                device_id, e
                            );
                        } else {
                            info!("Loaded analog config for device {} from profile", device_id);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Ungrab a device (release exclusive access)
    pub async fn ungrab_device(
        &mut self,
        device_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(grabbed) = self.grabbed_devices.remove(device_path) {
            info!("Ungrabbing device: {}", device_path);

            // Release the grab
            let result = unsafe { libc::ioctl(grabbed.fd, EVIOCGRAB, 0 as libc::c_int) };

            if result < 0 {
                warn!(
                    "Failed to ungrab device {}: {}",
                    device_path,
                    std::io::Error::last_os_error()
                );
            } else {
                info!("Successfully ungrabbed device {}", device_path);
            }
        }

        Ok(())
    }

    /// Handle device removal (hotplug unplug event)
    ///
    /// This method is called when a device is unplugged. It performs graceful cleanup:
    /// - Ungrabs the device (releases EVIOCGRAB)
    /// - Removes from grabbed_devices HashMap
    /// - Removes from devices HashMap
    ///
    /// This is a best-effort cleanup: errors are logged but don't cause failure,
    /// since the device is already gone from the system.
    ///
    /// # Arguments
    ///
    /// * `device_path` - Path to the removed device (e.g., "/dev/input/event0")
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Cleanup completed (device may not have been tracked)
    pub async fn handle_device_removal(
        &mut self,
        device_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Handling device removal: {}", device_path);

        // Ungrab the device if it was grabbed
        if let Err(e) = self.ungrab_device(device_path).await {
            warn!(
                "Error ungrabbing device during removal {}: {}",
                device_path, e
            );
        }

        // Remove from devices HashMap if present
        if self.devices.remove(device_path).is_some() {
            info!("Device {} removed from device list", device_path);
        }

        info!("Device {} removed and cleaned up", device_path);
        Ok(())
    }

    /// Handle device addition (hotplug plug-in event)
    ///
    /// This method is called when a device is plugged in. It:
    /// - Re-discovers the device information (name, vendor_id, product_id)
    /// - Adds the device to the devices HashMap
    /// - Checks if profiles exist for this device_id
    /// - Auto-grabs the device if profiles are configured
    ///
    /// # Arguments
    ///
    /// * `device_path` - Path to the added device (e.g., "/dev/input/event0")
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Device added successfully
    /// * `Err(Box<dyn Error>)` - Failed to get device info
    pub async fn handle_device_add(
        &mut self,
        device_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if device already exists
        if self.devices.contains_key(device_path) {
            debug!("Device {} already tracked, skipping", device_path);
            return Ok(());
        }

        info!("Handling device addition: {}", device_path);

        // Open device to get info
        let device = EvdevDevice::open(device_path)
            .map_err(|e| format!("Failed to open device {}: {}", device_path, e))?;

        let name = device.name().unwrap_or("Unknown Device").to_string();
        let input_id = device.input_id();
        let vendor_id = input_id.vendor();
        let product_id = input_id.product();
        let phys = device.physical_path().unwrap_or("unknown").to_string();

        // Detect device type from capabilities
        let device_type = Self::detect_device_type(&device);

        // Format device_id
        let device_id = Self::format_device_id(vendor_id, product_id);

        info!(
            "Re-discovered device {} ({}) at {}",
            name, device_id, device_path
        );

        // Create DeviceInfo and insert into devices HashMap
        let device_info = DeviceInfo {
            name: name.clone(),
            path: std::path::PathBuf::from(device_path),
            vendor_id,
            product_id,
            phys,
            device_type,
        };

        self.devices.insert(device_path.to_string(), device_info);

        // Check if we have profiles for this device
        let should_grab = if let Some(config_manager) = &self.config_manager {
            let profiles = config_manager.list_device_profiles(&device_id).await;
            if !profiles.is_empty() {
                info!(
                    "Device {} has {} profiles configured, auto-grabbing",
                    device_id,
                    profiles.len()
                );
                true
            } else {
                debug!("No profiles for {}, skipping auto-grab", device_id);
                false
            }
        } else {
            debug!("No config manager set, skipping auto-grab");
            false
        };

        // Grab the device if profiles exist
        if should_grab {
            if let Err(e) = self.grab_device(device_path).await {
                warn!("Failed to auto-grab device {}: {}", device_path, e);
            } else {
                info!("Device {} ({}) added and grabbed", device_id, device_path);
            }
        }

        Ok(())
    }

    /// Start reading events from a grabbed device
    #[allow(clippy::too_many_arguments)]
    async fn start_event_reader(
        &self,
        device_path: String,
        device_id: String,
        remap_engine: Option<Arc<crate::remap_engine::RemapEngine>>,
        injector: Option<Arc<RwLock<dyn crate::injector::Injector + Send + Sync>>>,
        analog_processor: Option<Arc<crate::analog_processor::AnalogProcessor>>,
        hotkey_manager: Option<
            Arc<tokio::sync::Mutex<crate::global_hotkey_manager::GlobalHotkeyManager>>,
        >,
        layer_manager: Option<Arc<tokio::sync::RwLock<crate::layer_manager::LayerManager>>>,
        macro_engine: Option<Arc<crate::macro_engine::MacroEngine>>,
        gamepad_device: Arc<GamepadVirtualDevice>,
        daemon_state: Option<Arc<tokio::sync::RwLock<crate::DaemonState>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sender = self.event_sender.clone();

        // Clone the path and device_id for the async task
        let path = device_path.clone();
        let id = device_id.clone();

        // Spawn a blocking task since evdev uses synchronous I/O
        tokio::task::spawn_blocking(move || {
            info!("Starting event reader for {}", path);

            // Re-open the device in the blocking context
            let mut device = match EvdevDevice::open(&path) {
                Ok(d) => {
                    info!("Device reopened for event reading: {}", path);
                    d
                }
                Err(e) => {
                    error!("Failed to open device {} for event reading: {}", path, e);
                    return;
                }
            };

            // Log device capabilities for debugging
            if let Some(keys) = device.supported_keys() {
                let key_count = keys.iter().count();
                debug!("Device {} supports {} key codes", path, key_count);
            }
            if let Some(rel_axes) = device.supported_relative_axes() {
                let has_wheel = rel_axes
                    .iter()
                    .any(|a| matches!(a, RelativeAxisType::REL_WHEEL));
                let has_hwheel = rel_axes
                    .iter()
                    .any(|a| matches!(a, RelativeAxisType::REL_HWHEEL));
                let has_x = rel_axes
                    .iter()
                    .any(|a| matches!(a, RelativeAxisType::REL_X));
                let has_y = rel_axes
                    .iter()
                    .any(|a| matches!(a, RelativeAxisType::REL_Y));
                debug!(
                    "Device {} relative axes: X={}, Y={}, WHEEL={}, HWHEEL={}",
                    path, has_x, has_y, has_wheel, has_hwheel
                );
            }

            // Create a runtime handle for async operations
            let rt = tokio::runtime::Handle::current();

            // Track hat switch state for D-pad processing
            let mut hat_state = HatSwitchState::default();
            let mut previous_hat_keys = Vec::<u16>::new();

            // Track analog stick position for D-pad emulation
            let mut dpad_state = DpadState::default();
            let mut previous_dpad_keys = Vec::<u16>::new();

            // Track previous WASD key state for proper press/release events
            let wasd_previous_keys =
                std::sync::Arc::new(tokio::sync::RwLock::new(Vec::<(evdev::Key, bool)>::new()));

            loop {
                // Fetch events synchronously (this blocks)
                match device.fetch_events() {
                    Ok(events) => {
                        for event in events {
                            match event.kind() {
                                // Process key events (including mouse buttons)
                                InputEventKind::Key(key) => {
                                    let key_code = evdev::Key(key.0);
                                    let value = event.value(); // 0=release, 1=press, 2=repeat

                                    // Check if this is a joystick button (BTN_JOYSTICK range 0x120-0x12f)
                                    let mapped_code = map_joystick_button(key_code);
                                    let final_key_code = mapped_code.unwrap_or(key_code.0);
                                    let is_joystick = mapped_code.is_some();

                                    // Check if this is a mouse button (codes 272+)
                                    let is_mouse_button = key_code.0 >= 272 && key_code.0 <= 288;

                                    debug!("Event from {}: key={:?} ({}), joy={}, is_mouse={}, value={}",
                                           path, key_code, key_code.0, is_joystick, is_mouse_button, value);

                                    // Check for global hotkeys FIRST (before remap processing)
                                    // Hotkey detection happens at daemon level for Wayland compatibility
                                    if let Some(hotkey_mgr) = &hotkey_manager {
                                        let pressed = value == 1; // Only check on press, not release or repeat
                                        let consumed = rt.block_on(async {
                                            let mut manager = hotkey_mgr.lock().await;
                                            manager.check_key_event(key_code.0, pressed).await
                                        });

                                        if consumed {
                                            // Hotkey matched and action was executed
                                            // Consume the event (don't forward to remap engine or macro engine)
                                            debug!(
                                                "Hotkey consumed event: key={:?}, value={}",
                                                key_code, value
                                            );
                                            continue;
                                        }
                                    }

                                    // Check for remap before sending to macro engine
                                    // Use final_key_code which may be a JOY_BTN_N code
                                    if let Some(remap_engine) = &remap_engine {
                                        let final_key = evdev::Key(final_key_code);
                                        if let Some((output_key, out_value)) = rt
                                            .block_on(remap_engine.process_event(final_key, value))
                                        {
                                            // Key is remapped - inject and skip macro engine
                                            if let Some(injector) = &injector {
                                                let injector_ref = rt.block_on(injector.read());
                                                // Convert evdev::Key back to u16 for injector
                                                match out_value {
                                                    0 => {
                                                        let _ = rt.block_on(
                                                            injector_ref.key_release(output_key.0),
                                                        );
                                                    }
                                                    1 => {
                                                        let _ = rt.block_on(
                                                            injector_ref.key_press(output_key.0),
                                                        );
                                                    }
                                                    2 => {
                                                        let _ = rt.block_on(
                                                            injector_ref.key_press(output_key.0),
                                                        );
                                                    } // Repeat sends press
                                                    _ => {}
                                                }
                                            }
                                            debug!(
                                                "Remapped {:?}({}) -> {:?}({})",
                                                final_key, value, output_key, out_value
                                            );
                                            continue; // Skip sending to macro engine
                                        }
                                    }

                                    // Send event to macro engine for non-remapped keys
                                    // Include original key code for potential passthrough
                                    // Use final_key_code which includes JOY_BTN_N codes for joystick buttons
                                    let sender_clone = sender.clone();
                                    let path_clone = path.clone();
                                    let msg = DeviceEventMessage::key_event(
                                        path_clone,
                                        key_code.0,     // original code for passthrough
                                        final_key_code, // processed code for macro engine
                                        value,
                                    );
                                    if let Err(e) = rt.block_on(sender_clone.send(msg)) {
                                        error!("Failed to send event: {}", e);
                                        return;
                                    }
                                }

                                // Process relative axis events (mouse movement, scroll)
                                InputEventKind::RelAxis(axis) => {
                                    let value = event.value(); // movement delta or scroll amount

                                    // Filter out high-resolution scroll events to avoid double-speed
                                    if matches!(axis, RelativeAxisType::REL_WHEEL_HI_RES)
                                        || matches!(axis, RelativeAxisType::REL_HWHEEL_HI_RES)
                                    {
                                        debug!("Filtering high-res scroll event: {:?}", axis);
                                        continue;
                                    }

                                    debug!("Event from {}: axis={:?}, value={}", path, axis, value);

                                    // Forward to macro engine for recording
                                    if let Some(macro_engine) = &macro_engine {
                                        let axis_code = match axis {
                                            RelativeAxisType::REL_X => Some(0),
                                            RelativeAxisType::REL_Y => Some(1),
                                            _ => None,
                                        };
                                        if let Some(code) = axis_code {
                                            let _ = rt.block_on(
                                                macro_engine
                                                    .process_relative_event(code, value, &path),
                                            );
                                        }
                                    }

                                    // Send to macro engine with full event data for potential passthrough
                                    let sender_clone = sender.clone();
                                    let path_clone = path.clone();
                                    let msg =
                                        DeviceEventMessage::rel_axis_event(path_clone, axis, value);
                                    if let Err(e) = rt.block_on(sender_clone.send(msg)) {
                                        error!("Failed to send mouse event: {}", e);
                                        return;
                                    }
                                }

                                // Process absolute axis events (analog stick, throttle, hat switch)
                                InputEventKind::AbsAxis(axis) => {
                                    let value = event.value();

                                    debug!("Event from {}: axis={:?}, value={}", path, axis, value);

                                    // Check if this is a hat switch axis (D-pad)
                                    let is_hat_switch = matches!(
                                        axis,
                                        AbsoluteAxisType::ABS_HAT0X | AbsoluteAxisType::ABS_HAT0Y
                                    );

                                    if is_hat_switch {
                                        // Update hat switch state
                                        match axis {
                                            AbsoluteAxisType::ABS_HAT0X => {
                                                hat_state.x = value;
                                            }
                                            AbsoluteAxisType::ABS_HAT0Y => {
                                                hat_state.y = value;
                                            }
                                            _ => {}
                                        }

                                        // Map hat position to key codes
                                        let current_hat_keys = map_hat_switch_to_keys(hat_state);

                                        // Send key release events for previous direction
                                        for key_code in &previous_hat_keys {
                                            let sender_clone = sender.clone();
                                            let path_clone = path.clone();
                                            let msg = DeviceEventMessage::key_event(
                                                path_clone, *key_code, *key_code, 0,
                                            );
                                            let _ = rt.block_on(sender_clone.send(msg));
                                        }

                                        // Send key press events for new direction
                                        for key_code in &current_hat_keys {
                                            let sender_clone = sender.clone();
                                            let path_clone = path.clone();
                                            let msg = DeviceEventMessage::key_event(
                                                path_clone, *key_code, *key_code, 1,
                                            );
                                            let _ = rt.block_on(sender_clone.send(msg));
                                        }

                                        previous_hat_keys = current_hat_keys.clone();

                                        debug!(
                                            "Hat switch state: ({}, {}) -> {} keys",
                                            hat_state.x,
                                            hat_state.y,
                                            current_hat_keys.len()
                                        );
                                        continue;
                                    }

                                    // Check if this is an analog stick axis (ABS_X or ABS_Y)
                                    let is_analog_stick = matches!(
                                        axis,
                                        AbsoluteAxisType::ABS_X | AbsoluteAxisType::ABS_Y
                                    );

                                    // Broadcast raw analog input for GUI visualization
                                    // This happens before any processing so the GUI shows actual stick position
                                    if is_analog_stick {
                                        if let Some(state) = &daemon_state {
                                            // Normalize to -1.0 to 1.0 range for GUI
                                            let _normalized = value as f32 / 32767.0;
                                            let device_id_clone = id.clone();

                                            match axis {
                                                AbsoluteAxisType::ABS_X => {
                                                    // We need both axes, so just update the stored value
                                                    // The actual broadcast happens when we have both values
                                                    dpad_state.x = value;
                                                }
                                                AbsoluteAxisType::ABS_Y => {
                                                    dpad_state.y = value;
                                                    // Broadcast when we have Y (second axis)
                                                    let x_norm = dpad_state.x as f32 / 32767.0;
                                                    let y_norm = dpad_state.y as f32 / 32767.0;

                                                    rt.block_on(async move {
                                                        crate::ipc::broadcast_analog_input(
                                                            state,
                                                            &device_id_clone,
                                                            x_norm,
                                                            y_norm,
                                                        )
                                                        .await;
                                                    });
                                                }
                                                _ => {}
                                            }
                                        }
                                    }

                                    // Check if D-pad mode is enabled
                                    let dpad_mode = if is_analog_stick {
                                        analog_processor.as_ref().map(|processor| {
                                            rt.block_on(processor.get_dpad_mode(&id))
                                        })
                                    } else {
                                        None
                                    };

                                    // If D-pad mode is enabled, handle D-pad emulation
                                    if let Some(mode) = dpad_mode {
                                        if mode != crate::analog_processor::DpadMode::Disabled {
                                            // Update D-pad state
                                            match axis {
                                                AbsoluteAxisType::ABS_X => {
                                                    dpad_state.x = value;
                                                }
                                                AbsoluteAxisType::ABS_Y => {
                                                    dpad_state.y = value;
                                                }
                                                _ => {}
                                            }

                                            // Only process on ABS_Y events (when both axes are updated)
                                            // or when we have both X and Y set
                                            if matches!(axis, AbsoluteAxisType::ABS_Y)
                                                || (matches!(axis, AbsoluteAxisType::ABS_X)
                                                    && dpad_state.y != 0)
                                            {
                                                // Normalize to -1.0 to 1.0 range
                                                let x_norm = dpad_state.x as f32 / 32767.0;
                                                let y_norm = dpad_state.y as f32 / 32767.0;

                                                // Map to D-pad directions
                                                let directions = crate::analog_processor::AnalogProcessor::map_analog_to_dpad(
                                                    x_norm, y_norm, mode
                                                );

                                                // Convert directions to key codes
                                                let current_dpad_keys: Vec<u16> = directions.iter()
                                                    .map(|d| crate::analog_processor::direction_to_key_code(*d))
                                                    .collect();

                                                // Send key release events for previous direction
                                                if let Some(inj) = &injector {
                                                    for key_code in &previous_dpad_keys {
                                                        if !current_dpad_keys.contains(key_code) {
                                                            let inj_clone = Arc::clone(inj);
                                                            let key = *key_code;
                                                            rt.block_on(async move {
                                                                let lock = inj_clone.write().await;
                                                                let _ = lock.key_release(key).await;
                                                            });
                                                            debug!(
                                                                "D-pad release: key={}",
                                                                key_code
                                                            );
                                                        }
                                                    }

                                                    // Send key press events for new direction
                                                    for key_code in &current_dpad_keys {
                                                        if !previous_dpad_keys.contains(key_code) {
                                                            let inj_clone = Arc::clone(inj);
                                                            let key = *key_code;
                                                            rt.block_on(async move {
                                                                let lock = inj_clone.write().await;
                                                                let _ = lock.key_press(key).await;
                                                            });
                                                            debug!("D-pad press: key={}", key_code);
                                                        }
                                                    }
                                                }

                                                previous_dpad_keys = current_dpad_keys.clone();
                                                debug!(
                                                    "D-pad emulation: ({}, {}) -> {} keys",
                                                    dpad_state.x,
                                                    dpad_state.y,
                                                    current_dpad_keys.len()
                                                );
                                            }

                                            // Skip sending to macro engine when in D-pad mode
                                            continue;
                                        }
                                    }

                                    // Check if active layer uses gamepad mode
                                    // This requires layer_manager to be available
                                    if let Some(lm) = &layer_manager {
                                        let effective_layer = rt.block_on(async {
                                            let lm_read = lm.read().await;
                                            lm_read
                                                .get_device_state(&id)
                                                .await
                                                .map(|s| s.get_effective_layer())
                                                .unwrap_or(0)
                                        });

                                        // Get layer config to check analog_mode
                                        let analog_mode = rt.block_on(async {
                                            let lm_read = lm.read().await;
                                            if let Some(state) = lm_read.get_device_state(&id).await
                                            {
                                                if let Some(config) =
                                                    state.get_layer_config(effective_layer)
                                                {
                                                    // Copy the AnalogMode value (Copy trait)
                                                    config.analog_mode
                                                } else {
                                                    crate::analog_processor::AnalogMode::Disabled
                                                }
                                            } else {
                                                crate::analog_processor::AnalogMode::Disabled
                                            }
                                        });

                                        if analog_mode
                                            == crate::analog_processor::AnalogMode::Gamepad
                                        {
                                            // Track both axes for gamepad processing
                                            match axis {
                                                AbsoluteAxisType::ABS_X => {
                                                    dpad_state.x = value; // Reuse dpad_state for tracking
                                                }
                                                AbsoluteAxisType::ABS_Y => {
                                                    dpad_state.y = value; // Reuse dpad_state for tracking

                                                    // Process gamepad output when we have both axes
                                                    let gamepad_clone = Arc::clone(&gamepad_device);
                                                    let lm_clone = Arc::clone(lm);
                                                    let id_clone = id.clone();
                                                    let analog_processor_clone =
                                                        analog_processor.clone();

                                                    rt.block_on(async move {
                                                        // Process through DeviceManager's process_analog_gamepad
                                                        // Note: We can't call self.process_analog_gamepad directly here
                                                        // Instead, we use the processor directly
                                                        if let Some(processor) = &analog_processor_clone {
                                                            // Get layer-specific calibration
                                                            let lm_read = lm_clone.read().await;
                                                            let device_state = lm_read.get_device_state(&id_clone).await;
                                                            let layer_id = device_state.as_ref()
                                                                .map(|s| s.get_effective_layer())
                                                                .unwrap_or(0);

                                                            let layer_calibration = device_state
                                                                .and_then(|s| {
                                                                    s.get_layer_config(layer_id)
                                                                        .and_then(|c| c.analog_calibration().cloned())
                                                                });

                                                            drop(lm_read);

                                                            // Process with calibration
                                                            let result = if let Some(cal) = layer_calibration {
                                                                processor.process_as_gamepad_with_calibration(
                                                                    dpad_state.x, dpad_state.y, &cal
                                                                ).await
                                                            } else {
                                                                processor.process_as_gamepad(
                                                                    &id_clone, dpad_state.x, dpad_state.y
                                                                ).await
                                                            };

                                                            if let Some((x, y)) = result {
                                                                use crate::gamepad_device::GamepadAxis;
                                                                let _ = gamepad_clone.emit_axis(GamepadAxis::ABS_X, x);
                                                                let _ = gamepad_clone.emit_axis(GamepadAxis::ABS_Y, y);
                                                                debug!("Gamepad output: device={}, X={}, Y={}", id_clone, x, y);
                                                            }
                                                        }
                                                    });

                                                    // Skip sending to macro engine when in gamepad mode
                                                    continue;
                                                }
                                                _ => {}
                                            }
                                        }

                                        // WASD mode handling
                                        if analog_mode == crate::analog_processor::AnalogMode::Wasd
                                        {
                                            // Track both axes for WASD processing
                                            match axis {
                                                AbsoluteAxisType::ABS_X => {
                                                    dpad_state.x = value; // Reuse dpad_state for tracking
                                                }
                                                AbsoluteAxisType::ABS_Y => {
                                                    dpad_state.y = value; // Reuse dpad_state for tracking

                                                    // Process WASD output when we have both axes
                                                    let injector_clone = injector.clone();
                                                    let lm_clone = Arc::clone(lm);
                                                    let id_clone = id.clone();
                                                    let analog_processor_clone =
                                                        analog_processor.clone();
                                                    let wasd_keys_clone =
                                                        wasd_previous_keys.clone();

                                                    rt.block_on(async move {
                                                        if let (Some(processor), Some(inj)) = (
                                                            &analog_processor_clone,
                                                            &injector_clone,
                                                        ) {
                                                            // Get layer-specific calibration
                                                            let lm_read = lm_clone.read().await;
                                                            let device_state = lm_read
                                                                .get_device_state(&id_clone)
                                                                .await;
                                                            let layer_id = device_state
                                                                .as_ref()
                                                                .map(|s| s.get_effective_layer())
                                                                .unwrap_or(0);

                                                            let layer_calibration = device_state
                                                                .and_then(|s| {
                                                                    s.get_layer_config(layer_id)
                                                                        .and_then(|c| {
                                                                            c.analog_calibration()
                                                                                .cloned()
                                                                        })
                                                                });

                                                            drop(lm_read);

                                                            // Get calibration or use default
                                                            let calibration = if let Some(cal) =
                                                                layer_calibration
                                                            {
                                                                cal
                                                            } else {
                                                                // Create default calibration
                                                                use crate::analog_calibration::{
                                                                    AnalogCalibration,
                                                                    DeadzoneShape,
                                                                    SensitivityCurve,
                                                                };
                                                                AnalogCalibration {
                                                                    deadzone: 0.15,
                                                                    deadzone_shape:
                                                                        DeadzoneShape::Circular,
                                                                    sensitivity:
                                                                        SensitivityCurve::Linear,
                                                                    sensitivity_multiplier: 1.0,
                                                                    range_min: -32768,
                                                                    range_max: 32767,
                                                                    invert_x: false,
                                                                    invert_y: false,
                                                                }
                                                            };

                                                            // Process as WASD
                                                            let current_keys = processor
                                                                .process_as_wasd(
                                                                    &calibration,
                                                                    dpad_state.x,
                                                                    dpad_state.y,
                                                                );

                                                            // Track previous state and emit proper press/release events
                                                            let mut prev_keys =
                                                                wasd_keys_clone.write().await;
                                                            let inj_lock = inj.write().await;

                                                            // Release keys that are no longer active
                                                            for (key, _) in &*prev_keys {
                                                                if !current_keys
                                                                    .iter()
                                                                    .any(|(k, _)| k == key)
                                                                {
                                                                    let _ = inj_lock
                                                                        .key_release(key.0)
                                                                        .await;
                                                                }
                                                            }

                                                            // Press keys that are newly active
                                                            for (key, _) in &current_keys {
                                                                if !prev_keys
                                                                    .iter()
                                                                    .any(|(k, _)| k == key)
                                                                {
                                                                    let _ = inj_lock
                                                                        .key_press(key.0)
                                                                        .await;
                                                                }
                                                            }
                                                            drop(inj_lock);

                                                            // Update previous state for next iteration
                                                            *prev_keys = current_keys;
                                                            drop(prev_keys);

                                                            debug!(
                                                                "WASD output: device={}, keys={:?}",
                                                                id_clone,
                                                                &*wasd_keys_clone.read().await
                                                            );
                                                        }
                                                    });

                                                    // Skip sending to macro engine when in WASD mode
                                                    continue;
                                                }
                                                _ => {}
                                            }
                                        }

                                        // Mouse mode handling
                                        if analog_mode == crate::analog_processor::AnalogMode::Mouse
                                        {
                                            // Track both axes for mouse processing
                                            match axis {
                                                AbsoluteAxisType::ABS_X => {
                                                    dpad_state.x = value;
                                                }
                                                AbsoluteAxisType::ABS_Y => {
                                                    dpad_state.y = value;

                                                    // Process mouse output when we have both axes
                                                    let injector_clone = injector.clone();
                                                    let lm_clone = Arc::clone(lm);
                                                    let id_clone = id.clone();
                                                    let analog_processor_clone =
                                                        analog_processor.clone();

                                                    rt.block_on(async move {
                                                        if let (Some(processor), Some(inj)) =
                                                            (&analog_processor_clone, &injector_clone)
                                                        {
                                                            // Get layer-specific calibration
                                                            let lm_read = lm_clone.read().await;
                                                            let device_state = lm_read.get_device_state(&id_clone).await;
                                                            let layer_id = device_state.as_ref()
                                                                .map(|s| s.get_effective_layer())
                                                                .unwrap_or(0);

                                                            let layer_calibration = device_state
                                                                .and_then(|s| {
                                                                    s.get_layer_config(layer_id)
                                                                        .and_then(|c| c.analog_calibration().cloned())
                                                                });

                                                            drop(lm_read);

                                                            // Get calibration or use default
                                                            let calibration = if let Some(cal) = layer_calibration {
                                                                cal
                                                            } else {
                                                                // Create default calibration
                                                                use crate::analog_calibration::{AnalogCalibration, DeadzoneShape, SensitivityCurve};
                                                                AnalogCalibration {
                                                                    deadzone: 0.15,
                                                                    deadzone_shape: DeadzoneShape::Circular,
                                                                    sensitivity: SensitivityCurve::Linear,
                                                                    sensitivity_multiplier: 1.0,
                                                                    range_min: -32768,
                                                                    range_max: 32767,
                                                                    invert_x: false,
                                                                    invert_y: false,
                                                                }
                                                            };

                                                            // Get default mouse velocity config
                                                            let mouse_config = crate::analog_processor::default_mouse_velocity_config();

                                                            // Process as mouse
                                                            if let Some((vel_x, vel_y)) = processor.process_as_mouse(
                                                                &calibration,
                                                                dpad_state.x,
                                                                dpad_state.y,
                                                                &mouse_config,
                                                            ) {
                                                                let inj_lock = inj.write().await;
                                                                let _ = inj_lock.mouse_move(vel_x, vel_y).await;
                                                                debug!("Mouse output: device={}, velocity=({}, {})", id_clone, vel_x, vel_y);
                                                            }
                                                        }
                                                    });

                                                    // Skip sending to macro engine when in mouse mode
                                                    continue;
                                                }
                                                _ => {}
                                            }
                                        }

                                        // Camera mode handling
                                        if analog_mode
                                            == crate::analog_processor::AnalogMode::Camera
                                        {
                                            // Track both axes for camera processing
                                            match axis {
                                                AbsoluteAxisType::ABS_X => {
                                                    dpad_state.x = value;
                                                }
                                                AbsoluteAxisType::ABS_Y => {
                                                    dpad_state.y = value;

                                                    // Process camera output when we have both axes
                                                    let injector_clone = injector.clone();
                                                    let lm_clone = Arc::clone(lm);
                                                    let id_clone = id.clone();
                                                    let analog_processor_clone =
                                                        analog_processor.clone();

                                                    rt.block_on(async move {
                                                        if let (Some(processor), Some(inj)) =
                                                            (&analog_processor_clone, &injector_clone)
                                                        {
                                                            // Get layer-specific calibration and camera mode
                                                            let lm_read = lm_clone.read().await;
                                                            let device_state = lm_read.get_device_state(&id_clone).await;
                                                            let layer_id = device_state.as_ref()
                                                                .map(|s| s.get_effective_layer())
                                                                .unwrap_or(0);

                                                            // Extract values before dropping lm_read
                                                            let (layer_calibration, camera_mode) = if let Some(state) = device_state {
                                                                let config = state.get_layer_config(layer_id);
                                                                let cal = config.and_then(|c| c.analog_calibration().cloned());
                                                                let mode = config.map(|c| c.camera_output_mode())
                                                                    .unwrap_or(crate::analog_processor::CameraOutputMode::Scroll);
                                                                (cal, mode)
                                                            } else {
                                                                (None, crate::analog_processor::CameraOutputMode::Scroll)
                                                            };

                                                            drop(lm_read);

                                                            // Get calibration or use default
                                                            let calibration = if let Some(cal) = layer_calibration {
                                                                cal
                                                            } else {
                                                                // Create default calibration
                                                                use crate::analog_calibration::{AnalogCalibration, DeadzoneShape, SensitivityCurve};
                                                                AnalogCalibration {
                                                                    deadzone: 0.15,
                                                                    deadzone_shape: DeadzoneShape::Circular,
                                                                    sensitivity: SensitivityCurve::Linear,
                                                                    sensitivity_multiplier: 1.0,
                                                                    range_min: -32768,
                                                                    range_max: 32767,
                                                                    invert_x: false,
                                                                    invert_y: false,
                                                                }
                                                            };

                                                            // Process as camera
                                                            if let Some(output) = processor.process_as_camera(
                                                                &calibration,
                                                                dpad_state.x,
                                                                dpad_state.y,
                                                                camera_mode,
                                                            ) {
                                                                use crate::analog_processor::CameraOutput;
                                                                match output {
                                                                    CameraOutput::Scroll(amount) => {
                                                                        let inj_lock = inj.write().await;
                                                                        let _ = inj_lock.mouse_scroll(amount).await;
                                                                        debug!("Camera scroll: device={}, amount={}", id_clone, amount);
                                                                    }
                                                                    CameraOutput::Keys(keys) => {
                                                                        // Emit keys for camera control
                                                                        let inj_lock = inj.write().await;
                                                                        for key in &keys {
                                                                            let _ = inj_lock.key_press(key.0).await;
                                                                            let _ = inj_lock.key_release(key.0).await;  // Immediate for repeat
                                                                        }
                                                                        debug!("Camera keys: device={}, keys={:?}", id_clone, &keys);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    });

                                                    // Skip sending to macro engine when in camera mode
                                                    continue;
                                                }
                                                _ => {}
                                            }
                                        }
                                    }

                                    // Encode absolute axis events in the message format
                                    // We use high key codes to represent analog events:
                                    // 61000: ABS_X, 61001: ABS_Y, 61002: ABS_Z, 61003: ABS_RX, 61004: ABS_RY, 61005: ABS_RZ
                                    let analog_event_code = match axis {
                                        AbsoluteAxisType::ABS_X => 61000,
                                        AbsoluteAxisType::ABS_Y => 61001,
                                        AbsoluteAxisType::ABS_Z => 61002,
                                        AbsoluteAxisType::ABS_RX => 61003,
                                        AbsoluteAxisType::ABS_RY => 61004,
                                        AbsoluteAxisType::ABS_RZ => 61005,
                                        _ => {
                                            debug!("Unsupported absolute axis: {:?}", axis);
                                            continue; // Skip unsupported axes
                                        }
                                    };

                                    let sender_clone = sender.clone();
                                    let path_clone = path.clone();

                                    // Send as key event (analog input is treated like a key for macro engine)
                                    let msg = DeviceEventMessage::key_event(
                                        path_clone,
                                        analog_event_code,
                                        analog_event_code,
                                        value,
                                    );
                                    if let Err(e) = rt.block_on(sender_clone.send(msg)) {
                                        error!("Failed to send analog event: {}", e);
                                        return;
                                    }
                                }

                                // Ignore other event types
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error reading event from {}: {}", path, e);
                        break;
                    }
                }
            }

            info!("Event reader stopped for {}", path);
        });

        Ok(())
    }

    /// Check if a device name is specific enough to be displayed
    /// Filters out generic names from incomplete sysfs detection
    fn is_valid_device_name(name: &str) -> bool {
        let generic_names = ["Razer Device", "Unknown Razer Device", "Unknown Device"];
        let trimmed = name.trim();
        // Filter if empty, too short, or matches generic names
        !trimmed.is_empty() && trimmed.len() > 5 && !generic_names.contains(&trimmed)
    }

    /// Get a unique physical identifier for a device
    /// Groups multiple event nodes from the same physical hardware
    fn get_physical_id(device: &DeviceInfo) -> String {
        // Strip the "/inputX" or node-specific suffix from physical path if present
        let phys_base = if let Some(idx) = device.phys.rfind('/') {
            &device.phys[..idx]
        } else {
            &device.phys
        };
        format!(
            "{:04x}:{:04x}:{}",
            device.vendor_id, device.product_id, phys_base
        )
    }

    /// Scan for input devices with robust physical deduplication
    async fn scan_devices(&self) -> Result<Vec<DeviceInfo>, Box<dyn std::error::Error>> {
        use std::collections::HashMap;
        let mut physical_groups: HashMap<String, Vec<DeviceInfo>> = HashMap::new();

        // 1. Discover Razer devices through openrazer sysfs
        if let Ok(razer_devices) = self.scan_razer_sysfs().await {
            for device in razer_devices {
                if Self::is_valid_device_name(&device.name) {
                    let phys_id = Self::get_physical_id(&device);
                    physical_groups.entry(phys_id).or_default().push(device);
                }
            }
        }

        // 2. Scan all /dev/input/event* devices
        for entry in fs::read_dir("/dev/input")? {
            let entry = entry?;
            let path = entry.path();

            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with("event") {
                    if let Ok(ev_device) = EvdevDevice::open(&path) {
                        if !Self::is_input_device(&ev_device) {
                            continue;
                        }

                        if let Ok(device_info) = self.get_device_info(&path).await {
                            let phys_id = Self::get_physical_id(&device_info);
                            let group = physical_groups.entry(phys_id).or_default();

                            // Only add if this specific path isn't already in the group
                            if !group.iter().any(|d| d.path == device_info.path) {
                                group.push(device_info);
                            }
                        }
                    }
                }
            }
        }

        // 3. Merge each group into a single representative DeviceInfo
        let mut merged_devices = Vec::new();
        for (_phys_id, mut group) in physical_groups {
            if group.is_empty() {
                continue;
            }

            // Sort by "quality" of classification (Keypad > Keyboard > Mouse > Other)
            group.sort_by_key(|d| match d.device_type {
                aethermap_common::DeviceType::Keypad => 0,
                aethermap_common::DeviceType::Keyboard => 1,
                aethermap_common::DeviceType::Mouse => 2,
                aethermap_common::DeviceType::Gamepad => 3,
                aethermap_common::DeviceType::Other => 4,
            });

            // The first element is now our best representative
            let mut representative = group.remove(0);

            // If the representative has a generic name and a sibling has a better one, swap
            for sibling in &group {
                if (representative.name.contains("Keyboard")
                    || representative.name.contains("Mouse"))
                    && !sibling.name.contains("Keyboard")
                    && !sibling.name.contains("Mouse")
                    && sibling.name.len() > 5
                {
                    representative.name = sibling.name.clone();
                }
            }

            merged_devices.push(representative);
        }

        Ok(merged_devices)
    }

    /// Get device information by opening it with evdev
    async fn get_device_info(
        &self,
        path: &PathBuf,
    ) -> Result<DeviceInfo, Box<dyn std::error::Error>> {
        let device = EvdevDevice::open(path)
            .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;

        let name = device.name().unwrap_or("Unknown Device").to_string();

        // Get input_id from evdev
        let input_id = device.input_id();

        let phys = device.physical_path().unwrap_or("unknown").to_string();

        // Detect device type from capabilities
        let device_type = Self::detect_device_type(&device);

        Ok(DeviceInfo {
            name,
            path: path.clone(),
            vendor_id: input_id.vendor(),
            product_id: input_id.product(),
            phys,
            device_type,
        })
    }

    /// Scan for Razer devices via sysfs (direct integration with OpenRazer)
    async fn scan_razer_sysfs(&self) -> Result<Vec<DeviceInfo>, Box<dyn std::error::Error>> {
        let mut devices: Vec<DeviceInfo> = Vec::new();

        // OpenRazer kernel module exposes devices at these paths
        let driver_paths = vec![
            "/sys/bus/hid/drivers/razerkbd",
            "/sys/bus/hid/drivers/razermouse",
            "/sys/bus/hid/drivers/razerchroma",
        ];

        for driver_path in driver_paths {
            if let Ok(entries) = fs::read_dir(driver_path) {
                for entry in entries {
                    let entry = entry?;
                    let path = entry.path();

                    // Device directories have format: XXXX:1532:YYYY.ZZZZ
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if name.contains(":1532:") {
                            // This is a Razer device (VID 1532)
                            if let Ok(device_info) = self.parse_razer_sysfs(&path).await {
                                info!("Found Razer device in sysfs: {}", device_info.name);
                                devices.push(device_info);
                            }
                        }
                    }
                }
            }
        }

        Ok(devices)
    }

    /// Parse Razer device information from sysfs
    async fn parse_razer_sysfs(
        &self,
        sysfs_path: &Path,
    ) -> Result<DeviceInfo, Box<dyn std::error::Error>> {
        // Extract device type from sysfs
        let device_type_path = sysfs_path.join("device_type");
        let device_type = if device_type_path.exists() {
            fs::read_to_string(&device_type_path)
                .unwrap_or_else(|_| "Unknown Razer Device".to_string())
                .trim()
                .to_string()
        } else {
            "Razer Device".to_string()
        };

        // Parse VID/PID from directory name (format: XXXX:1532:YYYY.ZZZZ)
        let dir_name = sysfs_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let parts: Vec<&str> = dir_name.split(':').collect();
        let (vendor_id, product_id) = if parts.len() >= 3 {
            let vid = u16::from_str_radix("1532", 16).unwrap_or(0x1532);
            let pid_part = parts[2].split('.').next().unwrap_or("0000");
            let pid = u16::from_str_radix(pid_part, 16).unwrap_or(0);
            (vid, pid)
        } else {
            (0x1532, 0x0000)
        };

        // Find the corresponding /dev/input/event* device
        let event_path = self
            .find_event_device_for_sysfs(sysfs_path)
            .await
            .unwrap_or_else(|| PathBuf::from("/dev/input/event0"));

        // Detect device type from the event path if possible
        let detected_type = if let Ok(device) = EvdevDevice::open(&event_path) {
            Self::detect_device_type(&device)
        } else {
            // Fallback to name-based heuristic if we can't open the device yet
            if device_type.to_lowercase().contains("mouse") {
                aethermap_common::DeviceType::Mouse
            } else if device_type.to_lowercase().contains("keyboard")
                || device_type.to_lowercase().contains("keypad")
            {
                aethermap_common::DeviceType::Keyboard
            } else {
                aethermap_common::DeviceType::Other
            }
        };

        Ok(DeviceInfo {
            name: device_type,
            path: event_path,
            vendor_id,
            product_id,
            phys: sysfs_path.to_string_lossy().to_string(),
            device_type: detected_type,
        })
    }

    /// Find the /dev/input/event* path for a sysfs device
    async fn find_event_device_for_sysfs(&self, sysfs_path: &Path) -> Option<PathBuf> {
        // Look for input subdirectory
        let input_dir = sysfs_path.join("input");
        if !input_dir.exists() {
            return None;
        }

        // Find input* directory
        if let Ok(entries) = fs::read_dir(&input_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if name.starts_with("input") {
                        // Look for event* in this directory
                        if let Ok(event_entries) = fs::read_dir(&path) {
                            for event_entry in event_entries.flatten() {
                                let event_path = event_entry.path();
                                if let Some(event_name) =
                                    event_path.file_name().and_then(|s| s.to_str())
                                {
                                    if event_name.starts_with("event") {
                                        // Extract event number
                                        let event_num = event_name.replace("event", "");
                                        return Some(PathBuf::from(format!(
                                            "/dev/input/event{}",
                                            event_num
                                        )));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Create a fallback device for testing
    #[allow(dead_code)]
    fn create_fallback_device(&self, device_type: &str) -> DeviceInfo {
        use aethermap_common::DeviceType;
        DeviceInfo {
            name: format!("Fallback {}", device_type),
            path: PathBuf::from(format!(
                "/dev/input/event{}",
                if device_type == "keyboard" { "0" } else { "1" }
            )),
            vendor_id: 0x1532,
            product_id: 0x0220,
            phys: "fallback-device".to_string(),
            device_type: DeviceType::Other,
        }
    }

    /// Shutdown the device manager
    ///
    /// This method performs a best-effort shutdown: it attempts to ungrab all devices
    /// and continues even if individual ungrab operations fail. All errors are logged
    /// but the method returns Ok() to allow shutdown to proceed.
    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Shutting down device manager");

        // Ungrab all devices, collecting errors for summary
        let device_paths: Vec<String> = self.grabbed_devices.keys().cloned().collect();
        let mut errors = Vec::new();

        for path in device_paths {
            if let Err(e) = self.ungrab_device(&path).await {
                warn!("Error ungrabbing device {}: {}", path, e);
                errors.push((path, e));
            }
        }

        // Log summary if any errors occurred
        if !errors.is_empty() {
            warn!("Shutdown completed with {} errors:", errors.len());
            for (path, e) in &errors {
                warn!("  - {}: {}", path, e);
            }
        }

        info!("Device manager shutdown complete");
        Ok(())
    }

    /// Activate a remap profile for a grabbed device
    ///
    /// This method performs atomic profile switching by exchanging the Arc pointer
    /// to the remappings HashMap. The remap engine uses the new profile for all
    /// subsequent events.
    ///
    /// # Arguments
    ///
    /// * `device_path` - Path to the grabbed device
    /// * `profile` - RemapProfile to activate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Profile activated successfully
    /// * `Err(Box<dyn Error>)` - Device not grabbed or other error
    pub async fn activate_profile(
        &mut self,
        device_path: &str,
        profile: RemapProfile,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let device = self
            .grabbed_devices
            .get_mut(device_path)
            .ok_or_else(|| format!("Device not grabbed: {}", device_path))?;

        info!(
            "Activating profile '{}' for device {}",
            profile.name(),
            device_path
        );

        // Store profile name
        device.active_profile = Some(profile.name().to_string());

        // Get the remappings Arc from the profile
        let remappings = profile.remaps_arc().clone();

        // Store in device for O(1) lookup during event processing
        device.active_remappings = Some(remappings);

        // If device has a remap engine, update it
        if let Some(remap_engine) = &device.remap_engine {
            // Convert RemapProfile remappings to HashMap<String, String> for RemapEngine
            let remaps_hash = profile.get_remaps().await;
            let config_hash: HashMap<String, String> = remaps_hash
                .iter()
                .map(|(k, v)| (format!("{:?}", k), format!("{:?}", v)))
                .collect();

            if let Err(e) = remap_engine.load_config(&config_hash).await {
                warn!("Failed to load profile into remap engine: {}", e);
            } else {
                info!("Profile '{}' loaded into remap engine", profile.name());
            }
        }

        // Reload analog configuration from profile if available
        let device_id = Self::format_device_id(device.info.vendor_id, device.info.product_id);
        if let Some(config_manager) = &self.config_manager {
            if let Ok(analog_configs) = config_manager.load_analog_configs().await {
                if let Some(analog_config) = analog_configs.get(&device_id) {
                    if let Some(processor) = &self.analog_processor {
                        if let Err(e) = processor.load_config(&device_id, analog_config).await {
                            warn!(
                                "Failed to reload analog config for device {}: {}",
                                device_id, e
                            );
                        } else {
                            info!(
                                "Reloaded analog config for device {} on profile activation",
                                device_id
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Deactivate the current profile for a grabbed device
    ///
    /// Removes the active profile and clears custom remappings.
    ///
    /// # Arguments
    ///
    /// * `device_path` - Path to the grabbed device
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Profile deactivated successfully
    /// * `Err(Box<dyn Error>)` - Device not grabbed or other error
    pub async fn deactivate_profile(
        &mut self,
        device_path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let device = self
            .grabbed_devices
            .get_mut(device_path)
            .ok_or_else(|| format!("Device not grabbed: {}", device_path))?;

        let prev_profile = device.active_profile.take();
        device.active_remappings = None;

        if let Some(profile_name) = &prev_profile {
            info!(
                "Deactivated profile '{}' for device {}",
                profile_name, device_path
            );

            // Clear remap engine if present
            if let Some(remap_engine) = &device.remap_engine {
                remap_engine.clear().await;
            }
        }

        Ok(())
    }

    /// Get the currently active profile for a device
    ///
    /// # Arguments
    ///
    /// * `device_path` - Path to the grabbed device
    ///
    /// # Returns
    ///
    /// * `Some(String)` - Name of active profile
    /// * `None` - No active profile or device not grabbed
    pub fn get_active_profile(&self, device_path: &str) -> Option<String> {
        self.grabbed_devices
            .get(device_path)?
            .active_profile
            .clone()
    }

    /// Get analog configuration for a device
    ///
    /// Returns the current analog configuration for a device from the AnalogProcessor.
    /// This is used by IPC handlers to retrieve the current calibration settings.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    ///
    /// # Returns
    ///
    /// * `Some(AnalogDeviceConfig)` - Current analog configuration
    /// * `None` - Device not configured or AnalogProcessor not available
    pub async fn get_analog_config(
        &self,
        device_id: &str,
    ) -> Option<crate::config::AnalogDeviceConfig> {
        if let Some(processor) = &self.analog_processor {
            processor.save_config(device_id).await.ok()
        } else {
            None
        }
    }

    /// Process analog stick event for gamepad output
    ///
    /// This method is called from the device event loop when an analog stick
    /// event is detected and the active layer's analog_mode is Gamepad.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    /// * `raw_x` - Raw X axis value (0-255)
    /// * `raw_y` - Raw Y axis value (0-255)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Event processed successfully
    /// * `Err(String)` - Processing failed
    pub async fn process_analog_gamepad(
        &self,
        device_id: &str,
        raw_x: i32,
        raw_y: i32,
    ) -> Result<(), String> {
        // Get the effective layer and layer config for this device
        let (effective_layer, layer_calibration) = if let Some(lm) = &self.layer_manager {
            let lm_read = lm.read().await;

            // Get effective layer ID
            let device_state = lm_read.get_device_state(device_id).await;
            let layer_id = device_state
                .as_ref()
                .map(|s| s.get_effective_layer())
                .unwrap_or(0);

            // Get layer-specific calibration if available
            let calibration = device_state.and_then(|state| {
                state
                    .get_layer_config(layer_id)
                    .and_then(|config| config.analog_calibration().cloned())
            });

            (layer_id, calibration)
        } else {
            (0, None) // Default to base layer if no layer manager
        };

        debug!(
            "Processing analog gamepad: device={}, layer={}, raw_x={}, raw_y={}, layer_calib={}",
            device_id,
            effective_layer,
            raw_x,
            raw_y,
            layer_calibration.is_some()
        );

        // Process through calibration pipeline
        let processor = self
            .analog_processor
            .as_ref()
            .ok_or("AnalogProcessor not initialized")?;

        // Use layer-specific calibration if configured, otherwise use device default
        let calibration_result = if let Some(cal) = layer_calibration {
            processor
                .process_as_gamepad_with_calibration(raw_x, raw_y, &cal)
                .await
        } else {
            processor.process_as_gamepad(device_id, raw_x, raw_y).await
        };

        if let Some((x, y)) = calibration_result {
            // Emit to virtual gamepad device
            use crate::gamepad_device::GamepadAxis;

            self.gamepad_device
                .emit_axis(GamepadAxis::ABS_X, x)
                .map_err(|e| format!("Failed to emit X axis: {}", e))?;
            self.gamepad_device
                .emit_axis(GamepadAxis::ABS_Y, y)
                .map_err(|e| format!("Failed to emit Y axis: {}", e))?;

            debug!("Emitted gamepad axis: X={}, Y={}", x, y);
        } else {
            debug!(
                "Analog event filtered by deadzone: device=({}, {}, {})",
                device_id, raw_x, raw_y
            );
        }

        Ok(())
    }
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    ///
    /// # Returns
    ///
    /// * `Some(String)` - Name of active profile
    /// * `None` - No active profile or device not found
    pub fn get_active_profile_by_id(&self, device_id: &str) -> Option<String> {
        // Find device path by device ID
        for device in self.grabbed_devices.values() {
            let id = Self::format_device_id(device.info.vendor_id, device.info.product_id);
            if id == device_id {
                return device.active_profile.clone();
            }
        }
        None
    }

    /// Find the device path for a grabbed device by its device ID
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    ///
    /// # Returns
    ///
    /// * `Some(String)` - Device path if found
    /// * `None` - Device not grabbed
    pub fn get_device_path_by_id(&self, device_id: &str) -> Option<String> {
        for (path, device) in &self.grabbed_devices {
            let id = Self::format_device_id(device.info.vendor_id, device.info.product_id);
            if id == device_id {
                return Some(path.clone());
            }
        }
        None
    }

    /// Check if a remapped key should use the device's custom profile
    ///
    /// This method is called during event processing to check if the device
    /// has an active profile with custom remappings.
    ///
    /// # Arguments
    ///
    /// * `device_path` - Path to the device
    /// * `key_code` - The input key code
    ///
    /// # Returns
    ///
    /// * `Some(Key)` - Remapped output key if profile has remapping
    /// * `None` - No remapping for this key or no active profile
    pub fn check_profile_remapping(&self, device_path: &str, _key_code: Key) -> Option<Key> {
        if let Some(device) = self.grabbed_devices.get(device_path) {
            if let Some(_remappings) = &device.active_remappings {
                // Note: This is a synchronous check - in real usage, we'd need async
                // For now, return None as we need async to read from RwLock
                // The actual check happens in the event reader loop
                return None;
            }
        }
        None
    }

    /// Get active remaps for a device
    ///
    /// Returns the currently active profile name and its remap table.
    ///
    /// # Arguments
    ///
    /// * `device_path` - Path to the grabbed device
    ///
    /// # Returns
    ///
    /// * `Ok(Some((profile_name, remappings)))` - Active profile with remappings
    /// * `Ok(None)` - No active profile
    /// * `Err(Box<dyn Error>)` - Device not grabbed
    pub async fn get_active_remaps(
        &self,
        device_path: &str,
    ) -> Result<Option<(String, Arc<RwLock<RemapTable>>)>, Box<dyn std::error::Error + Send + Sync>>
    {
        let grabbed_device = self
            .grabbed_devices
            .get(device_path)
            .ok_or_else(|| format!("Device not grabbed: {}", device_path))?;

        if let Some(profile_name) = &grabbed_device.active_profile {
            if let Some(remappings) = &grabbed_device.active_remappings {
                return Ok(Some((profile_name.clone(), Arc::clone(remappings))));
            }
        }

        Ok(None)
    }

    /// Activate a remap profile for a device by name
    ///
    /// This method looks up a profile by name from the stored device_profiles
    /// and activates it for the specified device.
    ///
    /// # Arguments
    ///
    /// * `device_path` - Path to the grabbed device
    /// * `profile_name` - Name of the profile to activate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Profile activated successfully
    /// * `Err(Box<dyn Error>)` - Device not grabbed, profile not found, or other error
    pub async fn activate_profile_by_name(
        &mut self,
        device_path: &str,
        profile_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get the grabbed device
        let grabbed_device = self
            .grabbed_devices
            .get_mut(device_path)
            .ok_or_else(|| format!("Device not grabbed: {}", device_path))?;

        // Get device key for profile lookup
        let device_key = format!(
            "{:04x}:{:04x}",
            grabbed_device.info.vendor_id, grabbed_device.info.product_id
        );

        // Find the profile in stored profiles
        let profiles = self
            .device_profiles
            .get(&device_key)
            .ok_or_else(|| format!("No profiles found for device {}", device_key))?;

        let profile = profiles
            .iter()
            .find(|p| p.name() == profile_name)
            .ok_or_else(|| {
                format!(
                    "Profile '{}' not found for device {}",
                    profile_name, device_key
                )
            })?;

        // Clone the profile for activation
        let profile_clone = profile.clone();

        // End mutable borrow of grabbed_device before calling activate_profile
        // (which also needs mutable access to self.grabbed_devices)

        // Use the existing activate_profile method
        self.activate_profile(device_path, profile_clone).await
    }

    /// Get device info from path
    ///
    /// Returns DeviceInfo for a device at the given path.
    ///
    /// # Arguments
    ///
    /// * `device_path` - Path to the device
    ///
    /// # Returns
    ///
    /// * `Some(DeviceInfo)` - Device info if found
    /// * `None` - Device not found
    pub fn get_device_info_from_path(&self, device_path: &str) -> Option<DeviceInfo> {
        self.devices.get(device_path).cloned()
    }

    /// Check if device is an Azeron Cyborg keypad
    ///
    /// Uses USB vendor ID matching for reliable identification.
    /// Azeron Cyborg 2 uses vendor ID 0x16d0.
    ///
    /// # Arguments
    ///
    /// * `device` - The evdev device to check
    ///
    /// # Returns
    ///
    /// * `true` if device is an Azeron Cyborg
    fn is_azeron_device(device: &EvdevDevice) -> bool {
        let input_id = device.input_id();
        input_id.vendor() == AZERON_VENDOR_ID
    }

    /// Detect device type from evdev capabilities
    ///
    /// Uses kernel-reported capabilities to classify device as Keyboard, Mouse, Gamepad, Keypad, or Other.
    /// This is more reliable than name-based heuristics.
    ///
    /// # Arguments
    ///
    /// * `device` - The evdev device to inspect
    ///
    /// # Returns
    ///
    /// * `DeviceType` - The detected device type
    fn detect_device_type(device: &EvdevDevice) -> aethermap_common::DeviceType {
        use aethermap_common::DeviceType;
        use std::collections::HashSet;

        // Check for Azeron devices first (vendor ID match)
        if Self::is_azeron_device(device) {
            return DeviceType::Keypad;
        }

        // Check for relative axes (indicates mouse/pointing device)
        let has_relative_axes = device
            .supported_relative_axes()
            .is_some_and(|axes| axes.iter().next().is_some());

        // Check for absolute axes (indicates gamepad/joystick or keypad with analog stick)
        let has_absolute_axes = device
            .supported_absolute_axes()
            .is_some_and(|axes| axes.iter().next().is_some());

        // Get supported keys for button detection
        let supported_keys: HashSet<Key> = device
            .supported_keys()
            .map_or_else(HashSet::new, |keys| keys.iter().collect());

        // Count total keys for keypad vs gamepad distinction
        let key_count = supported_keys.len();

        // Check for mouse buttons (BTN_LEFT, BTN_RIGHT, etc.)
        let has_mouse_buttons = supported_keys.contains(&Key::BTN_LEFT)
            || supported_keys.contains(&Key::BTN_RIGHT)
            || supported_keys.contains(&Key::BTN_MIDDLE);

        // Check for gamepad buttons (BTN_SOUTH, BTN_EAST, etc. - gamepad API)
        let has_gamepad_buttons = supported_keys.contains(&Key::BTN_SOUTH)
            || supported_keys.contains(&Key::BTN_EAST)
            || supported_keys.contains(&Key::BTN_NORTH)
            || supported_keys.contains(&Key::BTN_WEST)
            || supported_keys.contains(&Key::BTN_TRIGGER)
            || supported_keys.contains(&Key::BTN_THUMB)
            || supported_keys.contains(&Key::BTN_THUMB2)
            || supported_keys.contains(&Key::BTN_MODE);

        // Check for keyboard keys (exclude mouse-only devices)
        // A real keyboard has letter keys, number keys, etc.
        let has_keyboard_keys = supported_keys.contains(&Key::KEY_A)
            || supported_keys.contains(&Key::KEY_Q)
            || supported_keys.contains(&Key::KEY_1)
            || supported_keys.contains(&Key::KEY_SPACE);

        // Classification logic (order matters - more specific first)
        if has_keyboard_keys && key_count > 20 {
            // Significant number of keyboard keys = Keyboard
            // This prevents gaming mice with many buttons from being misclassified if they also send keyboard events
            DeviceType::Keyboard
        } else if has_relative_axes && has_mouse_buttons {
            // Device with relative axes and mouse buttons is likely a mouse
            DeviceType::Mouse
        } else if has_absolute_axes && key_count > 30 {
            // Device with absolute axes and many keys is a keypad (e.g., Azeron Cyborg)
            DeviceType::Keypad
        } else if has_gamepad_buttons {
            // Device with gamepad buttons but fewer keys is a gamepad
            DeviceType::Gamepad
        } else if has_keyboard_keys {
            // Minimal keyboard keys
            DeviceType::Keyboard
        } else {
            // No recognizable patterns, or minimal keys
            DeviceType::Other
        }
    }

    /// Check if device is an input device we care about
    ///
    /// Filters out non-input devices like audio devices, sensors,
    /// power buttons, and other devices that don't generate user input.
    ///
    /// # Arguments
    ///
    /// * `device` - The evdev device to check
    ///
    /// # Returns
    ///
    /// * `true` - Device is a keyboard, mouse, gamepad, or keypad
    /// * `false` - Device is not a relevant input device
    fn is_input_device(device: &EvdevDevice) -> bool {
        use aethermap_common::DeviceType;

        // Skip our own virtual device and input-remapper devices to prevent loops/noise
        if let Some(name) = device.name() {
            if name.contains("Aethermap Virtual") || name.contains("input-remapper") {
                return false;
            }
        }

        // Use our device type detection
        let device_type = Self::detect_device_type(device);

        matches!(
            device_type,
            DeviceType::Keyboard | DeviceType::Mouse | DeviceType::Gamepad | DeviceType::Keypad
        )
    }
}

#[cfg(test)]
#[allow(unused_variables)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_manager_creation() {
        let manager = DeviceManager::new();
        assert!(manager.devices.is_empty());
        assert!(manager.grabbed_devices.is_empty());
    }

    #[tokio::test]
    async fn test_device_discovery() {
        let mut manager = DeviceManager::new();

        // This test requires /dev/input access
        // In a non-privileged test environment, it may fail
        let result = manager.start_discovery().await;

        // Just check that it doesn't panic
        // In a real test environment with devices, check for devices
        if result.is_ok() {
            // We should have found some devices (at least virtual ones)
            // On a system with no input devices, this may be empty
            println!("Found {} devices", manager.get_devices().len());
        }
    }

    #[tokio::test]
    async fn test_device_id_formatting() {
        let manager = DeviceManager::new();

        // Test Razer device ID formatting
        let device_id = DeviceManager::format_device_id(0x1532, 0x0220);
        assert_eq!(device_id, "1532:0220");

        // Test different device
        let device_id = DeviceManager::format_device_id(0x046D, 0xC52B);
        assert_eq!(device_id, "046d:c52b");
    }

    #[tokio::test]
    async fn test_device_type_enum_exists() {
        use aethermap_common::DeviceType;

        // Verify DeviceType enum has the expected variants
        let _keyboard = DeviceType::Keyboard;
        let _mouse = DeviceType::Mouse;
        let _gamepad = DeviceType::Gamepad;
        let _other = DeviceType::Other;

        // Verify Display impl works
        assert_eq!(DeviceType::Keyboard.to_string(), "Keyboard");
        assert_eq!(DeviceType::Mouse.to_string(), "Mouse");
        assert_eq!(DeviceType::Gamepad.to_string(), "Gamepad");
        assert_eq!(DeviceType::Other.to_string(), "Other");
    }

    #[tokio::test]
    async fn test_activate_profile_on_device() {
        use std::collections::HashMap;

        let mut manager = DeviceManager::new();

        // Create a test profile
        let mut config = HashMap::new();
        config.insert("capslock".to_string(), "leftctrl".to_string());

        let profile = crate::remap_engine::RemapProfile::new("test".to_string(), &config).unwrap();

        // We can't fully test without grabbing a device, but we can test the API
        // The profile activation will fail with "Device not grabbed" which is expected
        let result = manager.activate_profile("/dev/input/event0", profile).await;
        assert!(result.is_err()); // Device not grabbed

        // Check that no active profile is set
        assert_eq!(manager.get_active_profile("/dev/input/event0"), None);
    }

    #[tokio::test]
    async fn test_format_device_id_case_insensitive() {
        assert_eq!(DeviceManager::format_device_id(0xABCD, 0x1234), "abcd:1234");
        assert_eq!(DeviceManager::format_device_id(0xFFFF, 0x0000), "ffff:0000");
    }
}

// Integration tests for gamepad mode (plan 14-06)
#[cfg(test)]
mod gamepad_tests {
    use super::*;
    use crate::analog_calibration::{AnalogCalibration, DeadzoneShape, SensitivityCurve};
    use crate::analog_processor::AnalogMode;
    use crate::layer_manager::LayerManager;

    #[tokio::test]
    async fn test_process_analog_gamepad_with_gamepad_mode() {
        // Create layer manager with gamepad mode on layer 1
        let layer_manager = Arc::new(LayerManager::new(None));

        // Create device state first
        let mut device_state = layer_manager.get_or_create_device_state("32b6:12f7").await;

        // Set layer 1 to gamepad mode (modify locally)
        device_state.layer_configs[1].analog_mode = AnalogMode::Gamepad;

        // Now we need to test the state's behavior directly
        // since set_layer_config doesn't update analog_mode yet
        assert_eq!(
            device_state.layer_configs[1].analog_mode,
            AnalogMode::Gamepad
        );

        // Verify we can get the effective layer after activation
        device_state.activate_layer(1);
        assert_eq!(device_state.get_effective_layer(), 1);
    }

    #[tokio::test]
    async fn test_per_layer_analog_calibration() {
        let layer_manager = Arc::new(LayerManager::new(None));

        // Get or create device state
        let mut device_state = layer_manager.get_or_create_device_state("32b6:12f7").await;

        // Set layer 1 to have custom calibration with quadratic curve
        device_state.layer_configs[1].analog_calibration = Some(AnalogCalibration {
            deadzone_shape: DeadzoneShape::Circular,
            deadzone: 0.25,
            sensitivity: SensitivityCurve::Quadratic,
            sensitivity_multiplier: 1.5,
            range_min: -32768,
            range_max: 32767,
            invert_x: false,
            invert_y: false,
        });

        // Verify layer 1 has custom calibration
        let layer_config = device_state.get_layer_config(1);
        assert!(layer_config.is_some());

        let cal = layer_config.unwrap().analog_calibration.as_ref();
        assert!(cal.is_some());
        assert_eq!(cal.unwrap().sensitivity, SensitivityCurve::Quadratic);
    }
}

// Integration tests for WASD, Mouse, and Camera modes (plan 15-08)
#[cfg(test)]
#[allow(unused_variables)]
mod analog_mode_tests {
    use super::*;
    use crate::analog_calibration::{AnalogCalibration, DeadzoneShape, SensitivityCurve};
    use crate::analog_processor::AnalogMode;
    use crate::layer_manager::LayerManager;

    #[tokio::test]
    async fn test_wasd_mode_per_layer() {
        let layer_manager = Arc::new(LayerManager::new(None));

        // Get or create device state
        let mut device_state = layer_manager.get_or_create_device_state("32b6:12f7").await;

        // Set layer 0 to Dpad mode (base layer)
        device_state.layer_configs[0].analog_mode = AnalogMode::Dpad;

        // Set layer 1 to WASD mode
        device_state.layer_configs[1].analog_mode = AnalogMode::Wasd;

        // Verify layer 0 has Dpad mode
        let layer_config = device_state.get_layer_config(0);
        assert!(layer_config.is_some());
        assert_eq!(layer_config.unwrap().analog_mode, AnalogMode::Dpad);

        // Verify layer 1 has WASD mode
        let layer_config = device_state.get_layer_config(1);
        assert!(layer_config.is_some());
        assert_eq!(layer_config.unwrap().analog_mode, AnalogMode::Wasd);
    }

    #[tokio::test]
    async fn test_mouse_mode_per_layer() {
        let layer_manager = Arc::new(LayerManager::new(None));

        // Get or create device state
        let mut device_state = layer_manager.get_or_create_device_state("32b6:12f7").await;

        // Set layer 2 to Mouse mode
        device_state.layer_configs[2].analog_mode = AnalogMode::Mouse;

        // Verify layer 2 has Mouse mode
        let layer_config = device_state.get_layer_config(2);
        assert!(layer_config.is_some());
        assert_eq!(layer_config.unwrap().analog_mode, AnalogMode::Mouse);
    }

    #[tokio::test]
    async fn test_camera_mode_per_layer() {
        let layer_manager = Arc::new(LayerManager::new(None));

        // Get or create device state
        let mut device_state = layer_manager.get_or_create_device_state("32b6:12f7").await;

        // Set layer 1 to Camera mode
        device_state.layer_configs[1].analog_mode = AnalogMode::Camera;

        // Verify layer 1 has Camera mode
        let layer_config = device_state.get_layer_config(1);
        assert!(layer_config.is_some());
        assert_eq!(layer_config.unwrap().analog_mode, AnalogMode::Camera);
    }

    #[tokio::test]
    async fn test_analog_mode_layer_switching() {
        let layer_manager = Arc::new(LayerManager::new(None));

        // Get or create device state
        let mut device_state = layer_manager.get_or_create_device_state("32b6:12f7").await;

        // Set layer 0 to Dpad mode (base layer)
        device_state.layer_configs[0].analog_mode = AnalogMode::Dpad;

        // Set layer 1 to WASD mode
        device_state.layer_configs[1].analog_mode = AnalogMode::Wasd;

        // Initially, no layers active - effective layer should be 0 (base)
        assert_eq!(device_state.get_effective_layer(), 0);
        let layer_config = device_state.get_layer_config(0);
        assert_eq!(layer_config.unwrap().analog_mode, AnalogMode::Dpad);

        // Activate layer 1
        device_state.activate_layer(1);
        assert_eq!(device_state.get_effective_layer(), 1);
        let layer_config = device_state.get_layer_config(1);
        assert_eq!(layer_config.unwrap().analog_mode, AnalogMode::Wasd);

        // Deactivate layer 1 - should fall back to base layer
        device_state.deactivate_layer(1);
        assert_eq!(device_state.get_effective_layer(), 0);
        let layer_config = device_state.get_layer_config(0);
        assert_eq!(layer_config.unwrap().analog_mode, AnalogMode::Dpad);
    }

    #[tokio::test]
    async fn test_analog_mode_with_calibration_per_layer() {
        let layer_manager = Arc::new(LayerManager::new(None));

        // Get or create device state
        let mut device_state = layer_manager.get_or_create_device_state("32b6:12f7").await;

        // Set layer 0 to WASD mode with default calibration
        device_state.layer_configs[0].analog_mode = AnalogMode::Wasd;
        device_state.layer_configs[0].analog_calibration = None; // Use defaults

        // Set layer 1 to WASD mode with custom calibration
        device_state.layer_configs[1].analog_mode = AnalogMode::Wasd;
        device_state.layer_configs[1].analog_calibration = Some(AnalogCalibration {
            deadzone_shape: DeadzoneShape::Circular,
            deadzone: 0.25, // Larger deadzone
            sensitivity: SensitivityCurve::Quadratic,
            sensitivity_multiplier: 1.5,
            range_min: -32768,
            range_max: 32767,
            invert_x: false,
            invert_y: false,
        });

        // Verify layer 0 has WASD with default calibration
        let layer_config = device_state.get_layer_config(0);
        assert_eq!(layer_config.unwrap().analog_mode, AnalogMode::Wasd);
        // Default calibration is None, which uses AnalogCalibration::default()

        // Verify layer 1 has WASD with custom calibration
        let layer_config = device_state.get_layer_config(1);
        assert_eq!(layer_config.unwrap().analog_mode, AnalogMode::Wasd);
        let cal = &layer_config.unwrap().analog_calibration;
        assert!(cal.is_some());
        assert_eq!(cal.as_ref().unwrap().deadzone, 0.25);
    }

    #[tokio::test]
    async fn test_all_analog_modes_configurable() {
        let layer_manager = Arc::new(LayerManager::new(None));

        // Get or create device state
        let mut device_state = layer_manager.get_or_create_device_state("32b6:12f7").await;

        // Set each layer to a different mode
        device_state.layer_configs[0].analog_mode = AnalogMode::Disabled;
        device_state.layer_configs[1].analog_mode = AnalogMode::Dpad;
        device_state.layer_configs[2].analog_mode = AnalogMode::Wasd;

        // Verify each layer's mode
        assert_eq!(
            device_state.get_layer_config(0).unwrap().analog_mode,
            AnalogMode::Disabled
        );
        assert_eq!(
            device_state.get_layer_config(1).unwrap().analog_mode,
            AnalogMode::Dpad
        );
        assert_eq!(
            device_state.get_layer_config(2).unwrap().analog_mode,
            AnalogMode::Wasd
        );

        // Set additional layers to other modes
        device_state.layer_configs[0].analog_mode = AnalogMode::Gamepad;
        device_state.layer_configs[1].analog_mode = AnalogMode::Camera;
        device_state.layer_configs[2].analog_mode = AnalogMode::Mouse;

        // Verify updated modes
        assert_eq!(
            device_state.get_layer_config(0).unwrap().analog_mode,
            AnalogMode::Gamepad
        );
        assert_eq!(
            device_state.get_layer_config(1).unwrap().analog_mode,
            AnalogMode::Camera
        );
        assert_eq!(
            device_state.get_layer_config(2).unwrap().analog_mode,
            AnalogMode::Mouse
        );
    }

    #[tokio::test]
    async fn test_analog_mode_default_is_disabled() {
        let layer_manager = Arc::new(LayerManager::new(None));

        // Create new device state
        let device_state = layer_manager.get_or_create_device_state("test:1234").await;

        // Verify all default layers have Disabled mode
        assert_eq!(
            device_state.layer_configs[0].analog_mode,
            AnalogMode::Disabled
        );
        assert_eq!(
            device_state.layer_configs[1].analog_mode,
            AnalogMode::Disabled
        );
        assert_eq!(
            device_state.layer_configs[2].analog_mode,
            AnalogMode::Disabled
        );
    }

    // WASD state tracking unit tests
    // These tests verify the state diffing logic for proper key press/release

    #[test]
    fn test_wasd_state_tracking_cardinal_to_diagonal() {
        use crate::analog_processor::AnalogProcessor;
        use evdev::Key;

        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();

        // Simulate: North (W) → NorthEast (W+D)
        // Expected: W stays pressed, D gets pressed, no releases

        // Event 1: Stick moves North (W active)
        // Input range: 0-255 where 128 is center, smaller Y = up
        let previous_keys: Vec<(Key, bool)> = Vec::new();
        let current_keys = processor.process_as_wasd(&calibration, 128, 30); // X=center, Y=up (small value)

        // W should be pressed (new key)
        let newly_pressed: Vec<_> = current_keys
            .iter()
            .filter(|(k, _)| !previous_keys.iter().any(|(pk, _)| pk == k))
            .collect();
        assert_eq!(newly_pressed.len(), 1);
        assert_eq!(newly_pressed[0].0, Key::KEY_W);

        // No keys should be released
        let released: Vec<_> = previous_keys
            .iter()
            .filter(|(k, _)| !current_keys.iter().any(|(ck, _)| ck == k))
            .collect();
        assert_eq!(released.len(), 0);

        // Event 2: Stick moves to NorthEast (W+D active)
        let previous_keys = current_keys;
        let current_keys = processor.process_as_wasd(&calibration, 200, 30); // X=right, Y=up

        // W should stay pressed (no new event)
        let newly_pressed: Vec<_> = current_keys
            .iter()
            .filter(|(k, _)| !previous_keys.iter().any(|(pk, _)| pk == k))
            .collect();
        assert_eq!(newly_pressed.len(), 1);
        assert_eq!(newly_pressed[0].0, Key::KEY_D);

        // No keys released
        let released: Vec<_> = previous_keys
            .iter()
            .filter(|(k, _)| !current_keys.iter().any(|(ck, _)| ck == k))
            .collect();
        assert_eq!(released.len(), 0);
    }

    #[test]
    fn test_wasd_state_tracking_diagonal_to_cardinal() {
        use crate::analog_processor::AnalogProcessor;
        use evdev::Key;

        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();

        // Simulate: NorthEast (W+D) → East (D)
        // Expected: W gets released, D stays pressed

        // Event 1: Start with NorthEast (W+D active)
        let previous_keys = processor.process_as_wasd(&calibration, 200, 30);
        assert_eq!(previous_keys.len(), 2);

        // Event 2: Stick moves to East (D active)
        let current_keys = processor.process_as_wasd(&calibration, 200, 128);
        assert_eq!(current_keys.len(), 1);
        assert_eq!(current_keys[0].0, Key::KEY_D);

        // W should be released
        let released: Vec<_> = previous_keys
            .iter()
            .filter(|(k, _)| !current_keys.iter().any(|(ck, _)| ck == k))
            .collect();
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].0, Key::KEY_W);

        // D stays pressed (no new press event)
        let newly_pressed: Vec<_> = current_keys
            .iter()
            .filter(|(k, _)| !previous_keys.iter().any(|(pk, _)| pk == k))
            .collect();
        assert_eq!(newly_pressed.len(), 0);
    }

    #[test]
    fn test_wasd_state_tracking_diagonal_to_opposite_diagonal() {
        use crate::analog_processor::AnalogProcessor;
        use evdev::Key;

        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();

        // Simulate: NorthEast (W+D) → SouthWest (S+A)
        // Expected: W and D released, S and A pressed

        // Event 1: Start with NorthEast (W+D active)
        let previous_keys = processor.process_as_wasd(&calibration, 200, 30);
        assert_eq!(previous_keys.len(), 2);

        // Event 2: Stick moves to SouthWest (S+A active)
        let current_keys = processor.process_as_wasd(&calibration, 30, 200);
        assert_eq!(current_keys.len(), 2);

        // W and D should be released
        let released: Vec<_> = previous_keys
            .iter()
            .filter(|(k, _)| !current_keys.iter().any(|(ck, _)| ck == k))
            .collect();
        assert_eq!(released.len(), 2);
        let released_keys: Vec<Key> = released.iter().map(|(k, _)| *k).collect();
        assert!(released_keys.contains(&Key::KEY_W));
        assert!(released_keys.contains(&Key::KEY_D));

        // S and A should be pressed
        let newly_pressed: Vec<_> = current_keys
            .iter()
            .filter(|(k, _)| !previous_keys.iter().any(|(pk, _)| pk == k))
            .collect();
        assert_eq!(newly_pressed.len(), 2);
        let pressed_keys: Vec<Key> = newly_pressed.iter().map(|(k, _)| *k).collect();
        assert!(pressed_keys.contains(&Key::KEY_S));
        assert!(pressed_keys.contains(&Key::KEY_A));
    }

    #[test]
    fn test_wasd_state_tracking_center_to_direction() {
        use crate::analog_processor::AnalogProcessor;
        use evdev::Key;

        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();

        // Simulate: Center (no keys) → North (W)
        // Expected: W pressed, no releases

        // Event 1: Stick in deadzone (center)
        let previous_keys = processor.process_as_wasd(&calibration, 128, 128);
        assert_eq!(previous_keys.len(), 0);

        // Event 2: Stick moves North (W active)
        let current_keys = processor.process_as_wasd(&calibration, 128, 30);
        assert_eq!(current_keys.len(), 1);
        assert_eq!(current_keys[0].0, Key::KEY_W);

        // W should be pressed (new key)
        let newly_pressed: Vec<_> = current_keys
            .iter()
            .filter(|(k, _)| !previous_keys.iter().any(|(pk, _)| pk == k))
            .collect();
        assert_eq!(newly_pressed.len(), 1);
        assert_eq!(newly_pressed[0].0, Key::KEY_W);

        // No keys released (was empty)
        let released: Vec<_> = previous_keys
            .iter()
            .filter(|(k, _)| !current_keys.iter().any(|(ck, _)| ck == k))
            .collect();
        assert_eq!(released.len(), 0);
    }

    #[test]
    fn test_wasd_state_tracking_direction_to_center() {
        use crate::analog_processor::AnalogProcessor;
        use evdev::Key;

        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();

        // Simulate: North (W) → Center (deadzone)
        // Expected: W released, no presses

        // Event 1: Stick moves North (W active)
        let previous_keys = processor.process_as_wasd(&calibration, 128, 30);
        assert_eq!(previous_keys.len(), 1);
        assert_eq!(previous_keys[0].0, Key::KEY_W);

        // Event 2: Stick returns to center (deadzone)
        let current_keys = processor.process_as_wasd(&calibration, 128, 128);
        assert_eq!(current_keys.len(), 0);

        // W should be released
        let released: Vec<_> = previous_keys
            .iter()
            .filter(|(k, _)| !current_keys.iter().any(|(ck, _)| ck == k))
            .collect();
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].0, Key::KEY_W);

        // No keys pressed
        let newly_pressed: Vec<_> = current_keys
            .iter()
            .filter(|(k, _)| !previous_keys.iter().any(|(pk, _)| pk == k))
            .collect();
        assert_eq!(newly_pressed.len(), 0);
    }

    // Integration test for WASD event loop state tracking
    // This test verifies the complete state machine behavior

    #[test]
    fn test_wasd_event_loop_state_tracking() {
        use crate::analog_processor::AnalogProcessor;
        use evdev::Key;

        // Create processor
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();

        // Simulate event loop state
        let mut previous_keys: Vec<(Key, bool)> = Vec::new();

        // Event 1: Stick moves North (W pressed)
        // Input range: 0-255 where 128 is center, smaller Y = up
        let current_keys = processor.process_as_wasd(&calibration, 128, 30); // X=center, Y=up
        assert_eq!(current_keys.len(), 1);
        assert_eq!(current_keys[0].0, Key::KEY_W);

        // Verify: W should be pressed (new key)
        let newly_pressed: Vec<_> = current_keys
            .iter()
            .filter(|(k, _)| !previous_keys.iter().any(|(pk, _)| pk == k))
            .collect();
        assert_eq!(newly_pressed.len(), 1);
        assert_eq!(newly_pressed[0].0, Key::KEY_W);

        // Update state
        previous_keys = current_keys;

        // Event 2: Stick moves to NorthEast (W+D active)
        let current_keys = processor.process_as_wasd(&calibration, 200, 30); // X=right, Y=up
        assert_eq!(current_keys.len(), 2);

        // Verify: W stays pressed (no new event), D gets pressed (new)
        let released: Vec<_> = previous_keys
            .iter()
            .filter(|(k, _)| !current_keys.iter().any(|(ck, _)| ck == k))
            .collect();
        assert_eq!(released.len(), 0); // No keys released

        let pressed: Vec<_> = current_keys
            .iter()
            .filter(|(k, _)| !previous_keys.iter().any(|(pk, _)| pk == k))
            .collect();
        assert_eq!(pressed.len(), 1);
        assert_eq!(pressed[0].0, Key::KEY_D);

        previous_keys = current_keys;

        // Event 3: Stick moves to East (D active)
        let current_keys = processor.process_as_wasd(&calibration, 200, 128); // X=right, Y=center
        assert_eq!(current_keys.len(), 1);
        assert_eq!(current_keys[0].0, Key::KEY_D);

        // Verify: W released, D stays pressed
        let released: Vec<_> = previous_keys
            .iter()
            .filter(|(k, _)| !current_keys.iter().any(|(ck, _)| ck == k))
            .collect();
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].0, Key::KEY_W);

        let pressed: Vec<_> = current_keys
            .iter()
            .filter(|(k, _)| !previous_keys.iter().any(|(pk, _)| pk == k))
            .collect();
        assert_eq!(pressed.len(), 0); // No new presses

        previous_keys = current_keys;

        // Event 4: Stick returns to center (deadzone)
        let current_keys = processor.process_as_wasd(&calibration, 128, 128); // Center
        assert_eq!(current_keys.len(), 0);

        // Verify: D released
        let released: Vec<_> = previous_keys
            .iter()
            .filter(|(k, _)| !current_keys.iter().any(|(ck, _)| ck == k))
            .collect();
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].0, Key::KEY_D);
    }
}
