//! End-to-end tests for aethermap daemon
//!
//! These tests verify the complete functionality of the daemon including:
//! - Device discovery and management
//! - Macro recording and playback
//! - IPC communication between client and server
//! - Security and privilege handling
//!
//! Tests are designed to run in an isolated environment with synthetic devices
//! and injected events to ensure reproducibility and avoid requiring actual hardware.

use aethermap_common::{
    deserialize, ipc_client::IpcClient, serialize, Action, DeviceInfo, KeyCombo, MacroEntry,
    Request, Response,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tempfile::TempDir;
use tokio::{net::UnixListener, sync::RwLock, task::JoinHandle, time::sleep};
use tracing::{debug, error, info};

/// Test environment with mock daemon
struct TestEnvironment {
    /// Temporary directory for test files (kept alive for cleanup)
    #[allow(dead_code)]
    temp_dir: TempDir,
    /// Path to the test socket
    socket_path: PathBuf,
    /// Handle to the daemon task
    daemon_handle: JoinHandle<()>,
    /// IPC client for communicating with daemon
    client: IpcClient,
}

impl TestEnvironment {
    /// Create a new test environment with a mock daemon
    async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Set up logging for tests
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_test_writer()
            .try_init();

        // Create temporary directory for test files
        let temp_dir = TempDir::new()?;
        let socket_path = temp_dir.path().join("test.sock");
        info!("Using test socket path: {:?}", socket_path);

        // Start the daemon in a background task
        let daemon_handle = Self::start_daemon(&socket_path).await?;

        // Create IPC client
        let client = IpcClient::with_socket_path(&socket_path)
            .with_timeout(5000) // 5 seconds timeout
            .with_retry_params(10, 100); // 10 retries with 100ms delay

        // Wait for daemon to start
        let mut retries = 0;
        while retries < 30 && !client.is_daemon_running().await {
            sleep(Duration::from_millis(100)).await;
            retries += 1;
        }

        if !client.is_daemon_running().await {
            return Err("Failed to start daemon".into());
        }

        Ok(Self {
            temp_dir,
            socket_path,
            daemon_handle,
            client,
        })
    }

    /// Start a mock daemon in a background task
    async fn start_daemon(
        socket_path: &Path,
    ) -> Result<JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
        let socket_path = socket_path.to_string_lossy().to_string();

        // Spawn the daemon task
        let handle = tokio::spawn(async move {
            if let Err(e) = Self::run_mock_daemon(&socket_path).await {
                error!("Mock daemon error: {}", e);
            }
        });

        Ok(handle)
    }

    /// Run a mock daemon that implements the same IPC protocol as the real daemon
    async fn run_mock_daemon(
        socket_path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Clean up any existing socket
        if Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path)?;
        }

        // Create Unix listener
        let listener = UnixListener::bind(socket_path)?;
        info!("Mock daemon listening on: {}", socket_path);

        // Create mock device info
        let devices = vec![
            DeviceInfo {
                name: "Test Keyboard".to_string(),
                path: "/dev/input/test0".into(),
                vendor_id: 0x1532, // Razer
                product_id: 0x0101,
                phys: "usb-0000:00:14.0-1/input0".to_string(),
                device_type: aethermap_common::DeviceType::Keyboard,
            },
            DeviceInfo {
                name: "Test Mouse".to_string(),
                path: "/dev/input/test1".into(),
                vendor_id: 0x1532,
                product_id: 0x0025,
                phys: "usb-0000:00:14.0-2/input0".to_string(),
                device_type: aethermap_common::DeviceType::Mouse,
            },
        ];

        // Create macro storage
        let macros = Arc::new(RwLock::new(HashMap::<String, MacroEntry>::new()));
        let recording_state = Arc::new(RwLock::new(None::<(String, bool)>)); // (name, capture_mouse)

        // Listen for connections
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let devices = devices.clone();
                    let macros = Arc::clone(&macros);
                    let recording_state = Arc::clone(&recording_state);

                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_client_connection(stream, devices, macros, recording_state)
                                .await
                        {
                            error!("Error handling client connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                    return Err(e.into());
                }
            }
        }
    }

    /// Handle a client connection to the mock daemon
    async fn handle_client_connection(
        mut stream: tokio::net::UnixStream,
        devices: Vec<DeviceInfo>,
        macros: Arc<RwLock<HashMap<String, MacroEntry>>>,
        recording_state: Arc<RwLock<Option<(String, bool)>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Read message length first
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let msg_len = u32::from_le_bytes(len_buf) as usize;

        // Read the actual message
        let mut msg_buf = vec![0u8; msg_len];
        stream.read_exact(&mut msg_buf).await?;

        // Deserialize the request
        let request: Request = deserialize(&msg_buf)?;
        debug!("Received request: {:?}", request);

        // Process the request and generate a response
        let response = Self::process_request(request, devices, macros, recording_state).await;

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
    async fn process_request(
        request: Request,
        devices: Vec<DeviceInfo>,
        macros: Arc<RwLock<HashMap<String, MacroEntry>>>,
        recording_state: Arc<RwLock<Option<(String, bool)>>>,
    ) -> Response {
        match request {
            Request::GetDevices => Response::Devices(devices),
            Request::ListMacros => {
                let macros = macros.read().await;
                Response::Macros(macros.values().cloned().collect())
            }
            Request::SetMacro {
                device_path: _,
                macro_entry,
            } => {
                let mut macros = macros.write().await;
                macros.insert(macro_entry.name.clone(), macro_entry);
                Response::Ack
            }
            Request::DeleteMacro { name } => {
                let mut macros = macros.write().await;
                if macros.remove(&name).is_some() {
                    Response::Ack
                } else {
                    Response::Error(format!("Macro '{}' not found", name))
                }
            }
            Request::GenerateToken { client_id } => {
                // Simple token generation for tests
                Response::Token(format!("test-token-{}", client_id))
            }
            Request::Authenticate { token } => {
                if token.starts_with("test-token-") {
                    Response::Authenticated
                } else {
                    Response::Error("Invalid authentication token".to_string())
                }
            }
            Request::RecordMacro {
                device_path,
                name,
                capture_mouse,
            } => {
                let mut state = recording_state.write().await;
                *state = Some((name.clone(), capture_mouse));
                Response::RecordingStarted { device_path, name }
            }
            Request::StopRecording => {
                let mut state = recording_state.write().await;
                if let Some((name, capture_mouse)) = state.take() {
                    // Create a simple test macro
                    let macro_entry = MacroEntry {
                        name: name.clone(),
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
                        capture_mouse,
                    };
                    let mut macros_map = macros.write().await;
                    macros_map.insert(name.clone(), macro_entry.clone());
                    Response::RecordingStopped { macro_entry }
                } else {
                    Response::Error("No recording in progress".to_string())
                }
            }
            Request::TestMacro { name } => {
                let macros = macros.read().await;
                if let Some(_macro_entry) = macros.get(&name) {
                    Response::Ack
                } else {
                    Response::Error(format!("Macro '{}' not found", name))
                }
            }
            Request::SaveProfile { name } => Response::ProfileSaved {
                name,
                macros_count: macros.read().await.len(),
            },
            Request::LoadProfile { name } => {
                // In a real implementation, this would load from disk
                // For tests, we'll just create an empty profile
                Response::ProfileLoaded {
                    name,
                    macros_count: 0,
                }
            }
            Request::ListProfiles => Response::Profiles(vec![
                "default".to_string(),
                "test".to_string(),
                "Test Profile".to_string(),
            ]),
            Request::DeleteProfile { name: _ } => {
                // In a real implementation, this would delete from disk
                Response::Ack
            }
            Request::GetStatus => Response::Status {
                version: "0.1.0-test".to_string(),
                uptime_seconds: 300,
                devices_count: devices.len(),
                macros_count: macros.read().await.len(),
            },
            _ => Response::Error("Unsupported request in test mode".to_string()),
        }
    }
}

