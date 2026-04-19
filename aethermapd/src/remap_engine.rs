//! RemapEngine for translating input key codes to output key codes
//!
//! This module provides the core remapping functionality that translates
//! input key codes from grabbed devices to remapped output key codes.
//!
//! # Layer-Aware Remapping
//!
//! The RemapEngine supports multi-layer remapping with cascading fallback.
//! Each layer can have its own set of key remappings, and higher-priority
//! layers override lower ones. When looking up a remapping, the engine
//! searches from the effective layer down to the base layer.

use evdev::Key;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::key_parser::{KeyParser, ParseError};
use crate::layer_manager::LayerManager;

/// Pre-validated remap table for atomic profile switching
///
/// RemapTable is an immutable HashMap of key remappings that has been validated
/// at creation time. It is stored in an Arc for O(1) atomic pointer swaps during
/// profile switching, avoiding memory allocations or locks during event processing.
///
/// # Examples
///
/// ```ignore
/// let table: RemapTable = HashMap::from([
///     (Key::KEY_A, Key::KEY_B),
///     (Key::KEY_CAPSLOCK, Key::KEY_LEFTCTRL),
/// ]);
/// let shared_table = Arc::new(table);
/// ```
pub type RemapTable = HashMap<Key, Key>;

/// Error type for remap engine operations
#[derive(Debug)]
pub enum RemapError {
    /// Invalid key name in configuration
    InvalidKey {
        key: String,
        source: String,
        parse_error: String,
    },

    /// Configuration file error
    Config(String),

    /// Key parsing error
    ParseError(ParseError),
}

impl fmt::Display for RemapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RemapError::InvalidKey { key, source, parse_error } => {
                write!(f, "Invalid key name '{}' in {}: {}", key, source, parse_error)
            }
            RemapError::Config(msg) => write!(f, "Configuration error: {}", msg),
            RemapError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for RemapError {}

impl From<ParseError> for RemapError {
    fn from(err: ParseError) -> Self {
        RemapError::ParseError(err)
    }
}

/// A pre-validated remap profile for fast switching
///
/// RemapProfile stores key remappings that have been validated at creation time.
/// This enables atomic profile switching via Arc pointer exchange (O(1) operation).
///
/// # Examples
///
/// ```ignore
/// let profile = RemapProfile::new("gaming", &config)?;
/// // remaps is Arc<RwLock<HashMap<Key, Key>>> - ready for O(1) lookup
/// ```
#[derive(Debug, Clone)]
pub struct RemapProfile {
    /// Profile name for identification
    pub name: String,

    /// Pre-validated remappings from input to output key codes
    /// Arc enables cheap cloning for profile switching
    pub remaps: Arc<RwLock<HashMap<Key, Key>>>,

    /// Key parser used for validation (shared with RemapEngine)
    key_parser: Arc<KeyParser>,
}

impl RemapProfile {
    /// Create a new RemapProfile from a configuration HashMap
    ///
    /// This method validates all key names before creating the profile.
    /// If any key name is invalid, the profile creation fails.
    ///
    /// # Arguments
    ///
    /// * `name` - Profile name for identification
    /// * `config` - HashMap of key name pairs (input -> output)
    ///
    /// # Returns
    ///
    /// * `Ok(RemapProfile)` - Profile with validated remappings
    /// * `Err(RemapError)` - Invalid key name
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut config = HashMap::new();
    /// config.insert("capslock".to_string(), "leftctrl".to_string());
    /// let profile = RemapProfile::new("work", &config)?;
    /// ```
    pub fn new(
        name: String,
        config: &HashMap<String, String>,
    ) -> Result<Self, RemapError> {
        let key_parser = Arc::new(KeyParser::new());
        Self::with_key_parser(name, config, key_parser)
    }

    /// Create a new RemapProfile with a shared key parser
    ///
    /// This allows sharing a KeyParser instance across multiple components,
    /// reducing memory overhead when creating many profiles.
    ///
    /// # Arguments
    ///
    /// * `name` - Profile name for identification
    /// * `config` - HashMap of key name pairs (input -> output)
    /// * `key_parser` - Shared KeyParser instance
    pub fn with_key_parser(
        name: String,
        config: &HashMap<String, String>,
        key_parser: Arc<KeyParser>,
    ) -> Result<Self, RemapError> {
        // Validate all keys first (eager validation)
        let mut remaps = HashMap::new();

        for (input_name, output_name) in config.iter() {
            let input_key = key_parser.parse(input_name).map_err(|e| {
                RemapError::InvalidKey {
                    key: input_name.clone(),
                    source: "input".to_string(),
                    parse_error: e.to_string(),
                }
            })?;

            let output_key = key_parser.parse(output_name).map_err(|e| {
                RemapError::InvalidKey {
                    key: output_name.clone(),
                    source: "output".to_string(),
                    parse_error: e.to_string(),
                }
            })?;

            remaps.insert(input_key, output_key);
        }

        Ok(Self {
            name,
            remaps: Arc::new(RwLock::new(remaps)),
            key_parser,
        })
    }

