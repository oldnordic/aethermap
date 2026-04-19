# Phase 2: Per-Device Profiles & IPC - Research

**Researched:** 2026-02-17
**Domain:** Linux Input Remapping with Per-Device Configuration, Rust async IPC
**Confidence:** HIGH

## Summary

Phase 2 extends the Phase 1 RemapEngine from global remaps to per-device profiles with runtime switching via IPC. The implementation requires: (1) extending the YAML config format to support per-device and per-profile remap sections, (2) modifying RemapEngine to support multiple device-specific remap tables, (3) adding IPC request/response types for querying active remaps and switching profiles, and (4) integrating profile activation into DeviceManager's event processing loop.

**Primary recommendation:** Use a hierarchical config structure (devices -> [device_id] -> profiles -> [profile_name] -> remaps), store per-device RemapEngine instances in DeviceManager, add IPC messages GetActiveRemaps/ListRemapProfiles/ActivateProfile/DeactivateProfile, and switch profiles atomically via Arc::clone of pre-validated remap tables.

**Key technical insight:** The existing codebase already has profile infrastructure for macros (ConfigManager::save_profile, Profile struct). Reuse this pattern for remaps: a RemapProfile struct with device_id, profile_name, and remaps HashMap. Profile switching becomes an atomic pointer swap of Arc<RemapTable> rather than rebuilding HashMaps.

---

## Standard Stack

### Core Dependencies (Already in Project)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde_yaml` | 0.9 | Extended YAML config parsing | Already used for config; supports nested structures |
| `tokio` | 1 (full features) | Async runtime for IPC | Project standard; required for async IPC handlers |
| `tracing` | 0.1 | Structured logging | Already used throughout codebase |
| `serde` | 1 (derive) | Serialization/deserialization | Required for new IPC messages |
| `evdev` | 0.12 | Device identification | DeviceInfo already uses vendor_id/product_id for matching |

### Supporting Dependencies

| Library | Purpose | When to Use |
|---------|---------|-------------|
| `std::collections::HashMap` | O(1) device/profile lookup | Core storage: device_id -> (profile_name -> remaps) |
| `Arc<RwLock<T>>` | Thread-safe shared state | RemapEngine instances accessed from event loop |
| `Arc<Clone>` | Atomic pointer swaps for profile switching | Zero-copy profile activation |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hierarchical YAML | Flat YAML with device:profile:remap naming | Hierarchical is clearer; flat requires parsing key prefixes |
| Per-device RemapEngine | Single RemapEngine with device routing | Per-device is simpler (no routing logic); single engine requires device_id on every lookup |
| Arc pointer swap | Rebuild HashMap on switch | Arc swap is O(1); rebuild requires parsing all keys |

**Installation:** No new dependencies required. All libraries already in project.

---

## Architecture Patterns

### Recommended Project Structure

```
aethermapd/src/
├── remap_engine.rs     # UPDATE: Add RemapProfile struct, device-specific storage
├── config.rs           # UPDATE: Extended YAML parsing for device profiles
├── ipc.rs              # UPDATE: New IPC request/response types
├── device.rs           # UPDATE: Per-device RemapEngine storage and profile lookup
└── lib.rs              # UPDATE: Export RemapProfile type
```

### Pattern 1: Per-Device Profile Configuration

**What:** A hierarchical YAML structure where each device has named profiles containing remap rules. Device matching uses vendor_id:product_id or phys path patterns.

**When to use:** Phase 2 per-device remaps where users want different key layouts for different keyboards.

**Example:**

```yaml
# /etc/aethermapd/remaps.yaml
devices:
  # Match by vendor:product ID (Razer BlackWidow)
  "1532:0220":
    profiles:
      default:
        name: "Default Layout"
        remaps:
          capslock: leftctrl
          esc: grave
      gaming:
        name: "Gaming Profile"
        remaps:
          capslock: esc
          a: b

  # Match by device name pattern
  "Keyboard:*":
    profiles:
      default:
        remaps:
          ctrl: alt
```

**Source:** Based on existing Profile struct pattern in `/home/feanor/Projects/remapper_rs/aethermap/aethermap-common/src/lib.rs:227-231` and macro profile loading in `config.rs:414-465`.

