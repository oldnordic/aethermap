//! Virtual Gamepad Device Module
//!
//! This module provides functionality to create and manage virtual gamepad devices
//! using the Linux uinput subsystem. It emulates an Xbox 360 controller that can be
//! detected by games and applications.

use aethermap_common::tracing;
use std::fs::OpenOptions;
use std::mem;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::{Arc, RwLock};
use tracing::{debug, error, info, warn};

// Linux input event constants
const EV_SYN: u16 = 0x00;
const EV_ABS: u16 = 0x03;
const SYN_REPORT: u16 = 0x00;

// Absolute axis codes (matching evdev::AbsoluteAxisType)
pub const ABS_X: u16 = 0x00;
pub const ABS_Y: u16 = 0x01;
pub const ABS_Z: u16 = 0x02;
pub const ABS_RX: u16 = 0x03;
pub const ABS_RY: u16 = 0x04;
pub const ABS_RZ: u16 = 0x05;

// uinput ioctl constants
const UINPUT_IOCTL_BASE: u8 = b'U';
const UI_SET_EVBIT: u64 = 0x40045564; // _IOW('U', 100, int)
const UI_SET_ABSBIT: u64 = 0x40045567; // _IOW('U', 103, int)
const UI_DEV_CREATE: u64 = 0x5501; // _IO('U', 1)
const UI_DEV_DESTROY: u64 = 0x5502; // _IO('U', 2)

// Bus type constants
const BUS_USB: u16 = 0x03;

// Xbox 360 Controller identifiers
const XBOX360_VENDOR_ID: u16 = 0x045e; // Microsoft
const XBOX360_PRODUCT_ID: u16 = 0x028e; // Xbox 360 Controller
const XBOX360_VERSION: u16 = 0x0110;

// Default device name
const DEFAULT_DEVICE_NAME: &str = "Aethermap Virtual Gamepad";

// Axis range constants (standard evdev range for gamepads)
const AXIS_MIN: i32 = -32768;
const AXIS_MAX: i32 = 32767;
const AXIS_FUZZ: i32 = 0;
const AXIS_FLAT: i32 = 0;

/// Linux input_event structure
///
/// This structure matches the kernel's input_event struct defined in linux/input.h.
/// It represents a single input event from a device.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

/// uinput_user_dev structure for device setup
///
/// This structure is used to configure the virtual device before creation.
/// It contains the device name, ID information, and axis configuration.
#[repr(C)]
#[allow(dead_code)]
struct UinputUserDev {
    name: [u8; 80],
    id: InputId,
    ff_effects_max: u32,
    absmax: [i32; 64],
    absmin: [i32; 64],
    absfuzz: [i32; 64],
    absflat: [i32; 64],
}

/// Input device identification structure
///
/// Contains bus type and vendor/product information for the virtual device.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

/// Gamepad axis enumeration
///
/// Represents the absolute axes available on the virtual gamepad.
/// These map to standard Linux input axis codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum GamepadAxis {
    /// Left stick X axis
    ABS_X = 0,
    /// Left stick Y axis
    ABS_Y = 1,
    /// Left trigger / Z axis
    ABS_Z = 2,
    /// Right stick X axis
    ABS_RX = 3,
    /// Right stick Y axis
    ABS_RY = 4,
    /// Right trigger / RZ axis
    ABS_RZ = 5,
}

impl GamepadAxis {
    /// Get the axis code as u16
    pub fn code(self) -> u16 {
        self as u16
    }

    /// Get axis index for absmin/absmax arrays
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Virtual gamepad device
///
/// Creates and manages a virtual Xbox 360 gamepad via uinput.
/// The device can emit absolute axis events for analog stick emulation.
#[derive(Clone)]
pub struct GamepadVirtualDevice {
    /// Thread-safe file descriptor access for uinput device
    uinput_fd: Arc<RwLock<Option<RawFd>>>,
    /// Device name (shown in /dev/input/)
    device_name: String,
    /// Vendor ID (default: Microsoft 0x045e)
    vendor_id: u16,
    /// Product ID (default: Xbox 360 Controller 0x028e)
    product_id: u16,
    /// Device version
    version: u16,
}

impl Default for GamepadVirtualDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl GamepadVirtualDevice {
    /// Create a new virtual gamepad with default Xbox 360 identifiers
    ///
    /// # Returns
    /// A new GamepadVirtualDevice instance with Xbox 360 vendor/product IDs
    pub fn new() -> Self {
        info!("Creating new GamepadVirtualDevice with Xbox 360 identifiers");
        Self {
            uinput_fd: Arc::new(RwLock::new(None)),
            device_name: DEFAULT_DEVICE_NAME.to_string(),
            vendor_id: XBOX360_VENDOR_ID,
            product_id: XBOX360_PRODUCT_ID,
            version: XBOX360_VERSION,
        }
    }

