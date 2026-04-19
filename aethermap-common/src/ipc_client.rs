//! IPC client for communicating with the aethermap daemon
//!
//! This module provides utilities for sending requests to the daemon and receiving responses
//! over a Unix domain socket with robust error handling, timeouts, and reconnection logic.

use crate::{Request, Response, AnalogCalibrationConfig};
use bincode;
use serde::{Serialize, de::DeserializeOwned};

use std::io;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use tokio::time::timeout;

/// Errors that can occur during IPC communication
#[derive(Error, Debug)]
pub enum IpcError {
    #[error("failed to connect to daemon: {0}")]
    Connect(std::io::Error),
    #[error("failed to send request: {0}")]
    Send(std::io::Error),
    #[error("failed to receive response: {0}")]
    Receive(std::io::Error),
    #[error("serialization error: {0}")]
    Serialize(bincode::Error),
    #[error("deserialization error: {0}")]
    Deserialize(bincode::Error),
    #[error("request timed out")]
    Timeout,

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Connection timeout")]
    ConnectionTimeout,

    #[error("Operation timeout after {0}ms")]
    OperationTimeout(u64),

    #[error("Daemon not running at {0}")]
    DaemonNotRunning(String),

    #[error("Invalid response from daemon")]
    InvalidResponse,

    #[error("Message too large: {0} bytes exceeds maximum of {1} bytes")]
    MessageTooLarge(usize, usize),

    #[error("Connection closed unexpectedly")]
    ConnectionClosed,

    #[error("Other error: {0}")]
    Other(String),
}

/// Default socket path for the aethermap daemon
pub const DEFAULT_SOCKET_PATH: &str = "/run/aethermap/aethermap.sock";

/// Default timeout for operations (in milliseconds)
pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// Maximum message size (1MB)
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// Maximum number of reconnection attempts
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Delay between reconnection attempts (in milliseconds)
pub const DEFAULT_RETRY_DELAY_MS: u64 = 1000;

/// IPC client with connection management and error handling
#[derive(Debug)]
pub struct IpcClient {
    socket_path: String,
    timeout: Duration,
    max_retries: u32,
    retry_delay: Duration,
    // stream: Option<Mutex<UnixStream>>, // Unused for now, will be implemented later
}

impl IpcClient {
    /// Create a new IPC client with default settings
    pub fn new() -> Self {
        Self::with_socket_path(DEFAULT_SOCKET_PATH)
    }

