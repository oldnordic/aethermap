# Phase 17: Hotkey and Auto-Switch Persistence - Research

**Researched:** 2026-02-20
**Domain:** Rust IPC, YAML config persistence, GUI state management
**Confidence:** HIGH

## Summary

Phase 17 completes the persistence layer for global hotkeys and auto-profile switching rules. The infrastructure exists (IPC protocol definitions, GUI views, daemon managers) but the glue code connecting them is incomplete. The phase involves:

1. Implementing three stub IPC handlers in the daemon (`RegisterHotkey`, `ListHotkeys`, `RemoveHotkey`)
2. Connecting GUI auto-switch and hotkey rule changes to daemon persistence
3. Loading hotkey bindings from YAML configuration (currently hardcoded defaults)
4. Ensuring `GlobalHotkeyManager` and `AutoProfileSwitcher` reload from config on changes

**Primary recommendation:** Follow the existing `SetAnalogCalibration` / `GetAnalogCalibration` IPC pattern for hotkey and auto-switch persistence, leveraging `ConfigManager.save_device_profiles()` for YAML persistence.

## User Constraints

No CONTEXT.md exists for this phase. This is a fresh phase based on CONCERNS.md items.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde_yaml` | latest (workspace) | YAML serialization | Already used throughout config.rs for all persistence |
| `bincode` | latest (workspace) | IPC binary protocol | Already used for all IPC messages |
| `tokio` | 1.x | Async runtime | Async file I/O and IPC handlers |
| `aethermap-common` | workspace | Shared types | HotkeyBinding, AutoSwitchRule already defined |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tracing` | workspace | Structured logging | Use `debug!`, `info!`, `warn!` for operation tracking |
| `Arc<RwLock<T>>` | std | Thread-safe shared state | Already used for ConfigManager access patterns |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| YAML | JSON, TOML | YAML already established; human-editable config is a feature |
| bincode | protobuf, MessagePack | bincode already used; no need to add dependency |
| tokio fs | std fs with blocking | Async I/O required for IPC handlers (cannot block tokio executor) |

**Installation:** No new dependencies needed.

## Architecture Patterns

### Recommended IPC Handler Pattern

Follow the existing `SetAnalogCalibration` / `GetAnalogCalibration` pattern from `ipc.rs:1566-1615`:

```rust
// Pattern: Receive request -> update ConfigManager -> save YAML -> reload manager
Request::SetAnalogCalibration { device_id, layer_id, calibration } => {
    info!("Setting analog calibration: device={}, layer={}", device_id, layer_id);

    // 1. Update ConfigManager's device_profiles
    config_manager.set_analog_calibration(&device_id, layer_id, calibration.clone()).await;

    // 2. Save to YAML
    if let Err(e) = config_manager.save_device_profiles().await {
        warn!("Failed to save device profiles: {}", e);
        return Response::Error(format!("Failed to save: {}", e));
    }

    // 3. Reload component state
    if let Err(e) = analog_processor.load_calibrations(&device_id).await {
        warn!("Failed to reload calibrations: {}", e);
    }

    Response::AnalogCalibrationAck
}
```

### Recommended ConfigManager Integration Pattern

ConfigManager already has `hotkey_bindings` in `ExtendedDeviceRemapConfig` (line 552). Use:

```rust
// Add to ConfigManager impl (following save_analog_calibration pattern at ~2048)
pub async fn add_hotkey_binding(
    &self,
    device_id: &str,
    binding: HotkeyBinding,
) -> Result<(), RemapConfigError> {
    let mut config = self.load_device_profiles_extended().await?;

    let device_entry = config.devices.entry(device_id.to_string()).or_insert_with(|| {
        ExtendedDeviceRemapConfig {
            match_pattern: None,
            profiles: HashMap::new(),
            capabilities: None,
            analog_config: None,
            analog_calibration: HashMap::new(),
            led_config: None,
            hotkey_bindings: Vec::new(),
        }
    });

    device_entry.hotkey_bindings.push(binding);

    let yaml = serde_yaml::to_string(&config)?;
    fs::write(&self.device_profiles_path, yaml).await?;

    Ok(())
}
```

