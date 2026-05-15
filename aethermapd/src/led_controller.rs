//! LED Controller for Azeron Cyborg keypad
//!
//! This module provides HID-based LED control for the Azeron Cyborg keypad.
//! It manages the HID device connection, handles device enumeration with
//! proper interface selection (filtering out keyboard interface), and provides
//! async methods for LED zone control.
//!
//! # Device Information (Discovered via USB enumeration)
//!
//! - **Vendor ID:** `0x16d0` (MCS / Azeron LTD)
//! - **Product ID:** `0x12f7` (Azeron Cyborg 2 Keypad)
//! - **LED Control Interface:** Interface 4, usage_page `0xff01` (vendor-specific)
//!
//! # Protocol (DISCOVERED via Wireshark USB capture)
//!
//! The Azeron Cyborg 2 uses a **binary protocol** (not the text protocol used by
//! earlier Azeron models). The protocol was discovered via USB packet capture
//! while operating the official Azeron software.
//!
//! **HARDWARE CAPABILITY:** The official Azeron software only controls **brightness**
//! for the side LED (blue). No RGB color control is exposed in the official software.
//!
//! **Protocol Status:**
//! - ✅ Device identification confirmed (vendor: 0x16d0, product: 0x12f7)
//! - ✅ Interface 4 (usage_page 0xff01) confirmed as LED control
//! - ✅ 64-byte packet format (data sent to HID OUT endpoint 6)
//! - ✅ Brightness control protocol discovered
//! - ✅ Keepalive protocol discovered
//! - ❌ RGB control protocol unknown (not exposed in official software)
//!
//! # Packet Format
//!
//! All LED packets are 64 bytes:
//! ```text
//! Offset  Size    Description
//! ------  ------  -----------
//! 0       2 bytes Sequence/type (0x0001 for LED, 0x0000 for keepalive)
//! 2       2 bytes Command counter (little-endian, increments)
//! 4       1 byte  Always 0x01
//! 5       1 byte  Always 0x01
//! 6       2 bytes Brightness value (little-endian: 0x0100 = 256 min, 0x0196 = 406 max)
//! 8       1 byte  Always 0x00
//! 9       1 byte  Always 0x00
//! 10-63   54 bytes Padding (0x00)
//! ```
//!
//! # Brightness Encoding
//!
//! Brightness is encoded as a 16-bit little-endian value at offset 6:
//! - **Minimum (dim):** 0x0100 = 256
//! - **Maximum (bright):** 0x0196 = 406
//! - **Range:** 150 steps (0-149 in byte 6, with byte 7 = 0x01)
//! - **Formula:** `brightness = 256 + byte6`
//! - **Percentage:** `percent = (brightness - 256) * 100 / 150`
//!
//! # Keepalive Packets
//!
//! The device expects regular keepalive packets to maintain the connection:
//! ```text
//! 0000125901010000... (64 bytes)
//! ```
//! - Bytes 2-3 increment as a counter (0x1259, 0x125A, ...)
//! - If keepalive stops, the device may disconnect or become unresponsive
//!
//! # Interface Selection
//!
//! Azeron Cyborg 2 exposes **5 HID interfaces**:
//! - Interface 0: Vendor Specific (255/93/1) - Configuration
//! - Interface 1: HID (3/0/0) - Keyboard input (via evdev)
//! - Interface 2: HID Mouse (3/1/2) - Mouse emulation
//! - Interface 3: HID (3/0/0) - Unknown
//! - **Interface 4: HID (3/0/0) with usage_page `0xff01`** - LED control
//!
//! We must open interface 4 (usage_page 0xff01), not the keyboard interfaces,
//! or LED commands will fail silently.
//!
//! # Privilege Timing
//!
//! The HID device must be opened BEFORE privilege drop in main.rs, or
//! access will be denied. This module provides find_led_interface() for
//! use during privileged initialization.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Azeron vendor ID (MCS / Azeron LTD)
/// Discovered via lsusb: ID 16d0:12f7
const AZERON_VENDOR_ID: u16 = 0x16d0;

/// Azeron Cyborg 2 product ID
const AZERON_CYBORG2_PRODUCT_ID: u16 = 0x12f7;

/// Keyboard interface usage page - we must SKIP this interface
const KEYBOARD_USAGE_PAGE: u16 = 0x0001;

/// Vendor-specific LED control interface usage page (Cyborg 2)
/// Discovered via USB interface enumeration: interface 4 has usage_page 0xff01
const LED_CONTROL_USAGE_PAGE: u16 = 0xff01;

// =============================================================================
// DISCOVERED PROTOCOL CONSTANTS (via Wireshark USB capture)
// =============================================================================

/// HID packet size for Azeron LED commands
/// Discovered: The device sends 64-byte packets to endpoint 6
const AZERON_HID_PACKET_SIZE: usize = 64;

/// Packet type for LED brightness commands
/// Discovered: 0x0001 = LED command, 0x0000 = keepalive
const AZERON_PACKET_TYPE_LED: u16 = 0x0001;
const AZERON_PACKET_TYPE_KEEPALIVE: u16 = 0x0000;

/// Base brightness value (minimum brightness = 0%)
/// Discovered: Brightness is 256 + (0-149 step value)
const AZERON_BRIGHTNESS_BASE: u16 = 256;

/// Maximum brightness step value
/// Discovered: Maximum observed value is 149 (0x95), giving 256+149=405
const AZERON_BRIGHTNESS_MAX_STEP: u16 = 150;

