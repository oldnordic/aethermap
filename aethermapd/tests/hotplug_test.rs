//! Integration tests for device hotplug event handling
//!
//! This test module verifies the behavior of the hotplug module's DeviceEvent
//! and DeviceMonitor types. Tests focus on:
//!
//! - DeviceEvent enum structure and parsing
//! - Device ID formatting (VID:PID hexadecimal format)
//! - Event handling through the public API
//!
//! # Test Strategy
//!
//! These are unit-level integration tests that verify the public API contracts
//! of the hotplug module without requiring actual hardware devices. The tests
//! use the types directly from the hotplug module.
//!
//! # Mock vs Real
//!
//! Full udev mock testing would require significant infrastructure. These tests
//! focus on verifying the event structure and device ID formatting that are
//! critical for correct hotplug behavior.

use aethermapd::hotplug::{DeviceEvent, format_device_id};

/// Test that format_device_id produces correct vendor:product format
///
/// Verifies:
/// - Lowercase hexadecimal output
/// - 4-digit zero-padding for both vendor and product IDs
/// - Correct separator (:)
#[test]
fn test_format_device_id() {
    // Test Razer device (common case)
    assert_eq!(format_device_id(0x1532, 0x0220), "1532:0220");

    // Test lowercase output
    assert_eq!(format_device_id(0xABCD, 0x1234), "abcd:1234");

    // Test zero padding
    assert_eq!(format_device_id(0x0001, 0x0002), "0001:0002");
    assert_eq!(format_device_id(0x1, 0x2), "0001:0002");

    // Test Logitech device (different vendor)
    assert_eq!(format_device_id(0x046D, 0xC52B), "046d:c52b");
}

/// Test DeviceEvent::Add variant structure
///
/// Verifies:
/// - Event can be constructed with devnode and device_id
/// - Pattern matching extracts fields correctly
/// - Add events have the expected structure
#[test]
fn test_device_event_add() {
    let event = DeviceEvent::Add {
        devnode: "/dev/input/event0".to_string(),
        device_id: "1532:0220".to_string(),
    };

    // Verify pattern matching works
    match &event {
        DeviceEvent::Add { devnode, device_id } => {
            assert_eq!(devnode, "/dev/input/event0");
            assert_eq!(device_id, "1532:0220");
        }
        _ => panic!("Expected Add event"),
    }

    // Verify Clone works (needed for event handling)
    let event_clone = event.clone();
    match &event_clone {
        DeviceEvent::Add { device_id, .. } => {
            assert_eq!(device_id, "1532:0220");
        }
        _ => panic!("Cloned event should be Add"),
    }
}

/// Test DeviceEvent::Remove variant structure
///
/// Verifies:
/// - Event can be constructed with devnode and device_id
/// - Pattern matching extracts fields correctly
/// - Remove events have the expected structure
#[test]
fn test_device_event_remove() {
    let event = DeviceEvent::Remove {
        devnode: "/dev/input/event1".to_string(),
        device_id: "046d:c52b".to_string(),
    };

    // Verify pattern matching works
    match &event {
        DeviceEvent::Remove { devnode, device_id } => {
            assert_eq!(devnode, "/dev/input/event1");
            assert_eq!(device_id, "046d:c52b");
        }
        _ => panic!("Expected Remove event"),
    }

    // Verify Clone works (needed for event handling)
    let event_clone = event.clone();
    match &event_clone {
        DeviceEvent::Remove { device_id, .. } => {
            assert_eq!(device_id, "046d:c52b");
        }
        _ => panic!("Cloned event should be Remove"),
    }
}

/// Test device_id format matches config expectations
///
/// The device_id format must match the YAML configuration format
/// where devices are keyed by "vendor:product" hex strings.
#[test]
fn test_device_id_format_consistency() {
    // These formats should match what users write in device_profiles.yaml
    let test_cases = vec![
        ((0x1532, 0x0220), "1532:0220"),
        ((0x046D, 0xC52B), "046d:c52b"),
        ((0x1234, 0x5678), "1234:5678"),
    ];

    for ((vid, pid), expected) in test_cases {
        let formatted = format_device_id(vid, pid);
        assert_eq!(
            formatted, expected,
            "Device ID format mismatch for VID:0x{:04x} PID:0x{:04x}",
            vid, pid
        );
    }
}

/// Test that DeviceEvent carries sufficient information
///
/// For hotplug handling, we need:
/// - devnode: to open/grab the device
/// - device_id: to look up configured profiles
#[test]
fn test_device_event_completeness() {
    let add_event = DeviceEvent::Add {
        devnode: "/dev/input/event5".to_string(),
        device_id: "1532:0220".to_string(),
    };

    let remove_event = DeviceEvent::Remove {
        devnode: "/dev/input/event5".to_string(),
        device_id: "1532:0220".to_string(),
    };

    // Verify Add event has both required fields
    match &add_event {
        DeviceEvent::Add { devnode, device_id } => {
            assert!(!devnode.is_empty(), "devnode must not be empty");
            assert!(!device_id.is_empty(), "device_id must not be empty");
            assert!(devnode.starts_with("/dev/"), "devnode must be absolute path");
            assert!(device_id.contains(":"), "device_id must contain separator");
        }
        _ => panic!("Expected Add event"),
    }

    // Verify Remove event has both required fields
    match &remove_event {
        DeviceEvent::Remove { devnode, device_id } => {
            assert!(!devnode.is_empty(), "devnode must not be empty");
            assert!(!device_id.is_empty(), "device_id must not be empty");
            assert!(devnode.starts_with("/dev/"), "devnode must be absolute path");
            assert!(device_id.contains(":"), "device_id must contain separator");
        }
        _ => panic!("Expected Remove event"),
    }
}

/// Test DeviceEvent Debug representation
///
/// Debug output is useful for logging during hotplug events.
#[test]
fn test_device_event_debug() {
    let event = DeviceEvent::Add {
        devnode: "/dev/input/event0".to_string(),
        device_id: "1532:0220".to_string(),
    };

    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("Add"), "Debug output should show event type");
    assert!(debug_str.contains("/dev/input/event0"), "Debug output should show devnode");
    assert!(debug_str.contains("1532:0220"), "Debug output should show device_id");
}
