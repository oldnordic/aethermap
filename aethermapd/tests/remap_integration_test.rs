//! End-to-end integration tests for key remapping with virtual devices
//!
//! This test module uses evdev's VirtualDeviceBuilder to create virtual input
//! devices for testing remapping without physical hardware.

use std::collections::HashMap;
use std::sync::Arc;

use evdev::AttributeSet;
use evdev::Key;
use evdev::EventType;
use evdev::InputEvent;

use aethermapd::remap_engine::RemapEngine;

/// Helper macro to create an AttributeSet<Key> from a list of keys
///
/// This macro creates a new AttributeSet, inserts each key, and returns the set.
macro_rules! key_set {
    ($($key:expr),* $(,)?) => {{
        let mut set = AttributeSet::new();
        $(set.insert($key);)*
        set
    }};
}

/// Test basic end-to-end key remapping with virtual devices
///
/// This test:
/// 1. Creates a virtual source device (simulates physical keyboard)
/// 2. Creates a virtual sink device (captures remapped output)
/// 3. Sets up RemapEngine with a test configuration
/// 4. Verifies RemapEngine correctly remaps keys
/// 5. Verifies unmapped keys pass through unchanged
///
/// Note: This test creates virtual devices but uses RemapEngine.process_event()
/// directly rather than injecting into the virtual device, as reading from
/// virtual devices would require a more complex async event loop.
#[tokio::test]
async fn test_end_to_end_key_remapping() {
    // Skip if /dev/uinput not available (CI compatibility)
    if !std::path::Path::new("/dev/uinput").exists() {
        println!("Skipping: /dev/uinput not available");
        return;
    }

    // Import VirtualDeviceBuilder from evdev::uinput
    use evdev::uinput::VirtualDeviceBuilder;

    // Create virtual source device (simulates physical keyboard)
    let mut source_device = VirtualDeviceBuilder::new()
        .expect("Failed to create VirtualDeviceBuilder")
        .name(&b"Test Source Keyboard"[..])
        .with_keys(&key_set![Key::KEY_A, Key::KEY_B, Key::KEY_C])
        .expect("Failed to set key capabilities")
        .build()
        .expect("Failed to create source device");

    // Create virtual sink device (captures remapped output)
    let mut sink_device = VirtualDeviceBuilder::new()
        .expect("Failed to create VirtualDeviceBuilder")
        .name(&b"Test Sink Keyboard"[..])
        .with_keys(&key_set![Key::KEY_A, Key::KEY_B, Key::KEY_C])
        .expect("Failed to set key capabilities")
        .build()
        .expect("Failed to create sink device");

    // Verify devices were created successfully
    // Virtual devices are created successfully if we got here without panicking
    // The devices will be automatically cleaned up when dropped

    // Setup RemapEngine with test configuration
    let engine = Arc::new(RemapEngine::new());

    let mut config = HashMap::new();
    config.insert("KEY_A".to_string(), "KEY_B".to_string());
    engine.load_config(&config).await.expect("Config load failed");

    // Test 1: Verify KEY_A press remaps to KEY_B
    let result = engine.process_event(Key::KEY_A, 1).await;
    assert_eq!(result, Some((Key::KEY_B, 1)), "KEY_A press should remap to KEY_B");

    // Test 2: Verify KEY_A release also remaps to KEY_B release
    let result = engine.process_event(Key::KEY_A, 0).await;
    assert_eq!(result, Some((Key::KEY_B, 0)), "KEY_A release should remap to KEY_B release");

    // Test 3: Verify unmapped keys return None (pass through unchanged)
    let result = engine.process_event(Key::KEY_C, 1).await;
    assert_eq!(result, None, "Unmapped KEY_C should return None");

    // Test 4: Emit events to virtual devices to verify they work
    // This exercises the virtual device API even though we use RemapEngine directly
    source_device
        .emit(&[InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1)])
        .expect("Failed to emit event to source device");

    sink_device
        .emit(&[InputEvent::new(EventType::KEY, Key::KEY_B.code(), 1)])
        .expect("Failed to emit event to sink device");
}

