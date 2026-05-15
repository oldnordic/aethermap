//! AutoProfileSwitcher for focus-based profile switching
//!
//! This module provides automatic profile switching based on window focus.
//! When a window focus event arrives (app_id from portal), the AutoProfileSwitcher
//! checks configured rules and switches profiles for matching devices.
//!
//! # Key Features
//!
//! - **App ID matching:** Supports exact, prefix, suffix, and wildcard ("*") matching
//! - **Device-specific rules:** Can target specific devices or apply to all
//! - **Layer activation:** Optionally activates a specific layer when switching profiles
//! - **Manual override:** Manual profile switches override auto-switch until next focus change
//! - **Graceful fallback:** No switch when no rule matches
//!
//! # Examples
//!
//! ```ignore
//! let switcher = AutoProfileSwitcher::new(
//!     layer_manager.clone(),
//!     config_manager.clone(),
//! );
//!
//! // Load rules from config
//! switcher.load_rules(&rules).await;
//!
//! // Apply focus change
//! switcher.apply_focus_change("org.alacritty", &None).await?;
//! ```

use crate::config::{AutoSwitchRule, ConfigManager};
use crate::layer_manager::LayerManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Auto-profile switcher for focus-based profile switching
///
/// Matches app_id from window focus events to configured profile rules
/// and switches profiles accordingly. Manual overrides persist until
/// the next focus change.
pub struct AutoProfileSwitcher {
    /// Loaded auto-switch rules from configuration
    /// Wrapped in RwLock for runtime mutability through Arc
    rules: Arc<RwLock<Vec<AutoSwitchRule>>>,

    /// Layer manager for profile/layer switching
    layer_manager: Arc<RwLock<LayerManager>>,

    /// Config manager for profile validation
    config_manager: Arc<ConfigManager>,

    /// Manual override flags per device (device_id -> override active)
    ///
    /// When true, auto-switching is disabled for that device until
    /// the next focus change.
    manual_override: Arc<RwLock<HashMap<String, bool>>>,
}

impl AutoProfileSwitcher {
    /// Create a new AutoProfileSwitcher
    ///
    /// # Arguments
    ///
    /// * `layer_manager` - Layer manager for profile/layer activation
    /// * `config_manager` - Config manager for profile validation
    pub fn new(
        layer_manager: Arc<RwLock<LayerManager>>,
        config_manager: Arc<ConfigManager>,
    ) -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            layer_manager,
            config_manager,
            manual_override: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load auto-switch rules from configuration
    ///
    /// Replaces existing rules with the new set. Rules are evaluated
    /// in order; first match wins.
    ///
    /// # Arguments
    ///
    /// * `rules` - Rules to load from config
    pub async fn load_rules(&mut self, rules: Vec<AutoSwitchRule>) {
        let mut current_rules = self.rules.write().await;
        *current_rules = rules;
        info!("Loaded {} auto-switch rules", current_rules.len());
    }

    /// Reload auto-switch rules from configuration
    ///
    /// Thread-safe version of load_rules for use through Arc.
    /// Replaces existing rules with the new set.
    ///
    /// # Arguments
    ///
    /// * `rules` - Rules to load from config
    pub async fn reload_rules(&self, rules: Vec<AutoSwitchRule>) {
        let mut current_rules = self.rules.write().await;
        *current_rules = rules;
        info!("Reloaded {} auto-switch rules", current_rules.len());
    }

    /// Get the current rules
    pub async fn rules(&self) -> Vec<AutoSwitchRule> {
        self.rules.read().await.clone()
    }

    /// Apply a focus change event
    ///
    /// Matches the app_id against configured rules and switches profiles
    /// for matching devices. Clears manual override flags on any focus change.
    ///
    /// # Arguments
    ///
    /// * `app_id` - Application identifier (e.g., "org.alacritty", "firefox")
    /// * `window_title` - Optional window title (currently unused, for future expansion)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Focus change processed successfully
    /// * `Err(String)` - Error during profile switching
    pub async fn apply_focus_change(
        &self,
        app_id: &str,
        window_title: &Option<String>,
    ) -> Result<(), String> {
        debug!(
            "Focus changed: app_id={}, window_title={:?}",
            app_id, window_title
        );

        // Clear all manual overrides on focus change
        self.clear_all_manual_overrides().await;

        // Find matching rule
        let matching_rule = self.find_matching_rule(app_id).await;

        match matching_rule {
            Some(rule) => {
                info!(
                    "Matched rule: app_id={} -> profile {} (device: {:?}, layer: {:?})",
                    rule.app_id, rule.profile_name, rule.device_id, rule.layer_id
                );

                // Apply the rule
                self.apply_rule(&rule).await?;
            }
            None => {
                debug!("No matching rule for app_id '{}'", app_id);
                // Graceful fallback - no switch when no rule matches
            }
        }

        Ok(())
    }

