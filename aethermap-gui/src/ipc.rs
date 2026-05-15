//! IPC module for the aethermap GUI
//!
//! This module provides a simplified interface for the GUI to communicate
//! with the aethermap daemon using the common IPC client.

use aethermap_common::{
    ipc_client, AnalogCalibrationConfig, DeviceCapabilities, DeviceInfo, LayerConfigInfo,
    LayerMode, LedPattern, LedZone, MacroEntry, Request, Response,
};
use std::collections::HashMap;
use std::path::PathBuf;

/// Simplified IPC client for the GUI
pub struct GuiIpcClient {
    socket_path: PathBuf,
}

impl GuiIpcClient {
    /// Create a new GUI IPC client with the specified socket path
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Connect to the daemon
    pub async fn connect(&self) -> Result<(), String> {
        match ipc_client::is_daemon_running(Some(&self.socket_path)).await {
            true => Ok(()),
            false => Err("Daemon is not running".to_string()),
        }
    }

    /// Get list of available devices
    pub async fn get_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        let request = Request::GetDevices;
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::Devices(devices)) => Ok(devices),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get devices: {}", e)),
        }
    }

    /// Get list of configured macros
    pub async fn list_macros(&self) -> Result<Vec<MacroEntry>, String> {
        let request = Request::ListMacros;
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::Macros(macros)) => Ok(macros),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to list macros: {}", e)),
        }
    }

    /// Start recording a macro for a device
    pub async fn start_recording_macro(
        &self,
        device_path: &str,
        name: &str,
        capture_mouse: bool,
    ) -> Result<(), String> {
        let request = Request::RecordMacro {
            device_path: device_path.to_string(),
            name: name.to_string(),
            capture_mouse,
        };
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::RecordingStarted { .. }) => Ok(()),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to start recording: {}", e)),
        }
    }

    /// Stop recording a macro
    pub async fn stop_recording_macro(&self) -> Result<MacroEntry, String> {
        let request = Request::StopRecording;
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::RecordingStopped { macro_entry }) => Ok(macro_entry),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to stop recording: {}", e)),
        }
    }

    /// Delete a macro by name
    pub async fn delete_macro(&self, name: &str) -> Result<(), String> {
        let request = Request::DeleteMacro {
            name: name.to_string(),
        };
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::Ack) => Ok(()),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to delete macro: {}", e)),
        }
    }

    /// Test a macro execution
    pub async fn test_macro(&self, name: &str) -> Result<(), String> {
        let request = Request::TestMacro {
            name: name.to_string(),
        };
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::Ack) => Ok(()),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to test macro: {}", e)),
        }
    }

    /// Save current macros to a profile
    pub async fn save_profile(&self, name: &str) -> Result<(String, usize), String> {
        let request = Request::SaveProfile {
            name: name.to_string(),
        };
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::ProfileSaved { name, macros_count }) => Ok((name, macros_count)),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to save profile: {}", e)),
        }
    }

    /// Load macros from a profile
    pub async fn load_profile(&self, name: &str) -> Result<(String, usize), String> {
        let request = Request::LoadProfile {
            name: name.to_string(),
        };
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::ProfileLoaded { name, macros_count }) => Ok((name, macros_count)),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to load profile: {}", e)),
        }
    }

    /// Grab a device exclusively for input interception
    pub async fn grab_device(&self, device_path: &str) -> Result<(), String> {
        let request = Request::GrabDevice {
            device_path: device_path.to_string(),
        };
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::Ack) => Ok(()),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to grab device: {}", e)),
        }
    }

    /// Release exclusive access to a device
    pub async fn ungrab_device(&self, device_path: &str) -> Result<(), String> {
        let request = Request::UngrabDevice {
            device_path: device_path.to_string(),
        };
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::Ack) => Ok(()),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to ungrab device: {}", e)),
        }
    }

    /// Get available profiles for a specific device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format (e.g., "1532:0220")
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - List of available profile names
    /// * `Err(String)` - IPC communication error
    pub async fn get_device_profiles(&self, device_id: String) -> Result<Vec<String>, String> {
        let request = Request::GetDeviceProfiles { device_id };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::DeviceProfiles { profiles, .. }) => Ok(profiles),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get device profiles: {}", e)),
        }
    }

    /// Activate a remap profile for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    /// * `profile_name` - Name of the profile to activate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Profile activated successfully
    /// * `Err(String)` - IPC communication error
    pub async fn activate_profile(
        &self,
        device_id: String,
        profile_name: String,
    ) -> Result<(), String> {
        let request = Request::ActivateProfile {
            device_id,
            profile_name,
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::ProfileActivated { .. }) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to activate profile: {}", e)),
        }
    }

    /// Deactivate the current remap profile for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Profile deactivated successfully
    /// * `Err(String)` - IPC communication error
    pub async fn deactivate_profile(&self, device_id: String) -> Result<(), String> {
        let request = Request::DeactivateProfile { device_id };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::ProfileDeactivated { .. }) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to deactivate profile: {}", e)),
        }
    }

    /// Get the currently active profile for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    ///
    /// # Returns
    ///
    /// * `Ok(Option<String>)` - Active profile name or None
    /// * `Err(String)` - IPC communication error
    pub async fn get_active_profile(&self, device_id: String) -> Result<Option<String>, String> {
        let request = Request::GetActiveProfile { device_id };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::ActiveProfile { profile_name, .. }) => Ok(profile_name),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get active profile: {}", e)),
        }
    }

    /// Get active remaps for a device
    ///
    /// # Arguments
    ///
    /// * `device_path` - Full path to the device (e.g., "/dev/input/event0")
    ///
    /// # Returns
    ///
    /// * `Ok(Option<(String, Vec<RemapEntry>)>)` - Active profile name and remap entries, or None if no profile active
    /// * `Err(String)` - IPC communication error
    pub async fn get_active_remaps(
        &self,
        device_path: &str,
    ) -> Result<Option<(String, Vec<aethermap_common::RemapEntry>)>, String> {
        let request = Request::GetActiveRemaps {
            device_path: device_path.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::ActiveRemaps {
                profile_name,
                remaps,
                ..
            }) => {
                if let Some(name) = profile_name {
                    Ok(Some((name, remaps)))
                } else {
                    Ok(None)
                }
            }
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get active remaps: {}", e)),
        }
    }

    /// List available remap profiles for a device
    ///
    /// # Arguments
    ///
    /// * `device_path` - Full path to the device
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<aethermap_common::RemapProfileInfo>)` - List of available profile info
    /// * `Err(String)` - IPC communication error
    pub async fn list_remap_profiles(
        &self,
        device_path: &str,
    ) -> Result<Vec<aethermap_common::RemapProfileInfo>, String> {
        let request = Request::ListRemapProfiles {
            device_path: device_path.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::RemapProfiles { profiles, .. }) => Ok(profiles),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to list remap profiles: {}", e)),
        }
    }

    /// Activate a remap profile for a device
    ///
    /// # Arguments
    ///
    /// * `device_path` - Full path to the device
    /// * `profile_name` - Name of the profile to activate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Profile activated successfully
    /// * `Err(String)` - IPC communication error
    pub async fn activate_remap_profile(
        &self,
        device_path: &str,
        profile_name: &str,
    ) -> Result<(), String> {
        let request = Request::ActivateRemapProfile {
            device_path: device_path.to_string(),
            profile_name: profile_name.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::RemapProfileActivated { .. }) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to activate remap profile: {}", e)),
        }
    }

    /// Deactivate the current remap profile for a device
    ///
    /// # Arguments
    ///
    /// * `device_path` - Full path to the device
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Profile deactivated successfully
    /// * `Err(String)` - IPC communication error
    pub async fn deactivate_remap_profile(&self, device_path: &str) -> Result<(), String> {
        let request = Request::DeactivateRemapProfile {
            device_path: device_path.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::RemapProfileDeactivated { .. }) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to deactivate remap profile: {}", e)),
        }
    }

    /// Get device capabilities and features
    ///
    /// # Arguments
    ///
    /// * `device_path` - Full path to the device
    ///
    /// # Returns
    ///
    /// * `Ok(DeviceCapabilities)` - Device capabilities including button count, hat switch, analog stick
    /// * `Err(String)` - IPC communication error
    pub async fn get_device_capabilities(
        &self,
        device_path: &str,
    ) -> Result<DeviceCapabilities, String> {
        let request = Request::GetDeviceCapabilities {
            device_path: device_path.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::DeviceCapabilities { capabilities, .. }) => Ok(capabilities),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get device capabilities: {}", e)),
        }
    }

    /// Get the currently active layer for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format (e.g., "1532:0220")
    ///
    /// # Returns
    ///
    /// * `Ok(Option<usize>)` - Active layer ID (Some) or None if no layer active
    /// * `Err(String)` - IPC communication error
    pub async fn get_active_layer(&self, device_id: &str) -> Result<Option<usize>, String> {
        let request = Request::GetActiveLayer {
            device_id: device_id.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::ActiveLayer { layer_id, .. }) => Ok(Some(layer_id)),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get active layer: {}", e)),
        }
    }

    /// Set layer configuration for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    /// * `layer_id` - Layer ID to configure
    /// * `name` - Human-readable layer name
    /// * `mode` - Layer activation mode (Hold or Toggle)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Layer configuration updated successfully
    /// * `Err(String)` - IPC communication error
    pub async fn set_layer_config(
        &self,
        device_id: &str,
        layer_id: usize,
        name: String,
        mode: LayerMode,
    ) -> Result<(), String> {
        let config = LayerConfigInfo {
            layer_id,
            name: name.clone(),
            mode,
            remap_count: 0,         // Remaps are managed separately via RemapEngine
            led_color: (0, 0, 255), // Default blue - TODO: allow GUI configuration
            led_zone: None,         // Default zone - TODO: allow GUI configuration
        };

        let request = Request::SetLayerConfig {
            device_id: device_id.to_string(),
            layer_id,
            config,
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::LayerConfigured { .. }) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to set layer config: {}", e)),
        }
    }

    /// Activate a layer for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    /// * `layer_id` - Layer ID to activate
    /// * `mode` - Layer activation mode (Hold or Toggle)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Layer activated successfully
    /// * `Err(String)` - IPC communication error
    pub async fn activate_layer(
        &self,
        device_id: &str,
        layer_id: usize,
        mode: LayerMode,
    ) -> Result<(), String> {
        let request = Request::ActivateLayer {
            device_id: device_id.to_string(),
            layer_id,
            mode,
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::LayerConfigured { .. }) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to activate layer: {}", e)),
        }
    }

    /// List all configured layers for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LayerConfigInfo>)` - List of layer configurations
    /// * `Err(String)` - IPC communication error
    pub async fn list_layers(&self, device_id: &str) -> Result<Vec<LayerConfigInfo>, String> {
        let request = Request::ListLayers {
            device_id: device_id.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::LayerList { layers, .. }) => Ok(layers),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to list layers: {}", e)),
        }
    }

    /// Set D-pad emulation mode for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    /// * `mode` - D-pad mode: "disabled", "eight_way", or "four_way"
    ///
    /// # Returns
    ///
    /// * `Ok(())` - D-pad mode set successfully
    /// * `Err(String)` - IPC communication error
    pub async fn set_analog_dpad_mode(&self, device_id: &str, mode: &str) -> Result<(), String> {
        let request = Request::SetAnalogDpadMode {
            device_id: device_id.to_string(),
            mode: mode.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::AnalogDpadModeSet { .. }) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to set D-pad mode: {}", e)),
        }
    }

    /// Get D-pad emulation mode for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - D-pad mode: "disabled", "eight_way", or "four_way"
    /// * `Err(String)` - IPC communication error
    pub async fn get_analog_dpad_mode(&self, device_id: &str) -> Result<String, String> {
        let request = Request::GetAnalogDpadMode {
            device_id: device_id.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::AnalogDpadMode { mode, .. }) => Ok(mode),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get D-pad mode: {}", e)),
        }
    }

    /// Set per-axis analog deadzone for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    /// * `x_percentage` - X-axis deadzone percentage (0-100)
    /// * `y_percentage` - Y-axis deadzone percentage (0-100)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Per-axis deadzone set successfully
    /// * `Err(String)` - IPC communication error
    pub async fn set_analog_deadzone_xy(
        &self,
        device_id: &str,
        x_percentage: u8,
        y_percentage: u8,
    ) -> Result<(), String> {
        let request = Request::SetAnalogDeadzoneXY {
            device_id: device_id.to_string(),
            x_percentage,
            y_percentage,
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::AnalogDeadzoneXYSet { .. }) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to set per-axis deadzone: {}", e)),
        }
    }

    /// Get per-axis analog deadzone for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    ///
    /// # Returns
    ///
    /// * `Ok((u8, u8))` - X and Y deadzone percentages (0-100 each)
    /// * `Err(String)` - IPC communication error
    pub async fn get_analog_deadzone_xy(&self, device_id: &str) -> Result<(u8, u8), String> {
        let request = Request::GetAnalogDeadzoneXY {
            device_id: device_id.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::AnalogDeadzoneXY {
                x_percentage,
                y_percentage,
                ..
            }) => Ok((x_percentage, y_percentage)),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get per-axis deadzone: {}", e)),
        }
    }

    /// Set per-axis outer deadzone (max clamp) for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    /// * `x_percentage` - X-axis outer deadzone percentage (0-100)
    /// * `y_percentage` - Y-axis outer deadzone percentage (0-100)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Per-axis outer deadzone set successfully
    /// * `Err(String)` - IPC communication error
    pub async fn set_analog_outer_deadzone_xy(
        &self,
        device_id: &str,
        x_percentage: u8,
        y_percentage: u8,
    ) -> Result<(), String> {
        let request = Request::SetAnalogOuterDeadzoneXY {
            device_id: device_id.to_string(),
            x_percentage,
            y_percentage,
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::AnalogOuterDeadzoneXYSet { .. }) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to set per-axis outer deadzone: {}", e)),
        }
    }

    /// Get per-axis outer deadzone for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    ///
    /// # Returns
    ///
    /// * `Ok((u8, u8))` - X and Y outer deadzone percentages (0-100 each)
    /// * `Err(String)` - IPC communication error
    pub async fn get_analog_outer_deadzone_xy(&self, device_id: &str) -> Result<(u8, u8), String> {
        let request = Request::GetAnalogOuterDeadzoneXY {
            device_id: device_id.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::AnalogOuterDeadzoneXY {
                x_percentage,
                y_percentage,
                ..
            }) => Ok((x_percentage, y_percentage)),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get per-axis outer deadzone: {}", e)),
        }
    }

    /// Set LED color for a specific zone
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    /// * `zone` - LED zone to configure (Logo, Keys, Thumbstick, All, Global)
    /// * `red` - Red component (0-255)
    /// * `green` - Green component (0-255)
    /// * `blue` - Blue component (0-255)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Color set successfully
    /// * `Err(String)` - IPC communication error
    pub async fn set_led_color(
        &self,
        device_id: &str,
        zone: LedZone,
        red: u8,
        green: u8,
        blue: u8,
    ) -> Result<(), String> {
        let request = Request::SetLedColor {
            device_id: device_id.to_string(),
            zone,
            red,
            green,
            blue,
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::LedColorSet { .. }) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to set LED color: {}", e)),
        }
    }

    /// Get LED color for a specific zone
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    /// * `zone` - LED zone to query
    ///
    /// # Returns
    ///
    /// * `Ok(Option<(u8, u8, u8)>)` - RGB color tuple if set, None if not set
    /// * `Err(String)` - IPC communication error
    pub async fn get_led_color(
        &self,
        device_id: &str,
        zone: LedZone,
    ) -> Result<Option<(u8, u8, u8)>, String> {
        let request = Request::GetLedColor {
            device_id: device_id.to_string(),
            zone,
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::LedColor { color, .. }) => Ok(color),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get LED color: {}", e)),
        }
    }

    /// Get all LED colors for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<LedZone, (u8, u8, u8)>)` - Map of zones to RGB colors
    /// * `Err(String)` - IPC communication error
    pub async fn get_all_led_colors(
        &self,
        device_id: &str,
    ) -> Result<HashMap<LedZone, (u8, u8, u8)>, String> {
        let request = Request::GetAllLedColors {
            device_id: device_id.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::AllLedColors { colors, .. }) => Ok(colors),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get all LED colors: {}", e)),
        }
    }

    /// Set LED brightness for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    /// * `zone` - LED zone (None = global brightness)
    /// * `brightness` - Brightness percentage (0-100)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Brightness set successfully
    /// * `Err(String)` - IPC communication error
    pub async fn set_led_brightness(
        &self,
        device_id: &str,
        zone: Option<LedZone>,
        brightness: u8,
    ) -> Result<(), String> {
        let request = Request::SetLedBrightness {
            device_id: device_id.to_string(),
            zone,
            brightness,
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::LedBrightnessSet { .. }) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to set LED brightness: {}", e)),
        }
    }

    /// Get LED brightness for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    /// * `zone` - LED zone (None = global brightness)
    ///
    /// # Returns
    ///
    /// * `Ok(u8)` - Brightness percentage (0-100)
    /// * `Err(String)` - IPC communication error
    pub async fn get_led_brightness(
        &self,
        device_id: &str,
        zone: Option<LedZone>,
    ) -> Result<u8, String> {
        let request = Request::GetLedBrightness {
            device_id: device_id.to_string(),
            zone,
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::LedBrightness { brightness, .. }) => Ok(brightness),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get LED brightness: {}", e)),
        }
    }

    /// Set LED pattern for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    /// * `pattern` - LED pattern (Static, Breathing, Rainbow, RainbowWave)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Pattern set successfully
    /// * `Err(String)` - IPC communication error
    pub async fn set_led_pattern(
        &self,
        device_id: &str,
        pattern: LedPattern,
    ) -> Result<(), String> {
        let request = Request::SetLedPattern {
            device_id: device_id.to_string(),
            pattern,
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::LedPatternSet { .. }) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to set LED pattern: {}", e)),
        }
    }

    /// Get LED pattern for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier in vendor:product format
    ///
    /// # Returns
    ///
    /// * `Ok(LedPattern)` - Current LED pattern
    /// * `Err(String)` - IPC communication error
    pub async fn get_led_pattern(&self, device_id: &str) -> Result<LedPattern, String> {
        let request = Request::GetLedPattern {
            device_id: device_id.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::LedPattern { pattern, .. }) => Ok(pattern),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get LED pattern: {}", e)),
        }
    }

    /// Send focus change event to daemon for auto-profile switching
    ///
    /// # Arguments
    ///
    /// * `app_id` - Application identifier (e.g., "org.alacritty", "firefox")
    /// * `window_title` - Optional window title (may be empty on some compositors)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Focus event acknowledged by daemon
    /// * `Err(String)` - IPC communication error
    pub async fn send_focus_change(
        &self,
        app_id: String,
        window_title: Option<String>,
    ) -> Result<(), String> {
        let request = Request::FocusChanged {
            app_id,
            window_title,
        };
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::FocusChangedAck { .. }) => Ok(()),
            Ok(Response::Error(e)) => Err(e),
            Ok(other) => Err(format!("Unexpected response: {:?}", other)),
            Err(e) => Err(format!("Failed to send focus change: {}", e)),
        }
    }

    /// Get analog calibration for a device and layer
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (e.g., "32b6:12f7")
    /// * `layer_id` - Layer ID (0=base, 1, 2, ...)
    ///
    /// # Returns
    ///
    /// * `Ok(AnalogCalibrationConfig)` - Calibration settings
    /// * `Err(String)` - IPC communication error
    pub async fn get_analog_calibration(
        &self,
        device_id: &str,
        layer_id: usize,
    ) -> Result<AnalogCalibrationConfig, String> {
        let request = Request::GetAnalogCalibration {
            device_id: device_id.to_string(),
            layer_id,
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::AnalogCalibration {
                calibration: Some(cal),
                ..
            }) => Ok(cal),
            Ok(Response::AnalogCalibration {
                calibration: None, ..
            }) => {
                // Return default config
                Ok(AnalogCalibrationConfig::default())
            }
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get analog calibration: {}", e)),
        }
    }

    /// Set analog calibration for a device and layer
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (e.g., "32b6:12f7")
    /// * `layer_id` - Layer ID (0=base, 1, 2, ...)
    /// * `calibration` - New calibration settings
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Calibration updated successfully
    /// * `Err(String)` - IPC communication or validation error
    pub async fn set_analog_calibration(
        &self,
        device_id: &str,
        layer_id: usize,
        calibration: AnalogCalibrationConfig,
    ) -> Result<(), String> {
        let request = Request::SetAnalogCalibration {
            device_id: device_id.to_string(),
            layer_id,
            calibration,
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::AnalogCalibrationAck) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to set analog calibration: {}", e)),
        }
    }

    /// Subscribe to real-time analog input updates for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Subscription successful
    /// * `Err(String)` - IPC communication error
    pub async fn subscribe_analog_input(&self, device_id: &str) -> Result<(), String> {
        let request = Request::SubscribeAnalogInput {
            device_id: device_id.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::AnalogInputSubscribed) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to subscribe to analog input: {}", e)),
        }
    }

    /// Unsubscribe from analog input updates
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Unsubscription successful
    /// * `Err(String)` - IPC communication error
    pub async fn unsubscribe_analog_input(&self, device_id: &str) -> Result<(), String> {
        let request = Request::UnsubscribeAnalogInput {
            device_id: device_id.to_string(),
        };

        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::Ack) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to unsubscribe from analog input: {}", e)),
        }
    }

    /// Get global macro settings
    pub async fn get_macro_settings(&self) -> Result<aethermap_common::MacroSettings, String> {
        let request = Request::GetMacroSettings;
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::MacroSettings(settings)) => Ok(settings),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to get macro settings: {}", e)),
        }
    }

    /// Set global macro settings
    pub async fn set_macro_settings(
        &self,
        settings: aethermap_common::MacroSettings,
    ) -> Result<(), String> {
        let request = Request::SetMacroSettings(settings);
        match ipc_client::send_to_path(&request, &self.socket_path).await {
            Ok(Response::Ack) => Ok(()),
            Ok(Response::Error(msg)) => Err(msg),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(format!("Failed to set macro settings: {}", e)),
        }
    }
}

/// Type alias for the IPC client used in the GUI
pub type IpcClient = GuiIpcClient;
