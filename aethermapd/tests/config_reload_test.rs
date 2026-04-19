//! Integration tests for configuration hot-reload validation
//!
//! This test module verifies the hot-reload behavior of ConfigManager:
//!
//! - Valid configurations are reloaded atomically (validate-then-swap)
//! - Invalid configurations are rejected without affecting active config
//! - Both reload_remaps() and reload_device_profiles() are tested
//!
//! # Test Strategy
//!
//! These tests use temporary YAML files to verify the reload pattern:
//! 1. Load an initial valid configuration
//! 2. Verify the config is active
//! 3. Attempt to reload with invalid config (should fail)
//! 4. Verify original config is still active (atomic swap)
//! 5. Reload with valid config (should succeed)
//!
//! # Validate-Then-Swap Pattern
//!
//! The hot-reload implementation must validate the entire configuration
//! before applying any changes. This ensures the daemon never runs with
//! a partially-applied or invalid configuration.

use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;

use aethermapd::config::ConfigManager;
use aethermapd::remap_engine::RemapEngine;

/// Helper to create a ConfigManager with temporary directories
///
/// This allows tests to manipulate config files without affecting
/// the system configuration.
fn create_test_config_manager(temp_dir: &TempDir) -> ConfigManager {
    ConfigManager {
        config_path: temp_dir.path().join("config.yaml"),
        macros_path: temp_dir.path().join("macros.yaml"),
        cache_path: temp_dir.path().join("macros.bin"),
        profiles_dir: temp_dir.path().join("profiles"),
        remaps_path: temp_dir.path().join("remaps.yaml"),
        device_profiles_path: temp_dir.path().join("device_profiles.yaml"),
        layer_state_path: temp_dir.path().join("layer_state.yaml"),
        config: Arc::new(tokio::sync::RwLock::new(aethermapd::config::DaemonConfig::default())),
        macros: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        profiles: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        remaps: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        device_profiles: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    }
}

/// Helper to write a remaps YAML file
fn write_remaps_file(path: &std::path::Path, content: &str) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    file.write_all(content.as_bytes())
}

/// Test valid config reload with atomic swap
///
/// Verifies that loading a valid configuration updates the remaps
/// without leaving the system in an inconsistent state.
#[test]
fn test_valid_remap_reload() {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let manager = create_test_config_manager(&temp_dir);

    // Create initial valid remaps file
    let remaps_path = temp_dir.path().join("remaps.yaml");
    let initial_config = r#"
# Initial remapping configuration
capslock: leftctrl
"#;
    write_remaps_file(&remaps_path, initial_config).unwrap();

    // Load initial config
    rt.block_on(async {
        let result = manager.load_remaps().await;
        assert!(result.is_ok(), "Initial config load should succeed");

        // Verify remaps are loaded
        let remaps = manager.remaps.read().await;
        assert_eq!(remaps.len(), 1);
        assert_eq!(remaps.get("capslock"), Some(&"leftctrl".to_string()));
    });

    // Update config with new remaps (still valid)
    let updated_config = r#"
# Updated remapping configuration
capslock: leftctrl
a: b
"#;
    write_remaps_file(&remaps_path, updated_config).unwrap();

    // Reload config
    rt.block_on(async {
        let result = manager.load_remaps().await;
        assert!(result.is_ok(), "Updated config load should succeed");

        // Verify new remaps are loaded
        let remaps = manager.remaps.read().await;
        assert_eq!(remaps.len(), 2);
        assert_eq!(remaps.get("capslock"), Some(&"leftctrl".to_string()));
        assert_eq!(remaps.get("a"), Some(&"b".to_string()));
    });
}

/// Test invalid config rejection preserves original config
///
/// Verifies the validate-then-swap pattern: when loading an invalid
/// configuration, the original configuration remains unchanged.
#[test]
fn test_invalid_remap_rejection() {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let manager = create_test_config_manager(&temp_dir);

    let remaps_path = temp_dir.path().join("remaps.yaml");

    // Create initial valid config
    let initial_config = r#"
capslock: leftctrl
a: b
"#;
    write_remaps_file(&remaps_path, initial_config).unwrap();

    rt.block_on(async {
        // Load initial config
        let result = manager.load_remaps().await;
        assert!(result.is_ok());

        // Verify initial state
        let remaps = manager.remaps.read().await;
        assert_eq!(remaps.len(), 2);
        drop(remaps);

        // Now write invalid config (bad key name)
        drop(write_remaps_file(
            &remaps_path,
            "invalid_key_xyz: leftctrl\n",
        ));

        // Attempt reload - should fail
        let result = manager.load_remaps().await;
        assert!(result.is_err(), "Invalid config should be rejected");

        // Verify original config is still active
        let remaps = manager.remaps.read().await;
        assert_eq!(remaps.len(), 2, "Original remaps should remain active");
        assert_eq!(
            remaps.get("capslock"),
            Some(&"leftctrl".to_string()),
            "Original remaps should be unchanged"
        );
    });
}

