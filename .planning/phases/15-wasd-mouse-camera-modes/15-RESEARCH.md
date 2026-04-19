# Phase 15: WASD, Mouse, and Camera Modes - Research

**Researched:** 2026-02-19
**Domain:** Linux Input Subsystem (evdev/uinput), Analog-to-Digital Conversion, Velocity Control
**Confidence:** HIGH

## Summary

Phase 15 extends the analog processing system (Phase 11) and gamepad mode (Phase 14) by adding three new output modes for analog stick data: WASD (8-way directional keys), Mouse (velocity-based cursor movement), and Camera (scroll events or key repeat). The existing codebase already provides the foundation with `AnalogMode::Wasd`, `AnalogMode::Mouse`, and `AnalogMode::Camera` enum variants defined in `analog_processor.rs`, along with the 8-way direction detection algorithm and `UinputInjector` for outputting EV_REL events.

**Primary recommendation:** Extend the existing `analog_processor.rs` with mode-specific processing methods (`process_as_wasd()`, `process_as_mouse()`, `process_as_camera()`) following the pattern established by `process_as_dpad()` and `process_as_gamepad()`. Use the existing 8-way detection algorithm for WASD mode, implement velocity scaling for mouse mode using existing `UinputInjector.mouse_move()`, and add scroll event support via `UinputInjector.mouse_scroll()` for camera mode.

**Key technical insight:** The 8-way sector detection algorithm already exists in `AnalogProcessor::detect_dpad_direction()` at lines 979-1033, using 22.5-degree sectors with atan2. This can be reused for WASD mode with different key mappings. The `UinputInjector` already supports EV_REL events (REL_X, REL_Y, REL_WHEEL) established in `injector.rs` lines 205-251, so mouse and camera modes require no new uinput configuration.

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Must Implement:**
- AnalogMode::WASD with 8-way sector detection
- AnalogMode::Mouse with velocity-based movement
- AnalogMode::Camera with configurable output (scroll or keys)
- Mode switching via GUI and IPC
- Per-layer mode configuration

### Claude's Discretion

- WASD diagonal key combinations (e.g., NE = W+D or separate keys)
- Mouse acceleration curve (linear vs curved)
- Camera mode default (scroll vs key repeat)

### Deferred Ideas (OUT OF SCOPE)

- Advanced acceleration curves
- Multi-monitor mouse positioning
- Gesture recognition

</user_constraints>

---

## Standard Stack

### Core Dependencies (Already in Project)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `evdev` | 0.12 | Linux input device access, Key enum, input_event types | De facto standard; already used throughout codebase |
| `tokio` | 1 (full features) | Async runtime for event processing | Project standard; required for async/await |
| `std::f32` methods | built-in | atan2(), sqrt(), to_degrees() for angle calculation | Rust standard library math functions |

### Supporting Dependencies

| Component | Purpose | When to Use |
|-----------|---------|-------------|
| `AnalogProcessor` | 8-way detection, calibration pipeline | All three modes use existing processing |
| `UinputInjector` | EV_REL event emission for mouse/scroll | Mouse and Camera modes |
| `evdev::Key` enum | Key code constants for WASD output | WASD mode key selection |
| `LayerConfig` | Per-layer analog_mode storage | Mode configuration per layer |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Existing `detect_dpad_direction()` | New WASD-specific detection | Reuse is simpler; algorithm is identical for 8-way |
| `UinputInjector::mouse_move()` | Direct write() to uinput fd | Existing method is tested and abstracts complexity |
| Linear velocity scaling | Quadratic/exponential curves | Linear is simplest; can add curves later if needed |

---

## Architecture Patterns

### Recommended Project Structure

```
aethermapd/src/
├── analog_processor.rs    # EXTEND: Add process_as_wasd(), process_as_mouse(), process_as_camera()
├── injector.rs            # USE: Existing mouse_move(), mouse_scroll() for output
├── layer_manager.rs       # USE: Existing LayerConfig.analog_mode for per-layer config
├── ipc.rs                 # EXTEND: Add mode switching commands
└── device.rs              # EXTEND: Route analog events to mode-specific processors
```

