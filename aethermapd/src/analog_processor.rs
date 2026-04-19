//! AnalogProcessor for EV_ABS event handling with deadzone filtering
//!
//! This module provides analog stick processing with:
//! - Deadzone filtering (values within deadzone return None)
//! - Sensitivity adjustment (multiplier for output values)
//! - Response curve application (Linear, Exponential)
//! - Per-device calibration state tracking
//!
//! # Processing Pipeline
//!
//! 1. **Deadzone Filter**: If |value| < deadzone, return None (no event)
//! 2. **Normalization**: Map (deadzone, max) to (0, max)
//! 3. **Sensitivity**: Apply multiplier (default 1.0)
//! 4. **Response Curve**: Apply Linear or Exponential transformation
//!
//! # Device Configurations
//!
//! Each device has independent analog configuration:
//! - deadzone: 0-32767 range (default ~14000 for ~43%)
//! - sensitivity: 0.1-5.0 multiplier (default 1.0)
//! - response_curve: Linear or Exponential
//!
//! # Examples
//!
//! ```ignore
//! let processor = AnalogProcessor::new();
//!
//! // Process analog event
//! if let Some(processed) = processor.process_event("1532:0220", 61000, 25000).await {
//!     // Send processed value to macro engine
//! } else {
//!     // Value was filtered by deadzone
//! }
//!
//! // Configure device
//! processor.set_deadzone("1532:0220", 16000).await;
//! processor.set_sensitivity("1532:0220", 1.5).await;
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use evdev::Key;

// Import analog calibration types for 2D processing
use crate::analog_calibration::{AnalogCalibration, DeadzoneShape, SensitivityCurve};

/// Default deadzone value (~43% of 32767 range)
///
/// This provides a comfortable deadzone for typical analog sticks.
/// Users can adjust per-device via set_deadzone().
const DEFAULT_DEADZONE: u16 = 14000;

/// Maximum absolute value for evdev analog events
pub const MAX_ABS_VALUE: i32 = 32767;

/// Response curve type for analog processing
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ResponseCurve {
    /// Linear response (no transformation)
    Linear,
    /// Exponential response (f(x) = sign(x) * |x|^exponent)
    Exponential { exponent: f32 },
}

/// D-pad emulation mode for analog sticks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DpadMode {
    /// D-pad emulation disabled (analog passes through normally)
    Disabled,
    /// 8-way D-pad (N, NE, E, SE, S, SW, W, NW)
    EightWay,
    /// 4-way D-pad (N, E, S, W only - diagonals ignored)
    FourWay,
}

impl Default for DpadMode {
    fn default() -> Self {
        Self::Disabled
    }
}

/// Direction for D-pad emulation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Default for Direction {
    fn default() -> Self {
        Self::Up
    }
}

/// Arrow key codes for D-pad emulation
const DPAD_UP: u16 = 103;    // KEY_UP
const DPAD_DOWN: u16 = 108;  // KEY_DOWN
const DPAD_LEFT: u16 = 105;  // KEY_LEFT
const DPAD_RIGHT: u16 = 106; // KEY_RIGHT

/// 8-way D-pad direction for analog stick to D-pad conversion
///
/// Represents all possible directions from an analog stick converted to D-pad output.
/// Diagonal directions return two key codes (e.g., UpRight = Up + Right).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpadDirection {
    /// No direction (stick centered or within deadzone)
    None,
    /// Up (Y axis negative)
    Up,
    /// Up and to the right
    UpRight,
    /// Right (X axis positive)
    Right,
    /// Down and to the right
    DownRight,
    /// Down (Y axis positive)
    Down,
    /// Down and to the left
    DownLeft,
    /// Left (X axis negative)
    Left,
    /// Up and to the left
    UpLeft,
}

impl Default for DpadDirection {
    fn default() -> Self {
        Self::None
    }
}

/// Analog output mode determines how processed stick values are emitted
///
/// - Disabled: No output (pass-through only)
/// - Dpad: 8-way directional output as arrow keys
/// - Gamepad: Xbox 360 compatible gamepad axis output
/// - Camera: Mouse movement or scroll events (Phase 15)
/// - Mouse: Velocity-based cursor movement (Phase 15)
/// - WASD: Directional key output (Phase 15)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnalogMode {
    /// No output (pass-through only)
    Disabled,
    /// D-pad mode - 8-way directional keys
    Dpad,
    /// Gamepad mode - Xbox 360 compatible axis output
    Gamepad,
    /// Camera mode - scroll or key repeat (Phase 15)
    Camera,
    /// Mouse mode - velocity-based cursor (Phase 15)
    Mouse,
    /// WASD mode - directional keys (Phase 15)
    Wasd,
}

impl Default for AnalogMode {
    fn default() -> Self {
        Self::Disabled
    }
}

/// Convert direction to arrow key code
pub fn direction_to_key_code(dir: Direction) -> u16 {
    match dir {
        Direction::Up => DPAD_UP,
        Direction::Down => DPAD_DOWN,
        Direction::Left => DPAD_LEFT,
        Direction::Right => DPAD_RIGHT,
    }
}

/// Convert D-pad direction to Linux key codes
///
/// Diagonal directions return two keys (e.g., UpRight = Up + Right).
/// Cardinal directions return a single key. None returns empty vector.
///
/// # Arguments
///
/// * `direction` - D-pad direction to convert
///
/// # Returns
///
/// Vec of Key codes for the direction (0-2 keys)
pub fn dpad_direction_to_keys(direction: DpadDirection) -> Vec<Key> {
    match direction {
        DpadDirection::None => vec![],
        DpadDirection::Up => vec![Key::KEY_UP],
        DpadDirection::Down => vec![Key::KEY_DOWN],
        DpadDirection::Left => vec![Key::KEY_LEFT],
        DpadDirection::Right => vec![Key::KEY_RIGHT],
        DpadDirection::UpLeft => vec![Key::KEY_UP, Key::KEY_LEFT],
        DpadDirection::UpRight => vec![Key::KEY_UP, Key::KEY_RIGHT],
        DpadDirection::DownLeft => vec![Key::KEY_DOWN, Key::KEY_LEFT],
        DpadDirection::DownRight => vec![Key::KEY_DOWN, Key::KEY_RIGHT],
    }
}

/// Convert D-pad direction to WASD key codes
///
/// Diagonal directions return two keys (e.g., UpRight = W + D).
/// Cardinal directions return a single key. None returns empty vector.
///
/// This matches the standard PC gaming control scheme where:
/// - W = forward/up
/// - A = left/strafe left
/// - S = backward/down
/// - D = right/strafe right
///
/// Diagonal combinations allow for simultaneous movement (e.g., W+D = forward-right).
///
/// # Arguments
///
/// * `direction` - D-pad direction to convert
///
/// # Returns
///
/// Vec of WASD Key codes for the direction (0-2 keys)
pub fn wasd_direction_to_keys(direction: DpadDirection) -> Vec<Key> {
    match direction {
        DpadDirection::None => vec![],
        DpadDirection::Up => vec![Key::KEY_W],
        DpadDirection::Down => vec![Key::KEY_S],
        DpadDirection::Left => vec![Key::KEY_A],
        DpadDirection::Right => vec![Key::KEY_D],
        DpadDirection::UpLeft => vec![Key::KEY_W, Key::KEY_A],
        DpadDirection::UpRight => vec![Key::KEY_W, Key::KEY_D],
        DpadDirection::DownLeft => vec![Key::KEY_S, Key::KEY_A],
        DpadDirection::DownRight => vec![Key::KEY_S, Key::KEY_D],
    }
}

/// Convert D-pad direction to camera control keys
///
/// Maps 8-way D-pad directions to camera navigation keys:
/// - Vertical: PageUp/PageDown for scrolling/panning
/// - Horizontal: Arrow keys for left/right
/// - Diagonals: Key combinations (e.g., UpLeft = PageUp + Left)
///
/// This matches common camera controls in 3D applications and games:
/// - PageUp/PageDown for vertical movement (zoom/pan up-down)
/// - Arrow keys for horizontal movement (pan left-right)
/// - Diagonal combinations allow simultaneous multi-axis camera movement
///
/// # Arguments
///
/// * `direction` - D-pad direction to convert
///
/// # Returns
///
/// Vec of camera Key codes for the direction (0-2 keys)
pub fn camera_direction_to_keys(direction: DpadDirection) -> Vec<Key> {
    match direction {
        DpadDirection::None => vec![],
        DpadDirection::Up => vec![Key::KEY_PAGEUP],
        DpadDirection::Down => vec![Key::KEY_PAGEDOWN],
        DpadDirection::Left => vec![Key::KEY_LEFT],
        DpadDirection::Right => vec![Key::KEY_RIGHT],
        DpadDirection::UpLeft => vec![Key::KEY_PAGEUP, Key::KEY_LEFT],
        DpadDirection::UpRight => vec![Key::KEY_PAGEUP, Key::KEY_RIGHT],
        DpadDirection::DownLeft => vec![Key::KEY_PAGEDOWN, Key::KEY_LEFT],
        DpadDirection::DownRight => vec![Key::KEY_PAGEDOWN, Key::KEY_RIGHT],
    }
}

/// Camera mode output type (user-selectable)
///
/// Controls how analog stick input is converted in Camera mode:
/// - Scroll: Emits REL_WHEEL events for document/webpage scrolling
/// - Keys: Emits key repeat events (PageUp/PageDown/arrow keys) for 3D camera rotation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraOutputMode {
    /// Emit REL_WHEEL events (Y-axis scroll only)
    Scroll,
    /// Emit key repeat events (PageUp/PageDown for vertical, arrow keys for horizontal)
    Keys,
}

impl Default for CameraOutputMode {
    fn default() -> Self {
        Self::Scroll  // Default to scroll for broader compatibility
    }
}

/// Camera mode output value
///
/// Return type for `process_as_camera()`, containing either:
/// - Scroll(i32): Scroll amount for REL_WHEEL event (vertical only)
/// - Keys(Vec<Key>): Keys to emit for key repeat mode (supports diagonals)
pub enum CameraOutput {
    /// Scroll amount for REL_WHEEL event (Y-axis only)
    Scroll(i32),
    /// Keys to emit for key repeat mode (PageUp/PageDown/arrow combinations)
    Keys(Vec<Key>),
}

impl Default for ResponseCurve {
    fn default() -> Self {
        Self::Linear
    }
}

/// Per-device analog configuration
///
/// Stores deadzone, sensitivity, and response curve settings for a single device.
/// Devices are identified by device_id (vendor:product format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAnalogConfig {
    /// Device identifier (vendor:product format, e.g., "1532:0220")
    pub device_id: String,

    /// Deadzone threshold (0-32767 range) - DEPRECATED, use inner_deadzone_x/y
    /// Values within +/- deadzone return None (filtered)
    #[serde(default = "default_deadzone")]
    pub deadzone: u16,

    /// Sensitivity multiplier (0.1-5.0, default 1.0)
    #[serde(default = "default_sensitivity")]
    pub sensitivity: f32,

    /// Response curve for output transformation
    #[serde(default)]
    pub response_curve: ResponseCurve,

    /// D-pad emulation mode (configures HOW 8-way detection works)
    #[serde(default)]
    pub dpad_mode: DpadMode,

    /// Analog output mode (selects OUTPUT behavior: Dpad, Gamepad, Camera, Mouse, WASD)
    #[serde(default)]
    pub mode: AnalogMode,

    // Per-axis deadzone fields (plan 11-06)

    /// Inner deadzone for X-axis (noise filtering near center)
    #[serde(default = "default_deadzone")]
    pub inner_deadzone_x: u16,

    /// Inner deadzone for Y-axis (noise filtering near center)
    #[serde(default = "default_deadzone")]
    pub inner_deadzone_y: u16,

    /// Outer deadzone for X-axis (max input clamping)
    #[serde(default = "default_outer_deadzone")]
    pub outer_deadzone_x: u16,

    /// Outer deadzone for Y-axis (max input clamping)
    #[serde(default = "default_outer_deadzone")]
    pub outer_deadzone_y: u16,
}

fn default_deadzone() -> u16 {
    DEFAULT_DEADZONE
}

fn default_sensitivity() -> f32 {
    1.0
}

fn default_outer_deadzone() -> u16 {
    MAX_ABS_VALUE as u16  // No clamping by default
}

/// Mouse velocity configuration for analog-to-mouse processing
///
/// Controls how analog stick deflection translates to cursor velocity.
/// The multiplier determines cursor speed - higher values mean faster
/// cursor movement for the same stick deflection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseVelocityConfig {
    /// Velocity multiplier (pixels per unit analog deflection)
    ///
    /// Default: 10.0 pixels per 1.0 analog value (~10 pixels at full deflection)
    #[serde(default = "default_mouse_multiplier")]
    pub multiplier: f32,
}

fn default_mouse_multiplier() -> f32 {
    10.0  // 10 pixels per 1.0 analog value
}

impl Default for MouseVelocityConfig {
    fn default() -> Self {
        Self {
            multiplier: default_mouse_multiplier(),
        }
    }
}

/// Default mouse velocity configuration
///
/// Provides a sensible default when processing mouse events without explicit configuration.
/// Returns a MouseVelocityConfig with multiplier 10.0 (10 pixels per unit deflection).
pub fn default_mouse_velocity_config() -> MouseVelocityConfig {
    MouseVelocityConfig {
        multiplier: 10.0,  // 10 pixels per unit deflection
    }
}

impl DeviceAnalogConfig {
    /// Create a new device analog config with defaults
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            deadzone: DEFAULT_DEADZONE,
            sensitivity: 1.0,
            response_curve: ResponseCurve::Linear,
            dpad_mode: DpadMode::Disabled,
            mode: AnalogMode::Disabled,
            inner_deadzone_x: DEFAULT_DEADZONE,
            inner_deadzone_y: DEFAULT_DEADZONE,
            outer_deadzone_x: MAX_ABS_VALUE as u16,
            outer_deadzone_y: MAX_ABS_VALUE as u16,
        }
    }

    /// Create a new device analog config with specific deadzone
    pub fn with_deadzone(device_id: String, deadzone: u16) -> Self {
        Self {
            device_id,
            deadzone,
            sensitivity: 1.0,
            response_curve: ResponseCurve::Linear,
            dpad_mode: DpadMode::Disabled,
            mode: AnalogMode::Disabled,
            inner_deadzone_x: deadzone,
            inner_deadzone_y: deadzone,
            outer_deadzone_x: MAX_ABS_VALUE as u16,
            outer_deadzone_y: MAX_ABS_VALUE as u16,
        }
    }
}

/// Analog processor for EV_ABS event handling
///
/// Processes analog stick events with deadzone filtering, sensitivity adjustment,
/// and response curve application. Maintains per-device configuration state.
pub struct AnalogProcessor {
    /// Per-device analog configuration
    devices: Arc<RwLock<HashMap<String, DeviceAnalogConfig>>>,
}

impl AnalogProcessor {
    /// Create a new analog processor
    pub fn new() -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Process an analog event
    ///
    /// Applies deadzone filtering, normalization, sensitivity, and response curve.
    /// Returns None if value is within deadzone (filtered).
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    /// * `axis_code` - Axis code (61000-61005 for ABS_X through ABS_RZ)
    /// * `raw_value` - Raw value from evdev (-32768 to 32767)
    ///
    /// # Returns
    ///
    /// * `Some(i32)` - Processed value (outside deadzone)
    /// * `None` - Value filtered by deadzone
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Process ABS_X event
    /// if let Some(value) = processor.process_event("1532:0220", 61000, 25000).await {
    ///     // Send to macro engine
    /// }
    /// ```
    pub async fn process_event(
        &self,
        device_id: &str,
        axis_code: u16,
        raw_value: i32,
    ) -> Option<i32> {
        // Get or create device config
        let config = self.get_or_create_device_config(device_id).await;

        // Determine axis-specific deadzone (61000 = ABS_X, 61001 = ABS_Y)
        let (inner_deadzone, outer_deadzone) = match axis_code {
            61000 => (config.inner_deadzone_x as i32, config.outer_deadzone_x as i32), // X-axis
            61001 => (config.inner_deadzone_y as i32, config.outer_deadzone_y as i32), // Y-axis
            _ => (config.deadzone as i32, MAX_ABS_VALUE), // Other axes use average/default
        };

        // Step 1: Inner deadzone filtering (noise near center)
        // If |value| < inner_deadzone, return None (filtered)
        if raw_value.abs() < inner_deadzone {
            debug!(
                "Analog event filtered by inner deadzone: device={}, axis={}, value={}, inner_deadzone={}",
                device_id, axis_code, raw_value, inner_deadzone
            );
            return None;
        }

        // Step 2: Outer deadzone clamping (max input range)
        // If |value| > outer_deadzone, clamp to outer_deadzone
        let abs_value = raw_value.abs().min(outer_deadzone);

        // Step 3: Normalization
        // Map (inner_deadzone, outer_deadzone) to (0, max)
        let sign = raw_value.signum();
        let normalized = ((abs_value - inner_deadzone) as f32 / (outer_deadzone - inner_deadzone) as f32)
            .clamp(0.0, 1.0);

        // Step 4: Sensitivity multiplier
        let scaled = normalized * config.sensitivity;

        // Step 5: Response curve
        let output = match config.response_curve {
            ResponseCurve::Linear => {
                // Linear: pass through scaled value
                scaled
            }
            ResponseCurve::Exponential { exponent } => {
                // Exponential: f(x) = sign(x) * |x|^exponent
                // Apply exponent to scaled value (0.0 to 1.0)
                scaled.powf(exponent.clamp(0.1, 5.0))
            }
        };

        // Convert back to i32 range
        let final_value = (sign as f32 * output * MAX_ABS_VALUE as f32) as i32;

        debug!(
            "Analog event processed: device={}, axis={}, raw={}, inner_deadzone={}, outer_deadzone={}, sensitivity={:.2}, curve={:?}, output={}",
            device_id, axis_code, raw_value, inner_deadzone, outer_deadzone, config.sensitivity, config.response_curve, final_value
        );

        Some(final_value)
    }

