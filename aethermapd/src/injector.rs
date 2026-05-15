use aethermap_common::tracing;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::{Arc, RwLock};
// use std::io::Write;
use std::mem;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

// Linux input event constants
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const EV_ABS: u16 = 0x03;
const SYN_REPORT: u16 = 0x00;
const REL_X: u16 = 0x00;
const REL_Y: u16 = 0x01;
const REL_WHEEL: u16 = 0x08;

// Absolute axis codes (matching evdev::AbsoluteAxisType)
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_Z: u16 = 0x02;
const ABS_RX: u16 = 0x03;
const ABS_RY: u16 = 0x04;
const ABS_RZ: u16 = 0x05;

// Key codes
const KEY_LEFTSHIFT: u16 = 42;

// uinput ioctl constants
#[allow(dead_code)]
const UINPUT_IOCTL_BASE: u8 = b'U';
const UI_SET_EVBIT: u64 = 0x40045564; // _IOW('U', 100, int)
const UI_SET_KEYBIT: u64 = 0x40045565; // _IOW('U', 101, int)
const UI_SET_RELBIT: u64 = 0x40045566; // _IOW('U', 102, int)
const UI_SET_ABSBIT: u64 = 0x40045567; // _IOW('U', 103, int)
const UI_DEV_CREATE: u64 = 0x5501; // _IO('U', 1)
const UI_DEV_DESTROY: u64 = 0x5502; // _IO('U', 2)

/// Linux input_event structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

/// uinput_user_dev structure for device setup
#[repr(C)]
struct UinputUserDev {
    name: [u8; 80],
    id: InputId,
    ff_effects_max: u32,
    absmax: [i32; 64],
    absmin: [i32; 64],
    absfuzz: [i32; 64],
    absflat: [i32; 64],
}

#[repr(C)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