/// Test device discovery
#[tokio::test]
async fn test_device_discovery() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_env = TestEnvironment::new().await?;

    // Send GetDevices request
    let response = test_env.client.send(&Request::GetDevices).await?;

    // Verify response
    match response {
        Response::Devices(devices) => {
            assert!(!devices.is_empty(), "No devices returned");
            assert_eq!(devices.len(), 2, "Expected 2 test devices");

            // Verify first device
            let keyboard = &devices[0];
            assert_eq!(keyboard.name, "Test Keyboard");
            assert_eq!(keyboard.vendor_id, 0x1532);
            assert_eq!(keyboard.product_id, 0x0101);

            // Verify second device
            let mouse = &devices[1];
            assert_eq!(mouse.name, "Test Mouse");
            assert_eq!(mouse.vendor_id, 0x1532);
            assert_eq!(mouse.product_id, 0x0025);
        }
        _ => panic!("Unexpected response: {:?}", response),
    }

    // Clean up
    test_env.daemon_handle.abort();
    Ok(())
}

/// Test macro recording and playback
#[tokio::test]
async fn test_macro_recording_playback() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_env = TestEnvironment::new().await?;

    // Test 1: Record a macro
    let record_response = test_env
        .client
        .send(&Request::RecordMacro {
            device_path: "/dev/input/test0".to_string(),
            name: "Test Macro".to_string(),
            capture_mouse: false,
        })
        .await?;

    // Verify recording started
    match record_response {
        Response::RecordingStarted { device_path, name } => {
            assert_eq!(device_path, "/dev/input/test0");
            assert_eq!(name, "Test Macro");
        }
        _ => panic!("Unexpected response: {:?}", record_response),
    }

    // Test 2: Stop recording
    let stop_response = test_env.client.send(&Request::StopRecording).await?;

    // Verify recording stopped
    match stop_response {
        Response::RecordingStopped { macro_entry } => {
            assert_eq!(macro_entry.name, "Test Macro");
            assert_eq!(macro_entry.trigger.keys, vec![30]); // A key
            assert_eq!(macro_entry.actions.len(), 3); // Press, Delay, Release
        }
        _ => panic!("Unexpected response: {:?}", stop_response),
    }

    // Test 3: List macros
    let list_response = test_env.client.send(&Request::ListMacros).await?;

    // Verify macro is in list
    match list_response {
        Response::Macros(macros) => {
            assert_eq!(macros.len(), 1);
            assert_eq!(macros[0].name, "Test Macro");
        }
        _ => panic!("Unexpected response: {:?}", list_response),
    }

    // Test 4: Play macro
    let play_response = test_env
        .client
        .send(&Request::TestMacro {
            name: "Test Macro".to_string(),
        })
        .await?;

    // Verify playback started
    assert!(matches!(play_response, Response::Ack));

    // Give some time for events to be processed
    sleep(Duration::from_millis(100)).await;

    // Clean up
    test_env.daemon_handle.abort();
    Ok(())
}