    /// Get the profile name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the number of remappings in this profile
    pub async fn remap_count(&self) -> usize {
        self.remaps.read().await.len()
    }

    /// Check if a specific key has a remapping
    pub async fn has_remap(&self, key_code: Key) -> bool {
        self.remaps.read().await.contains_key(&key_code)
    }

    /// Get a clone of the remappings HashMap
    pub async fn get_remaps(&self) -> HashMap<Key, Key> {
        self.remaps.read().await.clone()
    }

    /// Get a reference to the shared remappings Arc
    ///
    /// This is used for atomic profile switching via Arc pointer exchange.
    /// The Arc<RwLock<>> wrapper allows cheap cloning for profile switching.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let profile = RemapProfile::new("gaming".to_string(), &config)?;
    /// let remaps_arc = profile.remaps_arc();
    /// // Clone Arc for atomic pointer swap (O(1) operation)
    /// let cloned = Arc::clone(remaps_arc);
    /// ```
    pub fn remaps_arc(&self) -> &Arc<RwLock<HashMap<Key, Key>>> {
        &self.remaps
    }

    /// Get the key parser
    pub fn key_parser(&self) -> &Arc<KeyParser> {
        &self.key_parser
    }
}

/// RemapEngine stores and applies key code remappings
///
/// The engine uses a HashMap for O(1) lookup performance during the event loop.
/// All key names are validated eagerly at config load time (fail-fast semantics).
///
/// # Layer-Aware Remapping
///
/// The engine supports per-layer remappings with cascading fallback. When querying
/// for a remapping, it searches from the effective layer down to the base layer,
/// allowing higher layers to override lower ones.
///
/// # Examples
///
/// ```ignore
/// let mut engine = RemapEngine::new();
///
/// // Load remappings from config
/// let mut config = HashMap::new();
/// config.insert("KEY_A".to_string(), "KEY_B".to_string());
/// config.insert("capslock".to_string(), "KEY_LEFTCTRL".to_string());
/// engine.load_config(&config).await.unwrap();
///
/// // Remap keys during event loop
/// if let Some(remapped) = engine.remap(Key::KEY_A).await {
///     inject_key(remapped); // Inject KEY_B instead
/// }
/// ```
pub struct RemapEngine {
    /// Mapping from input key codes to output key codes (base layer)
    /// Wrapped in Arc<RwLock<>> for concurrent access
    /// Maintained for backward compatibility - maps to layer_remaps[0]
    remaps: Arc<RwLock<HashMap<Key, Key>>>,

    /// Per-layer remap tables indexed by layer_id
    /// layer_remaps[0] = base layer, layer_remaps[1] = layer 1, etc.
    /// Use Arc for cheap cloning during profile switches
    layer_remaps: Vec<Arc<RwLock<HashMap<Key, Key>>>>,

    /// Layer manager for tracking effective layer per device
    layer_manager: Arc<RwLock<LayerManager>>,

    /// Key parser for converting key names to evdev::Key codes
    key_parser: Arc<KeyParser>,
}

impl RemapEngine {
    /// Create a new RemapEngine with empty remappings
    ///
    /// Initializes with minimum 3 empty layer remap tables (base, layer 1, layer 2).
    pub fn new() -> Self {
        // Initialize with minimum 3 layers (base, layer 1, layer 2)
        let layer_remaps = vec![
            Arc::new(RwLock::new(HashMap::new())), // Layer 0 (base)
            Arc::new(RwLock::new(HashMap::new())), // Layer 1
            Arc::new(RwLock::new(HashMap::new())), // Layer 2
        ];

        Self {
            remaps: Arc::new(RwLock::new(HashMap::new())),
            layer_remaps,
            layer_manager: Arc::new(RwLock::new(LayerManager::new(None))),
            key_parser: Arc::new(KeyParser::new()),
        }
    }

    /// Create a new RemapEngine with a specific key parser
    ///
    /// This allows sharing a KeyParser instance across multiple components
    pub fn with_key_parser(key_parser: Arc<KeyParser>) -> Self {
        let layer_remaps = vec![
            Arc::new(RwLock::new(HashMap::new())), // Layer 0 (base)
            Arc::new(RwLock::new(HashMap::new())), // Layer 1
            Arc::new(RwLock::new(HashMap::new())), // Layer 2
        ];

        Self {
            remaps: Arc::new(RwLock::new(HashMap::new())),
            layer_remaps,
            layer_manager: Arc::new(RwLock::new(LayerManager::new(None))),
            key_parser,
        }
    }

    /// Set the layer manager for this engine
    ///
    /// Allows external LayerManager injection (from DaemonState).
    /// This is useful when a single LayerManager is shared across
    /// multiple components.
    ///
    /// # Arguments
    ///
    /// * `layer_manager` - Shared layer manager instance
    pub fn set_layer_manager(&mut self, layer_manager: Arc<RwLock<LayerManager>>) {
        self.layer_manager = layer_manager;
    }

