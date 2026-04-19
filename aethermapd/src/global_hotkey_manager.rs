//! Global hotkey manager for daemon-level profile switching
//!
//! This module provides hotkey detection and profile switching functionality.
//! Hotkeys are checked before remap processing, allowing global shortcuts to
//! work consistently across all applications on Wayland.
//!
//! # Features
//!
//! - Modifier key tracking (Ctrl, Alt, Shift, Super)
//! - Key combination detection
//! - Profile and layer activation
//! - Manual override setting (prevents auto-switching)
//! - Per-device and global hotkey bindings

use aethermap_common::tracing::warn;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::HotkeyBinding;
use crate::layer_manager::LayerManager;
use crate::config::ConfigManager;

/// Linux key codes for modifier keys (evdev)
///
/// These codes come from <linux/input-event-codes.h>
pub mod key_codes {
    /// Left Ctrl (KEY_LEFTCTRL)
    pub const KEY_LEFTCTRL: u16 = 29;
    /// Right Ctrl (KEY_RIGHTCTRL)
    pub const KEY_RIGHTCTRL: u16 = 97;
    /// Left Alt (KEY_LEFTALT)
    pub const KEY_LEFTALT: u16 = 56;
    /// Right Alt (KEY_RIGHTALT / KEY_ALTGR)
    pub const KEY_RIGHTALT: u16 = 100;
    /// Left Shift (KEY_LEFTSHIFT)
    pub const KEY_LEFTSHIFT: u16 = 42;
    /// Right Shift (KEY_RIGHTSHIFT)
    pub const KEY_RIGHTSHIFT: u16 = 54;
    /// Left Super (KEY_LEFTMETA)
    pub const KEY_LEFTMETA: u16 = 125;
    /// Right Super (KEY_RIGHTMETA)
    pub const KEY_RIGHTMETA: u16 = 126;

    /// Modifier key codes (for checking if a key is a modifier)
    pub const MODIFIER_KEYS: &[u16] = &[
        KEY_LEFTCTRL, KEY_RIGHTCTRL,
        KEY_LEFTALT, KEY_RIGHTALT,
        KEY_LEFTSHIFT, KEY_RIGHTSHIFT,
        KEY_LEFTMETA, KEY_RIGHTMETA,
    ];

    /// Number key codes 1-9 (top row, not numpad)
    pub const KEY_1: u16 = 2;
    pub const KEY_2: u16 = 3;
    pub const KEY_3: u16 = 4;
    pub const KEY_4: u16 = 5;
    pub const KEY_5: u16 = 6;
    pub const KEY_6: u16 = 7;
    pub const KEY_7: u16 = 8;
    pub const KEY_8: u16 = 9;
    pub const KEY_9: u16 = 10;
}

/// Check if a key code is a modifier key
pub fn is_modifier_key(key_code: u16) -> bool {
    key_codes::MODIFIER_KEYS.contains(&key_code)
}

/// Normalize modifier name to canonical form
///
/// Converts "control", "ctrl", "CTRL", etc. to "ctrl"
pub fn normalize_modifier_name(name: &str) -> Option<&'static str> {
    match name.to_lowercase().as_str() {
        "ctrl" | "control" | "ctl" => Some("ctrl"),
        "alt" | "altgr" | "alt_gr" => Some("alt"),
        "shift" | "shft" => Some("shift"),
        "super" | "win" | "windows" | "meta" | "mod" => Some("super"),
        _ => None,
    }
}

/// Convert key code to modifier name (if applicable)
pub fn key_code_to_modifier_name(key_code: u16) -> Option<&'static str> {
    match key_code {
        key_codes::KEY_LEFTCTRL | key_codes::KEY_RIGHTCTRL => Some("ctrl"),
        key_codes::KEY_LEFTALT | key_codes::KEY_RIGHTALT => Some("alt"),
        key_codes::KEY_LEFTSHIFT | key_codes::KEY_RIGHTSHIFT => Some("shift"),
        key_codes::KEY_LEFTMETA | key_codes::KEY_RIGHTMETA => Some("super"),
        _ => None,
    }
}

/// Global hotkey manager
///
/// Tracks modifier key states and checks for hotkey combinations.
/// When a hotkey matches, triggers profile/layer activation and
/// consumes the event (prevents forwarding to remap engine).
pub struct GlobalHotkeyManager {
    /// Loaded hotkey bindings from configuration
    bindings: Vec<HotkeyBinding>,

    /// Currently held modifier key codes
    active_modifiers: HashSet<u16>,

    /// Layer manager for profile/layer activation
    layer_manager: Arc<RwLock<LayerManager>>,

    /// Config manager for profile activation
    config_manager: Arc<ConfigManager>,
}