### Pattern 1: 8-Way Direction Detection (Already Implemented)

**What:** The `AnalogProcessor::detect_dpad_direction()` method at lines 979-1033 already implements 8-way sector detection using atan2 with 22.5-degree sectors.

**When to use:** WASD mode can use the same algorithm with different key mappings.

**Example:**
```rust
// Source: /home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/analog_processor.rs:979-1033

pub fn detect_dpad_direction(&self, x: f32, y: f32) -> DpadDirection {
    const DIRECTION_THRESHOLD: f32 = 0.1;
    let magnitude = (x * x + y * y).sqrt();
    if magnitude < DIRECTION_THRESHOLD {
        return DpadDirection::None;
    }

    let angle_rad = y.atan2(x);
    let angle_deg = angle_rad.to_degrees();
    let normalized_angle = if angle_deg < 0.0 {
        angle_deg + 360.0
    } else {
        angle_deg
    };

    match normalized_angle {
        a if a >= 337.5 || a < 22.5 => DpadDirection::Right,
        a if a >= 22.5 && a < 67.5 => DpadDirection::UpRight,
        a if a >= 67.5 && a < 112.5 => DpadDirection::Up,
        a if a >= 112.5 && a < 157.5 => DpadDirection::UpLeft,
        a if a >= 157.5 && a < 202.5 => DpadDirection::Left,
        a if a >= 202.5 && a < 247.5 => DpadDirection::DownLeft,
        a if a >= 247.5 && a < 292.5 => DpadDirection::Down,
        a if a >= 292.5 && a < 337.5 => DpadDirection::DownRight,
        _ => DpadDirection::None,
    }
}
```

### Pattern 2: Mode-Specific Processing Methods

**What:** Add `process_as_wasd()`, `process_as_mouse()`, `process_as_camera()` methods to `AnalogProcessor`, following the pattern of `process_as_dpad()` and `process_as_gamepad()`.

**When to use:** Each mode has different output requirements but shares the same input calibration pipeline.

**Example:**
```rust
// WASD mode - returns Vec<(Key, bool)> like process_as_dpad()
pub fn process_as_wasd(
    &self,
    calibration: &AnalogCalibration,
    x: i32,
    y: i32,
) -> Vec<(Key, bool)> {
    // Use existing normalization and centering
    let (nx, ny) = self.normalize(x, y);
    let (cx, cy) = self.center(nx, ny);
    let (dx, dy) = self.apply_deadzone(cx, cy, calibration);

    // Apply inversion for WASD
    let (ix, iy) = (
        if calibration.invert_x { -dx } else { dx },
        if calibration.invert_y { -dy } else { dy },
    );
    let (sx, sy) = (ix * 2.0, iy * 2.0);

    // Detect direction using existing method
    let direction = self.detect_dpad_direction(sx, sy);

    // Convert to WASD keys instead of arrow keys
    wasd_direction_to_keys(direction)
        .into_iter()
        .map(|k| (k, true))
        .collect()
}

// Mouse mode - returns velocity (x, y) for cursor movement
pub fn process_as_mouse(
    &self,
    calibration: &AnalogCalibration,
    x: i32,
    y: i32,
) -> Option<(i32, i32)> {
    let (nx, ny) = self.normalize(x, y);
    let (cx, cy) = self.center(nx, ny);
    let (dx, dy) = self.apply_deadzone(cx, cy, calibration);

    // Check if beyond deadzone
    let magnitude = (dx * dx + dy * dy).sqrt();
    if magnitude < calibration.deadzone {
        return None;
    }

    // Apply sensitivity and scale to velocity range
    let (sx, sy) = self.apply_sensitivity(dx, dy, calibration);

    // Convert to velocity pixels (multiplier controls speed)
    let velocity_multiplier = 10.0; // Configurable
    Some((
        (sx * velocity_multiplier) as i32,
        (sy * velocity_multiplier) as i32,
    ))
}

// Camera mode - returns scroll amount or key repeat events
pub fn process_as_camera(
    &self,
    calibration: &AnalogCalibration,
    x: i32,
    y: i32,
    mode: CameraOutputMode,
) -> CameraOutput {
    let (nx, ny) = self.normalize(x, y);
    let (cx, cy) = self.center(nx, ny);
    let (dx, dy) = self.apply_deadzone(cx, cy, calibration);

    match mode {
        CameraOutputMode::Scroll => {
            // Y-axis movement becomes scroll events
            let scroll_amount = (dy * 3.0) as i32; // Scale factor
            CameraOutput::Scroll(scroll_amount)
        }
        CameraOutputMode::Keys => {
            // X/Y axis becomes key repeats
            let keys = camera_direction_to_keys(dx, dy);
            CameraOutput::Keys(keys)
        }
    }
}
```

