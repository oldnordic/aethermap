//! Device Hotplug Detection Module
//!
//! This module provides udev-based monitoring for input device hotplug events.
//! It detects when devices are added or removed from the system and streams
//! events to the main daemon loop via mpsc channels.
//!
//! # Architecture
//!
//! - `DeviceMonitor`: Spawns a blocking udev monitor task and provides an async receiver
//! - `DeviceEvent`: Enum representing device add/remove events with device identification
//! - Uses `udev::MonitorBuilder` with subsystem filtering for input devices only
//! - Blocking udev socket iteration runs in `spawn_blocking` to avoid blocking async runtime
//!
//! # Device ID Format
//!
//! Device IDs use the "vendor:product" hexadecimal format (e.g., "1532:0220")
//! matching the YAML configuration format and `DeviceManager::format_device_id()`.

use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Device hotplug event
///
/// Represents a device being added or removed from the system.
#[derive(Debug, Clone)]
pub enum DeviceEvent {
    /// A device was plugged in
    Add {
        /// Device node path (e.g., "/dev/input/event0")
        devnode: String,
        /// Device identifier in vendor:product format (e.g., "1532:0220")
        device_id: String,
    },
    /// A device was unplugged
    Remove {
        /// Device node path (e.g., "/dev/input/event0")
        devnode: String,
        /// Device identifier in vendor:product format (e.g., "1532:0220")
        device_id: String,
    },
}

/// Device hotplug monitor
///
/// Spawns a udev monitor task and provides an async interface for receiving
/// device add/remove events.
pub struct DeviceMonitor {
    /// Receiver for device events from the udev monitor task
    receiver: mpsc::Receiver<DeviceEvent>,
}

impl DeviceMonitor {
    /// Create a new device monitor with udev event detection
    ///
    /// This spawns a background task that monitors udev for input subsystem
    /// events. Events are sent to the receiver returned by `recv()`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - udev monitor cannot be created
    /// - Input subsystem cannot be matched
    /// - Monitor socket cannot be bound
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, receiver) = mpsc::channel(32);

        // Spawn blocking task for udev monitoring
        tokio::task::spawn_blocking(move || {
            if let Err(e) = run_udev_monitor(tx) {
                error!("Udev monitor task exited with error: {}", e);
            }
        });

        Ok(Self { receiver })
    }

    /// Receive the next device event
    ///
    /// Returns `None` when the udev monitor task exits (should not happen
    /// under normal operation).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut monitor = DeviceMonitor::new()?;
    /// while let Some(event) = monitor.recv().await {
    ///     match event {
    ///         DeviceEvent::Add { devnode, device_id } => {
    ///             info!("Device added: {} ({})", device_id, devnode);
    ///         }
    ///         DeviceEvent::Remove { devnode, device_id } => {
    ///             info!("Device removed: {} ({})", device_id, devnode);
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn recv(&mut self) -> Option<DeviceEvent> {
        self.receiver.recv().await
    }
}

/// Format a device identifier from vendor and product IDs
///
/// Uses the same format as `DeviceManager::format_device_id()`.
///
/// # Arguments
///
/// * `vendor_id` - Vendor ID (e.g., 0x1532 for Razer)
/// * `product_id` - Product ID (e.g., 0x0220)
///
/// # Returns
///
/// Device identifier string in "vendor:product" hexadecimal format (e.g., "1532:0220")
pub fn format_device_id(vendor_id: u16, product_id: u16) -> String {
    format!("{:04x}:{:04x}", vendor_id, product_id)
}

/// Run the udev monitor in a blocking task
///
/// This function blocks on udev socket iteration, which is why it must
/// run in a `spawn_blocking` task.
fn run_udev_monitor(tx: mpsc::Sender<DeviceEvent>) -> Result<(), Box<dyn std::error::Error>> {
    use udev::{EventType, MonitorBuilder};

    info!("Starting udev monitor for input subsystem");

    // Create udev monitor for input devices
    let socket = MonitorBuilder::new()?.match_subsystem("input")?.listen()?;

    debug!("Udev monitor listening on netlink socket");

    // Track devnode -> device_id mapping for remove events
    let mut device_map: HashMap<String, String> = HashMap::new();

    // Block on udev events
    for event in socket.iter() {
        let devnode = match event.devnode() {
            Some(path) => path.to_string_lossy().to_string(),
            None => continue,
        };

        match event.event_type() {
            EventType::Add => {
                debug!("Device add event: {}", devnode);

                // Try to extract device ID from the device
                if let Ok(device_id) = extract_device_id(&devnode) {
                    debug!("Device ID for {}: {}", devnode, device_id);

                    // Store mapping for remove events
                    device_map.insert(devnode.clone(), device_id.clone());

                    let event = DeviceEvent::Add { devnode, device_id };

                    if let Err(e) = tx.blocking_send(event) {
                        warn!("Failed to send device add event: {}", e);
                        // If send failed, receiver is dropped - exit monitoring
                        break;
                    }
                } else {
                    // Device doesn't have VID/PID or couldn't be opened
                    // Skip it - we only care about identifiable devices
                    debug!("Skipping device without VID/PID: {}", devnode);
                }
            }
            EventType::Remove => {
                debug!("Device remove event: {}", devnode);

                // Look up device ID from our tracking map
                if let Some(device_id) = device_map.remove(&devnode) {
                    let event = DeviceEvent::Remove { devnode, device_id };

                    if let Err(e) = tx.blocking_send(event) {
                        warn!("Failed to send device remove event: {}", e);
                        // If send failed, receiver is dropped - exit monitoring
                        break;
                    }
                } else {
                    // Device wasn't tracked (maybe failed to extract ID on add)
                    debug!("Remove event for untracked device: {}", devnode);
                }
            }
            _ => {
                // Ignore other event types (Change, Bind, etc.)
                debug!("Ignoring udev event: {:?}", event.event_type());
            }
        }
    }

    info!("Udev monitor task exited");
    Ok(())
}

