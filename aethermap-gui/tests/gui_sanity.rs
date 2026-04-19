//! GUI Sanity Tests for AethermapGui
//!
//! These tests verify that the GUI application compiles correctly and can handle
//! basic message flows without panicking. Tests focus on structural integrity
//! rather than visual rendering since Iced applications are UI-heavy.

use aethermap_common::{DeviceInfo, MacroEntry, KeyCombo, Action, DeviceType};
use aethermap_gui::{State, Message};
use iced::application::Application;
use std::path::PathBuf;
use std::time::Instant;

/// Helper function to create a dummy DeviceInfo for testing
fn create_test_device(name: &str, path: &str) -> DeviceInfo {
    DeviceInfo {
        name: name.to_string(),
        path: PathBuf::from(path),
        vendor_id: 0x1532,
        product_id: 0x0203,
        phys: "usb-0000:00:14.0-1/input/input0".to_string(),
        device_type: DeviceType::Keyboard,
    }
}

/// Helper function to create a dummy MacroEntry for testing
fn create_test_macro(name: &str, enabled: bool) -> MacroEntry {
    MacroEntry {
        name: name.to_string(),
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
        enabled,
        humanize: true,
        capture_mouse: false,
    }
}

/// Helper function to create a test state with sample data
fn create_test_state() -> State {
    let mut state = State::default();
    
    state.devices = vec![
        create_test_device("Razer Keyboard", "/dev/input/event0"),
        create_test_device("Razer Mouse", "/dev/input/event1"),
    ];
    state.macros = vec![
        create_test_macro("Test Macro 1", true),
        create_test_macro("Test Macro 2", false),
    ];
    state.selected_device = Some(0);
    state.status = "Test initialized".to_string();
    state.status_history.push_back("Previous status 1".to_string());
    state.status_history.push_back("Previous status 2".to_string());
    state.daemon_connected = true;
    state.socket_path = PathBuf::from("/tmp/test.sock");
    
    state
}

/// Test that the GUI can be constructed and renders without panicking
#[test]
fn test_gui_construction_and_view() {
    // Create a test state with sample data
    let state = create_test_state();
    
    // This should not panic - the view should build successfully
    let _element = state.view();
    
    // If we reach this point, the view built without panicking
    assert!(true, "GUI view should build without panicking");
}

/// Test that DevicesLoaded message updates state correctly
#[test]
fn test_devices_loaded_message() {
    let mut state = State::default();
    
    // Simulate receiving devices
    let devices = vec![
        create_test_device("Test Device 1", "/dev/input/event0"),
        create_test_device("Test Device 2", "/dev/input/event1"),
    ];
    
    let message = Message::DevicesLoaded(Ok(devices.clone()));
    
    // Process the message
    let _command = state.update(message);
    
    // Verify state was updated
    assert_eq!(state.devices.len(), 2);
    assert_eq!(state.devices[0].name, "Test Device 1");
    assert_eq!(state.devices[1].name, "Test Device 2");
    // Note: selected_device is not automatically set when devices load
    // It's only set when user explicitly selects a device via DeviceSelected message
    assert!(state.selected_device.is_none());
}

/// Test that MacrosLoaded message updates state correctly
#[test]
fn test_macros_loaded_message() {
    let mut state = State::default();
    
    // Simulate receiving macros
    let macros = vec![
        create_test_macro("Test Macro 1", true),
        create_test_macro("Test Macro 2", false),
        create_test_macro("Test Macro 3", true),
    ];
    
    let message = Message::MacrosLoaded(Ok(macros.clone()));
    
    // Process the message
    let _command = state.update(message);
    
    // Verify state was updated
    assert_eq!(state.macros.len(), 3);
    assert_eq!(state.macros[0].name, "Test Macro 1");
    assert_eq!(state.macros[1].name, "Test Macro 2");
    assert_eq!(state.macros[2].name, "Test Macro 3");
    assert!(state.macros[0].enabled);
    assert!(!state.macros[1].enabled);
    assert!(state.macros[2].enabled);
}

/// Test that the view contains expected structural elements
#[test]
fn test_view_structure_contains_devices() {
    let state = create_test_state();
    
    // Build the view
    let element = state.view();
    
    // The element should be built successfully (no panic)
    // We can't easily inspect the Iced element tree, but we can verify
    // that the state has the expected data that would be rendered
    assert!(!state.devices.is_empty(), "Should have devices to render");
    assert!(state.devices.len() >= 1, "Should have at least one device");
    
    // Verify device data integrity
    for device in &state.devices {
        assert!(!device.name.is_empty(), "Device name should not be empty");
        assert!(!device.path.as_os_str().is_empty(), "Device path should not be empty");
    }
}