### GUI Persistence Pattern

GUI should send IPC request on save/delete (similar to analog calibration at `gui.rs:1082-1091`):

```rust
Message::SaveAutoSwitchRule => {
    // ... build rule from view state ...

    let device_id = view.device_id.clone();
    let rule = rule.clone();
    let socket_path = self.socket_path.clone();

    Command::perform(
        async move {
            let client = IpcClient::new(socket_path);
            // Use new SetAutoSwitchRules request
            client.set_auto_switch_rules(&device_id, view.rules).await
        },
        Message::AutoSwitchRuleSaved
    )
}
```

### Anti-Patterns to Avoid

- **Blocking IPC handlers:** Never use `std::fs::write` directly in IPC handler; use `tokio::fs::write` to avoid blocking executor
- **Partial state updates:** Don't modify GUI state without waiting for daemon ack; use command-perform-message pattern
- **Skipping reload:** After saving YAML, always reload the component (`GlobalHotkeyManager.load_bindings()` or `AutoProfileSwitcher.load_rules()`)
- **Hardcoding defaults:** `default_hotkey_bindings()` at `config.rs:127` should only be used for empty config, not as replacement for loading

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| YAML serialization | Manual string formatting | `serde_yaml::to_string/from_str` | Already handles HotkeyBinding, AutoSwitchRule serialization |
| IPC binary protocol | Custom message framing | `bincode::serialize/deserialize` | Length-prefix + bincode already established |
| File write atomicity | Manual temp file + rename | `tokio::fs::write` (atomic on Linux) | Simpler, sufficient for this use case |
| Thread-safe config access | Mutex, RwLock manual | `Arc<RwLock<T>>` pattern | Already used in ConfigManager |

**Key insight:** The serialization and persistence infrastructure is complete. Only the wiring between IPC handlers, ConfigManager, and component reload is missing.

## Common Pitfalls

### Pitfall 1: Saving to Wrong Config File

**What goes wrong:** Hotkeys saved to main `config.yaml` instead of `device_profiles.yaml`, or vice versa for auto-switch rules.

**Why it happens:** Both files exist, and it's unclear which stores what. `DaemonConfig` has `auto_switch_rules` but `ExtendedDeviceRemapConfig` has `hotkey_bindings`.

**How to avoid:**
- **Hotkey bindings** are **per-device** in `ExtendedDeviceRemapConfig.hotkey_bindings` (device_profiles.yaml)
- **Auto-switch rules** are **global** in `DaemonConfig.auto_switch_rules` (config.yaml)

**Warning signs:** Looking for `config.save_config()` vs `config.save_device_profiles()`.

### Pitfall 2: Forgetting to Reload After Save

**What goes wrong:** YAML is saved but `GlobalHotkeyManager` or `AutoProfileSwitcher` continue using old in-memory state.

**Why it happens:** ConfigManager and the hotkey/switch managers are separate; saving one doesn't update the other.

**How to avoid:** After `save_device_profiles()` or `save_config()`, call the appropriate load method:
```rust
// For hotkeys
global_hotkey_manager.load_bindings().await?;

// For auto-switch
auto_profile_switcher.load_rules(new_rules).await;
```

**Warning signs:** New bindings don't work until daemon restart.

### Pitfall 3: IPC Response Type Mismatch

**What goes wrong:** Handler returns wrong `Response` variant, causing GUI to panic or ignore the response.

**Why it happens:** Response enum has many similar variants (e.g., `HotkeyRegistered` vs `HotkeyList`).

**How to avoid:** Match response variant to request variant exactly:
```rust
Request::RegisterHotkey { .. } => Response::HotkeyRegistered { .. }
Request::ListHotkeys { .. } => Response::HotkeyList { .. }
Request::RemoveHotkey { .. } => Response::HotkeyRemoved { .. }
```