### Pattern 3: WASD Key Mapping

**What:** Map 8 directions to WASD keys with diagonal combinations.

**When to use:** When converting analog position to keyboard output for movement.

**Example:**
```rust
// WASD key codes (from evdev)
const KEY_W: u16 = 17;  // evdev::Key::KEY_W
const KEY_A: u16 = 30;  // evdev::Key::KEY_A
const KEY_S: u16 = 31;  // evdev::Key::KEY_S
const KEY_D: u16 = 32;  // evdev::Key::KEY_D

pub enum WasdDirection {
    Up,      // W
    Down,    // S
    Left,    // A
    Right,   // D
    UpLeft,   // W + A
    UpRight,  // W + D
    DownLeft, // S + A
    DownRight,// S + D
    None,
}

pub fn wasd_direction_to_keys(direction: WasdDirection) -> Vec<Key> {
    match direction {
        WasdDirection::None => vec![],
        WasdDirection::Up => vec![Key::KEY_W],
        WasdDirection::Down => vec![Key::KEY_S],
        WasdDirection::Left => vec![Key::KEY_A],
        WasdDirection::Right => vec![Key::KEY_D],
        WasdDirection::UpLeft => vec![Key::KEY_W, Key::KEY_A],
        WasdDirection::UpRight => vec![Key::KEY_W, Key::KEY_D],
        WasdDirection::DownLeft => vec![Key::KEY_S, Key::KEY_A],
        WasdDirection::DownRight => vec![Key::KEY_S, Key::KEY_D],
    }
}
```

**Source:** This follows the pattern of `dpad_direction_to_keys()` at analog_processor.rs:192-204, using WASD keys instead of arrow keys.

### Pattern 4: EV_REL Event Emission

**What:** Use `UinputInjector::mouse_move()` and `UinputInjector::mouse_scroll()` for emitting relative events.

**When to use:** Mouse mode (REL_X/REL_Y) and Camera mode (REL_WHEEL).

**Example:**
```rust
// In device.rs event loop or analog event handler
match analog_mode {
    AnalogMode::Mouse => {
        if let Some((vel_x, vel_y)) = processor.process_as_mouse(&calibration, raw_x, raw_y) {
            injector.mouse_move(vel_x, vel_y).await?;
        }
    }
    AnalogMode::Camera => {
        if let Some(scroll_amt) = processor.process_as_camera(&calibration, raw_x, raw_y) {
            injector.mouse_scroll(scroll_amt).await?;
        }
    }
    _ => {}
}
```

**Source:** `injector.rs:448-470` shows existing mouse_move() and mouse_scroll() implementations using EV_REL events.

### Anti-Patterns to Avoid

