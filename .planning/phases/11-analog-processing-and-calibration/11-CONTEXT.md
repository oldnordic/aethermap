# Phase 11: Analog Processing and Calibration - Context

**Phase:** 11 - Analog Processing and Calibration
**Created:** 2026-02-19
**Status:** Ready to plan

## Overview

This phase implements the core analog stick processing pipeline. The Azeron Cyborg's analog stick produces raw EV_ABS events that need to be processed: deadzone filtering, sensitivity curve application, and range scaling. This foundation is required before implementing specific output modes (gamepad, WASD, mouse, camera) in later phases.

## Dependencies

- **Phase 9:** Device Detection and Basic Input (Azeron analog stick events flowing)
- **Phase 10:** Layer System (per-layer analog calibration configuration)

## Decisions (Locked)

### Must Implement
- AnalogProcessor component for processing raw EV_ABS events
- Deadzone configuration (circular with configurable size)
- Sensitivity curves (linear, quadratic, exponential)
- Range scaling (min/max output clamping)
- Per-layer calibration persistence

### Claude's Discretion
- Deadzone shape (circular vs square) - recommend circular for analog stick
- Sensitivity curve formulas - recommend standard game industry curves
- Range defaults - recommend -32768 to 32767 (standard Linux input range)

### Out of Scope
- Specific output modes (gamepad, WASD, mouse) - those are Phases 14-15
- LED control (Phase 12)
- GUI calibration visualization (Phase 16)

## Technical Context

**Raw Analog Input:**
- Azeron Cyborg analog stick outputs ABS_X and ABS_Y events
- Values typically range from 0-255 (8-bit) or similar
- Need to convert to Linux input range (-32768 to 32767)

**Processing Pipeline:**
```
Raw EV_ABS (x, y)
    ↓
Deadzone Filter (ignore small movements)
    ↓
Sensitivity Curve (linear/quadratic/exponential)
    ↓
Range Scaling (min/max clamp, inversion)
    ↓
Normalized Output (-32768 to 32767)
```

**Data Structures:**
```rust
pub struct AnalogCalibration {
    pub deadzone: f32,        // 0.0 to 1.0
    pub deadzone_shape: DeadzoneShape,
    pub sensitivity: SensitivityCurve,
    pub sensitivity_multiplier: f32,
    pub range_min: i32,
    pub range_max: i32,
    pub invert_x: bool,
    pub invert_y: bool,
}

pub enum DeadzoneShape {
    Circular,
    Square,
}

pub enum SensitivityCurve {
    Linear,
    Quadratic,
    Exponential,
}
```

## Success Criteria

1. AnalogProcessor processes raw analog events
2. Deadzone filtering removes small movements
3. Sensitivity curves modify response feel
4. Range scaling clamps output to valid range
5. Per-layer calibration configuration works
6. Calibration persists to YAML config

## Open Questions

None - this is foundational work with clear requirements.

## File Context

**New Files:**
- `razermapper/razermapperd/src/analog_processor.rs` - Core analog processing
- `razermapper/razermapperd/src/analog_calibration.rs` - Calibration types

**Modified Files:**
- `razermapper/razermapperd/src/config.rs` - Add calibration to YAML
- `razermapper/razermapperd/src/device.rs` - Integrate with event loop
- `razermapper/razermapperd/src/lib.rs` - Export new modules

---
*Context created: 2026-02-19*