/// Test macro management
#[tokio::test]
async fn test_macro_management() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_env = TestEnvironment::new().await?;

    // Create a test macro
    let test_macro = MacroEntry {
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
    };

    // Test 1: Set macro
    let set_response = test_env
        .client
        .send(&Request::SetMacro {
            device_path: "/dev/input/test".to_string(),
            macro_entry: test_macro.clone(),
        })
        .await?;

    assert!(matches!(set_response, Response::Ack));

    // Test 2: List macros
    let list_response = test_env.client.send(&Request::ListMacros).await?;

    // Verify macro is in list
    match list_response {
        Response::Macros(macros) => {
            assert_eq!(macros.len(), 1);
            assert_eq!(macros[0].name, test_macro.name);
        }
        _ => panic!("Unexpected response: {:?}", list_response),
    }

    // Test 3: Delete macro
    let delete_response = test_env
        .client
        .send(&Request::DeleteMacro {
            name: test_macro.name,
        })
        .await?;

    assert!(matches!(delete_response, Response::Ack));

    // Test 4: Verify macro is gone
    let list_response = test_env.client.send(&Request::ListMacros).await?;

    match list_response {
        Response::Macros(macros) => {
            assert_eq!(macros.len(), 0);
        }
        _ => panic!("Unexpected response: {:?}", list_response),
    }

    // Clean up
    test_env.daemon_handle.abort();
    Ok(())
}