- **Separate detection algorithm:** Reuse `detect_dpad_direction()` instead of duplicating the atan2 logic
- **Blocking on uinput write:** Use async methods from `UinputInjector` to avoid blocking event loop
- **Ignoring deadzone in mouse mode:** Velocity should be zero when within deadzone, not constant
- **Hardcoded key codes:** Use `evdev::Key` enum for WASD keys (KEY_W, KEY_A, KEY_S, KEY_D)
- **State tracking in processor:** Let caller (device.rs) track previous state for key release events

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 8-way angle calculation | New atan2 logic | `detect_dpad_direction()` | Already implements 22.5-degree sectors |
| EV_REL event emission | Direct write() to uinput | `UinputInjector::mouse_move()` | Tested, handles sync events |
| Key code constants | Manual u16 values | `evdev::Key` enum | Type-safe, documented |
| Calibration pipeline | New normalize/filter code | `normalize()`, `center()`, `apply_deadzone()` | Already tested for analog input |

**Key insight:** The only new code needed is the mapping from directions to WASD keys and the velocity scaling for mouse/camera modes. Everything else exists.

---

## Common Pitfalls

### Pitfall 1: Missing Key Release Events for WASD

**What goes wrong:** WASD keys get "stuck" pressed because release events are never sent when direction changes.

**Why it happens:** `process_as_wasd()` returns pressed=true keys, but the caller must track previous state and send releases when keys are no longer in the current direction.

**How to avoid:**
```rust
// In device.rs analog event loop
let mut prev_wasd_keys = HashSet::new();

loop {
    let current_keys = processor.process_as_wasd(&calibration, x, y)
        .into_iter()
        .map(|(k, _)| k)
        .collect::<HashSet<_>>();

    // Release keys that are no longer active
    for key in prev_wasd_keys.difference(&current_keys) {
        injector.key_release(key.0).await?;
    }

    // Press new keys
    for key in current_keys.difference(&prev_wasd_keys) {
        injector.key_press(key.0).await?;
    }

    prev_wasd_keys = current_keys;
}
```

**Warning signs:** Characters repeat infinitely, movement continues after releasing stick.

### Pitfall 2: Continuous Velocity in Deadzone

**What goes wrong:** Cursor drifts even when analog stick is centered.

**Why it happens:** Mouse mode returns small velocity values instead of None when within deadzone.

**How to avoid:**
```rust
// Always check deadzone before returning velocity
let magnitude = (dx * dx + dy * dy).sqrt();
if magnitude < calibration.deadzone {
    return None;  // No movement in deadzone
}
```

**Prevention strategy:** Unit test with centered input (127, 127 for 8-bit ADC).

### Pitfall 3: Mouse Velocity Too Fast/Slow

**What goes wrong:** Cursor moves too quickly to control or too slowly to be useful.

**Why it happens:** Hardcoded velocity multiplier doesn't account for user preference or screen resolution.

**How to avoid:**
1. Add configurable `mouse_sensitivity` field to `DeviceAnalogConfig`
2. Provide IPC command to adjust sensitivity at runtime
3. Default to conservative value (e.g., 5.0 pixels/frame)

**Example:**
```rust
#[serde(default = "default_mouse_sensitivity")]
pub mouse_sensitivity: f32,

fn default_mouse_sensitivity() -> f32 {
    5.0  // Pixels per unit of analog deflection
}
```

### Pitfall 4: Scroll Direction Inverted

**What goes wrong:** Pushing stick up scrolls down (or vice versa).

**Why it happens:** Y-axis inversion mismatch between calibration and scroll output.

**How to avoid:**
1. Respect `calibration.invert_y` for scroll direction
2. Document expected behavior (up = scroll up/negative REL_WHEEL)
3. Provide invert option in config

**Warning signs:** Pushing stick up makes page go down.

### Pitfall 5: No Repeat Events for Camera Keys

**What goes wrong:** Camera mode with keys only outputs one press event per stick movement.

**Why it happens:** Mode only outputs press events on state change, not continuous repeat while held.

**How to avoid:**
```rust
// For key repeat, use tokio::time::interval
let mut repeat_interval = tokio::time::interval(Duration::from_millis(100));

loop {
    repeat_interval.tick().await;
    if direction != Direction::None {
        injector.key_press(key).await?;
        injector.key_release(key).await?;  // Immediate for repeat
    }
}
```