    /// Set deadzone for a device (both X and Y axes)
    ///
    /// This is a backward-compatible method that sets both axes to the same value.
    /// Use set_deadzone_x/set_deadzone_y for per-axis control.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `value` - Deadzone threshold (0-32767)
    pub async fn set_deadzone(&self, device_id: &str, value: u16) {
        let mut devices = self.devices.write().await;
        let config = devices.entry(device_id.to_string()).or_insert_with(|| {
            DeviceAnalogConfig::new(device_id.to_string())
        });
        config.deadzone = value;
        config.inner_deadzone_x = value;
        config.inner_deadzone_y = value;
        info!(
            "Deadzone updated (both axes): device={}, deadzone={}",
            device_id, value
        );
    }

    /// Set deadzone for X-axis only
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `value` - Deadzone threshold (0-32767)
    pub async fn set_deadzone_x(&self, device_id: &str, value: u16) {
        let mut devices = self.devices.write().await;
        let config = devices.entry(device_id.to_string()).or_insert_with(|| {
            DeviceAnalogConfig::new(device_id.to_string())
        });
        config.inner_deadzone_x = value;
        info!(
            "X-axis deadzone updated: device={}, deadzone_x={}",
            device_id, value
        );
    }

    /// Set deadzone for Y-axis only
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `value` - Deadzone threshold (0-32767)
    pub async fn set_deadzone_y(&self, device_id: &str, value: u16) {
        let mut devices = self.devices.write().await;
        let config = devices.entry(device_id.to_string()).or_insert_with(|| {
            DeviceAnalogConfig::new(device_id.to_string())
        });
        config.inner_deadzone_y = value;
        info!(
            "Y-axis deadzone updated: device={}, deadzone_y={}",
            device_id, value
        );
    }


    /// Set deadzone percentage for a device (both X and Y axes)
    ///
    /// Converts percentage (0-100) to raw deadzone value (0-32767).
    /// Conversion formula: percentage * 32767 / 100
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `percentage` - Deadzone percentage (0-100)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Deadzone set successfully
    /// * `Err(String)` - Invalid percentage (must be 0-100)
    pub async fn set_deadzone_percentage(&self, device_id: &str, percentage: u8) -> Result<(), String> {
        if percentage > 100 {
            return Err(format!(
                "Invalid deadzone percentage: {} (must be 0-100)",
                percentage
            ));
        }

        // Convert percentage to raw value: percentage * 32767 / 100
        let raw_value = (percentage as u32 * MAX_ABS_VALUE as u32 / 100) as u16;

        let mut devices = self.devices.write().await;
        let config = devices.entry(device_id.to_string()).or_insert_with(|| {
            DeviceAnalogConfig::new(device_id.to_string())
        });
        config.deadzone = raw_value;
        config.inner_deadzone_x = raw_value;
        config.inner_deadzone_y = raw_value;

        info!(
            "Deadzone updated via percentage (both axes): device={}, {}% = {} raw",
            device_id, percentage, raw_value
        );

        Ok(())
    }

    /// Set deadzone percentage for X-axis only
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `percentage` - Deadzone percentage (0-100)
    pub async fn set_deadzone_percentage_x(&self, device_id: &str, percentage: u8) -> Result<(), String> {
        if percentage > 100 {
            return Err(format!("Invalid deadzone percentage: {}", percentage));
        }

        let raw_value = (percentage as u32 * MAX_ABS_VALUE as u32 / 100) as u16;
        let mut devices = self.devices.write().await;
        let config = devices.entry(device_id.to_string()).or_insert_with(|| {
            DeviceAnalogConfig::new(device_id.to_string())
        });
        config.inner_deadzone_x = raw_value;

        info!("X-axis deadzone updated: device={}, {}% = {} raw", device_id, percentage, raw_value);
        Ok(())
    }

    /// Set deadzone percentage for Y-axis only
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `percentage` - Deadzone percentage (0-100)
    pub async fn set_deadzone_percentage_y(&self, device_id: &str, percentage: u8) -> Result<(), String> {
        if percentage > 100 {
            return Err(format!("Invalid deadzone percentage: {}", percentage));
        }

        let raw_value = (percentage as u32 * MAX_ABS_VALUE as u32 / 100) as u16;
        let mut devices = self.devices.write().await;
        let config = devices.entry(device_id.to_string()).or_insert_with(|| {
            DeviceAnalogConfig::new(device_id.to_string())
        });
        config.inner_deadzone_y = raw_value;

        info!("Y-axis deadzone updated: device={}, {}% = {} raw", device_id, percentage, raw_value);
        Ok(())
    }

    /// Get deadzone percentage for a device (returns X-axis percentage)
    ///
    /// Converts raw deadzone value to percentage (0-100).
    /// Conversion formula: raw * 100 / 32767
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    ///
    /// # Returns
    ///
    /// Deadzone percentage (0-100), or default percentage if device not configured
    pub async fn get_deadzone_percentage(&self, device_id: &str) -> u8 {
        self.get_deadzone_percentage_x(device_id).await
    }

    /// Get X-axis deadzone percentage
    pub async fn get_deadzone_percentage_x(&self, device_id: &str) -> u8 {
        let devices = self.devices.read().await;
        if let Some(config) = devices.get(device_id) {
            (config.inner_deadzone_x as u32 * 100 / MAX_ABS_VALUE as u32) as u8
        } else {
            (DEFAULT_DEADZONE as u32 * 100 / MAX_ABS_VALUE as u32) as u8
        }
    }

    /// Get Y-axis deadzone percentage
    pub async fn get_deadzone_percentage_y(&self, device_id: &str) -> u8 {
        let devices = self.devices.read().await;
        if let Some(config) = devices.get(device_id) {
            (config.inner_deadzone_y as u32 * 100 / MAX_ABS_VALUE as u32) as u8
        } else {
            (DEFAULT_DEADZONE as u32 * 100 / MAX_ABS_VALUE as u32) as u8
        }
    }

    /// Set outer deadzone for X-axis (max input clamping)
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `value` - Outer deadzone threshold (0-32767, default 32767 = no clamp)
    pub async fn set_outer_deadzone_x(&self, device_id: &str, value: u16) {
        let mut devices = self.devices.write().await;
        let config = devices.entry(device_id.to_string()).or_insert_with(|| {
            DeviceAnalogConfig::new(device_id.to_string())
        });
        config.outer_deadzone_x = value;
        info!(
            "X-axis outer deadzone updated: device={}, outer_deadzone_x={}",
            device_id, value
        );
    }

    /// Set outer deadzone for Y-axis (max input clamping)
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `value` - Outer deadzone threshold (0-32767, default 32767 = no clamp)
    pub async fn set_outer_deadzone_y(&self, device_id: &str, value: u16) {
        let mut devices = self.devices.write().await;
        let config = devices.entry(device_id.to_string()).or_insert_with(|| {
            DeviceAnalogConfig::new(device_id.to_string())
        });
        config.outer_deadzone_y = value;
        info!(
            "Y-axis outer deadzone updated: device={}, outer_deadzone_y={}",
            device_id, value
        );
    }

    /// Get outer deadzone percentage for X-axis
    pub async fn get_outer_deadzone_percentage_x(&self, device_id: &str) -> u8 {
        let devices = self.devices.read().await;
        if let Some(config) = devices.get(device_id) {
            (config.outer_deadzone_x as u32 * 100 / MAX_ABS_VALUE as u32) as u8
        } else {
            100  // Default: no clamp (100% of max range)
        }
    }

    /// Get outer deadzone percentage for Y-axis
    pub async fn get_outer_deadzone_percentage_y(&self, device_id: &str) -> u8 {
        let devices = self.devices.read().await;
        if let Some(config) = devices.get(device_id) {
            (config.outer_deadzone_y as u32 * 100 / MAX_ABS_VALUE as u32) as u8
        } else {
            100  // Default: no clamp (100% of max range)
        }
    }
    /// Set sensitivity for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `value` - Sensitivity multiplier (0.1-5.0)
    pub async fn set_sensitivity(&self, device_id: &str, value: f32) {
        let clamped = value.clamp(0.1, 5.0);
        let mut devices = self.devices.write().await;
        let config = devices.entry(device_id.to_string()).or_insert_with(|| {
            DeviceAnalogConfig::new(device_id.to_string())
        });
        config.sensitivity = clamped;
        info!(
            "Sensitivity updated: device={}, sensitivity={:.2}",
            device_id, clamped
        );
    }

    /// Set response curve for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `curve` - Response curve type
    pub async fn set_response_curve(&self, device_id: &str, curve: ResponseCurve) {
        let mut devices = self.devices.write().await;
        let config = devices.entry(device_id.to_string()).or_insert_with(|| {
            DeviceAnalogConfig::new(device_id.to_string())
        });
        config.response_curve = curve;
        info!(
            "Response curve updated: device={}, curve={:?}",
            device_id, curve
        );
    }

    /// Set calibration for a device and layer
    ///
    /// This method updates the device configuration with the specified calibration
    /// settings for the given layer. The calibration will be applied to analog
    /// events from this device when the layer is active.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    /// * `layer_id` - Layer ID (0=base, 1, 2, ...)
    /// * `calibration` - Calibration settings to apply
    pub async fn set_calibration(&self, device_id: &str, layer_id: usize, calibration: AnalogCalibration) {
        let mut devices = self.devices.write().await;
        let config = devices.entry(device_id.to_string()).or_insert_with(|| {
            DeviceAnalogConfig::new(device_id.to_string())
        });

        // For now, we only support base layer (layer 0) calibration
        // In the future, per-layer calibration will be supported
        if layer_id == 0 {
            config.deadzone = (calibration.deadzone * 32767.0) as u16;
            config.sensitivity = calibration.sensitivity_multiplier;
            config.response_curve = match calibration.sensitivity {
                SensitivityCurve::Linear => ResponseCurve::Linear,
                SensitivityCurve::Quadratic => ResponseCurve::Exponential { exponent: 2.0 },
                SensitivityCurve::Exponential { exponent } => ResponseCurve::Exponential { exponent },
            };

            // TODO: Store layer-specific calibrations for future per-layer support
            // For now, only base layer is applied
            info!(
                "Calibration updated: device={}, deadzone={:.2}, sensitivity={:.2}, curve={:?}",
                device_id, calibration.deadzone, calibration.sensitivity_multiplier, calibration.sensitivity
            );
        } else {
            debug!(
                "Layer-specific calibration not yet supported, storing for layer {}",
                layer_id
            );
            // TODO: Store layer-specific calibrations
        }
    }

    /// Get or create device configuration
    ///
    /// Returns the device configuration, creating with defaults if not exists.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    ///
    /// # Returns
    ///
    /// Device configuration (cloned for independent use)
    pub async fn get_or_create_device_config(&self, device_id: &str) -> DeviceAnalogConfig {
        let devices = self.devices.read().await;
        if let Some(config) = devices.get(device_id) {
            config.clone()
        } else {
            drop(devices);
            let mut devices = self.devices.write().await;
            devices
                .entry(device_id.to_string())
                .or_insert_with(|| DeviceAnalogConfig::new(device_id.to_string()))
                .clone()
        }
    }

    /// Get device configuration
    ///
    /// Returns None if device not configured.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    ///
    /// # Returns
    ///
    /// * `Some(DeviceAnalogConfig)` - Device configuration
    /// * `None` - Device not configured
    pub async fn get_device_config(&self, device_id: &str) -> Option<DeviceAnalogConfig> {
        let devices = self.devices.read().await;
        devices.get(device_id).cloned()
    }

    /// Remove device configuration
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    pub async fn remove_device_config(&self, device_id: &str) {
        let mut devices = self.devices.write().await;
        if devices.remove(device_id).is_some() {
            info!("Device config removed: {}", device_id);
        }
    }

    /// Get all configured devices
    ///
    /// # Returns
    ///
    /// Vector of device identifiers with analog configuration
    pub async fn get_configured_devices(&self) -> Vec<String> {
        let devices = self.devices.read().await;
        devices.keys().cloned().collect()
    }

    /// Set D-pad mode for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `mode` - D-pad mode (Disabled, EightWay, FourWay)
    pub async fn set_dpad_mode(&self, device_id: &str, mode: DpadMode) {
        let mut devices = self.devices.write().await;
        let config = devices.entry(device_id.to_string()).or_insert_with(|| {
            DeviceAnalogConfig::new(device_id.to_string())
        });
        config.dpad_mode = mode;
        info!(
            "D-pad mode updated: device={}, mode={:?}",
            device_id, mode
        );
    }

    /// Get D-pad mode for a device
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    ///
    /// # Returns
    ///
    /// Current D-pad mode (Disabled if device not configured)
    pub async fn get_dpad_mode(&self, device_id: &str) -> DpadMode {
        let devices = self.devices.read().await;
        if let Some(config) = devices.get(device_id) {
            config.dpad_mode
        } else {
            DpadMode::Disabled
        }
    }

    /// Load analog configuration from profile
    ///
    /// Applies settings from AnalogDeviceConfig to the device's internal configuration.
    /// This is called when a device is grabbed or a profile is activated.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `config` - Analog configuration from profile
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Configuration loaded successfully
    /// * `Err(String)` - Invalid configuration (e.g., bad curve string)
    pub async fn load_config(
        &self,
        device_id: &str,
        config: &crate::config::AnalogDeviceConfig,
    ) -> Result<(), String> {
        // Create or update device config from profile
        let mut device_config = self.get_or_create_device_config(device_id).await;

        // Apply settings from profile
        device_config.inner_deadzone_x = percentage_to_raw(config.deadzone_percentage_x);
        device_config.inner_deadzone_y = percentage_to_raw(config.deadzone_percentage_y);
        device_config.outer_deadzone_x = percentage_to_raw(config.outer_deadzone_percentage_x);
        device_config.outer_deadzone_y = percentage_to_raw(config.outer_deadzone_percentage_y);
        device_config.sensitivity = config.sensitivity;
        device_config.response_curve = parse_response_curve(&config.response_curve)?;
        device_config.dpad_mode = parse_dpad_mode(&config.dpad_mode)?;

        // Store updated config
        let mut devices = self.devices.write().await;
        devices.insert(device_id.to_string(), device_config);

        info!(
            "Loaded analog config for device {}: deadzone_xy={}%,{}%, outer_deadzone_xy={}%,{}%, sensitivity={:.2}, curve={}, dpad={}",
            device_id,
            config.deadzone_percentage_x,
            config.deadzone_percentage_y,
            config.outer_deadzone_percentage_x,
            config.outer_deadzone_percentage_y,
            config.sensitivity,
            config.response_curve,
            config.dpad_mode
        );

        Ok(())
    }

    /// Save analog configuration to profile format
    ///
    /// Serializes the current device configuration to AnalogDeviceConfig for persistence.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    ///
    /// # Returns
    ///
    /// * `Ok(AnalogDeviceConfig)` - Current configuration
    /// * `Err(String)` - Device not found
    pub async fn save_config(
        &self,
        device_id: &str,
    ) -> Result<crate::config::AnalogDeviceConfig, String> {
        let devices = self.devices.read().await;
        let config = devices
            .get(device_id)
            .ok_or_else(|| format!("Device {} not found", device_id))?;

        Ok(crate::config::AnalogDeviceConfig {
            deadzone_percentage: raw_to_percentage(config.inner_deadzone_x),
            deadzone_percentage_x: raw_to_percentage(config.inner_deadzone_x),
            deadzone_percentage_y: raw_to_percentage(config.inner_deadzone_y),
            outer_deadzone_percentage: raw_to_percentage(config.outer_deadzone_x),
            outer_deadzone_percentage_x: raw_to_percentage(config.outer_deadzone_x),
            outer_deadzone_percentage_y: raw_to_percentage(config.outer_deadzone_y),
            sensitivity: config.sensitivity,
            response_curve: response_curve_to_string(config.response_curve),
            dpad_mode: dpad_mode_to_string(config.dpad_mode),
        })
    }