### Pattern 2: RemapProfile Struct with Device Association

**What:** A new struct parallel to MacroEntry's Profile, storing device identifier, profile name, and remap rules. Loaded from YAML and stored in ConfigManager.

**When to use:** Storing and retrieving per-device remap configurations.

**Example:**

```rust
/// Remap configuration for a specific device profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemapProfile {
    /// Device identifier (vendor:product or phys path pattern)
    pub device_id: String,
    /// Profile name (e.g., "default", "gaming")
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Key remappings for this profile
    pub remaps: HashMap<String, String>,
}

/// Active profile for a device
#[derive(Debug, Clone)]
pub struct ActiveProfile {
    pub device_path: String,
    pub profile_name: String,
    pub remaps: Arc<RwLock<HashMap<evdev::Key, evdev::Key>>>,
}
```

**Source:** Pattern from Profile struct (`aethermap-common/src/lib.rs:227-231`) and existing RemapEntry in `config.rs:10-17`.

### Pattern 3: IPC Extension for Profile Queries

**What:** New Request/Response variants added to existing IPC protocol in aethermap-common, following existing patterns for macro/profile operations.

**When to use:** GUI needs to query active remaps or switch profiles at runtime.

**Example:**

```rust
// In aethermap-common/src/lib.rs
pub enum Request {
    // ... existing variants ...

    /// Query active remap configuration for a device
    GetActiveRemaps {
        device_path: String,
    },

    /// List available remap profiles for a device
    ListRemapProfiles {
        device_path: String,
    },

    /// Activate a remap profile for a device
    ActivateRemapProfile {
        device_path: String,
        profile_name: String,
    },

    /// Deactivate current remap profile for a device
    DeactivateRemapProfile {
        device_path: String,
    },
}

pub enum Response {
    // ... existing variants ...

    /// Active remap configuration
    ActiveRemaps {
        device_path: String,
        profile_name: Option<String>,
        remaps: Vec<RemapEntry>,
    },

    /// List of available profiles
    RemapProfiles {
        device_path: String,
        profiles: Vec<RemapProfileInfo>,
    },

    /// Profile activation confirmation
    ProfileActivated {
        device_path: String,
        profile_name: String,
    },

    /// Profile deactivation confirmation
    ProfileDeactivated {
        device_path: String,
    },
}
```

**Source:** Existing IPC pattern in `aethermap-common/src/lib.rs:72-159` and `ipc.rs:284-554`.

### Pattern 4: Atomic Profile Switching via Arc Swap

**What:** Pre-validated remap tables stored as Arc<HashMap>. Profile activation swaps the Arc pointer without blocking the event loop.

**When to use:** Runtime profile switching without stopping event processing.

**Example:**

```rust
pub struct DeviceRemapState {
    device_path: String,
    active_profile: Arc<RwLock<Option<String>>>,
    remap_tables: Arc<RwLock<HashMap<String, Arc<HashMap<evdev::Key, evdev::Key>>>>>,
    current_table: Arc<RwLock<Arc<HashMap<evdev::Key, evdev::Key>>>>,
}

impl DeviceRemapState {
    /// Activate a profile by atomically swapping the remap table
    pub async fn activate_profile(&self, profile_name: &str) -> Result<(), RemapError> {
        // Get pre-validated table
        let tables = self.remap_tables.read().await;
        let new_table = tables.get(profile_name)
            .ok_or_else(|| RemapError::ProfileNotFound(profile_name.to_string()))?;

        // Atomic swap - no blocking of event loop
        *self.current_table.write().await = Arc::clone(new_table);
        *self.active_profile.write().await = Some(profile_name.to_string());

        info!("Activated profile '{}' for device {}", profile_name, self.device_path);
        Ok(())
    }

    /// Process event using current profile
    pub async fn process_event(&self, key: evdev::Key, value: i32) -> Option<(evdev::Key, i32)> {
        let table = self.current_table.read().await;
        table.get(&key).copied().map(|out_key| (out_key, value))
    }
}
```

**Source:** Pattern from MacroEngine's Arc<RwLock<HashMap>> storage (`macro_engine.rs:22-24`) and RwLock usage throughout codebase.

### Anti-Patterns to Avoid