    /// Create a new IPC client with a custom socket path
    pub fn with_socket_path<P: AsRef<Path>>(socket_path: P) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_string_lossy().to_string(),
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            max_retries: DEFAULT_MAX_RETRIES,
            retry_delay: Duration::from_millis(DEFAULT_RETRY_DELAY_MS),
        }
    }

    /// Set the timeout for operations
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout = Duration::from_millis(timeout_ms);
        self
    }

    /// Set reconnection parameters
    pub fn with_retry_params(mut self, max_retries: u32, retry_delay_ms: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_delay = Duration::from_millis(retry_delay_ms);
        self
    }

    /// Check if the daemon is running by attempting to connect to its socket
    pub async fn is_daemon_running(&self) -> bool {
        match UnixStream::connect(&self.socket_path).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Connect to the daemon with retry logic
    pub async fn connect(&self) -> Result<UnixStream, IpcError> {
        let mut attempts = 0;

        loop {
            match timeout(self.timeout, UnixStream::connect(&self.socket_path)).await {
                Ok(Ok(stream)) => return Ok(stream),
                Ok(Err(e)) => {
                    if attempts >= self.max_retries {
                        return Err(IpcError::DaemonNotRunning(self.socket_path.clone()));
                    }
                    tracing::warn!("Connection attempt {} failed: {}, retrying...", attempts + 1, e);
                    tokio::time::sleep(self.retry_delay).await;
                    attempts += 1;
                }
                Err(_) => return Err(IpcError::ConnectionTimeout),
            }
        }
    }

    /// Send a request to the daemon and wait for a response with reconnection logic
    pub async fn send(&self, request: &Request) -> Result<Response, IpcError> {
        self.send_with_retries(request, self.max_retries).await
    }

    /// Send a request with a specific number of retries
    pub async fn send_with_retries(&self, request: &Request, max_retries: u32) -> Result<Response, IpcError> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= max_retries {
            match self.connect().await {
                Ok(mut stream) => {
                    match self.send_with_stream(&mut stream, request).await {
                        Ok(response) => return Ok(response),
                        Err(e) => {
                            last_error = Some(e);
                            if attempts < max_retries {
                                tracing::warn!("Request attempt {} failed, retrying...", attempts + 1);
                                tokio::time::sleep(self.retry_delay).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempts < max_retries {
                        tracing::warn!("Connection attempt {} failed, retrying...", attempts + 1);
                        tokio::time::sleep(self.retry_delay).await;
                    }
                }
            }
            attempts += 1;
        }

        Err(last_error.unwrap_or(IpcError::Other("Unknown error".to_string())))
    }

    /// Set global macro settings
    pub async fn set_macro_settings(&self, settings: crate::MacroSettings) -> Result<(), IpcError> {
        let request = Request::SetMacroSettings(settings);
        match self.send(&request).await? {
            Response::Ack => Ok(()),
            Response::Error(msg) => Err(IpcError::Other(msg)),
            _ => Err(IpcError::InvalidResponse),
        }
    }

    /// Get global macro settings
    pub async fn get_macro_settings(&self) -> Result<crate::MacroSettings, IpcError> {
        let request = Request::GetMacroSettings;
        match self.send(&request).await? {
            Response::MacroSettings(settings) => Ok(settings),
            Response::Error(msg) => Err(IpcError::Other(msg)),
            _ => Err(IpcError::InvalidResponse),
        }
    }

    /// Send a request using an existing stream
    async fn send_with_stream(&self, stream: &mut UnixStream, request: &Request) -> Result<Response, IpcError> {
        // Serialize the request
        let serialized = bincode::serialize(request)
            .map_err(|e| IpcError::Serialization(e.to_string()))?;

        // Check message size
        if serialized.len() > MAX_MESSAGE_SIZE {
            return Err(IpcError::MessageTooLarge(serialized.len(), MAX_MESSAGE_SIZE));
        }

        // Send the request with timeout
        if let Err(_) = timeout(self.timeout, async {
            // Write the length of the message first (4 bytes little endian)
            let len = serialized.len() as u32;
            stream.write_all(&len.to_le_bytes()).await?;

            // Write the actual message
            stream.write_all(&serialized).await?;
            stream.flush().await?;

            Ok::<(), io::Error>(())
        }).await {
            return Err(IpcError::OperationTimeout(self.timeout.as_millis() as u64));
        }

        // Read the response with timeout
        let response = timeout(self.timeout, async {
            // Read the response length first
            let mut len_bytes = [0u8; 4];
            stream.read_exact(&mut len_bytes).await?;
            let response_len = u32::from_le_bytes(len_bytes) as usize;

            // Validate response length
            if response_len > MAX_MESSAGE_SIZE {
                return Err(IpcError::MessageTooLarge(response_len, MAX_MESSAGE_SIZE));
            }

            // Read the response
            let mut buffer = vec![0u8; response_len];
            stream.read_exact(&mut buffer).await?;

            // Deserialize the response
            bincode::deserialize(&buffer)
                .map_err(|e| IpcError::Serialization(e.to_string()))
        }).await;

        match response {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(IpcError::OperationTimeout(self.timeout.as_millis() as u64)),
        }
    }
}

/// Send a request to the daemon using the default client
///
/// # Arguments
///
/// * `request` - The request to send to the daemon
///
/// # Returns
///
/// Returns the response from the daemon or an IpcError if communication fails
///
/// # Example
///
/// ```rust,no_run
/// use aethermap_common::{ipc_client, Request};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let response = ipc_client::send(&Request::GetDevices).await?;
///     println!("Got response: {:?}", response);
///     Ok(())
/// }
/// ```
pub async fn send(request: &Request) -> Result<Response, IpcError> {
    let client = IpcClient::new();
    client.send(request).await
}

/// Send a request to the aethermap daemon
///
/// This function connects to the daemon socket at /run/aethermap.sock,
/// serializes the request using bincode, sends it with a length prefix,
/// and returns the deserialized response.
///
/// # Arguments
///
/// * `req` - The request to send to the daemon
///
/// # Returns
///
/// Returns the Response from the daemon or an IpcError if something went wrong.
pub async fn send_request(req: &Request) -> Result<Response, IpcError> {
    // Connect to the daemon socket
    let mut stream = timeout(
        Duration::from_secs(2),
        UnixStream::connect(DEFAULT_SOCKET_PATH)
    )
    .await
    .map_err(|_| IpcError::Timeout)?
    .map_err(IpcError::Connect)?;

    // Serialize the request
    let serialized = bincode::serialize(req).map_err(IpcError::Serialize)?;

    // Check message size
    if serialized.len() > MAX_MESSAGE_SIZE {
        return Err(IpcError::MessageTooLarge(serialized.len(), MAX_MESSAGE_SIZE));
    }

    // Write the length prefix (u32 little endian) and the payload
    let len_prefix = (serialized.len() as u32).to_le_bytes();
    timeout(
        Duration::from_secs(2),
        stream.write_all(&len_prefix)
    )
    .await
    .map_err(|_| IpcError::Timeout)?
    .map_err(IpcError::Send)?;

    timeout(
        Duration::from_secs(2),
        stream.write_all(&serialized)
    )
    .await
    .map_err(|_| IpcError::Timeout)?
    .map_err(IpcError::Send)?;

    // Read the length prefix of the response
    let mut response_len_bytes = [0u8; 4];
    timeout(
        Duration::from_secs(2),
        stream.read_exact(&mut response_len_bytes)
    )
    .await
    .map_err(|_| IpcError::Timeout)?
    .map_err(IpcError::Receive)?;

    let response_len = u32::from_le_bytes(response_len_bytes) as usize;

    // Check response size
    if response_len > MAX_MESSAGE_SIZE {
        return Err(IpcError::MessageTooLarge(response_len, MAX_MESSAGE_SIZE));
    }

    // Read the response payload
    let mut response_buffer = vec![0u8; response_len];
    timeout(
        Duration::from_secs(2),
        stream.read_exact(&mut response_buffer)
    )
    .await
    .map_err(|_| IpcError::Timeout)?
    .map_err(IpcError::Receive)?;

    // Deserialize and return the response
    bincode::deserialize(&response_buffer).map_err(IpcError::Deserialize)
}

/// Send a request to the daemon at a specific socket path
///
/// # Arguments
///
/// * `request` - The request to send to the daemon
/// * `socket_path` - Path to the Unix domain socket
///
/// # Returns
///
/// Returns the response from the daemon or an IpcError if communication fails
pub async fn send_to_path<P: AsRef<Path>>(request: &Request, socket_path: P) -> Result<Response, IpcError> {
    let client = IpcClient::with_socket_path(socket_path);
    client.send(request).await
}

/// Send a request with a custom timeout
///
/// # Arguments
///
/// * `request` - The request to send to the daemon
/// * `timeout_ms` - Timeout in milliseconds
///
/// # Returns
///
/// Returns the response from the daemon or an IpcError if communication fails
pub async fn send_with_timeout(request: &Request, timeout_ms: u64) -> Result<Response, IpcError> {
    let client = IpcClient::new().with_timeout(timeout_ms);
    client.send(request).await
}

/// Check if the daemon is running by attempting to connect to its socket
///
/// # Arguments
///
/// * `socket_path` - Optional custom socket path, defaults to DEFAULT_SOCKET_PATH
///
/// # Returns
///
/// Returns true if the daemon is running, false otherwise
pub async fn is_daemon_running<P: AsRef<Path>>(socket_path: Option<P>) -> bool {
    let path = socket_path.map(|p| p.as_ref().to_string_lossy().to_string())
        .unwrap_or_else(|| DEFAULT_SOCKET_PATH.to_string());

    match UnixStream::connect(path).await {
        Ok(_) => true,
        Err(_) => false,
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
/// * `Err(IpcError)` - Communication error or device not found
///
/// # Example
///
/// ```rust,no_run
/// use aethermap_common::ipc_client;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let calibration = ipc_client::get_analog_calibration("32b6:12f7", 0).await?;
///     println!("Deadzone: {}", calibration.deadzone);
///     Ok(())
/// }
/// ```
pub async fn get_analog_calibration(
    device_id: &str,
    layer_id: usize,
) -> Result<AnalogCalibrationConfig, IpcError> {
    let request = Request::GetAnalogCalibration {
        device_id: device_id.to_string(),
        layer_id,
    };

    match send(&request).await? {
        Response::AnalogCalibration { calibration: Some(cal), .. } => Ok(cal),
        Response::AnalogCalibration { calibration: None, .. } => {
            // Return default config
            Ok(AnalogCalibrationConfig::default())
        }
        Response::Error(msg) => Err(IpcError::Other(msg)),
        _ => Err(IpcError::InvalidResponse),
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
/// * `Err(IpcError)` - Communication or validation error
///
/// # Example
///
/// ```rust,no_run
/// use aethermap_common::{ipc_client, AnalogCalibrationConfig, AnalogMode};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let calibration = AnalogCalibrationConfig {
///         deadzone: 0.2,
///         deadzone_shape: "circular".to_string(),
///         sensitivity: "quadratic".to_string(),
///         sensitivity_multiplier: 1.5,
///         range_min: -32768,
///         range_max: 32767,
///         invert_x: false,
///         invert_y: false,
///         exponent: 2.0,
///         analog_mode: AnalogMode::Disabled,
///         camera_output_mode: None,
///     };
///
///     ipc_client::set_analog_calibration("32b6:12f7", 0, calibration).await?;
///     Ok(())
/// }
/// ```
pub async fn set_analog_calibration(
    device_id: &str,
    layer_id: usize,
    calibration: AnalogCalibrationConfig,
) -> Result<(), IpcError> {
    let request = Request::SetAnalogCalibration {
        device_id: device_id.to_string(),
        layer_id,
        calibration,
    };

    match send(&request).await? {
        Response::AnalogCalibrationAck => Ok(()),
        Response::Error(msg) => Err(IpcError::Other(msg)),
        _ => Err(IpcError::InvalidResponse),
    }
}

/// Get all auto-switch rules
///
/// # Returns
///
/// * `Ok(Vec<AutoSwitchRule>)` - All configured auto-switch rules
/// * `Err(IpcError)` - Communication error
///
/// # Example
///
/// ```rust,no_run
/// use aethermap_common::ipc_client;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let rules = ipc_client::get_auto_switch_rules().await?;
///     for rule in rules {
///         println!("App '{}' -> Profile '{}'", rule.app_id, rule.profile_name);
///     }
///     Ok(())
/// }
/// ```
pub async fn get_auto_switch_rules() -> Result<Vec<crate::AutoSwitchRule>, IpcError> {
    let request = Request::GetAutoSwitchRules;

    match send(&request).await? {
        Response::AutoSwitchRules { rules } => Ok(rules),
        Response::Error(msg) => Err(IpcError::Other(msg)),
        _ => Err(IpcError::InvalidResponse),
    }
}

/// Get global macro settings
pub async fn get_macro_settings() -> Result<crate::MacroSettings, IpcError> {
    IpcClient::new().get_macro_settings().await
}

/// Set global macro settings
pub async fn set_macro_settings(settings: crate::MacroSettings) -> Result<(), IpcError> {
    IpcClient::new().set_macro_settings(settings).await
}

/// Serialize a message using bincode
pub fn serialize<T: Serialize>(msg: &T) -> Result<Vec<u8>, IpcError> {
    bincode::serialize(msg)
        .map_err(|e| IpcError::Serialization(e.to_string()))
}

/// Deserialize a message using bincode
pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, IpcError> {
    bincode::deserialize(bytes)
        .map_err(|e| IpcError::Serialization(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Request, Response, DeviceInfo, DeviceType, Action, KeyCombo, MacroEntry};
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::net::UnixListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Mock daemon server for testing
    async fn mock_daemon(socket_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Remove any existing socket file
        if Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path)?;
        }

        let listener = UnixListener::bind(socket_path)?;

        loop {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    // Handle the connection in a new task
                    tokio::spawn(async move {
                        // Read the request
                        let mut len_buf = [0u8; 4];
                        if let Err(_) = stream.read_exact(&mut len_buf).await {
                            return;
                        }

                        let msg_len = u32::from_le_bytes(len_buf) as usize;
                        if msg_len > MAX_MESSAGE_SIZE {
                            return;
                        }

                        let mut msg_buf = vec![0u8; msg_len];
                        if let Err(_) = stream.read_exact(&mut msg_buf).await {
                            return;
                        }

                        // Deserialize the request
                        let request: Request = match bincode::deserialize(&msg_buf) {
                            Ok(req) => req,
                            Err(_) => return,
                        };

                        // Generate a response
                        let response = match request {
                            Request::GetDevices => {
                                let devices = vec![
                                    DeviceInfo {
                                        name: "Test Device".to_string(),
                                        path: PathBuf::from("/dev/input/event0"),
                                        vendor_id: 0x1234,
                                        product_id: 0x5678,
                                        phys: "usb-0000:00:14.0-1/input0".to_string(),
                                        device_type: DeviceType::Other,
                                    }
                                ];
                                Response::Devices(devices)
                            },
                            Request::ListMacros => {
                                let macros = vec![
                                    MacroEntry {
                                        name: "Test Macro".to_string(),
                                        trigger: KeyCombo {
                                            keys: vec![30], // A key
                                            modifiers: vec![],
                                        },
                                        actions: vec![
                                            Action::KeyPress(31), // Press B
                                            Action::Delay(100),
                                            Action::KeyRelease(31), // Release B
                                        ],
                                        device_id: None,
                                        enabled: true,
                                        humanize: false,
                                        capture_mouse: false,
                                    }
                                ];
                                Response::Macros(macros)
                            },
                            Request::GetStatus => {
                                Response::Status {
                                    version: "0.1.0".to_string(),
                                    uptime_seconds: 60,
                                    devices_count: 1,
                                    macros_count: 1,
                                }
                            },
                            _ => Response::Error("Unsupported request in test".to_string()),
                        };

                        // Send the response
                        let response_bytes = bincode::serialize(&response).unwrap();
                        let len = response_bytes.len() as u32;

                        if let Err(_) = stream.write_all(&len.to_le_bytes()).await {
                            return;
                        }

                        if let Err(_) = stream.write_all(&response_bytes).await {
                            return;
                        }

                        let _ = stream.flush().await;
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to accept connection: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_ipc_client_creation() {
        let client = IpcClient::new();
        assert_eq!(client.socket_path, DEFAULT_SOCKET_PATH);
        assert_eq!(client.timeout, Duration::from_millis(DEFAULT_TIMEOUT_MS));
        assert_eq!(client.max_retries, DEFAULT_MAX_RETRIES);
        assert_eq!(client.retry_delay, Duration::from_millis(DEFAULT_RETRY_DELAY_MS));

        let custom_path = "/tmp/test.sock";
        let custom_client = IpcClient::with_socket_path(custom_path)
            .with_timeout(10000)
            .with_retry_params(5, 2000);

        assert_eq!(custom_client.socket_path, custom_path);
        assert_eq!(custom_client.timeout, Duration::from_millis(10000));
        assert_eq!(custom_client.max_retries, 5);
        assert_eq!(custom_client.retry_delay, Duration::from_millis(2000));
    }

    #[tokio::test]
    async fn test_serialization_deserialization() {
        let request = Request::GetDevices;
        let serialized = serialize(&request).unwrap();
        let deserialized: Request = deserialize(&serialized).unwrap();
        assert!(matches!(deserialized, Request::GetDevices));

        let macro_entry = MacroEntry {
            name: "Test Macro".to_string(),
            trigger: KeyCombo {
                keys: vec![30, 40], // A and D keys
                modifiers: vec![29], // Ctrl key
            },
            actions: vec![
                Action::KeyPress(30),
                Action::Delay(100),
                Action::KeyRelease(30),
            ],
            device_id: Some("test_device".to_string()),
            enabled: true,
            humanize: false,
            capture_mouse: false,
        };

        let serialized = serialize(&macro_entry).unwrap();
        let deserialized: MacroEntry = deserialize(&serialized).unwrap();
        assert_eq!(deserialized.name, "Test Macro");
        assert_eq!(deserialized.trigger.keys, vec![30, 40]);
    }

    #[tokio::test]
    async fn test_client_server_communication() {
        // Create a temporary directory for our test socket
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        let socket_path_str = socket_path.to_string_lossy().to_string();
        let socket_path_clone = socket_path_str.clone();

        // Start a mock daemon in the background
        tokio::spawn(async move {
            mock_daemon(&socket_path_clone).await
        });

        // Give the mock daemon time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Test the client
        let client = IpcClient::with_socket_path(&socket_path_str);

        // Test is_daemon_running
        assert!(client.is_daemon_running().await);

        // Test sending a GetDevices request
        let response = client.send(&Request::GetDevices).await.unwrap();
        if let Response::Devices(devices) = response {
            assert_eq!(devices.len(), 1);
            assert_eq!(devices[0].name, "Test Device");
        } else {
            panic!("Expected Devices response");
        }

        // Test sending a ListMacros request
        let response = client.send(&Request::ListMacros).await.unwrap();
        if let Response::Macros(macros) = response {
            assert_eq!(macros.len(), 1);
            assert_eq!(macros[0].name, "Test Macro");
        } else {
            panic!("Expected Macros response");
        }

        // Test sending a GetStatus request
        let response = client.send(&Request::GetStatus).await.unwrap();
        if let Response::Status { version, uptime_seconds, devices_count, macros_count } = response {
            assert_eq!(version, "0.1.0");
            assert_eq!(uptime_seconds, 60);
            assert_eq!(devices_count, 1);
            assert_eq!(macros_count, 1);
        } else {
            panic!("Expected Status response");
        }

        // Test the convenience function
        let response = send_to_path(&Request::GetDevices, &socket_path_str).await.unwrap();
        if let Response::Devices(devices) = response {
            assert_eq!(devices.len(), 1);
        } else {
            panic!("Expected Devices response");
        }
    }

    #[tokio::test]
    async fn test_connection_timeout() {
        // Use a non-existent socket path
        let client = IpcClient::with_socket_path("/tmp/nonexistent.sock")
            .with_timeout(100) // Very short timeout
            .with_retry_params(1, 100); // Minimal retries

        // Should fail with DaemonNotRunning or ConnectionTimeout
        match client.send(&Request::GetDevices).await {
            Err(IpcError::DaemonNotRunning(_)) | Err(IpcError::ConnectionTimeout) => {
                // Expected outcome
            },
            _ => panic!("Expected DaemonNotRunning or ConnectionTimeout error"),
        }
    }

    #[tokio::test]
    async fn test_is_daemon_running() {
        // Test with non-existent socket
        assert!(!is_daemon_running(Some("/tmp/nonexistent.sock")).await);

        // Test with default socket (likely not running in test environment)
        assert!(!is_daemon_running(None::<&str>).await);
    }

    #[test]
    fn test_serialization_roundtrip() {
        // Test Request serialization and deserialization
        let request = Request::GetDevices;
        let serialized = bincode::serialize(&request).map_err(IpcError::Serialize).unwrap();
        let deserialized: Request = bincode::deserialize(&serialized).map_err(IpcError::Deserialize).unwrap();
        assert!(matches!(deserialized, Request::GetDevices));

        // Test Response serialization and deserialization
        let devices = vec![
            DeviceInfo {
                name: "Test Device".to_string(),
                path: std::path::PathBuf::from("/dev/input/test"),
                vendor_id: 0x1532,
                product_id: 0x0221,
                phys: "usb-0000:00:14.0-1/input0".to_string(),
                device_type: DeviceType::Other,
            }
        ];
        let response = Response::Devices(devices.clone());
        let serialized = bincode::serialize(&response).map_err(IpcError::Serialize).unwrap();
        let deserialized: Response = bincode::deserialize(&serialized).map_err(IpcError::Deserialize).unwrap();

        if let Response::Devices(deserialized_devices) = deserialized {
            assert_eq!(deserialized_devices.len(), devices.len());
            assert_eq!(deserialized_devices[0].name, devices[0].name);
            assert_eq!(deserialized_devices[0].vendor_id, devices[0].vendor_id);
        } else {
            panic!("Expected Devices response");
        }
    }

    #[test]
    fn test_send_request_error_handling() {
        // Test serialization error handling in send_request
        // We can't test the full send_request function without a socket,
        // but we can test the serialization part that it uses

        // Test valid request
        let request = Request::GetDevices;
        let result = bincode::serialize(&request);
        assert!(result.is_ok());

        // Test deserialization error handling
        let invalid_data = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let result: Result<Request, bincode::Error> = bincode::deserialize(&invalid_data);
        assert!(result.is_err());

        // Verify our error handling matches
        let _serialized = bincode::serialize(&request).unwrap();
        let error = bincode::deserialize::<Request>(&invalid_data).map_err(IpcError::Deserialize);
        assert!(matches!(error, Err(IpcError::Deserialize(_))));
    }
}
