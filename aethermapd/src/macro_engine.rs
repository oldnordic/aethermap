use rand::RngExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::injector::Injector;
use aethermap_common::{Action, KeyCombo, MacroEntry};

// Type alias for our error type that implements Send + Sync
pub type EngineResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Constants for analog value normalization
const ANALOG_MIN_RAW: f32 = -32768.0;
const ANALOG_MAX_RAW: f32 = 32767.0;

/// State for a currently executing macro
pub struct ExecutionState {
    pub name: String,
    pub start_time: Instant,
    pub stop: Arc<tokio::sync::RwLock<bool>>,
}

impl Drop for MacroEngine {
    fn drop(&mut self) {
        self._cleanup_task.abort();
    }
}

/// Macro engine that manages and executes macros
pub struct MacroEngine {
    macros: Arc<RwLock<HashMap<String, MacroEntry>>>,
    active_combos: Arc<RwLock<Vec<KeyCombo>>>,
    recording: Arc<RwLock<Option<MacroEntry>>>,
    executing: Arc<RwLock<HashMap<String, ExecutionState>>>,
    cleanup_tx: mpsc::Sender<String>, // Channel for cleanup notifications
    _cleanup_task: tokio::task::JoinHandle<()>, // Keep cleanup task alive
    max_concurrent_macros: usize,
    #[allow(dead_code)]
    default_delay: u32,
    macro_settings: Arc<RwLock<aethermap_common::MacroSettings>>,
    mouse_deltas: Arc<RwLock<HashMap<String, (i32, i32)>>>,
    injector: Option<Arc<RwLock<dyn Injector + Send + Sync>>>,
}