- **Blocking event loop during profile switch:** Don't rebuild HashMap in event loop. Pre-validate and cache tables.
- **Global profile state:** Don't store active profiles globally. Per-device state prevents race conditions.
- **Ad-hoc device matching:** Don't parse device strings in hot path. Match once at device discovery, store matched profile.
- **Profile validation at activation time:** Validate all profiles at startup; fail fast on invalid config.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Device matching | Custom string parsing on every event | Match once at discovery, store result | O(1) lookup vs O(n) string matching per event |
| Profile storage | Custom file format | serde_yaml with nested HashMap | Handles edge cases, human-editable |
| IPC protocol | Custom binary protocol | Existing bincode-based IPC | Already integrated with GUI |
| Thread-safe state | Custom mutex wrapper | Arc<RwLock<T>> | Already used in codebase; tokio-aware |
| Key validation | Re-implement KeyParser | Existing KeyParser from Phase 1 | All keys already mapped |

**Key insight:** Phase 1 built KeyParser with friendly name expansion (`key_parser.rs:41-299`). Reuse it for profile validation. Phase 1 built RemapEngine with `Arc<RwLock<HashMap>>` storage (`remap_engine.rs:73-80`). Extend it to support multiple named tables.

---

## Common Pitfalls

### Pitfall 1: Device Identifier Inconsistency

**What goes wrong:** Config uses vendor:product ID, but runtime looks up by /dev/input/event path. Profile never activates.

**Why it happens:** DeviceInfo has both identification methods, but codebase doesn't establish a canonical matching strategy.

**How to avoid:**
1. At device discovery, compute a "device key" from vendor_id:product_id (and optionally phys path)
2. Store this key in GrabbedDevice
3. Match config against device key, not full DeviceInfo
4. Store matched profile in GrabbedDevice for O(1) access

**Prevention strategy:**
```rust
fn device_key(info: &DeviceInfo) -> String {
    format!("{:04x}:{:04x}", info.vendor_id, info.product_id)
}

// At device grab time:
let key = device_key(&device_info);
let profile = match_config_for_device(&config, &key)?;
grabbed_device.active_profile = Some(profile);
```

### Pitfall 2: Profile Switch Race Conditions

**What goes wrong:** Event loop reads old profile while switch writes new one. HashMap gets corrupted or events lost.

**Why it happens:** RwLock write blocks all reads; if switch takes time, event processing pauses.

**How to avoid:**
1. Pre-validate all profiles at startup
2. Store each profile's remap table as Arc<HashMap>
3. Switch by Arc::clone, not by rebuilding
4. Write lock held only for pointer swap (microseconds)

**Warning signs:**
- Key latency spikes during profile switch
- Events "skipped" when profile changes
- GUI reports switch complete but keys still use old profile

### Pitfall 3: Stale Remap References

**What goes wrong:** Device is unplugged, replugged gets different event path. Old RemapEngine still points to stale path.

**Why it happens:** RemapEngine stored in DaemonState globally, not per-device. DeviceManager doesn't update RemapEngine on hotplug.

**How to avoid:**
1. Store RemapEngine per-device in GrabbedDevice
2. On device grab, look up profile and create RemapEngine instance
3. On device ungrab, drop the RemapEngine (Arc handles cleanup)
4. Phase 3 (hotplug) will handle replug correctly

**Code location:** Currently RemapEngine is in DaemonState (`lib.rs:36`), needs to move to GrabbedDevice (`device.rs:15-22`).

### Pitfall 4: Config Parsing Without Error Location

**What goes wrong:** User typo in YAML profile name (e.g., "gmaing" instead of "gaming") causes generic "profile not found" error.

**Why it happens:** serde_yaml errors can be verbose for nested structures. Custom error messages needed for UX.

**How to avoid:**
1. Parse config in two stages: first load all profiles, then validate each
2. Collect all errors before failing
3. Print error with device:profile:remap location

**Example error message:**
```
Error: Invalid key 'KEY_AA' in profile 'gaming' for device 1532:0220
  /etc/aethermapd/remaps.yaml:15:12
    |
 15 |       KEY_AA: KEY_B
    |            ^^^^^^ Did you mean KEY_A?
```