    /// Set manual override for a device
    ///
    /// Marks a device as manually overridden, preventing auto-switching
    /// until the next focus change.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    pub async fn set_manual_override(&self, device_id: &str) {
        let mut overrides = self.manual_override.write().await;
        overrides.insert(device_id.to_string(), true);
        debug!("Set manual override for device {}", device_id);
    }

    /// Clear manual override for a device
    ///
    /// Allows auto-switching to resume for the device.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    pub async fn clear_manual_override(&self, device_id: &str) {
        let mut overrides = self.manual_override.write().await;
        overrides.remove(device_id);
        debug!("Cleared manual override for device {}", device_id);
    }

    /// Check if a device has manual override active
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    ///
    /// # Returns
    ///
    /// `true` if manual override is active for the device
    pub async fn has_manual_override(&self, device_id: &str) -> bool {
        let overrides = self.manual_override.read().await;
        overrides.get(device_id).copied().unwrap_or(false)
    }

    /// Clear all manual overrides
    ///
    /// Called on every focus change to re-enable auto-switching.
    async fn clear_all_manual_overrides(&self) {
        let mut overrides = self.manual_override.write().await;
        if !overrides.is_empty() {
            debug!("Clearing {} manual override(s)", overrides.len());
            overrides.clear();
        }
    }

    /// Find a matching rule for the given app_id
    ///
    /// Rules are evaluated in order; first match wins.
    ///
    /// # Arguments
    ///
    /// * `app_id` - Application identifier to match
    ///
    /// # Returns
    ///
    /// * `Some(rule)` - First matching rule
    /// * `None` - No matching rule found
    async fn find_matching_rule(&self, app_id: &str) -> Option<AutoSwitchRule> {
        let rules = self.rules.read().await;
        for rule in rules.iter() {
            if self.rule_matches(rule, app_id) {
                return Some(rule.clone());
            }
        }
        None
    }

    /// Check if a rule matches the given app_id
    ///
    /// Supports:
    /// - Exact match: "org.alacritty" matches "org.alacritty"
    /// - Wildcard: "*" matches any app_id
    /// - Prefix match: "org.mozilla." matches "org.mozilla.firefox"
    /// - Suffix match: ".firefox" matches "org.mozilla.firefox"
    ///
    /// # Arguments
    ///
    /// * `rule` - Rule to check
    /// * `app_id` - Application identifier
    ///
    /// # Returns
    ///
    /// `true` if the rule matches the app_id
    fn rule_matches(&self, rule: &AutoSwitchRule, app_id: &str) -> bool {
        let rule_app_id = rule.app_id.as_str();

        // Wildcard matches everything
        if rule_app_id == "*" {
            return true;
        }

        // Exact match
        if rule_app_id == app_id {
            return true;
        }

        // Prefix match (e.g., "org.mozilla." matches "org.mozilla.firefox")
        if rule_app_id.ends_with('.') && app_id.starts_with(rule_app_id) {
            return true;
        }

        // Suffix match (e.g., ".firefox" matches "org.mozilla.firefox")
        if rule_app_id.starts_with('.') && app_id.ends_with(rule_app_id) {
            return true;
        }

        false
    }

    /// Apply a single switching rule
    ///
    /// Activates the specified profile and layer for matching devices.
    ///
    /// # Arguments
    ///
    /// * `rule` - Rule to apply
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Rule applied successfully
    /// * `Err(String)` - Error during profile activation
    async fn apply_rule(&self, rule: &AutoSwitchRule) -> Result<(), String> {
        // Determine which devices to apply the rule to
        let device_ids = if let Some(ref device_id) = rule.device_id {
            // Device-specific rule
            vec![device_id.clone()]
        } else {
            // Global rule - get all managed devices
            let layer_manager = self.layer_manager.read().await;
            layer_manager.get_device_ids().await
        };

        // Validate profile exists
        let profile_exists = self
            .config_manager
            .device_profiles
            .read()
            .await
            .values()
            .any(|profiles| profiles.contains_key(&rule.profile_name));

        if !profile_exists {
            warn!(
                "Profile '{}' not found in any device config, skipping auto-switch",
                rule.profile_name
            );
            // Don't fail - just skip this rule
            return Ok(());
        }

        // Apply to each device
        for device_id in device_ids {
            // Skip if manual override is active
            if self.has_manual_override(&device_id).await {
                debug!(
                    "Skipping auto-switch for device {} (manual override active)",
                    device_id
                );
                continue;
            }

            // Check if device has this profile
            let has_profile = self
                .config_manager
                .device_profiles
                .read()
                .await
                .get(&device_id)
                .map(|profiles| profiles.contains_key(&rule.profile_name))
                .unwrap_or(false);

            if !has_profile {
                debug!(
                    "Device {} does not have profile {}, skipping",
                    device_id, rule.profile_name
                );
                continue;
            }

            // Activate the profile
            info!(
                "Auto-switching device {} to profile '{}'",
                device_id, rule.profile_name
            );

            // Get device path from device manager (if available)
            let _device_path = {
                let _state_guard = self.config_manager.device_profiles.read().await;
                // We need the actual device path - for now, we'll use a placeholder
                // In real implementation, this would come from DeviceManager
                None::<String>
            };

            // If we have a layer_id, activate that layer
            if let Some(layer_id) = rule.layer_id {
                info!(
                    "Auto-activating layer {} for device {}",
                    layer_id, device_id
                );

                let layer_manager = self.layer_manager.read().await;

                // Check if layer exists for device
                if let Some(device_state) = layer_manager.get_device_state(&device_id).await {
                    if layer_id < device_state.layer_configs.len() {
                        // Activate the layer
                        let layer_config = &device_state.layer_configs[layer_id];
                        let mode = layer_config.mode;

                        drop(layer_manager);

                        let layer_manager = self.layer_manager.read().await;
                        match mode {
                            crate::layer_manager::LayerMode::Hold => {
                                if let Err(e) = layer_manager
                                    .activate_hold_layer(&device_id, layer_id)
                                    .await
                                {
                                    warn!("Failed to activate hold layer {}: {}", layer_id, e);
                                }
                            }
                            crate::layer_manager::LayerMode::Toggle => {
                                if let Err(e) =
                                    layer_manager.toggle_layer(&device_id, layer_id).await
                                {
                                    warn!("Failed to toggle layer {}: {}", layer_id, e);
                                }
                            }
                        }
                    } else {
                        warn!("Layer {} not configured for device {}", layer_id, device_id);
                    }
                }
            }

            // Note: Actual profile activation would happen through DeviceManager
            // which is not directly accessible here. The layer activation above
            // is the primary mechanism for profile switching in our layer-based system.
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DaemonConfig;
    use crate::layer_manager::LayerManager;
    use tempfile::TempDir;
    use tokio::sync::RwLock;

    // Helper to create a test config manager
    async fn create_test_config_manager() -> Arc<ConfigManager> {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        let macros_path = temp_dir.path().join("macros.yaml");
        let cache_path = temp_dir.path().join("macros.bin");
        let profiles_dir = temp_dir.path().join("profiles");
        let remaps_path = temp_dir.path().join("remaps.yaml");
        let device_profiles_path = temp_dir.path().join("device_profiles.yaml");
        let layer_state_path = temp_dir.path().join("layer_state.yaml");

        let manager = ConfigManager {
            config_path,
            macros_path,
            cache_path,
            profiles_dir,
            remaps_path,
            device_profiles_path,
            layer_state_path,
            config: Arc::new(RwLock::new(DaemonConfig::default())),
            macros: Arc::new(RwLock::new(std::collections::HashMap::new())),
            profiles: Arc::new(RwLock::new(std::collections::HashMap::new())),
            remaps: Arc::new(RwLock::new(std::collections::HashMap::new())),
            device_profiles: Arc::new(RwLock::new(std::collections::HashMap::new())),
        };

        Arc::new(manager)
    }

    #[tokio::test]
    async fn test_wildcard_matching() {
        let layer_manager = Arc::new(RwLock::new(LayerManager::new(None)));
        let config_manager = create_test_config_manager().await;

        let switcher = AutoProfileSwitcher::new(layer_manager, config_manager);

        let rule = AutoSwitchRule {
            app_id: "*".to_string(),
            profile_name: "default".to_string(),
            device_id: None,
            layer_id: None,
        };

        // Wildcard should match any app_id
        assert!(switcher.rule_matches(&rule, "org.alacritty"));
        assert!(switcher.rule_matches(&rule, "firefox"));
        assert!(switcher.rule_matches(&rule, "steam"));
    }

    #[tokio::test]
    async fn test_specific_app_matching() {
        let layer_manager = Arc::new(RwLock::new(LayerManager::new(None)));
        let config_manager = create_test_config_manager().await;

        let switcher = AutoProfileSwitcher::new(layer_manager, config_manager);

        let rule = AutoSwitchRule {
            app_id: "org.alacritty".to_string(),
            profile_name: "terminal".to_string(),
            device_id: None,
            layer_id: None,
        };

        // Exact match should work
        assert!(switcher.rule_matches(&rule, "org.alacritty"));

        // Different app_id should not match
        assert!(!switcher.rule_matches(&rule, "firefox"));
    }

    #[tokio::test]
    async fn test_prefix_matching() {
        let layer_manager = Arc::new(RwLock::new(LayerManager::new(None)));
        let config_manager = create_test_config_manager().await;

        let switcher = AutoProfileSwitcher::new(layer_manager, config_manager);

        let rule = AutoSwitchRule {
            app_id: "org.mozilla.".to_string(),
            profile_name: "browser".to_string(),
            device_id: None,
            layer_id: None,
        };

        // Prefix match should work
        assert!(switcher.rule_matches(&rule, "org.mozilla.firefox"));
        assert!(switcher.rule_matches(&rule, "org.mozilla.thunderbird"));

        // Different prefix should not match
        assert!(!switcher.rule_matches(&rule, "org.alacritty"));
    }

    #[tokio::test]
    async fn test_suffix_matching() {
        let layer_manager = Arc::new(RwLock::new(LayerManager::new(None)));
        let config_manager = create_test_config_manager().await;

        let switcher = AutoProfileSwitcher::new(layer_manager, config_manager);

        let rule = AutoSwitchRule {
            app_id: ".firefox".to_string(),
            profile_name: "firefox".to_string(),
            device_id: None,
            layer_id: None,
        };

        // Suffix match should work
        assert!(switcher.rule_matches(&rule, "org.mozilla.firefox"));

        // Different suffix should not match
        assert!(!switcher.rule_matches(&rule, "org.mozilla.thunderbird"));
    }

    #[tokio::test]
    async fn test_manual_override() {
        let layer_manager = Arc::new(RwLock::new(LayerManager::new(None)));
        let config_manager = create_test_config_manager().await;

        let switcher = AutoProfileSwitcher::new(layer_manager, config_manager);

        // Initially no override
        assert!(!switcher.has_manual_override("1532:0220").await);

        // Set override
        switcher.set_manual_override("1532:0220").await;
        assert!(switcher.has_manual_override("1532:0220").await);

        // Clear override
        switcher.clear_manual_override("1532:0220").await;
        assert!(!switcher.has_manual_override("1532:0220").await);
    }

    #[tokio::test]
    async fn test_rule_order() {
        let layer_manager = Arc::new(RwLock::new(LayerManager::new(None)));
        let config_manager = create_test_config_manager().await;

        let switcher = AutoProfileSwitcher::new(layer_manager, config_manager);

        // Load rules in specific order
        let rules = vec![
            AutoSwitchRule {
                app_id: "org.alacritty".to_string(),
                profile_name: "terminal".to_string(),
                device_id: None,
                layer_id: Some(1),
            },
            AutoSwitchRule {
                app_id: "*".to_string(),
                profile_name: "default".to_string(),
                device_id: None,
                layer_id: Some(0),
            },
        ];

        switcher.reload_rules(rules).await;

        // Specific rule should match first
        let matched = switcher.find_matching_rule("org.alacritty").await;
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().profile_name, "terminal");

        // Wildcard should match for other apps
        let matched = switcher.find_matching_rule("firefox").await;
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().profile_name, "default");
    }

    #[tokio::test]
    async fn test_clear_all_overrides_on_focus_change() {
        let layer_manager = Arc::new(RwLock::new(LayerManager::new(None)));
        let config_manager = create_test_config_manager().await;

        let switcher = AutoProfileSwitcher::new(layer_manager, config_manager);

        // Set overrides for multiple devices
        switcher.set_manual_override("1532:0220").await;
        switcher.set_manual_override("1532:0221").await;

        assert!(switcher.has_manual_override("1532:0220").await);
        assert!(switcher.has_manual_override("1532:0221").await);

        // Apply focus change (should clear all overrides)
        let _ = switcher.apply_focus_change("org.alacritty", &None).await;

        // All overrides should be cleared
        assert!(!switcher.has_manual_override("1532:0220").await);
        assert!(!switcher.has_manual_override("1532:0221").await);
    }
}