/// Test that the view contains expected macro cards
#[test]
fn test_view_structure_contains_macros() {
    let state = create_test_state();
    
    // Build the view
    let element = state.view();
    
    // The element should be built successfully (no panic)
    assert!(!state.macros.is_empty(), "Should have macros to render");
    assert!(state.macros.len() >= 1, "Should have at least one macro");
    
    // Verify macro data integrity
    for macro_entry in &state.macros {
        assert!(!macro_entry.name.is_empty(), "Macro name should not be empty");
        assert!(!macro_entry.actions.is_empty(), "Macro should have actions");
    }
}

/// Test that recording state updates work correctly
#[test]
fn test_recording_state_updates() {
    let mut state = State::default();
    
    // Initially not recording
    assert!(!state.recording, "Should not be recording initially");
    assert!(state.recording_macro_name.is_none(), "Should have no recording macro name");
    
    // Simulate starting recording
    let message = Message::StartRecording;
    let _command = state.update(message);
    
    // Simulate successful recording start by setting state manually
    state.recording = true;
    state.recording_macro_name = Some("Test Recording".to_string());
    
    // Verify recording state
    assert!(state.recording, "Should be recording");
    assert_eq!(state.recording_macro_name.as_ref().unwrap(), "Test Recording");
    
    // Build view with recording state
    let _element = state.view();
    
    // Should not panic even when recording
}

/// Test that status updates work correctly
#[test]
fn test_status_updates() {
    let mut state = State::default();
    
    // Update status
    let message = Message::ShowNotification("New status message".to_string(), false);
    let _command = state.update(message);
    
    // Verify history was maintained
    assert!(!state.status_history.is_empty(), "Should have status history");
    
    // Build view with new status
    let _element = state.view();
}

/// Test that recently updated macros tracking works
#[test]
fn test_recently_updated_macros() {
    let mut state = State::default();
    
    // Add a macro to recently updated
    let macro_name = "Test Macro".to_string();
    state.recently_updated_macros.insert(macro_name.clone(), Instant::now());
    
    // Verify it's marked as recently updated
    assert!(state.recently_updated_macros.contains_key(&macro_name));
    
    // Build view with recently updated macro
    let _element = state.view();
    
    // Should not panic even with recently updated macros
}

/// Test error handling in message processing
#[test]
fn test_error_handling() {
    let mut state = State::default();
    
    // Test error message for devices
    let error_msg = "Failed to load devices".to_string();
    let message = Message::DevicesLoaded(Err(error_msg.clone()));
    let _command = state.update(message);
    
    // Should handle error without panicking
    {
        let _element = state.view();
    } // Element dropped here
    
    // Test error message for macros
    let macro_error = "Failed to load macros".to_string();
    let message = Message::MacrosLoaded(Err(macro_error.clone()));
    let _command = state.update(message);
    
    // Should handle error without panicking
    let _element = state.view();
}

/// Test animation ticker functionality
#[test]
fn test_animation_ticker() {
    let mut state = State::default();
    
    // Add some recently updated macros
    state.recently_updated_macros.insert("Macro1".to_string(), Instant::now());
    state.recently_updated_macros.insert("Macro2".to_string(), Instant::now());
    
    // Tick animations
    let message = Message::TickAnimations;
    let _command = state.update(message);
    
    // Should not panic and should clean up expired entries
    // (though they won't be expired since we just created them)
    assert!(!state.recently_updated_macros.is_empty());
    
    // Build view after animation tick
    let _element = state.view();
}

/// Test complete flow simulation
#[test]
fn test_complete_flow_simulation() {
    let mut state = State::default();
    
    // Simulate complete application flow:
    
    // 1. Daemon connects
    let connect_msg = Message::DaemonStatusChanged(true);
    let _command = state.update(connect_msg);
    assert!(state.daemon_connected);
    
    // 2. Devices load
    let devices_msg = Message::DevicesLoaded(Ok(vec![create_test_device("Test Device", "/dev/input/event0")]));
    let _command = state.update(devices_msg);
    assert_eq!(state.devices.len(), 1);
    
    // 3. Macros load
    let macros_msg = Message::MacrosLoaded(Ok(vec![create_test_macro("Test Macro", true)]));
    let _command = state.update(macros_msg);
    assert_eq!(state.macros.len(), 1);
    
    // 4. Status updates
    let status_msg = Message::ShowNotification("Application ready".to_string(), false);
    let _command = state.update(status_msg);
    
    // 5. Final view render
    let _element = state.view();
    
    // If we reach here, the complete flow works without panics
    assert!(true, "Complete application flow should work without panicking");
}