/// Test key repeat event handling (event value 2)
///
/// This test verifies that repeat events (value=2) are preserved
/// correctly through the remapping process. Repeat events are generated
/// when a key is held down and should produce repeated output.
#[tokio::test]
async fn test_end_to_end_repeat_events() {
    // Skip if /dev/uinput not available (CI compatibility)
    if !std::path::Path::new("/dev/uinput").exists() {
        println!("Skipping: /dev/uinput not available");
        return;
    }

    let engine = Arc::new(RemapEngine::new());

    let mut config = HashMap::new();
    config.insert("capslock".to_string(), "leftctrl".to_string());
    engine
        .load_config(&config)
        .await
        .expect("Config load failed");

    // Test 1: Press event (value=1)
    let result = engine.process_event(Key::KEY_CAPSLOCK, 1).await;
    assert_eq!(
        result,
        Some((Key::KEY_LEFTCTRL, 1)),
        "Press event should remap correctly"
    );

    // Test 2: Release event (value=0)
    let result = engine.process_event(Key::KEY_CAPSLOCK, 0).await;
    assert_eq!(
        result,
        Some((Key::KEY_LEFTCTRL, 0)),
        "Release event should remap correctly"
    );

    // Test 3: Repeat event (value=2) - this is the critical test
    let result = engine.process_event(Key::KEY_CAPSLOCK, 2).await;
    assert_eq!(
        result,
        Some((Key::KEY_LEFTCTRL, 2)),
        "Repeat event value should be preserved"
    );
}

/// Test complex remapping with multiple concurrent remappings
///
/// This test verifies that multiple remappings can be active simultaneously
/// and that each one works correctly.
#[tokio::test]
async fn test_end_to_end_complex_remapping() {
    // Skip if /dev/uinput not available (CI compatibility)
    if !std::path::Path::new("/dev/uinput").exists() {
        println!("Skipping: /dev/uinput not available");
        return;
    }

    let engine = Arc::new(RemapEngine::new());

    // Test multiple concurrent remappings
    let mut config = HashMap::new();
    config.insert("a".to_string(), "b".to_string());
    config.insert("capslock".to_string(), "leftctrl".to_string());
    config.insert("esc".to_string(), "grave".to_string());
    engine
        .load_config(&config)
        .await
        .expect("Config load failed");

    // Verify all remappings work with press events
    assert_eq!(
        engine.process_event(Key::KEY_A, 1).await,
        Some((Key::KEY_B, 1)),
        "a -> b should work"
    );
    assert_eq!(
        engine.process_event(Key::KEY_CAPSLOCK, 1).await,
        Some((Key::KEY_LEFTCTRL, 1)),
        "capslock -> leftctrl should work"
    );
    assert_eq!(
        engine.process_event(Key::KEY_ESC, 1).await,
        Some((Key::KEY_GRAVE, 1)),
        "esc -> grave should work"
    );

    // Verify all remappings work with release events
    assert_eq!(
        engine.process_event(Key::KEY_A, 0).await,
        Some((Key::KEY_B, 0)),
        "a release should remap correctly"
    );
    assert_eq!(
        engine.process_event(Key::KEY_CAPSLOCK, 0).await,
        Some((Key::KEY_LEFTCTRL, 0)),
        "capslock release should remap correctly"
    );
    assert_eq!(
        engine.process_event(Key::KEY_ESC, 0).await,
        Some((Key::KEY_GRAVE, 0)),
        "esc release should remap correctly"
    );

    // Verify all remappings work with repeat events
    assert_eq!(
        engine.process_event(Key::KEY_A, 2).await,
        Some((Key::KEY_B, 2)),
        "a repeat should remap correctly"
    );
    assert_eq!(
        engine.process_event(Key::KEY_CAPSLOCK, 2).await,
        Some((Key::KEY_LEFTCTRL, 2)),
        "capslock repeat should remap correctly"
    );
    assert_eq!(
        engine.process_event(Key::KEY_ESC, 2).await,
        Some((Key::KEY_GRAVE, 2)),
        "esc repeat should remap correctly"
    );

    // Verify unmapped keys still pass through
    assert_eq!(
        engine.process_event(Key::KEY_Z, 1).await,
        None,
        "Unmapped keys should return None"
    );
}