    /// Load remappings for a specific layer
    ///
    /// Parses and validates config keys using key_parser, then stores
    /// the remappings in the specified layer's remap table.
    ///
    /// # Arguments
    ///
    /// * `layer_id` - Layer ID to load remappings into (0 = base, 1+ = additional)
    /// * `config` - HashMap of key name pairs (input -> output)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Config loaded successfully
    /// * `Err(RemapError)` - Invalid key name or layer ID out of bounds
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let engine = RemapEngine::new();
    ///
    /// let mut config = HashMap::new();
    /// config.insert("a".to_string(), "b".to_string());
    ///
    /// // Load into layer 1 (gaming layer)
    /// engine.load_layer_remap(1, &config).await?;
    /// ```
    pub async fn load_layer_remap(
        &self,
        layer_id: usize,
        config: &HashMap<String, String>,
    ) -> Result<(), RemapError> {
        // Validate layer_id
        if layer_id >= self.layer_remaps.len() {
            return Err(RemapError::Config(format!(
                "Layer ID {} exceeds available layers ({} layers configured)",
                layer_id,
                self.layer_remaps.len()
            )));
        }

        info!("Loading key remap configuration for layer {}", layer_id);

        // Eager validation: Parse ALL keys first before storing any
        let mut parsed_remaps = HashMap::new();

        for (input_name, output_name) in config.iter() {
            // Parse input key
            let input_key = self.key_parser.parse(input_name).map_err(|e| {
                RemapError::InvalidKey {
                    key: input_name.clone(),
                    source: "input".to_string(),
                    parse_error: e.to_string(),
                }
            })?;

            // Parse output key
            let output_key = self.key_parser.parse(output_name).map_err(|e| {
                RemapError::InvalidKey {
                    key: output_name.clone(),
                    source: "output".to_string(),
                    parse_error: e.to_string(),
                }
            })?;

            // Store the parsed mapping
            parsed_remaps.insert(input_key, output_key);

            debug!("Layer {} remap: {} -> {}", layer_id, input_name, output_name);
        }

        // All keys validated successfully - now update the layer remappings
        let mut layer_remap = self.layer_remaps[layer_id].write().await;
        *layer_remap = parsed_remaps;

        // Also update base layer (remaps field) for backward compatibility
        if layer_id == 0 {
            let mut remaps = self.remaps.write().await;
            *remaps = layer_remap.clone();
        }

        info!(
            "Loaded {} key remappings for layer {} successfully",
            layer_remap.len(),
            layer_id
        );

        Ok(())
    }

    /// Get the number of configured layers
    ///
    /// # Returns
    ///
    /// The count of layer remap tables available
    pub fn layer_count(&self) -> usize {
        self.layer_remaps.len()
    }