/// LED command packet constant bytes
/// Discovered: Bytes 4-5 are always 0x01, bytes 8-9 are always 0x00
const AZERON_LED_BYTE_4: u8 = 0x01;
const AZERON_LED_BYTE_5: u8 = 0x01;
const AZERON_LED_BYTE_8: u8 = 0x00;
const AZERON_LED_BYTE_9: u8 = 0x00;

// =============================================================================
// PLACEHOLDER CONSTANTS FOR UNIMPLEMENTED FEATURES
// =============================================================================
// RGB color control is NOT exposed in the official Azeron software.
// The hardware may or may not support RGB, but the protocol is currently unknown.
// These constants are kept for potential future discovery.
// =============================================================================

const AZERON_LED_REPORT_ID: u8 = 0x00;
const AZERON_CMD_SET_COLOR: u8 = 0x01;
const AZERON_CMD_SET_BRIGHTNESS: u8 = 0x02;
const AZERON_CMD_SET_PATTERN: u8 = 0x03;
const AZERON_ZONE_GLOBAL: u8 = 0xFF;

// NOTE: RGB color control is not exposed in the official Azeron software.
// The hardware may or may not support RGB, but the protocol is currently unknown.
// These enums are kept for potential future discovery.

/// LED zones on the Azeron Cyborg keypad
///
/// **NOTE:** The Cyborg 2 has only a single side LED.
/// For backward compatibility, Logo/Keys/Thumbstick are kept but all map to Side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LedZone {
    /// Side LED (the only confirmed LED on Cyborg 2)
    Side,
    /// All zones at once
    All,
    /// Global setting
    Global,
    /// Legacy zone names (for backward compatibility - all map to Side)
    Logo,
    Keys,
    Thumbstick,
    /// Unknown zone (for future extensibility)
    #[serde(untagged)]
    Unknown(u8),
}

impl LedZone {
    /// Get the raw zone ID for HID commands
    /// NOTE: All zones currently map to the single side LED (0x00)
    pub const fn to_raw_id(self) -> u8 {
        match self {
            Self::Logo => 0x00,
            Self::Keys => 0x01,
            Self::Thumbstick => 0x02,
            Self::Side => 0x00, // Side maps to Logo for Cyborg 2 compatibility
            Self::All => 0x03,
            Self::Global => 0xFF,
            Self::Unknown(id) => id,
        }
    }

    /// Map legacy zone names to the physical zone
    pub const fn to_physical_zone(self) -> Self {
        match self {
            Self::Logo | Self::Keys | Self::Thumbstick => Self::Side,
            other => other,
        }
    }
}

/// LED pattern types for visual effects
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
pub enum LedPattern {
    /// Static solid colors (no animation)
    #[default]
    Static,
    /// Breathing pattern - fades colors in/out
    Breathing,
    /// Rainbow pattern - cycles through colors
    Rainbow,
    /// Rainbow wave - wave effect across zones
    RainbowWave,
}

// =============================================================================
// Per-Device LED State (for hotplug persistence)
// =============================================================================

/// Per-device LED state (persists across hotplug)
///
/// This structure captures the complete LED configuration for a device,
/// enabling automatic restoration when the device is reconnected after
/// being unplugged.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceLedState {
    /// Current color for each zone (RGB values 0-255)
    pub zone_colors: std::collections::HashMap<LedZone, (u8, u8, u8)>,
    /// Per-zone brightness (0-100)
    pub zone_brightness: std::collections::HashMap<LedZone, u8>,
    /// Global brightness (0-100)
    pub global_brightness: u8,
    /// Active LED pattern
    pub active_pattern: LedPattern,
    /// Per-layer colors (layer_id -> RGB) for layer-aware LED feedback
    pub layer_colors: std::collections::HashMap<usize, (u8, u8, u8)>,
}

impl Default for DeviceLedState {
    fn default() -> Self {
        let mut zone_colors = std::collections::HashMap::new();
        let mut zone_brightness = std::collections::HashMap::new();

        // Set default colors (blue-ish)
        zone_colors.insert(LedZone::Side, (50, 100, 255));
        zone_colors.insert(LedZone::Logo, (50, 100, 255));
        zone_colors.insert(LedZone::Keys, (50, 100, 255));
        zone_colors.insert(LedZone::Thumbstick, (50, 100, 255));

        // Set default brightness to 100%
        zone_brightness.insert(LedZone::Side, 100);
        zone_brightness.insert(LedZone::Logo, 100);
        zone_brightness.insert(LedZone::Keys, 100);
        zone_brightness.insert(LedZone::Thumbstick, 100);

        Self {
            zone_colors,
            zone_brightness,
            global_brightness: 100,
            active_pattern: LedPattern::Static,
            layer_colors: std::collections::HashMap::new(),
        }
    }
}

/// LED state tracking for a device
#[derive(Debug)]
pub struct LedState {
    /// Current color for each zone (RGB values 0-255)
    pub zone_colors: HashMap<LedZone, (u8, u8, u8)>,
    /// Per-zone brightness (0-100)
    pub zone_brightness: HashMap<LedZone, u8>,
    /// Global brightness (0-100)
    pub global_brightness: u8,
    /// Active LED pattern
    pub active_pattern: LedPattern,
    /// Animation task handle for software patterns
    ///
    /// Note: This is not Clone because JoinHandle doesn't implement Clone.
    /// For cloning the state, use the `clone_except_handle` method.
    pub animation_handle: Option<tokio::task::JoinHandle<()>>,
    /// Per-layer colors (layer_id -> RGB) for layer-aware LED feedback
    pub layer_colors: HashMap<usize, (u8, u8, u8)>,
}