**Note:** This may be complex for initial implementation. Recommend scroll mode as default.

---

## Code Examples

### Example 1: WASD Mode Processing

```rust
// aethermap/aethermapd/src/analog_processor.rs

/// Convert D-pad direction to WASD keys
///
/// Diagonal directions return two keys (e.g., UpRight = W + D).
pub fn wasd_direction_to_keys(direction: DpadDirection) -> Vec<Key> {
    match direction {
        DpadDirection::None => vec![],
        DpadDirection::Up => vec![Key::KEY_W],
        DpadDirection::Down => vec![Key::KEY_S],
        DpadDirection::Left => vec![Key::KEY_A],
        DpadDirection::Right => vec![Key::KEY_D],
        DpadDirection::UpLeft => vec![Key::KEY_W, Key::KEY_A],
        DpadDirection::UpRight => vec![Key::KEY_W, Key::KEY_D],
        DpadDirection::DownLeft => vec![Key::KEY_S, Key::KEY_A],
        DpadDirection::DownRight => vec![Key::KEY_S, Key::KEY_D],
    }
}

/// Process analog input as WASD output
///
/// Returns Vec of (Key, bool) where bool indicates pressed (true).
/// The caller is responsible for tracking previous state and sending
/// release events when direction changes.
pub fn process_as_wasd(
    &self,
    calibration: &AnalogCalibration,
    x: i32,
    y: i32,
) -> Vec<(Key, bool)> {
    // Step 1: Normalize to 0.0-1.0 range
    let (nx, ny) = self.normalize(x, y);

    // Step 2: Center at origin
    let (cx, cy) = self.center(nx, ny);

    // Step 3: Apply deadzone filter
    let (dx, dy) = self.apply_deadzone(cx, cy, calibration);

    // Apply axis inversion
    let (ix, iy) = (
        if calibration.invert_x { -dx } else { dx },
        if calibration.invert_y { -dy } else { dy },
    );

    // Scale to -1.0 to 1.0 for direction detection
    let (sx, sy) = (ix * 2.0, iy * 2.0);

    // Detect direction using existing algorithm
    let direction = self.detect_dpad_direction(sx, sy);

    // Convert to WASD keys
    wasd_direction_to_keys(direction)
        .into_iter()
        .map(|k| (k, true))
        .collect()
}
```

### Example 2: Mouse Mode with Velocity Scaling

```rust
// aethermap/aethermapd/src/analog_processor.rs

/// Mouse velocity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseVelocityConfig {
    /// Velocity multiplier (pixels per unit analog deflection)
    #[serde(default = "default_mouse_multiplier")]
    pub multiplier: f32,

    /// Enable velocity ramping (smoothing)
    #[serde(default)]
    pub ramping: bool,
}

fn default_mouse_multiplier() -> f32 {
    10.0  // 10 pixels per 1.0 analog value
}

/// Process analog input as mouse velocity output
///
/// Returns Some((vel_x, vel_y)) with cursor velocity in pixels.
/// Returns None if within deadzone (no movement).
pub fn process_as_mouse(
    &self,
    calibration: &AnalogCalibration,
    x: i32,
    y: i32,
    config: &MouseVelocityConfig,
) -> Option<(i32, i32)> {
    // Normalize and center
    let (nx, ny) = self.normalize(x, y);
    let (cx, cy) = self.center(nx, ny);

    // Check deadzone (magnitude in centered coords)
    let magnitude = (cx * cx + cy * cy).sqrt();
    if magnitude < calibration.deadzone {
        return None;
    }

    // Apply deadzone scaling
    let (dx, dy) = self.apply_deadzone(cx, cy, calibration);

    // Apply sensitivity curve
    let (sx, sy) = self.apply_sensitivity(dx, dy, calibration);

    // Convert to velocity with multiplier
    let vel_x = (sx * config.multiplier) as i32;
    let vel_y = (sy * config.multiplier) as i32;

    Some((vel_x, vel_y))
}
```

