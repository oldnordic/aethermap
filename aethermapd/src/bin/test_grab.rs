//! Simple CLI tool to test device grabbing and event reading
//! Usage: cargo run --bin test_grab -- /dev/input/eventX

use aethermap_common::tracing;
use aethermapd::device::DeviceManager;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <device_path>", args[0]);
        eprintln!("Example: {} /dev/input/event3", args[0]);
        eprintln!("\nThis tool will:");
        eprintln!("  1. Discover all input devices");
        eprintln!("  2. Grab the specified device exclusively (EVIOCGRAB)");
        eprintln!("  3. Print all key events from that device");
        eprintln!("  4. Press Ctrl+C to exit and ungrab");
        std::process::exit(1);
    }

    let device_path = &args[1];
    info!("Testing device grab for: {}", device_path);

    // Check if running as root
    if !nix::unistd::getuid().is_root() {
        error!("This tool must be run as root for device access");
        error!("Try: sudo cargo run --bin test_grab -- {}", device_path);
        std::process::exit(1);
    }

    // Create device manager
    let mut device_manager = DeviceManager::new();

    // Discover devices
    info!("Discovering devices...");
    device_manager.start_discovery().await?;

    let devices = device_manager.get_devices();
    info!("Found {} devices:", devices.len());
    for device in &devices {
        info!("  - {} at {} (VID:{:04x} PID:{:04x})",
              device.name, device.path.display(), device.vendor_id, device.product_id);
    }

    // Check if target device exists
    if !devices.iter().any(|d| d.path.to_string_lossy() == device_path.as_str()) {
        error!("Device {} not found in discovered devices", device_path);
        error!("Available devices:");
        for device in &devices {
            error!("  {}", device.path.display());
        }
        std::process::exit(1);
    }

    // Get event receiver before grabbing
    let mut event_receiver = device_manager.get_event_receiver();

    // Grab the device
    info!("Grabbing device {}...", device_path);
    device_manager.grab_device(device_path).await?;
    info!("Device grabbed successfully! Events from this device are now intercepted.");
    info!("Press keys on the device - they will appear here but NOT in other applications.");
    info!("Press Ctrl+C to exit and release the device.");

    // Handle shutdown signal
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    // Event loop
    loop {
        tokio::select! {
            Some(msg) = event_receiver.recv() => {
                use aethermapd::device::DeviceEventType;
                match msg.event_type {
                    DeviceEventType::Key { original_code, value } => {
                        let pressed = value == 1;
                        let is_repeat = value == 2;
                        let action = if is_repeat { "REPEAT" } else if pressed { "PRESSED" } else { "RELEASED" };
                        let key_name = key_code_to_name(original_code);
                        info!("[{}] Key {} ({}) {}", msg.device_path, original_code, key_name, action);
                    }
                    DeviceEventType::RelAxis { axis, value } => {
                        info!("[{}] RelAxis {:?} = {}", msg.device_path, axis, value);
                    }
                }
            }
            _ = &mut shutdown => {
                info!("Received Ctrl+C, cleaning up...");
                break;
            }
        }
    }

    // Ungrab and cleanup
    info!("Ungrabbing device...");
    device_manager.ungrab_device(device_path).await?;
    info!("Device released. Test complete!");

    Ok(())
}

/// Convert key code to human-readable name
fn key_code_to_name(code: u16) -> &'static str {
    match code {
        1 => "ESC",
        2 => "1",
        3 => "2",
        4 => "3",
        5 => "4",
        6 => "5",
        7 => "6",
        8 => "7",
        9 => "8",
        10 => "9",
        11 => "0",
        12 => "MINUS",
        13 => "EQUAL",
        14 => "BACKSPACE",
        15 => "TAB",
        16 => "Q",
        17 => "W",
        18 => "E",
        19 => "R",
        20 => "T",
        21 => "Y",
        22 => "U",
        23 => "I",
        24 => "O",
        25 => "P",
        26 => "LEFTBRACE",
        27 => "RIGHTBRACE",
        28 => "ENTER",
        29 => "LEFTCTRL",
        30 => "A",
        31 => "S",
        32 => "D",
        33 => "F",
        34 => "G",
        35 => "H",
        36 => "J",
        37 => "K",
        38 => "L",
        39 => "SEMICOLON",
        40 => "APOSTROPHE",
        41 => "GRAVE",
        42 => "LEFTSHIFT",
        43 => "BACKSLASH",
        44 => "Z",
        45 => "X",
        46 => "C",
        47 => "V",
        48 => "B",
        49 => "N",
        50 => "M",
        51 => "COMMA",
        52 => "DOT",
        53 => "SLASH",
        54 => "RIGHTSHIFT",
        55 => "KPASTERISK",
        56 => "LEFTALT",
        57 => "SPACE",
        58 => "CAPSLOCK",
        59..=68 => "F1-F10",
        87 => "F11",
        88 => "F12",
        96 => "KPENTER",
        97 => "RIGHTCTRL",
        100 => "RIGHTALT",
        102 => "HOME",
        103 => "UP",
        104 => "PAGEUP",
        105 => "LEFT",
        106 => "RIGHT",
        107 => "END",
        108 => "DOWN",
        109 => "PAGEDOWN",
        110 => "INSERT",
        111 => "DELETE",
        125 => "LEFTMETA",
        126 => "RIGHTMETA",
        _ => "UNKNOWN",
    }
}
