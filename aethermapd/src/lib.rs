//! Aethermap Daemon Library
//!
//! This library provides the core functionality for the aethermap daemon:
//! - Device discovery and management
//! - Macro recording and playback
//! - Input injection via uinput
//! - IPC communication
//! - Security management
//! - Layer management for multi-layer remapping

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::RwLock;

pub mod analog_calibration;
pub mod analog_processor;
pub mod auto_profile_switcher;
pub mod config;
pub mod device;
pub mod gamepad_device;
pub mod global_hotkey_manager;
pub mod hotplug;
pub mod injector;
pub mod ipc;
pub mod key_parser;
pub mod layer_manager;
pub mod led_controller;
pub mod macro_engine;
pub mod remap_engine;
pub mod security;

// Re-export common types
pub use aethermap_common::{DeviceInfo, MacroEntry, Profile};
// Re-export config types
pub use config::{AutoSwitchRule, RemapConfigError, RemapEntry};
// Re-export remap types
pub use remap_engine::{RemapProfile, RemapTable};
// Re-export hotplug types
pub use hotplug::{DeviceEvent, DeviceMonitor};
// Re-export device types
pub use device::{DeviceProfileInfo, GrabbedDevice};
// Re-export layer manager types
pub use layer_manager::{DeviceLayerState, LayerConfig, LayerManager, LayerMode};
// Re-export analog calibration types
pub use analog_calibration::{AnalogCalibration, DeadzoneShape, SensitivityCurve};
// Re-export analog processor types
pub use analog_processor::{AnalogProcessor, DeviceAnalogConfig, ResponseCurve};
// Re-export LED controller types
pub use led_controller::{DeviceLedState, LedController, LedError, LedState, LedZone};
// Re-export auto profile switcher types
pub use auto_profile_switcher::AutoProfileSwitcher;
// Re-export global hotkey manager types
pub use global_hotkey_manager::GlobalHotkeyManager;
// Re-export gamepad device types
pub use gamepad_device::{GamepadAxis, GamepadVirtualDevice};

/// DaemonState holds the shared state of the daemon
pub struct DaemonState {
    pub start_time: Instant,
    pub devices: Arc<Mutex<Vec<DeviceInfo>>>,
    pub macros: Arc<Mutex<HashMap<String, MacroEntry>>>,
    pub profiles: Arc<Mutex<HashMap<String, Profile>>>,
    pub macro_engine: Option<Arc<macro_engine::MacroEngine>>,
    pub remap_engine: Option<Arc<remap_engine::RemapEngine>>,
    pub device_manager: Option<Arc<RwLock<device::DeviceManager>>>,
    pub layer_manager: Arc<RwLock<LayerManager>>,
    pub analog_processor: Option<Arc<analog_processor::AnalogProcessor>>,
    pub led_controller: Option<Arc<led_controller::LedController>>,
    /// Per-device LED state (persists across hotplug)
    ///
    /// Maps device_id (vendor:product format) to the device's LED configuration.
    /// This state survives device disconnection and is restored on reconnection.
    pub led_state: Arc<RwLock<HashMap<String, DeviceLedState>>>,
    /// Global hotkey manager for profile switching
    ///
    /// Tracks modifier key states and checks for hotkey combinations.
    /// Hotkeys are loaded from device_profiles.yaml during daemon startup.
    pub hotkey_manager: Option<Arc<tokio::sync::Mutex<global_hotkey_manager::GlobalHotkeyManager>>>,
    /// Analog input subscribers per device
    ///
    /// Maps device_id (vendor:product format) to a list of response senders.
    /// Subscribers receive real-time analog input updates via IPC.
    pub analog_subscribers: Arc<
        RwLock<
            HashMap<String, Vec<tokio::sync::mpsc::UnboundedSender<aethermap_common::Response>>>,
        >,
    >,
    pub active_recording: Option<(String, String)>, // (name, device_path)
}

impl Default for DaemonState {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonState {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            devices: Arc::new(Mutex::new(Vec::new())),
            macros: Arc::new(Mutex::new(HashMap::new())),
            profiles: Arc::new(Mutex::new(HashMap::new())),
            macro_engine: None,
            remap_engine: None,
            device_manager: None,
            layer_manager: Arc::new(RwLock::new(LayerManager::new(None))),
            analog_processor: None,
            led_controller: None,
            led_state: Arc::new(RwLock::new(HashMap::new())),
            hotkey_manager: None,
            analog_subscribers: Arc::new(RwLock::new(HashMap::new())),
            active_recording: None,
        }
    }

    /// Set LED controller and update LayerManager with LED support
    ///
    /// This method should be called after LED controller initialization to enable
    /// layer-aware LED feedback. The LayerManager is updated to use the LED controller
    /// for visual feedback when layers change.
    ///
    /// # Arguments
    ///
    /// * `led_controller` - Optional LED controller (None if no LED support)
    pub async fn set_led_controller(
        &mut self,
        led_controller: Option<Arc<led_controller::LedController>>,
    ) {
        // Update LayerManager with LED controller
        let mut layer_manager = self.layer_manager.write().await;
        *layer_manager = LayerManager::new(led_controller.clone());

        // Store LED controller in state for IPC access
        self.led_controller = led_controller;

        tracing::info!("LED controller configured for layer-aware feedback");
    }

    /// Initialize LED state entry for a device
    ///
    /// Creates a default LED state entry for the device if one doesn't exist.
    /// This should be called when a device is first connected or grabbed.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    pub async fn initialize_led_state(&self, device_id: &str) {
        let mut led_state = self.led_state.write().await;
        led_state
            .entry(device_id.to_string())
            .or_insert_with(DeviceLedState::default);
    }
}
