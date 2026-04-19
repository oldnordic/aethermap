use aethermap_common::{tracing, serialize, deserialize, Request, Response, RemapProfileInfo, RemapEntry, DeviceCapabilities, LayerConfigInfo, LayerMode as CommonLayerMode, LedZone, LedPattern as CommonLedPattern, AnalogCalibrationConfig, AnalogMode as CommonAnalogMode, CameraOutputMode as CommonCameraOutputMode};
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::RwLock;
use tokio::task;
use tracing::{debug, error, info, warn};

use crate::macro_engine;
use crate::config;
use crate::injector;
use crate::security;
use crate::layer_manager::{LayerConfig, LayerMode};
use crate::led_controller::LedZone as InternalLedZone;
use crate::auto_profile_switcher::AutoProfileSwitcher;
use crate::analog_calibration::{AnalogCalibration, DeadzoneShape, SensitivityCurve};
use crate::analog_processor::{AnalogMode, CameraOutputMode};

/// Convert IPC config to internal AnalogCalibration
fn config_to_calibration(config: AnalogCalibrationConfig) -> Result<AnalogCalibration, String> {
    // Validate deadzone range
    if config.deadzone < 0.0 || config.deadzone > 1.0 {
        return Err(format!(
            "Deadzone must be between 0.0 and 1.0, got {}",
            config.deadzone
        ));
    }

    // Validate sensitivity multiplier
    if config.sensitivity_multiplier < 0.1 || config.sensitivity_multiplier > 5.0 {
        return Err(format!(
            "Sensitivity multiplier must be between 0.1 and 5.0, got {}",
            config.sensitivity_multiplier
        ));
    }

    // Validate range
    if config.range_min >= config.range_max {
        return Err(format!(
            "Range min ({}) must be less than range max ({})",
            config.range_min, config.range_max
        ));
    }

    let deadzone_shape = match config.deadzone_shape.as_str() {
        "circular" => DeadzoneShape::Circular,
        "square" => DeadzoneShape::Square,
        _ => {
            return Err(format!(
                "Invalid deadzone shape: {}, expected 'circular' or 'square'",
                config.deadzone_shape
            ))
        }
    };

    let sensitivity = match config.sensitivity.as_str() {
        "linear" => SensitivityCurve::Linear,
        "quadratic" => SensitivityCurve::Quadratic,
        "exponential" => SensitivityCurve::Exponential {
            exponent: config.exponent,
        },
        _ => {
            return Err(format!(
                "Invalid sensitivity curve: {}, expected 'linear', 'quadratic', or 'exponential'",
                config.sensitivity
            ))
        }
    };

    Ok(AnalogCalibration {
        deadzone: config.deadzone,
        deadzone_shape,
        sensitivity,
        sensitivity_multiplier: config.sensitivity_multiplier,
        range_min: config.range_min,
        range_max: config.range_max,
        invert_x: config.invert_x,
        invert_y: config.invert_y,
    })
}

/// Convert internal AnalogCalibration to IPC config
fn calibration_to_config(calibration: &AnalogCalibration) -> AnalogCalibrationConfig {
    let (sensitivity, exponent) = match calibration.sensitivity {
        SensitivityCurve::Linear => ("linear".to_string(), 2.0),
        SensitivityCurve::Quadratic => ("quadratic".to_string(), 2.0),
        SensitivityCurve::Exponential { exponent } => ("exponential".to_string(), exponent),
    };

    AnalogCalibrationConfig {
        deadzone: calibration.deadzone,
        deadzone_shape: match calibration.deadzone_shape {
            DeadzoneShape::Circular => "circular".to_string(),
            DeadzoneShape::Square => "square".to_string(),
        },
        sensitivity,
        sensitivity_multiplier: calibration.sensitivity_multiplier,
        range_min: calibration.range_min,
        range_max: calibration.range_max,
        invert_x: calibration.invert_x,
        invert_y: calibration.invert_y,
        exponent,
        analog_mode: aethermap_common::AnalogMode::Disabled,
        camera_output_mode: None,
    }
}

/// Convert common AnalogMode to internal AnalogMode
fn common_to_internal_analog_mode(mode: CommonAnalogMode) -> AnalogMode {
    match mode {
        CommonAnalogMode::Disabled => AnalogMode::Disabled,
        CommonAnalogMode::Dpad => AnalogMode::Dpad,
        CommonAnalogMode::Gamepad => AnalogMode::Gamepad,
        CommonAnalogMode::Camera => AnalogMode::Camera,
        CommonAnalogMode::Mouse => AnalogMode::Mouse,
        CommonAnalogMode::Wasd => AnalogMode::Wasd,
    }
}

/// Convert common CameraOutputMode to internal CameraOutputMode
fn common_to_internal_camera_mode(mode: CommonCameraOutputMode) -> CameraOutputMode {
    match mode {
        CommonCameraOutputMode::Scroll => CameraOutputMode::Scroll,
        CommonCameraOutputMode::Keys => CameraOutputMode::Keys,
    }
}

/// IPC server for handling communication with GUI clients
pub struct IpcServer {
    socket_path: String,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    macro_engine: Option<Arc<macro_engine::MacroEngine>>,
    injector: Option<Arc<RwLock<dyn injector::Injector + Send + Sync>>>,
    security_manager: Option<Arc<RwLock<security::SecurityManager>>>,
    auto_profile_switcher: Option<Arc<AutoProfileSwitcher>>,
}


impl IpcServer {
    /// Create a new IPC server with the specified socket path
    pub fn new<P: AsRef<Path>>(socket_path: P) -> Result<Self, std::io::Error> {
        let path = socket_path.as_ref().to_string_lossy().to_string();

        // Remove any existing socket file
        if Path::new(&path).exists() {
            std::fs::remove_file(&path)?;
        }

        Ok(Self {
            socket_path: path,
            shutdown_tx: None,
            macro_engine: None,
            injector: None,
            security_manager: None,
            auto_profile_switcher: None,
        })
    }

