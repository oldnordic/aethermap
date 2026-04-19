//! Aethermap Daemon - Main Entry Point
//!
//! This is the privileged system daemon responsible for:
//! - Device discovery and management
//! - Macro recording and playback
//! - IPC communication with the GUI client
//! - Security management and privilege dropping

use aethermap_common::tracing;
use aethermapd::{DaemonState, config, device, hotplug, macro_engine, injector, ipc, security, remap_engine, analog_processor, led_controller};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, warn};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check for test mode first
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--test-security" {
        return security::test_security_functionality().await;
    }

// Main daemon implementation
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("Starting Aethermap Daemon v0.1.0");

    // Check if we're running as root (required for privileged operations)
    if !security::SecurityManager::is_root() {
        error!("Aethermap daemon must be started as root for device access");
        return Err("Insufficient privileges".into());
    }

    // Determine socket path
    let socket_path = determine_socket_path()?;
    info!("Using socket path: {}", socket_path);

    // Initialize security manager with token authentication based on feature flag
    let token_auth_enabled = cfg!(feature = "token-auth");
    let security_manager = Arc::new(RwLock::new(security::SecurityManager::new(token_auth_enabled)));
    info!("Token authentication {}", if token_auth_enabled { "enabled" } else { "disabled" });

    // Create shared state
    let state = Arc::new(RwLock::new(DaemonState::new()));

    // Initialize components
    let config_manager = Arc::new(config::ConfigManager::new().await?);
    let injector = injector::UinputInjector::new()?;

    // Initialize injector with full privileges before dropping them
    {
        injector.initialize().await.map_err(|e| -> Box<dyn std::error::Error> { e })?;
        info!("Uinput injector initialized");
    }

    // Wrap injector in Arc<RwLock<dyn Injector>> for MacroEngine
    let injector_for_macro: Arc<RwLock<dyn injector::Injector + Send + Sync>> =
        Arc::new(RwLock::new(injector));

    // Clone Arc for IPC server (it can downcast or use trait methods)
    let injector_for_ipc = Arc::clone(&injector_for_macro);

    // Create and initialize remap engine
    let remap_engine = Arc::new(remap_engine::RemapEngine::new());

    // Load remap configuration with eager validation
    let remap_entries = config_manager.load_remaps().await
        .map_err(|e| -> Box<dyn std::error::Error> {
            error!("Failed to load remap configuration: {}", e);
            e.into()
        })?;

    // Convert RemapEntry vec to HashMap for RemapEngine
    let mut remap_hash = HashMap::new();
    for entry in remap_entries {
        remap_hash.insert(entry.from, entry.to);
    }

    // Eager validation happens here - all keys validated before daemon accepts config
    remap_engine.load_config(&remap_hash).await
        .map_err(|e| -> Box<dyn std::error::Error> {
            error!("Failed to load remap configuration: {}", e);
            e.into()
        })?;

    info!("Loaded {} remap(s)", remap_engine.remap_count().await);

    // Store in daemon state
    {
        let mut state = state.write().await;
        state.remap_engine = Some(Arc::clone(&remap_engine));
    }

    // Initialize macro engine with injector
    let macro_engine = Arc::new(macro_engine::MacroEngine::with_injector(Arc::clone(&injector_for_macro)));
    {
        let mut state = state.write().await;
        state.macro_engine = Some(Arc::clone(&macro_engine));
    }

    // Initialize GlobalHotkeyManager for daemon-level profile switching
    // Get layer_manager from DaemonState (created in DaemonState::new())
    let layer_manager_for_hotkeys = {
        let state_guard = state.read().await;
        Arc::clone(&state_guard.layer_manager)
    };
    let global_hotkey_manager = Arc::new(tokio::sync::Mutex::new(
        aethermapd::GlobalHotkeyManager::new(
            layer_manager_for_hotkeys,
            Arc::clone(&config_manager),
        )
    ));

    // Load hotkey bindings from device_profiles.yaml at startup
    {
        let mut hotkey_mgr = global_hotkey_manager.lock().await;
        if let Err(e) = hotkey_mgr.load_bindings().await {
            warn!("Failed to load hotkey bindings at startup: {}", e);
        }
    }

    // Store GlobalHotkeyManager in DaemonState for IPC access
    {
        let mut state = state.write().await;
        state.hotkey_manager = Some(global_hotkey_manager);
    }

    // Create and initialize device manager
    let mut device_manager = device::DeviceManager::new();

    // Create analog processor for D-pad emulation
    let analog_processor = Arc::new(analog_processor::AnalogProcessor::new());

    // Set remap engine and injector on device manager for event processing
    device_manager.set_remap_engine(Some(remap_engine));
    device_manager.set_injector(Some(injector_for_macro.clone()));
    device_manager.set_macro_engine(Some(Arc::clone(&macro_engine)));
    device_manager.set_config_manager(Some(Arc::clone(&config_manager)));
    device_manager.set_analog_processor(Some(Arc::clone(&analog_processor)));
    device_manager.set_daemon_state(Some(Arc::clone(&state)));

    // Set layer_manager on device manager for analog mode processing
    let state_guard = state.read().await;
    device_manager.set_layer_manager(Some(Arc::clone(&state_guard.layer_manager)));

    // Set hotkey_manager on device manager for profile switching
    if let Some(hotkey_mgr) = state_guard.hotkey_manager.clone() {
        drop(state_guard);
        device_manager.set_hotkey_manager(Some(hotkey_mgr));
    } else {
        drop(state_guard);
    }

    if let Err(e) = device_manager.start_discovery().await {
        error!("Device discovery failed: {}", e);
    } else {
        info!("Device discovery successful");

        // Load device profiles from config
        let device_profiles = config_manager.load_device_profiles_extended().await
            .map_err(|e| -> Box<dyn std::error::Error> {
                error!("Failed to load device profiles: {}", e);
                e.into()
            })?;

        // Store profiles in device manager
        device_manager.set_device_profiles(device_profiles);
        info!("Device profiles loaded and stored in device manager");

        // Update devices in shared state
        let discovered_devices = device_manager.get_devices();
        {
            let state = state.write().await;
            *state.devices.lock().unwrap() = discovered_devices;
        }

        // Start device event processing loop
        let event_receiver = device_manager.get_event_receiver();

        // Wrap device_manager in Arc<RwLock<>> for sharing with IPC
        let device_manager = Arc::new(RwLock::new(device_manager));
        {
            let mut state = state.write().await;
            state.device_manager = Some(Arc::clone(&device_manager));
            state.analog_processor = Some(Arc::clone(&analog_processor));
        }

        let state_clone = Arc::clone(&state);

        let state_clone2 = Arc::clone(&state_clone);
        tokio::spawn(async move {
            let mut event_receiver = event_receiver;
            loop {
                if let Some(msg) = event_receiver.recv().await {
                    use aethermapd::device::DeviceEventMessage;
                    use aethermapd::device::DeviceEventType;

                    let device_path = msg.device_path.clone();

                    // Get injector early for passthrough
                    let injector = {
                        let state = state_clone2.read().await;
                        if let Some(device_manager) = &state.device_manager {
                            device_manager.read().await.get_injector()
                        } else {
                            None
                        }
                    };

                    // Handle based on event type
                    match msg.event_type {
                        DeviceEventType::Key { original_code, value } => {
                            let pressed = value == 1;
                            let is_repeat = value == 2;

                            // Forward to macro engine (skip repeat events)
                            // Note: Remapped keys are already handled in device.rs event loop
                            let consumed = if !is_repeat {
                                let state = state_clone2.read().await;
                                if let Some(macro_engine) = &state.macro_engine {
                                    match macro_engine.process_input_event(
                                        msg.key_code.unwrap_or(original_code),
                                        pressed,
                                        &device_path
                                    ).await {
                                        Ok(consumed) => consumed,
                                        Err(e) => {
                                            error!("Error processing input event: {}", e);
                                            false
                                        }
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                            // Passthrough: If not consumed by macro, inject the original event
                            if !consumed {
                                if let Some(inj) = injector {
                                    let injector_ref = inj.read().await;
                                    // Check if this is a mouse button (BTN_LEFT=272, BTN_RIGHT=273, etc.)
                                    if original_code >= 272 && original_code <= 288 {
                                        // Mouse button: convert to 1-based (272->1, 273->2, etc.)
                                        let button_num = original_code - 271;
                                        if value == 0 {
                                            let _ = injector_ref.mouse_release(button_num).await;
                                        } else {
                                            let _ = injector_ref.mouse_press(button_num).await;
                                        }
                                    } else {
                                        // Regular key
                                        if value == 0 {
                                            let _ = injector_ref.key_release(original_code).await;
                                        } else if value == 1 || value == 2 {
                                            let _ = injector_ref.key_press(original_code).await;
                                        }
                                    }
                                }
                            }
                        }
                        DeviceEventType::RelAxis { axis, value } => {
                            use evdev::RelativeAxisType;

                            // For relative axis events, always passthrough (macros don't typically handle mouse movement)
                            if let Some(inj) = injector {
                                let injector_ref = inj.read().await;
                                match axis {
                                    RelativeAxisType::REL_X => {
                                        let _ = injector_ref.mouse_move(value, 0).await;
                                    }
                                    RelativeAxisType::REL_Y => {
                                        let _ = injector_ref.mouse_move(0, value).await;
                                    }
                                    RelativeAxisType::REL_WHEEL => {
                                        let _ = injector_ref.mouse_scroll(value).await;
                                    }
                                    RelativeAxisType::REL_HWHEEL => {
                                        let _ = injector_ref.mouse_scroll(-value).await;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    // Start device hotplug monitoring
    let mut device_monitor = hotplug::DeviceMonitor::new()
        .map_err(|e| -> Box<dyn std::error::Error> {
            error!("Failed to start device monitor: {}", e);
            e.into()
        })?;
    info!("Device hotplug monitoring started");

    // Load configuration
    config_manager.load_config_mut().await?;

    // Load macros from the default profile
    if let Some(default_profile) = config_manager.get_profile("default").await {
        for (macro_name, macro_entry) in &default_profile.macros {
            if let Err(e) = macro_engine.add_macro(macro_entry.clone()).await {
                error!("Failed to add macro '{}' from profile: {}", macro_name, e);
            }
        }
    }

    // Load device profiles
    config_manager.load_device_profiles().await
        .map_err(|e| {
            error!("Failed to load device profiles: {}", e);
            e
        })?;
    info!("Device profiles loaded");

    // Initialize AutoProfileSwitcher for focus-based profile switching
    let state_guard = state.read().await;
    let auto_profile_switcher = Some(Arc::new(aethermapd::auto_profile_switcher::AutoProfileSwitcher::new(
        state_guard.layer_manager.clone(),
        config_manager.clone(),
    )));
    drop(state_guard);
    info!("AutoProfileSwitcher initialized");

    // Log devices with profiles configured
    let devices_with_profiles = config_manager.list_profile_devices().await;
    if !devices_with_profiles.is_empty() {
        info!("Found profiles for {} devices: {:?}", devices_with_profiles.len(), devices_with_profiles);
    }

    // Initialize LED controller BEFORE privilege drop
    // HID devices must be opened with full privileges
    // Note: LedController is stored in DaemonState for IPC access.
    // The HID device handle is wrapped inside LedController which provides
    // async methods for thread-safe access.
    let led_controller = match led_controller::LedController::find_led_interface() {
        Ok(controller) => {
            info!("Azeron LED controller initialized successfully");
            Some(Arc::new(controller))
        }
        Err(led_controller::LedError::DeviceNotFound) => {
            info!("No Azeron LED control interface found - LED support disabled");
            None
        }
        Err(e) => {
            warn!("Failed to initialize LED controller: {} - continuing without LED support", e);
            None
        }
    };

    // Store LED controller in DaemonState and update LayerManager for layer-aware LED feedback
    {
        let mut state_guard = state.write().await;
        state_guard.set_led_controller(led_controller).await;
    }

    // Initialize LED state for Azeron device (if LED controller is available)
    // Azeron vendor ID is 0x16d0
    const AZERON_VENDOR_ID: &str = "16d0";
    {
        let state_guard = state.read().await;
        if let Some(device_manager) = &state_guard.device_manager {
            let dm = device_manager.read().await;
            // Check if any Azeron device is present
            let devices = dm.get_devices();
            for device in devices {
                let device_id = aethermapd::device::DeviceManager::format_device_id(
                    device.vendor_id,
                    device.product_id
                );
                // Check if this is an Azeron device
                if device_id.contains(AZERON_VENDOR_ID) {
                    info!("Initializing LED state for Azeron device {}", device_id);
                    drop(dm);
                    state_guard.initialize_led_state(&device_id).await;
                    break;
                }
            }
        }
    }

    // AFTER completing all privileged initialization (uinput, device discovery, LED controller, etc.)
    // Drop privileges to minimize attack surface
    {
        let mut security = security_manager.write().await;
        if let Err(e) = security.drop_privileges() {
            error!("Failed to drop privileges: {}", e);
        } else {
            info!("Successfully dropped privileges after initialization");
        }
    }

    // Clone config_manager and state for signal handlers before IPC server takes ownership
    let config_manager_for_signals = Arc::clone(&config_manager);
    let state_for_signals = Arc::clone(&state);
    let state_for_shutdown = Arc::clone(&state);

    // Start IPC server
    let mut ipc_server = ipc::IpcServer::new(&socket_path)?;
    ipc_server.start(
        state,
        macro_engine,
        injector_for_ipc,
        config_manager,
        security_manager,
        auto_profile_switcher.clone()
    ).await?;
    info!("IPC server started successfully");

    // Set up signal handlers for graceful shutdown and hot-reload
    let mut signals = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let mut sig_hup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())?;

    // Wait for shutdown signal, SIGHUP, or device hotplug events
    tokio::select! {
        _ = sig_hup.recv() => {
            info!("Received SIGHUP, reloading configuration");
            // Reload device profiles
            if let Err(e) = config_manager_for_signals.reload_device_profiles().await {
                error!("Failed to reload device profiles: {}", e);
            } else {
                info!("Device profiles reloaded successfully");
            }
            // Reload global remappings
            let state = state_for_signals.read().await;
            if let Some(remap_engine) = &state.remap_engine {
                if let Err(e) = config_manager_for_signals.reload_remaps(Arc::clone(remap_engine)).await {
                    error!("Failed to reload global remaps: {}", e);
                } else {
                    info!("Global remappings reloaded successfully");
                }
            }
            // After SIGHUP, continue to wait for shutdown signals or device events
            drop(state);
            tokio::select! {
                Some(event) = device_monitor.recv() => {
                    match event {
                        hotplug::DeviceEvent::Add { devnode, device_id } => {
                            info!("Device added: {} ({})", device_id, devnode);
                            let state = state_for_signals.read().await;
                            if let Some(device_manager) = &state.device_manager {
                                let mut dm = device_manager.write().await;
                                if let Err(e) = dm.handle_device_add(&devnode).await {
                                    warn!("Failed to handle device add {}: {}", devnode, e);
                                }
                            }
                            drop(state);
                            // Restore LED state if available
                            if let Err(e) = hotplug::handle_device_add(&device_id, &state_for_signals).await {
                                warn!("Failed to restore LED state for {}: {}", device_id, e);
                            }
                        }
                        hotplug::DeviceEvent::Remove { devnode, device_id } => {
                            info!("Device removed: {} ({})", device_id, devnode);
                            let state = state_for_signals.read().await;
                            if let Some(device_manager) = &state.device_manager {
                                let mut dm = device_manager.write().await;
                                if let Err(e) = dm.handle_device_removal(&devnode).await {
                                    warn!("Failed to handle device removal {}: {}", devnode, e);
                                }
                            }
                            drop(state);
                            // Preserve LED state (no-op - already in DaemonState.led_state)
                            if let Err(e) = hotplug::handle_device_remove(&device_id, &state_for_signals).await {
                                warn!("Failed to preserve LED state for {}: {}", device_id, e);
                            }
                        }
                    }
                    // Continue waiting for shutdown after handling device event
                    drop(device_monitor);
                    tokio::select! {
                        _ = signals.recv() => {
                            info!("Received SIGTERM, shutting down gracefully");
                        }
                        _ = interrupt.recv() => {
                            info!("Received SIGINT, shutting down gracefully");
                        }
                    }
                }
                _ = signals.recv() => {
                    info!("Received SIGTERM, shutting down gracefully");
                }
                _ = interrupt.recv() => {
                    info!("Received SIGINT, shutting down gracefully");
                }
            }
        }
        Some(event) = device_monitor.recv() => {
            match event {
                hotplug::DeviceEvent::Add { devnode, device_id } => {
                    info!("Device added: {} ({})", device_id, devnode);
                    // Restore LED state if available
                    if let Err(e) = hotplug::handle_device_add(&device_id, &state_for_signals).await {
                        warn!("Failed to restore LED state for {}: {}", device_id, e);
                    }
                }
                hotplug::DeviceEvent::Remove { devnode, device_id } => {
                    info!("Device removed: {} ({})", device_id, devnode);
                    // Preserve LED state (no-op - already in DaemonState.led_state)
                    if let Err(e) = hotplug::handle_device_remove(&device_id, &state_for_signals).await {
                        warn!("Failed to preserve LED state for {}: {}", device_id, e);
                    }
                }
            }
            // Continue waiting after handling device event
            drop(device_monitor);
            tokio::select! {
                _ = sig_hup.recv() => {
                    info!("Received SIGHUP, reloading configuration");
                    if let Err(e) = config_manager_for_signals.reload_device_profiles().await {
                        error!("Failed to reload device profiles: {}", e);
                    }
                    let state = state_for_signals.read().await;
                    if let Some(remap_engine) = &state.remap_engine {
                        if let Err(e) = config_manager_for_signals.reload_remaps(Arc::clone(remap_engine)).await {
                            error!("Failed to reload global remaps: {}", e);
                        }
                    }
                    drop(state);
                }
                _ = signals.recv() => {
                    info!("Received SIGTERM, shutting down gracefully");
                }
                _ = interrupt.recv() => {
                    info!("Received SIGINT, shutting down gracefully");
                }
            }
        }
        _ = signals.recv() => {
            info!("Received SIGTERM, shutting down gracefully");
        }
        _ = interrupt.recv() => {
            info!("Received SIGINT, shutting down gracefully");
        }
    }

    // Cleanup
    info!("Starting cleanup...");

    // Shutdown device manager first (ungrab all devices)
    {
        let state = state_for_shutdown.read().await;
        if let Some(device_manager) = &state.device_manager {
            let mut dm = device_manager.write().await;
            if let Err(e) = dm.shutdown().await {
                error!("Error during device manager shutdown: {}", e);
            }
        }
    }

    ipc_server.shutdown().await?;
    info!("Aethermap Daemon shutdown complete");
    Ok(())
}

/// Determine the appropriate socket path based on the platform
fn determine_socket_path() -> Result<String, Box<dyn std::error::Error>> {
    // For system daemon running as root, use RuntimeDirectory from systemd
    // This is created by RuntimeDirectory=aethermap in the service file
    let path = "/run/aethermap/aethermap.sock".to_string();
    info!("Using system-wide socket location: {}", path);
    Ok(path)
}