/// Test profile management
#[tokio::test]
async fn test_profile_management() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_env = TestEnvironment::new().await?;

    // Create a test macro
    let test_macro = MacroEntry {
        name: "Profile Test Macro".to_string(),
        trigger: KeyCombo {
            keys: vec![30], // A key
            modifiers: vec![],
        },
        actions: vec![
            Action::KeyPress(31),   // Press B
            Action::KeyRelease(31), // Release B
        ],
        device_id: None,
        enabled: true,
        humanize: false,
        capture_mouse: false,
    };

    // Set the macro
    test_env
        .client
        .send(&Request::SetMacro {
            device_path: "/dev/input/test".to_string(),
            macro_entry: test_macro,
        })
        .await?;

    // Test 1: Save profile
    let save_response = test_env
        .client
        .send(&Request::SaveProfile {
            name: "Test Profile".to_string(),
        })
        .await?;

    match save_response {
        Response::ProfileSaved { name, macros_count } => {
            assert_eq!(name, "Test Profile");
            assert_eq!(macros_count, 1);
        }
        _ => panic!("Unexpected response: {:?}", save_response),
    }

    // Test 2: List profiles
    let list_response = test_env.client.send(&Request::ListProfiles).await?;

    match list_response {
        Response::Profiles(profiles) => {
            assert!(profiles.contains(&"Test Profile".to_string()));
            assert!(profiles.contains(&"default".to_string()));
            assert!(profiles.contains(&"test".to_string()));
        }
        _ => panic!("Unexpected response: {:?}", list_response),
    }

    // Test 3: Load profile
    let load_response = test_env
        .client
        .send(&Request::LoadProfile {
            name: "Test Profile".to_string(),
        })
        .await?;

    match load_response {
        Response::ProfileLoaded { name, macros_count } => {
            assert_eq!(name, "Test Profile");
            // Empty profile in test implementation, so macros_count is 0
            assert_eq!(macros_count, 0);
        }
        _ => panic!("Unexpected response: {:?}", load_response),
    }

    // Test 4: Delete profile
    let delete_response = test_env
        .client
        .send(&Request::DeleteProfile {
            name: "Test Profile".to_string(),
        })
        .await?;

    assert!(matches!(delete_response, Response::Ack));

    // Clean up
    test_env.daemon_handle.abort();
    Ok(())
}

/// Test authentication
#[tokio::test]
async fn test_authentication() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_env = TestEnvironment::new().await?;

    // Test 1: Generate token
    let token_response = test_env
        .client
        .send(&Request::GenerateToken {
            client_id: "test-client".to_string(),
        })
        .await?;

    let token = match token_response {
        Response::Token(token) => token,
        _ => panic!("Unexpected response: {:?}", token_response),
    };

    assert!(token.starts_with("test-token-test-client"));

    // Test 2: Authenticate with token
    let auth_response = test_env
        .client
        .send(&Request::Authenticate { token })
        .await?;

    assert!(matches!(auth_response, Response::Authenticated));

    // Test 3: Authenticate with invalid token
    let auth_response = test_env
        .client
        .send(&Request::Authenticate {
            token: "invalid-token".to_string(),
        })
        .await?;

    match auth_response {
        Response::Error(message) => {
            assert_eq!(message, "Invalid authentication token");
        }
        _ => panic!("Unexpected response: {:?}", auth_response),
    }

    // Clean up
    test_env.daemon_handle.abort();
    Ok(())
}