    /// Start the IPC server with the provided daemon state
    pub async fn start(&mut self,
            state: Arc<RwLock<crate::DaemonState>>,
            macro_engine: Arc<macro_engine::MacroEngine>,
            injector: Arc<RwLock<dyn injector::Injector + Send + Sync>>,
            config_manager: Arc<config::ConfigManager>,
            security_manager: Arc<RwLock<security::SecurityManager>>,
            auto_profile_switcher: Option<Arc<AutoProfileSwitcher>>
        ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting IPC server at {}", self.socket_path);

        // Store references to macro engine and injector
        self.macro_engine = Some(macro_engine);
        self.injector = Some(injector.clone());
        self.security_manager = Some(security_manager.clone());
        self.auto_profile_switcher = auto_profile_switcher;

        // Create Unix listener
        let listener = UnixListener::bind(&self.socket_path)?;

        // Set socket permissions using the security manager
        {
            let security = security_manager.read().await;
            if let Err(e) = security.set_socket_permissions(&self.socket_path) {
                warn!("Failed to set socket permissions: {}", e);
                // Continue anyway, as the daemon should still work even if permissions aren't ideal
            }
        }

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        // Spawn the main server loop
        let _state_clone = Arc::clone(&state);
        // Clone references before moving into task
        let macro_engine = self.macro_engine.as_ref().unwrap().clone();
        let injector = self.injector.as_ref().unwrap().clone();
        let auto_profile_switcher = self.auto_profile_switcher.clone();

        task::spawn(async move {
            loop {
                tokio::select! {
                    // Accept new connections
                    connection = listener.accept() => {
                        match connection {
                            Ok((stream, _)) => {
                                debug!("New client connected");
                                let state = Arc::clone(&state);
                                let macro_engine = Arc::clone(&macro_engine);
                                let injector = Arc::clone(&injector);
                                let config_manager = Arc::clone(&config_manager);
                                let security_manager = Arc::clone(&security_manager);
                                let switcher = auto_profile_switcher.clone();
                                task::spawn(async move {
                                    if let Err(e) = handle_client(
                                        stream,
                                        state,
                                        macro_engine,
                                        injector,
                                        config_manager,
                                        security_manager,
                                        switcher
                                    ).await {
                                        error!("Error handling client: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                error!("Error accepting connection: {}", e);
                            }
                        }
                    }
                    // Handle shutdown signal
                    _ = &mut shutdown_rx => {
                        info!("Shutting down IPC server");
                        break;
                    }
                }
            }
        });

        // Note: We can't modify self.executing here because we're in a spawned task
        // In a real implementation, we would use a channel or other communication method
        debug!("Macro execution completed");
        Ok(())
    }

    /// Shutdown the IPC server
    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Shutting down IPC server");

        // Send shutdown signal to the server task
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        // Remove the socket file
        if Path::new(&self.socket_path).exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        Ok(())
    }
}

/// Handle a client connection
pub async fn handle_client(
    mut stream: UnixStream,
    state: Arc<RwLock<crate::DaemonState>>,
    macro_engine: Arc<macro_engine::MacroEngine>,
    injector: Arc<RwLock<dyn injector::Injector + Send + Sync>>,
    config_manager: Arc<config::ConfigManager>,
    security_manager: Arc<RwLock<security::SecurityManager>>,
    auto_profile_switcher: Option<Arc<AutoProfileSwitcher>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Set a reasonable timeout for operations
    // Note: set_keepalive is not available on UnixStream in this version of tokio
    // stream.set_keepalive(Some(std::time::Duration::from_secs(30)))?;

    // Read message length first
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let msg_len = u32::from_le_bytes(len_buf) as usize;

    // Validate message length to prevent excessive memory usage
    if msg_len > 1024 * 1024 { // 1MB max message size
        warn!("Received oversized message: {} bytes", msg_len);
        return Err("Message too large".into());
    }

    // Read the actual message
    let mut msg_buf = vec![0u8; msg_len];
    stream.read_exact(&mut msg_buf).await?;

    // Deserialize the request
    let request: Request = deserialize(&msg_buf)?;
    debug!("Received request: {:?}", request);

    // Check authentication if token auth is enabled
    let auth_required = cfg!(feature = "token-auth");

    if auth_required {
        // Handle authentication request
        if let Request::Authenticate { token } = &request {
            let security = security_manager.read().await;
            if security.validate_auth_token(token).await {
                debug!("Authentication successful");
                let response = Response::Authenticated;
                let response_bytes = serialize(&response);

                // Send the response length first
                let len = response_bytes.len() as u32;
                stream.write_all(&len.to_le_bytes()).await?;

                // Send the response
                stream.write_all(&response_bytes).await?;
                stream.flush().await?;

                return Ok(());
            } else {
                debug!("Authentication failed");
                let response = Response::Error("Invalid authentication token".to_string());
                let response_bytes = serialize(&response);

                // Send the response length first
                let len = response_bytes.len() as u32;
                stream.write_all(&len.to_le_bytes()).await?;

                // Send the response
                stream.write_all(&response_bytes).await?;
                stream.flush().await?;

                return Ok(());
            }
        }
        // Allow GenerateToken without authentication
        else if !matches!(request, Request::GenerateToken { .. }) {
            debug!("Authentication required but not provided");
            let response = Response::Error("Authentication required".to_string());
            let response_bytes = serialize(&response);

            // Send the response length first
            let len = response_bytes.len() as u32;
            stream.write_all(&len.to_le_bytes()).await?;

            // Send the response
            stream.write_all(&response_bytes).await?;
            stream.flush().await?;

            return Ok(());
        }
    }

    // Process the request and generate a response
    let response = handle_request(
        request,
        Arc::clone(&state),
        Arc::clone(&macro_engine),
        Arc::clone(&injector),
        Arc::clone(&config_manager),
        Arc::clone(&security_manager),
        auto_profile_switcher
    ).await;
    debug!("Sending response: {:?}", response);

    // Serialize the response
    let response_bytes = serialize(&response);

    // Send the response length first
    let len = response_bytes.len() as u32;
    stream.write_all(&len.to_le_bytes()).await?;

    // Send the response
    stream.write_all(&response_bytes).await?;
    stream.flush().await?;

    Ok(())
}

/// Process a request and generate a response
async fn handle_request(
    request: Request,
    state: Arc<RwLock<crate::DaemonState>>,
    macro_engine: Arc<macro_engine::MacroEngine>,
    _injector: Arc<RwLock<dyn injector::Injector + Send + Sync>>,
    config_manager: Arc<config::ConfigManager>,
    security_manager: Arc<RwLock<security::SecurityManager>>,
    auto_profile_switcher: Option<Arc<AutoProfileSwitcher>>,
) -> Response {
    match request {
        Request::GenerateToken { client_id } => {
            debug!("Generating token for client: {}", client_id);
            let security = security_manager.read().await;
            match security.generate_auth_token().await {
                Ok(token) => Response::Token(token),
                Err(e) => Response::Error(format!("Failed to generate token: {}", e)),
            }
        }
        Request::Authenticate { token } => {
            let security = security_manager.read().await;
            if security.validate_auth_token(&token).await {
                Response::Authenticated
            } else {
                Response::Error("Invalid authentication token".to_string())
            }
        }
        Request::GetDevices => {
            let state = state.read().await;
            let devices = state.devices.lock().unwrap().clone();
            Response::Devices(devices)
        }
        Request::ListMacros => {
            let state = state.read().await;
            let macros = state.macros.lock().unwrap().values().cloned().collect();
            return Response::Macros(macros);
        }
        Request::SetMacro { device_path, macro_entry } => {
            let state = state.write().await;

            // Check if the device exists
            let devices = state.devices.lock().unwrap();
            let device_exists = devices.iter().any(|d| d.path.to_string_lossy() == device_path);
            if !device_exists {
                return Response::Error(format!("Device not found: {}", device_path));
            }

            // Add or update the macro
            let mut macros = state.macros.lock().unwrap();
            macros.insert(macro_entry.name.clone(), macro_entry);

            return Response::Ack;
        }
        Request::DeleteMacro { name } => {
            let state = state.write().await;

            // Find and remove the macro
            let mut macros = state.macros.lock().unwrap();
            let original_len = macros.len();
            macros.remove(&name);

            if macros.len() == original_len {
                return Response::Error(format!("Macro not found: {}", name));
            } else {
                return Response::Ack;
            }
        }
        Request::ReloadConfig => {
            // This would trigger a config reload in a real implementation
            info!("Config reload requested");
            return Response::Ack;
        }
        Request::LedSet { device_path, color } => {
            // This would set LED colors in a real implementation
            info!("LED set request for {}: {:?}", device_path, color);
            return Response::Ack;
        }
        Request::RecordMacro { device_path, name, capture_mouse } => {
            // Start macro recording
            match macro_engine.start_recording(name.clone(), device_path.clone(), capture_mouse).await.map_err(|e| format!("Failed to start recording: {}", e)) {
                Ok(_) => {
                    info!("Macro recording started for {} on {} (capture_mouse: {})", name, device_path, capture_mouse);

                    // Store the recording in the daemon state for access by input event handlers
                    let mut state = state.write().await;
                    state.active_recording = Some((name.clone(), device_path.clone()));

                    return Response::RecordingStarted { device_path, name };
                }
                Err(e) => {
                    error!("Failed to start recording: {}", e);
                    return Response::Error(format!("Failed to start recording: {}", e));
                }
            }
        }
        Request::StopRecording => {
            // Stop macro recording
            match macro_engine.stop_recording().await.map_err(|e| format!("Failed to stop recording: {}", e)) {
                Ok(Some(macro_entry)) => {
                    let macro_name = macro_entry.name.clone();
                    info!("Macro recording stopped: {}", macro_name);

                    // Update the daemon state to remove the active recording
                    let mut state = state.write().await;
                    state.active_recording = None;

                    // Add the macro to the daemon state
                    let mut macros = state.macros.lock().unwrap();
                    macros.insert(macro_entry.name.clone(), macro_entry.clone());
                    drop(macros);

                    return Response::RecordingStopped { macro_entry };
                }
                Ok(None) => {
                    // Update the daemon state to remove the active recording
                    let mut state = state.write().await;
                    state.active_recording = None;

                    return Response::Error("Recording stopped but no macro was created".to_string());
                }
                Err(e) => {
                    error!("Failed to stop recording: {}", e);
                    return Response::Error(format!("Failed to stop recording: {}", e));
                }
            }
        }
        Request::TestMacro { name } => {
            // Test macro execution
            info!("Test macro execution requested: {}", name);
            // Get the macro to execute
            let macro_to_execute = {
                let macros = macro_engine.list_macros().await;
                macros.into_iter().find(|m| m.name == name)
            };

            match macro_to_execute {
                Some(macro_entry) => {
                    // Execute macro using macro engine
                    debug!("Macro execution requested: {}", macro_entry.name);
                    match macro_engine.execute_macro(macro_entry.clone()).await {
                        Ok(_) => {
                            info!("Successfully executed macro: {}", macro_entry.name);
                            Response::Ack
                        }
                        Err(e) => {
                            error!("Failed to execute macro '{}': {}", macro_entry.name, e);
                            Response::Error(format!("Failed to execute macro '{}': {}", macro_entry.name, e))
                        }
                    }
                }
                None => {
                    error!("Macro not found: {}", name);
                    Response::Error(format!("Macro not found: {}", name))
                }
            }
        }
        Request::ExecuteMacro { name } => {
            // Execute macro by name
            info!("Execute macro requested: {}", name);
            // Get the macro to execute
            let macro_to_execute = {
                let macros = macro_engine.list_macros().await;
                macros.into_iter().find(|m| m.name == name)
            };

            match macro_to_execute {
                Some(macro_entry) => {
                    // Use execute_macro method instead of manually executing actions
                    match macro_engine.execute_macro(macro_entry.clone()).await {
                        Ok(_) => {
                            info!("Successfully executed macro: {}", macro_entry.name);
                            Response::Ack
                        }
                        Err(e) => {
                            error!("Failed to execute macro '{}': {}", macro_entry.name, e);
                            Response::Error(format!("Failed to execute macro '{}': {}", macro_entry.name, e))
                        }
                    }
                }
                None => {
                    error!("Macro not found: {}", name);
                    Response::Error(format!("Macro not found: {}", name))
                }
            }
        }
        Request::GetStatus => {
            let state = state.read().await;
            let devices_count = state.devices.lock().unwrap().len();
            let macros_count = state.macros.lock().unwrap().len();
            return Response::Status {
                version: "0.1.0".to_string(),
                uptime_seconds: 0, // Would be calculated in real implementation
                devices_count,
                macros_count,
            };
        }
        Request::SaveProfile { name } => {
            // Save current macros as a profile
            let macros_count = {
                let state_guard = state.read().await;
                let macros = state_guard.macros.lock().unwrap();
                let count = macros.len();
                drop(macros);
                drop(state_guard);
                count
            };

            match config_manager.save_current_macros_as_profile(&name).await {
                Ok(_) => {
                    info!("Profile {} saved", name);
                    return Response::ProfileSaved {
                        name,
                        macros_count,
                    };
                }
                Err(e) => {
                    error!("Failed to save profile: {}", e);
                    return Response::Error(format!("Failed to save profile: {}", e));
                }
            }
        }
        Request::LoadProfile { name } => {
            // Load a profile
            match config_manager.load_profile(&name).await {
                Ok(profile) => {
                    info!("Profile {} loaded", name);
                    return Response::ProfileLoaded {
                        name,
                        macros_count: profile.macros.len()
                    };
                }
                Err(e) => {
                    error!("Failed to load profile: {}", e);
                    return Response::Error(format!("Failed to load profile: {}", e));
                }
            }
        }
        Request::ListProfiles => {
            // List available profiles
            match config_manager.list_profiles().await {
                Ok(profiles) => return Response::Profiles(profiles),
                Err(e) => {
                    error!("Failed to list profiles: {}", e);
                    return Response::Error(format!("Failed to list profiles: {}", e));
                }
            }
        }
        Request::DeleteProfile { name } => {
            // Delete a profile
            match config_manager.delete_profile(&name).await {
                Ok(_) => {
                    info!("Profile {} deleted", name);
                    return Response::Ack;
                }
                Err(e) => {
                    error!("Failed to delete profile: {}", e);
                    return Response::Error(format!("Failed to delete profile: {}", e));
                }
            }
        }
        Request::GrabDevice { device_path } => {
            // Grab a device exclusively for input interception
            let state = state.read().await;
            if let Some(device_manager) = &state.device_manager {
                let mut dm = device_manager.write().await;
                match dm.grab_device(&device_path).await {
                    Ok(_) => {
                        info!("Device {} grabbed successfully", device_path);
                        return Response::Ack;
                    }
                    Err(e) => {
                        error!("Failed to grab device {}: {}", device_path, e);
                        return Response::Error(format!("Failed to grab device: {}", e));
                    }
                }
            } else {
                return Response::Error("Device manager not initialized".to_string());
            }
        }
        Request::UngrabDevice { device_path } => {
            // Release exclusive access to a device
            let state = state.read().await;
            if let Some(device_manager) = &state.device_manager {
                let mut dm = device_manager.write().await;
                match dm.ungrab_device(&device_path).await {
                    Ok(_) => {
                        info!("Device {} ungrabbed successfully", device_path);
                        return Response::Ack;
                    }
                    Err(e) => {
                        error!("Failed to ungrab device {}: {}", device_path, e);
                        return Response::Error(format!("Failed to ungrab device: {}", e));
                    }
                }
            } else {
                return Response::Error("Device manager not initialized".to_string());
            }
        }
        Request::GetDeviceProfiles { device_id } => {
            debug!("GetDeviceProfiles request for device {}", device_id);

            let profiles = config_manager.list_device_profiles(&device_id).await;
            Response::DeviceProfiles {
                device_id: device_id.clone(),
                profiles,
            }
        }
        Request::ActivateProfile { device_id, profile_name } => {
            info!("ActivateProfile request: device={}, profile={}", device_id, profile_name);

            // Set manual override to prevent auto-switching until next focus change
            if let Some(switcher) = &auto_profile_switcher {
                switcher.set_manual_override(&device_id).await;
                debug!("Set manual override for device {} after manual profile activation", device_id);
            }

            // Get the profile from config
            match config_manager.get_device_profile(&device_id, &profile_name).await {
                Some(profile) => {
                    // Clone the device_manager Arc to avoid holding state lock
                    let device_manager_opt = {
                        let state = state.read().await;
                        state.device_manager.clone()
                    };

                    if let Some(device_manager) = device_manager_opt {
                        // Find the device path by device_id using the helper method
                        let device_path = {
                            let dm = device_manager.read().await;
                            dm.get_device_path_by_id(&device_id)
                        };

                        if let Some(path) = device_path {
                            let mut dm = device_manager.write().await;
                            match dm.activate_profile(&path, profile).await {
                                Ok(()) => {
                                    info!("Profile {} activated for device {}", profile_name, device_id);
                                    Response::ProfileActivated {
                                        device_id,
                                        profile_name,
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to activate profile: {}", e);
                                    Response::Error(format!("Failed to activate profile: {}", e))
                                }
                            }
                        } else {
                            // Device not grabbed, but profile is valid - store for when it is grabbed
                            info!("Device {} not currently grabbed, profile will activate on grab", device_id);
                            Response::ProfileActivated {
                                device_id,
                                profile_name,
                            }
                        }
                    } else {
                        Response::Error("Device manager not initialized".to_string())
                    }
                }
                None => {
                    warn!("Profile {} not found for device {}", profile_name, device_id);
                    Response::Error(format!(
                        "Profile '{}' not found for device '{}'. Available profiles: {:?}",
                        profile_name,
                        device_id,
                        config_manager.list_device_profiles(&device_id).await
                    ))
                }
            }
        }
        Request::DeactivateProfile { device_id } => {
            info!("DeactivateProfile request for device {}", device_id);

            // Clone the device_manager Arc to avoid holding state lock
            let device_manager_opt = {
                let state = state.read().await;
                state.device_manager.clone()
            };

            if let Some(device_manager) = device_manager_opt {
                // Find the device path by device_id using the helper method
                let device_path = {
                    let dm = device_manager.read().await;
                    dm.get_device_path_by_id(&device_id)
                };

                if let Some(path) = device_path {
                    let mut dm = device_manager.write().await;
                    match dm.deactivate_profile(&path).await {
                        Ok(()) => {
                            info!("Profile deactivated for device {}", device_id);
                            Response::ProfileDeactivated { device_id }
                        }
                        Err(e) => {
                            error!("Failed to deactivate profile: {}", e);
                            Response::Error(format!("Failed to deactivate profile: {}", e))
                        }
                    }
                } else {
                    Response::ProfileDeactivated { device_id }
                }
            } else {
                Response::Error("Device manager not initialized".to_string())
            }
        }
        Request::GetActiveProfile { device_id } => {
            debug!("GetActiveProfile request for device {}", device_id);

            let state = state.read().await;
            if let Some(device_manager) = &state.device_manager {
                let dm = device_manager.read().await;
                let profile_name = dm.get_active_profile_by_id(&device_id);
                Response::ActiveProfile {
                    device_id,
                    profile_name,
                }
            } else {
                Response::Error("Device manager not initialized".to_string())
            }
        }
        Request::GetActiveRemaps { device_path } => {
            debug!("GetActiveRemaps request for {}", device_path);

            let state_guard = state.read().await;
            let device_manager = match state_guard.device_manager.as_ref() {
                Some(dm) => dm,
                None => return Response::Error("Device manager not initialized".to_string()),
            };

            let dm = device_manager.read().await;

            match dm.get_active_remaps(&device_path).await {
                Ok(Some((profile_name, remappings))) => {
                    // Read the remappings table
                    use crate::remap_engine::RemapTable;
                    let remaps_guard: tokio::sync::RwLockReadGuard<'_, RemapTable> = remappings.read().await;
                    // Convert remappings to Vec<RemapEntry>
                    let entries: Vec<RemapEntry> = remaps_guard.iter()
                        .map(|(from, to)| RemapEntry {
                            from_key: format!("{:?}", from),
                            to_key: format!("{:?}", to),
                        })
                        .collect();
                    drop(remaps_guard);

                    Response::ActiveRemaps {
                        device_path,
                        profile_name: Some(profile_name),
                        remaps: entries,
                    }
                }
                Ok(None) => {
                    Response::ActiveRemaps {
                        device_path,
                        profile_name: None,
                        remaps: vec![],
                    }
                }
                Err(e) => Response::Error(format!("Failed to get active remaps: {}", e)),
            }
        }
        Request::ListRemapProfiles { device_path } => {
            debug!("ListRemapProfiles request for {}", device_path);

            let state_guard = state.read().await;
            let device_manager = match state_guard.device_manager.as_ref() {
                Some(dm) => dm,
                None => return Response::Error("Device manager not initialized".to_string()),
            };

            let dm = device_manager.read().await;

            // Get device info to determine device_id
            let device_info = match dm.get_device_info_from_path(&device_path) {
                Some(info) => info,
                None => {
                    return Response::Error(format!("Device not found: {}", device_path));
                }
            };

            let device_id = crate::device::DeviceManager::format_device_id(
                device_info.vendor_id,
                device_info.product_id
            );

            // Get profiles from DeviceManager's stored profiles
            match dm.get_device_profiles(&device_id) {
                Some(profiles) => {
                    // Need to collect remap counts asynchronously
                    let mut profile_infos = Vec::new();
                    for p in profiles {
                        let remap_count = p.remap_count().await;
                        profile_infos.push(RemapProfileInfo {
                            name: p.name().to_string(),
                            description: None, // Profiles don't have descriptions yet
                            remap_count,
                        });
                    }

                    Response::RemapProfiles {
                        device_path,
                        profiles: profile_infos,
                    }
                }
                None => Response::RemapProfiles {
                    device_path,
                    profiles: vec![],
                },
            }
        }
        Request::ActivateRemapProfile { device_path, profile_name } => {
            info!("ActivateRemapProfile request: device={}, profile={}", device_path, profile_name);

            let state_guard = state.read().await;
            let device_manager = match state_guard.device_manager.as_ref() {
                Some(dm) => dm,
                None => return Response::Error("Device manager not initialized".to_string()),
            };

            let mut dm = device_manager.write().await;

            match dm.activate_profile_by_name(&device_path, &profile_name).await {
                Ok(()) => {
                    info!("Activated profile '{}' for device {}", profile_name, device_path);
                    Response::RemapProfileActivated {
                        device_path,
                        profile_name,
                    }
                }
                Err(e) => {
                    error!("Failed to activate profile: {}", e);
                    Response::Error(format!("Failed to activate profile: {}", e))
                }
            }
        }
        Request::DeactivateRemapProfile { device_path } => {
            info!("DeactivateRemapProfile request for {}", device_path);

            let state_guard = state.read().await;
            let device_manager = match state_guard.device_manager.as_ref() {
                Some(dm) => dm,
                None => return Response::Error("Device manager not initialized".to_string()),
            };

            let mut dm = device_manager.write().await;

            match dm.deactivate_profile(&device_path).await {
                Ok(()) => {
                    info!("Deactivated profile for device {}", device_path);
                    Response::RemapProfileDeactivated {
                        device_path,
                    }
                }
                Err(e) => {
                    error!("Failed to deactivate profile: {}", e);
                    Response::Error(format!("Failed to deactivate profile: {}", e))
                }
            }
        }
        Request::GetDeviceCapabilities { device_path } => {
            info!("GetDeviceCapabilities request for {}", device_path);

            let state_guard = state.read().await;

            // First, try to get device info from DeviceManager
            let device_info = if let Some(device_manager) = &state_guard.device_manager {
                let dm = device_manager.read().await;
                dm.get_device(&device_path)
            } else {
                // Fallback: try from state.devices
                let devices = state_guard.devices.lock().unwrap();
                devices.iter().find(|d| d.path.to_string_lossy() == device_path).cloned()
            };

            match device_info {
                Some(info) => {
                    // Detect capabilities from device
                    let capabilities = detect_device_capabilities(&info);

                    debug!("Device capabilities for {}: analog={}, hat={}, buttons={}",
                           device_path, capabilities.has_analog_stick,
                           capabilities.has_hat_switch, capabilities.joystick_button_count);

                    Response::DeviceCapabilities {
                        device_path,
                        capabilities,
                    }
                }
                None => {
                    warn!("Device not found: {}", device_path);
                    Response::Error(format!("Device not found: {}", device_path))
                }
            }
        }
        Request::GetActiveLayer { device_id } => {
            debug!("GetActiveLayer request for device {}", device_id);

            let state = state.read().await;
            let layer_manager = state.layer_manager.read().await;
            let effective_layer = layer_manager.get_effective_layer(&device_id).await;

            // Get layer name from DeviceLayerState
            let layer_name = if let Some(device_state) = layer_manager.get_device_state(&device_id).await {
                device_state.layer_configs.iter()
                    .find(|c| c.layer_id == effective_layer)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| format!("Layer {}", effective_layer))
            } else {
                format!("Layer {}", effective_layer)
            };

            Response::ActiveLayer {
                device_id,
                layer_id: effective_layer,
                layer_name,
            }
        }
        Request::SetLayerConfig { device_id, layer_id, config } => {
            info!("SetLayerConfig request: device={}, layer={}, name={}", device_id, layer_id, config.name);

            let mut state = state.write().await;
            let mut layer_manager = state.layer_manager.write().await;

            // Convert LayerConfigInfo to LayerConfig for internal use
            let internal_mode = match config.mode {
                CommonLayerMode::Hold => LayerMode::Hold,
                CommonLayerMode::Toggle => LayerMode::Toggle,
            };

            // Convert common LedZone to internal LedZone
            let internal_led_zone = config.led_zone.map(|zone| match zone {
                aethermap_common::LedZone::Side => crate::led_controller::LedZone::Side,
                aethermap_common::LedZone::Logo => crate::led_controller::LedZone::Logo,
                aethermap_common::LedZone::Keys => crate::led_controller::LedZone::Keys,
                aethermap_common::LedZone::Thumbstick => crate::led_controller::LedZone::Thumbstick,
                aethermap_common::LedZone::All => crate::led_controller::LedZone::All,
                aethermap_common::LedZone::Global => crate::led_controller::LedZone::Global,
            });

            // Preserve existing analog_mode when updating layer config
            let existing_mode = layer_manager.get_device_state(&device_id).await
                .and_then(|state| {
                    state.layer_configs.iter()
                        .find(|lc| lc.layer_id == layer_id)
                        .map(|lc| lc.analog_mode)
                });

            let layer_config = LayerConfig {
                layer_id: config.layer_id,
                name: config.name.clone(),
                remaps: std::collections::HashMap::new(), // Remaps set separately via RemapEngine
                mode: internal_mode,
                led_color: config.led_color,
                led_zone: internal_led_zone,
                analog_calibration: None,  // Calibrations set separately via config system
                analog_mode: existing_mode.unwrap_or(AnalogMode::Disabled),  // Preserve existing or default
                camera_output_mode: CameraOutputMode::Scroll,  // Default to scroll for camera mode
            };

            // Apply config to layer_manager
            match layer_manager.set_layer_config(&device_id, layer_id, layer_config).await {
                Ok(_) => Response::LayerConfigured { device_id, layer_id },
                Err(e) => Response::Error(format!("Failed to set layer config: {}", e)),
            }
        }
        Request::ActivateLayer { device_id, layer_id, mode } => {
            info!("ActivateLayer request: device={}, layer={}, mode={:?}", device_id, layer_id, mode);

            let mut state = state.write().await;
            let mut layer_manager = state.layer_manager.write().await;

            let result = match mode {
                CommonLayerMode::Hold => layer_manager.activate_hold_layer(&device_id, layer_id).await,
                CommonLayerMode::Toggle => {
                    layer_manager.toggle_layer(&device_id, layer_id).await.map(|_| ())
                }
            };

            match result {
                Ok(_) => Response::LayerConfigured { device_id, layer_id },
                Err(e) => Response::Error(format!("Failed to activate layer: {}", e)),
            }
        }
        Request::ListLayers { device_id } => {
            debug!("ListLayers request for device {}", device_id);

            let state = state.read().await;
            let layer_manager = state.layer_manager.read().await;

            let layers = layer_manager.get_device_state(&device_id).await
                .map(|s| {
                    s.layer_configs.iter().map(|c| {
                        let ipc_mode = match c.mode {
                            LayerMode::Hold => CommonLayerMode::Hold,
                            LayerMode::Toggle => CommonLayerMode::Toggle,
                        };
                        // Convert internal LedZone to common LedZone
                        let ipc_led_zone = c.led_zone.map(|zone| match zone {
                            crate::led_controller::LedZone::Side => aethermap_common::LedZone::Side,
                            crate::led_controller::LedZone::Logo => aethermap_common::LedZone::Logo,
                            crate::led_controller::LedZone::Keys => aethermap_common::LedZone::Keys,
                            crate::led_controller::LedZone::Thumbstick => aethermap_common::LedZone::Thumbstick,
                            crate::led_controller::LedZone::All => aethermap_common::LedZone::All,
                            crate::led_controller::LedZone::Global => aethermap_common::LedZone::Global,
                            crate::led_controller::LedZone::Unknown(_) => aethermap_common::LedZone::Side, // Default to side for unknown
                        });
                        LayerConfigInfo {
                            layer_id: c.layer_id,
                            name: c.name.clone(),
                            mode: ipc_mode,
                            remap_count: c.remaps.len(),
                            led_color: c.led_color,
                            led_zone: ipc_led_zone,
                        }
                    }).collect()
                })
                .unwrap_or_default();

            Response::LayerList { device_id, layers }
        }
        Request::SetAnalogSensitivity { device_id, sensitivity } => {
            info!("SetAnalogSensitivity request: device={}, sensitivity={:.2}", device_id, sensitivity);

            // Validate sensitivity range
            if sensitivity < 0.1 || sensitivity > 5.0 {
                warn!("Invalid sensitivity value: {:.2} (valid range: 0.1-5.0)", sensitivity);
                return Response::Error(format!(
                    "Invalid sensitivity value: {:.2}. Valid range is 0.1-5.0",
                    sensitivity
                ));
            }

            // Get analog processor from state
            let state = state.read().await;
            if let Some(analog_processor) = &state.analog_processor {
                // Set sensitivity
                analog_processor.set_sensitivity(&device_id, sensitivity).await;

                Response::AnalogSensitivitySet {
                    device_id,
                    sensitivity,
                }
            } else {
                Response::Error("Analog processor not initialized".to_string())
            }
        }
        Request::GetAnalogSensitivity { device_id } => {
            debug!("GetAnalogSensitivity request for device {}", device_id);

            // Get analog processor from state
            let state = state.read().await;
            if let Some(analog_processor) = &state.analog_processor {
                // Get device config
                match analog_processor.get_device_config(&device_id).await {
                    Some(config) => Response::AnalogSensitivity {
                        device_id,
                        sensitivity: config.sensitivity,
                    },
                    None => {
                        // Device not configured - return default
                        Response::AnalogSensitivity {
                            device_id,
                            sensitivity: 1.0,
                        }
                    }
                }
            } else {
                Response::Error("Analog processor not initialized".to_string())
            }
        }
        Request::SetAnalogResponseCurve { device_id, curve } => {
            debug!("SetAnalogResponseCurve request for device {}: {}", device_id, curve);

            // Parse curve string
            let response_curve = match curve.as_str() {
                "linear" => Ok(crate::analog_processor::ResponseCurve::Linear),
                s if s.starts_with("exponential") => {
                    // Parse exponent: "exponential" or "exponential(2.0)"
                    if s == "exponential" {
                        Ok(crate::analog_processor::ResponseCurve::Exponential { exponent: 2.0 })
                    } else {
                        // Extract exponent from "exponential(<number>)"
                        let start = s.find('(').unwrap_or(0);
                        let end = s.find(')').unwrap_or(s.len());
                        if start > 0 && end > start {
                            match s[start + 1..end].parse::<f32>() {
                                Ok(exp) => Ok(crate::analog_processor::ResponseCurve::Exponential { exponent: exp }),
                                Err(_) => Err(format!("Invalid exponent value: {}", &s[start + 1..end])),
                            }
                        } else {
                            Ok(crate::analog_processor::ResponseCurve::Exponential { exponent: 2.0 })
                        }
                    }
                }
                _ => Err(format!("Invalid curve type: {}. Use 'linear' or 'exponential'", curve)),
            };

            let response_curve = match response_curve {
                Ok(curve) => curve,
                Err(e) => return Response::Error(e),
            };

            // Get analog processor from state
            let state = state.read().await;
            if let Some(analog_processor) = &state.analog_processor {
                analog_processor.set_response_curve(&device_id, response_curve).await;
                // Return curve string in response for confirmation
                Response::AnalogResponseCurveSet {
                    device_id,
                    curve,
                }
            } else {
                Response::Error("Analog processor not initialized".to_string())
            }
        }
        Request::GetAnalogResponseCurve { device_id } => {
            debug!("GetAnalogResponseCurve request for device {}", device_id);

            // Get analog processor from state
            let state = state.read().await;
            if let Some(analog_processor) = &state.analog_processor {
                match analog_processor.get_device_config(&device_id).await {
                    Some(config) => {
                        let curve_str = match config.response_curve {
                            crate::analog_processor::ResponseCurve::Linear => "linear".to_string(),
                            crate::analog_processor::ResponseCurve::Exponential { exponent } => {
                                if exponent == 2.0 {
                                    "exponential".to_string()
                                } else {
                                    format!("exponential({})", exponent)
                                }
                            }
                        };
                        Response::AnalogResponseCurve {
                            device_id,
                            curve: curve_str,
                        }
                    }
                    None => Response::Error(format!("Device {} not configured", device_id)),
                }
            } else {
                Response::Error("Analog processor not initialized".to_string())
            }
        }
        Request::SetAnalogDeadzone { device_id, percentage } => {
            info!("SetAnalogDeadzone request: device={}, percentage={}", device_id, percentage);

            // Validate percentage range
            if percentage > 100 {
                warn!("Invalid deadzone percentage: {} (valid range: 0-100)", percentage);
                return Response::Error(format!(
                    "Invalid deadzone percentage: {}. Valid range is 0-100",
                    percentage
                ));
            }

            // Get analog processor from state
            let state = state.read().await;
            if let Some(analog_processor) = &state.analog_processor {
                // Set deadzone percentage
                match analog_processor.set_deadzone_percentage(&device_id, percentage).await {
                    Ok(_) => Response::AnalogDeadzoneSet {
                        device_id,
                        percentage,
                    },
                    Err(e) => Response::Error(format!("Failed to set deadzone: {}", e)),
                }
            } else {
                Response::Error("Analog processor not initialized".to_string())
            }
        }
        Request::GetAnalogDeadzone { device_id } => {
            debug!("GetAnalogDeadzone request for device {}", device_id);

            // Get analog processor from state
            let state = state.read().await;
            if let Some(analog_processor) = &state.analog_processor {
                // Get deadzone percentage
                let percentage = analog_processor.get_deadzone_percentage(&device_id).await;
                Response::AnalogDeadzone {
                    device_id,
                    percentage,
                }
            } else {
                Response::Error("Analog processor not initialized".to_string())
            }
        }
        Request::SetAnalogDeadzoneXY { device_id, x_percentage, y_percentage } => {
            info!("SetAnalogDeadzoneXY request: device={}, x={}%, y={}%", device_id, x_percentage, y_percentage);

            // Validate percentage ranges
            if x_percentage > 100 || y_percentage > 100 {
                warn!("Invalid deadzone percentage: x={}, y={} (valid range: 0-100)", x_percentage, y_percentage);
                return Response::Error(format!(
                    "Invalid deadzone percentage: x={}, y={}. Valid range is 0-100",
                    x_percentage, y_percentage
                ));
            }

            // Get analog processor from state
            let state = state.read().await;
            if let Some(analog_processor) = &state.analog_processor {
                // Set X and Y deadzone percentages
                let result_x = analog_processor.set_deadzone_percentage_x(&device_id, x_percentage).await;
                let result_y = analog_processor.set_deadzone_percentage_y(&device_id, y_percentage).await;

                match (result_x, result_y) {
                    (Ok(_), Ok(_)) => Response::AnalogDeadzoneXYSet {
                        device_id,
                        x_percentage,
                        y_percentage,
                    },
                    (Err(e), _) => Response::Error(format!("Failed to set X-axis deadzone: {}", e)),
                    (_, Err(e)) => Response::Error(format!("Failed to set Y-axis deadzone: {}", e)),
                }
            } else {
                Response::Error("Analog processor not initialized".to_string())
            }
        }
        Request::GetAnalogDeadzoneXY { device_id } => {
            debug!("GetAnalogDeadzoneXY request for device {}", device_id);

            // Get analog processor from state
            let state = state.read().await;
            if let Some(analog_processor) = &state.analog_processor {
                // Get X and Y deadzone percentages
                let x_percentage = analog_processor.get_deadzone_percentage_x(&device_id).await;
                let y_percentage = analog_processor.get_deadzone_percentage_y(&device_id).await;
                Response::AnalogDeadzoneXY {
                    device_id,
                    x_percentage,
                    y_percentage,
                }
            } else {
                Response::Error("Analog processor not initialized".to_string())
            }
        }
        Request::SetAnalogOuterDeadzoneXY { device_id, x_percentage, y_percentage } => {
            info!("SetAnalogOuterDeadzoneXY request: device={}, x={}%, y={}%", device_id, x_percentage, y_percentage);

            // Validate percentage ranges
            if x_percentage > 100 || y_percentage > 100 {
                warn!("Invalid outer deadzone percentage: x={}, y={} (valid range: 0-100)", x_percentage, y_percentage);
                return Response::Error(format!(
                    "Invalid outer deadzone percentage: x={}, y={}. Valid range is 0-100",
                    x_percentage, y_percentage
                ));
            }

            // Get analog processor from state
            let state = state.read().await;
            if let Some(analog_processor) = &state.analog_processor {
                // Convert percentages to raw values
                let x_raw = (x_percentage as u32 * crate::analog_processor::MAX_ABS_VALUE as u32 / 100) as u16;
                let y_raw = (y_percentage as u32 * crate::analog_processor::MAX_ABS_VALUE as u32 / 100) as u16;

                // Set outer deadzones via AnalogProcessor methods
                analog_processor.set_outer_deadzone_x(&device_id, x_raw).await;
                analog_processor.set_outer_deadzone_y(&device_id, y_raw).await;

                info!("Outer deadzones updated: device={}, x={}%, y={}%", device_id, x_percentage, y_percentage);
                Response::AnalogOuterDeadzoneXYSet {
                    device_id,
                    x_percentage,
                    y_percentage,
                }
            } else {
                Response::Error("Analog processor not initialized".to_string())
            }
        }
        Request::GetAnalogOuterDeadzoneXY { device_id } => {
            debug!("GetAnalogOuterDeadzoneXY request for device {}", device_id);

            // Get analog processor from state
            let state = state.read().await;
            if let Some(analog_processor) = &state.analog_processor {
                // Get X and Y outer deadzone percentages
                let x_percentage = analog_processor.get_outer_deadzone_percentage_x(&device_id).await;
                let y_percentage = analog_processor.get_outer_deadzone_percentage_y(&device_id).await;
                Response::AnalogOuterDeadzoneXY {
                    device_id,
                    x_percentage,
                    y_percentage,
                }
            } else {
                Response::Error("Analog processor not initialized".to_string())
            }
        }

        Request::SetAnalogDpadMode { device_id, mode } => {
            info!("SetAnalogDpadMode request: device={}, mode={}", device_id, mode);

            // Parse mode string to DpadMode
            let dpad_mode = match mode.as_str() {
                "disabled" => crate::analog_processor::DpadMode::Disabled,
                "eight_way" => crate::analog_processor::DpadMode::EightWay,
                "four_way" => crate::analog_processor::DpadMode::FourWay,
                _ => {
                    warn!("Invalid D-pad mode: {} (valid: disabled, eight_way, four_way)", mode);
                    return Response::Error(format!(
                        "Invalid D-pad mode: {}. Valid modes are: disabled, eight_way, four_way",
                        mode
                    ));
                }
            };

            // Get analog processor from state
            let state = state.read().await;
            if let Some(analog_processor) = &state.analog_processor {
                // Set D-pad mode
                analog_processor.set_dpad_mode(&device_id, dpad_mode).await;
                Response::AnalogDpadModeSet {
                    device_id,
                    mode,
                }
            } else {
                Response::Error("Analog processor not initialized".to_string())
            }
        }

        Request::GetAnalogDpadMode { device_id } => {
            debug!("GetAnalogDpadMode request for device {}", device_id);

            // Get analog processor from state
            let state = state.read().await;
            if let Some(analog_processor) = &state.analog_processor {
                // Get D-pad mode
                let dpad_mode = analog_processor.get_dpad_mode(&device_id).await;
                let mode_str = match dpad_mode {
                    crate::analog_processor::DpadMode::Disabled => "disabled".to_string(),
                    crate::analog_processor::DpadMode::EightWay => "eight_way".to_string(),
                    crate::analog_processor::DpadMode::FourWay => "four_way".to_string(),
                };
                Response::AnalogDpadMode {
                    device_id,
                    mode: mode_str,
                }
            } else {
                Response::Error("Analog processor not initialized".to_string())
            }
        }

        Request::GetAnalogCalibration { device_id, layer_id } => {
            debug!("GetAnalogCalibration request: device={}, layer={}", device_id, layer_id);

            // Get calibration from config manager
            let calibration = config_manager.get_analog_calibration(&device_id, layer_id).await;
            let config = calibration.as_ref().map(calibration_to_config);

            Response::AnalogCalibration {
                device_id,
                layer_id,
                calibration: config,
            }
        }

        Request::SetAnalogCalibration { device_id, layer_id, calibration } => {
            info!(
                "SetAnalogCalibration request: device={}, layer={}, deadzone={}, sensitivity={}, curve={}, mode={:?}",
                device_id,
                layer_id,
                calibration.deadzone,
                calibration.sensitivity_multiplier,
                calibration.sensitivity,
                calibration.analog_mode
            );

            // Extract mode settings from calibration (convert from common types to internal)
            let layer_mode = common_to_internal_analog_mode(calibration.analog_mode);
            let camera_mode = calibration.camera_output_mode
                .map(common_to_internal_camera_mode)
                .unwrap_or(CameraOutputMode::Scroll);

            // Convert IPC config to internal calibration
            match config_to_calibration(calibration.clone()) {
                Ok(cal) => {
                    // Validate the calibration
                    if let Err(e) = cal.validate() {
                        warn!("Invalid calibration: {}", e);
                        return Response::Error(format!("Invalid calibration: {}", e));
                    }

                    // Update the analog processor's calibration
                    {
                        let state_guard = state.read().await;
                        if let Some(analog_processor) = &state_guard.analog_processor {
                            analog_processor.set_calibration(&device_id, layer_id, cal.clone()).await;
                        } else {
                            return Response::Error("Analog processor not initialized".to_string());
                        }
                    } // Release read lock

                    // Save to config
                    if let Err(e) = config_manager.save_analog_calibration(&device_id, layer_id, cal).await {
                        warn!("Failed to save analog calibration: {}", e);
                        return Response::Error(format!("Failed to save calibration: {}", e));
                    }

                    // Also update analog_mode in layer config
                    let state_guard = state.write().await;
                    let mut layer_manager = state_guard.layer_manager.write().await;

                    // Get existing config or create default
                    let device_state = layer_manager.get_device_state(&device_id).await;
                    let existing_config = device_state.as_ref().and_then(|s| s.get_layer_config(layer_id));

                    let mut layer_config = existing_config.cloned().unwrap_or_else(|| {
                        LayerConfig::new(
                            layer_id,
                            format!("Layer {}", layer_id),
                            LayerMode::Hold,
                        )
                    });

                    // Update mode from calibration
                    layer_config.analog_mode = layer_mode;
                    if layer_mode == AnalogMode::Camera {
                        layer_config.camera_output_mode = camera_mode;
                    }

                    // Apply the updated config
                    match layer_manager.set_layer_config(&device_id, layer_id, layer_config).await {
                        Ok(_) => Response::AnalogCalibrationAck,
                        Err(e) => Response::Error(format!("Failed to set analog mode: {}", e)),
                    }
                }
                Err(e) => Response::Error(format!("Invalid calibration: {}", e)),
            }
        }

        Request::SubscribeAnalogInput { device_id } => {
            debug!("SubscribeAnalogInput request: device={}", device_id);

            // Create channel for this subscription
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            let state_guard = state.write().await;
            let mut subscribers = state_guard.analog_subscribers.write().await;
            subscribers.entry(device_id.clone()).or_default().push(tx);

            Response::AnalogInputSubscribed
        }

        Request::UnsubscribeAnalogInput { device_id } => {
            debug!("UnsubscribeAnalogInput request: device={}", device_id);

            let state_guard = state.write().await;
            let mut subscribers = state_guard.analog_subscribers.write().await;
            subscribers.remove(&device_id);

            Response::Ack
        }

        Request::SetMacroSettings(settings) => {
            info!("SetMacroSettings request: latency_offset_ms={}, jitter_pct={:.2}",
                  settings.latency_offset_ms, settings.jitter_pct);

            // Update in macro engine
            macro_engine.set_macro_settings(settings.clone()).await;

            // Update in config manager for persistence
            {
                let mut config = config_manager.config.write().await;
                config.macro_engine.latency_offset_ms = settings.latency_offset_ms;
                config.macro_engine.jitter_pct = settings.jitter_pct;
            }

            if let Err(e) = config_manager.save_config().await {
                warn!("Failed to save config after updating macro settings: {}", e);
            }

            Response::Ack
        }

        Request::GetMacroSettings => {
            debug!("GetMacroSettings request");
            let settings = macro_engine.get_macro_settings().await;
            Response::MacroSettings(settings)
        }

        Request::SetLedColor { device_id, zone, red, green, blue } => {
            info!("SetLedColor request: device={}, zone={:?}, RGB=({}, {}, {})", device_id, zone, red, green, blue);

            // Validate RGB range (0-255) - enforced by u8 type, but double-check
            // This is redundant since u8 can't exceed 255, but kept for clarity
            let state_guard = state.read().await;
            if let Some(led_controller) = &state_guard.led_controller {
                // Convert IPC LedZone to internal LedZone
                let internal_zone = match zone {
                    LedZone::Side => InternalLedZone::Side,
                    LedZone::Logo => InternalLedZone::Logo,
                    LedZone::Keys => InternalLedZone::Keys,
                    LedZone::Thumbstick => InternalLedZone::Thumbstick,
                    LedZone::All => InternalLedZone::All,
                    LedZone::Global => InternalLedZone::Global,
                };

                match led_controller.set_zone_color(internal_zone, red, green, blue).await {
                    Ok(_) => {
                        // Update DaemonState.led_state for hotplug persistence
                        drop(state_guard); // Release read lock before write
                        let mut state_guard = state.write().await;

                        // Update or create LED state entry for this device
                        let device_led_state = state_guard.led_state.read().await;
                        if let Some(led_state) = device_led_state.get(&device_id) {
                            // Clone and update
                            let mut updated = led_state.clone();
                            updated.zone_colors.insert(internal_zone, (red, green, blue));

                            // Write back
                            drop(device_led_state);
                            let mut device_led_state = state_guard.led_state.write().await;
                            device_led_state.insert(device_id.clone(), updated);
                        } else {
                            // Create new entry
                            let mut new_state = crate::led_controller::DeviceLedState::default();
                            new_state.zone_colors.insert(internal_zone, (red, green, blue));

                            drop(state_guard.led_state.read().await);
                            let mut device_led_state = state_guard.led_state.write().await;
                            device_led_state.insert(device_id.clone(), new_state);
                        }

                        Response::LedColorSet {
                            device_id,
                            zone,
                            color: (red, green, blue),
                        }
                    },
                    Err(e) => Response::Error(format!("Failed to set LED color: {}", e)),
                }
            } else {
                Response::Error("LED controller not available".to_string())
            }
        }
        Request::GetLedColor { device_id, zone } => {
            debug!("GetLedColor request: device={}, zone={:?}", device_id, zone);

            let state = state.read().await;
            if let Some(led_controller) = &state.led_controller {
                // Convert IPC LedZone to internal LedZone
                let internal_zone = match zone {
                    LedZone::Side => InternalLedZone::Side,
                    LedZone::Logo => InternalLedZone::Logo,
                    LedZone::Keys => InternalLedZone::Keys,
                    LedZone::Thumbstick => InternalLedZone::Thumbstick,
                    LedZone::All => InternalLedZone::All,
                    LedZone::Global => InternalLedZone::Global,
                };

                let color = led_controller.get_zone_color(internal_zone).await;
                Response::LedColor { device_id, zone, color }
            } else {
                Response::Error("LED controller not available".to_string())
            }
        }
        Request::GetAllLedColors { device_id } => {
            debug!("GetAllLedColors request: device={}", device_id);

            let state = state.read().await;
            if let Some(led_controller) = &state.led_controller {
                let internal_colors = led_controller.get_all_zone_colors().await;

                // Convert internal LedZone keys to IPC LedZone
                let mut colors = std::collections::HashMap::new();
                for (internal_zone, color) in internal_colors {
                    let ipc_zone = match internal_zone {
                        InternalLedZone::Side => LedZone::Side,
                        InternalLedZone::Logo => LedZone::Logo,
                        InternalLedZone::Keys => LedZone::Keys,
                        InternalLedZone::Thumbstick => LedZone::Thumbstick,
                        InternalLedZone::All => LedZone::All,
                        InternalLedZone::Global => LedZone::Global,
                        InternalLedZone::Unknown(_) => continue, // Skip unknown zones
                    };
                    colors.insert(ipc_zone, color);
                }

                Response::AllLedColors { device_id, colors }
            } else {
                Response::Error("LED controller not available".to_string())
            }
        }
        Request::SetLedBrightness { device_id, zone, brightness } => {
            info!("SetLedBrightness request: device={}, zone={:?}, brightness={}", device_id, zone, brightness);

            // Validate brightness range (0-100)
            if brightness > 100 {
                warn!("Invalid brightness value: {} (valid range: 0-100)", brightness);
                return Response::Error(format!(
                    "Invalid brightness value: {}. Valid range is 0-100",
                    brightness
                ));
            }

            let state_guard = state.read().await;
            if let Some(led_controller) = &state_guard.led_controller {
                // Use set_global_brightness for None zone, set_zone_brightness for specific zone
                let result = match zone {
                    Some(ipc_zone) => {
                        // Convert IPC LedZone to internal LedZone
                        let internal_zone = match ipc_zone {
                            LedZone::Side => InternalLedZone::Side,
                    LedZone::Logo => InternalLedZone::Logo,
                            LedZone::Keys => InternalLedZone::Keys,
                            LedZone::Thumbstick => InternalLedZone::Thumbstick,
                            LedZone::All => InternalLedZone::All,
                            LedZone::Global => InternalLedZone::Global,
                        };
                        led_controller.set_zone_brightness(internal_zone, brightness).await
                    }
                    None => led_controller.set_global_brightness(brightness).await,
                };

                match result {
                    Ok(_) => {
                        // Update DaemonState.led_state for hotplug persistence
                        drop(state_guard); // Release read lock before write
                        let mut state_guard = state.write().await;

                        // Update or create LED state entry for this device
                        let device_led_state = state_guard.led_state.read().await;
                        if let Some(led_state) = device_led_state.get(&device_id) {
                            // Clone and update
                            let mut updated = led_state.clone();

                            // Update brightness based on zone
                            match zone {
                                Some(ipc_zone) => {
                                    let internal_zone = match ipc_zone {
                                        LedZone::Side => InternalLedZone::Side,
                    LedZone::Logo => InternalLedZone::Logo,
                                        LedZone::Keys => InternalLedZone::Keys,
                                        LedZone::Thumbstick => InternalLedZone::Thumbstick,
                                        LedZone::All => InternalLedZone::All,
                                        LedZone::Global => InternalLedZone::Global,
                                    };
                                    updated.zone_brightness.insert(internal_zone, brightness);
                                }
                                None => {
                                    updated.global_brightness = brightness;
                                }
                            }

                            // Write back
                            drop(device_led_state);
                            let mut device_led_state = state_guard.led_state.write().await;
                            device_led_state.insert(device_id.clone(), updated);
                        } else {
                            // Create new entry
                            let mut new_state = crate::led_controller::DeviceLedState::default();

                            match zone {
                                Some(ipc_zone) => {
                                    let internal_zone = match ipc_zone {
                                        LedZone::Side => InternalLedZone::Side,
                    LedZone::Logo => InternalLedZone::Logo,
                                        LedZone::Keys => InternalLedZone::Keys,
                                        LedZone::Thumbstick => InternalLedZone::Thumbstick,
                                        LedZone::All => InternalLedZone::All,
                                        LedZone::Global => InternalLedZone::Global,
                                    };
                                    new_state.zone_brightness.insert(internal_zone, brightness);
                                }
                                None => {
                                    new_state.global_brightness = brightness;
                                }
                            }

                            drop(state_guard.led_state.read().await);
                            let mut device_led_state = state_guard.led_state.write().await;
                            device_led_state.insert(device_id.clone(), new_state);
                        }

                        Response::LedBrightnessSet {
                            device_id,
                            zone,
                            brightness,
                        }
                    },
                    Err(e) => Response::Error(format!("Failed to set LED brightness: {}", e)),
                }
            } else {
                Response::Error("LED controller not available".to_string())
            }
        }
        Request::GetLedBrightness { device_id, zone } => {
            debug!("GetLedBrightness request: device={}, zone={:?}", device_id, zone);

            let state = state.read().await;
            if let Some(led_controller) = &state.led_controller {
                // Get brightness for specific zone or global
                let brightness = match zone {
                    Some(ipc_zone) => {
                        // Convert IPC LedZone to internal LedZone
                        let internal_zone = match ipc_zone {
                            LedZone::Side => InternalLedZone::Side,
                            LedZone::Logo => InternalLedZone::Logo,
                            LedZone::Keys => InternalLedZone::Keys,
                            LedZone::Thumbstick => InternalLedZone::Thumbstick,
                            LedZone::All => InternalLedZone::All,
                            LedZone::Global => InternalLedZone::Global,
                        };
                        led_controller.get_zone_brightness(internal_zone).await
                    }
                    None => led_controller.get_global_brightness().await,
                };

                Response::LedBrightness {
                    device_id,
                    zone,
                    brightness,
                }
            } else {
                Response::Error("LED controller not available".to_string())
            }
        }
        Request::SetLedPattern { device_id, pattern } => {
            info!("SetLedPattern request: device={}, pattern={:?}", device_id, pattern);

            let state = state.read().await;
            if let Some(led_controller) = &state.led_controller {
                // Convert IPC LedPattern to internal LedPattern
                let internal_pattern = match pattern {
                    CommonLedPattern::Static => crate::led_controller::LedPattern::Static,
                    CommonLedPattern::Breathing => crate::led_controller::LedPattern::Breathing,
                    CommonLedPattern::Rainbow => crate::led_controller::LedPattern::Rainbow,
                    CommonLedPattern::RainbowWave => crate::led_controller::LedPattern::RainbowWave,
                };

                match led_controller.set_pattern(internal_pattern).await {
                    Ok(_) => Response::LedPatternSet {
                        device_id,
                        pattern,
                    },
                    Err(e) => Response::Error(format!("Failed to set LED pattern: {}", e)),
                }
            } else {
                Response::Error("LED controller not available".to_string())
            }
        }
        Request::GetLedPattern { device_id } => {
            debug!("GetLedPattern request: device={}", device_id);

            let state = state.read().await;
            if let Some(led_controller) = &state.led_controller {
                let internal_pattern = led_controller.get_pattern().await;
                // Convert internal LedPattern to IPC LedPattern
                let pattern = match internal_pattern {
                    crate::led_controller::LedPattern::Static => CommonLedPattern::Static,
                    crate::led_controller::LedPattern::Breathing => CommonLedPattern::Breathing,
                    crate::led_controller::LedPattern::Rainbow => CommonLedPattern::Rainbow,
                    crate::led_controller::LedPattern::RainbowWave => CommonLedPattern::RainbowWave,
                };

                Response::LedPattern {
                    device_id,
                    pattern,
                }
            } else {
                Response::Error("LED controller not available".to_string())
            }
        }
        Request::FocusChanged { app_id, window_title } => {
            debug!("Focus changed: app_id={}, window_title={:?}", app_id, window_title);

            // Apply focus-based profile switching via AutoProfileSwitcher
            if let Some(switcher) = &auto_profile_switcher {
                if let Err(e) = switcher.apply_focus_change(&app_id, &window_title).await {
                    warn!("Failed to apply focus change: {}", e);
                    Response::Error(format!("Failed to apply focus change: {}", e))
                } else {
                    Response::FocusChangedAck { app_id }
                }
            } else {
                // AutoProfileSwitcher not configured - acknowledge but no switching
                debug!("AutoProfileSwitcher not configured, skipping profile switch");
                Response::FocusChangedAck { app_id }
            }
        }

        Request::RegisterHotkey { device_id, binding } => {
            debug!("RegisterHotkey: device={}, key={:?}, profile={}",
                device_id, binding.key, binding.profile_name);

            // Validate profile exists
            let profiles = config_manager.device_profiles.read().await;
            let profile_exists = profiles.get(&device_id)
                .and_then(|device_profiles| device_profiles.get(&binding.profile_name))
                .is_some();
            drop(profiles);

            if !profile_exists {
                warn!("RegisterHotkey: profile '{}' not found for device {}",
                    binding.profile_name, device_id);
                return Response::Error(format!("Profile '{}' not found", binding.profile_name));
            }

            // Convert from common::HotkeyBinding to config::HotkeyBinding
            let internal_binding = crate::config::HotkeyBinding {
                modifiers: binding.modifiers.clone(),
                key: binding.key.clone(),
                profile_name: binding.profile_name.clone(),
                device_id: binding.device_id.clone(),
                layer_id: binding.layer_id,
            };

            // Add to config
            match config_manager.add_hotkey_binding(&device_id, internal_binding).await {
                Ok(()) => {
                    // Reload hotkey manager
                    let state = state.read().await;
                    if let Some(device_manager) = &state.device_manager {
                        let dm = device_manager.read().await;
                        if let Some(hotkey_manager) = dm.hotkey_manager() {
                            if let Err(e) = hotkey_manager.lock().await.load_bindings().await {
                                warn!("Failed to reload hotkey bindings: {}", e);
                            }
                        }
                    }

                    info!("Registered hotkey: {}+{:?} for device {}",
                        binding.modifiers.join("+"), binding.key, device_id);
                    Response::HotkeyRegistered {
                        device_id,
                        key: binding.key,
                        modifiers: binding.modifiers,
                    }
                }
                Err(e) => {
                    warn!("RegisterHotkey failed: {}", e);
                    Response::Error(format!("Failed to register hotkey: {}", e))
                }
            }
        }

        Request::ListHotkeys { device_id } => {
            debug!("ListHotkeys: device={}", device_id);

            match config_manager.get_hotkey_bindings(&device_id).await {
                Ok(bindings) => {
                    // Convert from config::HotkeyBinding to common::HotkeyBinding
                    let common_bindings: Vec<aethermap_common::HotkeyBinding> = bindings.into_iter()
                        .map(|b| aethermap_common::HotkeyBinding {
                            modifiers: b.modifiers,
                            key: b.key,
                            profile_name: b.profile_name,
                            device_id: b.device_id,
                            layer_id: b.layer_id,
                        })
                        .collect();

                    debug!("Returning {} hotkey bindings for device {}",
                        common_bindings.len(), device_id);
                    Response::HotkeyList {
                        device_id,
                        bindings: common_bindings,
                    }
                }
                Err(e) => {
                    warn!("ListHotkeys failed: {}", e);
                    Response::HotkeyList {
                        device_id,
                        bindings: Vec::new(),
                    }
                }
            }
        }

        Request::RemoveHotkey { device_id, key, modifiers } => {
            debug!("RemoveHotkey: device={}, key={}, modifiers={:?}",
                device_id, key, modifiers);

            match config_manager.remove_hotkey_binding(&device_id, &key, &modifiers).await {
                Ok(()) => {
                    // Reload hotkey manager
                    let state = state.read().await;
                    if let Some(device_manager) = &state.device_manager {
                        let dm = device_manager.read().await;
                        if let Some(hotkey_manager) = dm.hotkey_manager() {
                            if let Err(e) = hotkey_manager.lock().await.load_bindings().await {
                                warn!("Failed to reload hotkey bindings: {}", e);
                            }
                        }
                    }

                    info!("Removed hotkey: {}+{:?} for device {}",
                        modifiers.join("+"), key, device_id);
                    Response::HotkeyRemoved {
                        device_id,
                        key,
                        modifiers,
                    }
                }
                Err(e) => {
                    warn!("RemoveHotkey failed: {}", e);
                    Response::Error(format!("Failed to remove hotkey: {}", e))
                }
            }
        }
        Request::SetAutoSwitchRules { rules } => {
            info!("SetAutoSwitchRules request: {} rules", rules.len());

            // Keep a reference to rules for reloading the switcher
            let rules_count = rules.len();

            // Convert from common::AutoSwitchRule to config::AutoSwitchRule
            let internal_rules: Vec<crate::config::AutoSwitchRule> = rules.into_iter()
                .map(|r| crate::config::AutoSwitchRule {
                    app_id: r.app_id,
                    profile_name: r.profile_name,
                    device_id: r.device_id,
                    layer_id: r.layer_id,
                })
                .collect();

            // Clone for switcher reload
            let switcher_rules = internal_rules.clone();

            match config_manager.set_auto_switch_rules(internal_rules).await {
                Ok(()) => {
                    // Reload auto-profile switcher with new rules
                    if let Some(switcher) = auto_profile_switcher.as_ref() {
                        switcher.reload_rules(switcher_rules).await;
                        info!("Reloaded auto-profile switcher with {} rules", rules_count);
                    }

                    info!("Saved {} auto-switch rules", rules_count);
                    Response::AutoSwitchRulesAck
                }
                Err(e) => {
                    warn!("SetAutoSwitchRules failed: {}", e);
                    Response::Error(format!("Failed to save auto-switch rules: {}", e))
                }
            }
        }

        Request::GetAutoSwitchRules => {
            // Get all auto-switch rules from config
            let rules = config_manager.get_auto_switch_rules().await;

            // Convert internal AutoSwitchRule to IPC AutoSwitchRule
            let ipc_rules = rules
                .into_iter()
                .map(|r| aethermap_common::AutoSwitchRule {
                    app_id: r.app_id,
                    profile_name: r.profile_name,
                    device_id: r.device_id,
                    layer_id: r.layer_id,
                })
                .collect();

            Response::AutoSwitchRules { rules: ipc_rules }
        }
    }
}

/// Broadcast analog input update to all subscribers for a device
///
/// This function sends real-time analog stick position updates to all
/// subscribed GUI clients. It's called from the analog event processing
/// loop when analog input changes.
///
/// # Arguments
///
/// * `state` - The daemon state containing subscriber information
/// * `device_id` - Device identifier (vendor:product format)
/// * `axis_x` - Normalized X axis value (-1.0 to 1.0)
/// * `axis_y` - Normalized Y axis value (-1.0 to 1.0)
///
/// # Note
///
/// This broadcasts the RAW input values before calibration, so the GUI
/// shows actual stick position. The deadzone overlay in visualizer shows
/// how calibration will affect it.
pub async fn broadcast_analog_input(
    state: &Arc<RwLock<crate::DaemonState>>,
    device_id: &str,
    axis_x: f32,
    axis_y: f32,
) {
    use aethermap_common::Response;

    let update = Response::AnalogInputUpdate {
        device_id: device_id.to_string(),
        axis_x,
        axis_y,
    };

    // Get subscribers map and clone the Vec of senders for this device
    let state_guard = state.read().await;
    let subscribers = state_guard.analog_subscribers.read().await;
    if let Some(clients) = subscribers.get(device_id) {
        for sender in clients {
            let _ = sender.send(update.clone());
        }
    }
}

/// Detect device capabilities from DeviceInfo
///
/// This function infers device capabilities based on device type.
/// For more accurate detection, the device would need to be grabbed
/// and queried directly.
///
/// # Arguments
///
/// * `device_info` - The device information from discovery
///
/// # Returns
///
/// DeviceCapabilities struct with detected features
fn detect_device_capabilities(
    device_info: &aethermap_common::DeviceInfo,
) -> DeviceCapabilities {
    use aethermap_common::DeviceType;

    // Infer analog stick from device type
    // Keypads (like Azeron) and gamepads typically have analog sticks
    let has_analog = matches!(device_info.device_type, DeviceType::Keypad | DeviceType::Gamepad);

    // Azeron keypads have hat switches (detected via DeviceType::Keypad)
    let has_hat_switch = matches!(device_info.device_type, DeviceType::Keypad);

    // Button count: Azeron Cybo has 26 joystick buttons
    // For other devices, we use reasonable defaults
    let joystick_button_count = match device_info.device_type {
        DeviceType::Keypad => 26,  // Azeron Cybo
        DeviceType::Gamepad => 12,  // Generic gamepad
        _ => 0,
    };

    // LED zones: defer to Phase 12 (LED Control), empty for now
    let led_zones = Vec::new();

    DeviceCapabilities {
        has_analog_stick: has_analog,
        has_hat_switch,
        joystick_button_count,
        led_zones,
    }
}


/// Get the GID for a group name
#[cfg(target_os = "linux")]
fn get_group_gid(group_name: &str) -> Option<u32> {
    // Simplified implementation for now
    // In a real implementation, this would use libc or nix to resolve group names
    match group_name {
        "root" => Some(0),
        "input" => Some(1001), // Common GID for input group
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DaemonState;
    use aethermap_common::{DeviceInfo, MacroEntry, KeyCombo, Action};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;

    // Helper function to create a test injector or skip the test if permissions are insufficient
    fn create_test_injector() -> Arc<RwLock<dyn injector::Injector + Send + Sync>> {
        match injector::UinputInjector::new() {
            Ok(injector) => Arc::new(RwLock::new(injector)),
            Err(_) => {
                // Skip test if we don't have permission to create injector
                panic!("Test requires root access to create UinputInjector. Run with sudo or set CAP_SYS_ADMIN capability.");
            }
        }
    }

    // Helper function to create a test ConfigManager with temporary paths
    async fn create_test_config_manager() -> Arc<config::ConfigManager> {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        let macros_path = temp_dir.path().join("macros.yaml");
        let cache_path = temp_dir.path().join("macros.bin");
        let profiles_dir = temp_dir.path().join("profiles");
        let remaps_path = temp_dir.path().join("remaps.yaml");
        let device_profiles_path = temp_dir.path().join("device_profiles.yaml");

        let manager = config::ConfigManager {
            config_path,
            macros_path,
            cache_path,
            profiles_dir,
            remaps_path,
            device_profiles_path,
            layer_state_path: temp_dir.path().join("layer_state.yaml"),
            config: Arc::new(RwLock::new(config::DaemonConfig::default())),
            macros: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            profiles: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            remaps: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            device_profiles: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        };

        Arc::new(manager)
    }

    #[tokio::test]
    async fn test_ipc_server_creation() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let server = IpcServer::new(&socket_path).unwrap();
        assert_eq!(server.socket_path, socket_path.to_string_lossy());
    }

    #[tokio::test]
    async fn test_request_handling() {
        let state = Arc::new(RwLock::new(DaemonState::new()));
        let macro_engine = Arc::new(macro_engine::MacroEngine::new());
        let injector = create_test_injector();
        let config_manager = create_test_config_manager().await;
        let security_manager = Arc::new(RwLock::new(security::SecurityManager::new(false)));

        // Test GetDevices request
        let response = handle_request(Request::GetDevices, Arc::clone(&state), Arc::clone(&macro_engine), Arc::clone(&injector), Arc::clone(&config_manager), Arc::clone(&security_manager), None).await;
        assert!(matches!(response, Response::Devices(_)));

        // Test GetStatus request
        let response = handle_request(Request::GetStatus, Arc::clone(&state), Arc::clone(&macro_engine), Arc::clone(&injector), Arc::clone(&config_manager), Arc::clone(&security_manager), None).await;
        match response {
            Response::Status { version, .. } => assert_eq!(version, "0.1.0"),
            _ => panic!("Expected Status response"),
        }

        // Test SetMacro request with non-existent device
        let test_macro = MacroEntry {
            name: "test".to_string(),
            trigger: KeyCombo {
                keys: vec![30],
                modifiers: vec![],
            },
            actions: vec![Action::KeyPress(30)],
            device_id: None,
            enabled: true,
            humanize: false,
            capture_mouse: false,
        };

        let response = handle_request(
            Request::SetMacro {
                device_path: "/nonexistent".to_string(),
                macro_entry: test_macro,
            },
            Arc::clone(&state),
            Arc::clone(&macro_engine),
            Arc::clone(&injector),
            Arc::clone(&config_manager),
            Arc::clone(&security_manager),
            None
        ).await;

        match response {
            Response::Error(msg) => assert!(msg.contains("not found")),
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_macro_addition() {
        // Create test state
        let state = Arc::new(RwLock::new(DaemonState::new()));
        let macro_engine = Arc::new(macro_engine::MacroEngine::new());
        let injector = create_test_injector();
        let config_manager = create_test_config_manager().await;
        let security_manager = Arc::new(RwLock::new(security::SecurityManager::new(false)));

        // Add a device first
        {
            let state = state.write().await;
            state.devices.lock().unwrap().push(DeviceInfo {
                name: "Test Device".to_string(),
                path: PathBuf::from("/dev/input/test"),
                vendor_id: 0x1234,
                product_id: 0x5678,
                phys: "test-phys".to_string(),
                device_type: aethermap_common::DeviceType::Other,
            });
        }

        // Now add a macro
        let test_macro = MacroEntry {
            name: "test".to_string(),
            trigger: KeyCombo {
                keys: vec![30],
                modifiers: vec![],
            },
            actions: vec![Action::KeyPress(30)],
            device_id: None,
            enabled: true,
            humanize: false,
            capture_mouse: false,
        };

        let response = handle_request(
            Request::SetMacro {
                device_path: "/dev/input/test".to_string(),
                macro_entry: test_macro.clone(),
            },
            Arc::clone(&state),
            Arc::clone(&macro_engine),
            Arc::clone(&injector),
            Arc::clone(&config_manager),
            Arc::clone(&security_manager),
            None
        ).await;

        assert!(matches!(response, Response::Ack));

        // Verify the macro was added
        let state = state.read().await;
        assert_eq!(state.macros.lock().unwrap().len(), 1);
        let macros = state.macros.lock().unwrap();
        let first_macro = macros.values().next().unwrap();
        assert_eq!(first_macro.name, test_macro.name);
    }

    #[tokio::test]
    async fn test_profile_activation_ipc_flow() {
        use crate::config::ConfigManager;
        use crate::device::DeviceManager;
        use crate::remap_engine::RemapProfile;
        use std::collections::HashMap;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create a config manager with test paths
        let config_manager = ConfigManager {
            config_path: temp_dir.path().join("config.yaml"),
            macros_path: temp_dir.path().join("macros.yaml"),
            cache_path: temp_dir.path().join("macros.bin"),
            profiles_dir: temp_dir.path().join("profiles"),
            remaps_path: temp_dir.path().join("remaps.yaml"),
            device_profiles_path: temp_dir.path().join("device_profiles.yaml"),
                layer_state_path: std::path::PathBuf::from("/tmp/layer_state.yaml"),
            config: Default::default(),
            macros: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            remaps: Arc::new(RwLock::new(HashMap::new())),
            device_profiles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Create a test profile directly in storage
        let mut config = HashMap::new();
        config.insert("capslock".to_string(), "leftctrl".to_string());
        let profile = RemapProfile::new("test".to_string(), &config).unwrap();

        // Store in device_profiles
        let mut profiles = config_manager.device_profiles.write().await;
        profiles.entry("1532:0220".to_string())
            .or_insert_with(HashMap::new)
            .insert("test".to_string(), profile);
        drop(profiles);

        let state = Arc::new(RwLock::new(DaemonState::new()));
        let macro_engine = Arc::new(macro_engine::MacroEngine::new());
        let injector = create_test_injector();
        let security_manager = Arc::new(RwLock::new(security::SecurityManager::new(false)));

        // Test GetDeviceProfiles request
        let request = Request::GetDeviceProfiles {
            device_id: "1532:0220".to_string(),
        };

        let response = handle_request(
            request,
            Arc::clone(&state),
            Arc::clone(&macro_engine),
            Arc::clone(&injector),
            Arc::new(config_manager),
            Arc::clone(&security_manager),
            None
        ).await;

        assert!(matches!(response, Response::DeviceProfiles { .. }));
        if let Response::DeviceProfiles { profiles, .. } = response {
            assert!(profiles.contains(&"test".to_string()));
        }

        // Test GetActiveProfile request (no device manager initialized)
        let request = Request::GetActiveProfile {
            device_id: "1532:0220".to_string(),
        };

        let response = handle_request(
            request,
            Arc::clone(&state),
            Arc::clone(&macro_engine),
            Arc::clone(&injector),
            Arc::new(ConfigManager {
                config_path: temp_dir.path().join("config2.yaml"),
                macros_path: temp_dir.path().join("macros2.yaml"),
                cache_path: temp_dir.path().join("macros2.bin"),
                profiles_dir: temp_dir.path().join("profiles2"),
                remaps_path: temp_dir.path().join("remaps2.yaml"),
                device_profiles_path: temp_dir.path().join("device_profiles2.yaml"),
                layer_state_path: std::path::PathBuf::from("/tmp/layer_state.yaml"),
                config: Default::default(),
                macros: Arc::new(RwLock::new(HashMap::new())),
                profiles: Arc::new(RwLock::new(HashMap::new())),
                remaps: Arc::new(RwLock::new(HashMap::new())),
                device_profiles: Arc::new(RwLock::new(HashMap::new())),
            }),
            Arc::clone(&security_manager),
            None
        ).await;

        // Should return error since device manager is not initialized
        assert!(matches!(response, Response::Error(_)));
    }

    #[tokio::test]
    async fn test_analog_calibration_conversion() {
        use aethermap_common::AnalogCalibrationConfig;

        let config = AnalogCalibrationConfig {
            deadzone: 0.2,
            deadzone_shape: "circular".to_string(),
            sensitivity: "quadratic".to_string(),
            sensitivity_multiplier: 1.5,
            range_min: -32768,
            range_max: 32767,
            invert_x: true,
            invert_y: false,
            exponent: 2.0,
            analog_mode: aethermap_common::AnalogMode::Disabled,
            camera_output_mode: None,
        };

        let calibration = config_to_calibration(config.clone()).unwrap();
        let restored = calibration_to_config(&calibration);

        assert_eq!(restored.deadzone, config.deadzone);
        assert_eq!(restored.deadzone_shape, config.deadzone_shape);
        assert_eq!(restored.sensitivity, config.sensitivity);
        assert_eq!(restored.sensitivity_multiplier, config.sensitivity_multiplier);
        assert_eq!(restored.range_min, config.range_min);
        assert_eq!(restored.range_max, config.range_max);
        assert_eq!(restored.invert_x, config.invert_x);
        assert_eq!(restored.invert_y, config.invert_y);
    }

    #[tokio::test]
    async fn test_invalid_calibration_rejected() {
        use aethermap_common::AnalogCalibrationConfig;

        // Invalid deadzone (> 1.0)
        let invalid_config = AnalogCalibrationConfig {
            deadzone: 1.5,
            deadzone_shape: "circular".to_string(),
            sensitivity: "linear".to_string(),
            sensitivity_multiplier: 1.0,
            range_min: -32768,
            range_max: 32767,
            invert_x: false,
            invert_y: false,
            exponent: 2.0,
            analog_mode: aethermap_common::AnalogMode::Disabled,
            camera_output_mode: None,
        };

        let result = config_to_calibration(invalid_config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_analog_calibration_ipc_roundtrip() {
        use aethermap_common::AnalogCalibrationConfig;
        use crate::analog_processor::AnalogProcessor;

        let mut state = DaemonState::new();
        state.analog_processor = Some(Arc::new(AnalogProcessor::new()));
        let state = Arc::new(RwLock::new(state));

        let macro_engine = Arc::new(macro_engine::MacroEngine::new());
        let injector = create_test_injector();
        let config_manager = create_test_config_manager().await;
        let security_manager = Arc::new(RwLock::new(security::SecurityManager::new(false)));

        // First, set a calibration
        let config = AnalogCalibrationConfig {
            deadzone: 0.25,
            deadzone_shape: "square".to_string(),
            sensitivity: "exponential".to_string(),
            sensitivity_multiplier: 2.0,
            range_min: -16384,
            range_max: 16383,
            invert_x: true,
            invert_y: false,
            exponent: 3.0,
            analog_mode: aethermap_common::AnalogMode::Wasd,
            camera_output_mode: None,
        };

        let response = handle_request(
            Request::SetAnalogCalibration {
                device_id: "32b6:12f7".to_string(),
                layer_id: 0,
                calibration: config.clone(),
            },
            Arc::clone(&state),
            Arc::clone(&macro_engine),
            Arc::clone(&injector),
            Arc::clone(&config_manager),
            Arc::clone(&security_manager),
            None
        ).await;

        assert!(matches!(response, Response::AnalogCalibrationAck));

        // Now retrieve it
        let response = handle_request(
            Request::GetAnalogCalibration {
                device_id: "32b6:12f7".to_string(),
                layer_id: 0,
            },
            Arc::clone(&state),
            Arc::clone(&macro_engine),
            Arc::clone(&injector),
            Arc::clone(&config_manager),
            Arc::clone(&security_manager),
            None
        ).await;

        match response {
            Response::AnalogCalibration { calibration: Some(cal), .. } => {
                assert_eq!(cal.deadzone, config.deadzone);
                assert_eq!(cal.deadzone_shape, config.deadzone_shape);
                assert_eq!(cal.sensitivity, config.sensitivity);
                assert_eq!(cal.sensitivity_multiplier, config.sensitivity_multiplier);
            }
            _ => panic!("Expected AnalogCalibration response with calibration data, got {:?}", response),
        }
    }

    #[tokio::test]
    async fn test_analog_calibration_default_on_missing() {
        use aethermap_common::AnalogCalibrationConfig;

        let state = Arc::new(RwLock::new(DaemonState::new()));
        let macro_engine = Arc::new(macro_engine::MacroEngine::new());
        let injector = create_test_injector();
        let config_manager = create_test_config_manager().await;
        let security_manager = Arc::new(RwLock::new(security::SecurityManager::new(false)));

        // Request calibration for non-existent device
        let response = handle_request(
            Request::GetAnalogCalibration {
                device_id: "nonexistent".to_string(),
                layer_id: 0,
            },
            Arc::clone(&state),
            Arc::clone(&macro_engine),
            Arc::clone(&injector),
            Arc::clone(&config_manager),
            Arc::clone(&security_manager),
            None
        ).await;

        // Should return default calibration
        match response {
            Response::AnalogCalibration { calibration: None, .. } => {
                // Expected - no custom calibration configured
            }
            Response::AnalogCalibration { calibration: Some(cal), .. } => {
                // Should have default values
                assert_eq!(cal.deadzone, 0.15);
                assert_eq!(cal.deadzone_shape, "circular");
            }
            _ => panic!("Expected AnalogCalibration response"),
        }
    }
}