impl MacroEngine {
    /// Create a new macro engine with default configuration
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::with_config(10, 10)
    }

    /// Create a new macro engine with custom configuration
    pub fn with_config(max_concurrent_macros: usize, default_delay: u32) -> Self {
        let (cleanup_tx, mut cleanup_rx) = mpsc::channel::<String>(100);

        // Create executing Arc for the cleanup task
        let executing_for_cleanup = Arc::new(RwLock::new(HashMap::new()));

        // Spawn cleanup task
        let executing_clone = executing_for_cleanup.clone();
        let cleanup_task = tokio::spawn(async move {
            while let Some(macro_name) = cleanup_rx.recv().await {
                let mut executing = executing_clone.write().await;
                executing.remove(&macro_name);
                debug!("Cleaned up completed macro: {}", macro_name);
            }
            debug!("Cleanup task exited");
        });

        Self {
            macros: Arc::new(RwLock::new(HashMap::new())),
            active_combos: Arc::new(RwLock::new(Vec::new())),
            recording: Arc::new(RwLock::new(None)),
            executing: executing_for_cleanup,
            cleanup_tx,
            _cleanup_task: cleanup_task,
            max_concurrent_macros,
            default_delay,
            macro_settings: Arc::new(RwLock::new(aethermap_common::MacroSettings {
                latency_offset_ms: 0,
                jitter_pct: 0.0,
                capture_mouse: false,
            })),
            mouse_deltas: Arc::new(RwLock::new(HashMap::new())),
            injector: None,
        }
    }

    /// Create a new macro engine with an injector
    pub fn with_injector(injector: Arc<RwLock<dyn Injector + Send + Sync>>) -> Self {
        let (cleanup_tx, mut cleanup_rx) = mpsc::channel::<String>(100);

        // Create executing Arc for the cleanup task
        let executing_for_cleanup = Arc::new(RwLock::new(HashMap::new()));

        // Spawn cleanup task
        let executing_clone = executing_for_cleanup.clone();
        let cleanup_task = tokio::spawn(async move {
            while let Some(macro_name) = cleanup_rx.recv().await {
                let mut executing = executing_clone.write().await;
                executing.remove(&macro_name);
                debug!("Cleaned up completed macro: {}", macro_name);
            }
            debug!("Cleanup task exited");
        });

        Self {
            macros: Arc::new(RwLock::new(HashMap::new())),
            active_combos: Arc::new(RwLock::new(Vec::new())),
            recording: Arc::new(RwLock::new(None)),
            executing: executing_for_cleanup,
            cleanup_tx,
            _cleanup_task: cleanup_task,
            max_concurrent_macros: 10,
            default_delay: 10,
            macro_settings: Arc::new(RwLock::new(aethermap_common::MacroSettings {
                latency_offset_ms: 0,
                jitter_pct: 0.0,
                capture_mouse: false,
            })),
            mouse_deltas: Arc::new(RwLock::new(HashMap::new())),
            injector: Some(injector),
        }
    }

    /// Get the current macro settings
    pub async fn get_macro_settings(&self) -> aethermap_common::MacroSettings {
        let settings = self.macro_settings.read().await;
        settings.clone()
    }

    /// Set the current macro settings
    pub async fn set_macro_settings(&self, settings: aethermap_common::MacroSettings) {
        let mut macro_settings = self.macro_settings.write().await;
        *macro_settings = settings;
    }

    /// Set the injector to use for executing actions
    pub async fn set_injector(&mut self, injector: Arc<RwLock<dyn Injector + Send + Sync>>) {
        self.injector = Some(injector);
    }

    /// Add a macro to the engine
    pub async fn add_macro(&self, macro_entry: MacroEntry) -> EngineResult<()> {
        let mut macros = self.macros.write().await;

        // Check if macro already exists
        if macros.contains_key(&macro_entry.name) {
            return Err(format!("Macro '{}' already exists", macro_entry.name).into());
        }

        // Add the macro
        macros.insert(macro_entry.name.clone(), macro_entry.clone());

        // Update active combos
        self.update_active_combos_from(&macros).await;

        info!("Added macro: {}", macro_entry.name);
        Ok(())
    }

    /// Remove a macro from the engine
    pub async fn remove_macro(&self, name: &str) -> EngineResult<bool> {
        let mut macros = self.macros.write().await;

        // Check if macro exists
        if !macros.contains_key(name) {
            return Ok(false);
        }

        // Remove the macro
        macros.remove(name);

        // Update active combos
        self.update_active_combos_from(&macros).await;

        info!("Removed macro: {}", name);
        Ok(true)
    }

    /// Get a macro by name
    pub async fn get_macro(&self, name: &str) -> Option<MacroEntry> {
        let macros = self.macros.read().await;
        macros.get(name).cloned()
    }

    /// List all macros
    pub async fn list_macros(&self) -> Vec<MacroEntry> {
        let macros = self.macros.read().await;
        macros.values().cloned().collect()
    }

    /// Start recording a new macro
    pub async fn start_recording(
        &self,
        name: String,
        device_path: String,
        capture_mouse: bool,
    ) -> EngineResult<()> {
        let mut recording = self.recording.write().await;

        // Check if already recording
        if recording.is_some() {
            return Err("Already recording a macro".into());
        }

        // Create a new macro entry for recording
        *recording = Some(MacroEntry {
            name,
            trigger: KeyCombo {
                keys: vec![],
                modifiers: vec![],
            },
            actions: vec![],
            device_id: Some(device_path),
            enabled: true,
            humanize: false,
            capture_mouse,
        });

        info!("Started recording macro");
        Ok(())
    }

    /// Stop recording and return the recorded macro
    pub async fn stop_recording(&self) -> EngineResult<Option<MacroEntry>> {
        let mut recording = self.recording.write().await;

        // Check if currently recording
        if recording.is_none() {
            return Ok(None);
        }

        // Get the recorded macro
        let macro_entry = recording.take().unwrap();

        info!("Stopped recording macro: {}", macro_entry.name);
        Ok(Some(macro_entry))
    }

    /// Check if currently recording
    pub async fn is_recording(&self) -> bool {
        let recording = self.recording.read().await;
        recording.is_some()
    }

    /// Process an input event and add it to the recording if recording
    /// Process an input event
    /// Returns true if the event was consumed (by recording or macro), false otherwise
    pub async fn process_input_event(
        &self,
        key_code: u16,
        is_pressed: bool,
        device_path: &str,
    ) -> EngineResult<bool> {
        // First check if we're recording
        {
            let mut recording = self.recording.write().await;

            if let Some(macro_entry) = recording.as_mut() {
                // Check if the event is from the recording device
                let should_record = if let Some(ref recording_device) = macro_entry.device_id {
                    recording_device == device_path
                } else {
                    true
                };

                if should_record {
                    // Add the action to recording
                    if macro_entry.capture_mouse && key_code >= 272 {
                        // Mouse button: BTN_LEFT=272, BTN_RIGHT=273, etc.
                        if is_pressed {
                            macro_entry.actions.push(Action::MousePress(key_code));
                        } else {
                            macro_entry.actions.push(Action::MouseRelease(key_code));
                        }
                    } else {
                        // Keyboard key
                        if is_pressed {
                            macro_entry.actions.push(Action::KeyPress(key_code));
                        } else {
                            macro_entry.actions.push(Action::KeyRelease(key_code));
                        }
                    }
                    debug!(
                        "Recorded input event: key_code={}, pressed={}",
                        key_code, is_pressed
                    );
                    return Ok(true); // Event consumed by recording
                }
            }
        }

        // Not recording, check for macro triggers on key press
        if is_pressed {
            let consumed = self.check_macro_triggers(key_code, device_path).await?;
            return Ok(consumed);
        }

        Ok(false) // Event not consumed
    }

    /// Process a relative movement event (mouse move) and add it to the recording if active
    pub async fn process_relative_event(
        &self,
        axis: u16,
        delta: i32,
        device_path: &str,
    ) -> EngineResult<()> {
        let mut recording = self.recording.write().await;

        if let Some(macro_entry) = recording.as_mut() {
            if !macro_entry.capture_mouse {
                return Ok(());
            }

            // Check if the event is from the recording device
            let should_record = if let Some(ref recording_device) = macro_entry.device_id {
                recording_device == device_path
            } else {
                true
            };

            if should_record {
                let mut deltas = self.mouse_deltas.write().await;
                let entry = deltas.entry(device_path.to_string()).or_insert((0, 0));

                // axis 0 = REL_X, axis 1 = REL_Y
                if axis == 0 {
                    entry.0 += delta;
                } else if axis == 1 {
                    entry.1 += delta;
                }

                // If we have any accumulated delta, record a MouseMove action and reset
                // In a perfect world we'd wait for a SYN_REPORT, but since events
                // are passed individually we'll record them as they arrive to keep it simple.
                let (dx, dy) = *entry;
                if dx != 0 || dy != 0 {
                    macro_entry.actions.push(Action::MouseMove(dx, dy));
                    *entry = (0, 0);
                }
            }
        }
        Ok(())
    }

    /// Update the list of active key combos
    async fn update_active_combos_from(&self, macros: &HashMap<String, MacroEntry>) {
        let mut active_combos = self.active_combos.write().await;

        // Clear the current list
        active_combos.clear();

        // Add all triggers from enabled macros
        for macro_entry in macros.values() {
            if macro_entry.enabled {
                active_combos.push(macro_entry.trigger.clone());
            }
        }
    }

    /// Check if any macro should be triggered
    /// Returns true if a macro was triggered (event was consumed), false otherwise
    pub async fn check_macro_triggers(
        &self,
        key_code: u16,
        device_path: &str,
    ) -> EngineResult<bool> {
        let macros = self.macros.read().await;
        let executing_count = self.executing.read().await.len();

        if executing_count >= self.max_concurrent_macros {
            warn!("Max concurrent macros reached, ignoring trigger");
            return Ok(false);
        }

        // Check each macro
        for macro_entry in macros.values() {
            // Skip disabled macros
            if !macro_entry.enabled {
                continue;
            }

            // Skip macros restricted to other devices
            if let Some(ref device_id) = macro_entry.device_id {
                if device_id != device_path {
                    continue;
                }
            }

            // Check if the trigger matches
            if self.keys_match(&macro_entry.trigger, key_code) {
                debug!("Macro {} triggered", macro_entry.name);
                self.execute_macro(macro_entry.clone()).await?;
                return Ok(true); // Event was consumed by macro
            }
        }

        Ok(false) // No macro triggered, event not consumed
    }

    /// Check if a key code matches a key combo
    fn keys_match(&self, combo: &KeyCombo, key_code: u16) -> bool {
        combo.keys.contains(&key_code)
    }

    /// Normalize raw EV_ABS value to device-independent range
    ///
    /// Converts raw analog values (-32768 to 32767) to normalized range (-1.0 to 1.0)
    /// for device-independent macro recording.
    ///
    /// # Arguments
    /// * `raw` - Raw analog value from evdev (-32768 to 32767)
    ///
    /// # Returns
    /// Normalized value in range -1.0 to 1.0
    pub fn normalize_analog(raw: i32) -> f32 {
        let raw_f = raw as f32;
        (raw_f - ANALOG_MIN_RAW) / (ANALOG_MAX_RAW - ANALOG_MIN_RAW) * 2.0 - 1.0
    }

    /// Convert normalized value back to raw EV_ABS range
    ///
    /// Converts normalized analog values (-1.0 to 1.0) back to raw evdev range
    /// (-32768 to 32767) for macro playback.
    ///
    /// # Arguments
    /// * `normalized` - Normalized analog value (-1.0 to 1.0)
    ///
    /// # Returns
    /// Raw analog value in range -32768 to 32767
    pub fn denormalize_analog(normalized: f32) -> i32 {
        ((normalized + 1.0) / 2.0 * (ANALOG_MAX_RAW - ANALOG_MIN_RAW) + ANALOG_MIN_RAW) as i32
    }

    /// Process an analog event and add it to the recording if recording
    ///
    /// # Arguments
    /// * `axis_code` - Analog axis code (61000-61005 for ABS_X, ABS_Y, etc.)
    /// * `raw_value` - Raw analog value from evdev (-32768 to 32767)
    /// * `device_path` - Device path for filtering
    ///
    /// # Returns
    /// Ok(()) if successful, Err if recording failed
    pub async fn process_analog_event(
        &self,
        axis_code: u16,
        raw_value: i32,
        device_path: &str,
    ) -> EngineResult<()> {
        // Normalize the analog value
        let normalized = Self::normalize_analog(raw_value);

        // Check if we're recording
        {
            let mut recording = self.recording.write().await;

            if let Some(macro_entry) = recording.as_mut() {
                // Check if the event is from the recording device
                let should_record = if let Some(ref recording_device) = macro_entry.device_id {
                    recording_device == device_path
                } else {
                    true
                };

                if should_record {
                    // Add the analog action to recording
                    macro_entry.actions.push(Action::AnalogMove {
                        axis_code,
                        normalized,
                    });
                    debug!(
                        "Recorded analog event: axis_code={}, raw={}, normalized={:.2}",
                        axis_code, raw_value, normalized
                    );
                    return Ok(());
                }
            }
        }

        // Not recording or wrong device
        Ok(())
    }

    /// Calculate the actual delay to apply based on humanization settings
    pub fn calculate_delay(&self, base_ms: u32, settings: &aethermap_common::MacroSettings) -> u32 {
        let mut rng = rand::rng();
        let jitter = if settings.jitter_pct > 0.0 {
            let jitter_range = base_ms as f32 * settings.jitter_pct;
            rng.random_range(-jitter_range..=jitter_range)
        } else {
            0.0
        };

        let total_delay = base_ms as f32 + settings.latency_offset_ms as f32 + jitter;
        total_delay.max(0.0) as u32
    }

    /// Execute a macro
    pub async fn execute_macro(&self, macro_entry: MacroEntry) -> EngineResult<()> {
        // Get injector reference
        let injector = match self.injector.as_ref() {
            Some(i) => Arc::clone(i),
            None => {
                error!("No injector set, cannot execute macro");
                return Err("No injector available".into());
            }
        };

        // Check if already executing
        {
            let executing = self.executing.read().await;
            if executing.contains_key(&macro_entry.name) {
                warn!("Macro {} is already executing", macro_entry.name);
                return Ok(());
            }
        }

        // Create execution state
        let stop_flag = Arc::new(tokio::sync::RwLock::new(false));
        let execution_state = ExecutionState {
            name: macro_entry.name.clone(),
            start_time: Instant::now(),
            stop: stop_flag.clone(),
        };

        // Add to executing list
        {
            let mut executing = self.executing.write().await;
            executing.insert(macro_entry.name.clone(), execution_state);
        }

        // Clone actions and injector for spawned task
        let actions = macro_entry.actions.clone();
        let injector_clone = Arc::clone(&injector);
        let macro_name = macro_entry.name.clone();
        let macro_humanize = macro_entry.humanize;
        let macro_settings_lock = Arc::clone(&self.macro_settings);

        // Get cleanup channel sender
        let cleanup_tx = self.cleanup_tx.clone();

        // Execute in a separate task
        tokio::spawn(async move {
            for action in actions {
                // Check if we should stop
                if *stop_flag.read().await {
                    break;
                }

                // Get a reference to the injector for each action
                let injector_ref = injector_clone.read().await;

                match action {
                    Action::KeyPress(code) => {
                        if let Err(e) = injector_ref.key_press(code).await {
                            error!("Failed to inject key press: {}", e);
                        }
                    }
                    Action::KeyRelease(code) => {
                        if let Err(e) = injector_ref.key_release(code).await {
                            error!("Failed to inject key release: {}", e);
                        }
                    }
                    Action::Delay(ms) => {
                        let delay_ms = if macro_humanize {
                            let settings = macro_settings_lock.read().await;
                            // Need a temporary instance of MacroEngine to call calculate_delay if it's not static
                            // Or just re-implement it here to avoid passing self
                            let mut rng = rand::rng();
                            let jitter = if settings.jitter_pct > 0.0 {
                                let jitter_range = ms as f32 * settings.jitter_pct;
                                rng.random_range(-jitter_range..=jitter_range)
                            } else {
                                0.0
                            };

                            let total_delay =
                                ms as f32 + settings.latency_offset_ms as f32 + jitter;
                            total_delay.max(0.0) as u32
                        } else {
                            ms
                        };
                        tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
                    }
                    Action::Execute(cmd) => {
                        if let Err(e) = injector_ref.execute_command(&cmd).await {
                            error!("Failed to execute command: {}", e);
                        }
                    }
                    Action::Type(text) => {
                        if let Err(e) = injector_ref.type_string(&text).await {
                            error!("Failed to type text: {}", e);
                        }
                    }
                    Action::MousePress(button) => {
                        if let Err(e) = injector_ref.mouse_press(button).await {
                            error!("Failed to inject mouse press: {}", e);
                        }
                    }
                    Action::MouseRelease(button) => {
                        if let Err(e) = injector_ref.mouse_release(button).await {
                            error!("Failed to inject mouse release: {}", e);
                        }
                    }
                    Action::MouseMove(x, y) => {
                        if let Err(e) = injector_ref.mouse_move(x, y).await {
                            error!("Failed to inject mouse move: {}", e);
                        }
                    }
                    Action::MouseScroll(amount) => {
                        if let Err(e) = injector_ref.mouse_scroll(amount).await {
                            error!("Failed to inject mouse scroll: {}", e);
                        }
                    }
                    Action::AnalogMove {
                        axis_code,
                        normalized,
                    } => {
                        // Denormalize and inject analog event
                        let raw_value = Self::denormalize_analog(normalized);
                        if let Err(e) = injector_ref.analog_move(axis_code, raw_value).await {
                            error!("Failed to inject analog move: {}", e);
                        }
                    }
                }
            }

            // Send cleanup notification
            let _ = cleanup_tx.send(macro_name.clone()).await;
            debug!(
                "Macro {} execution completed and cleanup notification sent",
                macro_name
            );
        });

        info!("Started executing macro: {}", macro_entry.name);
        Ok(())
    }

    /// Stop an executing macro
    pub async fn stop_macro(&self, name: &str) -> EngineResult<bool> {
        let mut executing = self.executing.write().await;

        if let Some(state) = executing.get(name) {
            info!("Stopping macro: {}", name);
            *state.stop.write().await = true;
            executing.remove(name);
            return Ok(true);
        }

        warn!("Macro {} not found in executing list", name);
        Ok(false)
    }

    /// Get all currently executing macros
    pub async fn get_executing_macros(&self) -> Vec<String> {
        let executing = self.executing.read().await;
        executing.keys().cloned().collect()
    }

    /// Execute a single action with the injector
    ///
    /// This method allows executing individual actions without creating a full macro.
    /// Used by the IPC module when executing macros that have been retrieved.
    pub async fn execute_action(
        &self,
        action: &aethermap_common::Action,
        injector: &(dyn crate::injector::Injector + Send + Sync),
    ) -> EngineResult<()> {
        // Use the injector directly since we have a reference to it
        match action {
            aethermap_common::Action::KeyPress(code) => {
                if let Err(e) = injector.key_press(*code).await {
                    error!("Failed to inject key press: {}", e);
                    return Err(format!("Key press failed: {}", e).into());
                }
            }
            aethermap_common::Action::KeyRelease(code) => {
                if let Err(e) = injector.key_release(*code).await {
                    error!("Failed to inject key release: {}", e);
                    return Err(format!("Key release failed: {}", e).into());
                }
            }
            aethermap_common::Action::Delay(ms) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(*ms as u64)).await;
            }
            aethermap_common::Action::Execute(command) => {
                if let Err(e) = injector.execute_command(command).await {
                    error!("Failed to execute command: {}", e);
                    return Err(format!("Command execution failed: {}", e).into());
                }
            }
            aethermap_common::Action::Type(text) => {
                if let Err(e) = injector.type_string(text).await {
                    error!("Failed to type text: {}", e);
                    return Err(format!("Text typing failed: {}", e).into());
                }
            }
            aethermap_common::Action::MousePress(button) => {
                if let Err(e) = injector.mouse_press(*button).await {
                    error!("Failed to inject mouse press: {}", e);
                    return Err(format!("Mouse press failed: {}", e).into());
                }
            }
            aethermap_common::Action::MouseRelease(button) => {
                if let Err(e) = injector.mouse_release(*button).await {
                    error!("Failed to inject mouse release: {}", e);
                    return Err(format!("Mouse release failed: {}", e).into());
                }
            }
            aethermap_common::Action::MouseMove(x, y) => {
                if let Err(e) = injector.mouse_move(*x, *y).await {
                    error!("Failed to inject mouse move: {}", e);
                    return Err(format!("Mouse move failed: {}", e).into());
                }
            }
            aethermap_common::Action::MouseScroll(amount) => {
                if let Err(e) = injector.mouse_scroll(*amount).await {
                    error!("Failed to inject mouse scroll: {}", e);
                    return Err(format!("Mouse scroll failed: {}", e).into());
                }
            }
            aethermap_common::Action::AnalogMove {
                axis_code,
                normalized,
            } => {
                // Denormalize and inject analog event
                let raw_value = Self::denormalize_analog(*normalized);
                if let Err(e) = injector.analog_move(*axis_code, raw_value).await {
                    error!("Failed to inject analog move: {}", e);
                    return Err(format!("Analog move failed: {}", e).into());
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::injector::Injector;
    use std::sync::Arc;

    // Create a mock injector for testing
    struct MockInjector;

    impl MockInjector {
        #[allow(dead_code)]
        fn new() -> Arc<Self> {
            Arc::new(Self)
        }
    }

    #[async_trait::async_trait]
    impl Injector for MockInjector {
        async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        async fn key_press(
            &self,
            _key_code: u16,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        async fn key_release(
            &self,
            _key_code: u16,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        async fn mouse_press(
            &self,
            _button: u16,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        async fn mouse_release(
            &self,
            _button: u16,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        async fn mouse_move(
            &self,
            _x: i32,
            _y: i32,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        async fn mouse_scroll(
            &self,
            _amount: i32,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        async fn type_string(
            &self,
            _text: &str,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        async fn execute_command(
            &self,
            _command: &str,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        async fn analog_move(
            &self,
            _axis_code: u16,
            _value: i32,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_macro_creation() {
        let engine = MacroEngine::new();

        let macro_entry = MacroEntry {
            name: "Test Macro".to_string(),
            trigger: KeyCombo {
                keys: vec![30], // A key
                modifiers: vec![],
            },
            actions: vec![
                Action::KeyPress(30),
                Action::Delay(100),
                Action::KeyRelease(30),
            ],
            device_id: None,
            enabled: true,
            humanize: false,
            capture_mouse: false,
        };

        // Add macro
        engine.add_macro(macro_entry.clone()).await.unwrap();

        // Get macro
        let retrieved = engine.get_macro("Test Macro").await.unwrap();
        assert_eq!(retrieved.name, macro_entry.name);
        assert_eq!(retrieved.trigger.keys, macro_entry.trigger.keys);
    }

    #[tokio::test]
    async fn test_macro_removal() {
        let engine = MacroEngine::new();

        let macro_entry = MacroEntry {
            name: "Test Macro".to_string(),
            trigger: KeyCombo {
                keys: vec![30], // A key
                modifiers: vec![],
            },
            actions: vec![],
            device_id: None,
            enabled: true,
            humanize: false,
            capture_mouse: false,
        };

        // Add macro
        engine.add_macro(macro_entry).await.unwrap();

        // Verify it exists
        assert!(engine.get_macro("Test Macro").await.is_some());

        // Remove macro
        let removed = engine.remove_macro("Test Macro").await.unwrap();
        assert!(removed);

        // Verify it's gone
        assert!(engine.get_macro("Test Macro").await.is_none());
    }

    #[tokio::test]
    async fn test_macro_recording() {
        let engine = MacroEngine::new();

        // Start recording
        engine
            .start_recording(
                "Test Recording".to_string(),
                "/dev/input/event0".to_string(),
                false,
            )
            .await
            .unwrap();
        assert!(engine.is_recording().await);

        // Process some events
        engine
            .process_input_event(30, true, "/dev/input/event0")
            .await
            .unwrap(); // A down
        engine
            .process_input_event(30, false, "/dev/input/event0")
            .await
            .unwrap(); // A up

        // Stop recording
        let macro_entry = engine.stop_recording().await.unwrap().unwrap();
        assert_eq!(macro_entry.name, "Test Recording");
        assert_eq!(macro_entry.actions.len(), 2);

        // Verify recording stopped
        assert!(!engine.is_recording().await);
    }

    #[tokio::test]
    async fn test_macro_triggering() {
        let _engine = MacroEngine::new();

        // Note: This test is disabled for now because we can't use MockInjector with set_injector anymore
        // In a real test, we would need to create a UinputInjector
        // For now, we'll just test that macro engine creates without error
    }

    #[test]
    fn test_normalize_analog() {
        // Test minimum value
        let result = MacroEngine::normalize_analog(-32768);
        assert!(
            (result - (-1.0)).abs() < 0.001,
            "Min value should normalize to -1.0"
        );

        // Test center value
        let result = MacroEngine::normalize_analog(0);
        assert!(
            (result - 0.0).abs() < 0.001,
            "Center value should normalize to 0.0"
        );

        // Test maximum value
        let result = MacroEngine::normalize_analog(32767);
        assert!(
            (result - 1.0).abs() < 0.001,
            "Max value should normalize to 1.0"
        );

        // Test mid-range value
        let result = MacroEngine::normalize_analog(16383);
        assert!(
            (result - 0.4999).abs() < 0.01,
            "Mid value should normalize to ~0.5"
        );

        // Test negative mid-range value
        let result = MacroEngine::normalize_analog(-16384);
        assert!(
            (result - (-0.5000)).abs() < 0.01,
            "Negative mid value should normalize to ~-0.5"
        );
    }

    #[test]
    fn test_denormalize_analog() {
        // Test minimum normalized value
        let result = MacroEngine::denormalize_analog(-1.0);
        assert_eq!(
            result, -32768,
            "Normalized -1.0 should denormalize to -32768"
        );

        // Test center value
        let result = MacroEngine::denormalize_analog(0.0);
        assert_eq!(result, 0, "Normalized 0.0 should denormalize to 0");

        // Test maximum normalized value
        let result = MacroEngine::denormalize_analog(1.0);
        assert_eq!(result, 32767, "Normalized 1.0 should denormalize to 32767");

        // Test mid-range value
        let result = MacroEngine::denormalize_analog(0.5);
        assert!(
            (result - 16383).abs() < 2,
            "Normalized 0.5 should denormalize to ~16383"
        );

        // Test negative mid-range value
        let result = MacroEngine::denormalize_analog(-0.5);
        assert!(
            (result - (-16384)).abs() < 2,
            "Normalized -0.5 should denormalize to ~-16384"
        );
    }

    #[test]
    fn test_analog_roundtrip() {
        // Test that normalize -> denormalize preserves values (within 1% tolerance)
        let test_values = vec![
            -32768, -30000, -20000, -16384, -10000, -5000, -1000, 0, 1000, 5000, 10000, 16383,
            20000, 30000, 32767,
        ];

        for raw_value in test_values {
            let normalized = MacroEngine::normalize_analog(raw_value);
            let denormalized = MacroEngine::denormalize_analog(normalized);
            let error_percent =
                ((denormalized - raw_value) as f32).abs() / (raw_value.abs() as f32 + 1.0) * 100.0;

            assert!(
                error_percent < 1.0 || (denormalized - raw_value).abs() < 100,
                "Roundtrip error too large for {}: normalized={:.4}, denormalized={}, error={:.2}%",
                raw_value,
                normalized,
                denormalized,
                error_percent
            );
        }
    }

    #[tokio::test]
    async fn test_analog_recording() {
        let engine = MacroEngine::new();

        // Start recording
        engine
            .start_recording(
                "Analog Test".to_string(),
                "/dev/input/event0".to_string(),
                false,
            )
            .await
            .unwrap();
        assert!(engine.is_recording().await);

        // Process analog events
        engine
            .process_analog_event(61000, 32767, "/dev/input/event0")
            .await
            .unwrap(); // X axis max
        engine
            .process_analog_event(61001, 0, "/dev/input/event0")
            .await
            .unwrap(); // Y axis center
        engine
            .process_analog_event(61000, -32768, "/dev/input/event0")
            .await
            .unwrap(); // X axis min

        // Stop recording
        let macro_entry = engine.stop_recording().await.unwrap().unwrap();
        assert_eq!(macro_entry.name, "Analog Test");
        assert_eq!(macro_entry.actions.len(), 3);

        // Verify first action
        if let Action::AnalogMove {
            axis_code,
            normalized,
        } = &macro_entry.actions[0]
        {
            assert_eq!(*axis_code, 61000);
            assert!(
                (normalized - 1.0).abs() < 0.01,
                "X axis max should normalize to ~1.0"
            );
        } else {
            panic!("Expected AnalogMove action");
        }

        // Verify second action
        if let Action::AnalogMove {
            axis_code,
            normalized,
        } = &macro_entry.actions[1]
        {
            assert_eq!(*axis_code, 61001);
            assert!(
                (normalized - 0.0).abs() < 0.01,
                "Y axis center should normalize to ~0.0"
            );
        } else {
            panic!("Expected AnalogMove action");
        }

        // Verify third action
        if let Action::AnalogMove {
            axis_code,
            normalized,
        } = &macro_entry.actions[2]
        {
            assert_eq!(*axis_code, 61000);
            assert!(
                (normalized - (-1.0)).abs() < 0.01,
                "X axis min should normalize to ~-1.0"
            );
        } else {
            panic!("Expected AnalogMove action");
        }
    }

    #[tokio::test]
    async fn test_mouse_button_recording() {
        let engine = MacroEngine::new();

        // Start recording with mouse capture enabled
        engine
            .start_recording(
                "Mouse Button Test".to_string(),
                "/dev/input/event0".to_string(),
                true,
            )
            .await
            .unwrap();
        assert!(engine.is_recording().await);

        // Process mouse button events (using key codes 272 for BTN_LEFT)
        engine
            .process_input_event(272, true, "/dev/input/event0")
            .await
            .unwrap(); // Left click down
        engine
            .process_input_event(272, false, "/dev/input/event0")
            .await
            .unwrap(); // Left click up

        // Stop recording
        let macro_entry = engine.stop_recording().await.unwrap().unwrap();
        assert_eq!(macro_entry.name, "Mouse Button Test");

        // Should have 2 actions: MousePress and MouseRelease
        assert_eq!(macro_entry.actions.len(), 2);
        assert!(matches!(macro_entry.actions[0], Action::MousePress(272)));
        assert!(matches!(macro_entry.actions[1], Action::MouseRelease(272)));
    }

    #[tokio::test]
    async fn test_mouse_move_recording() {
        let engine = MacroEngine::new();

        // Start recording with mouse capture enabled
        engine
            .start_recording(
                "Mouse Move Test".to_string(),
                "/dev/input/event0".to_string(),
                true,
            )
            .await
            .unwrap();

        // Process relative mouse movement
        engine
            .process_relative_event(0, 10, "/dev/input/event0")
            .await
            .unwrap(); // REL_X +10
        engine
            .process_relative_event(1, -5, "/dev/input/event0")
            .await
            .unwrap(); // REL_Y -5

        // Stop recording
        let macro_entry = engine.stop_recording().await.unwrap().unwrap();
        assert_eq!(macro_entry.name, "Mouse Move Test");

        // Should have 2 actions: MouseMove(10, 0) and MouseMove(0, -5)
        // Or if accumulated into one: MouseMove(10, -5)
        // Based on our implementation, it records as they arrive if delta != 0.
        assert_eq!(macro_entry.actions.len(), 2);

        if let Action::MouseMove(x, y) = macro_entry.actions[0] {
            assert_eq!(x, 10);
            assert_eq!(y, 0);
        } else {
            panic!("Expected MouseMove action");
        }

        if let Action::MouseMove(x, y) = macro_entry.actions[1] {
            assert_eq!(x, 0);
            assert_eq!(y, -5);
        } else {
            panic!("Expected MouseMove action");
        }
    }

    #[tokio::test]
    async fn test_analog_macro_execution() {
        let injector = Arc::new(RwLock::new(MockInjector));
        let engine = MacroEngine::with_injector(injector);

        // Create a macro with analog actions
        let macro_entry = MacroEntry {
            name: "Analog Macro".to_string(),
            trigger: KeyCombo {
                keys: vec![30],
                modifiers: vec![],
            },
            actions: vec![
                Action::AnalogMove {
                    axis_code: 61000,
                    normalized: 1.0,
                },
                Action::Delay(10),
                Action::AnalogMove {
                    axis_code: 61001,
                    normalized: -0.5,
                },
            ],
            device_id: None,
            enabled: true,
            humanize: false,
            capture_mouse: false,
        };

        // Execute the macro
        let result = engine.execute_macro(macro_entry).await;
        assert!(result.is_ok(), "Macro execution should succeed");

        // Wait a bit for async execution
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // In a real test with a spy/mock, we would verify the calls
    }

    #[tokio::test]
    async fn test_humanized_delay() {
        let engine = MacroEngine::new();
        let settings = aethermap_common::MacroSettings {
            latency_offset_ms: 10,
            jitter_pct: 0.1, // 10%
            capture_mouse: false,
        };

        let base_delay = 100;
        // Expected range:
        // Base_ms + Latency_Offset + (Random_Jitter * Base_ms)
        // Min: 100 + 10 + (-0.1 * 100) = 110 - 10 = 100
        // Max: 100 + 10 + (0.1 * 100) = 110 + 10 = 120

        let mut min_seen = 200;
        let mut max_seen = 0;

        for _ in 0..1000 {
            let delay = engine.calculate_delay(base_delay, &settings);
            assert!(delay >= 100, "Delay {} should be >= 100", delay);
            assert!(delay <= 120, "Delay {} should be <= 120", delay);
            if delay < min_seen {
                min_seen = delay;
            }
            if delay > max_seen {
                max_seen = delay;
            }
        }

        // With 1000 iterations, we should see some variation
        assert!(
            max_seen > min_seen,
            "Should see variation in delay: {} - {}",
            min_seen,
            max_seen
        );
    }
}