### Pitfall 5: IPC Message Ordering

**What goes wrong:** GUI sends ActivateProfile immediately after ListProfiles. Activation fails because profiles not loaded yet.

**Why it happens:** IPC handlers are async but not serialized. Requests can complete out of order.

**How to avoid:**
1. Ensure all profiles loaded before IPC server starts (already done in main.rs:169-175)
2. Return ProfileNotLoaded error gracefully (don't crash)
3. Document in protocol that profiles available only after daemon startup

---

## Code Examples

### Example 1: Extended RemapConfig Structure

```rust
// aethermapd/src/config.rs

/// Top-level remap configuration with per-device profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemapConfig {
    /// Global default remaps (applied to all devices if no device-specific profile)
    #[serde(default)]
    pub default: Option<GlobalRemaps>,

    /// Per-device remap configurations
    #[serde(default)]
    pub devices: HashMap<String, DeviceRemapConfig>,
}

/// Global remaps applied to all devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalRemaps {
    /// Default remap rules
    pub remaps: HashMap<String, String>,
}

/// Remap configuration for a single device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRemapConfig {
    /// Device match pattern (vendor:product, "Keyboard:*", etc.)
    #[serde(default)]
    pub match_pattern: Option<String>,

    /// Named profiles for this device
    #[serde(default)]
    pub profiles: HashMap<String, ProfileRemaps>,
}

/// Remap rules for a named profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileRemaps {
    /// Human-readable name
    pub name: String,

    /// Key remappings
    #[serde(default)]
    pub remaps: HashMap<String, String>,
}

/// Load and validate remap configuration with profiles
pub async fn load_remap_config(&self) -> Result<RemapConfig, RemapConfigError> {
    let content = fs::read_to_string(&self.remaps_path).await
        .map_err(|e| RemapConfigError::ReadError {
            path: self.remaps_path.clone(),
            source: e,
        })?;

    let config: RemapConfig = serde_yaml::from_str(&content)
        .map_err(|e| RemapConfigError::ParseError {
            path: self.remaps_path.clone(),
            source: e,
        })?;

    // Validate all key names in all profiles
    let key_parser = crate::key_parser::KeyParser::new();

    for (device_id, device_config) in &config.devices {
        for (profile_name, profile) in &device_config.profiles {
            for (from, to) in &profile.remaps {
                // Validate input key
                key_parser.parse(from).map_err(|_| RemapConfigError::InvalidKey {
                    path: self.remaps_path.clone(),
                    key: from.clone(),
                    context: format!("device={}, profile={}", device_id, profile_name),
                })?;

                // Validate output key
                key_parser.parse(to).map_err(|_| RemapConfigError::InvalidKey {
                    path: self.remaps_path.clone(),
                    key: to.clone(),
                    context: format!("device={}, profile={}", device_id, profile_name),
                })?;
            }
        }
    }

    info!("Loaded remap config: {} devices with profiles",
          config.devices.len());
    Ok(config)
}
```

### Example 2: IPC Handler for Profile Operations

```rust
// aethermapd/src/ipc.rs - add to handle_request function

Request::GetActiveRemaps { device_path } => {
    let state = state.read().await;

    // Get device manager
    let device_manager = state.device_manager.as_ref()
        .ok_or_else(|| Response::Error("Device manager not initialized".to_string()))?;

    let dm = device_manager.read().await;

    // Get active remaps for this device
    match dm.get_active_remaps(&device_path).await {
        Ok(Some((profile_name, remaps)) => {
            // Convert remaps to Vec<RemapEntry> for IPC
            let entries: Vec<RemapEntry> = remaps.iter()
                .map(|(from, to)| RemapEntry {
                    from: format!("{:?}", from),
                    to: format!("{:?}", to),
                })
                .collect();

            Response::ActiveRemaps {
                device_path,
                profile_name,
                remaps: entries,
            }
        }
        Ok(None) => {
            Response::ActiveRemaps {
                device_path,
                profile_name: None,
                remaps: vec![],
            }
        }
        Err(e) => Response::Error(format!("Failed to get active remaps: {}", e)),
    }
}

Request::ListRemapProfiles { device_path } => {
    let config_manager = config_manager.clone();

    // Get device identifier from path
    let device_info = match get_device_info(&device_path).await {
        Some(info) => info,
        None => return Response::Error(format!("Device not found: {}", device_path)),
    };

    let device_key = format!("{:04x}:{:04x}", device_info.vendor_id, device_info.product_id);

    // Load config to get profiles
    match config_manager.load_remap_config().await {
        Ok(config) => {
            let profiles = config.devices.get(&device_key)
                .and_then(|d| Some(d.profiles.keys().cloned().collect()))
                .unwrap_or_default();

            Response::RemapProfiles {
                device_path,
                profiles,
            }
        }
        Err(e) => Response::Error(format!("Failed to load config: {}", e)),
    }
}

Request::ActivateRemapProfile { device_path, profile_name } => {
    let state = state.read().await;
    let device_manager = state.device_manager.as_ref()
        .ok_or_else(|| Response::Error("Device manager not initialized".to_string()))?;

    let mut dm = device_manager.write().await;

    match dm.activate_profile(&device_path, &profile_name).await {
        Ok(()) => {
            info!("Activated profile '{}' for device {}", profile_name, device_path);
            Response::ProfileActivated {
                device_path,
                profile_name,
            }
        }
        Err(e) => Response::Error(format!("Failed to activate profile: {}", e)),
    }
}

Request::DeactivateRemapProfile { device_path } => {
    let state = state.read().await;
    let device_manager = state.device_manager.as_ref()
        .ok_or_else(|| Response::Error("Device manager not initialized".to_string()))?;

    let mut dm = device_manager.write().await;

    match dm.deactivate_profile(&device_path).await {
        Ok(()) => {
            info!("Deactivated profile for device {}", device_path);
            Response::ProfileDeactivated {
                device_path,
            }
        }
        Err(e) => Response::Error(format!("Failed to deactivate profile: {}", e)),
    }
}
```

**Source:** IPC handler pattern from `ipc.rs:284-554`.

### Example 3: DeviceManager Profile Activation

```rust
// aethermapd/src/device.rs - add methods to DeviceManager

impl DeviceManager {
    /// Activate a remap profile for a device
    pub async fn activate_profile(
        &mut self,
        device_path: &str,
        profile_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get the grabbed device
        let grabbed_device = self.grabbed_devices.get_mut(device_path)
            .ok_or_else(|| format!("Device not grabbed: {}", device_path))?;

        // Load the profile from config
        let device_key = format!("{:04x}:{:04x}",
            grabbed_device.info.vendor_id,
            grabbed_device.info.product_id
        );

        // TODO: Load profile from config manager
        // For now, assume we have access to pre-validated profiles
        let remap_table = self.load_profile_table(&device_key, profile_name).await?;

        // Update the device's remap engine
        if let Some(remap_engine) = &grabbed_device.remap_engine {
            remap_engine.load_config(&remap_table).await?;
            info!("Activated profile '{}' for device {}", profile_name, device_path);
        }

        Ok(())
    }

    /// Deactivate the current remap profile for a device
    pub async fn deactivate_profile(
        &mut self,
        device_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let grabbed_device = self.grabbed_devices.get_mut(device_path)
            .ok_or_else(|| format!("Device not grabbed: {}", device_path))?;

        // Clear all remaps
        if let Some(remap_engine) = &grabbed_device.remap_engine {
            remap_engine.clear().await;
            info!("Deactivated profile for device {}", device_path);
        }

        Ok(())
    }

    /// Get active remaps for a device
    pub async fn get_active_remaps(
        &self,
        device_path: &str,
    ) -> Result<Option<(String, HashMap<evdev::Key, evdev::Key>)>, Box<dyn std::error::Error>> {
        let grabbed_device = self.grabbed_devices.get(device_path)
            .ok_or_else(|| format!("Device not grabbed: {}", device_path))?;

        if let Some(remap_engine) = &grabbed_device.remap_engine {
            let remaps = remap_engine.get_remaps().await;
            // TODO: Return actual profile name when stored in RemapEngine
            Ok(Some(("default".to_string(), remaps)))
        } else {
            Ok(None)
        }
    }
}
```

**Source:** Pattern from existing grab_device/ungrab_device methods (`device.rs:106-175`).

---

## State of the Art

### Existing Implementations

| Tool | Approach | Relevance |
|------|----------|-----------|
| **keyd** | Per-device config with `[devices]` section, layers as nested tables | HIGH - YAML structure similar to Phase 2 needs |
| **xremap** | Device-specific remaps in YAML, mod+key to switch layers | MEDIUM - IPC is HTTP-based, not Unix socket |
| **kanata** | Config as defn-style language, layers with base/defn | LOW - Custom parser, not YAML |

**Key insight from keyd:** keyd uses a simple `[ids]` section with vendor:product matching. The config structure:

```
[ids]
*
razer:blackwidow_v3 = []

[razer:blackwidow_v3]
default = {
    capslock = leftctrl
}
gaming = {
    capslock = esc
}
```

This maps directly to the proposed YAML structure with `devices` as top-level map.

### Deprecated/Outdated Approaches

- **xmodmap:** X11-specific; doesn't support per-device profiles
- **xkbconfig:** Complex syntax; not user-friendly for profiles
- **interception-tools config:** C-based; requires recompilation for profile changes

---

## Open Questions

1. **Profile storage format**
   - What we know: ConfigManager supports profiles for macros in separate YAML files
   - What's unclear: Should remap profiles be separate files (`/etc/aethermapd/remaps.d/{device}.yaml`) or single file with nested structure?
   - **Recommendation:** Start with single file (`remaps.yaml`) for simplicity; split to directory in Phase 3 if config becomes large

2. **Device match pattern syntax**
   - What we know: Devices identified by vendor_id:product_id or name/phys path
   - What's unclear: Should we support glob patterns ("Razer:*") or exact matches only?
   - **Recommendation:** Start with exact vendor:product matches; add glob patterns in Phase 2.1 if user feedback indicates need

3. **Default profile behavior**
   - What we know: Each device should have an active profile
   - What's unclear: What if device has no matching config? Use global defaults or no remaps?
   - **Recommendation:** Fall back to global default remaps; if none, device operates without remaps (log warning)

4. **Profile persistence across daemon restart**
   - What we know: IPC can activate profile at runtime
   - What's unclear: Should active profile be saved to state file for auto-activation on next startup?
   - **Recommendation:** Defer to Phase 3; Phase 2 always starts with "default" profile

---

## Sources

### Primary (HIGH confidence)

- **remap_engine.rs** (`/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/remap_engine.rs`) - Existing RemapEngine implementation; Arc<RwLock<HashMap>> pattern
- **config.rs** (`/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/config.rs`) - ConfigManager profile loading methods (save_profile, load_profile)
- **ipc.rs** (`/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/ipc.rs`) - IPC handler patterns; existing Request/Response processing
- **device.rs** (`/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/device.rs`) - DeviceManager and GrabbedDevice structures
- **lib.rs** (`/home/feanor/Projects/remapper_rs/aethermap/aethermap-common/src/lib.rs`) - Common types including Profile struct
- **main.rs** (`/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/main.rs`) - Daemon initialization; component wiring

### Secondary (MEDIUM confidence)

- **keyd configuration documentation** (https://github.com/rvaiya/keyd#configuration) - Per-device config patterns with vendor:product matching
- **xremap config example** (https://github.com/xremap/xremap#configuration) - Device-specific remaps in YAML format

### Tertiary (LOW confidence)

- Various GitHub issues discussing profile switching patterns (verify via actual project documentation)

---

## Metadata

### Confidence Breakdown

| Area | Level | Reason |
|------|-------|--------|
| Standard stack | HIGH | All dependencies already in project |
| Architecture | HIGH | Based on existing codebase patterns (Profile, ConfigManager, IPC) |
| Pitfalls | HIGH | Derived from existing code analysis and common async/race condition issues |
| Device matching | MEDIUM | vendor:product matching is standard; pattern matching needs validation |
| Profile storage | MEDIUM | Single file vs directory is a tradeoff; either works |

### Research Date

2026-02-17

### Valid Until

2026-03-19 (30 days - dependencies are stable; Phase 2 should complete within window)

---

*Research complete. Planner can now create PLAN.md files for Phase 2 implementation.*