    /// Create a virtual gamepad with custom device identifiers
    ///
    /// # Arguments
    /// * `name` - Device name (max 79 characters)
    /// * `vendor_id` - USB vendor ID
    /// * `product_id` - USB product ID
    ///
    /// # Returns
    /// A new GamepadVirtualDevice with custom identifiers
    pub fn with_ids(name: &str, vendor_id: u16, product_id: u16) -> Self {
        let truncated_name = if name.len() > 79 {
            warn!("Device name too long, truncating to 79 characters");
            &name[..79]
        } else {
            name
        };

        info!(
            "Creating GamepadVirtualDevice: name={}, vendor={:04x}, product={:04x}",
            truncated_name, vendor_id, product_id
        );

        Self {
            uinput_fd: Arc::new(RwLock::new(None)),
            device_name: truncated_name.to_string(),
            vendor_id,
            product_id,
            version: XBOX360_VERSION,
        }
    }

    /// Check if the device is currently created
    ///
    /// # Returns
    /// true if the virtual device exists, false otherwise
    pub fn is_created(&self) -> bool {
        self.uinput_fd
            .try_read()
            .map(|fd| fd.is_some())
            .unwrap_or(false)
    }

    /// Create the virtual gamepad device via uinput
    ///
    /// Opens /dev/uinput, configures the device with EV_ABS support for
    /// ABS_X and ABS_Y axes, and creates the virtual device.
    ///
    /// # Returns
    /// Ok(()) if successful, Err with description on failure
    ///
    /// # Errors
    /// Returns an error if:
    /// - /dev/uinput cannot be opened (requires root privileges)
    /// - ioctl calls fail
    /// - Device structure cannot be written
    pub fn create(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check if already created
        if self.is_created() {
            warn!("Gamepad virtual device already created");
            return Ok(());
        }

        info!(
            "Creating virtual gamepad device: {} ({:04x}:{:04x})",
            self.device_name, self.vendor_id, self.product_id
        );

        // Open /dev/uinput
        let uinput_file = OpenOptions::new()
            .write(true)
            .open("/dev/uinput")
            .map_err(|e| {
                error!(
                    "Failed to open /dev/uinput: {}. Ensure you have root privileges and uinput module is loaded.",
                    e
                );
                format!("Failed to open /dev/uinput: {}", e)
            })?;

        let fd = uinput_file.as_raw_fd();

        // Leak the file to keep fd valid (we'll clean up in Drop)
        mem::forget(uinput_file);

        unsafe {
            // Enable EV_ABS event type (absolute axes)
            if libc::ioctl(fd, UI_SET_EVBIT, EV_ABS as libc::c_int) < 0 {
                return Err("Failed to set EV_ABS bit".into());
            }

            // Enable EV_SYN event type (synchronization)
            if libc::ioctl(fd, UI_SET_EVBIT, EV_SYN as libc::c_int) < 0 {
                return Err("Failed to set EV_SYN bit".into());
            }

            // Enable ABS_X axis (left stick X)
            if libc::ioctl(fd, UI_SET_ABSBIT, ABS_X as libc::c_int) < 0 {
                return Err("Failed to set ABS_X bit".into());
            }

            // Enable ABS_Y axis (left stick Y)
            if libc::ioctl(fd, UI_SET_ABSBIT, ABS_Y as libc::c_int) < 0 {
                return Err("Failed to set ABS_Y bit".into());
            }

            // Also enable additional axes for full gamepad support
            for axis in [ABS_Z, ABS_RX, ABS_RY, ABS_RZ] {
                if libc::ioctl(fd, UI_SET_ABSBIT, axis as libc::c_int) < 0 {
                    warn!("Failed to set ABS bit for axis {}", axis);
                }
            }
        }

        // Create device structure
        let mut dev: UinputUserDev = unsafe { mem::zeroed() };

        // Set device name (truncate if needed)
        let name_bytes = self.device_name.as_bytes();
        let name_len = name_bytes.len().min(79);
        dev.name[..name_len].copy_from_slice(&name_bytes[..name_len]);

        // Set device identification
        dev.id.bustype = BUS_USB;
        dev.id.vendor = self.vendor_id;
        dev.id.product = self.product_id;
        dev.id.version = self.version;

        // Set up absolute axis ranges for all axes
        for axis in [ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ] {
            let idx = axis as usize;
            dev.absmin[idx] = AXIS_MIN;
            dev.absmax[idx] = AXIS_MAX;
            dev.absfuzz[idx] = AXIS_FUZZ;
            dev.absflat[idx] = AXIS_FLAT;
        }

        // Write device structure to uinput
        unsafe {
            let dev_ptr = &dev as *const UinputUserDev as *const u8;
            let dev_slice = std::slice::from_raw_parts(dev_ptr, mem::size_of::<UinputUserDev>());

            if libc::write(fd, dev_slice.as_ptr() as *const libc::c_void, dev_slice.len()) < 0 {
                return Err("Failed to write uinput device structure".into());
            }

            // Create the device
            if libc::ioctl(fd, UI_DEV_CREATE) < 0 {
                return Err("Failed to create uinput device".into());
            }
        }

        info!(
            "Successfully created virtual gamepad: {} ({:04x}:{:04x})",
            self.device_name, self.vendor_id, self.product_id
        );

        // Store the file descriptor
        {
            let mut uinput_fd = self.uinput_fd.try_write()
                .map_err(|_| "Lock poisoned on uinput_fd write")?;
            *uinput_fd = Some(fd);
        }

        Ok(())
    }