    /// Load remappings from a configuration HashMap
    ///
    /// This method performs eager validation - ALL key names are validated
    /// before any remappings are stored. If any key name is invalid, the entire
    /// config load fails and no changes are made to the remappings.
    ///
    /// # Arguments
    ///
    /// * `config` - HashMap of key name pairs (input -> output)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Config loaded successfully
    /// * `Err(RemapError)` - Invalid key name or config error
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let engine = RemapEngine::new();
    ///
    /// let mut config = HashMap::new();
    /// config.insert("a".to_string(), "b".to_string());
    /// config.insert("capslock".to_string(), "leftctrl".to_string());
    ///
    /// engine.load_config(&config).await?;
    /// ```
    pub async fn load_config(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<(), RemapError> {
        info!("Loading key remap configuration");

        // Eager validation: Parse ALL keys first before storing any
        let mut parsed_remaps = HashMap::new();

        for (input_name, output_name) in config.iter() {
            // Parse input key
            let input_key = self.key_parser.parse(input_name).map_err(|e| {
                RemapError::InvalidKey {
                    key: input_name.clone(),
                    source: "input".to_string(),
                    parse_error: e.to_string(),
                }
            })?;

            // Parse output key
            let output_key = self.key_parser.parse(output_name).map_err(|e| {
                RemapError::InvalidKey {
                    key: output_name.clone(),
                    source: "output".to_string(),
                    parse_error: e.to_string(),
                }
            })?;

            // Store the parsed mapping
            parsed_remaps.insert(input_key, output_key);

            debug!("Remap: {} -> {}", input_name, output_name);
        }

        // All keys validated successfully - now update the remappings
        let mut remaps = self.remaps.write().await;
        *remaps = parsed_remaps;

        info!(
            "Loaded {} key remappings successfully",
            remaps.len()
        );

        Ok(())
    }

    /// Remap an input key code to its configured output
    ///
    /// Returns `Some(output_key)` if a remapping exists, `None` otherwise.
    /// This allows the caller to distinguish between "no remapping" and
    /// "remapped to a specific key".
    ///
    /// # Arguments
    ///
    /// * `key_code` - The input key code to translate
    ///
    /// # Returns
    ///
    /// * `Some(Key)` - The remapped output key code
    /// * `None` - No remapping configured for this input
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if let Some(output_key) = engine.remap(input_key).await {
    ///     // A remapping exists - inject the output key
    ///     injector.key_press(output_key).await?;
    /// } else {
    ///     // No remapping - inject the original key
    ///     injector.key_press(input_key).await?;
    /// }
    /// ```
    pub async fn remap(&self, key_code: Key) -> Option<Key> {
        let remaps = self.remaps.read().await;
        let output = remaps.get(&key_code).copied();

        if let Some(out) = output {
            debug!("Remapped {:?} -> {:?}", key_code, out);
        }

        output
    }

    /// Process an input event with full event value (0=release, 1=press, 2=repeat)
    ///
    /// This method handles key repeat events correctly by forwarding the event value
    /// unchanged. For simple key->key remaps, repeat events (value=2) are forwarded
    /// as-is to ensure held keys produce repeated output.
    ///
    /// # Arguments
    ///
    /// * `key_code` - The input key code to translate
    /// * `value` - The event value (0=release, 1=press, 2=repeat)
    ///
    /// # Returns
    ///
    /// * `Some((Key, i32))` - The remapped output key and event value
    /// * `None` - No remapping configured for this input
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if let Some((output_key, out_value)) = engine.process_event(input_key, value).await {
    ///     match out_value {
    ///         0 => injector.key_release(output_key.0).await?,
    ///         1 => injector.key_press(output_key.0).await?,
    ///         2 => injector.key_press(output_key.0).await?, // Repeat sends press
    ///         _ => {},
    ///     }
    /// }
    /// ```
    pub async fn process_event(&self, key_code: Key, value: i32) -> Option<(Key, i32)> {
        let remaps = self.remaps.read().await;

        if let Some(&output_key) = remaps.get(&key_code) {
            // For simple key->key remaps, forward all event values unchanged
            // This ensures repeat events work correctly
            Some((output_key, value))
        } else {
            None
        }
    }

    /// Get the number of active remappings
    ///
    /// # Returns
    ///
    /// The count of configured key remappings
    pub async fn remap_count(&self) -> usize {
        let remaps = self.remaps.read().await;
        remaps.len()
    }

    /// Check if a specific key has a remapping configured
    ///
    /// # Arguments
    ///
    /// * `key_code` - The key code to check
    ///
    /// # Returns
    ///
    /// `true` if a remapping exists for this key, `false` otherwise
    pub async fn has_remap(&self, key_code: Key) -> bool {
        let remaps = self.remaps.read().await;
        remaps.contains_key(&key_code)
    }

    /// Clear all remappings
    ///
    /// This removes all configured key remappings from the engine
    pub async fn clear(&self) {
        let mut remaps = self.remaps.write().await;
        let count = remaps.len();
        remaps.clear();
        info!("Cleared {} key remappings", count);
    }

    /// Get a reference to the key parser
    ///
    /// This allows other components to reuse the same parser instance
    pub fn key_parser(&self) -> &Arc<KeyParser> {
        &self.key_parser
    }

    /// Get a reference to the remappings HashMap
    ///
    /// This is primarily useful for debugging and inspection
    pub async fn get_remaps(&self) -> HashMap<Key, Key> {
        let remaps = self.remaps.read().await;
        remaps.clone()
    }

    /// Remap an input key code using layer-aware lookup with cascade fallback
    ///
    /// Gets the effective layer from layer_manager and searches for the remapping
    /// from that layer down to the base layer. Higher layers override lower ones.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    /// * `key_code` - The input key code to translate
    ///
    /// # Returns
    ///
    /// * `Some(Key)` - The remapped output key code
    /// * `None` - No remapping configured for this input in any active layer
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let engine = RemapEngine::new();
    ///
    /// // Load different remappings for layer 1
    /// let mut config = HashMap::new();
    /// config.insert("KEY_A".to_string(), "KEY_X".to_string());
    /// engine.load_layer_remap(1, &config).await.unwrap();
    ///
    /// // If layer 1 is active, A -> X
    /// if let Some(output_key) = engine.remap_layer_aware("1532:0220", Key::KEY_A).await {
    ///     inject_key(output_key);
    /// }
    /// ```
    pub async fn remap_layer_aware(&self, device_id: &str, key_code: Key) -> Option<Key> {
        // Get the effective layer from layer_manager
        let effective_layer = self.layer_manager.read().await.get_effective_layer(device_id).await;

        // Cascade from effective layer down to base layer
        for layer_id in (0..=effective_layer).rev() {
            if let Some(remaps) = self.layer_remaps.get(layer_id) {
                let remap_table = remaps.read().await;
                if let Some(&output_key) = remap_table.get(&key_code) {
                    debug!(
                        "Layer-aware remap: device {} key {:?} -> {:?} (from layer {})",
                        device_id, key_code, output_key, layer_id
                    );
                    return Some(output_key);
                }
            }
        }

        // Key not found in any layer
        debug!(
            "Layer-aware remap: device {} key {:?} not found in any layer (effective: {})",
            device_id, key_code, effective_layer
        );
        None
    }

    /// Process an input event with layer-aware lookup
    ///
    /// Same logic as remap_layer_aware but returns (output_key, value) tuple
    /// for easier integration with event processing pipelines. Forwards the
    /// event value unchanged (preserves press/release/repeat).
    ///
    /// # Arguments
    ///
    /// * `device_id` - Device identifier (vendor:product format)
    /// * `key_code` - The input key code to translate
    /// * `value` - The event value (0=release, 1=press, 2=repeat)
    ///
    /// # Returns
    ///
    /// * `Some((Key, i32))` - The remapped output key and event value
    /// * `None` - No remapping configured for this input in any active layer
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if let Some((output_key, out_value)) = engine.process_event_layer_aware(
    ///     device_id, input_key, value
    /// ).await {
    ///     match out_value {
    ///         0 => injector.key_release(output_key).await?,
    ///         1 => injector.key_press(output_key).await?,
    ///         2 => injector.key_press(output_key).await?,
    ///         _ => {},
    ///     }
    /// }
    /// ```
    pub async fn process_event_layer_aware(
        &self,
        device_id: &str,
        key_code: Key,
        value: i32,
    ) -> Option<(Key, i32)> {
        // Get the effective layer from layer_manager
        let effective_layer = self.layer_manager.read().await.get_effective_layer(device_id).await;

        // Cascade from effective layer down to base layer
        for layer_id in (0..=effective_layer).rev() {
            if let Some(remaps) = self.layer_remaps.get(layer_id) {
                let remap_table = remaps.read().await;
                if let Some(&output_key) = remap_table.get(&key_code) {
                    debug!(
                        "Layer-aware event: device {} key {:?} -> {:?} (value: {}, from layer {})",
                        device_id, key_code, output_key, value, layer_id
                    );
                    return Some((output_key, value));
                }
            }
        }

        // Key not found in any layer
        None
    }

    /// Get a reference to the layer manager
    ///
    /// This allows external components to query or manipulate layer state
    pub fn layer_manager(&self) -> &Arc<RwLock<LayerManager>> {
        &self.layer_manager
    }

    /// Get remappings for a specific layer
    ///
    /// Returns a clone of the remappings HashMap for the specified layer.
    /// This is primarily useful for debugging and inspection.
    ///
    /// # Arguments
    ///
    /// * `layer_id` - Layer ID to get remappings from
    ///
    /// # Returns
    ///
    /// * `Some(HashMap<Key, Key>)` - Layer remappings if layer exists
    /// * `None` - Layer ID out of bounds
    pub async fn get_layer_remaps(&self, layer_id: usize) -> Option<HashMap<Key, Key>> {
        if let Some(remaps) = self.layer_remaps.get(layer_id) {
            let remap_table = remaps.read().await;
            Some(remap_table.clone())
        } else {
            None
        }
    }
}

impl Default for RemapEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_remap_engine_creation() {
        let engine = RemapEngine::new();
        assert_eq!(engine.remap_count().await, 0);
    }

    #[tokio::test]
    async fn test_load_config_basic() {
        let engine = RemapEngine::new();

        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "KEY_B".to_string());

        let result = engine.load_config(&config).await;
        assert!(result.is_ok());
        assert_eq!(engine.remap_count().await, 1);
    }

    #[tokio::test]
    async fn test_load_config_with_friendly_names() {
        let engine = RemapEngine::new();

        let mut config = HashMap::new();
        config.insert("a".to_string(), "b".to_string());
        config.insert("capslock".to_string(), "leftctrl".to_string());

        let result = engine.load_config(&config).await;
        assert!(result.is_ok());
        assert_eq!(engine.remap_count().await, 2);
    }

    #[tokio::test]
    async fn test_remap_returns_correct_key() {
        let engine = RemapEngine::new();

        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "KEY_B".to_string());

        engine.load_config(&config).await.unwrap();

        // Test that A is remapped to B
        let result = engine.remap(Key::KEY_A).await;
        assert_eq!(result, Some(Key::KEY_B));

        // Test that unmapped keys return None
        let result = engine.remap(Key::KEY_C).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_invalid_input_key_fails_validation() {
        let engine = RemapEngine::new();

        let mut config = HashMap::new();
        config.insert("nonexistent_key".to_string(), "KEY_B".to_string());

        let result = engine.load_config(&config).await;
        assert!(result.is_err());

        match result {
            Err(RemapError::InvalidKey { key, source, .. }) => {
                assert_eq!(key, "nonexistent_key");
                assert_eq!(source, "input");
            }
            _ => panic!("Expected InvalidKey error"),
        }
    }

    #[tokio::test]
    async fn test_invalid_output_key_fails_validation() {
        let engine = RemapEngine::new();

        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "nonexistent_key".to_string());

        let result = engine.load_config(&config).await;
        assert!(result.is_err());

        match result {
            Err(RemapError::InvalidKey { key, source, .. }) => {
                assert_eq!(key, "nonexistent_key");
                assert_eq!(source, "output");
            }
            _ => panic!("Expected InvalidKey error"),
        }
    }

    #[tokio::test]
    async fn test_eager_validation_no_partial_load() {
        let engine = RemapEngine::new();

        let mut config = HashMap::new();
        // First entry is valid
        config.insert("KEY_A".to_string(), "KEY_B".to_string());
        // Second entry has invalid output key
        config.insert("KEY_C".to_string(), "invalid_key".to_string());

        let result = engine.load_config(&config).await;

        // Should fail entirely - no partial load
        assert!(result.is_err());

        // Engine should have 0 remappings, not 1
        assert_eq!(engine.remap_count().await, 0);
    }

    #[tokio::test]
    async fn test_case_insensitive_config() {
        let engine = RemapEngine::new();

        let mut config = HashMap::new();
        config.insert("key_a".to_string(), "KEY_B".to_string());
        config.insert("CAPSLOCK".to_string(), "leftctrl".to_string());

        let result = engine.load_config(&config).await;
        assert!(result.is_ok());
        assert_eq!(engine.remap_count().await, 2);

        // Verify the remappings work
        assert_eq!(engine.remap(Key::KEY_A).await, Some(Key::KEY_B));
        assert_eq!(engine.remap(Key::KEY_CAPSLOCK).await, Some(Key::KEY_LEFTCTRL));
    }

    #[tokio::test]
    async fn test_has_remap() {
        let engine = RemapEngine::new();

        assert!(!engine.has_remap(Key::KEY_A).await);

        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "KEY_B".to_string());
        engine.load_config(&config).await.unwrap();

        assert!(engine.has_remap(Key::KEY_A).await);
        assert!(!engine.has_remap(Key::KEY_C).await);
    }

