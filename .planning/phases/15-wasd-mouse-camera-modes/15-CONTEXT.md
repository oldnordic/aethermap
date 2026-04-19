# Phase 15: WASD, Mouse, and Camera Modes - Context

**Phase:** 15 - WASD, Mouse, and Camera Modes
**Created:** 2026-02-18
**Status:** Ready to plan (depends on Phase 14)

## Overview

This phase implements three additional analog processing modes: WASD (8-way directional keys), Mouse (velocity-based cursor movement), and Camera (scroll or key repeat for rotation). Each mode transforms analog stick position into different output types, enabling flexible use cases for gaming and productivity.

## Dependencies

- **Phase 14:** Gamepad Emulation Mode (provides mode switching foundation)

## Decisions (Locked)

### Must Implement
- AnalogMode::WASD with 8-way sector detection
- AnalogMode::Mouse with velocity-based movement
- AnalogMode::Camera with configurable output (scroll or keys)
- Mode switching via GUI and IPC
- Per-layer mode configuration

### Claude's Discretion
- WASD diagonal key combinations (e.g., NE = W+D or separate keys)
- Mouse acceleration curve (linear vs curved)
- Camera mode default (scroll vs key repeat)

### Out of Scope
- Advanced acceleration curves
- Multi-monitor mouse positioning
- Gesture recognition

## Technical Context

**WASD Mode - 8-Way Detection:**
```
       N (W)
        |
   NW (Q)    NE (E)
        |
W (A) +  E (D)
        |
   SW (Z)    SE (C)
        |
       S (X)

Deadzone: Center circle = no output
Beyond deadzone: Determine sector, emit keys
```

**Sector Detection Algorithm:**
```rust
let angle = (y.atan2(x) + 2*PI) % (2*PI);  // 0 to 2PI
let sector = match angle {
    0.0..=PI/8   => Direction::E,
    PI/8..=3*PI/8 => Direction::NE,
    // ... 8 sectors total
};
```

**Mouse Mode - Velocity Control:**
```rust
// Distance from center = velocity
let distance = sqrt(x*x + y*y);
let velocity = apply_sensitivity(distance, curve, multiplier);

// Emit EV_REL events
emit(REL_X, velocity_x as i32);
emit(REL_Y, velocity_y as i32);
```

**Camera Mode - Scroll vs Keys:**
- Scroll: Emit REL_WHEEL events (vertical scrolling)
- Keys: Repeat PageUp/PageDown or arrow keys based on stick position

## Success Criteria

1. WASD mode outputs 8-way directional keys
2. Diagonal directions output key combinations (W+D, etc.)
3. Mouse mode moves cursor smoothly
4. Camera mode emits scroll or key events
5. Mode switching works via GUI and IPC
6. All modes support per-layer configuration

## Open Questions

1. **WASD diagonal keys:** Should diagonals use key combinations (W+A) or dedicated keys?

2. **Camera mode default:** Should default be scroll or key-based? (Recommend scroll for broader compatibility)

3. **Repeat rate:** For camera key mode, what should the repeat rate be?

## File Context

**Modified Files:**
- `aethermap/aethermapd/src/analog_processor.rs` - Add WASD, Mouse, Camera modes
- `aethermap/aethermapd/src/analog_modes.rs` - (if extracted in Phase 14)

**IPC Extensions:**
- Add mode switching commands to IPC protocol

---
*Context created: 2026-02-18*