**Warning signs:** GUI message handler expects `Response::HotkeyList` but gets `Response::Error`.

### Pitfall 4: Empty Vec Serialization

**What goes wrong:** Empty hotkey_bindings or auto_switch_rules not saved because of `#[serde(skip_serializing_if = "Vec::is_empty")]`.

**Why it happens:** Serde skip attributes prevent saving empty collections, making it hard to distinguish "not configured" from "configured as empty".

**How to avoid:** The skip attributes are intentional (cleaner YAML). When loading, treat missing as empty:
```rust
let bindings = device_entry.hotkey_bindings.as_ref().cloned().unwrap_or_default();
```

**Warning signs:** Removing last hotkey doesn't persist; reload brings it back.

### Pitfall 5: Modifier Name Normalization

**What goes wrong:** Hotkey with "Ctrl" doesn't match "ctrl" during lookup.

**Why it happens:** Modifier names are case-insensitive in config but compared case-sensitively in code.

**How to avoid:** Always normalize modifier names using `normalize_modifier_name()` from `global_hotkey_manager.rs:73` before storing or comparing.

**Warning signs:** Hotkey works in YAML but not when registered via GUI.

## Code Examples

### ConfigManager: Add Hotkey Binding

**Source:** Based on `set_analog_calibration` at `config.rs:2022-2048`

```rust
/// Add a hotkey binding to a device's configuration
pub async fn add_hotkey_binding(
    &self,
    device_id: &str,
    binding: HotkeyBinding,
) -> Result<(), RemapConfigError> {
    let mut config = self.load_device_profiles_extended().await?;

    let device_entry = config.devices.entry(device_id.to_string())
        .or_insert_with(|| ExtendedDeviceRemapConfig {
            match_pattern: None,
            profiles: HashMap::new(),
            capabilities: None,
            analog_config: None,
            analog_calibration: HashMap::new(),
            led_config: None,
            hotkey_bindings: Vec::new(),
        });

    // Check for duplicate (same key + modifiers)
    let is_duplicate = device_entry.hotkey_bindings.iter().any(|existing| {
        existing.key == binding.key &&
        existing.modifiers == binding.modifiers
    });

    if is_duplicate {
        return Err(RemapConfigError::Validation {
            field: "hotkey".to_string(),
            message: "Hotkey with this key and modifiers already exists".to_string(),
        });
    }

    device_entry.hotkey_bindings.push(binding);

    // Write back to file
    let yaml = serde_yaml::to_string(&config).map_err(|e| RemapConfigError::WriteError {
        path: self.device_profiles_path.clone(),
        source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
    })?;

    fs::write(&self.device_profiles_path, yaml).await
        .map_err(|e| RemapConfigError::WriteError {
            path: self.device_profiles_path.clone(),
            source: e,
        })?;

    info!("Added hotkey binding for device {}", device_id);
    Ok(())
}
```

### IPC Handler: Register Hotkey

**Source:** Based on `SetAnalogCalibration` at `ipc.rs:1566-1615`

```rust
Request::RegisterHotkey { device_id, binding } => {
    debug!("RegisterHotkey: device={}, key={:?}, profile={}",
        device_id, binding.key, binding.profile_name);

    // Validate profile exists
    let profile_exists = config_manager.device_profiles.read().await
        .get(&device_id)
        .and_then(|profiles| profiles.get(&binding.profile_name))
        .is_some();

    if !profile_exists {
        warn!("RegisterHotkey: profile '{}' not found for device {}",
            binding.profile_name, device_id);
        return Response::Error(format!("Profile '{}' not found", binding.profile_name));
    }

    // Add to config
    match config_manager.add_hotkey_binding(&device_id, binding.clone()).await {
        Ok(()) => {
            // Reload hotkey manager
            if let Some(hotkey_manager) = global_hotkey_manager.as_ref() {
                // Need to add reload method that can reload specific device
                // For now, reload all
                if let Err(e) = hotkey_manager.write().await.load_bindings().await {
                    warn!("Failed to reload hotkey bindings: {}", e);
                }
            }

            Response::HotkeyRegistered {
                device_id,
                key: binding.key,
                modifiers: binding.modifiers,
            }
        }
        Err(e) => {
            warn!("RegisterHotkey failed: {}", e);
            Response::Error(format!("Failed to register hotkey: {}", e))
        }
    }
}
```