    #[tokio::test]
    async fn test_clear_remaps() {
        let engine = RemapEngine::new();

        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "KEY_B".to_string());
        config.insert("KEY_C".to_string(), "KEY_D".to_string());
        engine.load_config(&config).await.unwrap();

        assert_eq!(engine.remap_count().await, 2);

        engine.clear().await;
        assert_eq!(engine.remap_count().await, 0);
    }

    #[tokio::test]
    async fn test_complex_remap_scenario() {
        let engine = RemapEngine::new();

        // Simulate a typical CapsLock -> Ctrl remap configuration
        let mut config = HashMap::new();
        config.insert("capslock".to_string(), "leftctrl".to_string());
        config.insert("esc".to_string(), "grave".to_string());

        engine.load_config(&config).await.unwrap();

        // Test CapsLock -> LeftCtrl
        assert_eq!(engine.remap(Key::KEY_CAPSLOCK).await, Some(Key::KEY_LEFTCTRL));
        // Test ESC -> Grave
        assert_eq!(engine.remap(Key::KEY_ESC).await, Some(Key::KEY_GRAVE));
    }

    #[tokio::test]
    async fn test_shared_key_parser() {
        let parser = Arc::new(KeyParser::new());
        let _engine = RemapEngine::with_key_parser(parser.clone());

        // Verify we can use the parser
        assert_eq!(parser.parse("a"), Ok(Key::KEY_A));
    }

    #[tokio::test]
    async fn test_get_remaps() {
        let engine = RemapEngine::new();

        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "KEY_B".to_string());

        engine.load_config(&config).await.unwrap();

        let remaps = engine.get_remaps().await;
        assert_eq!(remaps.len(), 1);
        assert_eq!(remaps.get(&Key::KEY_A), Some(&Key::KEY_B));
    }

    #[tokio::test]
    async fn test_remap_to_none_for_unmapped_keys() {
        let engine = RemapEngine::new();

        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "KEY_B".to_string());

        engine.load_config(&config).await.unwrap();

        // Mapped key returns Some
        assert!(engine.remap(Key::KEY_A).await.is_some());

        // Unmapped keys return None (not an error, just no mapping)
        assert!(engine.remap(Key::KEY_Z).await.is_none());
        assert!(engine.remap(Key::KEY_0).await.is_none());
    }

    // RemapProfile tests

    #[tokio::test]
    async fn test_remap_profile_creation() {
        let mut config = HashMap::new();
        config.insert("capslock".to_string(), "leftctrl".to_string());
        config.insert("a".to_string(), "b".to_string());

        let profile = RemapProfile::new("test-profile".to_string(), &config);
        assert!(profile.is_ok());

        let profile = profile.unwrap();
        assert_eq!(profile.name(), "test-profile");
        assert_eq!(profile.remap_count().await, 2);
    }

    #[tokio::test]
    async fn test_remap_profile_invalid_key_fails() {
        let mut config = HashMap::new();
        config.insert("invalid_key".to_string(), "KEY_A".to_string());

        let result = RemapProfile::new("bad-profile".to_string(), &config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remap_profile_arc_cloning() {
        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "KEY_B".to_string());

        let profile1 = RemapProfile::new("profile1".to_string(), &config).unwrap();
        let profile2 = profile1.clone();

        // Both should point to same remaps (Arc::clone)
        assert_eq!(profile1.name(), profile2.name());
        assert_eq!(profile1.remap_count().await, profile2.remap_count().await);
    }

    // Layer-aware remapping tests

    #[tokio::test]
    async fn test_layer_remap_creation() {
        let engine = RemapEngine::new();

        // Verify minimum 3 layers are created
        assert_eq!(engine.layer_count(), 3);

        // All layers should be empty initially
        let layer_0 = engine.get_layer_remaps(0).await;
        let layer_1 = engine.get_layer_remaps(1).await;
        let layer_2 = engine.get_layer_remaps(2).await;

        assert!(layer_0.is_some());
        assert!(layer_1.is_some());
        assert!(layer_2.is_some());

        assert!(layer_0.unwrap().is_empty());
        assert!(layer_1.unwrap().is_empty());
        assert!(layer_2.unwrap().is_empty());

        // Layer 3 should not exist
        assert!(engine.get_layer_remaps(3).await.is_none());
    }

    #[tokio::test]
    async fn test_load_layer_remap() {
        let engine = RemapEngine::new();

        // Load config into layer 1
        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "KEY_X".to_string());
        config.insert("KEY_B".to_string(), "KEY_Y".to_string());

        let result = engine.load_layer_remap(1, &config).await;
        assert!(result.is_ok());

        // Verify layer 1 has the remappings
        let layer_1 = engine.get_layer_remaps(1).await.unwrap();
        assert_eq!(layer_1.len(), 2);
        assert_eq!(layer_1.get(&Key::KEY_A), Some(&Key::KEY_X));
        assert_eq!(layer_1.get(&Key::KEY_B), Some(&Key::KEY_Y));

        // Base layer should still be empty
        let layer_0 = engine.get_layer_remaps(0).await.unwrap();
        assert!(layer_0.is_empty());
    }

    #[tokio::test]
    async fn test_load_layer_remap_invalid_layer() {
        let engine = RemapEngine::new();

        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "KEY_B".to_string());

        // Try to load into non-existent layer (only 0, 1, 2 exist)
        let result = engine.load_layer_remap(5, &config).await;
        assert!(result.is_err());

        match result {
            Err(RemapError::Config(msg)) => {
                assert!(msg.contains("Layer ID 5"));
                assert!(msg.contains("exceeds available layers"));
            }
            _ => panic!("Expected Config error with layer ID message"),
        }
    }

    #[tokio::test]
    async fn test_remap_layer_aware_base_layer() {
        let engine = RemapEngine::new();

        // Load config into base layer (layer 0)
        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "KEY_B".to_string());

        engine.load_layer_remap(0, &config).await.unwrap();

        // With no active layers, should use base layer
        let result = engine.remap_layer_aware("test_device", Key::KEY_A).await;
        assert_eq!(result, Some(Key::KEY_B));

        // Unmapped key should return None
        let result = engine.remap_layer_aware("test_device", Key::KEY_C).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_remap_layer_aware_cascade() {
        let engine = RemapEngine::new();

        // Load different remappings for each layer
        // Base layer: A -> B, C -> D
        let mut base_config = HashMap::new();
        base_config.insert("KEY_A".to_string(), "KEY_B".to_string());
        base_config.insert("KEY_C".to_string(), "KEY_D".to_string());
        engine.load_layer_remap(0, &base_config).await.unwrap();

        // Layer 1: A -> X (overrides base)
        let mut layer1_config = HashMap::new();
        layer1_config.insert("KEY_A".to_string(), "KEY_X".to_string());
        engine.load_layer_remap(1, &layer1_config).await.unwrap();

        // Activate layer 1 using hold mode
        engine.layer_manager.write().await.activate_hold_layer("test_device", 1).await.unwrap();

        // A should be remapped to X (layer 1 override)
        let result = engine.remap_layer_aware("test_device", Key::KEY_A).await;
        assert_eq!(result, Some(Key::KEY_X));

        // C should cascade to base layer (D)
        let result = engine.remap_layer_aware("test_device", Key::KEY_C).await;
        assert_eq!(result, Some(Key::KEY_D));
    }

    #[tokio::test]
    async fn test_remap_layer_aware_priority() {
        let engine = RemapEngine::new();

        // Base layer: A -> B
        let mut base_config = HashMap::new();
        base_config.insert("KEY_A".to_string(), "KEY_B".to_string());
        engine.load_layer_remap(0, &base_config).await.unwrap();

        // Layer 1: A -> X
        let mut layer1_config = HashMap::new();
        layer1_config.insert("KEY_A".to_string(), "KEY_X".to_string());
        engine.load_layer_remap(1, &layer1_config).await.unwrap();

        // Layer 2: A -> Y (highest priority)
        let mut layer2_config = HashMap::new();
        layer2_config.insert("KEY_A".to_string(), "KEY_Y".to_string());
        engine.load_layer_remap(2, &layer2_config).await.unwrap();

        // Activate both layers 1 and 2 (layer 2 has higher priority)
        let lm = engine.layer_manager.write().await;
        lm.activate_hold_layer("test_device", 1).await.unwrap();
        lm.activate_hold_layer("test_device", 2).await.unwrap();
        drop(lm);

        // With both layers active, A -> Y (highest priority wins)
        let result = engine.remap_layer_aware("test_device", Key::KEY_A).await;
        assert_eq!(result, Some(Key::KEY_Y));

        // Deactivate layer 2, should fall back to layer 1
        engine.layer_manager.write().await.deactivate_hold_layer("test_device", 2).await.unwrap();
        let result = engine.remap_layer_aware("test_device", Key::KEY_A).await;
        assert_eq!(result, Some(Key::KEY_X));

        // Deactivate layer 1, should fall back to base
        engine.layer_manager.write().await.deactivate_hold_layer("test_device", 1).await.unwrap();
        let result = engine.remap_layer_aware("test_device", Key::KEY_A).await;
        assert_eq!(result, Some(Key::KEY_B));
    }

    #[tokio::test]
    async fn test_remap_layer_aware_no_remap() {
        let engine = RemapEngine::new();

        // Load config into base layer only
        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "KEY_B".to_string());
        engine.load_layer_remap(0, &config).await.unwrap();

        // Activate layer 1 (which is empty)
        engine.layer_manager.write().await.activate_layer("test_device", 1).await;

        // Key not in any layer should return None (cascades through all layers)
        let result = engine.remap_layer_aware("test_device", Key::KEY_Z).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_process_event_layer_aware() {
        let engine = RemapEngine::new();

        // Load config into layer 1
        let mut config = HashMap::new();
        config.insert("KEY_A".to_string(), "KEY_X".to_string());
        engine.load_layer_remap(1, &config).await.unwrap();

        // Activate layer 1 using hold mode
        engine.layer_manager.write().await.activate_hold_layer("test_device", 1).await.unwrap();

        // Test press event (value = 1)
        let result = engine.process_event_layer_aware("test_device", Key::KEY_A, 1).await;
        assert_eq!(result, Some((Key::KEY_X, 1)));

        // Test release event (value = 0)
        let result = engine.process_event_layer_aware("test_device", Key::KEY_A, 0).await;
        assert_eq!(result, Some((Key::KEY_X, 0)));

        // Test repeat event (value = 2)
        let result = engine.process_event_layer_aware("test_device", Key::KEY_A, 2).await;
        assert_eq!(result, Some((Key::KEY_X, 2)));

        // Unmapped key should return None
        let result = engine.process_event_layer_aware("test_device", Key::KEY_Z, 1).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_layer_manager_accessor() {
        let engine = RemapEngine::new();

        // Get layer manager reference
        let layer_manager = engine.layer_manager();

        // Activate a layer through the accessor
        layer_manager.write().await.activate_layer("test_device", 1).await;

        // Verify layer is active
        let effective = layer_manager.read().await.get_effective_layer("test_device").await;
        assert_eq!(effective, 1);
    }

    #[tokio::test]
    async fn test_multiple_devices_independent_layers() {
        let engine = RemapEngine::new();

        // Load different configs for each layer
        let mut config1 = HashMap::new();
        config1.insert("KEY_A".to_string(), "KEY_X".to_string());
        engine.load_layer_remap(1, &config1).await.unwrap();

        let mut config2 = HashMap::new();
        config2.insert("KEY_A".to_string(), "KEY_Y".to_string());
        engine.load_layer_remap(2, &config2).await.unwrap();

        // Device 1 on layer 1, Device 2 on layer 2
        let lm = engine.layer_manager.write().await;
        lm.activate_hold_layer("device1", 1).await.unwrap();
        lm.activate_hold_layer("device2", 2).await.unwrap();
        drop(lm);

        // Device 1 should get X (from layer 1)
        let result1 = engine.remap_layer_aware("device1", Key::KEY_A).await;
        assert_eq!(result1, Some(Key::KEY_X));

        // Device 2 should get Y (from layer 2)
        let result2 = engine.remap_layer_aware("device2", Key::KEY_A).await;
        assert_eq!(result2, Some(Key::KEY_Y));

        // Device 3 (no layers active) should get None (base layer empty)
        let result3 = engine.remap_layer_aware("device3", Key::KEY_A).await;
        assert_eq!(result3, None);
    }

    #[tokio::test]
    async fn test_load_layer_remap_eager_validation() {
        let engine = RemapEngine::new();

        let mut config = HashMap::new();
        // First entry is valid
        config.insert("KEY_A".to_string(), "KEY_B".to_string());
        // Second entry has invalid key
        config.insert("KEY_C".to_string(), "invalid_key".to_string());

        // Should fail entirely - no partial load
        let result = engine.load_layer_remap(1, &config).await;
        assert!(result.is_err());

        // Layer 1 should still be empty (no partial load)
        let layer_1 = engine.get_layer_remaps(1).await.unwrap();
        assert!(layer_1.is_empty());
    }
}