/// Test virtual device creation with full keyboard support
///
/// This test creates a more complete virtual keyboard with many keys
/// to verify the virtual device API works correctly.
#[tokio::test]
async fn test_virtual_device_full_keyboard() {
    // Skip if /dev/uinput not available (CI compatibility)
    if !std::path::Path::new("/dev/uinput").exists() {
        println!("Skipping: /dev/uinput not available");
        return;
    }

    use evdev::uinput::VirtualDeviceBuilder;

    // Create a more complete key set
    let keys = key_set![
        // Letters
        Key::KEY_A, Key::KEY_B, Key::KEY_C, Key::KEY_D, Key::KEY_E, Key::KEY_F, Key::KEY_G,
        Key::KEY_H, Key::KEY_I, Key::KEY_J, Key::KEY_K, Key::KEY_L, Key::KEY_M, Key::KEY_N,
        Key::KEY_O, Key::KEY_P, Key::KEY_Q, Key::KEY_R, Key::KEY_S, Key::KEY_T, Key::KEY_U,
        Key::KEY_V, Key::KEY_W, Key::KEY_X, Key::KEY_Y, Key::KEY_Z,
        // Numbers
        Key::KEY_1, Key::KEY_2, Key::KEY_3, Key::KEY_4, Key::KEY_5, Key::KEY_6, Key::KEY_7,
        Key::KEY_8, Key::KEY_9, Key::KEY_0,
        // Modifiers
        Key::KEY_LEFTCTRL, Key::KEY_LEFTSHIFT, Key::KEY_LEFTALT, Key::KEY_LEFTMETA,
        Key::KEY_RIGHTCTRL, Key::KEY_RIGHTSHIFT, Key::KEY_RIGHTALT, Key::KEY_RIGHTMETA,
        // Special keys
        Key::KEY_ENTER, Key::KEY_SPACE, Key::KEY_TAB, Key::KEY_BACKSPACE, Key::KEY_ESC,
        // Lock keys
        Key::KEY_CAPSLOCK, Key::KEY_NUMLOCK, Key::KEY_SCROLLLOCK,
        // Function keys
        Key::KEY_F1, Key::KEY_F2, Key::KEY_F3, Key::KEY_F4, Key::KEY_F5, Key::KEY_F6,
        Key::KEY_F7, Key::KEY_F8, Key::KEY_F9, Key::KEY_F10, Key::KEY_F11, Key::KEY_F12,
    ];

    let mut device = VirtualDeviceBuilder::new()
        .expect("Failed to create VirtualDeviceBuilder")
        .name(&b"Full Test Keyboard"[..])
        .with_keys(&keys)
        .expect("Failed to set key capabilities")
        .build()
        .expect("Failed to create device");

    // Verify device was created successfully
    // The virtual device is created successfully if we got here without panicking
    // The device will be automatically cleaned up when dropped

    // Test emitting events
    device
        .emit(&[InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1)])
        .expect("Failed to emit KEY_A press");
    device
        .emit(&[InputEvent::new(EventType::KEY, Key::KEY_A.code(), 0)])
        .expect("Failed to emit KEY_A release");

    // Test modifier keys
    device
        .emit(&[InputEvent::new(EventType::KEY, Key::KEY_LEFTCTRL.code(), 1)])
        .expect("Failed to emit LeftCtrl press");
    device
        .emit(&[InputEvent::new(EventType::KEY, Key::KEY_LEFTCTRL.code(), 0)])
        .expect("Failed to emit LeftCtrl release");
}