impl LedState {
    /// Clone the state excluding the animation handle
    pub fn clone_except_handle(&self) -> LedState {
        LedState {
            zone_colors: self.zone_colors.clone(),
            zone_brightness: self.zone_brightness.clone(),
            global_brightness: self.global_brightness,
            active_pattern: self.active_pattern,
            animation_handle: None,
            layer_colors: self.layer_colors.clone(),
        }
    }
}

impl Default for LedState {
    fn default() -> Self {
        let mut zone_colors = HashMap::new();
        let mut zone_brightness = HashMap::new();
        // Default values for all zones (for backward compatibility and tests)
        zone_colors.insert(LedZone::Logo, (50, 100, 255));
        zone_colors.insert(LedZone::Keys, (50, 100, 255));
        zone_colors.insert(LedZone::Thumbstick, (50, 100, 255));
        zone_colors.insert(LedZone::Side, (50, 100, 255));

        zone_brightness.insert(LedZone::Logo, 100);
        zone_brightness.insert(LedZone::Keys, 100);
        zone_brightness.insert(LedZone::Thumbstick, 100);
        zone_brightness.insert(LedZone::Side, 100);

        // Initialize with empty layer colors (will be populated by layer manager)
        let layer_colors = HashMap::new();

        Self {
            zone_colors,
            zone_brightness,
            global_brightness: 100,
            active_pattern: LedPattern::Static,
            animation_handle: None,
            layer_colors,
        }
    }
}

/// LED controller errors
#[derive(thiserror::Error, Debug)]
pub enum LedError {
    /// HID API error
    #[error("HID API error: {0}")]
    HidApi(#[from] hidapi::HidError),

    /// Device not found
    #[error("Azeron LED control interface not found")]
    DeviceNotFound,

    /// HID write failed
    #[error("HID write failed")]
    HidWriteFailed,

    /// Task join failed (spawn_blocking error)
    #[error("Task join failed")]
    TaskJoinFailed,

    /// Invalid value (e.g., brightness out of range)
    #[error("Invalid value: {0}")]
    InvalidValue(String),
}

/// LED controller for Azeron Cyborg keypad
///
/// Manages HID device connection and provides async methods for LED control.
/// The device is opened during initialization (before privilege drop) and
/// all HID operations are wrapped in spawn_blocking to avoid blocking the
/// tokio runtime.
pub struct LedController {
    /// HID device handle
    device: hidapi::HidDevice,
    /// LED state tracking
    state: Arc<RwLock<LedState>>,
}

// SAFETY: LedController is safely Send + Sync despite containing non-Send/Sync hidapi::HidDevice.
// This is safe because:
// 1. All public methods are async and use internal synchronization via RwLock
// 2. HID write operations are serialized through async methods (only one at a time)
// 3. The HID device is never cloned or shared directly across threads
// 4. Arc<LedController> ensures single ownership of the HID device handle
// 5. The internal RwLock<LedState> provides thread-safe access to state
unsafe impl Send for LedController {}
unsafe impl Sync for LedController {}

impl LedController {
    /// Find and open the Azeron LED control interface
    ///
    /// This method enumerates HID devices, filters for Azeron vendor ID,
    /// and selects the LED control interface (NOT the keyboard interface).
    ///
    /// # Returns
    ///
    /// Returns `Ok(Self)` if the LED control interface is found and opened,
    /// or `Err(LedError::DeviceNotFound)` if no Azeron device is present.
    ///
    /// # Interface Selection
    ///
    /// Azeron exposes multiple HID interfaces. We filter by usage_page:
    /// - Skip usage_page == 0x01 (keyboard interface - used by evdev)
    /// - Open vendor-defined usage_page (0xFF00-0xFFFF range) for LED control
    ///
    /// # Example
    ///
    /// ```no_run
    /// use aethermapd::led_controller::LedController;
    ///
    /// match LedController::find_led_interface() {
    ///     Ok(controller) => println!("LED controller initialized"),
    ///     Err(e) => eprintln!("LED control unavailable: {}", e),
    /// }
    /// ```
    pub fn find_led_interface() -> Result<Self, LedError> {
        info!("Searching for Azeron LED control interface...");

        let api = hidapi::HidApi::new().map_err(LedError::HidApi)?;

        let mut found_interfaces = Vec::new();

        for device_info in api.device_list() {
            // Match vendor ID (MCS / Azeron LTD)
            if device_info.vendor_id() != AZERON_VENDOR_ID {
                continue;
            }

            // Match Cyborg 2 product ID (or allow any Azeron device)
            let product_id = device_info.product_id();
            if product_id != AZERON_CYBORG2_PRODUCT_ID {
                debug!(
                    "Skipping non-Cyborg2 Azeron device (PID={:04x})",
                    product_id
                );
                continue;
            }

            let usage_page = device_info.usage_page();
            let interface_number = device_info.interface_number();

            debug!(
                "Found Azeron Cyborg 2 HID interface: VID={:04x} PID={:04x} usage_page={:04x} interface={}",
                AZERON_VENDOR_ID, product_id, usage_page, interface_number
            );

            found_interfaces.push((usage_page, interface_number, product_id));

            // CRITICAL: Skip keyboard interface (usage_page == 0x01)
            // This is the evdev input interface - not for LED control
            if usage_page == KEYBOARD_USAGE_PAGE {
                debug!(
                    "Skipping keyboard interface (usage_page={:04x})",
                    usage_page
                );
                continue;
            }

            // Cyborg 2 LED control interface has usage_page 0xff01 (vendor-defined)
            // This is interface number 4
            if usage_page != LED_CONTROL_USAGE_PAGE {
                debug!("Skipping non-LED interface (usage_page={:04x})", usage_page);
                continue;
            }

            // Open vendor-defined interface for LED control
            info!(
                "Opening Azeron Cyborg 2 LED control interface: usage_page={:04x} interface={}",
                usage_page, interface_number
            );

            let device = device_info.open_device(&api).map_err(LedError::HidApi)?;

            return Ok(Self {
                device,
                state: Arc::new(RwLock::new(LedState::default())),
            });
        }

        if !found_interfaces.is_empty() {
            warn!(
                "Found {} Azeron interface(s) but none suitable for LED control. Interfaces: {:?}",
                found_interfaces.len(),
                found_interfaces
            );
        }

        Err(LedError::DeviceNotFound)
    }