/// Extract device ID (vendor:product) from a device node
///
/// Opens the device with evdev and reads the input_id to get vendor
/// and product IDs. Returns `Err` if the device cannot be opened or
/// doesn't have the required information.
fn extract_device_id(devnode: &str) -> Result<String, Box<dyn std::error::Error>> {
    use evdev::Device;

    let device = Device::open(devnode)?;
    let input_id = device.input_id();

    let vendor_id = input_id.vendor();
    let product_id = input_id.product();

    // Skip devices with no VID/PID
    if vendor_id == 0 && product_id == 0 {
        return Err("Device has no vendor/product ID".into());
    }

    Ok(format_device_id(vendor_id, product_id))
}

// =============================================================================
// LED State Hotplug Handlers
// =============================================================================

/// Handle device removal - preserve LED state in memory
///
/// LED state is already stored in DaemonState.led_state HashMap, so
/// this is primarily a no-op that logs the event for debugging.
/// The LED state persists in memory and will be available when the
/// device is reconnected.
///
/// # Arguments
///
/// * `device_id` - Device identifier (vendor:product format)
/// * `_state` - Daemon state reference (for future LED cleanup if needed)
///
/// # Returns
///
/// * `Ok(())` - LED state preserved
/// * `Err(Box<dyn Error>)` - Error during preservation
pub async fn handle_device_remove(
    device_id: &str,
    _state: &tokio::sync::RwLock<crate::DaemonState>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "Device {} removed - LED state preserved in memory",
        device_id
    );

    // LED state is already in DaemonState.led_state HashMap
    // No explicit save needed - state persists in memory

    // Note: If led_controller needs cleanup, do it here
    // e.g., state.led_controller = None;

    Ok(())
}

/// Handle device addition - restore LED state if available
///
/// Checks if the device has previously saved LED state and restores it.
/// This involves re-opening the LED controller and applying the saved
/// configuration (colors, brightness, patterns).
///
/// # Arguments
///
/// * `device_id` - Device identifier (vendor:product format)
/// * `state` - Daemon state reference containing LED state and controller
///
/// # Returns
///
/// * `Ok(())` - LED state restored or no previous state
/// * `Err(Box<dyn Error>)` - Error during restoration
pub async fn handle_device_add(
    device_id: &str,
    state: &tokio::sync::RwLock<crate::DaemonState>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "Device {} added - checking LED state restoration",
        device_id
    );

    // Check if this device has saved LED state
    let has_led_state = {
        let state_guard = state.read().await;
        let led_state = state_guard.led_state.read().await;
        led_state.contains_key(device_id)
    };

    if has_led_state {
        info!("Restoring LED state for device {}", device_id);

        // Re-open LED controller
        match crate::led_controller::LedController::find_led_interface() {
            Ok(led_controller) => {
                // Get saved LED state
                let saved_state = {
                    let state_guard = state.read().await;
                    let led_state = state_guard.led_state.read().await;
                    led_state.get(device_id).cloned()
                };

                if let Some(saved) = saved_state {
                    // Restore LED state
                    if let Err(e) = led_controller.import_state(saved).await {
                        warn!("Failed to restore LED state for {}: {}", device_id, e);
                    } else {
                        info!("LED state restored for device {}", device_id);
                    }
                }

                // Store controller in DaemonState
                let mut state_guard = state.write().await;
                state_guard
                    .set_led_controller(Some(std::sync::Arc::new(led_controller)))
                    .await;
            }
            Err(e) => {
                warn!("Failed to open LED controller for {}: {}", device_id, e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_device_id() {
        // Test Razer device
        assert_eq!(format_device_id(0x1532, 0x0220), "1532:0220");
        // Test case handling
        assert_eq!(format_device_id(0xABCD, 0x1234), "abcd:1234");
        // Test zero padding
        assert_eq!(format_device_id(0x0001, 0x0002), "0001:0002");
    }

    #[test]
    fn test_device_event_add() {
        let event = DeviceEvent::Add {
            devnode: "/dev/input/event0".to_string(),
            device_id: "1532:0220".to_string(),
        };

        match event {
            DeviceEvent::Add { devnode, device_id } => {
                assert_eq!(devnode, "/dev/input/event0");
                assert_eq!(device_id, "1532:0220");
            }
            _ => panic!("Expected Add event"),
        }
    }

    #[test]
    fn test_device_event_remove() {
        let event = DeviceEvent::Remove {
            devnode: "/dev/input/event1".to_string(),
            device_id: "046d:c52b".to_string(),
        };

        match event {
            DeviceEvent::Remove { devnode, device_id } => {
                assert_eq!(devnode, "/dev/input/event1");
                assert_eq!(device_id, "046d:c52b");
            }
            _ => panic!("Expected Remove event"),
        }
    }
}