/// Trait for input injection functionality
#[async_trait::async_trait]
pub trait Injector: Send + Sync {
    async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn key_press(
        &self,
        key_code: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn key_release(
        &self,
        key_code: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn mouse_press(
        &self,
        button: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn mouse_release(
        &self,
        button: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn mouse_move(
        &self,
        x: i32,
        y: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn mouse_scroll(
        &self,
        amount: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn type_string(&self, text: &str)
        -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn execute_command(
        &self,
        command: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn analog_move(
        &self,
        axis_code: u16,
        value: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Convert axis code to evdev absolute axis type
///
/// Maps internal axis codes (61000-61005) to Linux evdev absolute axis codes.
/// Returns None for invalid axis codes.
fn axis_code_to_type(code: u16) -> Option<u16> {
    match code {
        61000 => Some(ABS_X),
        61001 => Some(ABS_Y),
        61002 => Some(ABS_Z),
        61003 => Some(ABS_RX),
        61004 => Some(ABS_RY),
        61005 => Some(ABS_RZ),
        _ => None,
    }
}

/// Real uinput-based injector that creates virtual input devices
#[derive(Clone)]
pub struct UinputInjector {
    initialized: Arc<RwLock<bool>>,
    uinput_fd: Arc<RwLock<Option<RawFd>>>,
    key_map: Arc<RwLock<HashMap<char, u16>>>,
}

impl UinputInjector {
    /// Create a new injector instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        info!("Creating new UinputInjector instance");

        // Initialize US QWERTY keyboard mapping
        let mut key_map = HashMap::new();

        // Numbers 0-9 (KEY_1=2, KEY_2=3, ..., KEY_0=11)
        key_map.insert('1', 2);
        key_map.insert('2', 3);
        key_map.insert('3', 4);
        key_map.insert('4', 5);
        key_map.insert('5', 6);
        key_map.insert('6', 7);
        key_map.insert('7', 8);
        key_map.insert('8', 9);
        key_map.insert('9', 10);
        key_map.insert('0', 11);

        // Letters (KEY_Q=16, KEY_W=17, etc.)
        let qwerty_row1 = "qwertyuiop";
        for (i, c) in qwerty_row1.chars().enumerate() {
            key_map.insert(c, 16 + i as u16);
            key_map.insert(c.to_ascii_uppercase(), 16 + i as u16);
        }

        let qwerty_row2 = "asdfghjkl";
        for (i, c) in qwerty_row2.chars().enumerate() {
            key_map.insert(c, 30 + i as u16);
            key_map.insert(c.to_ascii_uppercase(), 30 + i as u16);
        }

        let qwerty_row3 = "zxcvbnm";
        for (i, c) in qwerty_row3.chars().enumerate() {
            key_map.insert(c, 44 + i as u16);
            key_map.insert(c.to_ascii_uppercase(), 44 + i as u16);
        }

        // Special characters
        key_map.insert(' ', 57); // KEY_SPACE
        key_map.insert('-', 12); // KEY_MINUS
        key_map.insert('=', 13); // KEY_EQUAL
        key_map.insert('[', 26); // KEY_LEFTBRACE
        key_map.insert(']', 27); // KEY_RIGHTBRACE
        key_map.insert('\\', 43); // KEY_BACKSLASH
        key_map.insert(';', 39); // KEY_SEMICOLON
        key_map.insert('\'', 40); // KEY_APOSTROPHE
        key_map.insert('`', 41); // KEY_GRAVE
        key_map.insert(',', 51); // KEY_COMMA
        key_map.insert('.', 52); // KEY_DOT
        key_map.insert('/', 53); // KEY_SLASH
        key_map.insert('\n', 28); // KEY_ENTER
        key_map.insert('\t', 15); // KEY_TAB

        Ok(Self {
            initialized: Arc::new(RwLock::new(false)),
            uinput_fd: Arc::new(RwLock::new(None)),
            key_map: Arc::new(RwLock::new(key_map)),
        })
    }

    /// Initialize the uinput device - creates a virtual keyboard and mouse
    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        {
            let initialized = self
                .initialized
                .try_read()
                .map_err(|_| "Lock poisoned on initialized check")?;
            if *initialized {
                return Ok(());
            }
        }

        info!("Initializing uinput virtual device");

        // Open /dev/uinput
        let uinput_file = OpenOptions::new()
            .write(true)
            .open("/dev/uinput")
            .map_err(|e| {
                error!("Failed to open /dev/uinput: {}. Ensure you have root privileges and uinput module is loaded.", e);
                format!("Failed to open /dev/uinput: {}", e)
            })?;

        let fd = uinput_file.as_raw_fd();

        // Leak the file to keep fd valid (we'll clean up in Drop)
        mem::forget(uinput_file);

        // Set up event types
        unsafe {
            // Enable EV_KEY events (keyboard/mouse buttons)
            if libc::ioctl(fd, UI_SET_EVBIT, EV_KEY as libc::c_int) < 0 {
                return Err("Failed to set EV_KEY bit".into());
            }

            // Enable EV_REL events (relative mouse movement)
            if libc::ioctl(fd, UI_SET_EVBIT, EV_REL as libc::c_int) < 0 {
                return Err("Failed to set EV_REL bit".into());
            }

            // Enable EV_SYN events (synchronization)
            if libc::ioctl(fd, UI_SET_EVBIT, EV_SYN as libc::c_int) < 0 {
                return Err("Failed to set EV_SYN bit".into());
            }

            // Enable EV_ABS events (absolute axes for analog sticks)
            if libc::ioctl(fd, UI_SET_EVBIT, EV_ABS as libc::c_int) < 0 {
                return Err("Failed to set EV_ABS bit".into());
            }

            // Enable all key codes (0-255)
            for key in 0..256u16 {
                if libc::ioctl(fd, UI_SET_KEYBIT, key as libc::c_int) < 0 {
                    warn!("Failed to set keybit for key {}", key);
                }
            }

            // Enable mouse buttons (BTN_LEFT=272, BTN_RIGHT=273, BTN_MIDDLE=274)
            for btn in 272..280u16 {
                if libc::ioctl(fd, UI_SET_KEYBIT, btn as libc::c_int) < 0 {
                    warn!("Failed to set keybit for button {}", btn);
                }
            }

            // Enable joystick button range (BTN_JOYSTICK=0x120 to BTN_DEAD=0x12f)
            // These are used by gamepads, joysticks, and keypads like Azeron Cyborg
            for btn in 0x120..=0x12fu16 {
                if libc::ioctl(fd, UI_SET_KEYBIT, btn as libc::c_int) < 0 {
                    warn!("Failed to set keybit for joystick button {}", btn);
                }
            }

            // Enable relative axes for mouse movement
            if libc::ioctl(fd, UI_SET_RELBIT, REL_X as libc::c_int) < 0 {
                warn!("Failed to set REL_X bit");
            }
            if libc::ioctl(fd, UI_SET_RELBIT, REL_Y as libc::c_int) < 0 {
                warn!("Failed to set REL_Y bit");
            }
            if libc::ioctl(fd, UI_SET_RELBIT, REL_WHEEL as libc::c_int) < 0 {
                warn!("Failed to set REL_WHEEL bit");
            }

            // Enable absolute axes for analog sticks (X, Y, Z, RX, RY, RZ)
            for abs_axis in [ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ] {
                if libc::ioctl(fd, UI_SET_ABSBIT, abs_axis as libc::c_int) < 0 {
                    warn!("Failed to set ABS bit for axis {}", abs_axis);
                }
            }
        }

        // Create device structure
        let mut dev: UinputUserDev = unsafe { mem::zeroed() };
        let name = b"Aethermap Virtual Input";
        dev.name[..name.len()].copy_from_slice(name);
        dev.id.bustype = 0x03; // BUS_USB
        dev.id.vendor = 0x1532; // Razer vendor ID
        dev.id.product = 0xFFFF; // Virtual device
        dev.id.version = 1;

        // Set up absolute axis ranges for analog sticks
        // Standard evdev range is -32768 to 32767
        const ABS_MIN: i32 = -32768;
        const ABS_MAX: i32 = 32767;
        const ABS_FUZZ: i32 = 0;
        const ABS_FLAT: i32 = 0;

        // Configure X, Y, Z, RX, RY, RZ axes
        for axis in [ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ] {
            dev.absmin[axis as usize] = ABS_MIN;
            dev.absmax[axis as usize] = ABS_MAX;
            dev.absfuzz[axis as usize] = ABS_FUZZ;
            dev.absflat[axis as usize] = ABS_FLAT;
        }

        // Write device structure
        unsafe {
            let dev_ptr = &dev as *const UinputUserDev as *const u8;
            let dev_slice = std::slice::from_raw_parts(dev_ptr, mem::size_of::<UinputUserDev>());

            if libc::write(
                fd,
                dev_slice.as_ptr() as *const libc::c_void,
                dev_slice.len(),
            ) < 0
            {
                return Err("Failed to write uinput device structure".into());
            }

            // Create the device
            if libc::ioctl(fd, UI_DEV_CREATE) < 0 {
                return Err("Failed to create uinput device".into());
            }
        }

        info!(
            "Successfully created uinput virtual device: {}",
            String::from_utf8_lossy(name)
        );

        // Store the file descriptor
        {
            let mut uinput_fd = self
                .uinput_fd
                .try_write()
                .map_err(|_| "Lock poisoned on uinput_fd write")?;
            *uinput_fd = Some(fd);
        }

        {
            let mut initialized = self
                .initialized
                .try_write()
                .map_err(|_| "Lock poisoned on initialized write")?;
            *initialized = true;
        }

        // Small delay to let the device settle
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Write an input event to the uinput device
    fn write_event(
        &self,
        type_: u16,
        code: u16,
        value: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let fd = {
            let uinput_fd = self
                .uinput_fd
                .try_read()
                .map_err(|_| "Lock poisoned on uinput_fd read")?;
            uinput_fd.ok_or("Uinput device not initialized")?
        };

        let mut event: InputEvent = unsafe { mem::zeroed() };

        // Get current time
        unsafe {
            libc::gettimeofday(&mut event.time, std::ptr::null_mut());
        }

        event.type_ = type_;
        event.code = code;
        event.value = value;

        unsafe {
            let event_ptr = &event as *const InputEvent as *const u8;
            let event_slice = std::slice::from_raw_parts(event_ptr, mem::size_of::<InputEvent>());

            let written = libc::write(
                fd,
                event_slice.as_ptr() as *const libc::c_void,
                event_slice.len(),
            );
            if written < 0 {
                return Err(
                    format!("Failed to write event: {}", std::io::Error::last_os_error()).into(),
                );
            }
        }

        Ok(())
    }

    /// Send a synchronization event
    fn sync(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.write_event(EV_SYN, SYN_REPORT, 0)
    }

    /// Press a key (sends key down event + sync)
    pub async fn key_press(
        &self,
        key_code: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let needs_init = {
            let initialized = self
                .initialized
                .try_read()
                .map_err(|_| "Lock poisoned on initialized check")?;
            !*initialized
        };

        if needs_init {
            self.initialize().await?;
        }

        debug!("Key press: {}", key_code);
        self.write_event(EV_KEY, key_code, 1)?; // 1 = key down
        self.sync()?;
        Ok(())
    }

    /// Release a key (sends key up event + sync)
    pub async fn key_release(
        &self,
        key_code: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let needs_init = {
            let initialized = self
                .initialized
                .try_read()
                .map_err(|_| "Lock poisoned on initialized check")?;
            !*initialized
        };

        if needs_init {
            self.initialize().await?;
        }

        debug!("Key release: {}", key_code);
        self.write_event(EV_KEY, key_code, 0)?; // 0 = key up
        self.sync()?;
        Ok(())
    }

    /// Press a mouse button
    pub async fn mouse_press(
        &self,
        button: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let needs_init = {
            let initialized = self
                .initialized
                .try_read()
                .map_err(|_| "Lock poisoned on initialized check")?;
            !*initialized
        };

        if needs_init {
            self.initialize().await?;
        }

        // Convert button number to Linux button code
        // 1=left (272), 2=right (273), 3=middle (274)
        let btn_code = 271 + button;
        debug!("Mouse button {} press (code {})", button, btn_code);
        self.write_event(EV_KEY, btn_code, 1)?;
        self.sync()?;
        Ok(())
    }

    /// Release a mouse button
    pub async fn mouse_release(
        &self,
        button: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let needs_init = {
            let initialized = self
                .initialized
                .try_read()
                .map_err(|_| "Lock poisoned on initialized check")?;
            !*initialized
        };

        if needs_init {
            self.initialize().await?;
        }

        let btn_code = 271 + button;
        debug!("Mouse button {} release (code {})", button, btn_code);
        self.write_event(EV_KEY, btn_code, 0)?;
        self.sync()?;
        Ok(())
    }

    /// Move the mouse cursor (relative movement)
    pub async fn mouse_move(
        &self,
        x: i32,
        y: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let needs_init = {
            let initialized = self
                .initialized
                .try_read()
                .map_err(|_| "Lock poisoned on initialized check")?;
            !*initialized
        };

        if needs_init {
            self.initialize().await?;
        }

        debug!("Mouse move: dx={}, dy={}", x, y);
        if x != 0 {
            self.write_event(EV_REL, REL_X, x)?;
        }
        if y != 0 {
            self.write_event(EV_REL, REL_Y, y)?;
        }
        self.sync()?;
        Ok(())
    }

    /// Scroll the mouse wheel
    pub async fn mouse_scroll(
        &self,
        amount: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let needs_init = {
            let initialized = self
                .initialized
                .try_read()
                .map_err(|_| "Lock poisoned on initialized check")?;
            !*initialized
        };

        if needs_init {
            self.initialize().await?;
        }

        debug!("Mouse scroll: {}", amount);
        self.write_event(EV_REL, REL_WHEEL, amount)?;
        self.sync()?;
        Ok(())
    }

    /// Type a string by simulating key presses and releases
    pub async fn type_string(
        &self,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let needs_init = {
            let initialized = self
                .initialized
                .try_read()
                .map_err(|_| "Lock poisoned on initialized check")?;
            !*initialized
        };

        if needs_init {
            self.initialize().await?;
        }

        info!("Typing string: {}", text);

        for c in text.chars() {
            let key_code = {
                let key_map = self
                    .key_map
                    .try_read()
                    .map_err(|_| "Lock poisoned on key_map read")?;
                key_map.get(&c).copied()
            };

            if let Some(key_code) = key_code {
                let needs_shift = c.is_ascii_uppercase() || "!@#$%^&*()_+{}|:\"<>?~".contains(c);

                if needs_shift {
                    self.key_press(KEY_LEFTSHIFT).await?;
                    sleep(Duration::from_millis(10)).await;
                }

                self.key_press(key_code).await?;
                sleep(Duration::from_millis(20)).await;
                self.key_release(key_code).await?;

                if needs_shift {
                    sleep(Duration::from_millis(10)).await;
                    self.key_release(KEY_LEFTSHIFT).await?;
                }

                sleep(Duration::from_millis(30)).await;
            } else {
                warn!("No key mapping for character: '{}' (U+{:04X})", c, c as u32);
            }
        }

        Ok(())
    }

    /// Execute a system command with security restrictions
    pub async fn execute_command(
        &self,
        command: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Executing command: {}", command);

        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".into());
        }

        let program = parts[0];
        let args = &parts[1..];

        // Security: Only allow whitelisted commands
        let allowed_commands = [
            "xdotool",
            "xrandr",
            "amixer",
            "notify-send",
            "pactl",
            "playerctl",
            "brightnessctl",
            "xbacklight",
        ];

        if !allowed_commands.contains(&program) {
            warn!("Blocked non-whitelisted command: {}", program);
            return Err(format!("Command '{}' is not allowed", program).into());
        }

        info!("Executing allowed command: {} {:?}", program, args);

        use std::process::Stdio;
        use tokio::process::Command;

        let output = tokio::time::timeout(
            Duration::from_secs(10),
            Command::new(program)
                .args(args)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env(
                    "DISPLAY",
                    std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()),
                )
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await;

        match output {
            Ok(Ok(output)) => {
                if output.status.success() {
                    info!("Command executed successfully");
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("Command failed: {}", stderr);
                    Err(format!("Command failed: {}", stderr).into())
                }
            }
            Ok(Err(e)) => Err(format!("Failed to execute: {}", e).into()),
            Err(_) => Err("Command timed out".into()),
        }
    }

    /// Inject an absolute axis event (analog stick movement)
    ///
    /// # Arguments
    /// * `axis_code` - Axis code (61000-61005 for ABS_X, ABS_Y, etc.)
    /// * `value` - Raw analog value (-32768 to 32767)
    ///
    /// # Returns
    /// Ok(()) if successful, Err on failure
    pub async fn analog_move(
        &self,
        axis_code: u16,
        value: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let needs_init = {
            let initialized = self
                .initialized
                .try_read()
                .map_err(|_| "Lock poisoned on initialized check")?;
            !*initialized
        };

        if needs_init {
            self.initialize().await?;
        }

        // Convert axis code to evdev absolute axis type
        let abs_axis = axis_code_to_type(axis_code)
            .ok_or_else(|| format!("Invalid axis code: {}", axis_code))?;

        debug!(
            "Analog move: axis_code={}, abs_axis={}, value={}",
            axis_code, abs_axis, value
        );
        self.write_event(EV_ABS, abs_axis, value)?;
        self.sync()?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Injector for UinputInjector {
    async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        UinputInjector::initialize(self).await
    }

    async fn key_press(
        &self,
        key_code: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        UinputInjector::key_press(self, key_code).await
    }

    async fn key_release(
        &self,
        key_code: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        UinputInjector::key_release(self, key_code).await
    }

    async fn mouse_press(
        &self,
        button: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        UinputInjector::mouse_press(self, button).await
    }

    async fn mouse_release(
        &self,
        button: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        UinputInjector::mouse_release(self, button).await
    }

    async fn mouse_move(
        &self,
        x: i32,
        y: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        UinputInjector::mouse_move(self, x, y).await
    }

    async fn mouse_scroll(
        &self,
        amount: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        UinputInjector::mouse_scroll(self, amount).await
    }

    async fn type_string(
        &self,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        UinputInjector::type_string(self, text).await
    }

    async fn execute_command(
        &self,
        command: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        UinputInjector::execute_command(self, command).await
    }

    async fn analog_move(
        &self,
        axis_code: u16,
        value: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        UinputInjector::analog_move(self, axis_code, value).await
    }
}

impl Drop for UinputInjector {
    fn drop(&mut self) {
        if let Ok(initialized) = self.initialized.try_read() {
            if *initialized {
                if let Ok(uinput_fd) = self.uinput_fd.try_read() {
                    if let Some(fd) = *uinput_fd {
                        info!("Destroying uinput virtual device");
                        unsafe {
                            libc::ioctl(fd, UI_DEV_DESTROY);
                            libc::close(fd);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_injector_creation() {
        let injector = UinputInjector::new().unwrap();
        assert!(!*injector.initialized.read().unwrap());
    }

    #[tokio::test]
    async fn test_key_map_setup() {
        let injector = UinputInjector::new().unwrap();
        let key_map = injector.key_map.read().unwrap();

        // Check that basic keys are mapped
        assert_eq!(key_map.get(&'a'), Some(&30));
        assert_eq!(key_map.get(&'A'), Some(&30));
        assert_eq!(key_map.get(&' '), Some(&57));
        assert_eq!(key_map.get(&'1'), Some(&2));
    }

    // Note: Actual injection tests require root privileges and /dev/uinput access
    // They should be run in integration tests with proper permissions
}
