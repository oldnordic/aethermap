---
phase: 14-gamepad-emulation-mode
plan: 04
title: "Per-Layer Gamepad Mode Configuration"
status: complete
date_completed: "2026-02-19"

# Dependency Graph
requires:
  - "14-02"  # AnalogMode enum
provides:
  - "14-05"  # IPC protocol for layer-aware analog mode
affects:
  - "layer_manager.rs"  # LayerConfig with analog_mode field
  - "config.rs"  # YAML examples with analog_mode

# Tech Stack
added:
  - "analog_mode field to LayerConfig struct"
  - "YAML configuration examples for per-layer analog modes"
patterns:
  - "Serde default serialization for backward compatibility"
  - "Per-layer configuration independent of global settings"

# Key Files
created: []
modified:
  - path: "aethermap/aethermapd/src/layer_manager.rs"
    changes:
      - "Added import: use crate::analog_processor::AnalogMode"
      - "Added analog_mode field to LayerConfig struct with documentation"
      - "Updated LayerConfig::default() to initialize analog_mode: AnalogMode::Disabled"
      - "Updated LayerConfig::new() to initialize analog_mode: AnalogMode::Disabled"
      - "Updated DeviceLayerState::new() all three layer configs with analog_mode: AnalogMode::Disabled"
  - path: "aethermap/aethermapd/src/ipc.rs"
    changes:
      - "Added import: use crate::analog_processor::AnalogMode"
      - "Updated SetLayerConfig handler to preserve existing analog_mode value"
  - path: "aethermap/aethermapd/src/config.rs"
    changes:
      - "Added EXAMPLE_CONFIG_WITH_ANALOG_MODES constant with per-layer configuration examples"

# Decisions Made
key_decisions:
  - "analog_mode field uses #[serde(default)] for backward compatibility with existing YAML configs"
  - "All LayerConfig constructors initialize analog_mode to Disabled for safety"
  - "IPC SetLayerConfig handler preserves existing analog_mode when updating other layer config fields"
  - "Per-layer analog_mode enables different behaviors: base layer D-pad for menus, Layer 1 Gamepad for gaming"

# Deviations from Plan
deviations: []

# Performance Metrics
duration_minutes: 8
files_modified: 3
lines_added: 45
tasks_completed: 3
---

# Phase 14 Plan 04: Per-Layer Gamepad Mode Configuration Summary

## One-Liner
Extended LayerConfig with analog_mode field enabling per-layer analog output mode selection (D-pad, Gamepad, etc.).

## Objective
Add per-layer gamepad mode configuration to the layer system, enabling different analog processing modes per layer. For example, base layer could use D-pad mode for menu navigation while Layer 1 uses Gamepad mode for actual gameplay.

## What Was Done

### Task 1: Add analog_mode field to LayerConfig
Added the `analog_mode` field to the `LayerConfig` struct in `layer_manager.rs`:

```rust
/// Analog output mode for this layer
///
/// Determines how analog stick input is processed:
/// - Disabled: No output
/// - Dpad: 8-way directional keys
/// - Gamepad: Xbox 360 compatible gamepad axes
/// - Camera/Mouse/WASD: Additional modes (Phase 15)
#[serde(default)]
pub analog_mode: AnalogMode,
```

Updated all constructors:
- `LayerConfig::default()` - initializes to `AnalogMode::Disabled`
- `LayerConfig::new()` - initializes to `AnalogMode::Disabled`
- `DeviceLayerState::new()` - all three default layer configs include `analog_mode: AnalogMode::Disabled`

### Task 2: Update IPC protocol for analog_mode
Updated the `SetLayerConfig` handler in `ipc.rs` to preserve the existing `analog_mode` value when updating layer configuration:

```rust
// Preserve existing analog_mode when updating layer config
let existing_mode = layer_manager.get_device_state(&device_id).await
    .and_then(|state| {
        state.layer_configs.iter()
            .find(|lc| lc.layer_id == layer_id)
            .map(|lc| lc.analog_mode)
    });

let layer_config = LayerConfig {
    // ... other fields ...
    analog_mode: existing_mode.unwrap_or(AnalogMode::Disabled),  // Preserve existing or default
};
```

This ensures that partial layer config updates don't reset the analog_mode setting.

### Task 3: Add YAML configuration example with analog_mode
Added `EXAMPLE_CONFIG_WITH_ANALOG_MODES` constant to `config.rs` demonstrating per-layer analog mode configuration:

```yaml
layers:
  - layer_id: 0
    name: "Base"
    mode: "Hold"
    analog_mode: "Dpad"  # 8-way directional keys

  - layer_id: 1
    name: "Gaming"
    mode: "Toggle"
    analog_mode: "Gamepad"  # Xbox 360 compatible gamepad

  - layer_id: 2
    name: "Keyboard Only"
    mode: "Toggle"
    analog_mode: "Disabled"  # No analog output
```

The example shows:
- Layer 0 (Base): D-pad mode for menu navigation
- Layer 1: Gamepad mode for gaming
- Layer 2: Disabled (no analog output)

## Key Design Points

### Backward Compatibility
The `#[serde(default)]` attribute ensures that existing YAML configs without `analog_mode` will deserialize with the default `Disabled` value, maintaining backward compatibility.

### Per-Layer Independence
Each layer can independently configure its analog mode. This enables:
- Base layer with D-pad for menu navigation
- Gaming layer with Gamepad for actual gameplay
- Specialized layers with Mouse, Camera, or WASD modes (Phase 15)

### IPC Integration
The IPC handler preserves the existing `analog_mode` value when other layer config fields are updated, preventing accidental resets. Full IPC support for setting `analog_mode` will require `aethermap-common` changes in future phases.

## Verification

- [x] LayerConfig has analog_mode field
- [x] Default value is AnalogMode::Disabled
- [x] All LayerConfig constructors initialize analog_mode
- [x] YAML example demonstrates per-layer configuration
- [x] IPC handlers compile without errors

## Next Steps

Plan 14-05 will integrate the per-layer analog_mode with the event loop to ensure the correct mode is used based on the active layer.