/// Test reload_remaps with RemapEngine
///
/// Verifies that reload_remaps properly updates the RemapEngine.
#[test]
fn test_reload_remaps_with_engine() {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let manager = create_test_config_manager(&temp_dir);

    let remaps_path = temp_dir.path().join("remaps.yaml");

    // Create initial config
    write_remaps_file(&remaps_path, "capslock: leftctrl\n").unwrap();

    rt.block_on(async {
        let engine = Arc::new(RemapEngine::new());

        // Load initial config
        let result = manager.reload_remaps(engine.clone()).await;
        assert!(result.is_ok(), "Initial reload should succeed");

        // Verify engine has the remapping
        assert_eq!(engine.remap_count().await, 1);

        // Update config
        write_remaps_file(&remaps_path, "capslock: leftctrl\na: b\n").unwrap();

        // Reload
        let result = manager.reload_remaps(engine.clone()).await;
        assert!(result.is_ok(), "Updated reload should succeed");

        // Verify engine has updated remappings
        assert_eq!(engine.remap_count().await, 2);
    });
}

/// Test reload_remaps rejects invalid keys
///
/// Verifies that reload_remaps validates key names and rejects
/// configurations with invalid key names.
#[test]
fn test_reload_remaps_validates_keys() {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let manager = create_test_config_manager(&temp_dir);

    let remaps_path = temp_dir.path().join("remaps.yaml");

    rt.block_on(async {
        let engine = Arc::new(RemapEngine::new());

        // Load valid initial config
        write_remaps_file(&remaps_path, "capslock: leftctrl\n").unwrap();
        let result = manager.reload_remaps(engine.clone()).await;
        assert!(result.is_ok());

        let initial_count = engine.remap_count().await;
        assert_eq!(initial_count, 1);

        // Try to load invalid config
        write_remaps_file(&remaps_path, "not_a_real_key: leftctrl\n").unwrap();
        let result = manager.reload_remaps(engine.clone()).await;

        assert!(result.is_err(), "Should reject invalid key name");

        // Verify engine still has original config
        let final_count = engine.remap_count().await;
        assert_eq!(
            final_count, initial_count,
            "Engine should keep original config on validation failure"
        );
    });
}

/// Helper to write device profiles YAML file
fn write_device_profiles_file(path: &std::path::Path, content: &str) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    file.write_all(content.as_bytes())
}

/// Test valid device profile reload
///
/// Verifies that reload_device_profiles loads valid configurations.
#[test]
fn test_valid_device_profile_reload() {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let manager = create_test_config_manager(&temp_dir);

    let profiles_path = temp_dir.path().join("device_profiles.yaml");

    // Create valid device profiles config
    let config = r#"
devices:
  "1532:0220":
    device_id: "1532:0220"
    profiles:
      gaming:
        name: "gaming"
        remaps:
          - from: capslock
            to: leftctrl
      work:
        name: "work"
        remaps:
          - from: a
            to: b
"#;
    write_device_profiles_file(&profiles_path, config).unwrap();

    rt.block_on(async {
        let result = manager.reload_device_profiles().await;
        assert!(result.is_ok(), "Valid device profiles should load successfully");

        // Verify profiles are loaded
        let devices = manager.list_profile_devices().await;
        assert_eq!(devices.len(), 1);
        assert!(devices.contains(&"1532:0220".to_string()));

        // Verify profiles for device
        let profiles = manager.list_device_profiles("1532:0220").await;
        assert_eq!(profiles.len(), 2);
        assert!(profiles.contains(&"gaming".to_string()));
        assert!(profiles.contains(&"work".to_string()));
    });
}

/// Test invalid device profile rejection
///
/// Verifies that reload_device_profiles rejects configurations
/// with invalid key names and preserves the original config.
#[test]
fn test_invalid_device_profile_rejection() {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let manager = create_test_config_manager(&temp_dir);

    let profiles_path = temp_dir.path().join("device_profiles.yaml");

    rt.block_on(async {
        // Load valid initial config
        let valid_config = r#"
devices:
  "1532:0220":
    device_id: "1532:0220"
    profiles:
      gaming:
        name: "gaming"
        remaps:
          - from: capslock
            to: leftctrl
"#;
        write_device_profiles_file(&profiles_path, valid_config).unwrap();

        let result = manager.reload_device_profiles().await;
        assert!(result.is_ok());

        // Verify initial state
        let devices = manager.list_profile_devices().await;
        assert_eq!(devices.len(), 1);
        let profiles = manager.list_device_profiles("1532:0220").await;
        assert_eq!(profiles.len(), 1);

        // Try to load invalid config (bad key name)
        let invalid_config = r#"
devices:
  "1532:0220":
    device_id: "1532:0220"
    profiles:
      bad_profile:
        name: "bad_profile"
        remaps:
          - from: invalid_key_name_xyz
            to: leftctrl
"#;
        write_device_profiles_file(&profiles_path, invalid_config).unwrap();

        let result = manager.reload_device_profiles().await;
        assert!(result.is_err(), "Should reject invalid key name");

        // Verify original config is still active
        let devices = manager.list_profile_devices().await;
        assert_eq!(
            devices.len(),
            1,
            "Original device list should remain"
        );
        let profiles = manager.list_device_profiles("1532:0220").await;
        assert_eq!(
            profiles.len(),
            1,
            "Original profiles should remain"
        );
    });
}