    /// Process raw analog input (x, y) through the 2D calibration pipeline
    ///
    /// This method processes both X and Y axes together, enabling circular deadzone
    /// calculations that consider the combined magnitude of both axes.
    ///
    /// # Processing Pipeline
    ///
    /// 1. **Normalize**: Convert raw ADC (0-255) to 0.0-1.0 range
    /// 2. **Center**: Transform to centered coordinates (-0.5 to 0.5), negate Y
    /// 3. **Deadzone**: Filter small movements based on shape (circular/square)
    /// 4. **Sensitivity**: Apply response curve (linear/quadratic/exponential)
    /// 5. **Scale**: Convert to output range with inversion support
    ///
    /// # Arguments
    ///
    /// * `calibration` - Calibration settings for processing
    /// * `x` - Raw X value (typically 0-255 for 8-bit ADC)
    /// * `y` - Raw Y value (typically 0-255 for 8-bit ADC)
    ///
    /// # Returns
    ///
    /// Calibrated (x, y) in range [range_min, range_max]
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let calibration = AnalogCalibration::default();
    /// let (x, y) = processor.process(&calibration, 128, 64);
    /// ```
    pub fn process(&self, calibration: &AnalogCalibration, x: i32, y: i32) -> (i32, i32) {
        // Step 1: Normalize to 0.0-1.0 range
        let (nx, ny) = self.normalize(x, y);

        // Step 2: Center at origin (0,0) -> (-0.5 to 0.5)
        let (cx, cy) = self.center(nx, ny);

        // Step 3: Apply deadzone filter
        let (dx, dy) = self.apply_deadzone(cx, cy, calibration);

        // Step 4: Apply sensitivity curve
        let (sx, sy) = self.apply_sensitivity(dx, dy, calibration);

        // Step 5: Scale to output range with inversion
        let (ox, oy) = self.scale_to_output(sx, sy, calibration);

        (ox, oy)
    }

    /// Normalize raw ADC values to 0.0-1.0 range
    ///
    /// Assumes 8-bit input (0-255) per CONTEXT.md.
    /// Values are clamped to valid range before conversion.
    fn normalize(&self, x: i32, y: i32) -> (f32, f32) {
        const INPUT_MAX: f32 = 255.0;
        (
            x.clamp(0, 255) as f32 / INPUT_MAX,
            y.clamp(0, 255) as f32 / INPUT_MAX,
        )
    }

    /// Center normalized coordinates at origin
    ///
    /// Transforms (0.0-1.0, 0.0-1.0) to (-0.5-0.5, -0.5-0.5).
    /// Y-axis is negated to match standard coordinate system (up = negative).
    fn center(&self, x: f32, y: f32) -> (f32, f32) {
        (x - 0.5, -(y - 0.5))
    }

    /// Detect 8-way D-pad direction from centered analog coordinates
    ///
    /// Uses 22.5-degree sectors for 8-way direction detection.
    /// The angle is calculated with atan2, where 0 degrees = right, positive = clockwise.
    /// Applies deadzone checking before direction detection to prevent drift.
    ///
    /// # Coordinate System Note
    ///
    /// The input coordinates come from center(), which negates Y to match the visual
    /// coordinate system where "up" is toward the top of the screen (Y=0 input).
    /// After center(): positive Y = visual up (Y=0 input), negative Y = visual down (Y=255 input).
    ///
    /// # Arguments
    ///
    /// * `x` - Centered X value (-0.5 to 0.5, will be scaled to -1.0 to 1.0)
    /// * `y` - Centered Y value (-0.5 to 0.5, will be scaled to -1.0 to 1.0)
    ///
    /// # Returns
    ///
    /// D-pad direction based on the angle of the input vector
    pub fn detect_dpad_direction(&self, x: f32, y: f32) -> DpadDirection {
        // Small threshold for detecting meaningful movement (prevents drift)
        const DIRECTION_THRESHOLD: f32 = 0.1;

        // Calculate magnitude
        let magnitude = (x * x + y * y).sqrt();
        if magnitude < DIRECTION_THRESHOLD {
            return DpadDirection::None;
        }

        // Calculate angle in degrees
        // center() negates Y, so: Y=0 input -> cy=0.5 (visual up), Y=255 -> cy=-0.5 (visual down)
        // atan2(cy, cx): 90 deg = up, -90/270 deg = down (after center())
        let angle_rad = y.atan2(x);
        let angle_deg = angle_rad.to_degrees();
        let normalized_angle = if angle_deg < 0.0 {
            angle_deg + 360.0
        } else {
            angle_deg
        };

        // 8 sectors of 45 degrees each, centered on cardinal and diagonal directions
        // After center(), atan2 gives: 90=up, 0=right, -90/270=down, 180=left
        match normalized_angle {
            a if a >= 337.5 || a < 22.5 => DpadDirection::Right,
            a if a >= 22.5 && a < 67.5 => DpadDirection::UpRight,
            a if a >= 67.5 && a < 112.5 => DpadDirection::Up,
            a if a >= 112.5 && a < 157.5 => DpadDirection::UpLeft,
            a if a >= 157.5 && a < 202.5 => DpadDirection::Left,
            a if a >= 202.5 && a < 247.5 => DpadDirection::DownLeft,
            a if a >= 247.5 && a < 292.5 => DpadDirection::Down,
            a if a >= 292.5 && a < 337.5 => DpadDirection::DownRight,
            _ => DpadDirection::None,
        }
    }

    /// Process analog input as D-pad output
    ///
    /// Converts analog stick position to 8-way directional keys.
    /// Returns a vector of (key, pressed) tuples for the current state.
    ///
    /// This method processes raw ADC values through the normalization and centering
    /// pipeline, then detects direction based on the resulting position.
    /// Deadzone filtering is applied via the direction threshold in detect_dpad_direction().
    ///
    /// # Arguments
    ///
    /// * `calibration` - Calibration settings for processing
    /// * `x` - Raw X value (0-255 for 8-bit ADC)
    /// * `y` - Raw Y value (0-255 for 8-bit ADC)
    ///
    /// # Returns
    ///
    /// Vec of (Key, bool) where bool indicates pressed (true) or released (false).
    /// Cardinal directions return 1 key, diagonals return 2 keys.
    /// Centered/deadzone position returns empty Vec.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let calibration = AnalogCalibration::default();
    /// let output = processor.process_as_dpad(&calibration, 200, 128);  // Right
    /// // Returns: [(KEY_RIGHT, true)]
    ///
    /// let output = processor.process_as_dpad(&calibration, 200, 50);  // Up-Right
    /// // Returns: [(KEY_UP, true), (KEY_RIGHT, true)]
    ///
    /// let output = processor.process_as_dpad(&calibration, 128, 128);  // Center
    /// // Returns: []
    /// ```
    ///
    /// # Note
    ///
    /// This method returns only the current frame's state with pressed=true.
    /// The caller (device.rs event loop) is responsible for tracking previous state
    /// and sending release events when direction changes (matching Phase 09-04 hat switch pattern).
    pub fn process_as_dpad(&self, calibration: &AnalogCalibration, x: i32, y: i32) -> Vec<(Key, bool)> {
        // Step 1: Normalize to 0.0-1.0 range
        let (nx, ny) = self.normalize(x, y);

        // Step 2: Center at origin (0,0) -> (-0.5 to 0.5)
        let (cx, cy) = self.center(nx, ny);

        // Step 3: Apply deadzone filter (returns (0,0) if within deadzone)
        let (dx, dy) = self.apply_deadzone(cx, cy, calibration);

        // Apply axis inversion for direction detection
        // This mirrors the inversion logic from scale_to_output
        let (ix, iy) = (
            if calibration.invert_x { -dx } else { dx },
            if calibration.invert_y { -dy } else { dy },
        );

        // Scale to -1.0 to 1.0 range for direction detection
        // After center(), values are in -0.5 to 0.5 range, so multiply by 2
        let (sx, sy) = (ix * 2.0, iy * 2.0);

        // Detect direction
        let direction = self.detect_dpad_direction(sx, sy);

        // Convert to key codes (all pressed = true)
        dpad_direction_to_keys(direction)
            .into_iter()
            .map(|k| (k, true))
            .collect()
    }

    /// Process analog input as WASD key output
    ///
    /// Converts analog stick position to 8-way WASD directional keys.
    /// Returns a vector of (key, pressed) tuples for the current state.
    ///
    /// This method follows the EXACT same processing pipeline as process_as_dpad(),
    /// but outputs WASD keys instead of arrow keys. This enables gamers to use
    /// the analog stick for movement with the standard PC gaming control scheme.
    ///
    /// # Arguments
    ///
    /// * `calibration` - Calibration settings for processing
    /// * `x` - Raw X value (0-255 for 8-bit ADC)
    /// * `y` - Raw Y value (0-255 for 8-bit ADC)
    ///
    /// # Returns
    ///
    /// Vec of (Key, bool) where bool indicates pressed (true) or released (false).
    /// Cardinal directions return 1 WASD key, diagonals return 2 keys.
    /// Centered/deadzone position returns empty Vec.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let calibration = AnalogCalibration::default();
    /// let output = processor.process_as_wasd(&calibration, 200, 128);  // Right
    /// // Returns: [(KEY_D, true)]
    ///
    /// let output = processor.process_as_wasd(&calibration, 200, 50);  // Up-Right
    /// // Returns: [(KEY_W, true), (KEY_D, true)]
    ///
    /// let output = processor.process_as_wasd(&calibration, 128, 128);  // Center
    /// // Returns: []
    /// ```
    ///
    /// # WASD Key Mapping
    ///
    /// - Up (forward) → W
    /// - Down (backward) → S
    /// - Left (strafe left) → A
    /// - Right (strafe right) → D
    /// - Diagonals → key combinations (e.g., UpRight → W + D)
    ///
    /// # Note
    ///
    /// This method returns only the current frame's state with pressed=true.
    /// The caller (device.rs event loop) is responsible for tracking previous state
    /// and sending release events when direction changes (matching Phase 09-04 hat switch pattern).
    pub fn process_as_wasd(
        &self,
        calibration: &AnalogCalibration,
        x: i32,
        y: i32,
    ) -> Vec<(Key, bool)> {
        // Step 1: Normalize to 0.0-1.0 range
        let (nx, ny) = self.normalize(x, y);

        // Step 2: Center at origin (0,0) -> (-0.5 to 0.5)
        let (cx, cy) = self.center(nx, ny);

        // Step 3: Apply deadzone filter (returns (0,0) if within deadzone)
        let (dx, dy) = self.apply_deadzone(cx, cy, calibration);

        // Apply axis inversion for direction detection
        // This mirrors the inversion logic from scale_to_output
        let (ix, iy) = (
            if calibration.invert_x { -dx } else { dx },
            if calibration.invert_y { -dy } else { dy },
        );

        // Scale to -1.0 to 1.0 range for direction detection
        // After center(), values are in -0.5 to 0.5 range, so multiply by 2
        let (sx, sy) = (ix * 2.0, iy * 2.0);

        // Detect direction
        let direction = self.detect_dpad_direction(sx, sy);

        // Convert to WASD key codes (all pressed = true)
        wasd_direction_to_keys(direction)
            .into_iter()
            .map(|k| (k, true))
            .collect()
    }

    /// Process analog stick input as mouse velocity output
    ///
    /// Applies the full calibration pipeline (normalize, center, deadzone, sensitivity)
    /// and returns cursor velocity proportional to stick deflection.
    ///
    /// # Arguments
    ///
    /// * `calibration` - Analog calibration settings (deadzone, sensitivity, etc.)
    /// * `x` - Raw X axis value (0-255)
    /// * `y` - Raw Y axis value (0-255)
    /// * `config` - Mouse velocity configuration (multiplier for cursor speed)
    ///
    /// # Returns
    ///
    /// * `Some((vel_x, vel_y))` - Cursor velocity in pixels
    /// * `None` - Value filtered by deadzone (no movement)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let calibration = AnalogCalibration::default();
    /// let config = MouseVelocityConfig::default();
    ///
    /// // Full right deflection: ~10 pixels velocity
    /// let result = processor.process_as_mouse(&calibration, 255, 128, &config);
    /// assert_eq!(result, Some((10, 0)));
    ///
    /// // Center position: no movement (deadzone)
    /// let result = processor.process_as_mouse(&calibration, 128, 128, &config);
    /// assert_eq!(result, None);
    /// ```
    pub fn process_as_mouse(
        &self,
        calibration: &AnalogCalibration,
        x: i32,
        y: i32,
        config: &MouseVelocityConfig,
    ) -> Option<(i32, i32)> {
        // Step 1: Normalize and center using process_2d() - this handles the full calibration pipeline
        let (processed_x, processed_y) = Self::process_2d(x, y, calibration)?;

        // Step 2: Convert the processed values from Linux input range (-32768 to 32767) to velocity
        // Normalize to -1.0 to 1.0, then apply multiplier
        let vel_x = ((processed_x as f32 / 32768.0) * config.multiplier) as i32;
        let vel_y = ((processed_y as f32 / 32768.0) * config.multiplier) as i32;

        Some((vel_x, vel_y))
    }

    /// Process analog stick input as camera control
    ///
    /// Applies calibration pipeline (deadzone, sensitivity) then returns
    /// either scroll amount or camera keys based on the output mode.
    ///
    /// # Arguments
    ///
    /// * `calibration` - Analog calibration settings
    /// * `x` - Raw X axis value (0-255)
    /// * `y` - Raw Y axis value (0-255)
    /// * `mode` - Camera output mode (Scroll or Keys)
    ///
    /// # Returns
    ///
    /// * `Some(CameraOutput::Scroll(amount))` - Scroll amount for REL_WHEEL event
    /// * `Some(CameraOutput::Keys(keys))` - Keys for key repeat mode
    /// * `None` - Value filtered by deadzone (no output)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let calibration = AnalogCalibration::default();
    ///
    /// // Scroll mode: Y-axis deflection controls scroll
    /// let result = processor.process_as_camera(&calibration, 128, 0, CameraOutputMode::Scroll);
    /// assert!(matches!(result, Some(CameraOutput::Scroll(_))));
    ///
    /// // Key mode: 8-way direction detection for camera keys
    /// let result = processor.process_as_camera(&calibration, 255, 0, CameraOutputMode::Keys);
    /// assert!(matches!(result, Some(CameraOutput::Keys(_))));
    /// ```
    pub fn process_as_camera(
        &self,
        calibration: &AnalogCalibration,
        x: i32,
        y: i32,
        mode: CameraOutputMode,
    ) -> Option<CameraOutput> {
        // Step 1: Normalize and center using process_2d() - this handles the full calibration pipeline
        let (processed_x, processed_y) = Self::process_2d(x, y, calibration)?;

        match mode {
            CameraOutputMode::Scroll => {
                // Scroll mode: Use Y-axis deflection for scroll amount (Y-axis only)
                // Normalize to -1.0 to 1.0, then apply sensitivity multiplier
                let scroll_amount = ((processed_y as f32 / 32768.0)
                    * calibration.sensitivity_multiplier
                    * 3.0) as i32;
                Some(CameraOutput::Scroll(scroll_amount))
            }
            CameraOutputMode::Keys => {
                // Key mode: Use 8-way direction detection for camera keys
                // Normalize to -1.0 to 1.0 for direction detection (scale factor 2.0)
                let sx = processed_x as f32 / 32768.0 * 2.0;
                let sy = processed_y as f32 / 32768.0 * 2.0;
                let direction = self.detect_dpad_direction(sx, sy);
                Some(CameraOutput::Keys(camera_direction_to_keys(direction)))
            }
        }
    }

    /// Process analog stick input as gamepad axis output
    ///
    /// Applies calibration pipeline (deadzone, sensitivity, range) then
    /// returns calibrated values ready for gamepad device emission.
    /// In plan 14-03, this will integrate with GamepadVirtualDevice.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier
    /// * `raw_x` - Raw X axis value (0-255)
    /// * `raw_y` - Raw Y axis value (0-255)
    ///
    /// # Returns
    ///
    /// * `Some((x, y))` - Calibrated axis values for gamepad output
    /// * `None` - Value filtered by deadzone (no output)
    pub async fn process_as_gamepad(
        &self,
        device_id: &str,
        raw_x: i32,
        raw_y: i32,
    ) -> Option<(i32, i32)> {
        // Get device config
        let devices = self.devices.read().await;
        let config = devices.get(device_id)?;

        // Use AnalogCalibration for 2D processing
        // Note: inner_deadzone_x is in Linux input range (0-32767)
        // For centered coordinates (-0.5 to 0.5), we need to adjust:
        // deadzone in centered system = (inner_deadzone_x / MAX_ABS_VALUE) * 0.5
        let deadzone_normalized = (config.inner_deadzone_x as f32 / MAX_ABS_VALUE as f32) * 0.5;
        let calibration = AnalogCalibration {
            deadzone: deadzone_normalized,
            deadzone_shape: DeadzoneShape::Circular,
            sensitivity: SensitivityCurve::Linear,
            sensitivity_multiplier: config.sensitivity,
            range_min: -32768,
            range_max: 32767,
            invert_x: false,
            invert_y: true,  // Invert Y for gamepad coordinates (up = negative)
        };

        // Process 2D coordinates (plan 14-05 will integrate full calibration)
        let (x, y) = Self::process_2d(raw_x, raw_y, &calibration)?;

        Some((x, y))
    }

    /// Process analog with specific calibration (for per-layer config)
    ///
    /// This method allows passing a custom AnalogCalibration for layer-specific
    /// analog processing. It bypasses the device config lookup and uses the
    /// provided calibration directly.
    ///
    /// # Arguments
    ///
    /// * `raw_x` - Raw X axis value (0-255)
    /// * `raw_y` - Raw Y axis value (0-255)
    /// * `calibration` - Custom calibration settings
    ///
    /// # Returns
    ///
    /// * `Some((x, y))` - Calibrated axis values for gamepad output
    /// * `None` - Value filtered by deadzone (no output)
    pub async fn process_as_gamepad_with_calibration(
        &self,
        raw_x: i32,
        raw_y: i32,
        calibration: &AnalogCalibration,
    ) -> Option<(i32, i32)> {
        // Process 2D coordinates with the provided calibration
        Self::process_2d(raw_x, raw_y, calibration)
    }