### IPC Handler: List Hotkeys

**Source:** Stub at `ipc.rs:1829-1838`

```rust
Request::ListHotkeys { device_id } => {
    debug!("ListHotkeys: device={}", device_id);

    match config_manager.get_hotkey_bindings(&device_id).await {
        Ok(bindings) => {
            Response::HotkeyList {
                device_id,
                bindings,
            }
        }
        Err(e) => {
            warn!("ListHotkeys failed: {}", e);
            Response::HotkeyList {
                device_id,
                bindings: Vec::new(),
            }
        }
    }
}
```

### ConfigManager: Get/Set Auto-Switch Rules (Global)

**Source:** Based on `DaemonConfig.auto_switch_rules` at `config.rs:773-775`

```rust
/// Set global auto-switch rules
pub async fn set_auto_switch_rules(
    &self,
    rules: Vec<AutoSwitchRule>,
) -> Result<(), RemapConfigError> {
    // Update in-memory config
    self.config.auto_switch_rules = rules.clone();

    // Save to main config file
    let content = serde_yaml::to_string(&self.config)
        .map_err(|e| RemapConfigError::WriteError {
            path: self.config_path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
        })?;

    fs::write(&self.config_path, content).await
        .map_err(|e| RemapConfigError::WriteError {
            path: self.config_path.clone(),
            source: e,
        })?;

    info!("Saved {} auto-switch rules", rules.len());
    Ok(())
}

/// Get global auto-switch rules
pub async fn get_auto_switch_rules(&self) -> Vec<AutoSwitchRule> {
    self.config.auto_switch_rules.clone()
}
```

### GUI: Send Auto-Switch Rules to Daemon

**Source:** Based on `Message::SaveHotkeyBinding` at `gui.rs:1024-1053`

