# Phase 14: Gamepad Emulation Mode - Context

**Phase:** 14 - Gamepad Emulation Mode
**Created:** 2026-02-18
**Status:** Ready to plan (depends on Phase 11)

## Overview

This phase implements Xbox 360 controller emulation mode for analog sticks. The analog stick position is converted to gamepad axis events (ABS_X, ABS_Y) and output via a virtual uinput device. Games and applications recognize this as a standard gamepad. All calibration settings (deadzone, sensitivity, range) are applied before output.

## Dependencies

- **Phase 11:** Analog Processing and Calibration (provides AnalogProcessor foundation)
- **Phase 10:** Layer System (per-layer analog mode configuration)

## Decisions (Locked)

### Must Implement
- Create virtual Xbox 360 compatible gamepad device via uinput
- AnalogMode::Gamepad enum variant
- Calibration pipeline applies before gamepad output
- Per-layer gamepad mode configuration

### Claude's Discretion
- Which Xbox 360 features to implement (full button set or minimum viable)
- Whether to support multiple analog sticks (left/right) or just one

### Out of Scope
- Gamepad button remapping (pass-through only)
- Rumble/force feedback
- Multiple gamepad devices simultaneously

## Technical Context

**Xbox 360 Controller Layout:**
```
Axes:
  ABS_X    - Left stick X (-32768 to 32767)
  ABS_Y    - Left stick Y (-32768 to 32767)
  ABS_Z    - Right stick X (triggers in some drivers)
  ABS_RZ   - Right stick Y (triggers in some drivers)
  ABS_HAT0X - D-pad X
  ABS_HAT0Y - D-pad Y

Buttons:
  BTN_A/B/X/Y (SOUTH/EAST/NORTH/WEST)
  BTN_TL/TR (left/right bumper)
  BTN_THUMBL/THUMBR (stick clicks)
  BTN_START/SELECT
  BTN_MODE (guide button)
```

**Data Flow:**
```
Azeron Analog Input
        ↓
Raw EV_ABS events (x, y values)
        ↓
AnalogProcessor::process() with AnalogMode::Gamepad
        ↓
Deadzone → Sensitivity → Range calibration
        ↓
GamepadVirtualDevice::emit(ABS_X, calibrated_x)
        ↓
Game seen as standard Xbox 360 controller
```

## Success Criteria

1. Virtual Xbox 360 gamepad device created via uinput
2. Analog stick position outputs to gamepad axes
3. Games recognize device via standard Linux gamepad drivers
4. Calibration affects gamepad output
5. Per-layer gamepad mode configuration works

## Open Questions

1. **Stick count:** Should we support left+right stick (Azeron has one), or map single analog to both?

2. **Button pass-through:** Should gamepad buttons be generated from Azeron keys, or just expose axes?

3. **Device naming:** What should the virtual gamepad be named in /dev/input?

## File Context

**New Files:**
- `razermapper/razermapperd/src/gamepad_device.rs` - Virtual gamepad device
- `razermapper/razermapperd/src/analog_modes.rs` - Mode-specific processing (or extend analog_processor.rs)

**Modified Files:**
- `razermapper/razermapperd/src/analog_processor.rs` - Add Gamepad mode
- `razermapper/razermapperd/src/config.rs` - Add gamepad config to YAML
- `razermapper/razermapperd/Cargo.toml` - Verify evdev::uinput capabilities

---
*Context created: 2026-02-18*
