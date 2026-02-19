# Phase 16: Calibration GUI - Context

**Phase:** 16 - Calibration GUI
**Created:** 2026-02-18
**Status:** Ready to plan (depends on Phase 15)

## Overview

This phase creates a comprehensive GUI for analog calibration. Users can configure deadzone (shape and size), sensitivity (curve type and multiplier), and range (min/max, inversion). All changes apply immediately with live visual feedback showing the analog stick position and affected zone.

## Dependencies

- **Phase 15:** WASD, Mouse, and Camera Modes (all modes must be implemented)

## Decisions (Locked)

### Must Implement
- Real-time analog input visualization (stick position widget)
- Deadzone controls (slider, shape selector, visual preview)
- Sensitivity controls (curve selector, multiplier, graph)
- Range controls (min/max sliders, inversion checkboxes)
- Live preview with immediate application
- Calibration persistence to device profile YAML

### Claude's Discretion
- Visual layout of calibration screen
- Curve graph rendering approach
- Color scheme for visualization

### Out of Scope
- Advanced Bezier curves
- 3D visualization
- Preset management (beyond YAML config)

## Technical Context

**GUI Component Hierarchy (Iced):**
```
CalibrationScreen
├── ModeSelector (PickList: Gamepad, WASD, Mouse, Camera)
├── AnalogVisualizer (Canvas: shows stick position + zones)
├── DeadzoneSection
│   ├── ShapeSelector (Radio: Circular, Square)
│   ├── RadiusSlider (Slider: 0-1)
│   └── Preview (updated in AnalogVisualizer)
├── SensitivitySection
│   ├── CurveSelector (PickList: Linear, Quadratic, Exponential)
│   ├── MultiplierSlider (Slider: 0.1-3.0)
│   └── CurveGraph (Canvas: plot input → output)
└── RangeSection
    ├── MinXSlider, MaxXSlider, MinYSlider, MaxYSlider
    ├── InvertXCheckbox, InvertYCheckbox
```

**Visualization:**
```rust
// AnalogVisualizer shows:
// 1. Stick position as dot
// 2. Deadzone as shaded circle/square
// 3. Range limits as bounding box
// 4. Direction indicators (N, NE, E, etc.)

// CurveGraph shows:
// 1. Input on X axis (0-1)
// 2. Output on Y axis (0-1)
// 3. Selected curve plotted
// 4. Current stick position marked
```

## Success Criteria

1. Calibration screen accessible from device configuration
2. Real-time analog input visualization works
3. Deadzone controls update visual preview immediately
4. Sensitivity curve selector shows accurate graph
5. Range sliders and inversion work correctly
6. All calibration changes persist to YAML
7. IPC protocol supports calibration read/write

## Open Questions

1. **Layout:** Should calibration be a separate screen or a panel within device config?

2. **Live preview:** How to get real-time analog input into GUI? (Need IPC subscription)

3. **Curve graph:** Draw via Iced Canvas or static images?

## File Context

**New Files:**
- `razermapper/razermapper/src/calibration_screen.rs` - Main calibration UI
- `razermapper/razermapper/src/analog_visualizer.rs` - Stick position widget
- `razermapper/razermapper/src/curve_graph.rs` - Sensitivity curve plot

**Modified Files:**
- `razermapper/razermapper/src/main.rs` - Add calibration screen navigation
- `razermapper/razermapper/src/ipc.rs` - Add calibration subscription/messages
- `razermapper/razermapperd/src/ipc.rs` - Add calibration handlers

**IPC Protocol:**
- `GetCalibration(device_id)` → `CalibrationConfig`
- `SetCalibration(device_id, CalibrationConfig)` → `Ok/Err`
- `SubscribeAnalogInput(device_id)` → stream of `(x, y)` values

---
*Context created: 2026-02-18*