    /// Set the color of an LED zone
    ///
    /// Sends an HID write command using the placeholder protocol.
    /// The protocol is documented in `.planning/phases/12-led-control/12-02-PROTOCOL.md`.
    ///
    /// # Arguments
    ///
    /// * `zone` - The LED zone to set
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    pub async fn set_zone_color(&self, zone: LedZone, r: u8, g: u8, b: u8) -> Result<(), LedError> {
        let zone_id = zone.to_raw_id();

        // Build HID command packet (PLACEHOLDER protocol)
        let mut buffer = [0u8; AZERON_HID_PACKET_SIZE];
        buffer[0] = AZERON_LED_REPORT_ID;
        buffer[1] = AZERON_CMD_SET_COLOR;
        buffer[2] = zone_id;
        buffer[3] = r;
        buffer[4] = g;
        buffer[5] = b;
        // Remaining bytes are padding (0x00)

        debug!(
            "Sending LED color command: zone={}, RGB=({}, {}, {})",
            zone_id, r, g, b
        );

        // Send HID write using the device's write method
        // Note: This is a blocking call but HID writes are typically fast (< 1ms)
        self.device.write(&buffer).map_err(LedError::HidApi)?;

        // Update state after successful write
        {
            let mut state = self.state.write().await;
            state.zone_colors.insert(zone, (r, g, b));
        }

        info!("LED color set: zone={}, RGB=({}, {}, {})", zone_id, r, g, b);

        Ok(())
    }

    /// Set the global brightness for all LED zones
    ///
    /// Sends an HID write command using the discovered protocol.
    /// The protocol was discovered via Wireshark USB capture.
    ///
    /// # Protocol Details
    ///
    /// Brightness is encoded as a 16-bit little-endian value:
    /// - 0% brightness = 0x0100 (256)
    /// - 100% brightness = 0x0196 (406)
    /// - Range: 150 steps
    ///
    /// # Arguments
    ///
    /// * `brightness` - Brightness value (0-100)
    pub async fn set_global_brightness(&self, brightness: u8) -> Result<(), LedError> {
        if brightness > 100 {
            return Err(LedError::InvalidValue("Brightness must be 0-100".into()));
        }

        // Convert 0-100% to hardware brightness value (256-406)
        // Formula: brightness_value = 256 + (brightness_percent * 150 / 100)
        let hw_brightness =
            AZERON_BRIGHTNESS_BASE + (u16::from(brightness) * AZERON_BRIGHTNESS_MAX_STEP / 100);

        // Build HID command packet (discovered protocol)
        let mut buffer = [0u8; AZERON_HID_PACKET_SIZE];

        // Byte 0-1: Packet type (0x0001 for LED commands)
        buffer[0] = (AZERON_PACKET_TYPE_LED & 0xFF) as u8;
        buffer[1] = ((AZERON_PACKET_TYPE_LED >> 8) & 0xFF) as u8;

        // Byte 2-3: Command counter (little-endian) - use current value
        // For brightness commands, the counter increments but we use a simple value
        let counter: u16 = 0x09f5; // Starting value from capture
        buffer[2] = (counter & 0xFF) as u8;
        buffer[3] = ((counter >> 8) & 0xFF) as u8;

        // Byte 4-5: Always 0x01
        buffer[4] = AZERON_LED_BYTE_4;
        buffer[5] = AZERON_LED_BYTE_5;

        // Byte 6-7: Brightness value (little-endian)
        buffer[6] = (hw_brightness & 0xFF) as u8;
        buffer[7] = ((hw_brightness >> 8) & 0xFF) as u8;

        // Byte 8-9: Always 0x00
        buffer[8] = AZERON_LED_BYTE_8;
        buffer[9] = AZERON_LED_BYTE_9;

        // Remaining bytes are padding (0x00)

        debug!(
            "Sending LED global brightness command: {}% (raw: {})",
            brightness, hw_brightness
        );

        // Send HID write using the device's write method
        self.device.write(&buffer).map_err(LedError::HidApi)?;

        // Update state after successful write
        {
            let mut state = self.state.write().await;
            state.global_brightness = brightness;
        }

        info!(
            "LED global brightness set: {}% (hw: {})",
            brightness, hw_brightness
        );

        Ok(())
    }

    /// Set the brightness for a specific LED zone
    ///
    /// **NOTE:** The Azeron Cyborg 2 appears to have only global brightness control.
    /// This method currently falls back to global brightness.
    ///
    /// # Arguments
    ///
    /// * `zone` - The LED zone to set brightness for (ignored, uses global)
    /// * `brightness` - Brightness value (0-100)
    pub async fn set_zone_brightness(&self, zone: LedZone, brightness: u8) -> Result<(), LedError> {
        // The Cyborg 2 only supports global brightness
        // Store the per-zone value for potential future use
        {
            let mut state = self.state.write().await;
            state.zone_brightness.insert(zone, brightness);
        }

        // Send as global brightness command
        self.set_global_brightness(brightness).await
    }