    /// Emit an absolute axis event to the virtual gamepad
    ///
    /// Sends an EV_ABS event with the specified axis and value to the uinput device,
    /// followed by a synchronization event.
    ///
    /// # Arguments
    /// * `axis` - The gamepad axis to emit (ABS_X, ABS_Y, etc.)
    /// * `value` - The axis value (-32768 to 32767)
    ///
    /// # Returns
    /// Ok(()) if successful, Err on failure
    ///
    /// # Errors
    /// Returns an error if:
    /// - Device has not been created
    /// - Lock is poisoned
    /// - Write to uinput fails
    pub fn emit_axis(
        &self,
        axis: GamepadAxis,
        value: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get the file descriptor
        let fd = {
            let uinput_fd = self
                .uinput_fd
                .try_read()
                .map_err(|_| "Lock poisoned on uinput_fd read")?;
            uinput_fd.ok_or("Gamepad virtual device not created")?
        };

        debug!("Emitting axis: {:?} = {}", axis, value);

        // Create and write the axis event
        let mut event: InputEvent = unsafe { mem::zeroed() };

        // Get current time
        unsafe {
            libc::gettimeofday(&mut event.time, std::ptr::null_mut());
        }

        event.type_ = EV_ABS;
        event.code = axis.code();
        event.value = value;

        unsafe {
            let event_ptr = &event as *const InputEvent as *const u8;
            let event_slice = std::slice::from_raw_parts(event_ptr, mem::size_of::<InputEvent>());

            let written = libc::write(fd, event_slice.as_ptr() as *const libc::c_void, event_slice.len());
            if written < 0 {
                return Err(format!("Failed to write axis event: {}", std::io::Error::last_os_error()).into());
            }
        }

        // Send synchronization event
        self.sync()?;

        Ok(())
    }

    /// Send a synchronization event to the uinput device
    ///
    /// This marks the end of a set of related events and notifies the kernel
    /// that the event state is complete.
    ///
    /// # Returns
    /// Ok(()) if successful, Err on failure
    fn sync(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let fd = {
            let uinput_fd = self
                .uinput_fd
                .try_read()
                .map_err(|_| "Lock poisoned on uinput_fd read")?;
            uinput_fd.ok_or("Gamepad virtual device not created")?
        };

        let mut event: InputEvent = unsafe { mem::zeroed() };

        unsafe {
            libc::gettimeofday(&mut event.time, std::ptr::null_mut());
        }

        event.type_ = EV_SYN;
        event.code = SYN_REPORT;
        event.value = 0;

        unsafe {
            let event_ptr = &event as *const InputEvent as *const u8;
            let event_slice = std::slice::from_raw_parts(event_ptr, mem::size_of::<InputEvent>());

            let written = libc::write(fd, event_slice.as_ptr() as *const libc::c_void, event_slice.len());
            if written < 0 {
                return Err(format!("Failed to write sync event: {}", std::io::Error::last_os_error()).into());
            }
        }

        Ok(())
    }

    /// Get the vendor ID of the virtual gamepad
    ///
    /// # Returns
    /// The USB vendor ID (default: 0x045e for Microsoft)
    pub fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    /// Get the product ID of the virtual gamepad
    ///
    /// # Returns
    /// The USB product ID (default: 0x028e for Xbox 360 Controller)
    pub fn product_id(&self) -> u16 {
        self.product_id
    }