### Example 3: Camera Mode with Scroll Output

```rust
// aethermap/aethermapd/src/analog_processor.rs

/// Camera output mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraOutputMode {
    /// Emit REL_WHEEL events
    Scroll,
    /// Emit key repeat events (PageUp/PageDown or arrows)
    Keys,
}

/// Camera mode output
pub enum CameraOutput {
    /// Scroll amount for REL_WHEEL event
    Scroll(i32),
    /// Keys to emit for key repeat mode
    Keys(Vec<Key>),
}

/// Process analog input as camera output
///
/// For scroll mode: returns scroll amount based on Y-axis position
/// For key mode: returns keys based on X/Y direction
pub fn process_as_camera(
    &self,
    calibration: &AnalogCalibration,
    x: i32,
    y: i32,
    mode: CameraOutputMode,
) -> Option<CameraOutput> {
    let (nx, ny) = self.normalize(x, y);
    let (cx, cy) = self.center(nx, ny);

    let magnitude = (cx * cx + cy * cy).sqrt();
    if magnitude < calibration.deadzone {
        return None;
    }

    let (dx, dy) = self.apply_deadzone(cx, cy, calibration);

    match mode {
        CameraOutputMode::Scroll => {
            // Y-axis deflection becomes scroll amount
            // Apply sensitivity multiplier
            let scroll_amount = (dy * calibration.sensitivity_multiplier * 3.0) as i32;
            Some(CameraOutput::Scroll(scroll_amount))
        }
        CameraOutputMode::Keys => {
            // Detect direction for key output
            let (sx, sy) = (dx * 2.0, dy * 2.0);
            let direction = self.detect_dpad_direction(sx, sy);

            // Convert to camera keys (PageUp/PageDown or arrows)
            let keys = camera_direction_to_keys(direction);
            Some(CameraOutput::Keys(keys))
        }
    }
}

fn camera_direction_to_keys(direction: DpadDirection) -> Vec<Key> {
    match direction {
        DpadDirection::None => vec![],
        DpadDirection::Up => vec![Key::KEY_PAGEUP],
        DpadDirection::Down => vec![Key::KEY_PAGEDOWN],
        DpadDirection::Left => vec![Key::KEY_LEFT],
        DpadDirection::Right => vec![Key::KEY_RIGHT],
        // Diagonals: emit both axes
        DpadDirection::UpLeft => vec![Key::KEY_PAGEUP, Key::KEY_LEFT],
        DpadDirection::UpRight => vec![Key::KEY_PAGEUP, Key::KEY_RIGHT],
        DpadDirection::DownLeft => vec![Key::KEY_PAGEDOWN, Key::KEY_LEFT],
        DpadDirection::DownRight => vec![Key::KEY_PAGEDOWN, Key::KEY_RIGHT],
    }
}
```

---

## State of the Art

### Old Approach vs Current Approach

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Fixed key mapping | Per-layer configurable analog mode | Phase 10 | Different layers can use different modes |
| Arrow keys only | WASD + D-pad + gamepad | Phase 15 | Multiple input styles for different use cases |
| Absolute positioning | Velocity-based movement | Ongoing | Better control for cursor and scroll |

### Existing Codebase Patterns (2025-2026)

**Phase 11** established the calibration pipeline with `AnalogCalibration` and 2D coordinate processing.

**Phase 14** added `GamepadVirtualDevice` and `process_as_gamepad()` for Xbox 360 emulation.

**Phase 15** extends this pattern with WASD, Mouse, and Camera modes using the same foundation.

---

## Open Questions

### 1. WASD Diagonal Keys

**Question:** Should diagonals use key combinations (W+A) or dedicated keys?

**What we know:** The `dpad_direction_to_keys()` pattern returns two keys for diagonals. Most games expect W+A for diagonal movement.