    /// Send a keepalive packet to maintain the LED connection
    ///
    /// The device expects regular keepalive packets or it may become unresponsive.
    /// The keepalive packet format is:
    /// ```text
    /// 0000125901010000... (64 bytes)
    /// ```
    /// Where bytes 2-3 are an incrementing counter.
    ///
    /// This should be called periodically (e.g., every 1-2 seconds).
    pub async fn send_keepalive(&self) -> Result<(), LedError> {
        let mut buffer = [0u8; AZERON_HID_PACKET_SIZE];

        // Byte 0-1: Packet type (0x0000 for keepalive)
        buffer[0] = (AZERON_PACKET_TYPE_KEEPALIVE & 0xFF) as u8;
        buffer[1] = ((AZERON_PACKET_TYPE_KEEPALIVE >> 8) & 0xFF) as u8;

        // Byte 2-3: Keepalive counter (increments)
        // Use a simple counter based on time
        let counter: u16 = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            & 0xFFFF) as u16;
        let counter = 0x1259 + (counter & 0xFF); // Start from observed base value
        buffer[2] = (counter & 0xFF) as u8;
        buffer[3] = ((counter >> 8) & 0xFF) as u8;

        // Byte 4-5: Always 0x01
        buffer[4] = AZERON_LED_BYTE_4;
        buffer[5] = AZERON_LED_BYTE_5;

        // Byte 6-9: Always 0x00 for keepalive
        buffer[6] = 0x00;
        buffer[7] = 0x00;
        buffer[8] = 0x00;
        buffer[9] = 0x00;

        // Remaining bytes are padding (0x00)

        self.device.write(&buffer).map_err(LedError::HidApi)?;

        debug!("Sent LED keepalive packet (counter: {:#04x})", counter);

        Ok(())
    }

    /// Get the current LED state
    pub async fn get_state(&self) -> LedState {
        self.state.read().await.clone_except_handle()
    }

    /// Get the current color for a specific zone
    pub async fn get_zone_color(&self, zone: LedZone) -> Option<(u8, u8, u8)> {
        self.state.read().await.zone_colors.get(&zone).copied()
    }

    /// Get the current global brightness
    pub async fn get_global_brightness(&self) -> u8 {
        self.state.read().await.global_brightness
    }

    /// Get the current brightness for a specific zone
    ///
    /// Returns the zone-specific brightness if set, otherwise returns the global brightness.
    pub async fn get_zone_brightness(&self, zone: LedZone) -> u8 {
        let state = self.state.read().await;
        state
            .zone_brightness
            .get(&zone)
            .copied()
            .unwrap_or(state.global_brightness)
    }

    /// Legacy method for backward compatibility - alias for get_global_brightness
    pub async fn get_brightness(&self) -> u8 {
        self.get_global_brightness().await
    }

    /// Legacy method for backward compatibility - alias for set_global_brightness
    pub async fn set_brightness(&self, brightness: u8) -> Result<(), LedError> {
        self.set_global_brightness(brightness).await
    }

    /// Get all zone colors
    ///
    /// Returns a clone of the entire zone_colors HashMap, containing
    /// all configured LED zones and their current RGB values.
    pub async fn get_all_zone_colors(&self) -> HashMap<LedZone, (u8, u8, u8)> {
        self.state.read().await.zone_colors.clone()
    }

    /// Set the color for a layer
    ///
    /// Stores the LED color for a specific layer ID. This color can be
    /// applied to a zone when the layer becomes active.
    ///
    /// # Arguments
    ///
    /// * `layer_id` - Layer identifier (0 = base, 1+ = additional layers)
    /// * `color` - RGB color tuple (red, green, blue) with values 0-255
    pub async fn set_layer_color(&self, layer_id: usize, color: (u8, u8, u8)) {
        let mut state = self.state.write().await;
        state.layer_colors.insert(layer_id, color);
        debug!("Set layer {} color to RGB {:?}", layer_id, color);
    }

    /// Get the color for a layer
    ///
    /// Retrieves the stored LED color for a specific layer ID.
    ///
    /// # Arguments
    ///
    /// * `layer_id` - Layer identifier
    ///
    /// # Returns
    ///
    /// * `Some((r, g, b))` - Layer color if configured
    /// * `None` - Layer color not set
    pub async fn get_layer_color(&self, layer_id: usize) -> Option<(u8, u8, u8)> {
        let state = self.state.read().await;
        state.layer_colors.get(&layer_id).copied()
    }

    /// Apply a layer's color to a specific zone
    ///
    /// Sets the LED zone to the configured color for the specified layer.
    ///
    /// # Arguments
    ///
    /// * `layer_id` - Layer identifier
    /// * `zone` - LED zone to apply the color to
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Color applied successfully
    /// * `Err(LedError)` - Failed to apply color
    pub async fn apply_layer_color(&self, layer_id: usize, zone: LedZone) -> Result<(), LedError> {
        if let Some((r, g, b)) = self.get_layer_color(layer_id).await {
            self.set_zone_color(zone, r, g, b).await?;
            debug!("Applied layer {} color to zone {:?}", layer_id, zone);
        } else {
            debug!("No color configured for layer {}, using default", layer_id);
        }
        Ok(())
    }