    /// 2D coordinate processing using AnalogCalibration
    /// Extracted from process() for reusability across modes
    ///
    /// # Processing Pipeline
    ///
    /// 1. **Normalize**: Map raw ADC (0-255) to 0.0-1.0
    /// 2. **Center**: Convert to -0.5 to 0.5 (center = 0,0)
    /// 3. **Deadzone**: Filter values within deadzone threshold
    /// 4. **Sensitivity**: Apply response curve and multiplier
    /// 5. **Scale**: Convert to output range (-32768 to 32767)
    ///
    /// # Returns
    ///
    /// * `Some((x, y))` - Processed axis values in Linux input range
    /// * `None` - Value filtered by deadzone
    fn process_2d(raw_x: i32, raw_y: i32, calibration: &AnalogCalibration) -> Option<(i32, i32)> {
        // Step 1: Normalize raw ADC (0-255) to 0.0-1.0
        let nx = raw_x as f32 / 255.0;
        let ny = raw_y as f32 / 255.0;

        // Step 2: Center to -0.5 to 0.5
        // Y is inverted (up is negative in screen coordinates)
        let cx = nx - 0.5;
        let cy = 0.5 - ny;

        // Step 3: Apply deadzone filtering
        // This returns (0.0, 0.0) if within deadzone
        let (dx, dy) = Self::apply_deadzone_static(cx, cy, calibration);

        // For circular deadzone, check if magnitude was below threshold
        // For square deadzone, check if both axes were below threshold
        let magnitude_before = (cx * cx + cy * cy).sqrt();
        if magnitude_before < calibration.deadzone {
            return None;
        }

        // Step 4: Apply sensitivity curves
        let (sx, sy) = Self::apply_sensitivity_static(dx, dy, calibration);

        // Step 5: Scale to output range with inversion
        let result = Self::scale_to_output_static(sx, sy, calibration);

        Some(result)
    }

    /// Static version of apply_deadzone for use in process_2d
    fn apply_deadzone_static(x: f32, y: f32, calibration: &AnalogCalibration) -> (f32, f32) {
        // Max magnitude in centered coordinate system (-0.5 to 0.5)
        const MAX_MAGNITUDE: f32 = 0.70710678; // sqrt(0.5)
        const MAX_AXIS: f32 = 0.5;

        match calibration.deadzone_shape {
            DeadzoneShape::Circular => {
                let magnitude = (x * x + y * y).sqrt();
                if magnitude < calibration.deadzone {
                    return (0.0, 0.0);
                }
                let scale = if magnitude < MAX_MAGNITUDE && calibration.deadzone < MAX_MAGNITUDE {
                    (magnitude - calibration.deadzone) / (MAX_MAGNITUDE - calibration.deadzone)
                } else {
                    1.0
                };
                (x * scale, y * scale)
            }
            DeadzoneShape::Square => {
                let max_val = x.abs().max(y.abs());
                if max_val < calibration.deadzone {
                    return (0.0, 0.0);
                }
                let dx = if x.abs() < calibration.deadzone {
                    0.0
                } else {
                    let scale = (x.abs() - calibration.deadzone) / (MAX_AXIS - calibration.deadzone);
                    x.signum() * scale.min(1.0)
                };
                let dy = if y.abs() < calibration.deadzone {
                    0.0
                } else {
                    let scale = (y.abs() - calibration.deadzone) / (MAX_AXIS - calibration.deadzone);
                    y.signum() * scale.min(1.0)
                };
                (dx, dy)
            }
        }
    }

    /// Static version of apply_sensitivity for use in process_2d
    fn apply_sensitivity_static(x: f32, y: f32, calibration: &AnalogCalibration) -> (f32, f32) {
        let magnitude = (x * x + y * y).sqrt();
        if magnitude == 0.0 {
            return (0.0, 0.0);
        }

        let angle = y.atan2(x);

        let scaled = match calibration.sensitivity {
            SensitivityCurve::Linear => magnitude,
            SensitivityCurve::Quadratic => magnitude * magnitude,
            SensitivityCurve::Exponential { exponent } => {
                magnitude.powf(exponent.clamp(0.1, 10.0))
            }
        };

        let result = (scaled * calibration.sensitivity_multiplier).min(1.0);
        (angle.cos() * result, angle.sin() * result)
    }

    /// Static version of scale_to_output for use in process_2d
    fn scale_to_output_static(x: f32, y: f32, calibration: &AnalogCalibration) -> (i32, i32) {
        let range = (calibration.range_max - calibration.range_min) as f32;
        let center = (calibration.range_min + calibration.range_max) / 2;

        let mut ox = (x * range) as i32 + center;
        let mut oy = (y * range) as i32 + center;

        if calibration.invert_x {
            ox = calibration.range_max + calibration.range_min - ox;
        }
        if calibration.invert_y {
            oy = calibration.range_max + calibration.range_min - oy;
        }

        ox = ox.clamp(calibration.range_min, calibration.range_max);
        oy = oy.clamp(calibration.range_min, calibration.range_max);

        (ox, oy)
    }

    /// Apply deadzone filtering based on shape
    ///
    /// For circular deadzone: uses magnitude sqrt(x^2 + y^2)
    /// For square deadzone: uses max(abs(x), abs(y))
    ///
    /// Scales output to avoid "jump" at deadzone boundary.
    fn apply_deadzone(&self, x: f32, y: f32, calibration: &AnalogCalibration) -> (f32, f32) {
        // Max magnitude in centered coordinate system (-0.5 to 0.5)
        // At corner (0.5, 0.5), magnitude = sqrt(0.25 + 0.25) = sqrt(0.5) ≈ 0.707
        const MAX_MAGNITUDE: f32 = 0.70710678; // sqrt(0.5)

        match calibration.deadzone_shape {
            DeadzoneShape::Circular => {
                // Calculate magnitude from center
                let magnitude = (x * x + y * y).sqrt();
                if magnitude < calibration.deadzone {
                    return (0.0, 0.0);
                }
                // Scale output to avoid jump at deadzone boundary
                // Map (deadzone, MAX_MAGNITUDE) to (0, magnitude) to preserve direction
                let scale = if magnitude < MAX_MAGNITUDE && calibration.deadzone < MAX_MAGNITUDE {
                    (magnitude - calibration.deadzone) / (MAX_MAGNITUDE - calibration.deadzone)
                } else {
                    1.0
                };
                // Apply scaling to preserve direction but scale magnitude
                (x * scale, y * scale)
            }
            DeadzoneShape::Square => {
                // Square deadzone: use max of absolute values
                // Max value in centered system is 0.5
                const MAX_AXIS: f32 = 0.5;
                let max_val = x.abs().max(y.abs());
                if max_val < calibration.deadzone {
                    return (0.0, 0.0);
                }
                // Scale each axis independently
                let dx = if x.abs() < calibration.deadzone {
                    0.0
                } else {
                    let scale = (x.abs() - calibration.deadzone) / (MAX_AXIS - calibration.deadzone);
                    x.signum() * scale.min(1.0)
                };
                let dy = if y.abs() < calibration.deadzone {
                    0.0
                } else {
                    let scale = (y.abs() - calibration.deadzone) / (MAX_AXIS - calibration.deadzone);
                    y.signum() * scale.min(1.0)
                };
                (dx, dy)
            }
        }
    }

    /// Apply sensitivity curve to normalized values
    ///
    /// Applies the selected response curve and sensitivity multiplier.
    /// Uses polar coordinates to maintain direction while scaling magnitude.
    fn apply_sensitivity(&self, x: f32, y: f32, calibration: &AnalogCalibration) -> (f32, f32) {
        let magnitude = (x * x + y * y).sqrt();
        if magnitude == 0.0 {
            return (0.0, 0.0);
        }

        let angle = y.atan2(x);

        // Apply response curve
        let scaled = match calibration.sensitivity {
            SensitivityCurve::Linear => magnitude,
            SensitivityCurve::Quadratic => magnitude * magnitude,
            SensitivityCurve::Exponential { exponent } => {
                // Default to 2.0 if no exponent specified for backward compatibility
                let exp = match calibration.sensitivity {
                    SensitivityCurve::Exponential { exponent } => exponent,
                    _ => 2.0,
                };
                magnitude.powf(exp)
            }
        };

        // Apply sensitivity multiplier and clamp to valid range
        let result = (scaled * calibration.sensitivity_multiplier).min(1.0);

        // Convert back to Cartesian coordinates
        (angle.cos() * result, angle.sin() * result)
    }

    /// Scale normalized values to output range with inversion
    ///
    /// Converts values to (range_min to range_max).
    /// Applies axis inversion if configured.
    ///
    /// Input values are in centered coordinate system where:
    /// - 0.0 maps to the center of the output range
    /// - Max value after center() is 0.5, which should map to range_max
    /// - Min value after center() is -0.5, which should map to range_min
    ///
    /// After sensitivity curves, values can exceed 0.5, so we clamp to valid range.
    fn scale_to_output(&self, x: f32, y: f32, calibration: &AnalogCalibration) -> (i32, i32) {
        let range = (calibration.range_max - calibration.range_min) as f32;
        let center = (calibration.range_min + calibration.range_max) / 2;

        // Convert from centered coordinates to output range
        // x=0 -> center, x=0.5 -> range_max (approximately), x=-0.5 -> range_min
        let mut ox = (x * range) as i32 + center;
        let mut oy = (y * range) as i32 + center;

        // Apply inversion
        if calibration.invert_x {
            ox = calibration.range_max + calibration.range_min - ox;
        }
        if calibration.invert_y {
            oy = calibration.range_max + calibration.range_min - oy;
        }

        // Clamp to valid range
        ox = ox.clamp(calibration.range_min, calibration.range_max);
        oy = oy.clamp(calibration.range_min, calibration.range_max);

        (ox, oy)
    }

    /// Map analog position to D-pad directions
    ///
    /// Converts normalized X/Y analog values (-1.0 to 1.0) to direction vectors.
    /// Cardinal directions return single direction. Diagonals return two directions.
    ///
    /// # Arguments
    ///
    /// * `x` - Normalized X value (-1.0 to 1.0)
    /// * `y` - Normalized Y value (-1.0 to 1.0)
    /// * `mode` - D-pad mode (EightWay or FourWay)
    ///
    /// # Returns
    ///
    /// Vec of active directions (0-2 directions)
    pub fn map_analog_to_dpad(x: f32, y: f32, mode: DpadMode) -> Vec<Direction> {
        // Threshold for direction activation (30% of max)
        const THRESHOLD: f32 = 0.3;

        let mut directions = Vec::new();

        // Determine primary directions based on threshold
        let has_up = y < -THRESHOLD;
        let has_down = y > THRESHOLD;
        let has_left = x < -THRESHOLD;
        let has_right = x > THRESHOLD;

        match mode {
            DpadMode::EightWay => {
                // 8-way: support diagonals (both axes)
                if has_up {
                    directions.push(Direction::Up);
                }
                if has_down {
                    directions.push(Direction::Down);
                }
                if has_left {
                    directions.push(Direction::Left);
                }
                if has_right {
                    directions.push(Direction::Right);
                }
            }
            DpadMode::FourWay => {
                // 4-way: only cardinal directions (dominant axis wins)
                let abs_x = x.abs();
                let abs_y = y.abs();

                if abs_x > abs_y && abs_x > THRESHOLD {
                    // X is dominant
                    if has_left {
                        directions.push(Direction::Left);
                    } else if has_right {
                        directions.push(Direction::Right);
                    }
                } else if abs_y > THRESHOLD {
                    // Y is dominant
                    if has_up {
                        directions.push(Direction::Up);
                    } else if has_down {
                        directions.push(Direction::Down);
                    }
                }
                // If neither axis dominates, return empty (centered)
            }
            DpadMode::Disabled => {
                // No directions
            }
        }

        directions
    }
}

/// Convert percentage to raw deadzone value
fn percentage_to_raw(percentage: u8) -> u16 {
    (percentage as u32 * MAX_ABS_VALUE as u32 / 100) as u16
}

/// Convert raw deadzone value to percentage
fn raw_to_percentage(raw: u16) -> u8 {
    (raw as u32 * 100 / MAX_ABS_VALUE as u32) as u8
}

/// Parse response curve string to ResponseCurve enum
fn parse_response_curve(s: &str) -> Result<ResponseCurve, String> {
    match s.to_lowercase().as_str() {
        "linear" => Ok(ResponseCurve::Linear),
        s if s.starts_with("exponential") => {
            // Parse exponent: "exponential" or "exponential(2.0)"
            if s == "exponential" {
                Ok(ResponseCurve::Exponential { exponent: 2.0 })
            } else if s.starts_with("exponential(") && s.ends_with(')') {
                let inner = &s[12..s.len() - 1]; // Extract content between parentheses
                let exponent: f32 = inner
                    .parse()
                    .map_err(|_| format!("Invalid exponent: {}", inner))?;
                Ok(ResponseCurve::Exponential { exponent })
            } else {
                Err(format!("Invalid exponential format: {}", s))
            }
        }
        _ => Err(format!("Invalid response curve: {}", s)),
    }
}

/// Convert ResponseCurve enum to string
fn response_curve_to_string(curve: ResponseCurve) -> String {
    match curve {
        ResponseCurve::Linear => "linear".to_string(),
        ResponseCurve::Exponential { exponent } => {
            if (exponent - 2.0).abs() < 0.01 {
                "exponential".to_string()
            } else {
                format!("exponential({})", exponent)
            }
        }
    }
}

/// Parse D-pad mode string to DpadMode enum
fn parse_dpad_mode(s: &str) -> Result<DpadMode, String> {
    match s.to_lowercase().as_str() {
        "disabled" => Ok(DpadMode::Disabled),
        "eight_way" => Ok(DpadMode::EightWay),
        "four_way" => Ok(DpadMode::FourWay),
        _ => Err(format!("Invalid D-pad mode: {}", s)),
    }
}

/// Convert DpadMode enum to string
fn dpad_mode_to_string(mode: DpadMode) -> String {
    match mode {
        DpadMode::Disabled => "disabled".to_string(),
        DpadMode::EightWay => "eight_way".to_string(),
        DpadMode::FourWay => "four_way".to_string(),
    }
}