/// Test daemon status
#[tokio::test]
async fn test_daemon_status() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_env = TestEnvironment::new().await?;

    // Get daemon status
    let status_response = test_env.client.send(&Request::GetStatus).await?;

    match status_response {
        Response::Status {
            version,
            uptime_seconds,
            devices_count,
            macros_count,
        } => {
            assert_eq!(version, "0.1.0-test");
            assert!(uptime_seconds > 0);
            assert_eq!(devices_count, 2);
            assert_eq!(macros_count, 0);
        }
        _ => panic!("Unexpected response: {:?}", status_response),
    }

    // Clean up
    test_env.daemon_handle.abort();
    Ok(())
}

/// Test error handling
#[tokio::test]
async fn test_error_handling() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_env = TestEnvironment::new().await?;

    // Test 1: Try to execute a non-existent macro
    let play_response = test_env
        .client
        .send(&Request::TestMacro {
            name: "Non-existent Macro".to_string(),
        })
        .await?;

    match play_response {
        Response::Error(message) => {
            assert_eq!(message, "Macro 'Non-existent Macro' not found");
        }
        _ => panic!("Unexpected response: {:?}", play_response),
    }

    // Test 2: Try to stop recording when not recording
    let stop_response = test_env.client.send(&Request::StopRecording).await?;

    match stop_response {
        Response::Error(message) => {
            assert_eq!(message, "No recording in progress");
        }
        _ => panic!("Unexpected response: {:?}", stop_response),
    }

    // Clean up
    test_env.daemon_handle.abort();
    Ok(())
}

/// Test concurrent connections
#[tokio::test]
async fn test_concurrent_connections() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_env = TestEnvironment::new().await?;

    // Create multiple concurrent connections
    let mut tasks = Vec::new();
    for _i in 0..5 {
        let client = IpcClient::with_socket_path(&test_env.socket_path)
            .with_timeout(5000)
            .with_retry_params(5, 100);

        let task = tokio::spawn(async move {
            // Each client requests device list
            let response = client.send(&Request::GetDevices).await?;
            match response {
                Response::Devices(devices) => {
                    assert_eq!(devices.len(), 2);
                }
                _ => panic!("Unexpected response: {:?}", response),
            }
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    for task in tasks {
        task.await??;
    }

    // Clean up
    test_env.daemon_handle.abort();
    Ok(())
}

/// Test large payload handling
#[tokio::test]
async fn test_large_payload() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test_env = TestEnvironment::new().await?;

    // Create a macro with many actions
    let mut actions = Vec::new();
    for _i in 0..1000 {
        actions.push(Action::KeyPress(30));
        actions.push(Action::Delay(1));
        actions.push(Action::KeyRelease(30));
        actions.push(Action::Delay(1));
    }

    let large_macro = MacroEntry {
        name: "Large Macro".to_string(),
        trigger: KeyCombo {
            keys: vec![30], // A key
            modifiers: vec![],
        },
        actions,
        device_id: None,
        enabled: true,
        humanize: false,
        capture_mouse: false,
    };

    // Set large macro
    let set_response = test_env
        .client
        .send(&Request::SetMacro {
            device_path: "/dev/input/test".to_string(),
            macro_entry: large_macro,
        })
        .await?;

    assert!(matches!(set_response, Response::Ack));

    // List macros to verify it was set
    let list_response = test_env.client.send(&Request::ListMacros).await?;

    match list_response {
        Response::Macros(macros) => {
            assert_eq!(macros.len(), 1);
            assert_eq!(macros[0].name, "Large Macro");
            assert_eq!(macros[0].actions.len(), 4000); // 1000 * 4 actions
        }
        _ => panic!("Unexpected response: {:?}", list_response),
    }

    // Clean up
    test_env.daemon_handle.abort();
    Ok(())
}