```rust
Message::SaveAutoSwitchRule => {
    if let Some(view) = &self.auto_switch_view {
        let rules = view.rules.clone();
        let socket_path = self.socket_path.clone();

        // Create new task for IPC call
        Command::perform(
            async move {
                let client = IpcClient::new(socket_path);

                // Need to add this to Request enum:
                // SetAutoSwitchRules { rules: Vec<AutoSwitchRule> }
                let request = Request::SetAutoSwitchRules { rules };

                match client.send(&request).await {
                    Ok(Response::AutoSwitchRulesAck) => Ok(()),
                    Ok(Response::Error(msg)) => Err(msg),
                    Err(e) => Err(format!("IPC error: {}", e)),
                    _ => Err("Unexpected response".to_string()),
                }
            },
            |result| match result {
                Ok(()) => Message::AddNotification("Auto-switch rules saved".to_string(), false),
                Err(e) => Message::AddNotification(format!("Failed to save rules: {}", e), true),
            }
        )
    } else {
        Command::none()
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Hardcoded hotkey defaults | Load from device_profiles.yaml | Phase 17 (planned) | User-configurable hotkeys via GUI |
| Auto-switch rules in memory | Persist to config.yaml | Phase 17 (planned) | Rules survive daemon restart |
| Stub IPC handlers | Full implementation | Phase 17 (planned) | GUI can manage hotkeys/rules |

**Existing patterns to follow:**
- `SetAnalogCalibration`/`GetAnalogCalibration` (ipc.rs:1566+) - per-device config with reload
- `FocusChanged`/`FocusChangedAck` (ipc.rs:1789-1809) - async GUI daemon communication

## Open Questions

1. **Hotkey reload granularity:** `GlobalHotkeyManager.load_bindings()` reloads all bindings. Should we add per-device reload for efficiency?
   - **Recommendation:** No - hotkey bindings are small; full reload is simpler and sufficient.

2. **Auto-switch rule validation:** Should we validate that `profile_name` exists when adding an auto-switch rule?
   - **Recommendation:** Yes - follow the pattern in `RegisterHotkey` stub that checks profile existence.

3. **IPC request addition needed:** `SetAutoSwitchRules` doesn't exist in Request enum yet. Is this acceptable to add?
   - **Recommendation:** Yes - add `SetAutoSwitchRules { rules: Vec<AutoSwitchRule> }` and response `AutoSwitchRulesAck`.

4. **Global vs per-device hotkeys:** `HotkeyBinding.device_id` is `Option<String>`. Should `None` (global) work across all devices?
   - **Recommendation:** Yes - already supported by `GlobalHotkeyManager.trigger_action` at line 281-286.

## Sources

### Primary (HIGH confidence)

- **aethermap/aethermapd/src/config.rs** - Full file read, lines 1-2465
  - HotkeyBinding struct (line 40)
  - AutoSwitchRule struct (line 164)
  - ExtendedDeviceRemapConfig.hotkey_bindings (line 552)
  - DaemonConfig.auto_switch_rules (line 773)
  - save_config, load_config patterns (lines 974-971)
  - set_analog_calibration pattern (lines 2022-2048)

- **aethermap/aethermapd/src/global_hotkey_manager.rs** - Full file read, lines 1-502
  - load_bindings() stub (line 128-141)
  - Modifier normalization (lines 73-81)
  - Trigger action (lines 275-326)

- **aethermap/aethermapd/src/auto_profile_switcher.rs** - Full file read, lines 1-589
  - load_rules() implementation (line 86-89)
  - Rule matching logic (lines 209-253)

- **aethermap/aethermapd/src/ipc.rs** - Unimplemented handlers
  - RegisterHotkey stub (line 1812-1827)
  - ListHotkeys stub (line 1829-1838)
  - RemoveHotkey stub (line 1841-1854)
  - SetAnalogCalibration reference (line 1566-1615)

- **aethermap/aethermap-common/src/lib.rs** - IPC protocol definitions
  - HotkeyBinding struct (line 71-94)
  - Request enum (line 376-682) - RegisterHotkey, ListHotkeys, RemoveHotkey exist
  - Response enum (line 813-1094) - HotkeyRegistered, HotkeyList, HotkeyRemoved exist

- **aethermap/aethermap-gui/src/gui.rs** - GUI persistence TODOs
  - Auto-switch rule save TODO (line 918)
  - Auto-switch rule delete TODO (line 929)
  - Hotkey binding save TODO (line 1050)
  - Hotkey binding delete TODO (line 1061)

- **aethermap/aethermap-common/src/ipc_client.rs** - IPC client patterns
  - send() method (line 146-148)
  - set_analog_calibration helper (line 476-492)

### Secondary (MEDIUM confidence)

- **.planning/codebase/CONCERNS.md** - Technical debt and TODO items
  - Global Hotkey Configuration Loading (line 14-19)
  - Unimplemented IPC Handlers (line 28-35)
  - GUI Auto-Switch Rules Persistence (line 37-41)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All dependencies already in workspace
- Architecture: HIGH - IPC and persistence patterns well-established
- Pitfalls: HIGH - Code inspection reveals all integration points

**Research date:** 2026-02-20
**Valid until:** 60 days (stable architecture, unlikely to change)

## RESEARCH COMPLETE

All integration points identified. IPC protocol already defines request/response types. YAML serialization already works. Only the implementation of stub handlers and GUI-daemon wiring remains.