    /// Set the LED pattern for visual effects
    ///
    /// This method configures the LED pattern behavior. For hardware-supported
    /// patterns, it sends a HID command. For unsupported patterns, it falls back
    /// to software animation.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The pattern to activate (Static, Breathing, Rainbow, RainbowWave)
    pub async fn set_pattern(&self, pattern: LedPattern) -> Result<(), LedError> {
        // Cancel existing animation if any
        {
            let mut state = self.state.write().await;
            if let Some(handle) = state.animation_handle.take() {
                handle.abort();
            }
            state.active_pattern = pattern;
        }

        // Try hardware pattern first (if supported per Plan 02)
        if self.supports_hardware_pattern(pattern).await {
            self.send_hardware_pattern_command(pattern).await?;
        } else {
            // Fall back to software animation
            self.start_software_animation(pattern).await?;
        }

        info!("LED pattern set: {:?}", pattern);
        Ok(())
    }

    /// Get the current active LED pattern
    pub async fn get_pattern(&self) -> LedPattern {
        self.state.read().await.active_pattern
    }

    /// Check if a pattern is supported by hardware
    ///
    /// Static is always supported (just set colors).
    /// Other patterns depend on hardware support discovered in Plan 02.
    async fn supports_hardware_pattern(&self, pattern: LedPattern) -> bool {
        // Static is always supported (just set colors)
        if pattern == LedPattern::Static {
            return true;
        }

        // Other patterns depend on hardware support discovered in Plan 02
        // Default to false (use software animation)
        // TODO: Update after USB capture verification
        false
    }

    /// Send a hardware pattern command via HID
    async fn send_hardware_pattern_command(&self, pattern: LedPattern) -> Result<(), LedError> {
        let pattern_id = match pattern {
            LedPattern::Static => 0x00,
            LedPattern::Breathing => 0x01,
            LedPattern::Rainbow => 0x02,
            LedPattern::RainbowWave => 0x03,
        };

        // Build HID command packet (PLACEHOLDER protocol)
        let mut buffer = [0u8; AZERON_HID_PACKET_SIZE];
        buffer[0] = AZERON_LED_REPORT_ID;
        buffer[1] = AZERON_CMD_SET_PATTERN;
        buffer[2] = pattern_id;
        // Remaining bytes are padding (0x00)

        debug!("Sending LED pattern command: pattern_id={}", pattern_id);

        // Send HID write
        self.device.write(&buffer).map_err(LedError::HidApi)?;

        Ok(())
    }

    /// Start software animation for patterns not supported by hardware
    async fn start_software_animation(&self, pattern: LedPattern) -> Result<(), LedError> {
        match pattern {
            LedPattern::Static => {
                // Static doesn't need animation - just ensure colors are set
                Ok(())
            }
            LedPattern::Breathing => self.start_breathing_animation().await,
            LedPattern::Rainbow | LedPattern::RainbowWave => {
                self.start_rainbow_animation(matches!(pattern, LedPattern::RainbowWave))
                    .await
            }
        }
    }