/// Test atomic swap for reload_device_profiles
///
/// Verifies that on validation failure, the config is not partially applied.
#[test]
fn test_device_profile_atomic_swap() {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let manager = create_test_config_manager(&temp_dir);

    let profiles_path = temp_dir.path().join("device_profiles.yaml");

    rt.block_on(async {
        // Load config with multiple devices
        let multi_device_config = r#"
devices:
  "1532:0220":
    device_id: "1532:0220"
    profiles:
      gaming:
        name: "gaming"
        remaps:
          - from: capslock
            to: leftctrl
  "046d:c52b":
    device_id: "046d:c52b"
    profiles:
      default:
        name: "default"
        remaps:
          - from: a
            to: b
"#;
        write_device_profiles_file(&profiles_path, multi_device_config).unwrap();

        let result = manager.reload_device_profiles().await;
        assert!(result.is_ok());

        // Verify both devices loaded
        let devices = manager.list_profile_devices().await;
        assert_eq!(devices.len(), 2);

        // Try to load config where one device has invalid keys
        let partial_invalid_config = r#"
devices:
  "1532:0220":
    device_id: "1532:0220"
    profiles:
      gaming:
        name: "gaming"
        remaps:
          - from: capslock
            to: leftctrl
  "046d:c52b":
    device_id: "046d:c52b"
    profiles:
      bad_profile:
        name: "bad_profile"
        remaps:
          - from: invalid_key_xyz
            to: leftctrl
"#;
        write_device_profiles_file(&profiles_path, partial_invalid_config).unwrap();

        let result = manager.reload_device_profiles().await;
        assert!(result.is_err(), "Should reject config with invalid keys");

        // Verify original config is fully intact (not partially replaced)
        let devices = manager.list_profile_devices().await;
        assert_eq!(
            devices.len(),
            2,
            "Should keep both original devices"
        );
        assert!(devices.contains(&"1532:0220".to_string()));
        assert!(devices.contains(&"046d:c52b".to_string()));
    });
}

/// Test empty config handling during reload
///
/// Verifies that reloading with an empty config clears the
/// configuration without errors.
#[test]
fn test_empty_remap_reload() {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let manager = create_test_config_manager(&temp_dir);

    let remaps_path = temp_dir.path().join("remaps.yaml");

    rt.block_on(async {
        // Load initial config
        write_remaps_file(&remaps_path, "capslock: leftctrl\n").unwrap();
        let result = manager.load_remaps().await;
        assert!(result.is_ok());

        let remaps = manager.remaps.read().await;
        assert_eq!(remaps.len(), 1);
        drop(remaps);

        // Reload with empty config
        write_remaps_file(&remaps_path, "{}\n").unwrap();
        let result = manager.load_remaps().await;
        assert!(result.is_ok(), "Empty config should be valid");

        // Verify config is cleared
        let remaps = manager.remaps.read().await;
        assert_eq!(remaps.len(), 0, "Empty config should clear remaps");
    });
}

/// Test concurrent config reload safety
///
/// Verifies that multiple concurrent reload operations don't
/// cause race conditions (basic smoke test).
#[test]
fn test_concurrent_reload_safety() {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let manager = Arc::new(create_test_config_manager(&temp_dir));

    let remaps_path = temp_dir.path().join("remaps.yaml");

    rt.block_on(async {
        // Write valid config
        write_remaps_file(&remaps_path, "capslock: leftctrl\na: b\n").unwrap();

        // Spawn multiple concurrent reloads
        let manager1 = manager.clone();
        let manager2 = manager.clone();
        let manager3 = manager.clone();

        let task1 = tokio::spawn(async move {
            manager1.load_remaps().await
        });
        let task2 = tokio::spawn(async move {
            manager2.load_remaps().await
        });
        let task3 = tokio::spawn(async move {
            manager3.load_remaps().await
        });

        // All should succeed
        let results = vec![
            task1.await.unwrap(),
            task2.await.unwrap(),
            task3.await.unwrap(),
        ];

        for result in results {
            assert!(result.is_ok(), "Concurrent reloads should succeed");
        }

        // Final state should be consistent
        let remaps = manager.remaps.read().await;
        assert_eq!(remaps.len(), 2);
    });
}