    /// Get the device name of the virtual gamepad
    ///
    /// # Returns
    /// The device name as a string slice
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Destroy the virtual gamepad device
    ///
    /// Cleans up the uinput device and closes the file descriptor.
    /// This is also called automatically on Drop.
    ///
    /// # Returns
    /// Ok(()) if successful or device wasn't created, Err on failure
    pub fn destroy(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let fd = {
            let mut uinput_fd = self.uinput_fd.try_write()
                .map_err(|_| "Lock poisoned on uinput_fd write")?;
            uinput_fd.take()
        };

        if let Some(fd) = fd {
            info!("Destroying virtual gamepad device: {}", self.device_name);
            unsafe {
                libc::ioctl(fd, UI_DEV_DESTROY);
                libc::close(fd);
            }
        }

        Ok(())
    }
}

impl Drop for GamepadVirtualDevice {
    fn drop(&mut self) {
        // Clean up the uinput device if still active
        if let Ok(fd_guard) = self.uinput_fd.try_read() {
            if let Some(fd) = *fd_guard {
                info!("Auto-destroying virtual gamepad device: {}", self.device_name);
                unsafe {
                    libc::ioctl(fd, UI_DEV_DESTROY);
                    libc::close(fd);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamepad_axis_codes() {
        assert_eq!(GamepadAxis::ABS_X.code(), 0);
        assert_eq!(GamepadAxis::ABS_Y.code(), 1);
        assert_eq!(GamepadAxis::ABS_Z.code(), 2);
        assert_eq!(GamepadAxis::ABS_RX.code(), 3);
        assert_eq!(GamepadAxis::ABS_RY.code(), 4);
        assert_eq!(GamepadAxis::ABS_RZ.code(), 5);
    }

    #[test]
    fn test_gamepad_axis_indices() {
        assert_eq!(GamepadAxis::ABS_X.index(), 0);
        assert_eq!(GamepadAxis::ABS_Y.index(), 1);
        assert_eq!(GamepadAxis::ABS_Z.index(), 2);
        assert_eq!(GamepadAxis::ABS_RX.index(), 3);
        assert_eq!(GamepadAxis::ABS_RY.index(), 4);
        assert_eq!(GamepadAxis::ABS_RZ.index(), 5);
    }

    #[test]
    fn test_virtual_device_creation() {
        let device = GamepadVirtualDevice::new();
        assert!(!device.is_created());
        assert_eq!(device.vendor_id, XBOX360_VENDOR_ID);
        assert_eq!(device.product_id, XBOX360_PRODUCT_ID);
        assert_eq!(device.device_name, DEFAULT_DEVICE_NAME);
    }

    #[test]
    fn test_virtual_device_default() {
        let device = GamepadVirtualDevice::default();
        assert!(!device.is_created());
        assert_eq!(device.vendor_id, XBOX360_VENDOR_ID);
        assert_eq!(device.product_id, XBOX360_PRODUCT_ID);
    }

    #[test]
    fn test_virtual_device_with_custom_ids() {
        let device = GamepadVirtualDevice::with_ids("Custom Gamepad", 0x1234, 0x5678);
        assert_eq!(device.vendor_id, 0x1234);
        assert_eq!(device.product_id, 0x5678);
        assert_eq!(device.device_name, "Custom Gamepad");
    }

    #[test]
    fn test_long_device_name_truncation() {
        let long_name = "A".repeat(100);
        let device = GamepadVirtualDevice::with_ids(&long_name, 0x1234, 0x5678);
        assert_eq!(device.device_name.len(), 79);
    }

    // Integration tests for gamepad mode (plan 14-03)

    #[test]
    fn test_gamepad_device_creation() {
        let device = GamepadVirtualDevice::new();
        assert_eq!(device.vendor_id(), 0x045e);
        assert_eq!(device.product_id(), 0x028e);
    }

    #[test]
    fn test_gamepad_axis_values() {
        // Verify axis code constants match expected Linux input codes
        assert_eq!(GamepadAxis::ABS_X as u16, 0);
        assert_eq!(GamepadAxis::ABS_Y as u16, 1);
        assert_eq!(GamepadAxis::ABS_Z as u16, 2);
        assert_eq!(GamepadAxis::ABS_RX as u16, 3);
        assert_eq!(GamepadAxis::ABS_RY as u16, 4);
        assert_eq!(GamepadAxis::ABS_RZ as u16, 5);
    }

    #[test]
    fn test_device_name() {
        let device = GamepadVirtualDevice::new();
        assert!(!device.device_name().is_empty(), "Device name should not be empty");
        assert!(device.device_name().contains("Gamepad"), "Name should contain 'Gamepad'");
    }

    // Note: Full uinput integration tests require root and /dev/uinput access
    // These will be tested in plan 14-06 integration tests
}