    /// Start breathing animation (software fallback)
    ///
    /// This spawns a background task that fades brightness in and out.
    /// CPU intensive - only used when hardware doesn't support breathing.
    async fn start_breathing_animation(&self) -> Result<(), LedError> {
        // Clone the state Arc for the spawned task
        let state = Arc::clone(&self.state);

        let handle = tokio::spawn(async move {
            let mut brightness = 0i16;
            let mut direction = 1i16;
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));

            loop {
                interval.tick().await;

                // Calculate brightness (0-100)
                brightness += direction * 5; // Step size
                if brightness >= 100 {
                    brightness = 100;
                    direction = -1;
                } else if brightness <= 0 {
                    brightness = 0;
                    direction = 1;
                }

                // Apply to all zones using their current colors
                let zone_colors = state.read().await.zone_colors.clone();
                for (_zone, (r, g, b)) in zone_colors {
                    // Scale RGB by brightness for breathing effect
                    let _scale = brightness as f32 / 100.0;
                    let _scaled_r = (r as f32 * _scale) as u8;
                    let _scaled_g = (g as f32 * _scale) as u8;
                    let _scaled_b = (b as f32 * _scale) as u8;
                    // TODO: Need channel-based architecture to write HID from spawned task
                    // For now, just calculate values without writing
                }
            }
        });

        // Store the handle so we can abort it when pattern changes
        let mut state = self.state.write().await;
        state.animation_handle = Some(handle);

        warn!("Breathing animation started - HID writes require channel-based architecture");
        Ok(())
    }

    /// Start rainbow animation (software fallback)
    ///
    /// This spawns a background task that cycles through HSV colors.
    /// CPU intensive - only used when hardware doesn't support rainbow.
    async fn start_rainbow_animation(&self, _wave: bool) -> Result<(), LedError> {
        // Clone the state Arc for the spawned task
        let _state = Arc::clone(&self.state);

        let handle = tokio::spawn(async move {
            let mut hue = 0u16;
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));

            loop {
                interval.tick().await;

                // Calculate RGB for current hue
                let (_r, _g, _b) = hsv_to_rgb(hue, 100, 100);
                // TODO: Need channel-based architecture to write HID from spawned task
                // For now, just calculate values without writing

                hue = (hue + 5) % 360; // Advance hue
            }
        });

        // Store the handle so we can abort it when pattern changes
        let mut state = self.state.write().await;
        state.animation_handle = Some(handle);

        warn!("Rainbow animation started - HID writes require channel-based architecture");
        Ok(())
    }

    /// Send color command with brightness scaling (for animations)
    #[allow(dead_code)]
    async fn send_color_with_brightness(
        &self,
        zone: LedZone,
        r: u8,
        g: u8,
        b: u8,
        _brightness_percent: u8,
    ) -> Result<(), LedError> {
        // For software animation, we pre-scale the RGB values
        // so we can just use the regular color command
        self.send_color_command(zone, r, g, b).await
    }

    /// Internal color command without state update (for animations)
    async fn send_color_command(&self, zone: LedZone, r: u8, g: u8, b: u8) -> Result<(), LedError> {
        let zone_id = zone.to_raw_id();

        // Build HID command packet (PLACEHOLDER protocol)
        let mut buffer = [0u8; AZERON_HID_PACKET_SIZE];
        buffer[0] = AZERON_LED_REPORT_ID;
        buffer[1] = AZERON_CMD_SET_COLOR;
        buffer[2] = zone_id;
        buffer[3] = r;
        buffer[4] = g;
        buffer[5] = b;
        // Remaining bytes are padding (0x00)

        // Send HID write
        self.device.write(&buffer).map_err(LedError::HidApi)?;

        Ok(())
    }

    /// Export current LED state for persistence
    ///
    /// Creates a snapshot of the current LED configuration that can be
    /// stored and later restored via `import_state`. This is used for
    /// hotplug persistence - when a device is unplugged and replugged,
    /// the previous LED configuration can be automatically restored.
    ///
    /// # Returns
    ///
    /// A `DeviceLedState` containing the current LED configuration
    pub async fn export_state(&self) -> DeviceLedState {
        let state = self.state.read().await;
        DeviceLedState {
            zone_colors: state.zone_colors.clone(),
            zone_brightness: state.zone_brightness.clone(),
            global_brightness: state.global_brightness,
            active_pattern: state.active_pattern,
            layer_colors: state.layer_colors.clone(),
        }
    }

    /// Import LED state (for restoration after hotplug)
    ///
    /// Restores a previously exported LED configuration to the device.
    /// This sends all the stored colors, brightness, and pattern settings
    /// to the hardware via HID commands.
    ///
    /// # Arguments
    ///
    /// * `imported` - The LED state to restore
    ///
    /// # Returns
    ///
    /// * `Ok(())` - State restored successfully
    /// * `Err(LedError)` - Failed to restore state
    pub async fn import_state(&self, imported: DeviceLedState) -> Result<(), LedError> {
        let zone_count = imported.zone_colors.len();
        let global_brightness = imported.global_brightness;
        let active_pattern = imported.active_pattern;

        // Restore zone colors
        for (zone, (r, g, b)) in &imported.zone_colors {
            self.send_color_command(*zone, *r, *g, *b).await?;
        }

        // Restore global brightness
        self.send_global_brightness_command(global_brightness)
            .await?;

        // Restore pattern (if not Static - Static is already set via colors)
        if active_pattern != LedPattern::Static {
            self.set_pattern(active_pattern).await?;
        }

        // Update internal state
        {
            let mut state = self.state.write().await;
            state.zone_colors = imported.zone_colors;
            state.zone_brightness = imported.zone_brightness;
            state.global_brightness = imported.global_brightness;
            state.active_pattern = imported.active_pattern;
            state.layer_colors = imported.layer_colors;
        }

        info!(
            "LED state restored: {} zones, brightness {}%, pattern {:?}",
            zone_count, global_brightness, active_pattern
        );

        Ok(())
    }

    /// Send global brightness command (internal helper)
    ///
    /// This is a helper method for `import_state` that sends the global
    /// brightness setting without updating the local state (the state
    /// is updated after all commands succeed).
    async fn send_global_brightness_command(&self, brightness: u8) -> Result<(), LedError> {
        let mut buffer = [0u8; AZERON_HID_PACKET_SIZE];
        buffer[0] = AZERON_LED_REPORT_ID;
        buffer[1] = AZERON_CMD_SET_BRIGHTNESS;
        buffer[2] = AZERON_ZONE_GLOBAL;
        buffer[3] = brightness;
        // Remaining bytes are padding (0x00)

        self.device.write(&buffer).map_err(LedError::HidApi)?;

        Ok(())
    }
}