**Recommendation:** Use key combinations (W+D for NE, etc.) to match standard game expectations. Dedicated keys would require re-binding in most games.

**Confidence:** HIGH - this matches the existing `process_as_dpad()` pattern.

### 2. Camera Mode Default

**Question:** Should default be scroll or key-based?

**What we know:**
- Scroll (REL_WHEEL) works universally in browsers and document viewers
- Key repeat is application-specific and requires repeat rate tuning
- `UinputInjector` already supports `mouse_scroll()`

**Recommendation:** Default to scroll mode for broader compatibility. Key mode can be added as an option for specialized use cases (3D software navigation).

**Confidence:** HIGH - scroll is more universally applicable.

### 3. Mouse Acceleration Curve

**Question:** Should mouse mode use linear or curved velocity?

**What we know:**
- Existing `AnalogCalibration` supports `SensitivityCurve::Linear` and `Exponential`
- Linear is simpler and more predictable
- Exponential provides more precision at low deflection

**Recommendation:** Start with linear (1:1) velocity scaling. The existing sensitivity multiplier provides enough adjustment. Add curves in future if users report precision issues.

**Confidence:** MEDIUM - linear should be sufficient for initial release.

---

## Sources

### Primary (HIGH confidence)

- [analog_processor.rs:979-1033](/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/analog_processor.rs) - 8-way direction detection algorithm (atan2 with 22.5-degree sectors)
- [analog_processor.rs:1075-1104](/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/analog_processor.rs) - `process_as_dpad()` pattern for directional key output
- [analog_processor.rs:1122-1152](/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/analog_processor.rs) - `process_as_gamepad()` pattern for calibrated 2D processing
- [injector.rs:205-251](/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/injector.rs) - uinput EV_REL configuration (REL_X, REL_Y, REL_WHEEL)
- [injector.rs:448-470](/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/injector.rs) - `mouse_move()` and `mouse_scroll()` implementation
- [layer_manager.rs:180-191](/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/layer_manager.rs) - `LayerConfig.analog_mode` field for per-layer configuration
- [15-CONTEXT.md](/home/feanor/Projects/remapper_rs/.planning/phases/15-wasd-mouse-camera-modes/15-CONTEXT.md) - Phase context with locked decisions

### Secondary (MEDIUM confidence)

- [Linux Kernel Documentation - uinput](https://www.kernel.org/doc/html/v4.16/input/uinput.html) - Official uinput programming documentation
- [evdev crate documentation](https://docs.rs/evdev) - Confirms Key enum values for WASD (KEY_W=17, KEY_A=30, KEY_S=31, KEY_D=32)
- [Phase 01 Research](/home/feanor/Projects/remapper_rs/.planning/phases/01-core-remapping/01-RESEARCH.md) - Established evdev/uinput patterns used throughout project

### Tertiary (LOW confidence - marked for validation)

- Various game input programming resources online (verify with actual implementation)

---

## Metadata

### Confidence Breakdown

| Area | Level | Reason |
|------|-------|--------|
| Standard stack | HIGH | All dependencies already in project; verified via Cargo.toml and existing code |
| Architecture | HIGH | Patterns established by Phase 11 (calibration) and Phase 14 (gamepad mode) |
| 8-way detection | HIGH | Algorithm already implemented in `detect_dpad_direction()` |
| EV_REL events | HIGH | `UinputInjector` already supports mouse_move and mouse_scroll |
| WASD key mapping | HIGH | Standard gaming pattern; evdev::Key enum provides codes |
| Mouse acceleration | MEDIUM | Linear vs curved is user preference; can be adjusted later |
| Camera mode default | HIGH | Scroll is more universally compatible than key repeat |

### Research Date

2026-02-19

### Valid Until

2026-03-21 (30 days - Linux input subsystem is stable; evdev crate is mature)

---

*Research complete. Planner can now create PLAN.md files for implementing WASD, Mouse, and Camera modes.*