impl Default for AnalogProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_processor() -> AnalogProcessor {
        AnalogProcessor::new()
    }

    #[tokio::test]
    async fn test_analog_processor_creation() {
        let processor = create_test_processor();
        let devices = processor.get_configured_devices().await;
        assert!(devices.is_empty(), "New processor should have no devices");
    }

    #[tokio::test]
    async fn test_deadzone_filtering() {
        let processor = create_test_processor();
        let device_id = "test_device";

        // Value within deadzone (14000) should return None
        let result = processor.process_event(device_id, 61000, 10000).await;
        assert!(result.is_none(), "Value within deadzone should be filtered");

        // Negative value within deadzone should return None
        let result = processor.process_event(device_id, 61000, -10000).await;
        assert!(result.is_none(), "Negative value within deadzone should be filtered");

        // Center value should return None
        let result = processor.process_event(device_id, 61000, 0).await;
        assert!(result.is_none(), "Center value should be filtered");
    }

    #[tokio::test]
    async fn test_deadzone_passthrough() {
        let processor = create_test_processor();
        let device_id = "test_device";

        // Value outside deadzone should be processed
        let result = processor.process_event(device_id, 61000, 25000).await;
        assert!(result.is_some(), "Value outside deadzone should pass through");

        // Negative value outside deadzone should be processed
        let result = processor.process_event(device_id, 61000, -25000).await;
        assert!(result.is_some(), "Negative value outside deadzone should pass through");

        // Max value should be processed
        let result = processor.process_event(device_id, 61000, 32767).await;
        assert!(result.is_some(), "Max value should pass through");

        // Min value should be processed
        let result = processor.process_event(device_id, 61000, -32768).await;
        assert!(result.is_some(), "Min value should pass through");
    }

    #[tokio::test]
    async fn test_sensitivity_multiplier() {
        let processor = create_test_processor();
        let device_id = "test_device";

        // Set sensitivity to 2.0
        processor.set_sensitivity(device_id, 2.0).await;

        // Process same value with different sensitivity
        let output_default = create_test_processor()
            .process_event(device_id, 61000, 25000)
            .await;
        let output_boosted = processor.process_event(device_id, 61000, 25000).await;

        assert!(output_default.is_some());
        assert!(output_boosted.is_some());

        // Boosted output should be higher (approximately 2x)
        let default_val = output_default.unwrap();
        let boosted_val = output_boosted.unwrap();
        assert!(
            boosted_val.abs() > default_val.abs(),
            "Sensitivity 2.0 should produce higher output"
        );
    }

    #[tokio::test]
    async fn test_linear_response_curve() {
        let processor = create_test_processor();
        let device_id = "test_device";

        // Linear is default
        let config = processor.get_or_create_device_config(device_id).await;
        assert!(matches!(config.response_curve, ResponseCurve::Linear));

        // Process value
        let result = processor.process_event(device_id, 61000, 25000).await;
        assert!(result.is_some());

        // Set explicit linear curve
        processor
            .set_response_curve(device_id, ResponseCurve::Linear)
            .await;

        let result2 = processor.process_event(device_id, 61000, 25000).await;
        assert!(result2.is_some());

        // Results should be similar (linear = no transformation)
        let val1 = result.unwrap();
        let val2 = result2.unwrap();
        assert_eq!(val1, val2, "Linear curve should produce same output");
    }

    #[tokio::test]
    async fn test_exponential_response_curve() {
        let processor = create_test_processor();
        let device_id = "test_device";

        // Test exponential curve with exponent 2.0
        processor
            .set_response_curve(device_id, ResponseCurve::Exponential { exponent: 2.0 })
            .await;

        let result = processor.process_event(device_id, 61000, 25000).await;
        assert!(result.is_some());

        // Exponential curve should reduce small values more than large values
        let linear_result = create_test_processor()
            .process_event(device_id, 61000, 25000)
            .await;
        let exp_result = result.unwrap();
        let linear_val = linear_result.unwrap();

        // With exponent 2.0, medium values should be lower than linear
        assert!(
            exp_result.abs() < linear_val.abs(),
            "Exponential curve should reduce medium values"
        );
    }

    #[tokio::test]
    async fn test_per_device_config() {
        let processor = create_test_processor();
        let device1 = "device1";
        let device2 = "device2";

        // Set different deadzones
        processor.set_deadzone(device1, 10000).await;
        processor.set_deadzone(device2, 20000).await;

        // Same value should have different results
        let result1 = processor.process_event(device1, 61000, 15000).await;
        let result2 = processor.process_event(device2, 61000, 15000).await;

        // Device1 (deadzone 10000) should pass through 15000
        assert!(result1.is_some(), "Device1 should pass through value 15000");

        // Device2 (deadzone 20000) should filter 15000
        assert!(result2.is_none(), "Device2 should filter value 15000");
    }

    #[tokio::test]
    async fn test_default_config() {
        let processor = create_test_processor();
        let device_id = "new_device";

        // New device should get default config
        let config = processor.get_or_create_device_config(device_id).await;
        assert_eq!(config.deadzone, DEFAULT_DEADZONE);
        assert_eq!(config.sensitivity, 1.0);
        assert_eq!(config.response_curve, ResponseCurve::Linear);

        // Config should persist for subsequent calls
        let config2 = processor.get_or_create_device_config(device_id).await;
        assert_eq!(config.device_id, config2.device_id);
    }

    #[tokio::test]
    async fn test_remove_device_config() {
        let processor = create_test_processor();
        let device_id = "test_device";

        // Create config
        processor.set_deadzone(device_id, 15000).await;

        // Verify it exists
        let config = processor.get_device_config(device_id).await;
        assert!(config.is_some());

        // Remove it
        processor.remove_device_config(device_id).await;

        // Verify it's gone
        let config = processor.get_device_config(device_id).await;
        assert!(config.is_none());
    }

    #[tokio::test]
    async fn test_get_configured_devices() {
        let processor = create_test_processor();

        // Initially empty
        let devices = processor.get_configured_devices().await;
        assert!(devices.is_empty());

        // Add some devices
        processor.set_deadzone("device1", 10000).await;
        processor.set_deadzone("device2", 15000).await;
        processor.set_deadzone("device3", 20000).await;

        // Should have 3 devices
        let devices = processor.get_configured_devices().await;
        assert_eq!(devices.len(), 3);

        // Remove one
        processor.remove_device_config("device2").await;

        // Should have 2 devices
        let devices = processor.get_configured_devices().await;
        assert_eq!(devices.len(), 2);
    }

    #[tokio::test]
    async fn test_exponential_clamping() {
        let processor = create_test_processor();
        let device_id = "test_device";

        // Test with very high exponent (should clamp)
        processor
            .set_response_curve(device_id, ResponseCurve::Exponential { exponent: 10.0 })
            .await;

        let result = processor.process_event(device_id, 61000, 25000).await;
        assert!(result.is_some(), "Exponential with high exponent should work");

        // Test with very low exponent (should clamp)
        processor
            .set_response_curve(device_id, ResponseCurve::Exponential { exponent: 0.01 })
            .await;

        let result = processor.process_event(device_id, 61000, 25000).await;
        assert!(result.is_some(), "Exponential with low exponent should work");
    }

    #[tokio::test]
    async fn test_sensitivity_clamping() {
        let processor = create_test_processor();
        let device_id = "test_device";

        // Test with sensitivity too high (should clamp to 5.0)
        processor.set_sensitivity(device_id, 10.0).await;
        let config = processor.get_device_config(device_id).await;
        assert!(config.is_some());
        assert_eq!(config.unwrap().sensitivity, 5.0);

        // Test with sensitivity too low (should clamp to 0.1)
        processor.set_sensitivity(device_id, 0.01).await;
        let config = processor.get_device_config(device_id).await;
        assert!(config.is_some());
        assert_eq!(config.unwrap().sensitivity, 0.1);
    }

    #[tokio::test]
    async fn test_axis_codes() {
        let processor = create_test_processor();
        let device_id = "test_device";

        // Test all supported axis codes
        let axis_codes = [61000, 61001, 61002, 61003, 61004, 61005];

        for axis in axis_codes {
            let result = processor.process_event(device_id, axis, 25000).await;
            assert!(
                result.is_some(),
                "Axis code {} should be supported",
                axis
            );
        }
    }

    #[tokio::test]
    async fn test_dpad_mode_disabled_by_default() {
        let processor = create_test_processor();
        let device_id = "test_device";

        // Default should be Disabled
        let mode = processor.get_dpad_mode(device_id).await;
        assert_eq!(mode, DpadMode::Disabled);

        // New device config should have Disabled
        let config = processor.get_or_create_device_config(device_id).await;
        assert_eq!(config.dpad_mode, DpadMode::Disabled);
    }

    #[tokio::test]
    async fn test_set_dpad_mode() {
        let processor = create_test_processor();
        let device_id = "test_device";

        // Set to EightWay
        processor.set_dpad_mode(device_id, DpadMode::EightWay).await;
        let mode = processor.get_dpad_mode(device_id).await;
        assert_eq!(mode, DpadMode::EightWay);

        // Set to FourWay
        processor.set_dpad_mode(device_id, DpadMode::FourWay).await;
        let mode = processor.get_dpad_mode(device_id).await;
        assert_eq!(mode, DpadMode::FourWay);

        // Set to Disabled
        processor.set_dpad_mode(device_id, DpadMode::Disabled).await;
        let mode = processor.get_dpad_mode(device_id).await;
        assert_eq!(mode, DpadMode::Disabled);
    }

    #[tokio::test]
    async fn test_dpad_eight_way_cardinals() {
        // Test 8-way mode - cardinal directions

        // Up (Y = -1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(0.0, -1.0, DpadMode::EightWay);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], Direction::Up);

        // Down (Y = 1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(0.0, 1.0, DpadMode::EightWay);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], Direction::Down);

        // Left (X = -1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(-1.0, 0.0, DpadMode::EightWay);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], Direction::Left);

        // Right (X = 1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(1.0, 0.0, DpadMode::EightWay);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], Direction::Right);
    }

    #[tokio::test]
    async fn test_dpad_eight_way_diagonals() {
        // Test 8-way mode - diagonal directions return two keys

        // Up-Right (X = 1.0, Y = -1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(1.0, -1.0, DpadMode::EightWay);
        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains(&Direction::Up));
        assert!(dirs.contains(&Direction::Right));

        // Down-Right (X = 1.0, Y = 1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(1.0, 1.0, DpadMode::EightWay);
        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains(&Direction::Down));
        assert!(dirs.contains(&Direction::Right));

        // Down-Left (X = -1.0, Y = 1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(-1.0, 1.0, DpadMode::EightWay);
        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains(&Direction::Down));
        assert!(dirs.contains(&Direction::Left));

        // Up-Left (X = -1.0, Y = -1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(-1.0, -1.0, DpadMode::EightWay);
        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains(&Direction::Up));
        assert!(dirs.contains(&Direction::Left));
    }

    #[tokio::test]
    async fn test_dpad_four_way_only_cardinals() {
        // Test 4-way mode - only cardinal directions, diagonals ignored

        // Up (Y = -1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(0.0, -1.0, DpadMode::FourWay);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], Direction::Up);

        // Down (Y = 1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(0.0, 1.0, DpadMode::FourWay);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], Direction::Down);

        // Left (X = -1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(-1.0, 0.0, DpadMode::FourWay);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], Direction::Left);

        // Right (X = 1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(1.0, 0.0, DpadMode::FourWay);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], Direction::Right);
    }

    #[tokio::test]
    async fn test_dpad_four_way_diagonals_pick_dominant() {
        // Test 4-way mode - diagonals pick dominant axis

        // Up-Right (X = 1.0, Y = -1.0) - equal, but Y wins due to order
        let dirs = AnalogProcessor::map_analog_to_dpad(1.0, -1.0, DpadMode::FourWay);
        assert_eq!(dirs.len(), 1);
        // Y is dominant (abs_y > abs_x is false when equal, so X wins in implementation)
        // Actually with equal values, the implementation picks neither since neither is > the other
        // Let's test with dominant axis

        // Up-Right with dominant Y (X = 0.5, Y = -1.0)
        let dirs = AnalogProcessor::map_analog_to_dpad(0.5, -1.0, DpadMode::FourWay);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], Direction::Up);

        // Up-Right with dominant X (X = 1.0, Y = -0.5)
        let dirs = AnalogProcessor::map_analog_to_dpad(1.0, -0.5, DpadMode::FourWay);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], Direction::Right);
    }

    #[tokio::test]
    async fn test_dpad_centered_returns_empty() {
        // Centered stick (0.0, 0.0) should return no directions

        let dirs = AnalogProcessor::map_analog_to_dpad(0.0, 0.0, DpadMode::EightWay);
        assert_eq!(dirs.len(), 0);

        let dirs = AnalogProcessor::map_analog_to_dpad(0.0, 0.0, DpadMode::FourWay);
        assert_eq!(dirs.len(), 0);
    }

    #[tokio::test]
    async fn test_dpad_threshold_filters_small_movements() {
        // Small movements below threshold (0.3) should be filtered

        // Small X movement (0.2)
        let dirs = AnalogProcessor::map_analog_to_dpad(0.2, 0.0, DpadMode::EightWay);
        assert_eq!(dirs.len(), 0);

        // Small Y movement (0.2)
        let dirs = AnalogProcessor::map_analog_to_dpad(0.0, 0.2, DpadMode::EightWay);
        assert_eq!(dirs.len(), 0);

        // At threshold (0.3) should NOT trigger (uses > not >=)
        let dirs = AnalogProcessor::map_analog_to_dpad(0.3, 0.0, DpadMode::EightWay);
        assert_eq!(dirs.len(), 0);

        // Above threshold (0.31) should trigger
        let dirs = AnalogProcessor::map_analog_to_dpad(0.31, 0.0, DpadMode::EightWay);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], Direction::Right);
    }

    #[tokio::test]
    async fn test_config_save_load_roundtrip() {
        // Test save/load roundtrip preserves all values
        let processor = create_test_processor();
        let device_id = "test_device";

        // Configure device with non-default values
        processor.set_deadzone_percentage_x(device_id, 30).await.unwrap();
        processor.set_deadzone_percentage_y(device_id, 60).await.unwrap();
        processor.set_sensitivity(device_id, 2.0).await;
        processor.set_response_curve(device_id, ResponseCurve::Exponential { exponent: 3.0 }).await;
        processor.set_dpad_mode(device_id, DpadMode::EightWay).await;

        // Save config
        let saved = processor.save_config(device_id).await.unwrap();
        assert!((saved.deadzone_percentage_x as i32 - 30i32).abs() <= 1);
        assert!((saved.deadzone_percentage_y as i32 - 60i32).abs() <= 1);
        assert_eq!(saved.sensitivity, 2.0);
        assert_eq!(saved.response_curve, "exponential(3)");
        assert_eq!(saved.dpad_mode, "eight_way");

        // Create new processor and load config
        let processor2 = AnalogProcessor::new();
        processor2.load_config(device_id, &saved).await.unwrap();

        // Verify all values were loaded correctly (with rounding tolerance)
        let loaded_config = processor2.get_device_config(device_id).await.unwrap();
        // Allow 2% tolerance for double-conversion (percentage -> raw -> percentage)
        assert!((raw_to_percentage(loaded_config.inner_deadzone_x) as i32 - 30i32).abs() <= 2);
        assert!((raw_to_percentage(loaded_config.inner_deadzone_y) as i32 - 60i32).abs() <= 2);
        assert_eq!(loaded_config.sensitivity, 2.0);
        assert_eq!(loaded_config.response_curve, ResponseCurve::Exponential { exponent: 3.0 });
        assert_eq!(loaded_config.dpad_mode, DpadMode::EightWay);
    }

    #[tokio::test]
    async fn test_config_default_values() {
        // Test that unset fields use defaults
        let processor = create_test_processor();
        let device_id = "test_device";

        // Load minimal config (only sensitivity specified)
        let minimal_config = crate::config::AnalogDeviceConfig {
            deadzone_percentage: 43,
            deadzone_percentage_x: 43,
            deadzone_percentage_y: 43,
            outer_deadzone_percentage: 100,
            outer_deadzone_percentage_x: 100,
            outer_deadzone_percentage_y: 100,
            sensitivity: 1.5,
            response_curve: "linear".to_string(),
            dpad_mode: "disabled".to_string(),
        };

        processor.load_config(device_id, &minimal_config).await.unwrap();

        // Verify values were applied
        let config = processor.get_device_config(device_id).await.unwrap();
        assert_eq!(config.sensitivity, 1.5);
        assert_eq!(config.response_curve, ResponseCurve::Linear);
        assert_eq!(config.dpad_mode, DpadMode::Disabled);
    }

    #[tokio::test]
    async fn test_config_percentage_conversion() {
        // Test raw/percentage conversion accuracy
        let processor = create_test_processor();
        let device_id = "test_device";

        // Test common percentage values
        for percentage in [0u8, 20, 43, 50, 80, 100].iter() {
            let raw = percentage_to_raw(*percentage);
            let converted = raw_to_percentage(raw);

            // Allow small rounding errors (1% tolerance)
            assert!(
                (converted as i32 - *percentage as i32).abs() <= 1,
                "Percentage conversion failed: {}% -> {} raw -> {}%",
                percentage, raw, converted
            );
        }
    }

    #[tokio::test]
    async fn test_load_applies_all_settings() {
        // Test that load_config() sets every field
        let processor = create_test_processor();
        let device_id = "test_device";

        let config = crate::config::AnalogDeviceConfig {
            deadzone_percentage: 25,
            deadzone_percentage_x: 30,
            deadzone_percentage_y: 70,
            outer_deadzone_percentage: 95,
            outer_deadzone_percentage_x: 90,
            outer_deadzone_percentage_y: 95,
            sensitivity: 1.8,
            response_curve: "exponential(2.5)".to_string(),
            dpad_mode: "four_way".to_string(),
        };

        processor.load_config(device_id, &config).await.unwrap();

        // Verify all fields were applied (with rounding tolerance)
        let device_config = processor.get_device_config(device_id).await.unwrap();
        assert!((raw_to_percentage(device_config.inner_deadzone_x) as i32 - 30i32).abs() <= 1);
        assert!((raw_to_percentage(device_config.inner_deadzone_y) as i32 - 70i32).abs() <= 1);
        assert!((raw_to_percentage(device_config.outer_deadzone_x) as i32 - 90i32).abs() <= 1);
        assert!((raw_to_percentage(device_config.outer_deadzone_y) as i32 - 95i32).abs() <= 1);
        assert_eq!(device_config.sensitivity, 1.8);
        assert_eq!(device_config.response_curve, ResponseCurve::Exponential { exponent: 2.5 });
        assert_eq!(device_config.dpad_mode, DpadMode::FourWay);
    }

    #[tokio::test]
    async fn test_save_serializes_all_settings() {
        // Test that save_config() includes every field
        let processor = create_test_processor();
        let device_id = "test_device";

        // Set all fields
        processor.set_deadzone_percentage_x(device_id, 40).await.unwrap();
        processor.set_deadzone_percentage_y(device_id, 75).await.unwrap();
        processor.set_outer_deadzone_x(device_id, (90f32 * 32767.0 / 100.0) as u16).await;
        processor.set_outer_deadzone_y(device_id, (85f32 * 32767.0 / 100.0) as u16).await;
        processor.set_sensitivity(device_id, 2.2).await;
        processor.set_response_curve(device_id, ResponseCurve::Exponential { exponent: 4.0 }).await;
        processor.set_dpad_mode(device_id, DpadMode::EightWay).await;

        // Save config
        let saved = processor.save_config(device_id).await.unwrap();

        // Verify all fields are present (with rounding tolerance)
        assert!((saved.deadzone_percentage_x as i32 - 40i32).abs() <= 1);
        assert!((saved.deadzone_percentage_y as i32 - 75i32).abs() <= 1);
        assert!((saved.outer_deadzone_percentage_x as i32 - 90i32).abs() <= 1);
        assert!((saved.outer_deadzone_percentage_y as i32 - 85i32).abs() <= 1);
        assert_eq!(saved.sensitivity, 2.2);
        assert_eq!(saved.response_curve, "exponential(4)");
        assert_eq!(saved.dpad_mode, "eight_way");
    }

    #[tokio::test]
    async fn test_multiple_devices_configs() {
        // Test that different devices have independent configs
        let processor = create_test_processor();
        let device1 = "device1";
        let device2 = "device2";

        // Configure device1
        processor.set_deadzone_percentage_x(device1, 20).await.unwrap();
        processor.set_sensitivity(device1, 0.5).await;

        // Configure device2 differently
        processor.set_deadzone_percentage_x(device2, 80).await.unwrap();
        processor.set_sensitivity(device2, 3.0).await;

        // Verify configs are independent (with rounding tolerance)
        let config1 = processor.get_device_config(device1).await.unwrap();
        let config2 = processor.get_device_config(device2).await.unwrap();

        assert!((raw_to_percentage(config1.inner_deadzone_x) as i32 - 20i32).abs() <= 1);
        assert_eq!(config1.sensitivity, 0.5);

        assert!((raw_to_percentage(config2.inner_deadzone_x) as i32 - 80i32).abs() <= 1);
        assert_eq!(config2.sensitivity, 3.0);
    }

    // Tests for the new process() method with AnalogCalibration

    #[test]
    fn test_process_center_position() {
        let processor = create_test_processor();
        let calibration = AnalogCalibration::default();

        // Center position (128, 128) should output near center of range
        let (x, y) = processor.process(&calibration, 128, 128);

        // Output should be near center of range (-32768 to 32767)
        // Allow small tolerance for floating point math
        assert!(x.abs() < 1000, "X should be near center, got {}", x);
        assert!(y.abs() < 1000, "Y should be near center, got {}", y);
    }

    #[test]
    fn test_process_deadzone_filtering() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.2; // 20% deadzone

        // Small movements below deadzone should return (0, 0)
        let (x, y) = processor.process(&calibration, 130, 130);
        assert_eq!(x, 0, "Small movement should be filtered");
        assert_eq!(y, 0, "Small movement should be filtered");
    }

    #[test]
    fn test_process_circular_deadzone() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.15;
        calibration.deadzone_shape = DeadzoneShape::Circular;

        // Diagonal movement (145, 145) should be filtered by circular deadzone
        let (x, y) = processor.process(&calibration, 145, 145);
        // Both axes are equal, magnitude is sqrt(0.5^2 + 0.5^2) ~ 0.707 from corner
        // Actually (145-128)/255 = 0.067 from center, less than 0.15 deadzone
        // So this should be filtered
        assert_eq!(x, 0, "Small diagonal movement should be filtered");
        assert_eq!(y, 0, "Small diagonal movement should be filtered");

        // Large diagonal movement (200, 200) should pass through
        let (x, y) = processor.process(&calibration, 200, 200);
        assert!(x != 0 || y != 0, "Large diagonal movement should not be filtered");
    }

    #[test]
    fn test_process_square_deadzone() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.15;
        calibration.deadzone_shape = DeadzoneShape::Square;

        // Axis-aligned movement below deadzone should be filtered
        let (x, y) = processor.process(&calibration, 140, 128);
        // (140-128)/255 = 0.047 < 0.15, filtered
        assert_eq!(x, 0, "Small X movement should be filtered");

        // Axis-aligned movement above deadzone should pass through
        let (x, y) = processor.process(&calibration, 180, 128);
        assert!(x != 0, "Large X movement should not be filtered");
    }

    #[test]
    fn test_process_linear_sensitivity() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        // Use zero deadzone to get full range output
        calibration.deadzone = 0.0;

        // Full deflection should give full range output
        let (x, y) = processor.process(&calibration, 255, 255);

        // Should be near max range (allow some tolerance)
        assert!(x > 30000, "Full X should give large output, got {}", x);
        assert!(y > 30000 || y < -30000, "Full Y should give large output, got {}", y);
    }

    #[test]
    fn test_process_quadratic_sensitivity() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.sensitivity = SensitivityCurve::Quadratic;

        // Compare quadratic vs linear at same input
        let (qx, qy) = processor.process(&calibration, 200, 128);
        calibration.sensitivity = SensitivityCurve::Linear;
        let (lx, ly) = processor.process(&calibration, 200, 128);

        // Quadratic should reduce medium values compared to linear
        // (because x^2 < x for 0 < x < 1)
        assert!(qx.abs() < lx.abs(), "Quadratic should reduce medium X values");
    }

    #[test]
    fn test_process_exponential_sensitivity() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.sensitivity = SensitivityCurve::Exponential { exponent: 3.0 };

        // Exponential with high exponent should reduce medium values more
        let (ex, ey) = processor.process(&calibration, 200, 128);

        calibration.sensitivity = SensitivityCurve::Linear;
        let (lx, ly) = processor.process(&calibration, 200, 128);

        // Exponential should reduce values more than linear at medium inputs
        assert!(ex.abs() < lx.abs(), "Exponential should reduce medium X values");
    }

    #[test]
    fn test_process_range_scaling() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.range_min = -100;
        calibration.range_max = 100;

        // Full deflection should clamp to max range
        let (x, y) = processor.process(&calibration, 255, 0);

        assert!(x <= 100, "X should be clamped to max range");
        assert!(y >= -100, "Y should be clamped to min range");
        assert!(x >= -100, "X should be at least min range");
        assert!(y <= 100, "Y should be at most max range");
    }

    #[test]
    fn test_process_inversion() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();

        // Without inversion
        let (x1, y1) = processor.process(&calibration, 255, 128);

        // With X inversion
        calibration.invert_x = true;
        let (x2, y2) = processor.process(&calibration, 255, 128);

        // X should be flipped
        assert!((x1 + x2).abs() < 1000, "Inverted X should be opposite direction");

        // With Y inversion
        calibration.invert_x = false;
        calibration.invert_y = true;
        let (x3, y3) = processor.process(&calibration, 128, 0);

        calibration.invert_y = false;
        let (x4, y4) = processor.process(&calibration, 128, 0);

        // Y should be flipped
        assert!((y3 + y4).abs() < 1000, "Inverted Y should be opposite direction");
    }

    #[test]
    fn test_process_full_pipeline() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.1;
        calibration.deadzone_shape = DeadzoneShape::Circular;
        calibration.sensitivity = SensitivityCurve::Quadratic;
        calibration.sensitivity_multiplier = 1.5;
        calibration.range_min = -32768;
        calibration.range_max = 32767;

        // Test full pipeline with large movement
        let (x, y) = processor.process(&calibration, 230, 200);

        // Should be well outside deadzone and scaled
        assert!(x != 0, "Large X movement should not be filtered");
        // Y should be negative (200 > 128, and we negate Y in center())
        assert!(y < 0, "Y=200 should give negative output");
    }

    #[test]
    fn test_process_normalize_clamps_input() {
        let processor = create_test_processor();
        let calibration = AnalogCalibration::default();

        // Values outside 0-255 range should be clamped
        let (x, y) = processor.process(&calibration, 300, -50);

        // Should not panic or give invalid output
        assert!(x >= calibration.range_min);
        assert!(x <= calibration.range_max);
        assert!(y >= calibration.range_min);
        assert!(y <= calibration.range_max);
    }

    #[test]
    fn test_process_sensitivity_multiplier() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.sensitivity_multiplier = 2.0;

        let (x1, y1) = processor.process(&calibration, 200, 128);

        calibration.sensitivity_multiplier = 0.5;
        let (x2, y2) = processor.process(&calibration, 200, 128);

        // Higher multiplier should give larger output
        assert!(x1.abs() > x2.abs(), "Higher multiplier should give larger output");
    }

    // Tests for process_as_dpad() method

    #[test]
    fn test_dpad_center_returns_empty() {
        let processor = create_test_processor();
        let calibration = AnalogCalibration::default();

        // Center position (128, 128) should return empty Vec
        let result = processor.process_as_dpad(&calibration, 128, 128);
        assert!(result.is_empty(), "Center position should return no keys");
    }

    #[test]
    fn test_dpad_cardinal_up() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        // Use zero deadzone to avoid filtering
        calibration.deadzone = 0.0;

        // Y=0 (min) is up after center() negates it
        let result = processor.process_as_dpad(&calibration, 128, 0);
        assert_eq!(result.len(), 1, "Up should return 1 key");
        assert_eq!(result[0].0, Key::KEY_UP, "Should be KEY_UP");
        assert!(result[0].1, "Key should be pressed");
    }

    #[test]
    fn test_dpad_cardinal_down() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;

        // Y=255 (max) is down after center() negates it
        let result = processor.process_as_dpad(&calibration, 128, 255);
        assert_eq!(result.len(), 1, "Down should return 1 key");
        assert_eq!(result[0].0, Key::KEY_DOWN, "Should be KEY_DOWN");
        assert!(result[0].1, "Key should be pressed");
    }

    #[test]
    fn test_dpad_cardinal_left() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;

        // X=0 is left
        let result = processor.process_as_dpad(&calibration, 0, 128);
        assert_eq!(result.len(), 1, "Left should return 1 key");
        assert_eq!(result[0].0, Key::KEY_LEFT, "Should be KEY_LEFT");
        assert!(result[0].1, "Key should be pressed");
    }

    #[test]
    fn test_dpad_cardinal_right() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;

        // X=255 is right
        let result = processor.process_as_dpad(&calibration, 255, 128);
        assert_eq!(result.len(), 1, "Right should return 1 key");
        assert_eq!(result[0].0, Key::KEY_RIGHT, "Should be KEY_RIGHT");
        assert!(result[0].1, "Key should be pressed");
    }

    #[test]
    fn test_dpad_diagonal_up_right() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;

        // X=255, Y=0 is up-right
        let result = processor.process_as_dpad(&calibration, 255, 0);
        assert_eq!(result.len(), 2, "Up-Right should return 2 keys");

        let keys: Vec<Key> = result.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&Key::KEY_UP), "Should contain KEY_UP");
        assert!(keys.contains(&Key::KEY_RIGHT), "Should contain KEY_RIGHT");

        // All keys should be pressed
        assert!(result.iter().all(|(_, pressed)| *pressed), "All keys should be pressed");
    }

    #[test]
    fn test_dpad_diagonal_down_left() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;

        // X=0, Y=255 is down-left
        let result = processor.process_as_dpad(&calibration, 0, 255);
        assert_eq!(result.len(), 2, "Down-Left should return 2 keys");

        let keys: Vec<Key> = result.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&Key::KEY_DOWN), "Should contain KEY_DOWN");
        assert!(keys.contains(&Key::KEY_LEFT), "Should contain KEY_LEFT");
    }

    #[test]
    fn test_dpad_all_eight_directions() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;

        // Test all 8 directions at their approximate angles
        // Right (0 degrees): (255, 128)
        let result = processor.process_as_dpad(&calibration, 255, 128);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, Key::KEY_RIGHT);

        // Down-Right (45 degrees): (255, 200)
        let result = processor.process_as_dpad(&calibration, 255, 200);
        assert!(result.len() >= 1);

        // Down (90 degrees): (128, 255)
        let result = processor.process_as_dpad(&calibration, 128, 255);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, Key::KEY_DOWN);

        // Down-Left (135 degrees): (0, 200)
        let result = processor.process_as_dpad(&calibration, 0, 200);
        assert!(result.len() >= 1);

        // Left (180 degrees): (0, 128)
        let result = processor.process_as_dpad(&calibration, 0, 128);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, Key::KEY_LEFT);

        // Up-Left (225 degrees): (0, 50)
        let result = processor.process_as_dpad(&calibration, 0, 50);
        assert!(result.len() >= 1);

        // Up (270 degrees): (128, 0)
        let result = processor.process_as_dpad(&calibration, 128, 0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, Key::KEY_UP);

        // Up-Right (315 degrees): (255, 50)
        let result = processor.process_as_dpad(&calibration, 255, 50);
        assert!(result.len() >= 1);
    }

    #[test]
    fn test_dpad_deadzone_filters_small_movements() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        // Set a larger deadzone
        calibration.deadzone = 0.2;

        // Small movements within deadzone should be filtered
        let result = processor.process_as_dpad(&calibration, 135, 128);
        assert!(result.is_empty(), "Small X movement should be filtered by deadzone");

        let result = processor.process_as_dpad(&calibration, 128, 135);
        assert!(result.is_empty(), "Small Y movement should be filtered by deadzone");
    }

    #[test]
    fn test_dpad_direction_threshold() {
        let processor = create_test_processor();
        let calibration = AnalogCalibration::default();

        // Very small movements should be filtered by the direction threshold (0.1)
        // even with zero deadzone
        let result = processor.process_as_dpad(&calibration, 130, 128);
        assert!(result.is_empty(), "Tiny X movement should be filtered");

        let result = processor.process_as_dpad(&calibration, 128, 130);
        assert!(result.is_empty(), "Tiny Y movement should be filtered");
    }

    #[test]
    fn test_dpad_inversion() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;

        // Normal: X=255 is right
        let result = processor.process_as_dpad(&calibration, 255, 128);
        assert_eq!(result[0].0, Key::KEY_RIGHT);

        // With X inversion: X=255 should be left
        calibration.invert_x = true;
        let result = processor.process_as_dpad(&calibration, 255, 128);
        assert_eq!(result[0].0, Key::KEY_LEFT, "X inversion should flip direction");

        // With Y inversion: Y=0 should be down
        calibration.invert_x = false;
        calibration.invert_y = true;
        let result = processor.process_as_dpad(&calibration, 128, 0);
        assert_eq!(result[0].0, Key::KEY_DOWN, "Y inversion should flip direction");
    }

    #[test]
    fn test_dpad_all_pressed_true() {
        let processor = create_test_processor();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;

        // All returned keys should have pressed=true
        let test_cases = [
            (255, 128),  // Right
            (0, 128),    // Left
            (128, 0),    // Up
            (128, 255),  // Down
            (255, 0),    // Up-Right
            (0, 255),    // Down-Left
        ];

        for (x, y) in test_cases {
            let result = processor.process_as_dpad(&calibration, x, y);
            if !result.is_empty() {
                assert!(
                    result.iter().all(|(_, pressed)| *pressed),
                    "All keys should be pressed=true for ({}, {})",
                    x, y
                );
            }
        }
    }

    #[test]
    fn test_dpad_direction_to_keys() {
        // Test the helper function directly
        let keys = dpad_direction_to_keys(DpadDirection::None);
        assert!(keys.is_empty());

        let keys = dpad_direction_to_keys(DpadDirection::Up);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], Key::KEY_UP);

        let keys = dpad_direction_to_keys(DpadDirection::Down);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], Key::KEY_DOWN);

        let keys = dpad_direction_to_keys(DpadDirection::Left);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], Key::KEY_LEFT);

        let keys = dpad_direction_to_keys(DpadDirection::Right);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], Key::KEY_RIGHT);

        // Diagonals return 2 keys
        let keys = dpad_direction_to_keys(DpadDirection::UpRight);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&Key::KEY_UP));
        assert!(keys.contains(&Key::KEY_RIGHT));

        let keys = dpad_direction_to_keys(DpadDirection::DownLeft);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&Key::KEY_DOWN));
        assert!(keys.contains(&Key::KEY_LEFT));
    }

    // Gamepad mode tests (plan 14-03)

    #[tokio::test]
    async fn test_process_as_gamepad_center() {
        let processor = create_test_processor();
        let device_id = "32b6:12f7";

        // Center position (127, 127) should be filtered by deadzone
        let result = processor.process_as_gamepad(device_id, 127, 127).await;
        assert!(result.is_none(), "Center position should be filtered by deadzone");
    }

    #[tokio::test]
    async fn test_process_as_gamepad_full_deflection() {
        let processor = create_test_processor();
        let device_id = "32b6:12f7";

        // Create device config (this is done automatically when accessing the device)
        let _config = processor.get_or_create_device_config(device_id).await;

        // Full right (255, 127) - expects x ~20000 after deadzone scaling
        if let Some((x, y)) = processor.process_as_gamepad(device_id, 255, 127).await {
            assert!(x > 15000, "X should be high positive, got {}", x);
            assert!(y.abs() < 5000, "Y should be near center, got {}", y);
        } else {
            panic!("Full right deflection should produce output");
        }

        // Full up (127, 0) - expects y ~-20000 (inverted Y)
        if let Some((x, y)) = processor.process_as_gamepad(device_id, 127, 0).await {
            assert!(x.abs() < 5000, "X should be near center, got {}", x);
            assert!(y < -15000, "Y should be high negative (up), got {}", y);
        } else {
            panic!("Full up deflection should produce output");
        }

        // Full left (0, 127)
        if let Some((x, y)) = processor.process_as_gamepad(device_id, 0, 127).await {
            assert!(x < -15000, "X should be high negative, got {}", x);
            assert!(y.abs() < 5000, "Y should be near center, got {}", y);
        } else {
            panic!("Full left deflection should produce output");
        }

        // Full down (127, 255)
        if let Some((x, y)) = processor.process_as_gamepad(device_id, 127, 255).await {
            assert!(x.abs() < 5000, "X should be near center, got {}", x);
            assert!(y > 15000, "Y should be high positive (down), got {}", y);
        } else {
            panic!("Full down deflection should produce output");
        }
    }

    #[tokio::test]
    async fn test_process_as_gamepad_diagonal() {
        let processor = create_test_processor();
        let device_id = "32b6:12f7";

        // Create device config
        let _config = processor.get_or_create_device_config(device_id).await;

        // Up-right diagonal (255, 0) - expects both axes high after deadzone
        if let Some((x, y)) = processor.process_as_gamepad(device_id, 255, 0).await {
            assert!(x > 10000, "X should be positive, got {}", x);
            assert!(y < -10000, "Y should be negative (up), got {}", y);
        } else {
            panic!("Diagonal deflection should produce output");
        }

        // Down-left diagonal (0, 255)
        if let Some((x, y)) = processor.process_as_gamepad(device_id, 0, 255).await {
            assert!(x < -10000, "X should be negative, got {}", x);
            assert!(y > 10000, "Y should be positive (down), got {}", y);
        } else {
            panic!("Diagonal deflection should produce output");
        }
    }

    #[tokio::test]
    async fn test_process_as_gamepad_edge_positions() {
        let processor = create_test_processor();
        let device_id = "test_device";

        // Create device config
        let _config = processor.get_or_create_device_config(device_id).await;

        // The default deadzone for DeviceAnalogConfig is DEFAULT_DEADZONE (14000 out of 32767)
        // In normalized centered coordinates this is about 0.21
        // Full range in centered coords is -0.5 to 0.5

        // Value clearly outside deadzone (200, 127) - should pass
        let result = processor.process_as_gamepad(device_id, 200, 127).await;
        assert!(result.is_some(), "Value (200, 127) should produce output");

        // Value near center (135, 127) - might be filtered or near edge
        let _result = processor.process_as_gamepad(device_id, 135, 127).await;
        // The important thing is the center (127, 127) is filtered
    }

    #[tokio::test]
    async fn test_process_as_gamepad_unknown_device() {
        let processor = create_test_processor();

        // Unknown device should return None (no config)
        let result = processor.process_as_gamepad("unknown:1234", 255, 127).await;
        assert!(result.is_none(), "Unknown device should return None");
    }

    #[test]
    fn test_process_2d_center() {
        let calibration = AnalogCalibration::default();

        // Center position should be filtered by deadzone
        let result = AnalogProcessor::process_2d(127, 127, &calibration);
        assert!(result.is_none(), "Center position should be filtered by deadzone");
    }

    #[test]
    fn test_process_2d_full_deflection() {
        let calibration = AnalogCalibration::default();

        // Full right - after normalization and deadzone scaling
        // Input (255, 127) -> normalized (1.0, 0.498) -> centered (0.5, ~0)
        // After deadzone (15%): scaled to ~0.31 -> output ~20000
        if let Some((x, y)) = AnalogProcessor::process_2d(255, 127, &calibration) {
            assert!(x > 15000, "X should be high positive, got {}", x);
            assert!(y.abs() < 5000, "Y should be near center, got {}", y);
        } else {
            panic!("Full right deflection should produce output");
        }

        // Full up (Y inverted: 0 -> centered +0.5)
        // Input (127, 0) -> normalized (0.498, 0) -> centered (~0, 0.5)
        // After deadzone and scaling: output ~20000
        if let Some((x, y)) = AnalogProcessor::process_2d(127, 0, &calibration) {
            assert!(x.abs() < 5000, "X should be near center, got {}", x);
            // Without invert_y: physical up (raw_y=0) gives positive output ~20000
            // The test_process_2d uses default calibration (no inversion)
            assert!(y > 15000, "Y should be high positive, got {}", y);
        } else {
            panic!("Full up deflection should produce output");
        }
    }

    #[test]
    fn test_apply_deadzone_static_circular() {
        let calibration = AnalogCalibration::with_deadzone(0.2);

        // Within deadzone
        let (x, y) = AnalogProcessor::apply_deadzone_static(0.1, 0.1, &calibration);
        assert_eq!(x, 0.0, "Should be filtered");
        assert_eq!(y, 0.0, "Should be filtered");

        // Outside deadzone
        let (x, y) = AnalogProcessor::apply_deadzone_static(0.4, 0.0, &calibration);
        assert!(x > 0.0, "Should be scaled outward");
        assert_eq!(y, 0.0, "Y should remain 0");
    }

    #[test]
    fn test_apply_deadzone_static_square() {
        let calibration = AnalogCalibration {
            deadzone: 0.2,
            deadzone_shape: DeadzoneShape::Square,
            ..Default::default()
        };

        // Within deadzone on X
        let (x, y) = AnalogProcessor::apply_deadzone_static(0.1, 0.3, &calibration);
        assert_eq!(x, 0.0, "X should be filtered");
        assert!(y > 0.0, "Y should pass through");

        // Both outside deadzone
        let (x, y) = AnalogProcessor::apply_deadzone_static(0.3, 0.3, &calibration);
        assert!(x > 0.0, "X should pass through");
        assert!(y > 0.0, "Y should pass through");
    }

    #[test]
    fn test_apply_sensitivity_static_linear() {
        let calibration = AnalogCalibration::default();

        // Linear should maintain direction
        let (x, y) = AnalogProcessor::apply_sensitivity_static(0.5, 0.0, &calibration);
        assert!(x > 0.0, "X should be positive");
        assert!(y.abs() < 0.01, "Y should be near 0, got {}", y);

        let (x, y) = AnalogProcessor::apply_sensitivity_static(0.0, 0.5, &calibration);
        assert!(x.abs() < 0.01, "X should be near 0, got {}", x);
        assert!(y > 0.0, "Y should be positive");
    }

    #[test]
    fn test_apply_sensitivity_static_quadratic() {
        let calibration = AnalogCalibration {
            sensitivity: SensitivityCurve::Quadratic,
            ..Default::default()
        };

        // Quadratic should reduce small values more than linear
        let (x1, _) = AnalogProcessor::apply_sensitivity_static(0.3, 0.0, &AnalogCalibration::default());
        let (x2, _) = AnalogProcessor::apply_sensitivity_static(0.3, 0.0, &calibration);

        assert!(x2 < x1, "Quadratic should produce smaller output for same input");
    }

    #[test]
    fn test_scale_to_output_static() {
        let calibration = AnalogCalibration::default();

        // Center should map to center of output range
        let (x, _y) = AnalogProcessor::scale_to_output_static(0.0, 0.0, &calibration);
        assert!(x.abs() < 100, "X should be near 0");

        // Max input should map to max output
        let (x, _y) = AnalogProcessor::scale_to_output_static(0.5, 0.0, &calibration);
        assert!(x > 30000, "X should be near max");

        // Negative max should map to min output
        let (x, _y) = AnalogProcessor::scale_to_output_static(-0.5, 0.0, &calibration);
        assert!(x < -30000, "X should be near min");
    }

    #[test]
    fn test_scale_to_output_static_inversion() {
        let calibration = AnalogCalibration {
            invert_x: true,
            invert_y: true,
            ..Default::default()
        };

        // Positive X should become negative
        let (x, _y) = AnalogProcessor::scale_to_output_static(0.5, 0.0, &calibration);
        assert!(x < -30000, "Inverted X should be near min");

        // Positive Y should become negative
        let (_x, y) = AnalogProcessor::scale_to_output_static(0.0, 0.5, &calibration);
        assert!(y < -30000, "Inverted Y should be near min");
    }

    // Additional gamepad mode tests (plan 14-06)

    #[tokio::test]
    async fn test_process_as_gamepad_deadzone_filters_center() {
        let processor = AnalogProcessor::new();
        let device_id = "32b6:12f7";

        // Add device config first
        {
            let mut devices = processor.devices.write().await;
            devices.insert(device_id.to_string(), DeviceAnalogConfig::new(device_id.to_string()));
        }

        // Center position (127, 127) - within deadzone
        let result = processor.process_as_gamepad(device_id, 127, 127).await;
        assert!(result.is_none(), "Center should be filtered by deadzone");
    }

    #[tokio::test]
    async fn test_process_as_gamepad_full_right() {
        let processor = AnalogProcessor::new();
        let device_id = "32b6:12f7";

        // Add device config first
        {
            let mut devices = processor.devices.write().await;
            devices.insert(device_id.to_string(), DeviceAnalogConfig::new(device_id.to_string()));
        }

        // Full right (255, 127)
        let result = processor.process_as_gamepad(device_id, 255, 127).await;
        assert!(result.is_some(), "Full right should produce output");

        let (x, y) = result.unwrap();
        assert!(x > 15000, "X should be high positive, got {}", x);
        assert!(y.abs() < 5000, "Y should be near center, got {}", y);
    }

    #[tokio::test]
    async fn test_process_as_gamepad_full_left() {
        let processor = AnalogProcessor::new();
        let device_id = "32b6:12f7";

        // Add device config first
        {
            let mut devices = processor.devices.write().await;
            devices.insert(device_id.to_string(), DeviceAnalogConfig::new(device_id.to_string()));
        }

        let result = processor.process_as_gamepad(device_id, 0, 127).await;
        assert!(result.is_some());

        let (x, y) = result.unwrap();
        assert!(x < -15000, "X should be high negative, got {}", x);
        assert!(y.abs() < 5000, "Y should be near center, got {}", y);
    }

    #[tokio::test]
    async fn test_process_as_gamepad_full_up() {
        let processor = AnalogProcessor::new();
        let device_id = "32b6:12f7";

        // Add device config first
        {
            let mut devices = processor.devices.write().await;
            devices.insert(device_id.to_string(), DeviceAnalogConfig::new(device_id.to_string()));
        }

        let result = processor.process_as_gamepad(device_id, 127, 0).await;
        assert!(result.is_some());

        let (x, y) = result.unwrap();
        assert!(x.abs() < 5000, "X should be near center, got {}", x);
        assert!(y < -15000, "Y should be high negative (up), got {}", y);
    }

    #[tokio::test]
    async fn test_process_as_gamepad_full_down() {
        let processor = AnalogProcessor::new();
        let device_id = "32b6:12f7";

        // Add device config first
        {
            let mut devices = processor.devices.write().await;
            devices.insert(device_id.to_string(), DeviceAnalogConfig::new(device_id.to_string()));
        }

        let result = processor.process_as_gamepad(device_id, 127, 255).await;
        assert!(result.is_some());

        let (x, y) = result.unwrap();
        assert!(x.abs() < 5000, "X should be near center, got {}", x);
        assert!(y > 15000, "Y should be high positive (down), got {}", y);
    }

    #[tokio::test]
    async fn test_process_as_gamepad_diagonal_up_right() {
        let processor = AnalogProcessor::new();
        let device_id = "32b6:12f7";

        // Add device config first
        {
            let mut devices = processor.devices.write().await;
            devices.insert(device_id.to_string(), DeviceAnalogConfig::new(device_id.to_string()));
        }

        let result = processor.process_as_gamepad(device_id, 255, 0).await;
        assert!(result.is_some());

        let (x, y) = result.unwrap();
        assert!(x > 10000, "X should be positive, got {}", x);
        assert!(y < -10000, "Y should be negative (up), got {}", y);
    }

    #[tokio::test]
    async fn test_process_as_gamepad_sensitivity_affects_output() {
        let processor = AnalogProcessor::new();
        let device_id = "test_sens";

        // Set high sensitivity
        {
            let mut devices = processor.devices.write().await;
            let mut config = DeviceAnalogConfig::new(device_id.to_string());
            config.sensitivity = 2.0;  // 2x sensitivity
            devices.insert(device_id.to_string(), config);
        }

        let result = processor.process_as_gamepad(device_id, 200, 127).await;
        assert!(result.is_some());

        // With 2x sensitivity, output should be higher than default
        let (x, _) = result.unwrap();
        assert!(x > 0, "X should be positive");
    }

    #[tokio::test]
    async fn test_process_as_gamepad_with_calibration() {
        use crate::analog_calibration::{AnalogCalibration, DeadzoneShape, SensitivityCurve};

        let processor = AnalogProcessor::new();

        let calibration = AnalogCalibration {
            deadzone_shape: DeadzoneShape::Circular,
            deadzone: 0.2,  // Larger deadzone
            sensitivity: SensitivityCurve::Quadratic,
            sensitivity_multiplier: 1.5,
            range_min: -32768,
            range_max: 32767,
            invert_x: false,
            invert_y: false,
        };

        // Use process_as_gamepad_with_calibration
        let result = processor.process_as_gamepad_with_calibration(200, 127, &calibration).await;
        assert!(result.is_some(), "Should produce output outside deadzone");
    }

    // WASD mode tests (plan 15-08)

    #[test]
    fn test_wasd_direction_to_keys_all_directions() {
        // Test all 9 directions map to correct WASD keys
        let keys = wasd_direction_to_keys(DpadDirection::None);
        assert!(keys.is_empty(), "None should return empty Vec");

        let keys = wasd_direction_to_keys(DpadDirection::Up);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], Key::KEY_W, "Up should map to W");

        let keys = wasd_direction_to_keys(DpadDirection::Down);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], Key::KEY_S, "Down should map to S");

        let keys = wasd_direction_to_keys(DpadDirection::Left);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], Key::KEY_A, "Left should map to A");

        let keys = wasd_direction_to_keys(DpadDirection::Right);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], Key::KEY_D, "Right should map to D");

        let keys = wasd_direction_to_keys(DpadDirection::UpRight);
        assert_eq!(keys.len(), 2, "UpRight should return 2 keys");
        assert!(keys.contains(&Key::KEY_W), "UpRight should contain W");
        assert!(keys.contains(&Key::KEY_D), "UpRight should contain D");

        let keys = wasd_direction_to_keys(DpadDirection::UpLeft);
        assert_eq!(keys.len(), 2, "UpLeft should return 2 keys");
        assert!(keys.contains(&Key::KEY_W), "UpLeft should contain W");
        assert!(keys.contains(&Key::KEY_A), "UpLeft should contain A");

        let keys = wasd_direction_to_keys(DpadDirection::DownRight);
        assert_eq!(keys.len(), 2, "DownRight should return 2 keys");
        assert!(keys.contains(&Key::KEY_S), "DownRight should contain S");
        assert!(keys.contains(&Key::KEY_D), "DownRight should contain D");

        let keys = wasd_direction_to_keys(DpadDirection::DownLeft);
        assert_eq!(keys.len(), 2, "DownLeft should return 2 keys");
        assert!(keys.contains(&Key::KEY_S), "DownLeft should contain S");
        assert!(keys.contains(&Key::KEY_A), "DownLeft should contain A");
    }

    #[test]
    fn test_process_as_wasd_deadzone_filters_center() {
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();

        // Center position (128, 128) should return no keys
        let result = processor.process_as_wasd(&calibration, 128, 128);
        assert!(result.is_empty(), "Center position should return no keys");
    }

    #[test]
    fn test_process_as_wasd_cardinal_directions() {
        let processor = AnalogProcessor::new();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;  // Disable deadzone for testing

        // North (up) = W
        let result = processor.process_as_wasd(&calibration, 128, 0);
        assert_eq!(result.len(), 1, "North should return 1 key");
        assert_eq!(result[0].0, Key::KEY_W, "North should be W key");
        assert!(result[0].1, "Key should be pressed");

        // South (down) = S
        let result = processor.process_as_wasd(&calibration, 128, 255);
        assert_eq!(result.len(), 1, "South should return 1 key");
        assert_eq!(result[0].0, Key::KEY_S, "South should be S key");

        // West (left) = A
        let result = processor.process_as_wasd(&calibration, 0, 128);
        assert_eq!(result.len(), 1, "West should return 1 key");
        assert_eq!(result[0].0, Key::KEY_A, "West should be A key");

        // East (right) = D
        let result = processor.process_as_wasd(&calibration, 255, 128);
        assert_eq!(result.len(), 1, "East should return 1 key");
        assert_eq!(result[0].0, Key::KEY_D, "East should be D key");
    }

    #[test]
    fn test_process_as_wasd_diagonal_directions() {
        let processor = AnalogProcessor::new();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;

        // North-East = W + D
        let result = processor.process_as_wasd(&calibration, 255, 0);
        assert_eq!(result.len(), 2, "North-East should return 2 keys");
        let keys: Vec<Key> = result.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&Key::KEY_W), "North-East should contain W");
        assert!(keys.contains(&Key::KEY_D), "North-East should contain D");

        // North-West = W + A
        let result = processor.process_as_wasd(&calibration, 0, 0);
        assert_eq!(result.len(), 2, "North-West should return 2 keys");
        let keys: Vec<Key> = result.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&Key::KEY_W), "North-West should contain W");
        assert!(keys.contains(&Key::KEY_A), "North-West should contain A");

        // South-East = S + D
        let result = processor.process_as_wasd(&calibration, 255, 255);
        assert_eq!(result.len(), 2, "South-East should return 2 keys");
        let keys: Vec<Key> = result.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&Key::KEY_S), "South-East should contain S");
        assert!(keys.contains(&Key::KEY_D), "South-East should contain D");

        // South-West = S + A
        let result = processor.process_as_wasd(&calibration, 0, 255);
        assert_eq!(result.len(), 2, "South-West should return 2 keys");
        let keys: Vec<Key> = result.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&Key::KEY_S), "South-West should contain S");
        assert!(keys.contains(&Key::KEY_A), "South-West should contain A");
    }

    #[test]
    fn test_process_as_wasd_axis_inversion() {
        let processor = AnalogProcessor::new();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;

        // Normal: X=255 is right (D key)
        let result = processor.process_as_wasd(&calibration, 255, 128);
        assert_eq!(result[0].0, Key::KEY_D, "Normal X should be right (D)");

        // With X inversion: X=255 should be left (A key)
        calibration.invert_x = true;
        let result = processor.process_as_wasd(&calibration, 255, 128);
        assert_eq!(result[0].0, Key::KEY_A, "Inverted X should be left (A)");

        // With Y inversion: Y=0 should be down (S key)
        calibration.invert_x = false;
        calibration.invert_y = true;
        let result = processor.process_as_wasd(&calibration, 128, 0);
        assert_eq!(result[0].0, Key::KEY_S, "Inverted Y should be down (S)");

        // Both inverted
        calibration.invert_x = true;
        calibration.invert_y = true;
        let result = processor.process_as_wasd(&calibration, 255, 0);
        let keys: Vec<Key> = result.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&Key::KEY_A), "Double-inverted X should be left (A)");
        assert!(keys.contains(&Key::KEY_S), "Double-inverted Y should be down (S)");
    }

    #[test]
    fn test_process_as_wasd_all_pressed_true() {
        let processor = AnalogProcessor::new();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;

        // All returned keys should have pressed=true
        let test_cases = [
            (255, 128),  // Right
            (0, 128),    // Left
            (128, 0),    // Up
            (128, 255),  // Down
            (255, 0),    // Up-Right
            (0, 255),    // Down-Left
        ];

        for (x, y) in test_cases {
            let result = processor.process_as_wasd(&calibration, x, y);
            if !result.is_empty() {
                assert!(
                    result.iter().all(|(_, pressed)| *pressed),
                    "All keys should be pressed=true for ({}, {})",
                    x, y
                );
            }
        }
    }

    #[test]
    fn test_process_as_wasd_deadzone_filters_small_movements() {
        let processor = AnalogProcessor::new();
        let mut calibration = AnalogCalibration::default();
        // Set a larger deadzone
        calibration.deadzone = 0.2;

        // Small movements within deadzone should be filtered
        let result = processor.process_as_wasd(&calibration, 135, 128);
        assert!(result.is_empty(), "Small X movement should be filtered by deadzone");

        let result = processor.process_as_wasd(&calibration, 128, 135);
        assert!(result.is_empty(), "Small Y movement should be filtered by deadzone");

        // Center position definitely filtered
        let result = processor.process_as_wasd(&calibration, 128, 128);
        assert!(result.is_empty(), "Center should be filtered by deadzone");
    }

    // Mouse mode tests (plan 15-08)

    #[test]
    fn test_process_as_mouse_deadzone_filters_center() {
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();
        let config = MouseVelocityConfig::default();

        // Center position (128, 128) should return None
        let result = processor.process_as_mouse(&calibration, 128, 128, &config);
        assert!(result.is_none(), "Center position should return None");
    }

    #[test]
    fn test_process_as_mouse_full_right() {
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();
        let config = MouseVelocityConfig::default();

        // Full right (255, 128) - expects positive X velocity
        let result = processor.process_as_mouse(&calibration, 255, 128, &config);
        assert!(result.is_some(), "Full right should return velocity");

        let (vel_x, vel_y) = result.unwrap();
        assert!(vel_x > 0, "Full right should have positive X velocity, got {}", vel_x);
        assert_eq!(vel_y, 0, "Full right should have zero Y velocity");
    }

    #[test]
    fn test_process_as_mouse_full_left() {
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();
        let config = MouseVelocityConfig::default();

        // Full left (0, 128) - expects negative X velocity
        let result = processor.process_as_mouse(&calibration, 0, 128, &config);
        assert!(result.is_some(), "Full left should return velocity");

        let (vel_x, vel_y) = result.unwrap();
        assert!(vel_x < 0, "Full left should have negative X velocity, got {}", vel_x);
        assert_eq!(vel_y, 0, "Full left should have zero Y velocity");
    }

    #[test]
    fn test_process_as_mouse_full_up() {
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();
        let config = MouseVelocityConfig::default();

        // Full up (128, 0) - expects positive Y velocity
        // Note: Y is inverted during centering (cy = 0.5 - ny), so raw 0 becomes positive
        let result = processor.process_as_mouse(&calibration, 128, 0, &config);
        assert!(result.is_some(), "Full up should return velocity");

        let (vel_x, vel_y) = result.unwrap();
        assert_eq!(vel_x, 0, "Full up should have zero X velocity");
        assert!(vel_y > 0, "Full up should have positive Y velocity, got {}", vel_y);
    }

    #[test]
    fn test_process_as_mouse_full_down() {
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();
        let config = MouseVelocityConfig::default();

        // Full down (128, 255) - expects negative Y velocity
        // Note: Y is inverted during centering, so raw 255 becomes negative
        let result = processor.process_as_mouse(&calibration, 128, 255, &config);
        assert!(result.is_some(), "Full down should return velocity");

        let (vel_x, vel_y) = result.unwrap();
        assert_eq!(vel_x, 0, "Full down should have zero X velocity");
        assert!(vel_y < 0, "Full down should have negative Y velocity, got {}", vel_y);
    }

    #[test]
    fn test_process_as_mouse_multiplier_affects_velocity() {
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();

        // Test with default multiplier (10.0)
        let config_default = MouseVelocityConfig::default();
        let result1 = processor.process_as_mouse(&calibration, 255, 128, &config_default);
        assert!(result1.is_some());
        let (vel_x1, _) = result1.unwrap();

        // Test with higher multiplier (20.0)
        let config_high = MouseVelocityConfig { multiplier: 20.0 };
        let result2 = processor.process_as_mouse(&calibration, 255, 128, &config_high);
        assert!(result2.is_some());
        let (vel_x2, _) = result2.unwrap();

        // Higher multiplier should give higher velocity
        assert!(vel_x2 > vel_x1, "Higher multiplier should give higher velocity: {} > {}", vel_x2, vel_x1);

        // Test with lower multiplier (5.0)
        let config_low = MouseVelocityConfig { multiplier: 5.0 };
        let result3 = processor.process_as_mouse(&calibration, 255, 128, &config_low);
        assert!(result3.is_some());
        let (vel_x3, _) = result3.unwrap();

        // Lower multiplier should give lower velocity
        assert!(vel_x3 < vel_x1, "Lower multiplier should give lower velocity: {} < {}", vel_x3, vel_x1);
    }

    #[test]
    fn test_process_as_mouse_diagonal() {
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();
        let config = MouseVelocityConfig::default();

        // Diagonal up-right (255, 0) - expects both X and Y velocity
        let result = processor.process_as_mouse(&calibration, 255, 0, &config);
        assert!(result.is_some(), "Diagonal should return velocity");

        let (vel_x, vel_y) = result.unwrap();
        assert!(vel_x > 0, "Diagonal up-right should have positive X velocity");
        assert!(vel_y > 0, "Diagonal up-right should have positive Y velocity");

        // Diagonal down-left (0, 255)
        let result = processor.process_as_mouse(&calibration, 0, 255, &config);
        assert!(result.is_some(), "Diagonal should return velocity");

        let (vel_x, vel_y) = result.unwrap();
        assert!(vel_x < 0, "Diagonal down-left should have negative X velocity");
        assert!(vel_y < 0, "Diagonal down-left should have negative Y velocity");
    }

    #[test]
    fn test_process_as_mouse_deadzone_filters_small_movements() {
        let processor = AnalogProcessor::new();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.2;  // Larger deadzone
        let config = MouseVelocityConfig::default();

        // Small movements within deadzone should be filtered
        let result = processor.process_as_mouse(&calibration, 135, 128, &config);
        assert!(result.is_none(), "Small X movement should be filtered by deadzone");

        let result = processor.process_as_mouse(&calibration, 128, 135, &config);
        assert!(result.is_none(), "Small Y movement should be filtered by deadzone");
    }

    #[test]
    fn test_mouse_velocity_config_default() {
        let config = MouseVelocityConfig::default();
        assert_eq!(config.multiplier, 10.0, "Default multiplier should be 10.0");
    }

    #[test]
    fn test_process_as_mouse_axis_inversion() {
        let processor = AnalogProcessor::new();
        let mut calibration = AnalogCalibration::default();
        let config = MouseVelocityConfig::default();

        // Normal: X=255 is right (positive)
        let result = processor.process_as_mouse(&calibration, 255, 128, &config);
        assert!(result.is_some());
        let (vel_x, _) = result.unwrap();
        assert!(vel_x > 0, "Normal X should be positive");

        // With X inversion: X=255 should be left (negative)
        calibration.invert_x = true;
        let result = processor.process_as_mouse(&calibration, 255, 128, &config);
        assert!(result.is_some());
        let (vel_x, _) = result.unwrap();
        assert!(vel_x < 0, "Inverted X should be negative");

        // Normal Y: Y=0 is up (positive due to centering inversion)
        calibration.invert_x = false;
        calibration.invert_y = false;
        let result = processor.process_as_mouse(&calibration, 128, 0, &config);
        assert!(result.is_some());
        let (_, vel_y1) = result.unwrap();
        assert!(vel_y1 > 0, "Normal Y=0 should be positive (up)");

        // With Y inversion: Y=0 should be down (negative, flipped by invert_y)
        calibration.invert_y = true;
        let result = processor.process_as_mouse(&calibration, 128, 0, &config);
        assert!(result.is_some());
        let (_, vel_y2) = result.unwrap();
        assert!(vel_y2 < 0, "Inverted Y should flip direction");
    }

    // Camera mode tests (plan 15-08)

    #[test]
    fn test_process_as_camera_scroll_mode_deadzone() {
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();

        // Center position (128, 128) should return None
        let result = processor.process_as_camera(&calibration, 128, 128, CameraOutputMode::Scroll);
        assert!(result.is_none(), "Center position should return None in Scroll mode");
    }

    #[test]
    fn test_process_as_camera_scroll_mode_up() {
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();

        // Full up (128, 0) - Y is inverted during centering so raw 0 becomes positive
        let result = processor.process_as_camera(&calibration, 128, 0, CameraOutputMode::Scroll);
        assert!(result.is_some(), "Full up should return output");

        match result.unwrap() {
            CameraOutput::Scroll(amount) => {
                // Positive Y after centering = positive scroll (scroll up)
                assert!(amount > 0, "Full up should have positive scroll amount, got {}", amount);
            }
            _ => panic!("Should return Scroll variant"),
        }
    }

    #[test]
    fn test_process_as_camera_scroll_mode_down() {
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();

        // Full down (128, 255) - Y is inverted during centering so raw 255 becomes negative
        let result = processor.process_as_camera(&calibration, 128, 255, CameraOutputMode::Scroll);
        assert!(result.is_some(), "Full down should return output");

        match result.unwrap() {
            CameraOutput::Scroll(amount) => {
                // Negative Y after centering = negative scroll (scroll down)
                assert!(amount < 0, "Full down should have negative scroll amount, got {}", amount);
            }
            _ => panic!("Should return Scroll variant"),
        }
    }

    #[test]
    fn test_process_as_camera_key_mode_all_directions() {
        let processor = AnalogProcessor::new();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.0;  // Disable deadzone for testing

        // Test all 8 directions return correct keys

        // Up = PageUp
        let result = processor.process_as_camera(&calibration, 128, 0, CameraOutputMode::Keys);
        assert!(result.is_some());
        match result.unwrap() {
            CameraOutput::Keys(keys) => {
                assert_eq!(keys.len(), 1);
                assert_eq!(keys[0], Key::KEY_PAGEUP);
            }
            _ => panic!("Should return Keys variant"),
        }

        // Down = PageDown
        let result = processor.process_as_camera(&calibration, 128, 255, CameraOutputMode::Keys);
        assert!(result.is_some());
        match result.unwrap() {
            CameraOutput::Keys(keys) => {
                assert_eq!(keys.len(), 1);
                assert_eq!(keys[0], Key::KEY_PAGEDOWN);
            }
            _ => panic!("Should return Keys variant"),
        }

        // Left = Left arrow
        let result = processor.process_as_camera(&calibration, 0, 128, CameraOutputMode::Keys);
        assert!(result.is_some());
        match result.unwrap() {
            CameraOutput::Keys(keys) => {
                assert_eq!(keys.len(), 1);
                assert_eq!(keys[0], Key::KEY_LEFT);
            }
            _ => panic!("Should return Keys variant"),
        }

        // Right = Right arrow
        let result = processor.process_as_camera(&calibration, 255, 128, CameraOutputMode::Keys);
        assert!(result.is_some());
        match result.unwrap() {
            CameraOutput::Keys(keys) => {
                assert_eq!(keys.len(), 1);
                assert_eq!(keys[0], Key::KEY_RIGHT);
            }
            _ => panic!("Should return Keys variant"),
        }

        // Up-Right = PageUp + Right
        let result = processor.process_as_camera(&calibration, 255, 0, CameraOutputMode::Keys);
        assert!(result.is_some());
        match result.unwrap() {
            CameraOutput::Keys(keys) => {
                assert_eq!(keys.len(), 2);
                assert!(keys.contains(&Key::KEY_PAGEUP));
                assert!(keys.contains(&Key::KEY_RIGHT));
            }
            _ => panic!("Should return Keys variant"),
        }

        // Down-Left = PageDown + Left
        let result = processor.process_as_camera(&calibration, 0, 255, CameraOutputMode::Keys);
        assert!(result.is_some());
        match result.unwrap() {
            CameraOutput::Keys(keys) => {
                assert_eq!(keys.len(), 2);
                assert!(keys.contains(&Key::KEY_PAGEDOWN));
                assert!(keys.contains(&Key::KEY_LEFT));
            }
            _ => panic!("Should return Keys variant"),
        }
    }

    #[test]
    fn test_process_as_camera_sensitivity_affects_scroll() {
        let processor = AnalogProcessor::new();
        let mut calibration_low = AnalogCalibration::default();
        calibration_low.sensitivity_multiplier = 0.5;

        let mut calibration_high = AnalogCalibration::default();
        calibration_high.sensitivity_multiplier = 2.0;

        // Test with low sensitivity
        let result_low = processor.process_as_camera(&calibration_low, 128, 0, CameraOutputMode::Scroll);
        assert!(result_low.is_some());
        let amount_low = match result_low.unwrap() {
            CameraOutput::Scroll(amount) => amount,
            _ => panic!("Should return Scroll variant"),
        };

        // Test with high sensitivity
        let result_high = processor.process_as_camera(&calibration_high, 128, 0, CameraOutputMode::Scroll);
        assert!(result_high.is_some());
        let amount_high = match result_high.unwrap() {
            CameraOutput::Scroll(amount) => amount,
            _ => panic!("Should return Scroll variant"),
        };

        // Higher sensitivity should give larger scroll amount
        assert!(amount_high.abs() > amount_low.abs(),
            "Higher sensitivity should give larger scroll: {} > {}",
            amount_high.abs(), amount_low.abs());
    }

    #[test]
    fn test_camera_direction_to_keys() {
        // Test the helper function directly
        let keys = camera_direction_to_keys(DpadDirection::None);
        assert!(keys.is_empty(), "None should return empty Vec");

        let keys = camera_direction_to_keys(DpadDirection::Up);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], Key::KEY_PAGEUP, "Up should map to PageUp");

        let keys = camera_direction_to_keys(DpadDirection::Down);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], Key::KEY_PAGEDOWN, "Down should map to PageDown");

        let keys = camera_direction_to_keys(DpadDirection::Left);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], Key::KEY_LEFT, "Left should map to Left arrow");

        let keys = camera_direction_to_keys(DpadDirection::Right);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], Key::KEY_RIGHT, "Right should map to Right arrow");

        // Diagonals return 2 keys
        let keys = camera_direction_to_keys(DpadDirection::UpRight);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&Key::KEY_PAGEUP), "UpRight should contain PageUp");
        assert!(keys.contains(&Key::KEY_RIGHT), "UpRight should contain Right");

        let keys = camera_direction_to_keys(DpadDirection::DownLeft);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&Key::KEY_PAGEDOWN), "DownLeft should contain PageDown");
        assert!(keys.contains(&Key::KEY_LEFT), "DownLeft should contain Left");
    }

    #[test]
    fn test_process_as_camera_key_mode_deadzone() {
        let processor = AnalogProcessor::new();
        let calibration = AnalogCalibration::default();

        // Center position should be filtered
        let result = processor.process_as_camera(&calibration, 128, 128, CameraOutputMode::Keys);
        assert!(result.is_none(), "Center should be filtered by deadzone in Keys mode");

        // Small movement should also be filtered
        let result = processor.process_as_camera(&calibration, 135, 128, CameraOutputMode::Keys);
        assert!(result.is_none(), "Small movement should be filtered by deadzone");
    }

    #[test]
    fn test_process_as_camera_deadzone_filters_small_movements() {
        let processor = AnalogProcessor::new();
        let mut calibration = AnalogCalibration::default();
        calibration.deadzone = 0.2;  // Larger deadzone

        // Small movements within deadzone should be filtered (Scroll mode)
        let result = processor.process_as_camera(&calibration, 135, 128, CameraOutputMode::Scroll);
        assert!(result.is_none(), "Small X movement should be filtered in Scroll mode");

        // Small movements within deadzone should be filtered (Keys mode)
        let result = processor.process_as_camera(&calibration, 128, 135, CameraOutputMode::Keys);
        assert!(result.is_none(), "Small Y movement should be filtered in Keys mode");
    }
}