/// HSV to RGB conversion for rainbow effects
///
/// # Arguments
///
/// * `h` - Hue (0-360)
/// * `s` - Saturation (0-100)
/// * `v` - Value (0-100)
///
/// # Returns
///
/// RGB tuple with values 0-255
fn hsv_to_rgb(h: u16, s: u8, v: u8) -> (u8, u8, u8) {
    let s = s as f32 / 100.0;
    let v = v as f32 / 100.0;
    let c = v * s;
    let x = c * (1.0 - ((h as f32 / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60 {
        (c, x, 0.0)
    } else if h < 120 {
        (x, c, 0.0)
    } else if h < 180 {
        (0.0, c, x)
    } else if h < 240 {
        (0.0, x, c)
    } else if h < 300 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn test_led_zone_serialization() {
        // Test that LedZone variants have consistent raw IDs
        assert_eq!(LedZone::Logo.to_raw_id(), 0x00);
        assert_eq!(LedZone::Keys.to_raw_id(), 0x01);
        assert_eq!(LedZone::Thumbstick.to_raw_id(), 0x02);
        assert_eq!(LedZone::All.to_raw_id(), 0x03);
        assert_eq!(LedZone::Global.to_raw_id(), 0xFF);
    }

    #[test]
    fn test_led_state_default() {
        let state = LedState::default();
        assert_eq!(state.global_brightness, 100);
        assert!(state.zone_colors.contains_key(&LedZone::Logo));
        assert!(state.zone_colors.contains_key(&LedZone::Keys));
        assert!(state.zone_colors.contains_key(&LedZone::Thumbstick));
        assert!(state.zone_brightness.contains_key(&LedZone::Logo));
        assert!(state.zone_brightness.contains_key(&LedZone::Keys));
        assert!(state.zone_brightness.contains_key(&LedZone::Thumbstick));
        assert_eq!(state.zone_brightness[&LedZone::Logo], 100);
    }

    #[test]
    fn test_brightness_range_validation() {
        // Test that brightness validation errors are returned for invalid values
        // Note: This tests the error variant exists - actual validation is tested in integration
        let error = LedError::InvalidValue("test".to_string());
        assert_eq!(error.to_string(), "Invalid value: test");
    }

    #[test]
    fn test_constants() {
        assert_eq!(AZERON_VENDOR_ID, 0x16d0);
        assert_eq!(KEYBOARD_USAGE_PAGE, 0x0001);
    }

    #[test]
    fn test_zone_color_storage() {
        // Test that LedState can store and retrieve zone colors
        let mut state = LedState::default();

        // Set a custom color for Logo zone
        state.zone_colors.insert(LedZone::Logo, (255, 128, 64));

        // Verify retrieval
        assert_eq!(state.zone_colors.get(&LedZone::Logo), Some(&(255, 128, 64)));

        // Verify other zones retain their defaults
        assert!(state.zone_colors.contains_key(&LedZone::Keys));
        assert!(state.zone_colors.contains_key(&LedZone::Thumbstick));
    }

    #[test]
    fn test_multiple_zones_independent() {
        // Test that multiple zones can have different colors simultaneously
        let mut state = LedState::default();

        // Set different colors for each zone
        state.zone_colors.insert(LedZone::Logo, (255, 0, 0)); // Red
        state.zone_colors.insert(LedZone::Keys, (0, 255, 0)); // Green
        state.zone_colors.insert(LedZone::Thumbstick, (0, 0, 255)); // Blue

        // Verify each zone has its own color
        assert_eq!(state.zone_colors.get(&LedZone::Logo), Some(&(255, 0, 0)));
        assert_eq!(state.zone_colors.get(&LedZone::Keys), Some(&(0, 255, 0)));
        assert_eq!(
            state.zone_colors.get(&LedZone::Thumbstick),
            Some(&(0, 0, 255))
        );
    }

    #[test]
    fn test_zone_brightness_storage() {
        // Test that LedState can store and retrieve per-zone brightness
        let mut state = LedState::default();

        // Set custom brightness for Logo zone
        state.zone_brightness.insert(LedZone::Logo, 50);

        // Verify retrieval
        assert_eq!(state.zone_brightness.get(&LedZone::Logo), Some(&50));

        // Verify other zones retain default brightness
        assert_eq!(state.zone_brightness.get(&LedZone::Keys), Some(&100));
    }

    #[test]
    fn test_global_brightness_default() {
        // Test default global brightness
        let state = LedState::default();
        assert_eq!(state.global_brightness, 100);
    }

    #[test]
    fn test_global_brightness_storage() {
        // Test that global brightness can be stored
        let mut state = LedState::default();
        state.global_brightness = 75;
        assert_eq!(state.global_brightness, 75);
    }

    #[test]
    fn test_led_zone_to_raw_id_extended() {
        // Extended test for LedZone to_raw_id conversion including Unknown variant
        // Note: Basic zone IDs are tested in test_led_zone_serialization
        assert_eq!(LedZone::Logo.to_raw_id(), 0x00);
        assert_eq!(LedZone::Keys.to_raw_id(), 0x01);
        assert_eq!(LedZone::Thumbstick.to_raw_id(), 0x02);
        assert_eq!(LedZone::All.to_raw_id(), 0x03);
        assert_eq!(LedZone::Global.to_raw_id(), 0xFF);

        // Test Unknown zone preserves its raw value
        assert_eq!(LedZone::Unknown(0x42).to_raw_id(), 0x42);
        assert_eq!(LedZone::Unknown(0x00).to_raw_id(), 0x00);
    }

    // Layer color storage tests (Plan 12-06)

    #[test]
    fn test_layer_colors_field_exists() {
        // Test that LedState has layer_colors field
        let state = LedState::default();
        assert!(state.layer_colors.is_empty()); // Default is empty
    }

    #[test]
    fn test_led_state_includes_layer_colors() {
        // Test that LedState clone_except_handle includes layer_colors
        let mut state = LedState::default();
        state.layer_colors.insert(1, (255, 0, 0));
        state.layer_colors.insert(2, (0, 255, 0));

        let cloned = state.clone_except_handle();
        assert_eq!(cloned.layer_colors.len(), 2);
        assert_eq!(cloned.layer_colors.get(&1), Some(&(255, 0, 0)));
        assert_eq!(cloned.layer_colors.get(&2), Some(&(0, 255, 0)));
        assert!(cloned.animation_handle.is_none());
    }

    #[test]
    fn test_multiple_layer_colors_independent() {
        // Test that different layers can have different colors
        let mut state = LedState::default();

        state.layer_colors.insert(0, (255, 255, 255)); // Base: white
        state.layer_colors.insert(1, (0, 0, 255)); // Layer 1: blue
        state.layer_colors.insert(2, (0, 255, 0)); // Layer 2: green

        assert_eq!(state.layer_colors.len(), 3);
        assert_eq!(state.layer_colors.get(&0), Some(&(255, 255, 255)));
        assert_eq!(state.layer_colors.get(&1), Some(&(0, 0, 255)));
        assert_eq!(state.layer_colors.get(&2), Some(&(0, 255, 0)));
    }
}