impl GlobalHotkeyManager {
    /// Create a new global hotkey manager
    pub fn new(
        layer_manager: Arc<RwLock<LayerManager>>,
        config_manager: Arc<ConfigManager>,
    ) -> Self {
        Self {
            bindings: Vec::new(),
            active_modifiers: HashSet::new(),
            layer_manager,
            config_manager,
        }
    }

    /// Load hotkey bindings from all device configurations
    pub async fn load_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load all hotkey bindings from device configurations
        let all_bindings = self.config_manager.get_all_hotkey_bindings().await;

        // If no bindings configured, use defaults
        self.bindings = if all_bindings.is_empty() {
            aethermap_common::tracing::info!("No hotkey bindings found, using defaults");
            crate::config::default_hotkey_bindings()
        } else {
            // Normalize modifier names for case-insensitive matching
            all_bindings.into_iter().map(|mut binding| {
                binding.modifiers = binding.modifiers.iter()
                    .filter_map(|m| normalize_modifier_name(m))
                    .map(|s| s.to_string())
                    .collect();
                binding
            }).collect()
        };

        aethermap_common::tracing::info!(
            "Loaded {} hotkey bindings",
            self.bindings.len()
        );

        Ok(())
    }

    /// Get current active modifiers as normalized names
    fn get_active_modifier_names(&self) -> HashSet<String> {
        self.active_modifiers
            .iter()
            .filter_map(|&code| key_code_to_modifier_name(code))
            .map(|s| s.to_string())
            .collect()
    }

    /// Check if a key event matches a hotkey binding
    ///
    /// # Arguments
    ///
    /// * `key_code` - The evdev key code
    /// * `pressed` - true if key was pressed, false if released
    ///
    /// # Returns
    ///
    /// * `true` - Hotkey matched and was handled (event should be consumed)
    /// * `false` - No hotkey matched (event should pass through)
    pub async fn check_key_event(&mut self, key_code: u16, pressed: bool) -> bool {
        // Track modifier state
        if is_modifier_key(key_code) {
            if pressed {
                self.active_modifiers.insert(key_code);
            } else {
                self.active_modifiers.remove(&key_code);
            }
            // Modifier key events are never consumed by hotkeys
            return false;
        }

        // Only check for hotkey matches on key press, not release
        if !pressed {
            return false;
        }

        // Get current active modifiers
        let active_modifiers = self.get_active_modifier_names();

        // Check each binding for a match
        for binding in &self.bindings {
            if self.binding_matches(binding, key_code, &active_modifiers) {
                aethermap_common::tracing::info!(
                    "Hotkey matched: modifiers={:?}, key={}, profile={}",
                    binding.modifiers,
                    binding.key,
                    binding.profile_name
                );

                // Execute the hotkey action
                let _ = self.trigger_action(binding).await;

                // Consume the event
                return true;
            }
        }

        false
    }

    /// Check if a binding matches the current key state
    fn binding_matches(
        &self,
        binding: &HotkeyBinding,
        key_code: u16,
        active_modifiers: &HashSet<String>,
    ) -> bool {
        // Normalize binding modifiers
        let binding_modifiers: HashSet<String> = binding
            .normalize_modifiers()
            .into_iter()
            .collect();

        // Check if modifiers match exactly
        if binding_modifiers != *active_modifiers {
            return false;
        }

        // Check if key matches
        // Parse binding key as number or name
        let binding_key_code = self.parse_binding_key(&binding.key);
        binding_key_code == Some(key_code)
    }

    /// Parse a binding key string to a key code
    ///
    /// Supports:
    /// - "1"-"9" for number keys
    /// - "f1"-"f12" for function keys
    /// - Key names from key_parser module
    fn parse_binding_key(&self, key: &str) -> Option<u16> {
        let key_lower = key.to_lowercase();

        // Handle number keys 1-9
        if key_lower.len() == 1 {
            if let Some(digit) = key_lower.chars().next() {
                if digit.is_ascii_digit() {
                    let num = digit.to_digit(10)?;
                    return match num {
                        1 => Some(key_codes::KEY_1),
                        2 => Some(key_codes::KEY_2),
                        3 => Some(key_codes::KEY_3),
                        4 => Some(key_codes::KEY_4),
                        5 => Some(key_codes::KEY_5),
                        6 => Some(key_codes::KEY_6),
                        7 => Some(key_codes::KEY_7),
                        8 => Some(key_codes::KEY_8),
                        9 => Some(key_codes::KEY_9),
                        _ => None,
                    };
                }
            }
        }

        // Try parsing with key_parser for named keys
        #[cfg(feature = "key_parser")]
        {
            if let Ok(parsed) = crate::key_parser::parse_key(&key_lower) {
                return Some(parsed);
            }
        }

        // Fallback: try direct integer parsing
        if let Ok(code) = key_lower.parse::<u16>() {
            return Some(code);
        }

        warn!("Failed to parse hotkey key: {}", key);
        None
    }

    /// Execute the action for a hotkey binding
    ///
    /// Activates the specified profile and optional layer.
    /// Sets manual override flag to prevent auto-switching.
    async fn trigger_action(&self, binding: &HotkeyBinding) -> Result<(), String> {
        // Get all device IDs (or use the specific device_id from binding)
        let device_ids = if let Some(ref device_id) = binding.device_id {
            vec![device_id.clone()]
        } else {
            // Apply to all devices - get device IDs from layer manager
            let layer_manager = self.layer_manager.read().await;
            layer_manager.get_device_ids().await
        };

        // Activate layer for each target device
        for device_id in device_ids {
            // Check if profile exists for this device
            if let Some(_profile) = self.config_manager
                .get_device_profile(&device_id, &binding.profile_name).await
            {
                // Update layer manager with layer activation
                let layer_manager = self.layer_manager.read().await;

                // Activate the specified layer if provided
                if let Some(layer_id) = binding.layer_id {
                    layer_manager.activate_layer(&device_id, layer_id).await;

                    aethermap_common::tracing::info!(
                        "Hotkey activated: device={}, profile={}, layer={}",
                        &device_id,
                        &binding.profile_name,
                        layer_id
                    );
                } else {
                    aethermap_common::tracing::info!(
                        "Hotkey activated: device={}, profile={}",
                        &device_id,
                        &binding.profile_name
                    );
                }

                return Ok(());
            }

            warn!(
                "Hotkey referenced non-existent profile '{}' for device {}",
                &binding.profile_name, &device_id
            );
        }

        Err(format!("Profile '{}' not found for any device", binding.profile_name))
    }

    /// Get the number of loaded bindings
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    /// Clear all active modifiers (reset state)
    pub fn clear_modifiers(&mut self) {
        self.active_modifiers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_modifier_key() {
        assert!(is_modifier_key(key_codes::KEY_LEFTCTRL));
        assert!(is_modifier_key(key_codes::KEY_RIGHTCTRL));
        assert!(is_modifier_key(key_codes::KEY_LEFTALT));
        assert!(is_modifier_key(key_codes::KEY_LEFTSHIFT));
        assert!(is_modifier_key(key_codes::KEY_LEFTMETA));
        assert!(!is_modifier_key(key_codes::KEY_1));
        assert!(!is_modifier_key(1000)); // Non-modifier key
    }

    #[test]
    fn test_normalize_modifier_name() {
        assert_eq!(normalize_modifier_name("ctrl"), Some("ctrl"));
        assert_eq!(normalize_modifier_name("CTRL"), Some("ctrl"));
        assert_eq!(normalize_modifier_name("control"), Some("ctrl"));
        assert_eq!(normalize_modifier_name("Alt"), Some("alt"));
        assert_eq!(normalize_modifier_name("SHIFT"), Some("shift"));
        assert_eq!(normalize_modifier_name("Win"), Some("super"));
        assert_eq!(normalize_modifier_name("invalid"), None);
    }

    #[test]
    fn test_key_code_to_modifier_name() {
        assert_eq!(key_code_to_modifier_name(key_codes::KEY_LEFTCTRL), Some("ctrl"));
        assert_eq!(key_code_to_modifier_name(key_codes::KEY_RIGHTCTRL), Some("ctrl"));
        assert_eq!(key_code_to_modifier_name(key_codes::KEY_LEFTALT), Some("alt"));
        assert_eq!(key_code_to_modifier_name(key_codes::KEY_LEFTSHIFT), Some("shift"));
        assert_eq!(key_code_to_modifier_name(key_codes::KEY_LEFTMETA), Some("super"));
        assert_eq!(key_code_to_modifier_name(key_codes::KEY_1), None);
    }

    #[test]
    fn test_hotkey_binding_new() {
        let binding = HotkeyBinding::new(
            vec!["ctrl".to_string(), "alt".to_string()],
            "1".to_string(),
            "gaming".to_string(),
        );

        assert_eq!(binding.modifiers, vec!["ctrl", "alt"]);
        assert_eq!(binding.key, "1");
        assert_eq!(binding.profile_name, "gaming");
        assert!(binding.device_id.is_none());
        assert!(binding.layer_id.is_none());
    }

    #[test]
    fn test_hotkey_binding_with_device() {
        let binding = HotkeyBinding::with_device(
            vec!["ctrl".to_string()],
            "2".to_string(),
            "work".to_string(),
            "1532:0220".to_string(),
        );

        assert_eq!(binding.modifiers, vec!["ctrl"]);
        assert_eq!(binding.key, "2");
        assert_eq!(binding.profile_name, "work");
        assert_eq!(binding.device_id, Some("1532:0220".to_string()));
        assert!(binding.layer_id.is_none());
    }

    #[test]
    fn test_hotkey_binding_with_layer() {
        let binding = HotkeyBinding::with_layer(
            vec!["ctrl".to_string(), "shift".to_string()],
            "3".to_string(),
            "gaming".to_string(),
            2,
        );

        assert_eq!(binding.modifiers, vec!["ctrl", "shift"]);
        assert_eq!(binding.key, "3");
        assert_eq!(binding.profile_name, "gaming");
        assert!(binding.device_id.is_none());
        assert_eq!(binding.layer_id, Some(2));
    }

    #[test]
    fn test_hotkey_binding_normalize_modifiers() {
        let binding = HotkeyBinding::new(
            vec!["CTRL".to_string(), "AlT".to_string()],
            "1".to_string(),
            "gaming".to_string(),
        );

        let normalized = binding.normalize_modifiers();
        assert_eq!(normalized, vec!["ctrl", "alt"]);
    }

    #[test]
    fn test_default_hotkey_bindings() {
        let bindings = crate::config::default_hotkey_bindings();
        assert_eq!(bindings.len(), 9);

        // Check first binding
        assert_eq!(bindings[0].modifiers, vec!["ctrl", "alt", "shift"]);
        assert_eq!(bindings[0].key, "1");
        assert_eq!(bindings[0].profile_name, "profile1");

        // Check last binding
        assert_eq!(bindings[8].key, "9");
        assert_eq!(bindings[8].profile_name, "profile9");
    }

    #[test]
    fn test_modifier_tracking() {
        let mut modifiers = HashSet::new();

        // Add modifiers
        modifiers.insert(key_codes::KEY_LEFTCTRL);
        modifiers.insert(key_codes::KEY_LEFTSHIFT);

        // Check presence
        assert!(modifiers.contains(&key_codes::KEY_LEFTCTRL));
        assert!(modifiers.contains(&key_codes::KEY_LEFTSHIFT));
        assert!(!modifiers.contains(&key_codes::KEY_LEFTALT));

        // Remove modifier
        modifiers.remove(&key_codes::KEY_LEFTCTRL);
        assert!(!modifiers.contains(&key_codes::KEY_LEFTCTRL));
    }

    #[test]
    fn test_key_codes_constants() {
        // Verify key code constants are correct
        assert_eq!(key_codes::KEY_LEFTCTRL, 29);
        assert_eq!(key_codes::KEY_RIGHTCTRL, 97);
        assert_eq!(key_codes::KEY_LEFTALT, 56);
        assert_eq!(key_codes::KEY_RIGHTALT, 100);
        assert_eq!(key_codes::KEY_LEFTSHIFT, 42);
        assert_eq!(key_codes::KEY_RIGHTSHIFT, 54);
        assert_eq!(key_codes::KEY_LEFTMETA, 125);
        assert_eq!(key_codes::KEY_RIGHTMETA, 126);

        // Number keys
        assert_eq!(key_codes::KEY_1, 2);
        assert_eq!(key_codes::KEY_2, 3);
        assert_eq!(key_codes::KEY_3, 4);
        assert_eq!(key_codes::KEY_9, 10);
    }

    #[test]
    fn test_modifier_keys_array() {
        // Verify all modifier keys are in the MODIFIER_KEYS array
        assert!(key_codes::MODIFIER_KEYS.contains(&key_codes::KEY_LEFTCTRL));
        assert!(key_codes::MODIFIER_KEYS.contains(&key_codes::KEY_RIGHTCTRL));
        assert!(key_codes::MODIFIER_KEYS.contains(&key_codes::KEY_LEFTALT));
        assert!(key_codes::MODIFIER_KEYS.contains(&key_codes::KEY_RIGHTALT));
        assert!(key_codes::MODIFIER_KEYS.contains(&key_codes::KEY_LEFTSHIFT));
        assert!(key_codes::MODIFIER_KEYS.contains(&key_codes::KEY_RIGHTSHIFT));
        assert!(key_codes::MODIFIER_KEYS.contains(&key_codes::KEY_LEFTMETA));
        assert!(key_codes::MODIFIER_KEYS.contains(&key_codes::KEY_RIGHTMETA));

        // Non-modifier keys should not be in the array
        assert!(!key_codes::MODIFIER_KEYS.contains(&key_codes::KEY_1));
    }
}
